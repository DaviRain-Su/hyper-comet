//! Studio right-pane Preview: local HTML dapp + system browser.
//!
//! Start serves localhost HTML and opens the host browser (gpui has no reliable
//! in-pane WebView). Rich embedded dapp UI belongs in the web app later.
//! The pane keeps an ABI mirror for no-arg views.

use gpui::{
    AnyElement, App, Context, Entity, FocusHandle, Render, SharedString, Task, Window, div,
    prelude::*, px,
};

use comet_abi::{AbiFormFn, AbiFormSchema, schema_from_abi_json};
use comet_proto::{
    DeploymentRecord, DeploymentsResponse, EvmNetwork, NetworksResponse, StudioAbiRequest,
    StudioAbiResponse, StudioCallKind, StudioCallRequest, StudioCallResponse, StudioCandidate,
    StudioCandidatesResponse, StudioDeployEvent, StudioDeployRequest, StudioPreviewStartRequest,
    StudioPreviewStatus, WalletAccount, WalletSource, WalletsResponse,
};
use comet_rpc::methods;

use crate::composer::ComposerInput;
use crate::state::AppState;
use crate::theme::Theme;

struct CtorField {
    name: String,
    sol_type: String,
    input: Entity<ComposerInput>,
}

pub struct StudioPreviewPane {
    state: Entity<AppState>,
    focus: FocusHandle,
    deployments: Vec<DeploymentRecord>,
    networks: Vec<EvmNetwork>,
    wallets: Vec<WalletAccount>,
    candidates: Vec<StudioCandidate>,
    selected: Option<DeploymentRecord>,
    selected_candidate: Option<String>,
    selected_network_id: Option<String>,
    selected_wallet_id: Option<String>,
    preview: Option<StudioPreviewStatus>,
    schema: Option<AbiFormSchema>,
    ctor: Option<AbiFormFn>,
    ctor_fields: Vec<CtorField>,
    call_output: Option<String>,
    error: Option<String>,
    deploy_note: Option<String>,
    load_task: Option<Task<()>>,
    action_task: Option<Task<()>>,
    call_task: Option<Task<()>>,
    ctor_task: Option<Task<()>>,
    deploy_task: Option<Task<()>>,
}

