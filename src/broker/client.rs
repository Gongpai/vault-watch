use std::io;
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;

use super::response::{response_frame_len, response_header_len};
use super::{
    AtaBrokerOperation, BrokerAtaResponse, BrokerDeviceRef, BrokerPeerPolicy, BrokerRequest,
    BrokerResponseError, BrokerResponseWireError, BrokerWireError, decode_response_frame,
    encode_request_frame, peer_credentials,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const TRANSPORT_GRACE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerClientError {
    Connect(io::ErrorKind),
    UnauthorizedServer,
    RequestIdExhausted,
    EncodeRequest(BrokerWireError),
    Transport(io::ErrorKind),
    TimedOut,
    InvalidResponse(BrokerResponseWireError),
    MismatchedResponse,
    ServerDenied(BrokerResponseError),
    ConnectionClosed,
}

/// A sequential client session for the typed broker protocol. Request IDs and
/// framing are owned here so callers cannot replay or splice raw IPC frames.
#[derive(Debug)]
pub struct BrokerClient {
    stream: Option<UnixStream>,
    next_request_id: u64,
}

#[derive(Debug)]
enum ExchangeError {
    Io(io::ErrorKind),
    Wire(BrokerResponseWireError),
}

impl BrokerClient {
    pub async fn connect(
        socket_path: &Path,
        expected_server: BrokerPeerPolicy,
    ) -> Result<Self, BrokerClientError> {
        if !socket_path.is_absolute() || socket_path.file_name().is_none() {
            return Err(BrokerClientError::Connect(io::ErrorKind::InvalidInput));
        }
        let stream = timeout(CONNECT_TIMEOUT, UnixStream::connect(socket_path))
            .await
            .map_err(|_| BrokerClientError::TimedOut)?
            .map_err(|error| BrokerClientError::Connect(error.kind()))?;
        let credentials =
            peer_credentials(&stream).map_err(|error| BrokerClientError::Connect(error.kind()))?;
        if !expected_server.accepts(credentials) {
            return Err(BrokerClientError::UnauthorizedServer);
        }
        Ok(Self {
            stream: Some(stream),
            next_request_id: 1,
        })
    }

    pub async fn execute(
        &mut self,
        device: BrokerDeviceRef,
        operation: AtaBrokerOperation,
    ) -> Result<BrokerAtaResponse, BrokerClientError> {
        self.execute_with_deadline(device, operation, operation.timeout() + TRANSPORT_GRACE)
            .await
    }

    async fn execute_with_deadline(
        &mut self,
        device: BrokerDeviceRef,
        operation: AtaBrokerOperation,
        deadline: Duration,
    ) -> Result<BrokerAtaResponse, BrokerClientError> {
        let request_id = self.next_request_id;
        self.next_request_id = request_id
            .checked_add(1)
            .ok_or(BrokerClientError::RequestIdExhausted)?;
        let request = BrokerRequest {
            request_id,
            device,
            operation,
        };
        let encoded = encode_request_frame(&request).map_err(BrokerClientError::EncodeRequest)?;
        let mut stream = self
            .stream
            .take()
            .ok_or(BrokerClientError::ConnectionClosed)?;

        let exchange = async {
            stream
                .write_all(&encoded)
                .await
                .map_err(|error| ExchangeError::Io(error.kind()))?;
            let mut header = vec![0u8; response_header_len()];
            stream
                .read_exact(&mut header)
                .await
                .map_err(|error| ExchangeError::Io(error.kind()))?;
            let frame_len = response_frame_len(&header).map_err(ExchangeError::Wire)?;
            header.resize(frame_len, 0);
            stream
                .read_exact(&mut header[response_header_len()..])
                .await
                .map_err(|error| ExchangeError::Io(error.kind()))?;
            Ok::<Vec<u8>, ExchangeError>(header)
        };
        let frame = match timeout(deadline, exchange).await {
            Ok(Ok(frame)) => frame,
            Ok(Err(ExchangeError::Io(kind))) => return Err(BrokerClientError::Transport(kind)),
            Ok(Err(ExchangeError::Wire(error))) => {
                return Err(BrokerClientError::InvalidResponse(error));
            }
            Err(_) => return Err(BrokerClientError::TimedOut),
        };
        let response = decode_response_frame(&frame).map_err(BrokerClientError::InvalidResponse)?;
        if response.request_id != request_id || response.operation != operation {
            return Err(BrokerClientError::MismatchedResponse);
        }
        self.stream = Some(stream);
        response.result.map_err(BrokerClientError::ServerDenied)
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixStream as StdUnixStream;

    use super::*;
    use crate::ata::{AtaMedium, SmartStatus};
    use crate::broker::{
        AtaIdentifySummary, BrokerGeneration, BrokerResponseFrame, decode_request_frame,
        encode_response_frame,
    };

    fn device() -> BrokerDeviceRef {
        BrokerDeviceRef {
            node_id: "block:sda".to_owned(),
            generation: BrokerGeneration {
                diskseq: 42,
                dev_major: 8,
                dev_minor: 0,
            },
        }
    }

    fn client_from_pair() -> Option<(BrokerClient, StdUnixStream)> {
        let (client, server) = StdUnixStream::pair().unwrap();
        match peer_credentials(&server) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => return None,
            Err(error) => panic!("unexpected SO_PEERCRED error: {error}"),
        }
        client.set_nonblocking(true).unwrap();
        Some((
            BrokerClient {
                stream: Some(UnixStream::from_std(client).unwrap()),
                next_request_id: 1,
            },
            server,
        ))
    }

    #[tokio::test]
    async fn client_owns_monotonic_ids_and_validates_correlated_responses() {
        let Some((mut client, server)) = client_from_pair() else {
            return;
        };
        server.set_nonblocking(true).unwrap();
        let mut server = UnixStream::from_std(server).unwrap();
        let task = tokio::spawn(async move {
            for expected_id in 1..=2 {
                let mut header = vec![0; super::super::wire::request_header_len()];
                server.read_exact(&mut header).await.unwrap();
                let frame_len = super::super::wire::request_frame_len(&header).unwrap();
                header.resize(frame_len, 0);
                server
                    .read_exact(&mut header[super::super::wire::request_header_len()..])
                    .await
                    .unwrap();
                let request = decode_request_frame(&header).unwrap();
                assert_eq!(request.request_id, expected_id);
                let response = match request.operation {
                    AtaBrokerOperation::IdentifyDevice => {
                        BrokerAtaResponse::Identify(AtaIdentifySummary {
                            capacity_bytes: Some(1),
                            medium: AtaMedium::SolidState,
                            smart_supported: true,
                            general_purpose_logging_supported: false,
                        })
                    }
                    AtaBrokerOperation::SmartReturnStatus => {
                        BrokerAtaResponse::SmartStatus(SmartStatus::Passed)
                    }
                    _ => unreachable!(),
                };
                server
                    .write_all(
                        &encode_response_frame(&BrokerResponseFrame {
                            request_id: request.request_id,
                            operation: request.operation,
                            result: Ok(response),
                        })
                        .unwrap(),
                    )
                    .await
                    .unwrap();
            }
        });

        assert!(matches!(
            client
                .execute(device(), AtaBrokerOperation::IdentifyDevice)
                .await
                .unwrap(),
            BrokerAtaResponse::Identify(_)
        ));
        assert_eq!(
            client
                .execute(device(), AtaBrokerOperation::SmartReturnStatus)
                .await
                .unwrap(),
            BrokerAtaResponse::SmartStatus(SmartStatus::Passed)
        );
        task.await.unwrap();
    }

    #[tokio::test]
    async fn correlation_failure_closes_the_session() {
        let Some((mut client, server)) = client_from_pair() else {
            return;
        };
        server.set_nonblocking(true).unwrap();
        let mut server = UnixStream::from_std(server).unwrap();
        tokio::spawn(async move {
            let mut request = vec![0; super::super::wire::request_header_len()];
            server.read_exact(&mut request).await.unwrap();
            let frame_len = super::super::wire::request_frame_len(&request).unwrap();
            request.resize(frame_len, 0);
            server
                .read_exact(&mut request[super::super::wire::request_header_len()..])
                .await
                .unwrap();
            server
                .write_all(
                    &encode_response_frame(&BrokerResponseFrame {
                        request_id: 99,
                        operation: AtaBrokerOperation::SmartReturnStatus,
                        result: Ok(BrokerAtaResponse::SmartStatus(SmartStatus::Passed)),
                    })
                    .unwrap(),
                )
                .await
                .unwrap();
        });

        assert_eq!(
            client
                .execute(device(), AtaBrokerOperation::IdentifyDevice)
                .await,
            Err(BrokerClientError::MismatchedResponse)
        );
        assert_eq!(
            client
                .execute(device(), AtaBrokerOperation::IdentifyDevice)
                .await,
            Err(BrokerClientError::ConnectionClosed)
        );
    }

    #[tokio::test]
    async fn timeout_poisons_the_connection() {
        let Some((mut client, _server)) = client_from_pair() else {
            return;
        };
        assert_eq!(
            client
                .execute_with_deadline(
                    device(),
                    AtaBrokerOperation::IdentifyDevice,
                    Duration::from_millis(1),
                )
                .await,
            Err(BrokerClientError::TimedOut)
        );
    }

    #[test]
    fn oversized_response_is_rejected_before_payload_allocation() {
        let mut oversized = vec![0; response_header_len()];
        oversized[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            response_frame_len(&oversized),
            Err(BrokerResponseWireError::PayloadTooLarge)
        );
    }

    #[test]
    fn server_policy_rejects_wrong_kernel_identity() {
        let (client, _server) = StdUnixStream::pair().unwrap();
        let peer = match peer_credentials(&client) {
            Ok(peer) => peer,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => return,
            Err(error) => panic!("unexpected SO_PEERCRED error: {error}"),
        };
        let policy = BrokerPeerPolicy {
            allowed_uid: peer.uid,
            allowed_gid: peer.gid,
        };
        assert!(policy.accepts(peer));
        assert!(
            !BrokerPeerPolicy {
                allowed_uid: peer.uid.wrapping_add(1),
                allowed_gid: peer.gid,
            }
            .accepts(peer)
        );
    }
}
