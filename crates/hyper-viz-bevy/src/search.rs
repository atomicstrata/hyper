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
    requests: mpsc::SyncSender<(String, usize)>,
    results: Mutex<mpsc::Receiver<Result<SearchPage, String>>>,
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
    pub fn new(provider: SearchProvider, initial: Option<HypergraphScene>) -> Self {
        let (requests, rx) = mpsc::sync_channel::<(String, usize)>(1);
        let (tx, results) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            while let Ok((query, offset)) = rx.recv() {
                if tx.send(provider(&query, offset)).is_err() {
                    break;
                }
            }
        });
        Self {
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
    fn submit(&mut self, query: String, offset: usize) {
        if self.busy || query.trim().is_empty() {
            return;
        }
        match self.requests.try_send((query.clone(), offset)) {
            Ok(()) => {
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
                    submit = Some((state.query.clone(), 0));
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
                ui.label(format!("Query: {}", state.displayed));
                ui.label(&page.summary);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !state.busy && page.offset > 0,
                            egui::Button::new("Previous"),
                        )
                        .clicked()
                    {
                        submit = Some((state.displayed.clone(), page.offset.saturating_sub(40)));
                    }
                    if ui
                        .add_enabled(
                            !state.busy && page.offset + page.rows.len() < page.total,
                            egui::Button::new("Next"),
                        )
                        .clicked()
                    {
                        submit = Some((state.displayed.clone(), page.offset + page.rows.len()));
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
                                submit = Some((row.query.clone(), 0));
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
    if let Some((query, offset)) = submit {
        state.submit(query, offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_scene_stays_pinned_while_live_updates_arrive() {
        let mut state = SearchState::new(Arc::new(|_, _| Err("unused".into())), None);
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
            Arc::new(|_, _| Err("unused".into())),
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
