use std::future::Future;
use std::io;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::sync::{Arc, RwLock};

use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::storage::StorageInventory;

use super::BrokerSocket;
use super::wire::{request_frame_len, request_header_len};
use super::{
    AtaBrokerOperation, AuthorizedBrokerRequest, BrokerDeviceGrant, BrokerPeerCredentials,
    BrokerPeerPolicy, BrokerResponseError, BrokerResponseFrame, BrokerSession,
    authorize_ata_request, decode_request_frame, encode_response_frame,
    open_system_authorized_device, peer_credentials,
};

pub const MAX_CONCURRENT_BROKER_SESSIONS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerServerConfigError {
    InvalidInventory,
    DuplicateGrant,
    InvalidGrant,
    StateUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrokerServerStateSummary {
    pub revision: u64,
    pub node_count: usize,
    pub grant_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerServerAuditOutcome {
    Completed,
    InvalidRequest,
    AuthorizationDenied,
    DeviceUnavailable,
    ExecutionFailed,
    SessionLimit,
}

/// Sanitized server audit event. Device identity, generation, paths, protocol
/// payloads, and OS/transport error details deliberately remain absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrokerServerAuditRecord {
    pub peer: BrokerPeerCredentials,
    pub request_id: Option<u64>,
    pub operation: Option<AtaBrokerOperation>,
    pub outcome: BrokerServerAuditOutcome,
}

#[derive(Debug)]
pub struct BrokerServer {
    authorization: RwLock<BrokerAuthorizationState>,
    peer_policy: BrokerPeerPolicy,
}

#[derive(Debug)]
struct BrokerAuthorizationState {
    revision: u64,
    inventory: StorageInventory,
    grants: Vec<BrokerDeviceGrant>,
}

impl BrokerServer {
    pub fn new(
        inventory: StorageInventory,
        grants: Vec<BrokerDeviceGrant>,
        peer_policy: BrokerPeerPolicy,
    ) -> Result<Self, BrokerServerConfigError> {
        validate_authorization_state(&inventory, &grants)?;
        Ok(Self {
            authorization: RwLock::new(BrokerAuthorizationState {
                revision: 1,
                inventory,
                grants,
            }),
            peer_policy,
        })
    }

    /// Atomically replaces inventory and grants after validating the complete
    /// snapshot. Failed or partial snapshots leave the current state intact.
    pub fn reconcile_authorization(
        &self,
        inventory: StorageInventory,
        grants: Vec<BrokerDeviceGrant>,
    ) -> Result<BrokerServerStateSummary, BrokerServerConfigError> {
        validate_authorization_state(&inventory, &grants)?;
        let mut state = self
            .authorization
            .write()
            .map_err(|_| BrokerServerConfigError::StateUnavailable)?;
        let revision = state.revision.saturating_add(1);
        *state = BrokerAuthorizationState {
            revision,
            inventory,
            grants,
        };
        Ok(state.summary())
    }

    pub fn state_summary(&self) -> Result<BrokerServerStateSummary, BrokerServerConfigError> {
        self.authorization
            .read()
            .map(|state| state.summary())
            .map_err(|_| BrokerServerConfigError::StateUnavailable)
    }

    /// Runs the broker accept loop until shutdown. The listener is duplicated
    /// only to integrate readiness with Tokio; `BrokerSocket` retains endpoint
    /// identity and performs inode-safe cleanup.
    pub async fn serve_socket<F, S>(
        self: Arc<Self>,
        socket: &BrokerSocket,
        shutdown: S,
        audit: F,
    ) -> io::Result<()>
    where
        F: Fn(BrokerServerAuditRecord) + Send + Sync + 'static,
        S: Future<Output = ()> + Send,
    {
        if !socket.permits_peer_policy(self.peer_policy)? {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "broker socket owner/group does not permit the configured peer",
            ));
        }
        let listener = socket.listener().try_clone()?;
        listener.set_nonblocking(true)?;
        let listener = AsyncFd::new(listener)?;
        let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_BROKER_SESSIONS));
        let audit = Arc::new(audit);
        let mut sessions = JoinSet::new();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                completed = sessions.join_next(), if !sessions.is_empty() => {
                    // A malformed or disconnected client must not terminate the
                    // listener. Join failures are isolated to that session.
                    let _ = completed;
                }
                accepted = accept_ready(&listener) => {
                    let stream = accepted?;
                    let permit = match Arc::clone(&permits).try_acquire_owned() {
                        Ok(permit) => permit,
                        Err(_) => {
                            if let Ok(peer) = peer_credentials(&stream) {
                                audit(BrokerServerAuditRecord {
                                    peer,
                                    request_id: None,
                                    operation: None,
                                    outcome: BrokerServerAuditOutcome::SessionLimit,
                                });
                            }
                            continue;
                        }
                    };
                    let server = Arc::clone(&self);
                    let audit = Arc::clone(&audit);
                    sessions.spawn(async move {
                        let _permit = permit;
                        let _ = server.serve_connection(stream, |record| audit(record)).await;
                    });
                }
            }
        }

        // Dropping/aborting a future does not promise to cancel an SG_IO ioctl
        // already running in spawn_blocking. Typed command timeouts remain the
        // hard upper bound while process shutdown proceeds.
        sessions.abort_all();
        while sessions.join_next().await.is_some() {}
        Ok(())
    }

    /// Serves one already accepted Unix connection until EOF or the first
    /// malformed transport frame. Requests execute sequentially per session;
    /// the device executor applies its own process-wide concurrency bound.
    pub async fn serve_connection<F>(&self, stream: StdUnixStream, mut audit: F) -> io::Result<()>
    where
        F: FnMut(BrokerServerAuditRecord),
    {
        let peer = peer_credentials(&stream)?;
        let mut session = BrokerSession::new(self.peer_policy, peer)
            .map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "unauthorized peer"))?;
        stream.set_nonblocking(true)?;
        let mut stream = UnixStream::from_std(stream)?;

        loop {
            let frame = match read_request_frame(&mut stream).await {
                Ok(Some(frame)) => frame,
                Ok(None) => return Ok(()),
                Err(error) => {
                    if matches!(
                        error.kind(),
                        io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof
                    ) {
                        audit(BrokerServerAuditRecord {
                            peer,
                            request_id: None,
                            operation: None,
                            outcome: BrokerServerAuditOutcome::InvalidRequest,
                        });
                    }
                    return Err(error);
                }
            };
            let (response, record) = self.dispatch_frame(&mut session, &frame).await;
            audit(record);
            let Some(response) = response else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "request frame could not be correlated",
                ));
            };
            let encoded = encode_response_frame(&response).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "response encoding failed")
            })?;
            stream.write_all(&encoded).await?;
        }
    }

    async fn dispatch_frame(
        &self,
        session: &mut BrokerSession,
        frame: &[u8],
    ) -> (Option<BrokerResponseFrame>, BrokerServerAuditRecord) {
        let decoded_for_audit = decode_request_frame(frame).ok();
        let request = match session.decode_next(frame) {
            Ok(request) => request,
            Err(_) => {
                let response = decoded_for_audit
                    .as_ref()
                    .map(|request| BrokerResponseFrame {
                        request_id: request.request_id,
                        operation: request.operation,
                        result: Err(BrokerResponseError::InvalidRequest),
                    });
                return (
                    response,
                    audit_record(
                        session.peer(),
                        decoded_for_audit.as_ref(),
                        BrokerServerAuditOutcome::InvalidRequest,
                    ),
                );
            }
        };

        let authorized = match self.authorize_request(&request) {
            Ok(authorized) => authorized,
            Err(_) => {
                return denied(
                    session.peer(),
                    &request,
                    BrokerResponseError::AuthorizationDenied,
                    BrokerServerAuditOutcome::AuthorizationDenied,
                );
            }
        };
        let opened = match open_system_authorized_device(&authorized) {
            Ok(opened) => opened,
            Err(_) => {
                return denied(
                    session.peer(),
                    &request,
                    BrokerResponseError::DeviceUnavailable,
                    BrokerServerAuditOutcome::DeviceUnavailable,
                );
            }
        };
        let result = match opened.execute_ata().await {
            Ok(response) => Ok(response),
            Err(_) => Err(BrokerResponseError::ExecutionFailed),
        };
        let outcome = if result.is_ok() {
            BrokerServerAuditOutcome::Completed
        } else {
            BrokerServerAuditOutcome::ExecutionFailed
        };
        (
            Some(BrokerResponseFrame {
                request_id: request.request_id,
                operation: request.operation,
                result,
            }),
            audit_record(session.peer(), Some(&request), outcome),
        )
    }

    fn authorize_request(
        &self,
        request: &super::BrokerRequest,
    ) -> Result<AuthorizedBrokerRequest, super::BrokerDenyReason> {
        let state = self
            .authorization
            .read()
            .map_err(|_| super::BrokerDenyReason::UnknownDevice)?;
        let grant = state
            .grants
            .iter()
            .find(|grant| grant.node_id == request.device.node_id)
            .ok_or(super::BrokerDenyReason::UnknownDevice)?;
        authorize_ata_request(&state.inventory, grant, request)
    }
}

