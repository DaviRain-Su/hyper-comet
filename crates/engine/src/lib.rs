//! comet-engine — the headless backend: sessions engine, doc host + command executor,
//! run journal + crash recovery, and the IPC RPC server.
//!
//! Spec: ARCHITECTURE.md §5 and docs/research/feature-inventory.md §3. M2 surface:
//! sessions + docs + commands + minimal IPC. Terminals, repos/diffs, uploads, auth,
//! agent accounts, and the device-room host land in later milestones.

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use comet_proto::HarnessId;

use comet_sync::DocsStore;

pub mod agent_accounts;
pub mod auth;
pub mod chat2_host;
pub mod diff_sync;
pub mod doc_host;
pub mod instance_lock;
pub mod registry;
pub mod repos;
pub mod rpc;
pub mod run_journal;
pub mod sessions;
pub mod spaces;
pub mod studio;
pub mod terminals;
pub mod titles;
pub mod uploads;
pub mod workspace_host;

pub use agent_accounts::{AgentAccounts, AgentAccountsConfig};
pub use auth::{Auth, AuthConfig, AuthState, AuthUser, OrgMembership};
pub use diff_sync::{
    CheckoutDiffSync, DiffSidecar, DiffSnapshot, TurnSnapshot, capture_diff, capture_diff_against,
    capture_turn_diff, merge_base, snapshot_tree,
};
pub use doc_host::{ChatDocHandle, DocHost, DocHostConfig, EdgeConfig};
pub use instance_lock::InstanceLock;
pub use registry::{HarnessDescriptor, HarnessRegistry, default_registry};
pub use repos::{CheckoutIdentity, Repos, worktree_branch_from_title};
pub use rpc::EngineRpc;
pub use run_journal::{JournalError, RunJournal};
pub use sessions::{JournaledEvent, SessionsEngine, SteerOutcome};
pub use spaces::SpacesSync;
pub use studio::{
    DEFAULT_PROOFSHIP_RELAY, DEFAULT_PROOFSHIP_WEB, DeployStore, DraftRunner, NetworkStore,
    StudioDeployer, StudioGate, StudioInteract, StudioLaunchRunner, StudioPreview, StudioRelay,
    StudioStore, TemplateStore, WalletConnectBridge, WalletStore, resolve_device_token,
    resolve_relay_base, resolve_relay_identity,
};
pub use terminals::Terminals;
pub use titles::TitleGenerator;
pub use uploads::{AttachmentChunk, Uploads};
pub use workspace_host::{
    DEFAULT_ORG_ID, DEFAULT_USER_ID, WORKSPACE_DOC_ID, WorkspaceHost, WorkspaceHostConfig,
};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("doc: {0}")]
    Doc(#[from] comet_doc::DocError),
    #[error("journal: {0}")]
    Journal(#[from] run_journal::JournalError),
    #[error("store: {0}")]
    Store(#[from] comet_sync::StoreError),
    #[error("harness: {0}")]
    Harness(#[from] comet_harness::HarnessError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

/// Epoch millis now — the doc/journal timestamp base.
pub(crate) fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub(crate) fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn parse_relay_harness(lane: Option<&str>) -> HarnessId {
    let raw = lane.unwrap_or("codex").trim();
    let quoted = format!("\"{raw}\"");
    serde_json::from_str(&quoted).unwrap_or(HarnessId::Codex)
}

fn mirror_agent_event(relay: &StudioRelay, event: &comet_proto::AgentEvent) {
    match event {
        comet_proto::AgentEvent::TextDelta { text } => {
            relay.publish("session.agent", serde_json::json!({ "text": text }));
        }
        comet_proto::AgentEvent::ToolCall { id, call } => {
            relay.publish(
                "session.tool",
                serde_json::json!({ "id": id, "call": call }),
            );
        }
        comet_proto::AgentEvent::Done {
            status, error, ..
        } => {
            relay.publish(
                "session.done",
                serde_json::json!({
                    "status": status,
                    "error": error,
                }),
            );
        }
        comet_proto::AgentEvent::Error { message } => {
            relay.publish(
                "session.done",
                serde_json::json!({ "ok": false, "error": message }),
            );
        }
        _ => {}
    }
}

fn relay_module_source(module: &str) -> Option<String> {
    let file = format!("{module}.lean");
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::var("PROOFSHIP_RELAY_CWD") {
        candidates.push(std::path::PathBuf::from(cwd).join(&file));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(&file));
        candidates.push(cwd.join("proofship").join("inbox").join(&file));
    }
    candidates.push(crate::repos::home_dir().join("proofship").join("inbox").join(&file));
    for path in candidates {
        if let Ok(src) = std::fs::read_to_string(&path)
            && !src.trim().is_empty()
        {
            return Some(src);
        }
    }
    None
}

async fn handle_relay_deploy(
    relay: &StudioRelay,
    network_store: &NetworkStore,
    wallet_store: &WalletStore,
    studio_deploy: &StudioDeployer,
    studio_interact: &StudioInteract,
    network_id: Option<&str>,
    module: Option<&str>,
    digest: Option<&str>,
) {
    use futures::StreamExt;
    let Some(network_id) = network_id else {
        relay.publish(
            "executor.refused",
            serde_json::json!({ "reason": "missing_network_id" }),
        );
        return;
    };
    let Some(module) = module else {
        relay.publish(
            "executor.refused",
            serde_json::json!({ "reason": "missing_module" }),
        );
        return;
    };
    let networks = match network_store.load() {
        Ok(n) => n,
        Err(err) => {
            relay.note(&format!("deploy networks: {err}"));
            return;
        }
    };
    let Some(network) = networks.into_iter().find(|n| n.id == network_id) else {
        relay.publish(
            "executor.refused",
            serde_json::json!({ "reason": "unknown_network", "networkId": network_id }),
        );
        return;
    };
    let wallets = match wallet_store.load() {
        Ok(w) => w,
        Err(err) => {
            relay.note(&format!("deploy wallets: {err}"));
            return;
        }
    };
    let Some(wallet) = wallets
        .into_iter()
        .find(|w| !matches!(w.source, comet_proto::WalletSource::Watch))
    else {
        relay.publish(
            "executor.refused",
            serde_json::json!({
                "reason": "no_signing_wallet",
                "hint": "Add WalletConnect or DevEnvKey on the desktop executor",
            }),
        );
        return;
    };
    let Some(source) = relay_module_source(module) else {
        relay.publish(
            "executor.refused",
            serde_json::json!({
                "reason": "missing_module_source",
                "module": module,
                "hint": "Place a gated {Module}.lean under PROOFSHIP_RELAY_CWD or proofship/inbox",
            }),
        );
        return;
    };
    let expected_digest = digest.map(str::to_string);
    let req = comet_proto::StudioDeployRequest {
        module: module.into(),
        source,
        network_id: network_id.into(),
        wallet_id: wallet.id.clone(),
        ctor_sig: "-".into(),
        ctor_args: Vec::new(),
        launch_id: None,
        project_id: None,
    };
    relay.publish("gate.start", serde_json::json!({ "phase": "deploy" }));
    let mut stream = studio_deploy.deploy(req, network, wallet);
    let mut digest_ok = true;
    let mut sealed_digest: Option<String> = None;
    while let Some(ev) = stream.next().await {
        match ev {
            comet_proto::StudioDeployEvent::Done {
                ok,
                record,
                error,
            } => {
                if !digest_ok {
                    relay.publish(
                        "deploy.done",
                        serde_json::json!({
                            "ok": false,
                            "error": "digest_mismatch_after_gate",
                            "record": record,
                        }),
                    );
                    continue;
                }
                let deployed_addr = record.as_ref().map(|r| r.address.clone());
                relay.publish(
                    "deploy.done",
                    serde_json::json!({
                        "ok": ok,
                        "record": record,
                        "error": error,
                    }),
                );
                // Attach deployed address onto sealed artifact for Fill-from-snapshot.
                if ok {
                    relay.publish(
                        "artifact.sealed",
                        studio_interact.sealed_for_relay(
                            module,
                            sealed_digest.as_deref(),
                            deployed_addr.as_deref(),
                        ),
                    );
                }
            }
            comet_proto::StudioDeployEvent::Gate { ok, output } => {
                if ok
                    && let Some(expected) = expected_digest.as_deref()
                    && output != expected
                {
                    digest_ok = false;
                    relay.publish(
                        "executor.refused",
                        serde_json::json!({
                            "reason": "digest_mismatch",
                            "expected": expected,
                            "got": output,
                        }),
                    );
                }
                let digests = if ok {
                    serde_json::json!({
                        "outputSetDigest": output,
                        "raw": output,
                        "certified": true,
                    })
                } else {
                    serde_json::json!({
                        "raw": output,
                        "certified": false,
                    })
                };
                relay.publish(
                    "gate.done",
                    serde_json::json!({
                        "ok": ok,
                        "output": output,
                        "digests": digests,
                    }),
                );
                if ok && digest_ok {
                    sealed_digest = Some(output.clone());
                    relay.publish(
                        "artifact.sealed",
                        studio_interact.sealed_for_relay(module, Some(&output), None),
                    );
                }
            }
            _ => {}
        }
    }
}

#[allow(dead_code)]
fn mirror_launch_event(relay: &StudioRelay, event: &comet_proto::StudioLaunchRunEvent) {
    match event {
        comet_proto::StudioLaunchRunEvent::Draft { event, .. } => {
            if let comet_proto::StudioDraftEvent::Done {
                ok: true,
                module,
                source,
                lane,
                ..
            } = event
            {
                relay.publish(
                    "draft.ready",
                    serde_json::json!({
                        "lane": lane,
                        "module": module,
                        "source": source,
                    }),
                );
            }
        }
        comet_proto::StudioLaunchRunEvent::Gate { event, .. } => match event {
            comet_proto::StudioGateEvent::Started { .. } => {
                relay.publish("gate.start", serde_json::json!({}));
            }
            comet_proto::StudioGateEvent::Done { ok, digest, .. } => {
                relay.publish(
                    "gate.done",
                    serde_json::json!({
                        "ok": ok,
                        "digests": digest,
                    }),
                );
            }
            _ => {}
        },
        comet_proto::StudioLaunchRunEvent::Done {
            ok,
            module,
            artifacts,
            digest,
            exhausted,
            ..
        } => {
            if *ok {
                relay.publish(
                    "artifact.sealed",
                    serde_json::json!({
                        "module": module,
                        "outputSetDigest": digest.output_set_digest,
                        "files": artifacts,
                    }),
                );
            } else if *exhausted {
                relay.note("repair exhausted");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Data directory (default `~/.comet-native`, dev `~/.comet-native-dev`).
    pub data_dir: PathBuf,
    /// Edge base URL.
    pub edge_url: String,
    /// Bearer for edge room joins; `None` runs fully offline (sync disabled).
    pub edge_token: Option<String>,
    /// Localhost IPC port for the UI.
    pub ipc_port: u16,
    /// Harness for doc-command runs on chats without a workspace `config` row.
    pub default_harness: HarnessId,
    /// Workspace-doc org (`ws/{orgId}` room). `None` = `$COMET_ORG_ID` or the dev default.
    /// In WorkOS mode the signed-in session's org wins.
    pub org_id: Option<String>,
    /// WorkOS client id — enables real auth; `None` = dev mode (bearer = `edge_token`).
    pub workos_client_id: Option<String>,
}

/// The assembled engine core — also constructible without the IPC server for tests
/// and the in-process (headed) mode.
pub struct EngineCore {
    pub sessions: SessionsEngine,
    pub doc_host: DocHost,
    pub workspace: WorkspaceHost,
    pub registry: Arc<HarnessRegistry>,
    pub repos: Repos,
    pub terminals: Terminals,
    pub diff_sync: CheckoutDiffSync,
    pub spaces_sync: SpacesSync,
    pub studio_gate: StudioGate,
    pub studio_draft: DraftRunner,
    pub studio_launch: StudioLaunchRunner,
    pub studio_store: StudioStore,
    pub network_store: NetworkStore,
    pub wallet_store: WalletStore,
    pub deploy_store: DeployStore,
    pub studio_deploy: StudioDeployer,
    pub studio_interact: StudioInteract,
    pub studio_preview: StudioPreview,
    pub wallet_connect: WalletConnectBridge,
    pub studio_relay: StudioRelay,
    pub data_dir: PathBuf,
    pub template_store: TemplateStore,
    pub uploads: Uploads,
    pub agent_accounts: AgentAccounts,
    pub device_id: String,
    /// Auth service (attached by [`Engine::run`]; a lazy dev-mode instance otherwise).
    auth: std::sync::Mutex<Option<Auth>>,
    /// Peer link cache for `targetDeviceId` routing (attached when edge+auth are ready).
    links: std::sync::Mutex<Option<Arc<comet_rpc::LinkCache>>>,
    /// Release checker (attached by [`Engine::assemble_runtime`]) — the
    /// UpdateStatus stream + ApplyUpdate.
    updater: std::sync::Mutex<Option<comet_update::Updater>>,
    /// Exclusive data-dir lock — held for the engine's lifetime (single-instance).
    _instance_lock: InstanceLock,
}

impl EngineCore {
    /// Open stores under `data_dir`, wire sessions ⇄ doc host ⇄ workspace host, and
    /// recover stale journals from a previous crash. Identity comes from
    /// `$COMET_ORG_ID` / `$COMET_USER_ID` (dev defaults `dev-org` / `dev-user`);
    /// use [`Self::assemble_with_identity`] to pass one explicitly.
    pub fn assemble(
        data_dir: &Path,
        registry: Arc<HarnessRegistry>,
        default_harness: HarnessId,
        edge: Option<EdgeConfig>,
    ) -> Result<Self, EngineError> {
        let org_id = env_or("COMET_ORG_ID", DEFAULT_ORG_ID);
        let user_id = env_or("COMET_USER_ID", DEFAULT_USER_ID);
        Self::assemble_with_identity(data_dir, registry, default_harness, edge, &org_id, &user_id)
    }

    pub fn assemble_with_identity(
        data_dir: &Path,
        registry: Arc<HarnessRegistry>,
        default_harness: HarnessId,
        edge: Option<EdgeConfig>,
        org_id: &str,
        user_id: &str,
    ) -> Result<Self, EngineError> {
        std::fs::create_dir_all(data_dir)?;
        // Single-instance guard: two engines on one data dir would race the
        // SQLite snapshots + journals. Taken before any store opens or the IPC
        // port binds; held (and kernel-released on crash) for the engine's life.
        let lock = InstanceLock::acquire(data_dir)?;
        let device_id = load_or_create_device_id(data_dir)?;
        // This device's harness enablement (Settings → Agents) rides the
        // engine data dir — per-device, like the CLI installs it gates.
        registry.load_prefs(data_dir);
        // Identity-scoped storage: snapshots, the command ledger, and run
        // journals live under `orgs/{orgId}/{userId}/` so switching accounts or
        // orgs on one machine never reuses another identity's cached docs.
        let org_dir = data_dir
            .join("orgs")
            .join(sanitize_path_id(org_id))
            .join(sanitize_path_id(user_id));
        let store = Arc::new(DocsStore::open(&org_dir)?);
        let journal = Arc::new(RunJournal::open(org_dir.join("journals"))?);
        let sessions = SessionsEngine::new(device_id.clone(), journal, registry.clone());
        let doc_host = DocHost::new(
            store.clone(),
            DocHostConfig {
                device_id: device_id.clone(),
                default_harness,
                edge: edge.clone(),
            },
        );
        let workspace = WorkspaceHost::open(
            store,
            WorkspaceHostConfig {
                device_id: device_id.clone(),
                device_name: local_device_name(),
                platform: std::env::consts::OS.to_string(),
                org_id: org_id.to_string(),
                user_id: user_id.to_string(),
                edge: edge.clone(),
            },
        )?;
        doc_host.set_workspace(workspace.clone());
        doc_host.set_sessions(sessions.clone());
        sessions.set_doc_host(doc_host.clone());
        match sessions.recover_stale() {
            Ok(0) => {}
            Ok(recovered) => tracing::info!(recovered, "stale sessions recovered on boot"),
            Err(err) => tracing::error!(error = %err, "stale-session recovery failed"),
        }
        doc_host.spawn_transcript_salvage(org_dir.join("journals"));
        let repos = Repos::new(data_dir, &device_id);
        let terminals = Terminals::new();
        let uploads = Uploads::new(data_dir, edge.clone());
        let agent_accounts = AgentAccounts::new(AgentAccountsConfig::detect(data_dir));
        sessions.set_titles(TitleGenerator::new(
            workspace.clone(),
            registry.clone(),
            repos.clone(),
        ));
        let diff_sync = CheckoutDiffSync::start(repos.clone(), workspace.clone(), &device_id, edge);
        // Turn starts snapshot the checkout tree — the "Latest turn" diff base.
        let turn_diff = diff_sync.clone();
        sessions.set_turn_listener(Arc::new(move |chat_id, cwd| {
            turn_diff.note_turn_start(chat_id, cwd);
        }));
        let spaces_sync = SpacesSync::start(repos.clone(), workspace.clone(), &device_id);
        let studio_config = crate::studio::GateConfig {
            paths: crate::studio::StudioPaths {
                inbox_root: Some(data_dir.join("studio").join("inbox")),
                ..crate::studio::StudioPaths::default()
            },
            ..crate::studio::GateConfig::default()
        };
        let studio_gate = StudioGate::new(studio_config.clone());
        let studio_draft = DraftRunner::new(registry.clone(), studio_config);
        let studio_launch = StudioLaunchRunner::new(studio_draft.clone(), studio_gate.clone());
        let studio_store = StudioStore::new(data_dir);
        let network_store = NetworkStore::new(data_dir);
        let wallet_store = WalletStore::new(data_dir);
        let deploy_store = DeployStore::new(data_dir);
        let inbox_root = data_dir.join("studio").join("inbox");
        let wallet_connect = WalletConnectBridge::new();
        let studio_deploy = StudioDeployer::new(
            studio_gate.clone(),
            inbox_root.clone(),
            deploy_store.clone(),
            wallet_connect.clone(),
            wallet_store.secrets().clone(),
        );
        let studio_interact = StudioInteract::new(
            inbox_root,
            wallet_connect.clone(),
            wallet_store.secrets().clone(),
        );
        let studio_preview = StudioPreview::new();
        let studio_relay = StudioRelay::new();
        studio_relay.set_default_device(&device_id);
        let template_store = TemplateStore::new(Vec::<PathBuf>::new());
        Ok(Self {
            sessions,
            doc_host,
            workspace,
            registry,
            repos,
            terminals,
            diff_sync,
            spaces_sync,
            studio_gate,
            studio_draft,
            studio_launch,
            studio_store,
            network_store,
            wallet_store,
            deploy_store,
            studio_deploy,
            studio_interact,
            studio_preview,
            wallet_connect,
            studio_relay,
            data_dir: data_dir.to_path_buf(),
            template_store,
            uploads,
            agent_accounts,
            device_id,
            auth: std::sync::Mutex::new(None),
            links: std::sync::Mutex::new(None),
            updater: std::sync::Mutex::new(None),
            _instance_lock: lock,
        })
    }

    /// Attach the auth service (before building the RPC service / relays).
    pub fn set_auth(&self, auth: Auth) {
        *self
            .auth
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(auth);
    }

    /// The attached auth service, or a lazily-created dev-mode one (in-process embeds
    /// that never wired WorkOS still answer AuthStatus honestly).
    pub fn auth(&self) -> Auth {
        let mut slot = self
            .auth
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        slot.get_or_insert_with(|| {
            let dev_user = std::env::var("COMET_EDGE_TOKEN")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "dev-user".into());
            let mut config = AuthConfig::new("http://localhost:27640", std::env::temp_dir());
            config.dev_user_id = dev_user;
            Auth::new(config)
        })
        .clone()
    }

    /// Attach the peer link cache — enables `targetDeviceId` routing and [`Self::dial_device`].
    pub fn set_links(&self, links: Arc<comet_rpc::LinkCache>) {
        *self
            .links
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(links);
    }

    pub fn links(&self) -> Option<Arc<comet_rpc::LinkCache>> {
        self.links
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Attach the release checker (before building the RPC service).
    pub fn set_updater(&self, updater: comet_update::Updater) {
        *self
            .updater
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(updater);
    }

    pub fn updater(&self) -> Option<comet_update::Updater> {
        self.updater
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// A live RPC client to another device's engine through its relay DO (the router's
    /// dial seam). Cached per device; invalidated + re-dialed on failure.
    pub async fn dial_device(
        &self,
        device_id: &str,
    ) -> Result<Arc<comet_rpc::RpcClient>, EngineError> {
        let links = self
            .links()
            .ok_or_else(|| EngineError::Other("peer links unavailable (offline)".into()))?;
        links
            .client(device_id)
            .await
            .map_err(|e| EngineError::Other(e.to_string()))
    }

    /// Start hosting our device room: serve the full RPC surface to relay clients and
    /// warm-open chat docs on nudges (§7 cold-chat command delivery). The token source
    /// re-reads auth on every (re)dial, so token refreshes take effect at reconnect.
    pub fn start_host_relay(&self, edge_url: &str) -> comet_rpc::HostRelay {
        let auth = self.auth();
        let config =
            comet_rpc::HostRelayConfig::new(edge_url, self.device_id.clone(), Arc::new(auth));
        let doc_host = self.doc_host.clone();
        let on_nudge: comet_rpc::NudgeHandler = Arc::new(move |chat_id: String| {
            // Opening the doc joins its room + syncs; drain fires on the change
            // subscription — the command executes with no standing per-chat socket.
            match doc_host.open(&chat_id) {
                Ok(_) => tracing::info!(chat = %chat_id, "nudge: chat doc opened"),
                Err(err) => {
                    tracing::warn!(chat = %chat_id, error = %err, "nudge: open failed")
                }
            }
        });
        comet_rpc::HostRelay::spawn(config, self.rpc_service(), on_nudge)
    }

    /// Start the Cloudflare relay client (hosted Worker by default).
    /// Web prompts become Sessions runs (skill + MCP via enrich_sessions_run_request).
    pub fn boot_studio_relay(&self) {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            if let Some(mut cmds) = self.studio_relay.start_from_env(&self.device_id, &self.data_dir)
            {
                let relay = self.studio_relay.clone();
                let sessions = self.sessions.clone();
                let doc_host = self.doc_host.clone();
                let default_harness = std::env::var("PROOFSHIP_DEFAULT_HARNESS")
                    .ok()
                    .map(|s| parse_relay_harness(Some(&s)))
                    .unwrap_or(HarnessId::Codex);
                let network_store = self.network_store.clone();
                let wallet_store = self.wallet_store.clone();
                let studio_deploy = self.studio_deploy.clone();
                let studio_interact = self.studio_interact.clone();
                tokio::spawn(async move {
                    while let Some(cmd) = cmds.recv().await {
                        if let Some(id) = cmd.id.as_deref() {
                            relay.ack(id);
                        }
                        let chat_id = cmd
                            .chat_id
                            .clone()
                            .or_else(|| std::env::var("PROOFSHIP_RELAY_CHAT_ID").ok())
                            .unwrap_or_else(|| "proofship-relay".into());
                        match cmd.kind {
                            crate::studio::RelayCommandKind::Prompt => {
                                let nl = cmd.nl.unwrap_or_default();
                                let harness = cmd
                                    .lane
                                    .as_deref()
                                    .map(|l| parse_relay_harness(Some(l)))
                                    .unwrap_or(default_harness);
                                if let Err(err) = doc_host.open(&chat_id) {
                                    relay.note(&format!("relay open chat failed: {err}"));
                                    continue;
                                }
                                relay.publish(
                                    "session.user",
                                    serde_json::json!({ "text": nl, "chatId": chat_id, "harness": harness }),
                                );
                                let cwd = std::env::var("PROOFSHIP_RELAY_CWD").unwrap_or_else(|_| {
                                    crate::repos::home_dir().to_string_lossy().into_owned()
                                });
                                let request = comet_proto::RunRequest {
                                    prompt: nl,
                                    harness: Some(harness),
                                    model: None,
                                    reasoning: Some(comet_proto::ReasoningLevel::High),
                                    model_options: serde_json::Map::new(),
                                    cwd,
                                    sandbox: comet_proto::SandboxLevel::WorkspaceWrite,
                                    auto_approve: true,
                                    resume: None,
                                    attachments: Vec::new(),
                                    mcp_servers: Vec::new(),
                                };
                                let Ok((_hist, mut rx)) = sessions.subscribe(&chat_id, 0) else {
                                    relay.note("relay subscribe failed");
                                    continue;
                                };
                                match sessions
                                    .dispatch(&chat_id, harness, request, None)
                                    .await
                                {
                                    Ok(_run_id) => {
                                        while let Ok(ev) = rx.recv().await {
                                            mirror_agent_event(&relay, &ev.event);
                                            if matches!(
                                                ev.event,
                                                comet_proto::AgentEvent::Done { .. }
                                            ) {
                                                break;
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        relay.publish(
                                            "session.done",
                                            serde_json::json!({
                                                "ok": false,
                                                "error": err.to_string(),
                                            }),
                                        );
                                    }
                                }
                            }
                            crate::studio::RelayCommandKind::Steer => {
                                let nl = cmd.nl.unwrap_or_default();
                                match sessions.steer(&chat_id, &nl, None).await {
                                    Ok(_) => relay.note("steer delivered"),
                                    Err(err) => relay.note(&format!("steer failed: {err}")),
                                }
                            }
                            crate::studio::RelayCommandKind::Cancel => {
                                match sessions.interrupt(&chat_id).await {
                                    Ok(true) => relay.note("run interrupted"),
                                    Ok(false) => relay.note("no live run to cancel"),
                                    Err(err) => relay.note(&format!("cancel failed: {err}")),
                                }
                            }
                            crate::studio::RelayCommandKind::Deploy => {
                                handle_relay_deploy(
                                    &relay,
                                    &network_store,
                                    &wallet_store,
                                    &studio_deploy,
                                    &studio_interact,
                                    cmd.network_id.as_deref(),
                                    cmd.module.as_deref(),
                                    cmd.digest.as_deref(),
                                )
                                .await;
                            }
                        }
                    }
                });
            }
        });
    }

    pub fn rpc_service(&self) -> Arc<EngineRpc> {
        self.boot_studio_relay();
        let mut rpc = EngineRpc::new(
            self.sessions.clone(),
            self.doc_host.clone(),
            self.workspace.clone(),
            self.registry.clone(),
            self.repos.clone(),
            self.terminals.clone(),
            self.diff_sync.clone(),
            self.uploads.clone(),
            self.agent_accounts.clone(),
            self.studio_gate.clone(),
            self.studio_draft.clone(),
            self.studio_launch.clone(),
            self.studio_store.clone(),
            self.network_store.clone(),
            self.wallet_store.clone(),
            self.deploy_store.clone(),
            self.studio_deploy.clone(),
            self.studio_interact.clone(),
            self.studio_preview.clone(),
            self.wallet_connect.clone(),
            self.studio_relay.clone(),
            self.template_store.clone(),
        )
        .with_auth(self.auth());
        if let Some(links) = self.links() {
            rpc = rpc.with_links(links);
        }
        if let Some(updater) = self.updater() {
            rpc = rpc.with_updater(updater);
        }
        Arc::new(rpc)
    }

    /// Graceful teardown: settle live runs (streaming entries stamped `aborted`),
    /// kill live PTYs, stamp our workspace `lastSeenAt`, and flush every open doc
    /// snapshot.
    pub async fn shutdown(&self) {
        self.sessions.shutdown().await;
        self.terminals.shutdown();
        self.agent_accounts.shutdown();
        self.studio_preview.stop().await;
        self.wallet_connect.stop().await;
        self.doc_host.flush_all();
        self.workspace.shutdown();
    }
}

pub struct Engine {
    pub config: EngineConfig,
}

/// A fully assembled identity-scoped engine plus the relay handle whose lifetime
/// keeps this device reachable. Used by both the headless server and the headed
/// in-process engine so their production authentication paths cannot diverge.
pub struct EngineRuntime {
    core: EngineCore,
    _host_relay: Option<comet_rpc::HostRelay>,
}

impl EngineRuntime {
    pub fn core(&self) -> &EngineCore {
        &self.core
    }

    pub async fn shutdown(&self) {
        self.core.shutdown().await;
    }
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
    }

    /// Resolve the shared dev/WorkOS auth configuration for headed and headless
    /// modes. Production callers pass the baked WorkOS client id; explicit dev
    /// bearers still opt into the local dev identity.
    pub async fn build_auth(config: &EngineConfig) -> Auth {
        let mut auth_config = AuthConfig::new(config.edge_url.clone(), config.data_dir.clone());
        auth_config.workos_client_id = config.workos_client_id.clone();
        if let Ok(base) = std::env::var("COMET_WORKOS_API_BASE")
            && !base.trim().is_empty()
        {
            auth_config.workos_api_base = base;
        }
        auth_config.callback_port = Some(
            std::env::var("COMET_CALLBACK_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(27641),
        );
        if let Some(token) = &config.edge_token {
            auth_config.dev_user_id = token.clone();
        }
        Auth::detect(auth_config).await
    }

    /// Open the identity-scoped stores and online transports for an auth session
    /// that is already ready. The headed UI waits behind its sign-in gate before
    /// calling this; headless mode waits on the terminal flow.
    pub async fn assemble_runtime(
        config: &EngineConfig,
        auth: Auth,
    ) -> anyhow::Result<EngineRuntime> {
        let online = (auth.workos_enabled() || config.edge_token.is_some())
            && auth.access_token().await.is_some();
        let device_id = load_or_create_device_id(&config.data_dir)?;
        let edge = online.then(|| {
            EdgeConfig::new(config.edge_url.clone(), Arc::new(auth.clone())).with_device(device_id)
        });

        let dev_token_org = config
            .edge_token
            .as_deref()
            .and_then(|t| t.split_once('@'))
            .map(|(_, org)| org.to_string())
            .filter(|s| !s.is_empty());
        let org_id = auth
            .state()
            .org_id()
            .map(str::to_string)
            .or(dev_token_org)
            .or(config.org_id.clone())
            .unwrap_or_else(|| env_or("COMET_ORG_ID", DEFAULT_ORG_ID));
        let user_id = auth
            .user_id()
            .unwrap_or_else(|| env_or("COMET_USER_ID", DEFAULT_USER_ID));
        let core = EngineCore::assemble_with_identity(
            &config.data_dir,
            Arc::new(default_registry()),
            config.default_harness,
            edge.clone(),
            &org_id,
            &user_id,
        )?;
        core.set_auth(auth.clone());
        // Release checker: polls {edge}/releases on a 6h cadence; headless
        // installs with COMET_AUTO_UPDATE=1 apply + restart themselves — gated
        // on quiescence so a restart never lands under a live run or open PTY.
        let quiescent: comet_update::QuiescentCheck = {
            let sessions = core.sessions.clone();
            let terminals = core.terminals.clone();
            Arc::new(move || !sessions.any_active() && !terminals.any_open())
        };
        core.set_updater(comet_update::Updater::spawn(
            config.edge_url.clone(),
            Some(quiescent),
        ));
        tracing::info!(device_id = %core.device_id, "engine core assembled");

        let host_relay = edge.as_ref().map(|edge| {
            let links = comet_rpc::LinkCache::new(comet_rpc::LinkCacheConfig::new(
                edge.url.clone(),
                Arc::new(auth.clone()),
            ));
            let links_for_presence = links.clone();
            core.workspace
                .set_peer_alive_hook(Arc::new(move |device_id: &str| {
                    links_for_presence.reset_cooldown(device_id);
                }));
            core.set_links(links);
            core.start_host_relay(&edge.url)
        });

        Ok(EngineRuntime {
            core,
            _host_relay: host_relay,
        })
    }

    /// Run until ctrl-c: auth (dev or WorkOS), sessions engine + doc host + command
    /// executor, IPC server, and — when edge+auth are ready — the device-room host
    /// relay + peer link cache (targetDeviceId routing).
    pub async fn run(self) -> anyhow::Result<()> {
        let config = self.config;
        tracing::info!(data_dir = %config.data_dir.display(), "engine starting");

        std::fs::create_dir_all(&config.data_dir)?;
        let auth = Self::build_auth(&config).await;
        let _refresh_loop = auth.spawn_refresh_loop();

        // WorkOS mode: gate edge features on a signed-in, org-scoped session. A TTY
        // gets the interactive paste-code flow; a service manager (systemd/launchd)
        // fails fast with a "run `comet login`" error instead of hanging on a prompt.
        if auth.workos_enabled() {
            terminal_sign_in(&auth).await?;
        }

        let runtime = Self::assemble_runtime(&config, auth).await?;

        // A daemon exists to serve this port, so a bind failure is fatal here —
        // unlike the headed app, which can still work over its in-process
        // transport (see `serve_ipc`).
        let server = serve_ipc(config.ipc_port, runtime.core().rpc_service()).await?;

        shutdown_signal().await?;
        tracing::info!("shutting down");
        server.abort();
        runtime.shutdown().await;
        Ok(())
    }
}

/// Ctrl-C or SIGTERM. systemd/launchd stop (and the auto-updater's service
/// restart) deliver SIGTERM — without catching it the daemon dies mid-write
/// and every stop takes the crash-recovery path instead of the graceful drain.
async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = sigterm.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await
    }
}

/// Serve the typed RPC on the localhost IPC port.
///
/// Both engines call this: the headless daemon, and the headed app's embedded
/// engine. That second case is the point — an embedded engine that keeps the
/// port to itself forces anyone wanting a second viewport (the terminal app) to
/// stop the desktop app, start a daemon, and start it again in the right order.
/// Serving here means any viewport can just attach.
///
/// Localhost only, exactly as before: this widens *which process* can serve the
/// port, not who can reach it.
pub async fn serve_ipc(
    port: u16,
    service: std::sync::Arc<dyn comet_rpc::RpcService>,
) -> std::io::Result<tokio::task::JoinHandle<()>> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    tracing::info!(port, "IPC server listening");
    Ok(tokio::spawn(comet_rpc::serve_ws_listener(
        listener, service,
    )))
}

/// Block until the WorkOS session is signed in AND org-scoped. On a TTY, print the
/// headless (paste-code) sign-in URL, read the pasted `state.code` from stdin, and
/// run workspace onboarding (create / auto-join / numbered picker). Off a TTY this
/// errors immediately — a daemon under systemd/launchd must load the session that
/// `comet login` persisted, never wait on a prompt nobody can see.
pub async fn terminal_sign_in(auth: &Auth) -> Result<(), EngineError> {
    use std::io::IsTerminal;
    let interactive = std::io::stdin().is_terminal();
    let mut state_rx = auth.watch_state();
    let mut stdin_reader: Option<tokio::task::JoinHandle<()>> = None;
    let mut org_reader: Option<tokio::task::JoinHandle<()>> = None;
    loop {
        let state = state_rx.borrow().clone();
        match state {
            AuthState::SignedIn { user, org_id } => {
                tracing::info!(email = %user.email, org = org_id.as_deref().unwrap_or("<none>"),
                    "auth: session ready");
                break;
            }
            AuthState::NeedsOrganization { user } => {
                if !interactive {
                    // No reader tasks have been spawned on this path (both spawns
                    // are TTY-gated), so an early return leaks nothing.
                    return Err(EngineError::Other(format!(
                        "signed in as {} but no workspace is selected — run `comet login` on this machine to pick one",
                        user.email
                    )));
                }
                if org_reader.is_none() {
                    // Workspace onboarding on the TTY (old comet's
                    // `backend login` flow): create if none, auto-join a
                    // single membership, numbered picker otherwise.
                    println!("Signed in as {}.", user.email);
                    org_reader = Some(tokio::spawn(run_org_onboarding(auth.clone())));
                }
            }
            AuthState::SignedOut => {
                if !interactive {
                    return Err(EngineError::Other(
                        "not signed in — run `comet login` on this machine first".into(),
                    ));
                }
                if stdin_reader.is_none() {
                    let url = auth.start_headless_sign_in();
                    println!("Sign in to Comet:\n\n  {url}\n");
                    println!("Then paste the code shown in the browser here and press enter.");
                    let auth = auth.clone();
                    stdin_reader = Some(tokio::spawn(async move {
                        loop {
                            let Some(line) = read_stdin_line().await else {
                                return;
                            };
                            let pasted = line.trim();
                            if pasted.is_empty() {
                                continue;
                            }
                            match auth.complete_sign_in(pasted).await {
                                Ok(()) => return,
                                Err(err) => println!("Sign-in failed: {err}"),
                            }
                        }
                    }));
                }
            }
        }
        if state_rx.changed().await.is_err() {
            break;
        }
    }
    if let Some(reader) = stdin_reader {
        reader.abort();
    }
    if let Some(reader) = org_reader {
        reader.abort();
    }
    Ok(())
}

/// One line from stdin (blocking read off the runtime). `None` = stdin closed.
async fn read_stdin_line() -> Option<String> {
    tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(0) | Err(_) => None, // EOF / error
            Ok(_) => Some(line),
        }
    })
    .await
    .ok()
    .flatten()
}

/// TTY workspace onboarding for an org-less session (ports old comet's
/// `backend login` flow): no memberships → prompt a name and create; exactly
/// one → auto-join; several → numbered picker. Success flips the auth state to
/// `SignedIn`, which ends [`wait_for_sign_in`]'s wait (and aborts this task).
async fn run_org_onboarding(auth: Auth) {
    let orgs = match auth.list_orgs().await {
        Ok(orgs) => orgs,
        Err(err) => {
            println!(
                "Could not list workspaces ({err}) — create or select one from the Comet UI to continue."
            );
            return;
        }
    };
    match orgs.len() {
        0 => {
            println!("No workspaces yet — name your new workspace and press enter:");
            loop {
                let Some(line) = read_stdin_line().await else {
                    return;
                };
                let name = line.trim();
                if name.is_empty() {
                    continue;
                }
                match auth.create_org(name).await {
                    Ok(()) => return,
                    Err(err) => println!("Creating workspace failed: {err}"),
                }
            }
        }
        1 => {
            let only = &orgs[0];
            println!("Joining workspace \"{}\"…", only.name);
            if let Err(err) = auth.select_org(&only.organization_id).await {
                println!("Joining workspace failed: {err}");
            }
        }
        _ => {
            println!("\nYour workspaces:");
            for (index, org) in orgs.iter().enumerate() {
                println!("  {}. {}", index + 1, org.name);
            }
            println!("Pick a workspace [1-{}]:", orgs.len());
            loop {
                let Some(line) = read_stdin_line().await else {
                    return;
                };
                let choice = line
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .and_then(|n| n.checked_sub(1))
                    .and_then(|index| orgs.get(index));
                let Some(org) = choice else {
                    println!("Pick a workspace [1-{}]:", orgs.len());
                    continue;
                };
                match auth.select_org(&org.organization_id).await {
                    Ok(()) => return,
                    Err(err) => println!("Joining workspace failed: {err}"),
                }
            }
        }
    }
}

/// Best-effort human name for this device's registry row (hostname).
fn local_device_name() -> String {
    std::env::var("COMET_DEVICE_NAME")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown-device".to_string())
}

/// Trimmed env var or the given default.
fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Filesystem-safe form of an org/user id (path segments for `orgs/{org}/{user}/`).
fn sanitize_path_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Stable per-installation device id, persisted at `{data_dir}/device-id`.
fn load_or_create_device_id(data_dir: &Path) -> Result<String, EngineError> {
    let path = data_dir.join("device-id");
    match std::fs::read_to_string(&path) {
        Ok(id) if !id.trim().is_empty() => Ok(id.trim().to_string()),
        Ok(_) | Err(_) => {
            let id = new_id();
            std::fs::write(&path, &id)?;
            Ok(id)
        }
    }
}
