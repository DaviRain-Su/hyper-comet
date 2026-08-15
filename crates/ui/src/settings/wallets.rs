//! Settings → Wallets: device-local address book for deploy signers.

use gpui::{
    AnyElement, Context, Entity, Render, SharedString, Subscription, Task, Window, div, prelude::*,
    px,
};

use zeron_proto::{
    CreateLocalWalletRequest, CreateLocalWalletResponse, ImportLocalWalletRequest,
    RemoveWalletRequest, UpsertWalletRequest, WalletAccount, WalletConnectStartRequest,
    WalletConnectStartResponse, WalletSource, WalletsResponse,
};
use zeron_rpc::methods;

use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::popover::{self, Loadable};
use crate::settings::widgets;
use crate::state::AppState;
use crate::theme::Theme;

#[derive(Clone, Copy)]
enum AddDialogKind {
    Watch,
    DevEnvKey,
    Import,
    Create,
}

struct AddDialog {
    kind: AddDialogKind,
    label: Entity<ComposerInput>,
    second: Entity<ComposerInput>,
    error: Option<String>,
    _events: Vec<Subscription>,
}

pub struct WalletsPage {
    state: Entity<AppState>,
    wallets: Loadable<Vec<WalletAccount>>,
    dialog: Option<AddDialog>,
    error: Option<String>,
    load_task: Option<Task<()>>,
    action_task: Option<Task<()>>,
    backup_hex: Option<String>,
}

