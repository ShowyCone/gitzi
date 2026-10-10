use std::{collections::HashMap, time::Duration};

use eframe::egui;

use crate::{
    domain::{AppSnapshot, CommandEntry, UiAction},
    git::{GitOperation, GitRequest, GitResult, GitWorker},
    settings,
    ui::{self, UiState},
};

const COMMAND_LOG_LIMIT: usize = 200;

pub struct GitziApp {
    snapshot: AppSnapshot,
    worker: GitWorker,
    ui_state: UiState,
    next_request_id: u64,
    generation: u64,
    pending: HashMap<u64, u64>,
}

impl GitziApp {
    pub fn new(_creation_context: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            snapshot: AppSnapshot {
                recent_repositories: settings::load_recent_repositories(),
                ..Default::default()
            },
            worker: GitWorker::spawn(),
            ui_state: UiState::default(),
            next_request_id: 1,
            generation: 0,
            pending: HashMap::new(),
        };

        app.submit(GitOperation::DetectGit, 0);
        app
    }

    fn submit(&mut self, operation: GitOperation, generation: u64) {
        let is_detection = matches!(&operation, GitOperation::DetectGit);
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        let request = GitRequest {
            id,
            generation,
            operation,
        };

        match self.worker.submit(request) {
            Ok(()) => {
                self.pending.insert(id, generation);
                self.snapshot.busy = true;
                self.snapshot.error_message = None;
            }
            Err(error) => {
                if is_detection {
                    self.snapshot.git_checked = true;
                }
                self.snapshot.error_message = Some(error.message);
            }
        }
    }

    fn handle_action(&mut self, action: UiAction) {
        if self.snapshot.busy || self.snapshot.git_version.is_none() {
            return;
        }

        match action {
            UiAction::OpenRepository(path) => {
                self.generation = self.generation.saturating_add(1);
                self.submit(GitOperation::InspectRepository(path), self.generation);
            }
            UiAction::Refresh => {
                if let Some(path) = self.snapshot.active_repo.clone() {
                    self.submit(GitOperation::InspectRepository(path), self.generation);
                }
            }
        }
    }

    fn receive_responses(&mut self) {
        while let Some(response) = self.worker.try_recv() {
            let is_current = self.pending.remove(&response.id) == Some(response.generation);
            self.snapshot.busy = !self.pending.is_empty();
            self.append_commands(response.commands);

            if !is_current || response.generation != self.generation && response.generation != 0 {
                continue;
            }

            match response.result {
                Ok(GitResult::GitDetected { version }) => {
                    self.snapshot.git_checked = true;
                    self.snapshot.git_version = Some(version);
                }
                Ok(GitResult::RepositoryInspected { path, status }) => {
                    self.snapshot.active_repo = Some(path.clone());
                    self.snapshot.repository = Some(status);
                    self.snapshot.error_message = None;
                    settings::remember_repository(&path, &mut self.snapshot.recent_repositories);
                }
                Err(error) => {
                    self.snapshot.git_checked = true;
                    self.snapshot.error_message = Some(error.message);
                }
            }
        }
    }

    fn append_commands(&mut self, commands: Vec<CommandEntry>) {
        self.snapshot.command_log.extend(commands);
        let overflow = self
            .snapshot
            .command_log
            .len()
            .saturating_sub(COMMAND_LOG_LIMIT);
        if overflow > 0 {
            self.snapshot.command_log.drain(0..overflow);
        }
    }

    fn request_pending_repaint(&self, ctx: &egui::Context) {
        if self.snapshot.busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

impl eframe::App for GitziApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_responses();
        self.request_pending_repaint(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(action) = ui::render(ui, &mut self.ui_state, &self.snapshot) {
            self.handle_action(action);
        }
    }
}
