//! Settings → Networks: built-in X Layer presets plus custom EVM RPC entries.

use gpui::{
    AnyElement, Context, Entity, Render, SharedString, Subscription, Task, Window, div, prelude::*,
    px,
};

use comet_proto::{
    EvmNetwork, NetworksResponse, StudioRelayStatus, UpsertNetworkRequest,
};
use comet_rpc::methods;

use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::popover::{self, Loadable};
use crate::settings::widgets;
use crate::state::AppState;
use crate::theme::Theme;

struct NetworkDialog {
    /// `None` when adding a new network; `Some(id)` when editing.
    edit_id: Option<String>,
    edit_builtin: bool,
    name: Entity<ComposerInput>,
    chain_id: Entity<ComposerInput>,
    rpc_url: Entity<ComposerInput>,
    explorer_url: Entity<ComposerInput>,
    symbol: Entity<ComposerInput>,
    error: Option<String>,
    _events: Vec<Subscription>,
}

pub struct NetworksPage {
    state: Entity<AppState>,
    networks: Loadable<Vec<EvmNetwork>>,
    relay: Option<StudioRelayStatus>,
    dialog: Option<NetworkDialog>,
    error: Option<String>,
    load_task: Option<Task<()>>,
    action_task: Option<Task<()>>,
}

