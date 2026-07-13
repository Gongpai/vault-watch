#[cfg(target_os = "linux")]
mod linux {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::process::ExitCode;
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::signal::unix::{SignalKind, signal};
    use tokio::sync::Notify;
    use tokio::time::{self, MissedTickBehavior};
    use vault_watch::broker::{
        BrokerDeviceGrant, BrokerPeerPolicy, BrokerServer, BrokerServerAuditRecord, BrokerSocket,
        DEFAULT_BROKER_SOCKET_PATH, discover_ata_capabilities, reconcile_ata_capabilities,
    };
    use vault_watch::storage;

    const RECONCILE_INTERVAL: Duration = Duration::from_secs(5 * 60);

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Options {
        socket: PathBuf,
        allowed_uid: u32,
        allowed_gid: u32,
        discover_ata: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum ParseOutcome {
        Run(Options),
        Help,
    }

    pub async fn main() -> ExitCode {
        match run().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("vault-watch-broker: {error}");
                ExitCode::FAILURE
            }
        }
    }

    async fn run() -> Result<(), String> {
        let options = match parse_args(std::env::args_os().skip(1))? {
            ParseOutcome::Run(options) => options,
            ParseOutcome::Help => {
                print_help();
                return Ok(());
            }
        };
        let inventory = storage::discover_storage();
        let grants = if options.discover_ata {
            let report = discover_ata_capabilities(&inventory).await;
            eprintln!(
                "vault-watch-broker: ATA capability discovery granted {} of {} classified nodes",
                report.grants.len(),
                report.outcomes.len()
            );
            report.grants
        } else {
            Vec::new()
        };
        let grant_count = grants.len();
        let reconciliation_grants = grants.clone();
        let peer_policy = BrokerPeerPolicy {
            allowed_uid: options.allowed_uid,
            allowed_gid: options.allowed_gid,
        };
        let server = Arc::new(
            BrokerServer::new(inventory, grants, peer_policy)
                .map_err(|error| format!("refusing unsafe startup inventory: {error:?}"))?,
        );
        let socket = BrokerSocket::bind_for_peer(&options.socket, peer_policy)
            .map_err(|error| format!("cannot bind {}: {error}", options.socket.display()))?;
        let mut terminate = signal(SignalKind::terminate())
            .map_err(|error| format!("cannot install SIGTERM handler: {error}"))?;
        let reconciliation = if options.discover_ata {
            let hints = Arc::new(Notify::new());
            storage::spawn_block_event_hints(Arc::clone(&hints));
            let server = Arc::clone(&server);
            Some(tokio::spawn(async move {
                reconcile_authorization_loop(server, reconciliation_grants, hints).await;
            }))
        } else {
            None
        };

        eprintln!(
            "vault-watch-broker: listening with {grant_count} broker-owned ATA capability grants"
        );
        let result = Arc::clone(&server)
            .serve_socket(
                &socket,
                async move {
                    tokio::select! {
                        result = tokio::signal::ctrl_c() => {
                            let _ = result;
                        }
                        _ = terminate.recv() => {}
                    }
                },
                audit_record,
            )
            .await;
        if let Some(reconciliation) = reconciliation {
            reconciliation.abort();
            let _ = reconciliation.await;
        }
        result.map_err(|error| format!("server loop failed: {error}"))
    }

    async fn reconcile_authorization_loop(
        server: Arc<BrokerServer>,
        mut grants: Vec<BrokerDeviceGrant>,
        hints: Arc<Notify>,
    ) {
        let start = time::Instant::now() + RECONCILE_INTERVAL;
        let mut periodic = time::interval_at(start, RECONCILE_INTERVAL);
        periodic.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = periodic.tick() => {}
                _ = hints.notified() => {}
            }
            let inventory = storage::discover_storage();
            if inventory.partial || inventory.validate().is_err() {
                eprintln!(
                    "vault-watch-broker: retained authorization state after incomplete inventory reconciliation"
                );
                continue;
            }
            let report = reconcile_ata_capabilities(&inventory, &grants).await;
            let next_grants = report.grants;
            match server.reconcile_authorization(inventory, next_grants.clone()) {
                Ok(summary) => {
                    grants = next_grants;
                    eprintln!(
                        "vault-watch-broker: authorization revision={} nodes={} grants={}",
                        summary.revision, summary.node_count, summary.grant_count
                    );
                }
                Err(_) => {
                    eprintln!(
                        "vault-watch-broker: retained authorization state after rejected reconciliation"
                    );
                }
            }
        }
    }

    fn audit_record(record: BrokerServerAuditRecord) {
        eprintln!(
            "vault-watch-broker audit peer_uid={} peer_gid={} peer_pid={} request_id={:?} operation={:?} outcome={:?}",
            record.peer.uid,
            record.peer.gid,
            record.peer.pid,
            record.request_id,
            record.operation,
            record.outcome
        );
    }

    fn parse_args<I>(args: I) -> Result<ParseOutcome, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut socket = PathBuf::from(DEFAULT_BROKER_SOCKET_PATH);
        let mut socket_overridden = false;
        let mut uid = None;
        let mut gid = None;
        let mut discover_ata = false;
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            let Some(argument) = argument.to_str() else {
                return Err("option names must be valid UTF-8".to_owned());
            };
            match argument {
                "-h" | "--help" => return Ok(ParseOutcome::Help),
                "--socket" => {
                    if socket_overridden {
                        return Err("--socket may be specified only once".to_owned());
                    }
                    let value = args
                        .next()
                        .ok_or_else(|| "--socket requires a path".to_owned())?;
                    socket = PathBuf::from(value);
                    socket_overridden = true;
                }
                "--uid" => uid = Some(parse_id("--uid", args.next(), uid)?),
                "--gid" => gid = Some(parse_id("--gid", args.next(), gid)?),
                "--discover-ata" => {
                    if discover_ata {
                        return Err("--discover-ata may be specified only once".to_owned());
                    }
                    discover_ata = true;
                }
                _ => return Err(format!("unknown option {argument:?}")),
            }
        }
        if !socket.is_absolute() || socket.file_name().is_none() {
            return Err("--socket must be an absolute file path".to_owned());
        }
        Ok(ParseOutcome::Run(Options {
            socket,
            allowed_uid: uid.ok_or_else(|| "--uid is required".to_owned())?,
            allowed_gid: gid.ok_or_else(|| "--gid is required".to_owned())?,
            discover_ata,
        }))
    }

    fn parse_id(name: &str, value: Option<OsString>, previous: Option<u32>) -> Result<u32, String> {
        if previous.is_some() {
            return Err(format!("{name} may be specified only once"));
        }
        let value = value.ok_or_else(|| format!("{name} requires a numeric value"))?;
        value
            .to_str()
            .ok_or_else(|| format!("{name} must be valid UTF-8"))?
            .parse()
            .map_err(|_| format!("{name} must be a u32"))
    }

    fn print_help() {
        println!(
            "Usage: vault-watch-broker [--socket PATH] --uid UID --gid GID [--discover-ata]\n\
             \n\
             Runs the privileged IPC boundary with default-deny device grants.\n\
             --discover-ata enables fixed broker-owned SAT capability probes.\n\
             The socket parent must already exist and be owned by this process.\n\
             Run the service with an effective group matching GID so mode 0660 permits the peer."
        );
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn args(values: &[&str]) -> Vec<OsString> {
            values.iter().map(OsString::from).collect()
        }

        #[test]
        fn requires_explicit_peer_identity_and_rejects_ambiguous_options() {
            assert_eq!(parse_args(args(&[])), Err("--uid is required".to_owned()));
            assert_eq!(
                parse_args(args(&["--uid", "1000"])),
                Err("--gid is required".to_owned())
            );
            assert!(parse_args(args(&["--uid", "x", "--gid", "1"])).is_err());
            assert!(parse_args(args(&["--uid", "1", "--uid", "2", "--gid", "1"])).is_err());
            assert!(parse_args(args(&["--uid", "1", "--gid", "1", "extra"])).is_err());
            assert!(
                parse_args(args(&[
                    "--socket",
                    "/run/one.sock",
                    "--socket",
                    "/run/two.sock",
                    "--uid",
                    "1",
                    "--gid",
                    "1"
                ]))
                .is_err()
            );
        }

        #[test]
        fn parses_strict_runtime_options_and_help() {
            assert_eq!(
                parse_args(args(&[
                    "--socket",
                    "/run/private/vault-watch.sock",
                    "--uid",
                    "1000",
                    "--gid",
                    "100"
                ])),
                Ok(ParseOutcome::Run(Options {
                    socket: PathBuf::from("/run/private/vault-watch.sock"),
                    allowed_uid: 1000,
                    allowed_gid: 100,
                    discover_ata: false,
                }))
            );
            assert_eq!(
                parse_args(args(&["--uid", "1000", "--gid", "100", "--discover-ata"])),
                Ok(ParseOutcome::Run(Options {
                    socket: PathBuf::from(DEFAULT_BROKER_SOCKET_PATH),
                    allowed_uid: 1000,
                    allowed_gid: 100,
                    discover_ata: true,
                }))
            );
            assert_eq!(parse_args(args(&["--help"])), Ok(ParseOutcome::Help));
            assert!(
                parse_args(args(&[
                    "--socket",
                    "relative.sock",
                    "--uid",
                    "1",
                    "--gid",
                    "1"
                ]))
                .is_err()
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> std::process::ExitCode {
    linux::main().await
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("vault-watch-broker is supported only on Linux");
    std::process::ExitCode::FAILURE
}
