//! Optional host-backed search across data outside the currently rendered scene.
use crate::{
    focus::{FocusScope, FrameRequest},
    interaction::{LocalizeQuery, SelectionState},
};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use hyper_viz::HypergraphScene;
use std::sync::{Arc, Mutex, mpsc};

pub struct SearchRow {
    pub label: String,
    pub text: String,
    /// Exact query used when the user selects this row.
    pub query: String,
}
pub struct SearchPage {
    pub summary: String,
    pub total: usize,
    pub offset: usize,
    pub rows: Vec<SearchRow>,
    pub scene: HypergraphScene,
}
pub type SearchProvider = Arc<dyn Fn(&str, usize) -> Result<SearchPage, String> + Send + Sync>;

#[derive(Resource)]
pub(crate) struct SearchState {
    requests: mpsc::SyncSender<(usize, String, usize)>,
    results: Mutex<mpsc::Receiver<Result<SearchPage, String>>>,
    modes: Vec<String>,
    mode: usize,
    submitted_mode: usize,
    displayed_mode: usize,
    query: String,
    submitted: String,
    displayed: String,
    busy: bool,
    error: Option<String>,
    page: Option<SearchPage>,
    pub active: bool,
    showing_results: bool,
    pub pending_scene: Option<HypergraphScene>,
    pub live_scene: Option<HypergraphScene>,
}
impl SearchState {
    pub fn new(providers: Vec<(String, SearchProvider)>, initial: Option<HypergraphScene>) -> Self {
        let (requests, rx) = mpsc::sync_channel::<(usize, String, usize)>(1);
        let (tx, results) = mpsc::sync_channel(1);
        let modes = providers.iter().map(|(name, _)| name.clone()).collect();
        std::thread::spawn(move || {
            while let Ok((mode, query, offset)) = rx.recv() {
                if tx
                    .send(
                        providers
                            .get(mode)
                            .ok_or_else(|| "No search mode configured".to_string())
                            .and_then(|(_, provider)| provider(&query, offset)),
                    )
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            modes,
            mode: 0,
            submitted_mode: 0,
            displayed_mode: 0,
            requests,
            results: Mutex::new(results),
            query: String::new(),
            submitted: String::new(),
            displayed: String::new(),
            busy: false,
            error: None,
            page: None,
            active: false,
            showing_results: false,
            pending_scene: None,
            live_scene: initial,
        }
    }
    fn submit(&mut self, mode: usize, query: String, offset: usize) {
        if self.busy || query.trim().is_empty() {
            return;
        }
        match self.requests.try_send((mode, query.clone(), offset)) {
            Ok(()) => {
                self.submitted_mode = mode;
                self.submitted = query;
                self.busy = true;
                self.error = None;
            }
            Err(_) => self.error = Some("Search worker unavailable".into()),
        }
    }
    pub fn scene_update(&mut self, live: Option<HypergraphScene>) -> Option<HypergraphScene> {
        if let Some(scene) = &live {
            self.live_scene = Some(scene.clone());
        }
        if self.active {
            let result = self.pending_scene.take();
            self.showing_results |= result.is_some();
            result
        } else if self.showing_results {
            self.showing_results = false;
            self.live_scene.clone()
        } else {
            live
        }
    }
}

pub(crate) fn search_panel(
    mut contexts: EguiContexts,
    mut state: ResMut<SearchState>,
    mut focus: ResMut<FocusScope>,
    mut selection: ResMut<SelectionState>,
    mut localize: ResMut<LocalizeQuery>,
    mut frame: ResMut<FrameRequest>,
) {
    let result = state.results.lock().ok().and_then(|rx| rx.try_recv().ok());
    if let Some(result) = result {
        state.busy = false;
        match result {
            Ok(page) => {
                state.pending_scene = Some(page.scene.clone());
                state.active = true;
                state.page = Some(page);
                state.displayed = state.submitted.clone();
                state.displayed_mode = state.submitted_mode;
                focus.clear();
                selection.clear();
                localize.query.clear();
                frame.pending = true;
            }
            Err(error) => state.error = Some(error),
        }
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let mut submit = None;
    egui::Window::new("Search repository")
        .order(egui::Order::Foreground)
        .default_pos(egui::pos2(380.0, 300.0))
        .default_width(560.0)
        .default_height(350.0)
        .show(ctx, |ui| {
            ui.label("Search all stored source lines · text, symbols, questions, or path:line");
            ui.add_enabled_ui(!state.busy, |ui| {
                egui::ComboBox::from_label("Retrieval")
                    .selected_text(
                        state
                            .modes
                            .get(state.mode)
                            .map(String::as_str)
                            .unwrap_or("None"),
                    )
                    .show_ui(ui, |ui| {
                        for (index, label) in state.modes.clone().iter().enumerate() {
                            ui.selectable_value(&mut state.mode, index, label);
                        }
                    });
            });
            ui.horizontal(|ui| {
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .desired_width(420.0)
                        .hint_text("Where are bulk transactions committed?"),
                );
                if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
                    edit.request_focus();
                }
                let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui
                    .add_enabled(!state.busy, egui::Button::new("Search"))
                    .clicked()
                    || enter
                {
                    submit = Some((state.mode, state.query.clone(), 0));
                }
            });
            if state.busy {
                ui.spinner();
                ui.label("Searching the full snapshot…");
            }
            if let Some(error) = &state.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            if state.active
                && ui
                    .add_enabled(!state.busy, egui::Button::new("Return to live window"))
                    .clicked()
            {
                state.active = false;
                state.pending_scene = None;
                state.page = None;
                focus.clear();
                selection.clear();
                frame.pending = true;
            }
            if let Some(page) = &state.page {
                ui.label(format!(
                    "{} · Query: {}",
                    state
                        .modes
                        .get(state.displayed_mode)
                        .map(String::as_str)
                        .unwrap_or("Search"),
                    state.displayed
                ));
                ui.label(&page.summary);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !state.busy && page.offset > 0,
                            egui::Button::new("Previous"),
                        )
                        .clicked()
                    {
                        submit = Some((
                            state.displayed_mode,
                            state.displayed.clone(),
                            page.offset.saturating_sub(40),
                        ));
                    }
                    if ui
                        .add_enabled(
                            !state.busy && page.offset + page.rows.len() < page.total,
                            egui::Button::new("Next"),
                        )
                        .clicked()
                    {
                        submit = Some((
                            state.displayed_mode,
                            state.displayed.clone(),
                            page.offset + page.rows.len(),
                        ));
                    }
                });
                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        for row in &page.rows {
                            if ui
                                .add_enabled(!state.busy, egui::Button::new(&row.label))
                                .clicked()
                            {
                                submit = Some((state.displayed_mode, row.query.clone(), 0));
                            }
                            ui.add(
                                egui::Label::new(egui::RichText::new(&row.text).monospace())
                                    .wrap()
                                    .selectable(true),
                            );
                            ui.separator();
                        }
                    });
            }
        });
    if let Some((mode, query, offset)) = submit {
        state.submit(mode, query, offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_dispatches_selected_mode_and_keeps_errors_explicit() {
        let mut state = SearchState::new(
            vec![
                ("Lexical".into(), Arc::new(|_, _| Err("baseline".into()))),
                (
                    "LAYA".into(),
                    Arc::new(|_, _| Err("model unavailable".into())),
                ),
            ],
            None,
        );
        state.submit(1, "query".into(), 0);
        state.mode = 0;
        let result = state
            .results
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert!(matches!(result, Err(error) if error == "model unavailable"));
        assert_eq!(state.submitted_mode, 1);
    }

    #[test]
    fn search_scene_stays_pinned_while_live_updates_arrive() {
        let mut state = SearchState::new(
            vec![("Test".into(), Arc::new(|_, _| Err("unused".into())))],
            None,
        );
        let scene = |id| {
            hyper_viz::Hypergraph::new()
                .with_id(id)
                .project(hyper_viz::Projection::Bipartite)
        };
        state.active = true;
        state.pending_scene = Some(scene("result"));
        assert_eq!(
            state.scene_update(Some(scene("live-1"))).unwrap().meta.id,
            "result"
        );
        assert!(state.scene_update(Some(scene("live-2"))).is_none());
        state.active = false;
        assert_eq!(state.scene_update(None).unwrap().meta.id, "live-2");
    }

    #[test]
    fn static_initial_scene_returns_after_search_without_new_ingestion() {
        let scene = |id| {
            hyper_viz::Hypergraph::new()
                .with_id(id)
                .project(hyper_viz::Projection::Bipartite)
        };
        let mut state = SearchState::new(
            vec![("Test".into(), Arc::new(|_, _| Err("unused".into())))],
            Some(scene("snapshot")),
        );
        assert!(state.scene_update(None).is_none());
        state.active = true;
        state.pending_scene = Some(scene("results"));
        assert_eq!(state.scene_update(None).unwrap().meta.id, "results");
        state.active = false;
        assert_eq!(state.scene_update(None).unwrap().meta.id, "snapshot");
        assert!(state.scene_update(None).is_none());
    }
}
