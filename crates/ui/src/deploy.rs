//! One-click deploy dialog: sealed gate artifacts under the session cwd →
//! pick network + wallet → sign the create tx → explorer link.
//!
//! Artifacts come from `DeployScan` (the agent's `pf_build` output); the
//! dialog never compiles anything itself — no gate pass, no artifact, no
//! deploy. Mounted by the shell as a modal from the session titlebar.

use gpui::{
    AnyElement, Context, Entity, EventEmitter, Render, SharedString, Subscription, Task, Window,
    div, prelude::*, px,
};

use comet_proto::{
    DeployArtifact, DeployScanResponse, DeploySendResponse, DeploymentRecord, EvmNetwork,
    NetworksResponse, WalletAccount, WalletSource, WalletsResponse,
};
use comet_rpc::methods;

use crate::composer::ComposerInput;
use crate::popover::{self, Loadable};
use crate::state::AppState;
use crate::theme::{Theme, hairline, ink};

/// Shell-facing events.
pub enum DeployEvent {
    Dismiss,
}

pub struct DeployPanel {
    state: Entity<AppState>,
    cwd: String,
    artifacts: Loadable<Vec<DeployArtifact>>,
    selected_artifact: usize,
    networks: Loadable<Vec<EvmNetwork>>,
    selected_network: usize,
    wallets: Loadable<Vec<WalletAccount>>,
    selected_wallet: usize,
    ctor_sig: Entity<ComposerInput>,
    ctor_args: Entity<ComposerInput>,
    deploying: bool,
    result: Option<DeploymentRecord>,
    error: Option<String>,
    tasks: Vec<Task<()>>,
    _subs: Vec<Subscription>,
}

impl EventEmitter<DeployEvent> for DeployPanel {}