impl StudioPreviewPane {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let mut pane = Self {
            state,
            focus: cx.focus_handle(),
            deployments: Vec::new(),
            networks: Vec::new(),
            wallets: Vec::new(),
            candidates: Vec::new(),
            selected: None,
            selected_candidate: None,
            selected_network_id: None,
            selected_wallet_id: None,
            preview: None,
            schema: None,
            ctor: None,
            ctor_fields: Vec::new(),
            call_output: None,
            error: None,
            deploy_note: None,
            load_task: None,
            action_task: None,
            call_task: None,
            ctor_task: None,
            deploy_task: None,
        };
        pane.refresh(cx);
        pane
    }

    pub fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        self.load_task = Some(cx.spawn(async move |this, cx| {
            let deployments = engine
                .client()
                .call(methods::STUDIO_DEPLOYMENTS, serde_json::json!({}))
                .await;
            let networks = engine
                .client()
                .call(methods::STUDIO_NETWORKS, serde_json::json!({}))
                .await;
            let status = engine
                .client()
                .call(methods::STUDIO_PREVIEW_STATUS, serde_json::json!({}))
                .await;
            let candidates = engine
                .client()
                .call(methods::STUDIO_CANDIDATES, serde_json::json!({}))
                .await;
            let wallets = engine
                .client()
                .call(methods::STUDIO_WALLETS, serde_json::json!({}))
                .await;
            this.update(cx, |pane, cx| {
                match deployments {
                    Ok(value) => match serde_json::from_value::<DeploymentsResponse>(value) {
                        Ok(resp) => {
                            // Prefer X Layer deployments in the preview list.
                            let mut deps = resp.deployments;
                            deps.sort_by(|a, b| {
                                let a_x = a.network_id.starts_with("xlayer") as i32;
                                let b_x = b.network_id.starts_with("xlayer") as i32;
                                b_x.cmp(&a_x)
                            });
                            pane.deployments = deps;
                        }
                        Err(err) => pane.error = Some(err.to_string()),
                    },
                    Err(err) => pane.error = Some(err.to_string()),
                }
                if let Ok(value) = networks
                    && let Ok(resp) = serde_json::from_value::<NetworksResponse>(value)
                {
                    pane.networks = resp.networks;
                    if pane.selected_network_id.as_ref().is_none_or(|id| {
                        !pane.networks.iter().any(|n| n.id == *id)
                    }) {
                        pane.selected_network_id = pane
                            .networks
                            .iter()
                            .find(|n| n.id == "xlayer-testnet")
                            .map(|n| n.id.clone())
                            .or_else(|| pane.networks.first().map(|n| n.id.clone()));
                    }
                }
                if let Ok(value) = status
                    && let Ok(resp) = serde_json::from_value::<StudioPreviewStatus>(value)
                {
                    pane.preview = if resp.url.is_some() { Some(resp) } else { None };
                }
                if let Ok(value) = candidates
                    && let Ok(resp) = serde_json::from_value::<StudioCandidatesResponse>(value)
                {
                    pane.candidates = resp.candidates;
                    if pane.selected_candidate.as_ref().is_none_or(|module| {
                        !pane.candidates.iter().any(|c| c.module == *module)
                    }) {
                        pane.selected_candidate =
                            pane.candidates.first().map(|c| c.module.clone());
                    }
                    if pane.selected_candidate.is_some() {
                        pane.load_ctor_abi(cx);
                    }
                }
                if let Ok(value) = wallets
                    && let Ok(resp) = serde_json::from_value::<WalletsResponse>(value)
                {
                    pane.wallets = resp.wallets;
                    if pane.selected_wallet_id.as_ref().is_none_or(|id| {
                        !pane
                            .wallets
                            .iter()
                            .any(|w| w.id == *id && wallet_can_sign(w))
                    }) {
                        pane.selected_wallet_id = pane
                            .wallets
                            .iter()
                            .find(|w| w.source == WalletSource::DevEnvKey)
                            .or_else(|| pane.wallets.iter().find(|w| wallet_can_sign(w)))
                            .map(|w| w.id.clone());
                    }
                }
                if pane.selected.is_none() {
                    pane.selected = pane
                        .deployments
                        .iter()
                        .find(|d| d.network_id.starts_with("xlayer"))
                        .cloned()
                        .or_else(|| pane.deployments.first().cloned());
                    if pane.selected.is_some() {
                        pane.load_schema(cx);
                    }
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn load_schema(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let Some(module) = self.selected.as_ref().map(|d| d.module.clone()) else {
            return;
        };
        let req = StudioAbiRequest { module };
        self.call_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::STUDIO_ABI, serde_json::to_value(req).unwrap())
                .await;
            this.update(cx, |pane, cx| {
                pane.call_task = None;
                match result {
                    Ok(value) => match serde_json::from_value::<StudioAbiResponse>(value) {
                        Ok(resp) => match schema_from_abi_json(&resp.abi_json) {
                            Ok(schema) => {
                                pane.schema = Some(schema);
                                pane.error = None;
                            }
                            Err(err) => pane.error = Some(err.to_string()),
                        },
                        Err(err) => pane.error = Some(err.to_string()),
                    },
                    Err(err) => pane.error = Some(err.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn load_ctor_abi(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let Some(module) = self.selected_candidate.clone() else {
            self.ctor = None;
            self.ctor_fields.clear();
            return;
        };
        let req = StudioAbiRequest { module };
        self.ctor_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::STUDIO_ABI, serde_json::to_value(req).unwrap())
                .await;
            this.update(cx, |pane, cx| {
                pane.ctor_task = None;
                pane.ctor_fields.clear();
                pane.ctor = None;
                if let Ok(value) = result
                    && let Ok(resp) = serde_json::from_value::<StudioAbiResponse>(value)
                    && let Ok(schema) = schema_from_abi_json(&resp.abi_json)
                {
                    pane.apply_ctor_schema(schema, cx);
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn apply_ctor_schema(&mut self, schema: AbiFormSchema, cx: &mut Context<Self>) {
        self.ctor_fields.clear();
        self.ctor = schema.constructor.clone();
        if let Some(ctor) = &schema.constructor {
            for param in &ctor.inputs {
                let input = cx.new(|cx| {
                    ComposerInput::new(format!("{} ({})", param.name, param.sol_type), cx)
                });
                self.ctor_fields.push(CtorField {
                    name: param.name.clone(),
                    sol_type: param.sol_type.clone(),
                    input,
                });
            }
        }
    }

    fn select_candidate(&mut self, module: String, cx: &mut Context<Self>) {
        if self.selected_candidate.as_deref() == Some(module.as_str()) {
            return;
        }
        self.selected_candidate = Some(module);
        self.load_ctor_abi(cx);
        cx.notify();
    }

    fn ctor_payload(&self, cx: &Context<Self>) -> (String, Vec<String>) {
        let sig = ctor_sig_value(self.ctor.as_ref());
        if self.ctor_fields.is_empty() {
            return (sig, Vec::new());
        }
        let args = self
            .ctor_fields
            .iter()
            .map(|field| field.input.read(cx).text().trim().to_string())
            .collect();
        (sig, args)
    }

    fn can_deploy(&self) -> bool {
        self.selected_candidate.is_some()
            && self.selected_network_id.is_some()
            && self
                .selected_wallet_id
                .as_deref()
                .and_then(|id| self.wallets.iter().find(|w| w.id == id))
                .is_some_and(wallet_can_sign)
            && self.deploy_task.is_none()
    }

    fn start_deploy(&mut self, cx: &mut Context<Self>) {
        if !self.can_deploy() {
            return;
        }
        let Some(candidate) = self
            .selected_candidate
            .as_ref()
            .and_then(|module| self.candidates.iter().find(|c| c.module == *module))
            .cloned()
        else {
            self.deploy_note = Some("No ProgramV1 source yet".into());
            cx.notify();
            return;
        };
        let Some(network_id) = self.selected_network_id.clone() else {
            self.deploy_note = Some("Pick a network".into());
            cx.notify();
            return;
        };
        let Some(wallet_id) = self.selected_wallet_id.clone() else {
            self.deploy_note = Some("Pick a signable wallet in Settings → Wallets".into());
            cx.notify();
            return;
        };
        let (ctor_sig, ctor_args) = self.ctor_payload(cx);
        if !self.ctor_fields.is_empty() && ctor_args.iter().any(|a| a.trim().is_empty()) {
            self.deploy_note = Some("Fill every constructor argument before deploying".into());
            cx.notify();
            return;
        }
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            self.deploy_note = Some("Engine not connected".into());
            cx.notify();
            return;
        };
        let params = match serde_json::to_value(StudioDeployRequest {
            module: candidate.module.clone(),
            source: candidate.source,
            network_id,
            wallet_id,
            ctor_sig,
            ctor_args,
            launch_id: None,
            project_id: None,
        }) {
            Ok(params) => params,
            Err(err) => {
                self.deploy_note = Some(format!("Studio deploy: {err}"));
                cx.notify();
                return;
            }
        };
        self.deploy_note = Some("Deploying…".into());
        self.error = None;
        cx.notify();
        self.deploy_task = Some(cx.spawn(async move |this, cx| {
            let stream = engine
                .client()
                .subscribe(methods::STUDIO_DEPLOY, params)
                .await;
            match stream {
                Ok(mut rx) => {
                    while let Some(value) = rx.recv().await {
                        match serde_json::from_value::<StudioDeployEvent>(value) {
                            Ok(event) => {
                                this.update(cx, |pane, cx| {
                                    pane.apply_deploy_event(event, cx);
                                })
                                .ok();
                            }
                            Err(err) => {
                                this.update(cx, |pane, cx| {
                                    pane.deploy_note =
                                        Some(format!("Studio deploy event: {err}"));
                                    cx.notify();
                                })
                                .ok();
                            }
                        }
                    }
                }
                Err(err) => {
                    this.update(cx, |pane, cx| {
                        pane.deploy_note = Some(format!("Deploy failed: {err}"));
                        cx.notify();
                    })
                    .ok();
                }
            }
            this.update(cx, |pane, cx| {
                pane.deploy_task = None;
                cx.notify();
            })
            .ok();
        }));
    }

    fn apply_deploy_event(&mut self, event: StudioDeployEvent, cx: &mut Context<Self>) {
        match event {
            StudioDeployEvent::Started { network_id } => {
                self.deploy_note = Some(format!("Deploying to {network_id}…"));
            }
            StudioDeployEvent::Gate { ok, output } => {
                self.deploy_note = Some(if ok {
                    format!("Gate passed {output}")
                } else {
                    format!("Gate refused deploy: {output}")
                });
            }
            StudioDeployEvent::Sending { rpc_url } => {
                self.deploy_note = Some(format!("Sending via {rpc_url}"));
            }
            StudioDeployEvent::Done { ok, record, error } => {
                if ok {
                    if let Some(record) = record {
                        let note = format!(
                            "Deployed {}  tx {}",
                            truncate_addr(&record.address),
                            truncate_addr(&record.tx_hash)
                        );
                        self.deployments.retain(|d| d.id != record.id);
                        self.deployments.insert(0, record.clone());
                        self.selected = Some(record);
                        self.schema = None;
                        self.call_output = None;
                        self.load_schema(cx);
                        self.deploy_note = Some(note);
                    } else {
                        self.deploy_note = Some("Deployed".into());
                    }
                } else {
                    self.deploy_note = Some(error.unwrap_or_else(|| "Deploy failed".into()));
                }
            }
        }
        cx.notify();
    }

    fn render_deploy_card(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let can_deploy = self.can_deploy();
        let selected_module = self.selected_candidate.clone();
        let selected_network = self.selected_network_id.clone();
        let selected_wallet = self.selected_wallet_id.clone();

        let candidate_chips: Vec<AnyElement> = self
            .candidates
            .iter()
            .enumerate()
            .map(|(ix, cand)| {
                let module = cand.module.clone();
                let active = selected_module.as_deref() == Some(module.as_str());
                let label = if cand.certified {
                    format!("{} · certified", cand.module)
                } else {
                    cand.module.clone()
                };
                toggle_chip(("preview-cand", ix), label, active, false, theme)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_candidate(module.clone(), cx);
                    }))
                    .into_any_element()
            })
            .collect();

        let network_chips: Vec<AnyElement> = self
            .networks
            .iter()
            .enumerate()
            .map(|(ix, net)| {
                let id = net.id.clone();
                let active = selected_network.as_deref() == Some(id.as_str());
                let label = format!("{} ({})", net.name, net.chain_id);
                toggle_chip(("preview-net", ix), label, active, false, theme)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_network_id = Some(id.clone());
                        cx.notify();
                    }))
                    .into_any_element()
            })
            .collect();

        let wallet_chips: Vec<AnyElement> = self
            .wallets
            .iter()
            .enumerate()
            .map(|(ix, wallet)| {
                let id = wallet.id.clone();
                let disabled = !wallet_can_sign(wallet);
                let active = selected_wallet.as_deref() == Some(id.as_str());
                let label = format!("{} · {}", wallet.label, wallet_source_label(wallet.source));
                let chip = toggle_chip(("preview-wal", ix), label, active && !disabled, disabled, theme);
                if disabled {
                    chip.into_any_element()
                } else {
                    chip.on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_wallet_id = Some(id.clone());
                        cx.notify();
                    }))
                    .into_any_element()
                }
            })
            .collect();

        div()
            .rounded(px(10.0))
            .border_1()
            .border_color(theme.border)
            .p(px(14.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.text)
                    .child("Deploy"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.text_muted)
                    .child(SharedString::from(
                        "Deploy re-runs the machine gate. Keys stay in Settings → Wallets (env-key / WalletConnect). Never in this pane.",
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_muted)
                            .child("Program"),
                    )
                    .when(self.candidates.is_empty(), |el| {
                        el.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_muted)
                                .child(SharedString::from(
                                    "No ProgramV1 source yet. After Sessions MCP gate, stage `studio-inbox/{Module}.lean`, or keep a launch draft.",
                                )),
                        )
                    })
                    .when(!self.candidates.is_empty(), |el| {
                        el.child(div().flex().flex_wrap().gap(px(6.0)).children(candidate_chips))
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_muted)
                            .child("Network"),
                    )
                    .child(div().flex().flex_wrap().gap(px(6.0)).children(network_chips)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_muted)
                            .child("Wallet"),
                    )
                    .when(self.wallets.is_empty(), |el| {
                        el.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_muted)
                                .child("Add a testnet env-key wallet in Settings → Wallets."),
                        )
                    })
                    .when(!self.wallets.is_empty(), |el| {
                        el.child(div().flex().flex_wrap().gap(px(6.0)).children(wallet_chips))
                    }),
            )
            .children(self.ctor_fields.iter().map(|field| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_muted)
                            .child(SharedString::from(format!(
                                "{} ({})",
                                field.name, field.sol_type
                            ))),
                    )
                    .child(field.input.clone())
            }))
            .child(
                if can_deploy {
                    action_chip("Deploy", theme)
                        .on_click(cx.listener(|this, _, _, cx| this.start_deploy(cx)))
                        .into_any_element()
                } else {
                    toggle_chip("preview-deploy", "Deploy".into(), false, true, theme)
                        .into_any_element()
                },
            )
            .when_some(self.deploy_note.clone(), |el, note| {
                el.child(
                    div()
                        .font_family("Geist Mono")
                        .text_size(px(11.0))
                        .text_color(theme.text_dim)
                        .child(SharedString::from(note)),
                )
            })
            .into_any_element()
    }

    fn start_preview(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let Some(dep) = self.selected.clone() else {
            self.error = Some("Select a deployment first".into());
            cx.notify();
            return;
        };
        self.load_schema(cx);
        let req = StudioPreviewStartRequest {
            module: dep.module.clone(),
            address: dep.address.clone(),
            network_id: dep.network_id.clone(),
        };
        self.error = None;
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(
                    methods::STUDIO_PREVIEW_START,
                    serde_json::to_value(req).unwrap(),
                )
                .await;
            this.update(cx, |pane, cx| {
                match result {
                    Ok(value) => match serde_json::from_value::<StudioPreviewStatus>(value) {
                        Ok(status) => {
                            if let Some(url) = status.url.clone() {
                                cx.open_url(&url);
                            }
                            pane.preview = Some(status);
                        }
                        Err(err) => pane.error = Some(err.to_string()),
                    },
                    Err(err) => pane.error = Some(err.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn stop_preview(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let _ = engine
                .client()
                .call(methods::STUDIO_PREVIEW_STOP, serde_json::json!({}))
                .await;
            this.update(cx, |pane, cx| {
                pane.preview = None;
                cx.notify();
            })
            .ok();
        }));
    }

    fn open_browser(&mut self, cx: &mut Context<Self>) {
        if let Some(url) = self.preview.as_ref().and_then(|p| p.url.clone()) {
            cx.open_url(&url);
        }
    }

    fn call_view(&mut self, signature: String, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let Some(dep) = self.selected.clone() else {
            return;
        };
        let params = match serde_json::to_value(StudioCallRequest {
            network_id: dep.network_id,
            address: dep.address,
            signature: signature.clone(),
            args: Vec::new(),
            kind: StudioCallKind::View,
            wallet_id: None,
        }) {
            Ok(v) => v,
            Err(err) => {
                self.call_output = Some(err.to_string());
                cx.notify();
                return;
            }
        };
        self.call_output = Some(format!("calling {signature}…"));
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let result = engine.client().call(methods::STUDIO_CALL, params).await;
            this.update(cx, |pane, cx| {
                pane.action_task = None;
                pane.call_output = Some(match result {
                    Ok(value) => match serde_json::from_value::<StudioCallResponse>(value) {
                        Ok(resp) => {
                            if resp.ok {
                                format!("{signature} → {}", resp.output)
                            } else {
                                format!("{signature} failed: {}", resp.output)
                            }
                        }
                        Err(err) => err.to_string(),
                    },
                    Err(err) => err.to_string(),
                });
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn network_label(&self, network_id: &str) -> String {
        self.networks
            .iter()
            .find(|n| n.id == network_id)
            .map(|n| format!("{} ({})", n.name, n.chain_id))
            .unwrap_or_else(|| network_id.to_string())
    }
}

impl Render for StudioPreviewPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let url = self
            .preview
            .as_ref()
            .and_then(|p| p.url.clone())
            .unwrap_or_default();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.bg)
            .child(
                div()
                    .px(px(14.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.text)
                                    .child("Preview"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(6.0))
                                    .child(
                                        action_chip("Refresh", &theme).on_click(cx.listener(
                                            |this, _, _, cx| this.refresh(cx),
                                        )),
                                    )
                                    .child(
                                        action_chip("Start", &theme).on_click(cx.listener(
                                            |this, _, _, cx| this.start_preview(cx),
                                        )),
                                    )
                                    .when(self.preview.is_some(), |el| {
                                        el.child(
                                            action_chip("Open", &theme).on_click(cx.listener(
                                                |this, _, _, cx| this.open_browser(cx),
                                            )),
                                        )
                                        .child(
                                            action_chip("Stop", &theme).on_click(cx.listener(
                                                |this, _, _, cx| this.stop_preview(cx),
                                            )),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_muted)
                            .child(SharedString::from(
                                "Start serves localhost HTML and opens your system browser. In-pane ABI mirror for quick views; rich embed belongs in the web app.",
                            )),
                    ),
            )
            .child(
                div()
                    .id("studio-preview-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(14.0))
                    .py(px(12.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .when_some(self.error.clone(), |el, err| {
                        el.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.danger)
                                .child(SharedString::from(err)),
                        )
                    })
                    .child(self.render_deploy_card(&theme, cx))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.text_muted)
                                    .child("Deployment"),
                            )
                            .children(self.deployments.iter().enumerate().map(|(ix, dep)| {
                                let active = self
                                    .selected
                                    .as_ref()
                                    .is_some_and(|s| s.address == dep.address && s.module == dep.module);
                                let dep = dep.clone();
                                let label = format!(
                                    "{} · {}",
                                    dep.module,
                                    truncate_addr(&dep.address)
                                );
                                let meta = self.network_label(&dep.network_id);
                                div()
                                    .id(("preview-dep", ix))
                                    .px(px(10.0))
                                    .py(px(8.0))
                                    .rounded(px(8.0))
                                    .border_1()
                                    .border_color(if active {
                                        theme.accent
                                    } else {
                                        theme.border
                                    })
                                    .bg(if active {
                                        theme.accent.opacity(0.12)
                                    } else {
                                        theme.bg
                                    })
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.selected = Some(dep.clone());
                                        this.schema = None;
                                        this.call_output = None;
                                        this.start_preview(cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.text)
                                            .child(SharedString::from(label)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.text_muted)
                                            .child(SharedString::from(meta)),
                                    )
                                    .into_any_element()
                            })),
                    )
                    .when(!url.is_empty(), |el| {
                        let url = url.clone();
                        el.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(theme.text_muted)
                                        .child("Local preview URL"),
                                )
                                .child(
                                    div()
                                        .px(px(10.0))
                                        .py(px(8.0))
                                        .rounded(px(8.0))
                                        .border_1()
                                        .border_color(theme.border)
                                        .font_family("Geist Mono")
                                        .text_size(px(12.0))
                                        .text_color(theme.text)
                                        .child(SharedString::from(url.clone())),
                                )
                                .child(
                                    // Browser-chrome mock: the HTML page is the source of truth;
                                    // gpui shows status until WebView embed lands.
                                    div()
                                        .rounded(px(10.0))
                                        .border_1()
                                        .border_color(theme.border)
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .px(px(10.0))
                                                .py(px(8.0))
                                                .bg(theme.surface)
                                                .border_b_1()
                                                .border_color(theme.border)
                                                .flex()
                                                .items_center()
                                                .gap(px(8.0))
                                                .child(
                                                    div()
                                                        .size(px(8.0))
                                                        .rounded_full()
                                                        .bg(theme.text_muted.opacity(0.5)),
                                                )
                                                .child(
                                                    div()
                                                        .size(px(8.0))
                                                        .rounded_full()
                                                        .bg(theme.text_muted.opacity(0.5)),
                                                )
                                                .child(
                                                    div()
                                                        .size(px(8.0))
                                                        .rounded_full()
                                                        .bg(theme.text_muted.opacity(0.5)),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .px(px(8.0))
                                                        .py(px(4.0))
                                                        .rounded(px(6.0))
                                                        .bg(theme.bg)
                                                        .text_size(px(11.0))
                                                        .text_color(theme.text_muted)
                                                        .child(SharedString::from(url)),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .p(px(16.0))
                                                .flex()
                                                .flex_col()
                                                .gap(px(8.0))
                                                .child(
                                                    div()
                                                        .text_size(px(18.0))
                                                        .font_weight(gpui::FontWeight::MEDIUM)
                                                        .text_color(theme.text)
                                                        .child("ProofShip"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(14.0))
                                                        .font_weight(gpui::FontWeight::MEDIUM)
                                                        .text_color(theme.text)
                                                        .child(SharedString::from(
                                                            self.selected
                                                                .as_ref()
                                                                .map(|d| d.module.clone())
                                                                .unwrap_or_else(|| "Dapp".into()),
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .font_family("Geist Mono")
                                                        .text_size(px(11.0))
                                                        .text_color(theme.text_dim)
                                                        .child(SharedString::from(
                                                            self.selected
                                                                .as_ref()
                                                                .map(|d| truncate_addr(&d.address))
                                                                .unwrap_or_default(),
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.0))
                                                        .text_color(theme.text_muted)
                                                        .child(
                                                            "Live HTML opens in your system browser. In-app mirror below for quick views.",
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .gap(px(8.0))
                                                        .child(
                                                            action_chip("Open in browser", &theme)
                                                                .on_click(cx.listener(
                                                                    |this, _, _, cx| {
                                                                        this.open_browser(cx)
                                                                    },
                                                                )),
                                                        )
                                                        .child(
                                                            action_chip("Restart", &theme).on_click(
                                                                cx.listener(|this, _, _, cx| {
                                                                    this.start_preview(cx)
                                                                }),
                                                            ),
                                                        ),
                                                ),
                                        ),
                                ),
                        )
                    })
                    .when_some(self.schema.as_ref(), |el, schema| {
                        el.child(
                            div()
                                .rounded(px(10.0))
                                .border_1()
                                .border_color(theme.border)
                                .p(px(14.0))
                                .flex()
                                .flex_col()
                                .gap(px(10.0))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.text)
                                        .child("In-app mirror"),
                                )
                                .when(!schema.views.is_empty(), |el| {
                                    el.child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(6.0))
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(theme.text_muted)
                                                    .child("Views (no-arg)"),
                                            )
                                            .child(
                                                div().flex().flex_wrap().gap(px(6.0)).children(
                                                    schema
                                                        .views
                                                        .iter()
                                                        .filter(|f| f.inputs.is_empty())
                                                        .enumerate()
                                                        .map(|(ix, func)| {
                                                            let sig = func.signature();
                                                            action_chip_owned(sig.clone(), &theme)
                                                                .id(("preview-view", ix))
                                                                .on_click(cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        this.call_view(
                                                                            sig.clone(),
                                                                            cx,
                                                                        );
                                                                    },
                                                                ))
                                                        }),
                                                ),
                                            ),
                                    )
                                })
                                .when(!schema.events.is_empty(), |el| {
                                    el.child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(theme.text_muted)
                                                    .child("Events"),
                                            )
                                            .children(schema.events.iter().map(|ev| {
                                                div()
                                                    .font_family("Geist Mono")
                                                    .text_size(px(11.0))
                                                    .text_color(theme.text_dim)
                                                    .child(SharedString::from(ev.signature()))
                                            })),
                                    )
                                })
                                .when_some(self.call_output.clone(), |el, out| {
                                    el.child(
                                        div()
                                            .font_family("Geist Mono")
                                            .text_size(px(11.0))
                                            .text_color(theme.text)
                                            .child(SharedString::from(out)),
                                    )
                                }),
                        )
                    })
                    .when(url.is_empty() && self.selected.is_some(), |el| {
                        el.child(
                            div()
                                .rounded(px(10.0))
                                .border_1()
                                .border_color(theme.border)
                                .p(px(16.0))
                                .flex()
                                .flex_col()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .text_color(theme.text)
                                        .child("Preview idle"),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(theme.text_muted)
                                        .child(
                                            "Click Start (or re-select a deployment) to serve the local dapp HTML.",
                                        ),
                                ),
                        )
                    })
                    .when(self.deployments.is_empty(), |el| {
                        el.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_muted)
                                .child(
                                    "No deployments yet. Pass the gate, deploy, then select a deployment here.",
                                ),
                        )
                    }),
            )
    }
}