impl NetworksPage {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            state,
            networks: Loadable::Idle,
            relay: None,
            dialog: None,
            error: None,
            load_task: None,
            action_task: None,
        };
        page.load(cx);
        page
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        self.networks = Loadable::Loading;
        self.load_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::STUDIO_NETWORKS, serde_json::json!({}))
                .await;
            let relay = engine
                .client()
                .call(methods::STUDIO_RELAY_STATUS, serde_json::json!({}))
                .await;
            this.update(cx, |page, cx| {
                page.networks = match result {
                    Ok(value) => match serde_json::from_value::<NetworksResponse>(value) {
                        Ok(response) => Loadable::Ready(response.networks),
                        Err(err) => Loadable::Error(err.to_string()),
                    },
                    Err(err) => Loadable::Error(err.to_string()),
                };
                if let Ok(value) = relay {
                    page.relay = serde_json::from_value::<StudioRelayStatus>(value).ok();
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn render_relay_card(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let relay = self.relay.as_ref()?;
        let session = relay.session_id.clone();
        let web = relay.web_url.clone().unwrap_or_default();
        let base = relay.base.clone().unwrap_or_default();
        Some(
            div()
                .mb(px(16.0))
                .p(px(14.0))
                .rounded(px(10.0))
                .border_1()
                .border_color(theme.border)
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.text)
                        .child("Web relay"),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.text_muted)
                        .child(if relay.enabled {
                            SharedString::from(format!(
                                "This desktop is a UserExecutor on {base}. Session {session}."
                            ))
                        } else {
                            SharedString::from(
                                "Web relay is off (PROOFSHIP_RELAY=off). Web Sessions cannot reach this desktop.",
                            )
                        }),
                )
                .when(relay.enabled && !web.is_empty(), |el| {
                    let web_open = web.clone();
                    el.child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(
                                widgets::ghost_action(theme)
                                    .id("relay-open-web")
                                    .hover(|s| widgets::ghost_hover(theme, s))
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        cx.open_url(&web_open);
                                    }))
                                    .child(SharedString::from("Open web Sessions")),
                            ),
                    )
                    .child(
                        div()
                            .font_family("Geist Mono")
                            .text_size(px(11.0))
                            .text_color(theme.text_dim)
                            .child(SharedString::from(web)),
                    )
                })
                .into_any_element(),
        )
    }

    fn open_add(&mut self, cx: &mut Context<Self>) {
        self.open_dialog(None, None, cx);
    }

    fn open_edit(&mut self, network: EvmNetwork, cx: &mut Context<Self>) {
        self.open_dialog(Some(network.id.clone()), Some(network), cx);
    }

    fn open_dialog(
        &mut self,
        edit_id: Option<String>,
        existing: Option<EvmNetwork>,
        cx: &mut Context<Self>,
    ) {
        let name = cx.new(|cx| ComposerInput::new("Network name", cx));
        let chain_id = cx.new(|cx| ComposerInput::new("Chain ID", cx));
        let rpc_url = cx.new(|cx| ComposerInput::new("RPC URL", cx));
        let explorer_url = cx.new(|cx| ComposerInput::new("Explorer URL (optional)", cx));
        let symbol = cx.new(|cx| ComposerInput::new("Currency symbol", cx));
        let edit_builtin = existing.as_ref().is_some_and(|n| n.builtin);
        if let Some(network) = existing {
            name.update(cx, |input, cx| input.set_text(network.name, cx));
            chain_id.update(cx, |input, cx| {
                input.set_text(network.chain_id.to_string(), cx)
            });
            rpc_url.update(cx, |input, cx| input.set_text(network.rpc_url, cx));
            explorer_url.update(cx, |input, cx| {
                input.set_text(network.explorer_url.unwrap_or_default(), cx)
            });
            symbol.update(cx, |input, cx| input.set_text(network.currency_symbol, cx));
        }
        let mut events = Vec::new();
        for input in [&name, &chain_id, &rpc_url, &explorer_url, &symbol] {
            let input = input.clone();
            events.push(cx.subscribe(&input, |this: &mut Self, _, event, cx| {
                if matches!(event, ComposerInputEvent::Submitted) {
                    this.submit_dialog(cx);
                }
            }));
        }
        self.dialog = Some(NetworkDialog {
            edit_id,
            edit_builtin,
            name,
            chain_id,
            rpc_url,
            explorer_url,
            symbol,
            error: None,
            _events: events,
        });
        cx.notify();
    }

    fn submit_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = &self.dialog else {
            return;
        };
        let name = dialog.name.read(cx).text().trim().to_string();
        let chain_raw = dialog.chain_id.read(cx).text().trim().to_string();
        let rpc_url = dialog.rpc_url.read(cx).text().trim().to_string();
        let explorer_raw = dialog.explorer_url.read(cx).text().trim().to_string();
        let symbol = dialog.symbol.read(cx).text().trim().to_string();
        let edit_id = dialog.edit_id.clone();
        let edit_builtin = dialog.edit_builtin;

        if name.is_empty() {
            self.set_dialog_error("Name is required", cx);
            return;
        }
        let chain_id: u64 = match chain_raw.parse() {
            Ok(id) if id > 0 => id,
            _ => {
                self.set_dialog_error("Chain ID must be a positive number", cx);
                return;
            }
        };
        if !rpc_url.starts_with("http://") && !rpc_url.starts_with("https://") {
            self.set_dialog_error("RPC URL must start with http:// or https://", cx);
            return;
        }
        if symbol.is_empty() {
            self.set_dialog_error("Currency symbol is required", cx);
            return;
        }

        let id = edit_id.clone().unwrap_or_else(|| {
            let slug = slug_from_name(&name);
            if slug.is_empty() {
                uuid::Uuid::new_v4().to_string()
            } else {
                slug
            }
        });
        let explorer_url = if explorer_raw.is_empty() {
            None
        } else {
            Some(explorer_raw)
        };
        let network = EvmNetwork {
            id,
            name,
            chain_id,
            rpc_url,
            explorer_url,
            currency_symbol: symbol,
            builtin: edit_builtin,
        };
        self.dialog = None;
        self.upsert(network, cx);
    }

    fn set_dialog_error(&mut self, message: &str, cx: &mut Context<Self>) {
        if let Some(dialog) = &mut self.dialog {
            dialog.error = Some(message.to_string());
        }
        cx.notify();
    }

    fn upsert(&mut self, network: EvmNetwork, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let request = UpsertNetworkRequest { network };
        let params = match serde_json::to_value(request) {
            Ok(value) => value,
            Err(err) => {
                self.error = Some(err.to_string());
                cx.notify();
                return;
            }
        };
        self.error = None;
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::STUDIO_UPSERT_NETWORK, params)
                .await;
            this.update(cx, |page, cx| {
                match result {
                    Ok(value) => {
                        if let Ok(response) = serde_json::from_value::<NetworksResponse>(value) {
                            page.networks = Loadable::Ready(response.networks);
                        } else {
                            page.error = Some("Unexpected upsert response".into());
                        }
                    }
                    Err(err) => page.error = Some(err.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn remove(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        self.error = None;
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(
                    methods::STUDIO_REMOVE_NETWORK,
                    serde_json::json!({ "id": id }),
                )
                .await;
            this.update(cx, |page, cx| {
                match result {
                    Ok(value) => {
                        if let Ok(response) = serde_json::from_value::<NetworksResponse>(value) {
                            page.networks = Loadable::Ready(response.networks);
                        } else {
                            page.error = Some("Unexpected remove response".into());
                        }
                    }
                    Err(err) => page.error = Some(err.to_string()),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn render_dialog(
        &mut self,
        viewport: gpui::Size<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let theme = Theme::of(cx).clone();
        let dialog = self.dialog.as_ref()?;
        let title = if dialog.edit_id.is_some() {
            "Edit network"
        } else {
            "Add network"
        };
        let field = |label: &str, input: Entity<ComposerInput>| {
            div()
                .mt(px(12.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(widgets::field_label(&theme, label))
                .child(popover::dialog_field(input.into_any_element()))
        };
        let card = popover::dialog_card(&theme)
            .child(popover::dialog_title(&theme, title))
            .child(field("Name", dialog.name.clone()))
            .child(field("Chain ID", dialog.chain_id.clone()))
            .child(field("RPC URL", dialog.rpc_url.clone()))
            .child(field("Explorer URL", dialog.explorer_url.clone()))
            .child(field("Currency symbol", dialog.symbol.clone()))
            .when_some(dialog.error.clone(), |el, message| {
                el.child(
                    div()
                        .mt(px(12.0))
                        .text_size(px(12.0))
                        .text_color(theme.danger_muted)
                        .child(SharedString::from(message)),
                )
            })
            .child(
                div()
                    .mt(px(16.0))
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        popover::btn_ghost(&theme, "Cancel", "network-cancel")
                            .id("network-cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.dialog = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        popover::btn_primary(
                            &theme,
                            if dialog.edit_id.is_some() {
                                "Save"
                            } else {
                                "Add"
                            },
                        )
                        .id("network-save")
                        .on_click(cx.listener(|this, _, _, cx| this.submit_dialog(cx))),
                    ),
            )
            .into_any_element();
        Some(popover::modal("network-dialog", viewport, card))
    }

    fn rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = Theme::of(cx).clone();
        let Loadable::Ready(list) = &self.networks else {
            return Vec::new();
        };
        list.iter()
            .enumerate()
            .map(|(ix, network)| {
                let mut meta: Vec<AnyElement> = vec![
                    div()
                        .child(SharedString::from(format!("Chain {}", network.chain_id)))
                        .into_any_element(),
                    div()
                        .child(SharedString::from(network.rpc_url.clone()))
                        .into_any_element(),
                ];
                if let Some(explorer) = network.explorer_url.as_deref().filter(|u| !u.is_empty()) {
                    meta.push(
                        div()
                            .child(SharedString::from(explorer.to_string()))
                            .into_any_element(),
                    );
                }
                meta.push(
                    div()
                        .child(SharedString::from(network.currency_symbol.clone()))
                        .into_any_element(),
                );
                let edit_network = network.clone();
                let remove_id = network.id.clone();
                let can_remove = !network.builtin;
                widgets::card_row(&theme, ix == 0)
                    .child(widgets::row_tile(&theme, crate::icons::GLOBAL))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(widgets::row_title(&theme, network.name.clone()))
                            .child(widgets::meta_line(&theme, meta)),
                    )
                    .when(network.builtin, |el| {
                        el.child(widgets::badge(&theme, "Built-in"))
                    })
                    .child(
                        widgets::ghost_action(&theme)
                            .id(("network-edit", ix))
                            .opacity(0.7)
                            .hover(|s| widgets::ghost_hover(&theme, s))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_edit(edit_network.clone(), cx);
                            }))
                            .child(
                                crate::icons::icon(crate::icons::PEN)
                                    .size(px(14.0))
                                    .text_color(theme.text_muted),
                            )
                            .child(SharedString::from("Edit")),
                    )
                    .when(can_remove, |el| {
                        el.child(
                            widgets::ghost_action(&theme)
                                .id(("network-remove", ix))
                                .opacity(0.7)
                                .hover(|s| widgets::ghost_hover(&theme, s))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.remove(remove_id.clone(), cx);
                                }))
                                .child(
                                    crate::icons::icon(crate::icons::TRASH_BIN_MINIMALISTIC)
                                        .size(px(14.0))
                                        .text_color(theme.text_muted),
                                )
                                .child(SharedString::from("Remove")),
                        )
                    })
                    .into_any_element()
            })
            .collect()
    }
}

