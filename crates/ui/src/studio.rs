//! ProofShip Launch Studio panel (Track C1b).

use std::collections::BTreeMap;

use gpui::{
    AnyElement, Context, Entity, Focusable as _, ListAlignment, ListState, SharedString,
    Subscription, Task, Window, div, list, prelude::*, px,
};

use comet_engine::registry::HarnessDescriptor;
use comet_proto::HarnessId;
use comet_proto::studio::{
    StudioChatMsg, StudioDraftEvent, StudioDraftFields, StudioDraftMsg, StudioGateArtifact,
    StudioGateDigest, StudioGateEvent, StudioGateMsg, StudioGateRequest, StudioGateRunResult,
    StudioGateStage, StudioGateState, StudioLaunch, StudioLaunchRunEvent, StudioLaunchRunRequest,
    StudioLaunchesResponse, StudioNoteMsg, StudioPutLaunchesRequest, StudioStatusResponse,
    StudioUserMsg,
};
use comet_rpc::methods;

use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::icons::{self, icon};
use crate::markdown::highlight::Token;
use crate::markdown::parser::parse_full;
use crate::markdown::render::{RenderOptions, render_tree};
use crate::pickers::{harness_brand_icon, visible_harnesses};
use crate::popover::{self, Loadable};
use crate::settings::composer::ComposerDefaults;
use crate::state::AppState;
use crate::theme::Theme;