impl BrokerAuthorizationState {
    const fn summary(&self) -> BrokerServerStateSummary {
        BrokerServerStateSummary {
            revision: self.revision,
            node_count: self.inventory.nodes.len(),
            grant_count: self.grants.len(),
        }
    }
}

fn validate_authorization_state(
    inventory: &StorageInventory,
    grants: &[BrokerDeviceGrant],
) -> Result<(), BrokerServerConfigError> {
    if inventory.partial || inventory.validate().is_err() {
        return Err(BrokerServerConfigError::InvalidInventory);
    }
    for (index, grant) in grants.iter().enumerate() {
        if grants[..index]
            .iter()
            .any(|existing| existing.node_id == grant.node_id)
        {
            return Err(BrokerServerConfigError::DuplicateGrant);
        }
    }
    for grant in grants {
        let matching = inventory
            .nodes
            .iter()
            .filter(|node| node.id == grant.node_id)
            .collect::<Vec<_>>();
        if matching.len() != 1
            || matching[0].kind != crate::storage::StorageKind::ScsiLike
            || matching[0].materialization != crate::storage::Materialization::BlockDevice
            || super::broker_generation(&matching[0].generation) != Some(grant.generation)
        {
            return Err(BrokerServerConfigError::InvalidGrant);
        }
    }
    Ok(())
}