impl DeployPanel {
    pub fn new(state: Entity<AppState>, cwd: String, cx: &mut Context<Self>) -> Self {
        let ctor_sig = cx.new(|cx| ComposerInput::new("constructor(uint64,…) — optional", cx));
        let ctor_args = cx.new(|cx| ComposerInput::new("args, comma separated", cx));
        let mut panel = Self {
            state,
            cwd,
            artifacts: Loadable::Idle,
            selected_artifact: 0,
            networks: Loadable::Idle,
            selected_network: 0,
            wallets: Loadable::Idle,
            selected_wallet: 0,
            ctor_sig,
            ctor_args,
            deploying: false,
            result: None,
            error: None,
            tasks: Vec::new(),
            _subs: Vec::new(),
        };
        panel.load(cx);
        panel
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        self.artifacts = Loadable::Loading;
        self.networks = Loadable::Loading;
        self.wallets = Loadable::Loading;

        let cwd = self.cwd.clone();
        let scan_engine = engine.clone();
        self.tasks.push(cx.spawn(async move |this, cx| {
            let result = scan_engine
                .client()
                .call(methods::DEPLOY_SCAN, serde_json::json!({ "cwd": cwd }))
                .await;
            this.update(cx, |panel, cx| {
                panel.artifacts = match result {
                    Ok(value) => match serde_json::from_value::<DeployScanResponse>(value) {
                        Ok(resp) => Loadable::Ready(resp.artifacts),
                        Err(err) => Loadable::Error(err.to_string()),
                    },
                    Err(err) => Loadable::Error(err.to_string()),
                };
                panel.selected_artifact = 0;
                cx.notify();
            })
            .ok();
        }));

        let networks_engine = engine.clone();
        self.tasks.push(cx.spawn(async move |this, cx| {
            let result = networks_engine
                .client()
                .call(methods::STUDIO_NETWORKS, serde_json::json!({}))
                .await;
            this.update(cx, |panel, cx| {
                panel.networks = match result {
                    Ok(value) => match serde_json::from_value::<NetworksResponse>(value) {
                        // Pluggable multi-chain: disabled networks never
                        // reach the picker (preflight would reject anyway).
                        Ok(resp) => Loadable::Ready(
                            resp.networks.into_iter().filter(|n| n.enabled).collect(),
                        ),
                        Err(err) => Loadable::Error(err.to_string()),
                    },
                    Err(err) => Loadable::Error(err.to_string()),
                };
                cx.notify();
            })
            .ok();
        }));

        self.tasks.push(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::STUDIO_WALLETS, serde_json::json!({}))
                .await;
            this.update(cx, |panel, cx| {
                panel.wallets = match result {
                    Ok(value) => match serde_json::from_value::<WalletsResponse>(value) {
                        // Watch-only rows cannot sign — drop them here so the
                        // picker never offers a dead end.
                        Ok(resp) => Loadable::Ready(
                            resp.wallets
                                .into_iter()
                                .filter(|w| w.source != WalletSource::Watch)
                                .collect(),
                        ),
                        Err(err) => Loadable::Error(err.to_string()),
                    },
                    Err(err) => Loadable::Error(err.to_string()),
                };
                cx.notify();
            })
            .ok();
        }));
    }

    fn deploy(&mut self, cx: &mut Context<Self>) {
        if self.deploying {
            return;
        }
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let Some(artifact) = self
            .artifacts
            .ready()
            .and_then(|a| a.get(self.selected_artifact))
            .cloned()
        else {
            self.error = Some("Pick a gate artifact first".into());
            cx.notify();
            return;
        };
        let Some(network) = self
            .networks
            .ready()
            .and_then(|n| n.get(self.selected_network))
            .cloned()
        else {
            self.error = Some("Pick a network".into());
            cx.notify();
            return;
        };
        let Some(wallet) = self
            .wallets
            .ready()
            .and_then(|w| w.get(self.selected_wallet))
            .cloned()
        else {
            self.error = Some("Pick a wallet (Settings → Wallets to add one)".into());
            cx.notify();
            return;
        };

        let ctor_sig = self.ctor_sig.read(cx).text().trim().to_string();
        let ctor_args: Vec<String> = self
            .ctor_args
            .read(cx)
            .text()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if ctor_sig.is_empty() && !ctor_args.is_empty() {
            self.error = Some("Constructor args given but no signature".into());
            cx.notify();
            return;
        }

        self.deploying = true;
        self.error = None;
        self.result = None;
        cx.notify();

        let request = serde_json::json!({
            "binPath": artifact.bin_path,
            "module": artifact.module,
            "networkId": network.id,
            "walletId": wallet.id,
            "ctorSig": ctor_sig,
            "ctorArgs": ctor_args,
            "digest": artifact.digest,
        });
        self.tasks.push(cx.spawn(async move |this, cx| {
            let result = engine.client().call(methods::DEPLOY_SEND, request).await;
            this.update(cx, |panel, cx| {
                panel.deploying = false;
                match result {
                    Ok(value) => match serde_json::from_value::<DeploySendResponse>(value) {
                        Ok(resp) => panel.result = Some(resp.record),
                        Err(err) => panel.error = Some(err.to_string()),
                    },
                    Err(err) => panel.error = Some(err.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn explorer_url(&self, cx: &Context<Self>) -> Option<String> {
        let record = self.result.as_ref()?;
        let _ = cx;
        let networks = self.networks.ready()?;
        let network = networks.iter().find(|n| n.id == record.network_id)?;
        let base = network.explorer_url.as_deref()?.trim_end_matches('/');
        Some(format!("{base}/address/{}", record.address))
    }

    fn section_label(theme: &Theme, label: &str) -> gpui::Div {
        div()
            .mt(px(14.0))
            .mb(px(6.0))
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.text_muted.opacity(0.7))
            .child(SharedString::from(label.to_string()))
    }

    fn pill(
        &self,
        theme: &Theme,
        id: String,
        label: String,
        selected: bool,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let base = div()
            .id(SharedString::from(id))
            .px(px(10.0))
            .py(px(5.0))
            .rounded(px(999.0))
            .text_size(px(12.0))
            .cursor_pointer()
            .border_1()
            .on_click(cx.listener(move |this, _, _, cx| {
                on_click(this, cx);
                cx.notify();
            }))
            .child(SharedString::from(label));
        if selected {
            base.border_color(theme.text.opacity(0.6))
                .bg(ink(0.10))
                .text_color(theme.text)
        } else {
            base.border_color(hairline(0.10))
                .text_color(theme.text_muted)
                .hover(|s| s.bg(ink(0.05)))
        }
        .into_any_element()
    }

    fn artifact_rows(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        match &self.artifacts {
            Loadable::Idle | Loadable::Loading => popover::dialog_body(theme, "Scanning…")
                .into_any_element(),
            Loadable::Error(err) => popover::dialog_body(theme, format!("Scan failed: {err}"))
                .text_color(theme.danger)
                .into_any_element(),
            Loadable::Ready(artifacts) if artifacts.is_empty() => popover::dialog_body(
                theme,
                "No sealed artifacts under this session. Ask the agent to run the \
                 ProofForge gate (pf_check → pf_build) first, then rescan.",
            )
            .into_any_element(),
            Loadable::Ready(artifacts) => {
                let selected = self.selected_artifact;
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .children(artifacts.iter().enumerate().map(|(ix, artifact)| {
                        let row = div()
                            .id(SharedString::from(format!("deploy-artifact-{ix}")))
                            .px(px(10.0))
                            .py(px(7.0))
                            .rounded(px(8.0))
                            .border_1()
                            .cursor_pointer()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected_artifact = ix;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.text)
                                    .child(SharedString::from(artifact.module.clone())),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(theme.text_muted.opacity(0.7))
                                    .child(SharedString::from(artifact.dir.clone())),
                            )
                            .when_some(artifact.digest.clone(), |el, digest| {
                                let short: String = digest.chars().take(10).collect();
                                el.child(
                                    div()
                                        .flex_none()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(5.0))
                                        .bg(ink(0.06))
                                        .text_size(px(10.0))
                                        .font_family(theme.font_mono.clone())
                                        .text_color(theme.text_muted)
                                        .child(SharedString::from(format!("{short}…"))),
                                )
                            });
                        if ix == selected {
                            row.border_color(theme.text.opacity(0.5)).bg(ink(0.06))
                        } else {
                            row.border_color(hairline(0.08))
                        }
                    }))
                    .into_any_element()
            }
        }
    }
}

impl Render for DeployPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();

        let network_pills: Vec<AnyElement> = match &self.networks {
            Loadable::Ready(networks) => {
                let selected = self.selected_network;
                networks
                    .iter()
                    .enumerate()
                    .map(|(ix, n)| {
                        self.pill(
                            &theme,
                            format!("deploy-net-{ix}"),
                            n.name.clone(),
                            ix == selected,
                            move |this, _| this.selected_network = ix,
                            cx,
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let wallet_pills: Vec<AnyElement> = match &self.wallets {
            Loadable::Ready(wallets) if wallets.is_empty() => vec![
                popover::dialog_body(&theme, "No signer wallets — add one in Settings → Wallets.")
                    .into_any_element(),
            ],
            Loadable::Ready(wallets) => {
                let selected = self.selected_wallet;
                wallets
                    .iter()
                    .enumerate()
                    .map(|(ix, w)| {
                        let tag = match w.source {
                            WalletSource::Local => "local",
                            WalletSource::DevEnvKey => "env",
                            WalletSource::WalletConnect => "wc",
                            WalletSource::Watch => "watch",
                        };
                        self.pill(
                            &theme,
                            format!("deploy-wallet-{ix}"),
                            format!("{} · {tag}", w.label),
                            ix == selected,
                            move |this, _| this.selected_wallet = ix,
                            cx,
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        let result_block = self.result.clone().map(|record| {
            let explorer = self.explorer_url(cx);
            div()
                .mt(px(12.0))
                .p(px(10.0))
                .rounded(px(8.0))
                .bg(ink(0.05))
                .border_1()
                .border_color(theme.success.opacity(0.35))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.success)
                        .child(SharedString::from(format!("{} deployed", record.module))),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_family(theme.font_mono.clone())
                        .text_color(theme.text_muted)
                        .child(SharedString::from(record.address.clone())),
                )
                .when_some(explorer, |el, url| {
                    el.child(
                        div()
                            .id("deploy-explorer-link")
                            .text_size(px(12.0))
                            .text_color(theme.text_muted)
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.text))
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.open_url(&url);
                            }))
                            .child(SharedString::from("Open in explorer ↗")),
                    )
                })
        });

        let deploy_label = if self.deploying { "Deploying…" } else { "Deploy" };

        popover::dialog_card(&theme)
            .w(px(460.0))
            .on_key_down(cx.listener(|_, ev: &gpui::KeyDownEvent, _, cx| {
                if ev.keystroke.key == "escape" {
                    cx.emit(DeployEvent::Dismiss);
                }
            }))
            .child(popover::dialog_title(&theme, "Deploy gate artifact"))
            .child(
                div().mt(px(4.0)).child(popover::dialog_body(
                    &theme,
                    format!("Sealed ProofForge build outputs under {}", self.cwd),
                )),
            )
            .child(Self::section_label(&theme, "ARTIFACT"))
            .child(self.artifact_rows(&theme, cx))
            .child(Self::section_label(&theme, "NETWORK"))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(6.0))
                    .children(network_pills),
            )
            .child(Self::section_label(&theme, "WALLET"))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(6.0))
                    .children(wallet_pills),
            )
            .child(Self::section_label(&theme, "CONSTRUCTOR (OPTIONAL)"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(popover::dialog_field(
                        self.ctor_sig.clone().into_any_element(),
                    ))
                    .child(popover::dialog_field(
                        self.ctor_args.clone().into_any_element(),
                    )),
            )
            .when_some(self.error.clone(), |el, message| {
                el.child(
                    div()
                        .mt(px(10.0))
                        .text_size(px(12.0))
                        .text_color(theme.danger)
                        .child(SharedString::from(message)),
                )
            })
            .when_some(result_block, |el, block| el.child(block))
            .child(
                div()
                    .mt(px(16.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        popover::btn_ghost(&theme, "Rescan", "deploy-rescan")
                            .id("deploy-rescan")
                            .on_click(cx.listener(|this, _, _, cx| this.load(cx))),
                    )
                    .child(div().flex_1())
                    .child(
                        popover::btn_ghost(&theme, "Close", "deploy-cancel")
                            .id("deploy-cancel")
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.emit(DeployEvent::Dismiss);
                            })),
                    )
                    .child(
                        popover::btn_primary(&theme, deploy_label)
                            .id("deploy-send")
                            .when(self.deploying, |el| el.opacity(0.6))
                            .on_click(cx.listener(|this, _, _, cx| this.deploy(cx))),
                    ),
            )
    }
}