const SAMPLE_SOURCE: &str = include_str!("../../engine/tests/fixtures/rwa_share_registry.lean");
const SAMPLE_MODULE: &str = "RwaShareRegistry";
const INSTALL_COMMAND: &str = "proofship/scripts/install-toolchain.sh";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioStageStatus {
    Pending,
    Running,
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageView {
    pub stage: StudioGateStage,
    pub status: StudioStageStatus,
    pub output: Option<String>,
    pub expanded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCard {
    pub round: Option<u32>,
    pub state: StudioGateState,
    pub stages: Vec<StageView>,
    pub artifacts: Vec<StudioGateArtifact>,
    pub digest: StudioGateDigest,
    pub failed_stage: Option<StudioGateStage>,
    pub error: Option<String>,
}

impl GateCard {
    pub fn running() -> Self {
        Self {
            round: None,
            state: StudioGateState::Running,
            stages: gate_stages()
                .into_iter()
                .map(|stage| StageView {
                    stage,
                    status: StudioStageStatus::Pending,
                    output: None,
                    expanded: false,
                })
                .collect(),
            artifacts: Vec::new(),
            digest: StudioGateDigest::default(),
            failed_stage: None,
            error: None,
        }
    }

    fn stage_mut(&mut self, stage: StudioGateStage) -> Option<&mut StageView> {
        self.stages.iter_mut().find(|s| s.stage == stage)
    }
}

pub fn draft_done_row(ix: usize, event: StudioDraftEvent) -> Option<Result<StudioRow, String>> {
    draft_done_row_with_round(ix, None, event)
}

pub fn draft_done_row_with_round(
    ix: usize,
    round: Option<u32>,
    event: StudioDraftEvent,
) -> Option<Result<StudioRow, String>> {
    match event {
        StudioDraftEvent::Done {
            ok: true,
            lane,
            module: Some(module),
            source: Some(source),
            ..
        } => Some(Ok(StudioRow {
            id: format!("draft-{ix}"),
            at: now(),
            kind: StudioRowKind::Draft {
                round,
                program: module,
                fields: StudioDraftFields::new(),
                source,
                note: Some(format!("drafted by {}", lane_label(lane))),
                source_open: false,
            },
        })),
        StudioDraftEvent::Done {
            ok: false, error, ..
        } => Some(Err(error.unwrap_or_else(|| "draft failed".into()))),
        _ => None,
    }
}

pub fn draft_then_gate_sequence(
    rows: &mut Vec<StudioRow>,
    draft: StudioRow,
) -> Option<(String, String)> {
    let (module, source) = match &draft.kind {
        StudioRowKind::Draft {
            program, source, ..
        } => (program.clone(), source.clone()),
        _ => return None,
    };
    rows.push(draft);
    rows.push(StudioRow {
        id: format!("gate-{}", rows.len()),
        at: now(),
        kind: StudioRowKind::Gate(GateCard::running()),
    });
    Some((module, source))
}

pub fn reduce_gate_event(card: &mut GateCard, event: StudioGateEvent) {
    match event {
        StudioGateEvent::Started { stage } => {
            if let Some(view) = card.stage_mut(stage) {
                view.status = StudioStageStatus::Running;
            }
        }
        StudioGateEvent::StageDone { stage, ok, output } => {
            if let Some(view) = card.stage_mut(stage) {
                view.status = if ok {
                    StudioStageStatus::Pass
                } else {
                    StudioStageStatus::Fail
                };
                view.expanded = !ok;
                if !output.trim().is_empty() {
                    view.output = Some(output.clone());
                }
            }
            if !ok {
                card.state = StudioGateState::Fail;
                card.failed_stage = Some(stage);
                card.error = Some(output);
            }
        }
        StudioGateEvent::Done {
            ok,
            stage,
            artifacts,
            digest,
        } => {
            card.state = if ok {
                StudioGateState::Pass
            } else {
                StudioGateState::Fail
            };
            card.artifacts = artifacts;
            card.digest = digest;
            if !ok {
                card.failed_stage = Some(stage);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StudioRowKind {
    User {
        text: String,
    },
    Draft {
        round: Option<u32>,
        program: String,
        fields: StudioDraftFields,
        source: String,
        note: Option<String>,
        source_open: bool,
    },
    Gate(GateCard),
    Note {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StudioRow {
    pub id: String,
    pub at: String,
    pub kind: StudioRowKind,
}

pub fn rows_from_launch(launch: &StudioLaunch) -> Vec<StudioRow> {
    launch
        .msgs
        .iter()
        .enumerate()
        .map(|(ix, msg)| match msg {
            StudioChatMsg::User(msg) => StudioRow {
                id: format!("user-{ix}"),
                at: msg.at.clone(),
                kind: StudioRowKind::User {
                    text: msg.text.clone(),
                },
            },
            StudioChatMsg::AgentDraft(msg) => StudioRow {
                id: format!("draft-{ix}"),
                at: msg.at.clone(),
                kind: StudioRowKind::Draft {
                    round: None,
                    program: msg.program.clone(),
                    fields: msg.fields.clone(),
                    source: msg.source.clone(),
                    note: msg.note.clone(),
                    source_open: false,
                },
            },
            StudioChatMsg::AgentGate(msg) => StudioRow {
                id: format!("gate-{ix}"),
                at: msg.at.clone(),
                kind: StudioRowKind::Gate(gate_card_from_msg(msg)),
            },
            StudioChatMsg::AgentNote(msg) => StudioRow {
                id: format!("note-{ix}"),
                at: msg.at.clone(),
                kind: StudioRowKind::Note {
                    text: msg.text.clone(),
                },
            },
        })
        .collect()
}

pub fn launch_from_rows(existing: Option<&StudioLaunch>, rows: &[StudioRow]) -> StudioLaunch {
    let mut launch = existing.cloned().unwrap_or_else(StudioLaunch::new_now);
    launch.msgs = rows.iter().map(row_to_msg).collect();
    launch.title = rows
        .iter()
        .find_map(|row| match &row.kind {
            StudioRowKind::User { text } => Some(title_from_text(text)),
            StudioRowKind::Draft { program, .. } => Some(format!("Launch {program}")),
            _ => None,
        })
        .unwrap_or_else(|| "New launch".into());
    if let Some((program, fields, source)) = rows.iter().rev().find_map(|row| match &row.kind {
        StudioRowKind::Draft {
            program,
            fields,
            source,
            ..
        } => Some((program.clone(), fields.clone(), source.clone())),
        _ => None,
    }) {
        launch.program = Some(program);
        launch.fields = Some(fields);
        launch.source = Some(source);
    }
    launch
}

fn gate_stages() -> [StudioGateStage; 3] {
    [
        StudioGateStage::Check,
        StudioGateStage::Build,
        StudioGateStage::Inspect,
    ]
}

fn row_to_msg(row: &StudioRow) -> StudioChatMsg {
    match &row.kind {
        StudioRowKind::User { text } => StudioChatMsg::User(StudioUserMsg {
            role: "user".into(),
            text: text.clone(),
            at: row.at.clone(),
        }),
        StudioRowKind::Draft {
            program,
            fields,
            source,
            note,
            ..
        } => StudioChatMsg::AgentDraft(StudioDraftMsg {
            role: "agent".into(),
            kind: "draft".into(),
            fields: fields.clone(),
            program: program.clone(),
            source: source.clone(),
            note: note.clone(),
            at: row.at.clone(),
        }),
        StudioRowKind::Gate(card) => StudioChatMsg::AgentGate(StudioGateMsg {
            role: "agent".into(),
            kind: "gate".into(),
            state: card.state,
            result: Some(gate_result_from_card(card)),
            at: row.at.clone(),
        }),
        StudioRowKind::Note { text } => StudioChatMsg::AgentNote(StudioNoteMsg {
            role: "agent".into(),
            kind: "note".into(),
            text: text.clone(),
            at: row.at.clone(),
        }),
    }
}

fn gate_card_from_msg(msg: &StudioGateMsg) -> GateCard {
    let mut card = GateCard::running();
    card.state = if msg.state == StudioGateState::Running {
        StudioGateState::Offline
    } else {
        msg.state
    };
    if let Some(result) = &msg.result {
        card.failed_stage = (!result.ok).then_some(result.stage);
        card.error = result.error.clone();
        for (stage, output) in [
            (StudioGateStage::Check, &result.check),
            (StudioGateStage::Build, &result.build),
            (StudioGateStage::Inspect, &result.inspect),
        ] {
            if let Some(view) = card.stage_mut(stage) {
                view.output = output.clone();
                view.status = if result.ok || stage_before_or_eq(stage, result.stage) {
                    if !result.ok && stage == result.stage {
                        StudioStageStatus::Fail
                    } else {
                        StudioStageStatus::Pass
                    }
                } else {
                    StudioStageStatus::Pending
                };
                view.expanded = !result.ok && stage == result.stage;
            }
        }
    }
    card
}

fn gate_result_from_card(card: &GateCard) -> StudioGateRunResult {
    let output = |stage| {
        card.stages
            .iter()
            .find(|s| s.stage == stage)
            .and_then(|s| s.output.clone())
    };
    StudioGateRunResult {
        ok: card.state == StudioGateState::Pass,
        stage: card.failed_stage.unwrap_or(StudioGateStage::Done),
        check: output(StudioGateStage::Check),
        build: output(StudioGateStage::Build),
        inspect: output(StudioGateStage::Inspect),
        error: card.error.clone(),
    }
}

fn stage_before_or_eq(stage: StudioGateStage, terminal: StudioGateStage) -> bool {
    stage_rank(stage) <= stage_rank(terminal)
}

fn stage_rank(stage: StudioGateStage) -> u8 {
    match stage {
        StudioGateStage::Check => 0,
        StudioGateStage::Build => 1,
        StudioGateStage::Inspect => 2,
        StudioGateStage::Done => 3,
    }
}

fn title_from_text(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 48 {
        format!("{}…", flat.chars().take(47).collect::<String>())
    } else if flat.is_empty() {
        "New launch".into()
    } else {
        flat
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / MB)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / KB)
    } else {
        format!("{bytes} B")
    }
}

fn truncate_middle(value: &str, keep: usize) -> String {
    if value.chars().count() <= keep * 2 + 1 {
        return value.into();
    }
    let start: String = value.chars().take(keep).collect();
    let end: String = value
        .chars()
        .rev()
        .take(keep)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{start}…{end}")
}

struct GateSourceDialog {
    module: Entity<ComposerInput>,
    source: Entity<ComposerInput>,
    error: Option<SharedString>,
    _module_events: Subscription,
    _source_events: Subscription,
}

pub struct StudioView {
    state: Entity<AppState>,
    rows: Vec<StudioRow>,
    launch: Option<StudioLaunch>,
    list: ListState,
    composer: Entity<ComposerInput>,
    gate_dialog: Option<GateSourceDialog>,
    status: Option<StudioStatusResponse>,
    status_error: Option<SharedString>,
    status_dismissed: bool,
    error: Option<SharedString>,
    harnesses: Loadable<Vec<HarnessDescriptor>>,
    selected_lane: Option<HarnessId>,
    lane_menu_open: bool,
    draft_task: Option<Task<()>>,
    gate_task: Option<Task<()>>,
    load_task: Option<Task<()>>,
    save_task: Option<Task<()>>,
    _input_events: Subscription,
    _observe: Subscription,
}

impl StudioView {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let composer = cx.new(|cx| ComposerInput::new("Describe a contract to draft…", cx));
        let input_events = cx.subscribe(&composer, |this: &mut Self, _, event, cx| {
            if matches!(event, ComposerInputEvent::Submitted) {
                this.submit_nl(cx);
            }
        });
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        let mut this = Self {
            state,
            rows: Vec::new(),
            launch: None,
            list: ListState::new(0, ListAlignment::Bottom, px(320.0)),
            composer,
            gate_dialog: None,
            status: None,
            status_error: None,
            status_dismissed: false,
            error: None,
            harnesses: Loadable::Idle,
            selected_lane: None,
            lane_menu_open: false,
            draft_task: None,
            gate_task: None,
            load_task: None,
            save_task: None,
            _input_events: input_events,
            _observe: observe,
        };
        this.load(cx);
        this.load_harnesses(cx);
        this.load_studio_default(cx);
        this
    }

    pub fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.composer.focus_handle(cx)
    }

    fn engine(&self, cx: &Context<Self>) -> Option<crate::state::EngineHandle> {
        self.state.read(cx).engine().cloned()
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.engine(cx) else {
            return;
        };
        self.load_task = Some(cx.spawn(async move |this, cx| {
            let status = engine
                .client()
                .call(methods::STUDIO_STATUS, serde_json::json!({}))
                .await;
            let launches = engine
                .client()
                .call(methods::STUDIO_LAUNCHES, serde_json::json!({}))
                .await;
            this.update(cx, |view, cx| {
                match status {
                    Ok(value) => match serde_json::from_value::<StudioStatusResponse>(value) {
                        Ok(status) => view.status = Some(status),
                        Err(err) => {
                            view.status_error = Some(format!("Studio status: {err}").into())
                        }
                    },
                    Err(err) => view.status_error = Some(format!("Studio status: {err}").into()),
                }
                match launches {
                    Ok(value) => match serde_json::from_value::<StudioLaunchesResponse>(value) {
                        Ok(mut launches) => {
                            launches
                                .launches
                                .sort_by_key(|launch| launch.created_at_datetime());
                            if let Some(launch) = launches.launches.pop() {
                                view.rows = rows_from_launch(&launch);
                                view.launch = Some(launch);
                                view.sync_list();
                            }
                        }
                        Err(err) => view.error = Some(format!("Studio launches: {err}").into()),
                    },
                    Err(err) => view.error = Some(format!("Studio launches: {err}").into()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn sync_list(&mut self) {
        self.list.splice(0..self.list.item_count(), self.rows.len());
    }

    fn load_studio_default(&mut self, cx: &mut Context<Self>) {
        let data_dir = self.state.read(cx).data_dir.clone();
        if let Some(data_dir) = data_dir {
            self.selected_lane = ComposerDefaults::load(&data_dir).studio_harness;
        }
    }

    fn save_studio_default(&self, cx: &mut Context<Self>) {
        let Some(lane) = self.selected_lane else {
            return;
        };
        let data_dir = self.state.read(cx).data_dir.clone();
        if let Some(data_dir) = data_dir {
            let mut defaults = ComposerDefaults::load(&data_dir);
            defaults.studio_harness = Some(lane);
            if let Err(err) = defaults.save(&data_dir) {
                tracing::warn!(error = %err, "studio lane default save failed");
            }
        }
    }

    fn load_harnesses(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.engine(cx) else {
            return;
        };
        self.harnesses = Loadable::Loading;
        self.load_task = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::LIST_HARNESSES, serde_json::json!({}))
                .await;
            this.update(cx, |view, cx| {
                view.harnesses = match result {
                    Ok(value) => match serde_json::from_value::<Vec<HarnessDescriptor>>(value) {
                        Ok(list) => {
                            let offered = studio_offered_harnesses(&list);
                            let selected_valid = view
                                .selected_lane
                                .is_some_and(|lane| offered.iter().any(|h| h.id == lane));
                            if !selected_valid {
                                view.selected_lane = offered.first().map(|h| h.id);
                            }
                            Loadable::Ready(list)
                        }
                        Err(err) => Loadable::Error(err.to_string()),
                    },
                    Err(err) => Loadable::Error(err.to_string()),
                };
                cx.notify();
            })
            .ok();
        }));
    }

    fn mutate_rows(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Vec<StudioRow>)) {
        f(&mut self.rows);
        self.sync_list();
        self.persist(cx);
        cx.notify();
    }

    fn persist(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.engine(cx) else {
            return;
        };
        let launch = launch_from_rows(self.launch.as_ref(), &self.rows);
        self.launch = Some(launch.clone());
        self.save_task = Some(cx.spawn(async move |this, cx| {
            let params = match serde_json::to_value(StudioPutLaunchesRequest {
                launches: vec![launch.clone()],
            }) {
                Ok(params) => params,
                Err(err) => {
                    this.update(cx, |view, cx| {
                        view.error = Some(format!("Studio save: {err}").into());
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            };
            let result = engine
                .client()
                .call(methods::STUDIO_PUT_LAUNCHES, params)
                .await;
            this.update(cx, |view, cx| {
                match result {
                    Ok(value) => {
                        if let Ok(mut launches) =
                            serde_json::from_value::<StudioLaunchesResponse>(value)
                        {
                            launches
                                .launches
                                .sort_by_key(|launch| launch.created_at_datetime());
                            if let Some(saved) = launches.launches.pop() {
                                view.launch = Some(saved);
                            }
                        }
                    }
                    Err(err) => view.error = Some(format!("Studio save: {err}").into()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn submit_nl(&mut self, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).text().trim().to_string();
        if text.is_empty() {
            return;
        }
        let Some(lane) = self.selected_lane else {
            self.error = Some("No installed Studio lane is available".into());
            cx.notify();
            return;
        };
        let Some(engine) = self.engine(cx) else {
            self.error = Some("Engine not connected".into());
            cx.notify();
            return;
        };
        self.composer.update(cx, |input, cx| input.set_text("", cx));
        self.mutate_rows(cx, |rows| {
            rows.push(StudioRow {
                id: format!("user-{}", rows.len()),
                at: now(),
                kind: StudioRowKind::User { text: text.clone() },
            });
        });
        let params = match serde_json::to_value(StudioLaunchRunRequest {
            nl: text,
            harness: lane,
        }) {
            Ok(params) => params,
            Err(err) => {
                self.error = Some(format!("Studio launch: {err}").into());
                cx.notify();
                return;
            }
        };
        self.draft_task = Some(cx.spawn(async move |this, cx| {
            let stream = engine
                .client()
                .subscribe(methods::STUDIO_LAUNCH_RUN, params)
                .await;
            match stream {
                Ok(mut rx) => {
                    while let Some(value) = rx.recv().await {
                        match serde_json::from_value::<StudioLaunchRunEvent>(value) {
                            Ok(event) => {
                                this.update(cx, |view, cx| view.apply_launch_event(event, cx))
                                    .ok();
                            }
                            Err(err) => {
                                this.update(cx, |view, cx| {
                                    view.error = Some(format!("Studio launch event: {err}").into());
                                    cx.notify();
                                })
                                .ok();
                            }
                        }
                    }
                }
                Err(err) => {
                    this.update(cx, |view, cx| {
                        view.push_note(format!("Launch failed: {err}"), cx);
                    })
                    .ok();
                }
            }
        }));
    }

    fn load_sample(&mut self, cx: &mut Context<Self>) {
        self.mutate_rows(cx, |rows| {
            rows.push(draft_row(
                rows.len(),
                SAMPLE_MODULE.into(),
                SAMPLE_SOURCE.into(),
                Some("bundled offline demo sample".into()),
            ));
        });
    }

    fn open_gate_dialog(&mut self, cx: &mut Context<Self>) {
        let module = cx.new(|cx| ComposerInput::new("Module name", cx));
        let source = cx.new(|cx| ComposerInput::new("ProgramV1 Lean source", cx));
        if let Some((program, src)) = self.latest_draft() {
            module.update(cx, |input, cx| input.set_text(program, cx));
            source.update(cx, |input, cx| input.set_text(src, cx));
        }
        let module_events = cx.subscribe(&module, |this: &mut Self, _, event, cx| {
            if matches!(event, ComposerInputEvent::Submitted) {
                this.submit_gate_dialog(cx);
            }
        });
        let source_events = cx.subscribe(&source, |this: &mut Self, _, event, cx| {
            if matches!(event, ComposerInputEvent::Submitted) {
                this.submit_gate_dialog(cx);
            }
        });
        self.gate_dialog = Some(GateSourceDialog {
            module,
            source,
            error: None,
            _module_events: module_events,
            _source_events: source_events,
        });
        cx.notify();
    }

    fn latest_draft(&self) -> Option<(String, String)> {
        self.rows.iter().rev().find_map(|row| match &row.kind {
            StudioRowKind::Draft {
                program, source, ..
            } => Some((program.clone(), source.clone())),
            _ => None,
        })
    }

    fn submit_gate_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = &self.gate_dialog else {
            return;
        };
        let module = dialog.module.read(cx).text().trim().to_string();
        let source = dialog.source.read(cx).text().trim().to_string();
        if module.is_empty() || source.is_empty() {
            if let Some(dialog) = &mut self.gate_dialog {
                dialog.error = Some("Module and source are required".into());
            }
            cx.notify();
            return;
        }
        self.gate_dialog = None;
        self.start_gate(module, source, cx);
    }

    fn apply_launch_event(&mut self, event: StudioLaunchRunEvent, cx: &mut Context<Self>) {
        apply_launch_event_to_rows(&mut self.rows, event);
        self.sync_list();
        self.persist(cx);
        cx.notify();
    }

    fn push_note(&mut self, text: String, cx: &mut Context<Self>) {
        self.mutate_rows(cx, |rows| {
            rows.push(StudioRow {
                id: format!("note-{}", rows.len()),
                at: now(),
                kind: StudioRowKind::Note { text },
            });
        });
    }

    fn start_gate(&mut self, module: String, source: String, cx: &mut Context<Self>) {
        let gate_ix = self.rows.len() + 1;
        self.mutate_rows(cx, |rows| {
            if !matches!(rows.last().map(|r| &r.kind), Some(StudioRowKind::Draft { program, source: s, .. }) if program == &module && s == &source) {
                rows.push(draft_row(rows.len(), module.clone(), source.clone(), Some("manual source".into())));
            }
            rows.push(StudioRow {
                id: format!("gate-{gate_ix}"),
                at: now(),
                kind: StudioRowKind::Gate(GateCard::running()),
            });
        });
        self.start_gate_existing(module, source, cx);
    }

    fn start_gate_existing(&mut self, module: String, source: String, cx: &mut Context<Self>) {
        let Some(engine) = self.engine(cx) else {
            self.error = Some("Engine not connected".into());
            cx.notify();
            return;
        };
        let gate_row_id = self
            .rows
            .iter()
            .rev()
            .find(|row| matches!(row.kind, StudioRowKind::Gate(_)))
            .map(|row| row.id.clone())
            .unwrap_or_default();
        let params = match serde_json::to_value(StudioGateRequest { module, source }) {
            Ok(params) => params,
            Err(err) => {
                self.error = Some(format!("Studio gate: {err}").into());
                cx.notify();
                return;
            }
        };
        self.gate_task = Some(cx.spawn(async move |this, cx| {
            let stream = engine
                .client()
                .subscribe(methods::STUDIO_GATE, params)
                .await;
            match stream {
                Ok(mut rx) => {
                    while let Some(value) = rx.recv().await {
                        match serde_json::from_value::<StudioGateEvent>(value) {
                            Ok(event) => {
                                this.update(cx, |view, cx| {
                                    view.apply_gate_event(&gate_row_id, event, cx);
                                })
                                .ok();
                            }
                            Err(err) => {
                                this.update(cx, |view, cx| {
                                    view.error = Some(format!("Studio gate event: {err}").into());
                                    cx.notify();
                                })
                                .ok();
                            }
                        }
                    }
                }
                Err(err) => {
                    this.update(cx, |view, cx| {
                        view.fail_gate(&gate_row_id, format!("{err}"), cx);
                    })
                    .ok();
                }
            }
        }));
    }

    fn apply_gate_event(&mut self, row_id: &str, event: StudioGateEvent, cx: &mut Context<Self>) {
        if let Some(StudioRow {
            kind: StudioRowKind::Gate(card),
            ..
        }) = self.rows.iter_mut().find(|row| row.id == row_id)
        {
            reduce_gate_event(card, event);
        }
        self.persist(cx);
        cx.notify();
    }

    fn fail_gate(&mut self, row_id: &str, error: String, cx: &mut Context<Self>) {
        if let Some(StudioRow {
            kind: StudioRowKind::Gate(card),
            ..
        }) = self.rows.iter_mut().find(|row| row.id == row_id)
        {
            card.state = StudioGateState::Fail;
            card.error = Some(error);
        }
        self.persist(cx);
        cx.notify();
    }

    fn toggle_source(&mut self, row_id: &str, cx: &mut Context<Self>) {
        if let Some(StudioRow {
            kind: StudioRowKind::Draft { source_open, .. },
            ..
        }) = self.rows.iter_mut().find(|row| row.id == row_id)
        {
            *source_open = !*source_open;
        }
        cx.notify();
    }

    fn toggle_stage(&mut self, row_id: &str, stage: StudioGateStage, cx: &mut Context<Self>) {
        if let Some(StudioRow {
            kind: StudioRowKind::Gate(card),
            ..
        }) = self.rows.iter_mut().find(|row| row.id == row_id)
            && let Some(view) = card.stage_mut(stage)
        {
            view.expanded = !view.expanded;
        }
        cx.notify();
    }

    fn render_row(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows.get(ix).cloned() else {
            return div().into_any_element();
        };
        match row.kind {
            StudioRowKind::User { text } => self.render_user(&text, cx),
            StudioRowKind::Draft {
                round,
                program,
                fields,
                source,
                note,
                source_open,
            } => self.render_draft(
                &row.id,
                round,
                &program,
                &fields,
                &source,
                note.as_deref(),
                source_open,
                window,
                cx,
            ),
            StudioRowKind::Gate(card) => self.render_gate(&row.id, &card, cx),
            StudioRowKind::Note { text } => self.render_note(&text, cx),
        }
    }

    fn render_user(&self, text: &str, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx);
        div()
            .w_full()
            .flex()
            .justify_end()
            .px(px(24.0))
            .py(px(7.0))
            .child(
                div()
                    .max_w(px(620.0))
                    .rounded(px(14.0))
                    .bg(theme.surface_card)
                    .border_1()
                    .border_color(theme.border)
                    .px(px(14.0))
                    .py(px(10.0))
                    .text_size(px(14.0))
                    .line_height(px(21.0))
                    .text_color(theme.text)
                    .child(SharedString::from(text.to_string())),
            )
            .into_any_element()
    }

    fn render_note(&self, text: &str, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx);
        div()
            .w_full()
            .px(px(24.0))
            .py(px(6.0))
            .child(
                div()
                    .max_w(px(736.0))
                    .mx_auto()
                    .text_size(px(12.0))
                    .line_height(px(18.0))
                    .text_color(theme.text_muted)
                    .child(SharedString::from(text.to_string())),
            )
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_draft(
        &self,
        row_id: &str,
        round: Option<u32>,
        program: &str,
        fields: &StudioDraftFields,
        source: &str,
        note: Option<&str>,
        source_open: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::of(cx).clone();
        let row_id_owned = row_id.to_string();
        let source_md = format!("```lean\n{source}\n```");
        let tree = parse_full(&source_md);
        div()
            .w_full()
            .px(px(24.0))
            .py(px(8.0))
            .child(
                card(&theme)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.text)
                                    .child(SharedString::from(program.to_string())),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(6.0))
                                    .text_size(px(11.0))
                                    .text_color(theme.text_muted)
                                    .when_some(round, |el, round| {
                                        el.child(round_badge(round, &theme))
                                    })
                                    .child("draft"),
                            ),
                    )
                    .when(!fields.is_empty(), |el| {
                        el.child(
                            div()
                                .mt(px(12.0))
                                .border_1()
                                .border_color(theme.border)
                                .rounded(px(10.0))
                                .overflow_hidden()
                                .children(fields.iter().map(|(k, v)| {
                                    div()
                                        .flex()
                                        .border_b_1()
                                        .border_color(theme.border)
                                        .child(cell(&theme, k, true))
                                        .child(cell(&theme, v, false))
                                })),
                        )
                    })
                    .child(
                        div()
                            .mt(px(12.0))
                            .text_size(px(12.0))
                            .text_color(theme.text_muted)
                            .cursor_pointer()
                            .on_mouse_up(
                                gpui::MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.toggle_source(&row_id_owned, cx)
                                }),
                            )
                            .child(if source_open {
                                "Hide source"
                            } else {
                                "Show source"
                            }),
                    )
                    .when(source_open, |el| {
                        el.child(div().mt(px(8.0)).child(render_tree(
                            &tree,
                            &RenderOptions::settled(SharedString::from(format!("draft-{row_id}"))),
                            &theme,
                            window,
                            &|_| None::<std::sync::Arc<Vec<Vec<Token>>>>,
                        )))
                    })
                    .when_some(note, |el, note| {
                        el.child(
                            div()
                                .mt(px(10.0))
                                .text_size(px(12.0))
                                .text_color(theme.text_muted)
                                .child(SharedString::from(note.to_string())),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_gate(
        &self,
        row_id: &str,
        card_state: &GateCard,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::of(cx).clone();
        let row_id_owned = row_id.to_string();
        card(&theme)
            .mx(px(24.0))
            .my(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .text_size(px(15.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child("ProofForge gate")
                            .when_some(card_state.round, |el, round| {
                                el.child(round_badge(round, &theme))
                            }),
                    )
                    .child(status_label(card_state.state, &theme)),
            )
            .child(div().mt(px(12.0)).flex().flex_col().gap(px(8.0)).children(
                card_state.stages.iter().map(|stage| {
                    let rid = row_id_owned.clone();
                    self.render_stage(rid, stage, cx)
                }),
            ))
            .when(card_state.state == StudioGateState::Pass, |el| {
                el.child(self.render_artifacts(card_state, cx))
            })
            .when(card_state.state == StudioGateState::Fail, |el| {
                el.child(
                    div()
                        .mt(px(12.0))
                        .text_size(px(12.0))
                        .text_color(theme.danger)
                        .child(format!(
                            "Failed at {}",
                            card_state.failed_stage.map(stage_name).unwrap_or("gate")
                        )),
                )
            })
            .into_any_element()
    }

    fn render_stage(
        &self,
        row_id: String,
        stage: &StageView,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::of(cx).clone();
        let stage_id = stage.stage;
        div()
            .rounded(px(10.0))
            .border_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(10.0))
                    .py(px(8.0))
                    .cursor_pointer()
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(move |this, _, _, cx| this.toggle_stage(&row_id, stage_id, cx)),
                    )
                    .child(stage_icon(stage.status, &theme, cx))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(13.0))
                            .text_color(theme.text)
                            .child(stage_name(stage.stage)),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.text_muted)
                            .child(stage_status_label(stage.status)),
                    ),
            )
            .when(stage.expanded, |el| {
                el.child(
                    div()
                        .border_t_1()
                        .border_color(theme.border)
                        .px(px(10.0))
                        .py(px(8.0))
                        .font_family(theme.font_mono.clone())
                        .text_size(px(11.0))
                        .line_height(px(16.0))
                        .text_color(theme.text_muted)
                        .child(SharedString::from(stage.output.clone().unwrap_or_default())),
                )
            })
            .into_any_element()
    }

    fn render_artifacts(&self, card_state: &GateCard, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx);
        div()
            .mt(px(12.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .children(card_state.artifacts.iter().map(|artifact| {
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.0))
                    .text_color(theme.text_muted)
                    .child(SharedString::from(artifact.name.clone()))
                    .child(SharedString::from(human_size(artifact.size)))
            }))
            .when_some(
                card_state.digest.output_set_digest.as_deref(),
                |el, digest| {
                    el.child(
                        div()
                            .mt(px(4.0))
                            .font_family(theme.font_mono.clone())
                            .text_size(px(11.0))
                            .text_color(theme.text_dim)
                            .child(format!("outputSetDigest {}", truncate_middle(digest, 14))),
                    )
                },
            )
            .into_any_element()
    }

    fn render_lane_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx).clone();
        let selected = self.selected_lane;
        let label = selected.map(lane_label).unwrap_or("No lane");
        let mut trigger = button_dynamic(format!("Lane: {label}"), selected.is_none(), &theme)
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.lane_menu_open = !this.lane_menu_open;
                    if matches!(this.harnesses, Loadable::Idle | Loadable::Error(_)) {
                        this.load_harnesses(cx);
                    }
                    cx.notify();
                }),
            );
        if self.lane_menu_open {
            let menu = self.render_lane_menu(cx);
            trigger = trigger.child(popover::anchored_menu_above("studio-lane-menu", menu, None));
        }
        trigger.into_any_element()
    }

    fn render_lane_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx).clone();
        match &self.harnesses {
            Loadable::Ready(list) => {
                let offered = studio_offered_harnesses(list);
                div()
                    .w(px(220.0))
                    .children(offered.into_iter().enumerate().map(|(ix, descriptor)| {
                        let active = self.selected_lane == Some(descriptor.id);
                        let lane = descriptor.id;
                        let (icon_path, tint) = harness_brand_icon(lane);
                        popover::menu_row(&theme, active, format!("studio-lane-row-{ix}"))
                            .id(("studio-lane-row", ix))
                            .child(
                                icon(icon_path)
                                    .size(px(14.0))
                                    .text_color(tint.unwrap_or(theme.text_muted)),
                            )
                            .child(div().flex_1().child(SharedString::from(descriptor.name)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected_lane = Some(lane);
                                this.lane_menu_open = false;
                                this.save_studio_default(cx);
                                cx.notify();
                            }))
                    }))
                    .into_any_element()
            }
            Loadable::Error(err) => div()
                .w(px(220.0))
                .p(px(10.0))
                .text_size(px(12.0))
                .text_color(theme.danger)
                .child(SharedString::from(err.clone()))
                .into_any_element(),
            _ => div()
                .w(px(220.0))
                .p(px(10.0))
                .text_size(px(12.0))
                .text_color(theme.text_muted)
                .child("Loading lanes…")
                .into_any_element(),
        }
    }

    fn render_status_banner(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.status_dismissed {
            return None;
        }
        let theme = Theme::of(cx).clone();
        let message = match (&self.status, &self.status_error) {
            (Some(status), _) if !status.toolchain_ok || !status.cli_resolved => status
                .error
                .clone()
                .unwrap_or_else(|| "ProofForge toolchain is not resolved".into()),
            (_, Some(err)) => err.to_string(),
            _ => return None,
        };
        Some(
            div()
                .mx(px(24.0))
                .mt(px(16.0))
                .rounded(px(12.0))
                .border_1()
                .border_color(theme.border_strong)
                .bg(theme.surface_card)
                .px(px(12.0))
                .py(px(10.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(
                    icon(icons::CLOSE_CIRCLE)
                        .size(px(16.0))
                        .text_color(theme.danger),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.0))
                        .line_height(px(18.0))
                        .text_color(theme.text_muted)
                        .child(format!("{message} · install with {INSTALL_COMMAND}")),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.text)
                        .cursor_pointer()
                        .on_mouse_up(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.status_dismissed = true;
                                cx.notify();
                            }),
                        )
                        .child("Dismiss"),
                )
                .into_any_element(),
        )
    }

    fn render_composer(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::of(cx).clone();
        let can_submit = !self.composer.read(cx).text().trim().is_empty();
        div()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.bg)
            .p(px(12.0))
            .child(
                div()
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.surface_card)
                    .px(px(12.0))
                    .py(px(10.0))
                    .child(self.composer.clone())
                    .child(
                        div()
                            .mt(px(10.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.0))
                                    .child(self.render_lane_picker(cx))
                                    .child(button("Gate source…", false, &theme).on_mouse_up(
                                        gpui::MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.open_gate_dialog(cx)),
                                    ))
                                    .child(button("Load sample", false, &theme).on_mouse_up(
                                        gpui::MouseButton::Left,
                                        cx.listener(|this, _, _, cx| this.load_sample(cx)),
                                    )),
                            )
                            .child(button("Submit", !can_submit, &theme).on_mouse_up(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.submit_nl(cx)),
                            )),
                    ),
            )
            .into_any_element()
    }

    fn render_gate_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let dialog = self.gate_dialog.as_ref()?;
        let theme = Theme::of(cx).clone();
        Some(
            div()
                .absolute()
                .inset_0()
                .bg(theme.scrim().opacity(0.6))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(720.0))
                        .max_w_full()
                        .rounded(px(16.0))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.surface_dialog)
                        .p(px(16.0))
                        .child(
                            div()
                                .text_size(px(15.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.text)
                                .child("Gate source"),
                        )
                        .child(div().mt(px(10.0)).child(dialog.module.clone()))
                        .child(div().mt(px(10.0)).h(px(280.0)).child(dialog.source.clone()))
                        .when_some(dialog.error.as_ref(), |el, err| {
                            el.child(
                                div()
                                    .mt(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(theme.danger)
                                    .child(err.clone()),
                            )
                        })
                        .child(
                            div()
                                .mt(px(12.0))
                                .flex()
                                .justify_end()
                                .gap(px(8.0))
                                .child(button("Cancel", false, &theme).on_mouse_up(
                                    gpui::MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        this.gate_dialog = None;
                                        cx.notify();
                                    }),
                                ))
                                .child(button("Run gate", false, &theme).on_mouse_up(
                                    gpui::MouseButton::Left,
                                    cx.listener(|this, _, _, cx| this.submit_gate_dialog(cx)),
                                )),
                        ),
                )
                .into_any_element(),
        )
    }
}

