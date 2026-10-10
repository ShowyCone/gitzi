use std::{collections::HashMap, time::Duration};

use eframe::egui;

use crate::{
    domain::{AppSnapshot, CommandEntry, DiffArea, FileStatus, UiAction},
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
                self.snapshot.file_diff = None;
                self.submit(GitOperation::InspectRepository(path), self.generation);
            }
            UiAction::Refresh => {
                if let Some(path) = self.snapshot.active_repo.clone() {
                    self.snapshot.file_diff = None;
                    self.submit(GitOperation::InspectRepository(path), self.generation);
                }
            }
            UiAction::InitializeRepository(path) => {
                if path.as_os_str().is_empty() {
                    self.snapshot.error_message =
                        Some("Escribe la ruta del nuevo repositorio.".into());
                    return;
                }
                self.generation = self.generation.saturating_add(1);
                self.snapshot.file_diff = None;
                self.submit(GitOperation::InitializeRepository { path }, self.generation);
            }
            UiAction::CloneRepository {
                source,
                destination,
            } => {
                if source.trim().is_empty() || destination.as_os_str().is_empty() {
                    self.snapshot.error_message =
                        Some("Indica la URL o ruta de origen y el destino de clonación.".into());
                    return;
                }
                self.generation = self.generation.saturating_add(1);
                self.snapshot.file_diff = None;
                self.submit(
                    GitOperation::CloneRepository {
                        source,
                        destination,
                    },
                    self.generation,
                );
            }
            UiAction::LoadDiff { path, area } => {
                let Some(repository) = self.snapshot.active_repo.clone() else {
                    return;
                };
                let Some(file) = self.file_status(&path) else {
                    self.snapshot.error_message = Some(
                        "El archivo ya no está en el estado del repositorio. Actualiza.".into(),
                    );
                    return;
                };
                let untracked = is_untracked(file);
                let available = match area {
                    DiffArea::Unstaged => untracked || file.worktree_status != '.',
                    DiffArea::Staged => has_staged_change(file),
                };
                if !available {
                    self.snapshot.file_diff = None;
                    self.snapshot.error_message =
                        Some("Esa área no tiene cambios para mostrar.".into());
                    return;
                }
                self.snapshot.file_diff = None;
                self.submit(
                    GitOperation::ReadDiff {
                        repository,
                        path,
                        area,
                        untracked,
                    },
                    self.generation,
                );
            }
            UiAction::StageFile(path) => {
                let Some(repository) = self.snapshot.active_repo.clone() else {
                    return;
                };
                let Some(file) = self.file_status(&path) else {
                    self.snapshot.error_message = Some(
                        "El archivo ya no está en el estado del repositorio. Actualiza.".into(),
                    );
                    return;
                };
                if !is_untracked(file) && file.worktree_status == '.' {
                    self.snapshot.error_message =
                        Some("El archivo no tiene cambios sin preparar.".into());
                    return;
                }
                self.snapshot.file_diff = None;
                self.submit(
                    GitOperation::StageFile { repository, path },
                    self.generation,
                );
            }
            UiAction::UnstageFile(path) => {
                let Some(repository) = self.snapshot.active_repo.clone() else {
                    return;
                };
                let Some(file) = self.file_status(&path) else {
                    self.snapshot.error_message = Some(
                        "El archivo ya no está en el estado del repositorio. Actualiza.".into(),
                    );
                    return;
                };
                if !has_staged_change(file) {
                    self.snapshot.error_message =
                        Some("El archivo no tiene cambios preparados.".into());
                    return;
                }
                let Some(has_head) = self
                    .snapshot
                    .repository
                    .as_ref()
                    .and_then(|status| status.has_head)
                else {
                    self.snapshot.error_message = Some(
                        "No se pudo determinar si el repositorio tiene commits. Actualiza.".into(),
                    );
                    return;
                };
                self.snapshot.file_diff = None;
                self.submit(
                    GitOperation::UnstageFile {
                        repository,
                        path,
                        has_head,
                    },
                    self.generation,
                );
            }
            UiAction::Commit(message) => {
                let Some(repository) = self.snapshot.active_repo.clone() else {
                    return;
                };
                let Some(status) = self.snapshot.repository.as_ref() else {
                    return;
                };
                if message.trim().is_empty() {
                    self.snapshot.error_message = Some("Escribe un mensaje de commit.".into());
                    return;
                }
                if status.files.iter().any(is_unmerged) {
                    self.snapshot.error_message =
                        Some("Resuelve los conflictos antes de crear un commit.".into());
                    return;
                }
                if !status.files.iter().any(has_staged_change) {
                    self.snapshot.error_message =
                        Some("Prepara al menos un cambio antes de crear un commit.".into());
                    return;
                }
                self.snapshot.file_diff = None;
                self.submit(
                    GitOperation::Commit {
                        repository,
                        message,
                    },
                    self.generation,
                );
            }
        }
    }

    fn file_status(&self, path: &std::path::Path) -> Option<&FileStatus> {
        self.snapshot
            .repository
            .as_ref()?
            .files
            .iter()
            .find(|file| file.path == path)
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
                    self.snapshot.file_diff = None;
                    settings::remember_repository(&path, &mut self.snapshot.recent_repositories);
                }
                Ok(GitResult::RepositoryInitialized { path })
                | Ok(GitResult::RepositoryCloned { path }) => {
                    self.snapshot.active_repo = Some(path.clone());
                    self.snapshot.repository = None;
                    self.snapshot.file_diff = None;
                    self.submit(GitOperation::InspectRepository(path), self.generation);
                }
                Ok(GitResult::DiffLoaded { diff }) => {
                    let is_current = self.snapshot.active_repo.as_ref() == Some(&diff.repository)
                        && self
                            .file_status(&diff.path)
                            .is_some_and(|file| match diff.area {
                                DiffArea::Unstaged => {
                                    is_untracked(file) || file.worktree_status != '.'
                                }
                                DiffArea::Staged => has_staged_change(file),
                            });
                    if is_current {
                        self.snapshot.file_diff = Some(diff);
                        self.snapshot.error_message = None;
                    }
                }
                Ok(GitResult::FileStaged { path })
                | Ok(GitResult::FileUnstaged { path })
                | Ok(GitResult::CommitCreated { path }) => {
                    self.refresh_after_mutation(path);
                }
                Err(error) => {
                    self.snapshot.git_checked = true;
                    self.snapshot.error_message = Some(error.message);
                }
            }
        }
    }

    fn refresh_after_mutation(&mut self, repository: std::path::PathBuf) {
        if self.snapshot.active_repo.as_ref() == Some(&repository) {
            self.snapshot.file_diff = None;
            self.snapshot.repository = None;
            self.submit(GitOperation::InspectRepository(repository), self.generation);
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

fn is_untracked(file: &FileStatus) -> bool {
    file.index_status == '?' && file.worktree_status == '?'
}

fn has_staged_change(file: &FileStatus) -> bool {
    file.index_status != '.' && file.index_status != '?'
}

fn is_unmerged(file: &FileStatus) -> bool {
    file.unmerged || file.index_status == 'U' || file.worktree_status == 'U'
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