fn truncate_addr(address: &str) -> String {
    if address.len() > 12 {
        format!("{}…{}", &address[..6], &address[address.len() - 4..])
    } else {
        address.to_string()
    }
}

fn action_chip(label: &'static str, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(label)
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(theme.border)
        .text_size(px(11.0))
        .text_color(theme.text)
        .cursor_pointer()
        .hover(|s| s.bg(theme.surface))
        .child(SharedString::from(label))
}

fn action_chip_owned(label: String, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(label.clone()))
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(theme.border)
        .text_size(px(11.0))
        .text_color(theme.text)
        .cursor_pointer()
        .hover(|s| s.bg(theme.surface))
        .child(SharedString::from(label))
}

fn wallet_can_sign(wallet: &WalletAccount) -> bool {
    match wallet.source {
        WalletSource::Watch => false,
        WalletSource::WalletConnect => !wallet.address.trim().is_empty(),
        WalletSource::DevEnvKey => true,
    }
}

fn ctor_sig_value(ctor: Option<&AbiFormFn>) -> String {
    match ctor {
        Some(ctor) if !ctor.inputs.is_empty() => ctor.signature(),
        _ => "-".into(),
    }
}

fn wallet_source_label(source: WalletSource) -> &'static str {
    match source {
        WalletSource::Watch => "Watch",
        WalletSource::DevEnvKey => "Env key",
        WalletSource::WalletConnect => "WalletConnect",
    }
}