impl Render for StudioView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if window.focused(cx).is_none() {
            window.focus(&self.focus_handle(cx), cx);
        }
        let theme = Theme::of(cx).clone();
        let banner = self.render_status_banner(cx);
        let dialog = self.render_gate_dialog(cx);
        div()
            .id("studio-view")
            .relative()
            .size_full()
            .bg(theme.bg)
            .flex()
            .flex_col()
            .child(
                div()
                    .pt(px(12.0))
                    .px(px(24.0))
                    .text_size(px(18.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.text)
                    .child("Launch Studio"),
            )
            .when_some(banner, |el, banner| el.child(banner))
            .child(div().flex_1().min_h_0().child(if self.rows.is_empty() {
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(13.0))
                    .text_color(theme.text_muted)
                    .child("Describe a launch, load the sample, or paste a source to run the gate.")
                    .into_any_element()
            } else {
                list(self.list.clone(), cx.processor(Self::render_row))
                    .size_full()
                    .with_sizing_behavior(gpui::ListSizingBehavior::Auto)
                    .into_any_element()
            }))
            .child(self.render_composer(cx))
            .when_some(dialog, |el, dialog| el.child(dialog))
    }
}

pub fn apply_launch_event_to_rows(rows: &mut Vec<StudioRow>, event: StudioLaunchRunEvent) {
    match event {
        StudioLaunchRunEvent::Draft { round, event, .. } => {
            if let Some(result) = draft_done_row_with_round(rows.len(), Some(round), event) {
                match result {
                    Ok(row) => rows.push(row),
                    Err(error) => rows.push(StudioRow {
                        id: format!("note-{}", rows.len()),
                        at: now(),
                        kind: StudioRowKind::Note {
                            text: format!("Draft failed: {error}"),
                        },
                    }),
                }
            }
        }
        StudioLaunchRunEvent::Gate { round, event, .. } => {
            let row_id = format!("gate-r{round}");
            let gate_ix = rows
                .iter()
                .position(|row| row.id == row_id)
                .unwrap_or_else(|| {
                    let mut card = GateCard::running();
                    card.round = Some(round);
                    rows.push(StudioRow {
                        id: row_id.clone(),
                        at: now(),
                        kind: StudioRowKind::Gate(card),
                    });
                    rows.len() - 1
                });
            if let StudioRowKind::Gate(card) = &mut rows[gate_ix].kind {
                reduce_gate_event(card, event);
            }
        }
        StudioLaunchRunEvent::Done {
            ok: false,
            exhausted: true,
            ..
        } => rows.push(StudioRow {
            id: format!("note-{}", rows.len()),
            at: now(),
            kind: StudioRowKind::Note {
                text: "repair exhausted after 4 rounds — last diagnostics above".into(),
            },
        }),
        StudioLaunchRunEvent::Done { .. } => {}
    }
}