async fn accept_ready(
    listener: &AsyncFd<std::os::unix::net::UnixListener>,
) -> io::Result<StdUnixStream> {
    loop {
        let mut ready = listener.readable().await?;
        match ready.try_io(|inner| inner.get_ref().accept()) {
            Ok(Ok((stream, _))) => return Ok(stream),
            Ok(Err(error)) => return Err(error),
            Err(_) => continue,
        }
    }
}

fn denied(
    peer: BrokerPeerCredentials,
    request: &super::BrokerRequest,
    error: BrokerResponseError,
    outcome: BrokerServerAuditOutcome,
) -> (Option<BrokerResponseFrame>, BrokerServerAuditRecord) {
    (
        Some(BrokerResponseFrame {
            request_id: request.request_id,
            operation: request.operation,
            result: Err(error),
        }),
        audit_record(peer, Some(request), outcome),
    )
}

fn audit_record(
    peer: BrokerPeerCredentials,
    request: Option<&super::BrokerRequest>,
    outcome: BrokerServerAuditOutcome,
) -> BrokerServerAuditRecord {
    BrokerServerAuditRecord {
        peer,
        request_id: request.map(|request| request.request_id),
        operation: request.map(|request| request.operation),
        outcome,
    }
}

async fn read_request_frame(stream: &mut UnixStream) -> io::Result<Option<Vec<u8>>> {
    let mut header = vec![0u8; request_header_len()];
    match stream.read_exact(&mut header[..1]).await {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    stream.read_exact(&mut header[1..]).await?;
    let frame_len = request_frame_len(&header)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid request length"))?;
    let mut frame = header;
    frame.resize(frame_len, 0);
    stream
        .read_exact(&mut frame[request_header_len()..])
        .await?;
    Ok(Some(frame))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::broker::{
        AtaCapabilityGrant, BrokerDeviceRef, BrokerGeneration, BrokerRequest, BrokerWireError,
        decode_response_frame, encode_request_frame,
    };
    use crate::storage::model::Generation;
    use crate::storage::{Materialization, StorageKind, StorageNode};

    fn peer_policy() -> BrokerPeerPolicy {
        BrokerPeerPolicy {
            // SAFETY: geteuid/getegid have no preconditions.
            allowed_uid: unsafe { libc::geteuid() },
            // SAFETY: geteuid/getegid have no preconditions.
            allowed_gid: unsafe { libc::getegid() },
        }
    }

    fn request(request_id: u64) -> BrokerRequest {
        request_for_generation(request_id, 42)
    }

    fn request_for_generation(request_id: u64, diskseq: u64) -> BrokerRequest {
        BrokerRequest {
            request_id,
            device: BrokerDeviceRef {
                node_id: "block:sda".to_owned(),
                generation: BrokerGeneration {
                    diskseq,
                    dev_major: 8,
                    dev_minor: 0,
                },
            },
            operation: AtaBrokerOperation::IdentifyDevice,
        }
    }

    fn inventory_for_generation(diskseq: u64) -> StorageInventory {
        StorageInventory {
            nodes: vec![StorageNode {
                id: "block:sda".into(),
                name: "sda".into(),
                kind: StorageKind::ScsiLike,
                materialization: Materialization::BlockDevice,
                removable: Some(false),
                identities: Vec::new(),
                generation: Generation {
                    diskseq: Some(diskseq),
                    dev_t: Some((8, 0)),
                },
            }],
            edges: Vec::new(),
            partial: false,
        }
    }

    fn grant_for_generation(diskseq: u64) -> BrokerDeviceGrant {
        BrokerDeviceGrant {
            node_id: "block:sda".into(),
            generation: BrokerGeneration {
                diskseq,
                dev_major: 8,
                dev_minor: 0,
            },
            backend: super::super::GrantedBackend::AtaSat,
            ata: AtaCapabilityGrant::default(),
        }
    }

    fn private_test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "vault-watch-broker-server-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path.canonicalize().unwrap()
    }

    #[test]
    fn configuration_rejects_partial_inventory_and_duplicate_grants() {
        assert!(matches!(
            BrokerServer::new(
                StorageInventory {
                    partial: true,
                    ..StorageInventory::default()
                },
                Vec::new(),
                peer_policy(),
            ),
            Err(BrokerServerConfigError::InvalidInventory)
        ));

        let grant = BrokerDeviceGrant {
            node_id: "block:sda".to_owned(),
            generation: request(1).device.generation,
            backend: super::super::GrantedBackend::AtaSat,
            ata: AtaCapabilityGrant::default(),
        };
        assert!(matches!(
            BrokerServer::new(
                StorageInventory::default(),
                vec![grant.clone(), grant],
                peer_policy(),
            ),
            Err(BrokerServerConfigError::DuplicateGrant)
        ));
    }

    #[test]
    fn reconciliation_atomically_swaps_generation_and_retains_on_invalid_snapshot() {
        let server = BrokerServer::new(
            inventory_for_generation(42),
            vec![grant_for_generation(42)],
            peer_policy(),
        )
        .unwrap();
        assert_eq!(
            server.state_summary().unwrap(),
            BrokerServerStateSummary {
                revision: 1,
                node_count: 1,
                grant_count: 1,
            }
        );
        assert!(
            server
                .authorize_request(&request_for_generation(1, 42))
                .is_ok()
        );
        assert!(
            server
                .authorize_request(&request_for_generation(2, 43))
                .is_err()
        );

        let summary = server
            .reconcile_authorization(inventory_for_generation(43), vec![grant_for_generation(43)])
            .unwrap();
        assert_eq!(summary.revision, 2);
        assert!(
            server
                .authorize_request(&request_for_generation(3, 42))
                .is_err()
        );
        assert!(
            server
                .authorize_request(&request_for_generation(4, 43))
                .is_ok()
        );

        assert_eq!(
            server.reconcile_authorization(
                StorageInventory {
                    partial: true,
                    ..StorageInventory::default()
                },
                Vec::new(),
            ),
            Err(BrokerServerConfigError::InvalidInventory)
        );
        assert_eq!(server.state_summary().unwrap(), summary);
        assert!(
            server
                .authorize_request(&request_for_generation(5, 43))
                .is_ok()
        );

        assert_eq!(
            server.reconcile_authorization(
                inventory_for_generation(44),
                vec![grant_for_generation(43)],
            ),
            Err(BrokerServerConfigError::InvalidGrant)
        );
        assert_eq!(server.state_summary().unwrap(), summary);
        assert!(
            server
                .authorize_request(&request_for_generation(6, 43))
                .is_ok()
        );
    }

    #[tokio::test]
    async fn connection_authenticates_decodes_audits_and_returns_typed_denial() {
        let server =
            BrokerServer::new(StorageInventory::default(), Vec::new(), peer_policy()).unwrap();
        let (server_stream, client_stream) = StdUnixStream::pair().unwrap();
        match peer_credentials(&server_stream) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => return,
            Err(error) => panic!("unexpected SO_PEERCRED error: {error}"),
        }
        client_stream.set_nonblocking(true).unwrap();
        let mut client = UnixStream::from_std(client_stream).unwrap();
        let audits = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&audits);
        let task = tokio::spawn(async move {
            server
                .serve_connection(server_stream, |record| {
                    captured.lock().unwrap().push(record);
                })
                .await
        });

        let encoded = encode_request_frame(&request(1)).unwrap();
        client.write_all(&encoded).await.unwrap();
        let mut header = [0u8; 20];
        client.read_exact(&mut header).await.unwrap();
        let payload_len = u32::from_le_bytes(header[16..20].try_into().unwrap()) as usize;
        let mut response = header.to_vec();
        response.resize(20 + payload_len, 0);
        client.read_exact(&mut response[20..]).await.unwrap();
        let decoded = decode_response_frame(&response).unwrap();
        assert_eq!(
            decoded.result,
            Err(BrokerResponseError::AuthorizationDenied)
        );
        drop(client);
        task.await.unwrap().unwrap();

        let records = audits.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].request_id, Some(1));
        assert_eq!(
            records[0].outcome,
            BrokerServerAuditOutcome::AuthorizationDenied
        );
    }

    #[tokio::test]
    async fn replay_returns_invalid_request_without_consuming_device_access() {
        let server =
            BrokerServer::new(StorageInventory::default(), Vec::new(), peer_policy()).unwrap();
        let peer = BrokerPeerCredentials {
            uid: peer_policy().allowed_uid,
            gid: peer_policy().allowed_gid,
            pid: 1,
        };
        let mut session = BrokerSession::new(peer_policy(), peer).unwrap();
        let frame = encode_request_frame(&request(7)).unwrap();
        let (first, _) = server.dispatch_frame(&mut session, &frame).await;
        assert_eq!(
            first.unwrap().result,
            Err(BrokerResponseError::AuthorizationDenied)
        );
        let (replayed, audit) = server.dispatch_frame(&mut session, &frame).await;
        assert_eq!(
            replayed.unwrap().result,
            Err(BrokerResponseError::InvalidRequest)
        );
        assert_eq!(audit.outcome, BrokerServerAuditOutcome::InvalidRequest);
    }

    #[test]
    fn request_length_parser_rejects_unbounded_node_length() {
        let mut header = vec![0; request_header_len()];
        header[32..34].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(
            request_frame_len(&header),
            Err(BrokerWireError::InvalidNodeLength)
        );
    }

    #[tokio::test]
    async fn socket_loop_obeys_shutdown_and_cleans_up_endpoint() {
        let directory = private_test_directory();
        let path = directory.join("broker.sock");
        let socket = match BrokerSocket::bind(&path) {
            Ok(socket) => socket,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                fs::remove_dir(directory).unwrap();
                return;
            }
            Err(error) => panic!("unexpected broker bind error: {error}"),
        };
        let server = Arc::new(
            BrokerServer::new(StorageInventory::default(), Vec::new(), peer_policy()).unwrap(),
        );
        server
            .serve_socket(&socket, async {}, |_| {})
            .await
            .unwrap();
        assert!(path.exists());
        drop(socket);
        assert!(!path.exists());
        fs::remove_dir(directory).unwrap();
    }
}
