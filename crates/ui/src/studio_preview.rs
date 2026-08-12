//! Studio right-pane Preview: local HTML dapp + dedicated WebView window.
//!
//! In-pane child WebView is unreliable with gpui (Linux/Wayland). Preview Start
//! opens a managed OS WebView via `proofship-webview` (wry) or Chromium `--app=`.
//! The pane keeps an ABI mirror for no-arg views.

use gpui::{
    App, Context, Entity, FocusHandle, Render, SharedString, Task, Window, div, prelude::*, px,
};

use comet_abi::{AbiFormSchema, schema_from_abi_json};
use comet_proto::{
    DeploymentRecord, DeploymentsResponse, EvmNetwork, NetworksResponse, StudioAbiRequest,
    StudioAbiResponse, StudioCallKind, StudioCallRequest, StudioCallResponse,
    StudioPreviewStartRequest, StudioPreviewStatus,
};
use comet_rpc::methods;

use crate::state::AppState;
use crate::studio_webview::StudioWebView;
use crate::theme::Theme;

pub struct StudioPreviewPane {
    state: Entity<AppState>,
    focus: FocusHandle,
    deployments: Vec<DeploymentRecord>,
    networks: Vec<EvmNetwork>,
    selected: Option<DeploymentRecord>,
    preview: Option<StudioPreviewStatus>,
    schema: Option<AbiFormSchema>,
    call_output: Option<String>,
    error: Option<String>,
    webview_note: Option<String>,
    webview: StudioWebView,
    load_task: Option<Task<()>>,
    action_task: Option<Task<()>>,
    call_task: Option<Task<()>>,
}

impl StudioPreviewPane {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let mut pane = Self {
            state,
            focus: cx.focus_handle(),
            deployments: Vec::new(),
            networks: Vec::new(),
            selected: None,
            preview: None,
            schema: None,
            call_output: None,
            error: None,
            webview_note: None,
            webview: StudioWebView::new(),
            load_task: None,
            action_task: None,
            call_task: None,
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
                }
                if let Ok(value) = status
                    && let Ok(resp) = serde_json::from_value::<StudioPreviewStatus>(value)
                {
                    pane.preview = if resp.url.is_some() { Some(resp) } else { None };
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
                            if let Some(url) = status.url.as_deref() {
                                match pane.webview.open(url) {
                                    Ok(backend) => {
                                        pane.webview_note = Some(format!(
                                            "WebView open via {}",
                                            backend.label()
                                        ));
                                    }
                                    Err(err) => {
                                        pane.webview_note = Some(err);
                                    }
                                }
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
        self.webview.stop();
        self.webview_note = None;
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

    fn open_webview(&mut self, cx: &mut Context<Self>) {
        let Some(url) = self.preview.as_ref().and_then(|p| p.url.clone()) else {
            self.webview_note = Some("Start preview first".into());
            cx.notify();
            return;
        };
        match self.webview.open(&url) {
            Ok(backend) => {
                self.webview_note = Some(format!("WebView open via {}", backend.label()));
                self.error = None;
            }
            Err(err) => {
                self.webview_note = Some(err.clone());
                // Last resort: system browser.
                cx.open_url(&url);
                self.error = Some(format!("{err} — opened system browser instead"));
            }
        }
        cx.notify();
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
                                            action_chip("WebView", &theme).on_click(cx.listener(
                                                |this, _, _, cx| this.open_webview(cx),
                                            )),
                                        )
                                        .child(
                                            action_chip("Browser", &theme).on_click(cx.listener(
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
                                "Start serves localhost HTML and opens a dedicated WebView window (wry or Chromium --app). ABI mirror stays in-pane.",
                            )),
                    )
                    .when_some(self.webview_note.clone(), |el, note| {
                        el.child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.text_dim)
                                .child(SharedString::from(note)),
                        )
                    }),
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
                                                            "Dedicated WebView window shows the live HTML dapp. In-app mirror below for quick views.",
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .gap(px(8.0))
                                                        .child(
                                                            action_chip("WebView", &theme).on_click(
                                                                cx.listener(|this, _, _, cx| {
                                                                    this.open_webview(cx)
                                                                }),
                                                            ),
                                                        )
                                                        .child(
                                                            action_chip("Browser", &theme).on_click(
                                                                cx.listener(|this, _, _, cx| {
                                                                    this.open_browser(cx)
                                                                }),
                                                            ),
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
