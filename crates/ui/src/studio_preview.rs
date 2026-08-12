//! Studio right-pane Preview: local HTML dapp + Open in browser.

use gpui::{
    App, Context, Entity, FocusHandle, Render, SharedString, Task, Window, div, prelude::*, px,
};

use comet_proto::{
    DeploymentRecord, DeploymentsResponse, EvmNetwork, NetworksResponse, StudioPreviewStartRequest,
    StudioPreviewStatus,
};
use comet_rpc::methods;

use crate::state::AppState;
use crate::theme::Theme;

pub struct StudioPreviewPane {
    state: Entity<AppState>,
    focus: FocusHandle,
    deployments: Vec<DeploymentRecord>,
    networks: Vec<EvmNetwork>,
    selected: Option<DeploymentRecord>,
    preview: Option<StudioPreviewStatus>,
    error: Option<String>,
    load_task: Option<Task<()>>,
    action_task: Option<Task<()>>,
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
            error: None,
            load_task: None,
            action_task: None,
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
                        Ok(resp) => pane.deployments = resp.deployments,
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
                    pane.selected = pane.deployments.first().cloned();
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
        let req = StudioPreviewStartRequest {
            module: dep.module.clone(),
            address: dep.address.clone(),
            network_id: dep.network_id.clone(),
        };
        self.error = None;
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::STUDIO_PREVIEW_START, serde_json::to_value(req).unwrap())
                .await;
            this.update(cx, |pane, cx| {
                match result {
                    Ok(value) => match serde_json::from_value::<StudioPreviewStatus>(value) {
                        Ok(status) => {
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
                                "ABI → local dapp page (like Codex app preview). Open in browser for the live HTML UI.",
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
                                        cx.notify();
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
                                                        .text_size(px(12.0))
                                                        .text_color(theme.text_muted)
                                                        .child(
                                                            "Click Open to render the full ABI-driven frontend in your browser. In-app WebView comes next.",
                                                        ),
                                                )
                                                .child(
                                                    action_chip("Open in browser", &theme).on_click(
                                                        cx.listener(|this, _, _, cx| {
                                                            this.open_browser(cx)
                                                        }),
                                                    ),
                                                ),
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
                                    "No deployments yet. Pass the gate, deploy, then Start preview here.",
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