fn toggle_chip(
    id: impl Into<gpui::ElementId>,
    label: String,
    active: bool,
    disabled: bool,
    theme: &Theme,
) -> gpui::Stateful<gpui::Div> {
    let border = if active { theme.accent } else { theme.border };
    let bg = if active {
        theme.accent.opacity(0.12)
    } else {
        theme.bg
    };
    let text = if disabled {
        theme.text_muted.opacity(0.45)
    } else {
        theme.text
    };
    let chip = div()
        .id(id)
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(border)
        .bg(bg)
        .text_size(px(11.0))
        .text_color(text)
        .child(SharedString::from(label));
    if disabled {
        chip
    } else {
        chip.cursor_pointer().hover(|s| s.bg(theme.surface))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use comet_abi::{AbiFormParam, AbiWidget};

    #[test]
    fn watch_wallets_cannot_sign() {
        let watch = WalletAccount {
            id: "w".into(),
            label: "Watch".into(),
            address: "0xabc".into(),
            source: WalletSource::Watch,
            env_key_name: None,
        };
        let env = WalletAccount {
            id: "e".into(),
            label: "Dev".into(),
            address: String::new(),
            source: WalletSource::DevEnvKey,
            env_key_name: Some("PF_XLAYER_KEY".into()),
        };
        let wc_empty = WalletAccount {
            id: "c0".into(),
            label: "WC".into(),
            address: String::new(),
            source: WalletSource::WalletConnect,
            env_key_name: None,
        };
        let wc = WalletAccount {
            id: "c1".into(),
            label: "WC".into(),
            address: "0xabc".into(),
            source: WalletSource::WalletConnect,
            env_key_name: None,
        };
        assert!(!wallet_can_sign(&watch));
        assert!(wallet_can_sign(&env));
        assert!(!wallet_can_sign(&wc_empty));
        assert!(wallet_can_sign(&wc));
    }

    #[test]
    fn ctor_sig_is_dash_without_inputs() {
        assert_eq!(ctor_sig_value(None), "-");
        let empty = AbiFormFn {
            name: String::new(),
            state_mutability: "nonpayable".into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
        assert_eq!(ctor_sig_value(Some(&empty)), "-");
    }

    #[test]
    fn ctor_sig_uses_signature_when_inputs_present() {
        let ctor = AbiFormFn {
            name: String::new(),
            state_mutability: "nonpayable".into(),
            inputs: vec![
                AbiFormParam {
                    name: "unlock".into(),
                    sol_type: "uint64".into(),
                    widget: AbiWidget::Uint { bits: 64 },
                },
                AbiFormParam {
                    name: "amt".into(),
                    sol_type: "uint64".into(),
                    widget: AbiWidget::Uint { bits: 64 },
                },
            ],
            outputs: Vec::new(),
        };
        assert_eq!(ctor_sig_value(Some(&ctor)), "constructor(uint64,uint64)");
    }
}