/// Lowercase slug suitable for a network id (`[a-z0-9-]+`).
pub fn slug_from_name(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if c.is_whitespace() || c == '-' || c == '_' {
            if !slug.is_empty() && !slug.ends_with('-') {
                slug.push('-');
            }
        }
    }
    slug.trim_matches('-').chars().take(48).collect()
}

impl Render for NetworksPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let count = self.networks.ready().map(|n| n.len());
        let body: AnyElement = match &self.networks {
            Loadable::Idle | Loadable::Loading => widgets::section_card(&theme)
                .p(px(16.0))
                .child(popover::skeleton_rows(
                    "networks-skeleton",
                    &theme,
                    3,
                    cx.entity_id(),
                    cx,
                ))
                .into_any_element(),
            Loadable::Error(message) => {
                let message = message.clone();
                div()
                    .child(widgets::error_strip(&theme, message))
                    .child(
                        widgets::ghost_action(&theme)
                            .id("networks-retry")
                            .mt(px(8.0))
                            .hover(|s| widgets::ghost_hover(&theme, s))
                            .on_click(cx.listener(|page, _, _, cx| {
                                page.load(cx);
                                cx.notify();
                            }))
                            .child(SharedString::from("Retry")),
                    )
                    .into_any_element()
            }
            Loadable::Ready(_) => {
                let rows = self.rows(cx);
                widgets::section_card(&theme)
                    .children(rows)
                    .into_any_element()
            }
        };
        let dialog = self.render_dialog(window.viewport_size(), cx);
        let error = self
            .error
            .clone()
            .map(|message| widgets::error_strip(&theme, message).into_any_element());

        div()
            .id("networks-page")
            .size_full()
            .overflow_y_scroll()
            .child(
                widgets::page_column()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(widgets::page_header(
                                &theme,
                                "Networks",
                                count.filter(|&c| c > 0),
                            ))
                            .child(
                                widgets::ghost_action(&theme)
                                    .id("network-add")
                                    .hover(|s| widgets::ghost_hover(&theme, s))
                                    .on_click(cx.listener(|this, _, _, cx| this.open_add(cx)))
                                    .child(
                                        crate::icons::icon(crate::icons::ADD_CIRCLE)
                                            .size(px(14.0))
                                            .text_color(theme.text_muted),
                                    )
                                    .child(SharedString::from("Add network")),
                            ),
                    )
                    .child(widgets::page_subtitle(
                        &theme,
                        "EVM RPC endpoints for Launch Studio deploys. Built-in X Layer presets \
                         can be edited but not removed.",
                    ))
                    .children(error)
                    .children(self.render_relay_card(&theme, cx))
                    .child(body),
            )
            .when_some(dialog, |el, dialog| el.child(dialog))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_from_name_normalizes() {
        assert_eq!(slug_from_name("  My Custom Net  "), "my-custom-net");
        assert_eq!(slug_from_name("Arbitrum One"), "arbitrum-one");
        assert_eq!(slug_from_name("---"), "");
    }
}