fn draft_row(ix: usize, program: String, source: String, note: Option<String>) -> StudioRow {
    StudioRow {
        id: format!("draft-{ix}"),
        at: now(),
        kind: StudioRowKind::Draft {
            round: None,
            program,
            fields: default_fields(),
            source,
            note,
            source_open: false,
        },
    }
}

fn default_fields() -> StudioDraftFields {
    let mut fields = BTreeMap::new();
    fields.insert("kind".into(), "ProgramV1".into());
    fields
}

fn card(theme: &Theme) -> gpui::Div {
    div()
        .max_w(px(736.0))
        .mx_auto()
        .rounded(px(14.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface_card)
        .p(px(14.0))
}

fn cell(theme: &Theme, text: &str, key: bool) -> AnyElement {
    div()
        .flex_1()
        .px(px(10.0))
        .py(px(7.0))
        .text_size(px(12.0))
        .text_color(if key { theme.text_muted } else { theme.text })
        .child(SharedString::from(text.to_string()))
        .into_any_element()
}

fn button(label: &'static str, disabled: bool, theme: &Theme) -> gpui::Div {
    button_dynamic(label.to_string(), disabled, theme)
}

fn button_dynamic(label: String, disabled: bool, theme: &Theme) -> gpui::Div {
    div()
        .rounded(px(9.0))
        .border_1()
        .border_color(theme.border)
        .px(px(10.0))
        .py(px(6.0))
        .text_size(px(12.0))
        .text_color(if disabled {
            theme.text_faint
        } else {
            theme.text
        })
        .bg(if disabled {
            theme.bg
        } else {
            theme.surface_raised
        })
        .cursor_pointer()
        .child(SharedString::from(label))
}

fn stage_icon(
    status: StudioStageStatus,
    theme: &Theme,
    cx: &mut Context<StudioView>,
) -> AnyElement {
    match status {
        StudioStageStatus::Pending => div()
            .size(px(14.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(theme.border)
            .into_any_element(),
        StudioStageStatus::Running => {
            crate::loaders::mini_gradient_spinner("studio-stage", 2.5, cx.entity_id(), cx)
                .into_any_element()
        }
        StudioStageStatus::Pass => icon(icons::CHECK)
            .size(px(14.0))
            .text_color(theme.success)
            .into_any_element(),
        StudioStageStatus::Fail => icon(icons::CLOSE)
            .size(px(14.0))
            .text_color(theme.danger)
            .into_any_element(),
    }
}

fn round_badge(round: u32, theme: &Theme) -> AnyElement {
    div()
        .rounded(px(999.0))
        .px(px(6.0))
        .py(px(2.0))
        .text_size(px(10.0))
        .text_color(theme.text_muted)
        .bg(theme.element_hover)
        .child(format!("R{round}"))
        .into_any_element()
}

fn status_label(state: StudioGateState, theme: &Theme) -> AnyElement {
    div()
        .rounded(px(999.0))
        .px(px(8.0))
        .py(px(3.0))
        .text_size(px(11.0))
        .text_color(match state {
            StudioGateState::Pass => theme.success,
            StudioGateState::Fail => theme.danger,
            _ => theme.text_muted,
        })
        .bg(theme.element_hover)
        .child(match state {
            StudioGateState::Running => "running",
            StudioGateState::Pass => "pass",
            StudioGateState::Fail => "fail",
            StudioGateState::Offline => "offline",
        })
        .into_any_element()
}

fn stage_status_label(status: StudioStageStatus) -> &'static str {
    match status {
        StudioStageStatus::Pending => "pending",
        StudioStageStatus::Running => "running",
        StudioStageStatus::Pass => "pass",
        StudioStageStatus::Fail => "fail",
    }
}

fn studio_offered_harnesses(list: &[HarnessDescriptor]) -> Vec<HarnessDescriptor> {
    let visible = visible_harnesses(list);
    let offered: Vec<_> = visible
        .iter()
        .filter(|d| {
            comet_engine::registry::descriptor_enabled(d)
                || (std::env::var("COMET_HARNESS").ok().as_deref() == Some("mock")
                    && d.id == HarnessId::Mock)
        })
        .cloned()
        .collect();
    if offered.is_empty() { visible } else { offered }
}

fn lane_label(lane: HarnessId) -> &'static str {
    match lane {
        HarnessId::ClaudeCode => "Claude Code",
        HarnessId::Codex => "Codex",
        HarnessId::Cursor => "Cursor",
        HarnessId::OpenCode => "OpenCode",
        HarnessId::Grok => "Grok",
        HarnessId::Hermes => "Hermes",
        HarnessId::Pi => "pi",
        HarnessId::Mock => "Mock",
    }
}

fn stage_name(stage: StudioGateStage) -> &'static str {
    match stage {
        StudioGateStage::Check => "check",
        StudioGateStage::Build => "build",
        StudioGateStage::Inspect => "inspect",
        StudioGateStage::Done => "done",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_reducer_marks_failure_stage_and_output() {
        let mut card = GateCard::running();
        reduce_gate_event(
            &mut card,
            StudioGateEvent::Started {
                stage: StudioGateStage::Check,
            },
        );
        assert_eq!(card.stages[0].status, StudioStageStatus::Running);
        reduce_gate_event(
            &mut card,
            StudioGateEvent::StageDone {
                stage: StudioGateStage::Check,
                ok: false,
                output: "PF-001 failed".into(),
            },
        );
        assert_eq!(card.state, StudioGateState::Fail);
        assert_eq!(card.failed_stage, Some(StudioGateStage::Check));
        assert_eq!(card.stages[0].status, StudioStageStatus::Fail);
        assert!(card.stages[0].expanded);
        assert_eq!(card.stages[0].output.as_deref(), Some("PF-001 failed"));
    }

    #[test]
    fn gate_reducer_done_pass_records_artifacts_and_digest() {
        let mut card = GateCard::running();
        reduce_gate_event(
            &mut card,
            StudioGateEvent::Done {
                ok: true,
                stage: StudioGateStage::Done,
                artifacts: vec![StudioGateArtifact {
                    name: "manifest.json".into(),
                    size: 42,
                }],
                digest: StudioGateDigest {
                    output_set_digest: Some("abc".into()),
                    raw: "abc".into(),
                },
            },
        );
        assert_eq!(card.state, StudioGateState::Pass);
        assert_eq!(card.artifacts[0].name, "manifest.json");
        assert_eq!(card.digest.output_set_digest.as_deref(), Some("abc"));
    }

    #[test]
    fn running_gate_from_store_becomes_offline_row() {
        let launch = StudioLaunch {
            id: "l".into(),
            title: "t".into(),
            created_at: "2026-08-12T00:00:00Z".into(),
            msgs: vec![StudioChatMsg::AgentGate(StudioGateMsg {
                role: "agent".into(),
                kind: "gate".into(),
                state: StudioGateState::Running,
                result: None,
                at: "2026-08-12T00:00:00Z".into(),
            })],
            fields: None,
            program: None,
            source: None,
        };
        let rows = rows_from_launch(&launch);
        let StudioRowKind::Gate(card) = &rows[0].kind else {
            panic!("expected gate row");
        };
        assert_eq!(card.state, StudioGateState::Offline);
    }

    #[test]
    fn rows_round_trip_into_launch_messages() {
        let rows = vec![
            StudioRow {
                id: "u".into(),
                at: "2026-08-12T00:00:00Z".into(),
                kind: StudioRowKind::User {
                    text: "Ship".into(),
                },
            },
            draft_row(1, "Demo".into(), "import ProofForgeV2".into(), None),
        ];
        let launch = launch_from_rows(None, &rows);
        assert_eq!(launch.msgs.len(), 2);
        assert_eq!(launch.program.as_deref(), Some("Demo"));
        assert_eq!(rows_from_launch(&launch).len(), 2);
    }

    #[test]
    fn middle_truncation_keeps_ends() {
        assert_eq!(
            truncate_middle("abcdefghijklmnopqrstuvwxyz", 4),
            "abcd…wxyz"
        );
    }

    #[test]
    fn human_sizes_are_readable() {
        assert_eq!(human_size(42), "42 B");
        assert_eq!(human_size(2048), "2.0 KB");
    }

    #[test]
    fn draft_done_maps_to_draft_card() {
        let row = draft_done_row(
            0,
            StudioDraftEvent::Done {
                ok: true,
                lane: HarnessId::Codex,
                module: Some("Escrow".into()),
                source: Some("import ProofForgeV2".into()),
                error: None,
            },
        )
        .unwrap()
        .unwrap();
        let StudioRowKind::Draft {
            program,
            fields,
            note,
            ..
        } = row.kind
        else {
            panic!("expected draft");
        };
        assert_eq!(program, "Escrow");
        assert!(fields.is_empty());
        assert_eq!(note.as_deref(), Some("drafted by Codex"));
    }

    #[test]
    fn draft_failure_maps_to_note_error() {
        let result = draft_done_row(
            0,
            StudioDraftEvent::Done {
                ok: false,
                lane: HarnessId::Grok,
                module: None,
                source: None,
                error: Some("missing CLI".into()),
            },
        )
        .unwrap();
        assert_eq!(result.unwrap_err(), "missing CLI");
    }

    #[test]
    fn draft_then_gate_appends_gate_and_returns_source() {
        let mut rows = Vec::new();
        let draft = draft_row(0, "Escrow".into(), "import ProofForgeV2".into(), None);
        let (module, source) = draft_then_gate_sequence(&mut rows, draft).unwrap();
        assert_eq!(
            (module.as_str(), source.as_str()),
            ("Escrow", "import ProofForgeV2")
        );
        assert!(matches!(rows[0].kind, StudioRowKind::Draft { .. }));
        assert!(matches!(rows[1].kind, StudioRowKind::Gate(_)));
    }

    #[test]
    fn launch_run_multi_round_stream_creates_round_rows() {
        let mut rows = Vec::new();
        apply_launch_event_to_rows(
            &mut rows,
            StudioLaunchRunEvent::Draft {
                round: 1,
                phase: comet_proto::studio::StudioLaunchRunPhase::Draft,
                event: StudioDraftEvent::Done {
                    ok: true,
                    lane: HarnessId::Codex,
                    module: Some("Demo".into()),
                    source: Some("import ProofForgeV2".into()),
                    error: None,
                },
            },
        );
        apply_launch_event_to_rows(
            &mut rows,
            StudioLaunchRunEvent::Gate {
                round: 1,
                phase: comet_proto::studio::StudioLaunchRunPhase::Gate,
                event: StudioGateEvent::StageDone {
                    stage: StudioGateStage::Check,
                    ok: false,
                    output: "PF-001".into(),
                },
            },
        );
        apply_launch_event_to_rows(
            &mut rows,
            StudioLaunchRunEvent::Draft {
                round: 2,
                phase: comet_proto::studio::StudioLaunchRunPhase::Draft,
                event: StudioDraftEvent::Done {
                    ok: true,
                    lane: HarnessId::Codex,
                    module: Some("Demo".into()),
                    source: Some("import ProofForgeV2\n-- fixed".into()),
                    error: None,
                },
            },
        );
        assert!(matches!(
            &rows[0].kind,
            StudioRowKind::Draft { round: Some(1), .. }
        ));
        assert!(matches!(&rows[1].kind, StudioRowKind::Gate(card) if card.round == Some(1)));
        assert!(matches!(
            &rows[2].kind,
            StudioRowKind::Draft { round: Some(2), .. }
        ));
    }

    #[test]
    fn launch_run_pass_round_two_stops_without_exhaustion_note() {
        let mut rows = Vec::new();
        apply_launch_event_to_rows(
            &mut rows,
            StudioLaunchRunEvent::Gate {
                round: 2,
                phase: comet_proto::studio::StudioLaunchRunPhase::Gate,
                event: StudioGateEvent::Done {
                    ok: true,
                    stage: StudioGateStage::Done,
                    artifacts: vec![],
                    digest: StudioGateDigest::default(),
                },
            },
        );
        apply_launch_event_to_rows(
            &mut rows,
            StudioLaunchRunEvent::Done {
                ok: true,
                round: 2,
                module: Some("Demo".into()),
                source: Some("import ProofForgeV2".into()),
                artifacts: vec![],
                digest: StudioGateDigest::default(),
                last_diagnostics: None,
                exhausted: false,
            },
        );
        assert_eq!(rows.len(), 1);
        assert!(
            matches!(&rows[0].kind, StudioRowKind::Gate(card) if card.state == StudioGateState::Pass)
        );
    }

    #[test]
    fn launch_run_exhaustion_renders_note() {
        let mut rows = Vec::new();
        apply_launch_event_to_rows(
            &mut rows,
            StudioLaunchRunEvent::Done {
                ok: false,
                round: 4,
                module: Some("Demo".into()),
                source: Some("import ProofForgeV2".into()),
                artifacts: vec![],
                digest: StudioGateDigest::default(),
                last_diagnostics: Some("PF-999".into()),
                exhausted: true,
            },
        );
        assert!(
            matches!(&rows[0].kind, StudioRowKind::Note { text } if text == "repair exhausted after 4 rounds — last diagnostics above")
        );
    }
}