impl WalletsPage {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            state,
            wallets: Loadable::Idle,
            dialog: None,
            error: None,
            load_task: None,
            action_task: None,
            backup_hex: None,
        };
        page.load(cx);
        page
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        self.wallets = Loadable::Loading;
        self.load_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::STUDIO_WALLETS, serde_json::json!({}))
                .await;
            this.update(cx, |page, cx| {
                page.wallets = match result {
                    Ok(value) => match serde_json::from_value::<WalletsResponse>(value) {
                        Ok(response) => Loadable::Ready(response.wallets),
                        Err(err) => Loadable::Error(err.to_string()),
                    },
                    Err(err) => Loadable::Error(err.to_string()),
                };
                cx.notify();
            })
            .ok();
        }));
    }

    fn open_add(&mut self, kind: AddDialogKind, cx: &mut Context<Self>) {
        let (label_placeholder, second_placeholder) = match kind {
            AddDialogKind::Watch => ("Label", "0x address"),
            AddDialogKind::DevEnvKey => ("Label", "Environment variable name"),
            AddDialogKind::Import => ("Label", "Private key (0x…64 hex)"),
            AddDialogKind::Create => ("Label", ""),
        };
        let label = cx.new(|cx| ComposerInput::new(label_placeholder, cx));
        let second = cx.new(|cx| ComposerInput::new(second_placeholder, cx));
        let mut events = Vec::new();
        for input in [&label, &second] {
            let input = input.clone();
            events.push(cx.subscribe(&input, |this: &mut Self, _, event, cx| {
                if matches!(event, ComposerInputEvent::Submitted) {
                    this.submit_dialog(cx);
                }
            }));
        }
        self.dialog = Some(AddDialog {
            kind,
            label,
            second,
            error: None,
            _events: events,
        });
        cx.notify();
    }

    fn submit_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = &self.dialog else {
            return;
        };
        let label = dialog.label.read(cx).text().trim().to_string();
        let second = dialog.second.read(cx).text().trim().to_string();
        let kind = dialog.kind;

        if label.is_empty() {
            self.set_dialog_error("Label is required", cx);
            return;
        }

        let wallet = match kind {
            AddDialogKind::Watch => {
                if second.is_empty() {
                    self.set_dialog_error("Address is required", cx);
                    return;
                }
                WalletAccount {
                    id: uuid::Uuid::new_v4().to_string(),
                    label,
                    address: second,
                    source: WalletSource::Watch,
                    env_key_name: None,
                }
            }
            AddDialogKind::DevEnvKey => {
                if second.is_empty() {
                    self.set_dialog_error("Environment variable name is required", cx);
                    return;
                }
                WalletAccount {
                    id: uuid::Uuid::new_v4().to_string(),
                    label,
                    address: String::new(),
                    source: WalletSource::DevEnvKey,
                    env_key_name: Some(second),
                }
            }
            AddDialogKind::Import => {
                if second.is_empty() {
                    self.set_dialog_error("Private key is required", cx);
                    return;
                }
                self.dialog = None;
                self.import_local(label, second, cx);
                return;
            }
            AddDialogKind::Create => {
                self.dialog = None;
                self.create_local(label, cx);
                return;
            }
        };

        self.dialog = None;
        self.upsert(wallet, cx);
    }

    fn set_dialog_error(&mut self, message: &str, cx: &mut Context<Self>) {
        if let Some(dialog) = &mut self.dialog {
            dialog.error = Some(message.to_string());
        }
        cx.notify();
    }

    fn create_local(&mut self, label: String, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let req = CreateLocalWalletRequest { label };
        self.error = None;
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(
                    methods::STUDIO_WALLET_CREATE,
                    serde_json::to_value(req).unwrap_or_default(),
                )
                .await;
            this.update(cx, |page, cx| {
                match result {
                    Ok(value) => {
                        match serde_json::from_value::<CreateLocalWalletResponse>(value) {
                            Ok(resp) => {
                                page.wallets = Loadable::Ready(resp.wallets);
                                page.backup_hex = Some(resp.backup_hex);
                            }
                            Err(err) => page.error = Some(err.to_string()),
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

    fn import_local(&mut self, label: String, secret: String, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let req = ImportLocalWalletRequest { label, secret };
        self.error = None;
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(
                    methods::STUDIO_WALLET_IMPORT,
                    serde_json::to_value(req).unwrap_or_default(),
                )
                .await;
            this.update(cx, |page, cx| {
                match result {
                    Ok(value) => {
                        if let Ok(response) = serde_json::from_value::<WalletsResponse>(value) {
                            page.wallets = Loadable::Ready(response.wallets);
                        } else {
                            page.error = Some("Unexpected import response".into());
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

    fn upsert(&mut self, wallet: WalletAccount, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let request = UpsertWalletRequest { wallet };
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
                .call(methods::STUDIO_UPSERT_WALLET, params)
                .await;
            this.update(cx, |page, cx| {
                match result {
                    Ok(value) => {
                        if let Ok(response) = serde_json::from_value::<WalletsResponse>(value) {
                            page.wallets = Loadable::Ready(response.wallets);
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
        let request = RemoveWalletRequest { id: id.clone() };
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
                .call(methods::STUDIO_REMOVE_WALLET, params)
                .await;
            this.update(cx, |page, cx| {
                match result {
                    Ok(value) => {
                        if let Ok(response) = serde_json::from_value::<WalletsResponse>(value) {
                            page.wallets = Loadable::Ready(response.wallets);
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
        let (title, second_label, helper) = match dialog.kind {
            AddDialogKind::Watch => ("Add watch address", "Address", None),
            AddDialogKind::DevEnvKey => (
                "Add dev env-key",
                "Environment variable",
                Some(
                    "Testnet only. The key stays in your environment; ProofShip stores the \
                     variable name.",
                ),
            ),
            AddDialogKind::Import => (
                "Import local wallet",
                "Private key",
                Some(
                    "Hex key only (0x + 64 digits). ProofShip stores it next to the address \
                     book with mode 0600 — never in wallets.json.",
                ),
            ),
            AddDialogKind::Create => (
                "Create local wallet",
                "Unused",
                Some("Alloy generates a secp256k1 key on this machine. Copy the backup once."),
            ),
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
        let mut card = popover::dialog_card(&theme)
            .child(popover::dialog_title(&theme, title))
            .child(field("Label", dialog.label.clone()));
        if !matches!(dialog.kind, AddDialogKind::Create) {
            card = card.child(field(second_label, dialog.second.clone()));
        }
        if let Some(copy) = helper {
            card = card.child(
                popover::dialog_body(&theme, copy)
                    .mt(px(8.0))
                    .text_size(px(12.0)),
            );
        }
        let card = card
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
                        popover::btn_ghost(&theme, "Cancel", "wallet-cancel")
                            .id("wallet-cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.dialog = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        popover::btn_primary(&theme, "Add")
                            .id("wallet-save")
                            .on_click(cx.listener(|this, _, _, cx| this.submit_dialog(cx))),
                    ),
            )
            .into_any_element();
        Some(popover::modal("wallet-dialog", viewport, card))
    }

    fn wallet_rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = Theme::of(cx).clone();
        let Loadable::Ready(list) = &self.wallets else {
            return Vec::new();
        };
        list.iter()
            .enumerate()
            .map(|(ix, wallet)| {
                let mut meta: Vec<AnyElement> = vec![];
                if wallet.address.is_empty() {
                    meta.push(
                        div()
                            .child(SharedString::from("Address pending"))
                            .into_any_element(),
                    );
                } else {
                    meta.push(
                        div()
                            .font_family(theme.font_mono.clone())
                            .child(SharedString::from(truncate_address(&wallet.address)))
                            .into_any_element(),
                    );
                }
                if let Some(name) = wallet.env_key_name.as_deref().filter(|n| !n.is_empty()) {
                    meta.push(
                        div()
                            .font_family(theme.font_mono.clone())
                            .child(SharedString::from(name.to_string()))
                            .into_any_element(),
                    );
                    meta.push(
                        div()
                            .child(SharedString::from(env_key_status(name).to_string()))
                            .into_any_element(),
                    );
                }
                let remove_id = wallet.id.clone();
                let address = wallet.address.clone();
                widgets::card_row(&theme, ix == 0)
                    .child(widgets::row_tile(&theme, source_icon(wallet.source)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(widgets::row_title(&theme, wallet.label.clone()))
                            .child(widgets::meta_line(&theme, meta)),
                    )
                    .child(widgets::badge(&theme, source_badge(wallet.source)))
                    .when(!address.is_empty(), |el| {
                        el.child(
                            widgets::ghost_action(&theme)
                                .id(("wallet-copy", ix))
                                .opacity(0.7)
                                .hover(|s| widgets::ghost_hover(&theme, s))
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        address.clone(),
                                    ));
                                }))
                                .child(SharedString::from("Copy")),
                        )
                    })
                    .child(
                        widgets::ghost_action(&theme)
                            .id(("wallet-remove", ix))
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
                    .into_any_element()
            })
            .collect()
    }

    fn walletconnect_row(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        widgets::card_row(theme, false)
            .child(widgets::row_tile(theme, crate::icons::GLOBAL))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(widgets::row_title(theme, "WalletConnect (desktop)"))
                    .child(widgets::meta_line(
                        theme,
                        vec![
                            div()
                                .child(SharedString::from(
                                    "Opens a local bridge page — keep that tab open for signing. Requires PROOFSHIP_WC_PROJECT_ID.",
                                ))
                                .into_any_element(),
                        ],
                    )),
            )
            .child(
                widgets::ghost_action(theme)
                    .id("wallet-wc-connect")
                    .hover(|s| widgets::ghost_hover(theme, s))
                    .on_click(cx.listener(|this, _, _, cx| this.start_walletconnect(cx)))
                    .child(SharedString::from("Connect")),
            )
            .into_any_element()
    }

    fn start_walletconnect(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.state.read(cx).engine().cloned() else {
            return;
        };
        let req = WalletConnectStartRequest {
            label: "WalletConnect".into(),
            project_id: None,
        };
        self.error = None;
        self.action_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(
                    methods::STUDIO_WC_START,
                    serde_json::to_value(req).unwrap_or_default(),
                )
                .await;
            this.update(cx, |page, cx| match result {
                Ok(value) => match serde_json::from_value::<WalletConnectStartResponse>(value) {
                    Ok(resp) => {
                        cx.open_url(&resp.url);
                        page.error = Some(
                            "Approve in the browser tab, then Refresh — the address appears below."
                                .into(),
                        );
                        page.load(cx);
                    }
                    Err(err) => page.error = Some(err.to_string()),
                },
                Err(err) => page.error = Some(err.to_string()),
            })
            .ok();
        }));
    }
}

/// Short address for row meta (`0xabcd…ef01`).
pub fn truncate_address(address: &str) -> String {
    if address.is_empty() {
        return "—".to_string();
    }
    if address.len() > 12 {
        format!("{}…{}", &address[..6], &address[address.len() - 4..])
    } else {
        address.to_string()
    }
}

/// Badge label for a wallet source.
pub fn source_badge(source: WalletSource) -> &'static str {
    match source {
        WalletSource::Watch => "Watch",
        WalletSource::DevEnvKey => "Env key",
        WalletSource::WalletConnect => "WalletConnect",
        WalletSource::Local => "Local",
    }
}

fn source_icon(source: WalletSource) -> &'static str {
    match source {
        WalletSource::Watch => crate::icons::MONITOR,
        WalletSource::DevEnvKey => crate::icons::KEY_MINIMALISTIC,
        WalletSource::WalletConnect => crate::icons::GLOBAL,
        WalletSource::Local => crate::icons::KEY_MINIMALISTIC,
    }
}

/// Whether the named env var is set and non-empty (never reads the value into UI).
pub fn env_key_status(name: &str) -> &'static str {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => "key present",
        _ => "key missing",
    }
}

impl Render for WalletsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let count = self.wallets.ready().map(|w| w.len());
        let body: AnyElement = match &self.wallets {
            Loadable::Idle | Loadable::Loading => widgets::section_card(&theme)
                .p(px(16.0))
                .child(popover::skeleton_rows(
                    "wallets-skeleton",
                    &theme,
                    2,
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
                            .id("wallets-retry")
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
                let rows = self.wallet_rows(cx);
                widgets::section_card(&theme)
                    .when(rows.is_empty(), |el| {
                        el.child(
                            div()
                                .px(px(20.0))
                                .py(px(24.0))
                                .text_center()
                                .text_size(px(14.0))
                                .text_color(theme.text_muted.opacity(0.6))
                                .child(SharedString::from(
                                    "No wallets yet — create a local signer to deploy from this desktop.",
                                )),
                        )
                    })
                    .children(rows)
                    .child(self.walletconnect_row(&theme, cx))
                    .into_any_element()
            }
        };
        let dialog = self.render_dialog(window.viewport_size(), cx);
        let error = self
            .error
            .clone()
            .map(|message| widgets::error_strip(&theme, message).into_any_element());

        div()
            .id("wallets-page")
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
                                "Wallets",
                                count.filter(|&c| c > 0),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        widgets::ghost_action(&theme)
                                            .id("wallet-create-local")
                                            .hover(|s| widgets::ghost_hover(&theme, s))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.open_add(AddDialogKind::Create, cx);
                                            }))
                                            .child(
                                                crate::icons::icon(crate::icons::ADD_CIRCLE)
                                                    .size(px(14.0))
                                                    .text_color(theme.text_muted),
                                            )
                                            .child(SharedString::from("Create wallet")),
                                    )
                                    .child(
                                        widgets::ghost_action(&theme)
                                            .id("wallet-import-local")
                                            .hover(|s| widgets::ghost_hover(&theme, s))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.open_add(AddDialogKind::Import, cx);
                                            }))
                                            .child(SharedString::from("Import key")),
                                    )
                                    .child(
                                        widgets::ghost_action(&theme)
                                            .id("wallet-add-watch")
                                            .hover(|s| widgets::ghost_hover(&theme, s))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.open_add(AddDialogKind::Watch, cx);
                                            }))
                                            .child(
                                                crate::icons::icon(crate::icons::ADD_CIRCLE)
                                                    .size(px(14.0))
                                                    .text_color(theme.text_muted),
                                            )
                                            .child(SharedString::from("Add watch")),
                                    )
                                    .child(
                                        widgets::ghost_action(&theme)
                                            .id("wallet-add-env")
                                            .hover(|s| widgets::ghost_hover(&theme, s))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.open_add(AddDialogKind::DevEnvKey, cx);
                                            }))
                                            .child(
                                                crate::icons::icon(crate::icons::KEY_MINIMALISTIC)
                                                    .size(px(14.0))
                                                    .text_color(theme.text_muted),
                                            )
                                            .child(SharedString::from("Add env-key")),
                                    ),
                            ),
                    )
                    .child(widgets::page_subtitle(
                        &theme,
                        "Create a local Alloy signer for deploys. The hex key stays in \
                         studio/wallet-secrets (mode 0600), never in wallets.json.",
                    ))
                    .children(error)
                    .child(body),
            )
            .when_some(dialog, |el, dialog| el.child(dialog))
            .when_some(self.render_backup(window.viewport_size(), cx), |el, backup| {
                el.child(backup)
            })
    }
}

impl WalletsPage {
    fn render_backup(
        &self,
        viewport: gpui::Size<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let theme = Theme::of(cx).clone();
        let backup = self.backup_hex.clone()?;
        let shown = backup.clone();
        let card = popover::dialog_card(&theme)
            .child(popover::dialog_title(&theme, "Wallet created"))
            .child(
                popover::dialog_body(
                    &theme,
                    "Copy this private key now. ProofShip will not show it again. \
                     Fund the address on X Layer testnet before deploying.",
                )
                .mt(px(8.0)),
            )
            .child(
                div()
                    .mt(px(12.0))
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .bg(theme.surface)
                    .font_family(theme.font_mono.clone())
                    .text_size(px(11.0))
                    .text_color(theme.text)
                    .child(SharedString::from(shown)),
            )
            .child(
                div()
                    .mt(px(16.0))
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        popover::btn_ghost(&theme, "Copy key", "wallet-backup-copy")
                            .id("wallet-backup-copy")
                            .on_click(cx.listener({
                                let backup = backup.clone();
                                move |_, _, _, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        backup.clone(),
                                    ));
                                }
                            })),
                    )
                    .child(
                        popover::btn_primary(&theme, "I've saved it")
                            .id("wallet-backup-done")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.backup_hex = None;
                                cx.notify();
                            })),
                    ),
            )
            .into_any_element();
        Some(popover::modal("wallet-backup", viewport, card))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_address_shortens() {
        assert_eq!(
            truncate_address("0xAbCdEf0123456789AbCdEf0123456789AbCdEf01"),
            "0xAbCd…Ef01"
        );
        assert_eq!(truncate_address(""), "—");
        assert_eq!(truncate_address("0xabc"), "0xabc");
    }

    #[test]
    fn source_badges() {
        assert_eq!(source_badge(WalletSource::Watch), "Watch");
        assert_eq!(source_badge(WalletSource::DevEnvKey), "Env key");
        assert_eq!(source_badge(WalletSource::WalletConnect), "WalletConnect");
        assert_eq!(source_badge(WalletSource::Local), "Local");
    }
}
