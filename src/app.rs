use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use eframe::egui;

use crate::{
    domain::{
        AppSnapshot, BranchKind, CommandEntry, DiffArea, FileStatus, GitHubAuthStatus, UiAction,
        is_valid_github_repository_name,
    },
    git::{GitOperation, GitRequest, GitResult, GitWorker},
    settings,
    ui::{self, UiState},
};

const COMMAND_LOG_LIMIT: usize = 200;
const STATUS_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

struct PendingRequest {
    generation: u64,
    refresh_after_error: Option<PathBuf>,
}

pub struct GitziApp {
    snapshot: AppSnapshot,
    worker: GitWorker,
    ui_state: UiState,
    next_request_id: u64,
    generation: u64,
    pending: HashMap<u64, PendingRequest>,
    github_login_process_active: bool,
    github_auth_epoch: u64,
    auth_check_request: Option<(u64, u64)>,
    status_refresh_request: Option<(u64, PathBuf)>,
    last_status_refresh_started: Option<Instant>,
}

impl GitziApp {
    pub fn new(_creation_context: &eframe::CreationContext<'_>) -> Self {
        let preferences = settings::load_preferences();
        let mut app = Self {
            snapshot: AppSnapshot {
                recent_repositories: settings::load_recent_repositories(),
                pull_policy: preferences.pull_policy,
                external_editor: preferences.external_editor,
                ..Default::default()
            },
            worker: GitWorker::spawn(),
            ui_state: UiState::default(),
            next_request_id: 1,
            generation: 0,
            pending: HashMap::new(),
            github_login_process_active: false,
            github_auth_epoch: 0,
            auth_check_request: None,
            status_refresh_request: None,
            last_status_refresh_started: None,
        };

        app.submit(GitOperation::DetectGit, 0);
        app.request_github_auth_check();
        app
    }

    fn submit(&mut self, operation: GitOperation, generation: u64) {
        let is_detection = matches!(&operation, GitOperation::DetectGit);
        let refresh_after_error = match &operation {
            GitOperation::Fetch(repository) => Some(repository.clone()),
            GitOperation::Pull { repository, .. }
            | GitOperation::CreateStash { repository, .. }
            | GitOperation::ApplyStash {
                repository,
                stash_oid: _,
            }
            | GitOperation::DropStash {
                repository,
                stash_oid: _,
            }
            | GitOperation::MergeBranch { repository, .. }
            | GitOperation::RebaseBranch { repository, .. }
            | GitOperation::RevertCommit { repository, .. }
            | GitOperation::ContinueIntegration(repository)
            | GitOperation::AbortIntegration(repository)
            | GitOperation::DiscardFile { repository, .. }
            | GitOperation::DeleteBranch { repository, .. } => Some(repository.clone()),
            GitOperation::PublishBranch { repository, .. }
            | GitOperation::CreateGitHubRepository { repository, .. } => Some(repository.clone()),
            _ => None,
        };
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        let request = GitRequest {
            id,
            generation,
            operation,
        };

        match self.worker.submit(request) {
            Ok(()) => {
                self.pending.insert(
                    id,
                    PendingRequest {
                        generation,
                        refresh_after_error,
                    },
                );
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

    fn request_github_auth_check(&mut self) {
        if self.auth_check_request.is_some() {
            return;
        }
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        let request = GitRequest {
            id,
            generation: 0,
            operation: GitOperation::CheckGitHubAuth,
        };
        match self.worker.submit(request) {
            Ok(()) => {
                self.auth_check_request = Some((id, self.github_auth_epoch));
                self.snapshot.github_auth_checking = true;
            }
            Err(error) => {
                self.snapshot.github_auth_checking = false;
                self.snapshot.error_message = Some(error.message);
            }
        }
    }

    fn submit_preserving_error(&mut self, operation: GitOperation, generation: u64) {
        let previous_error = self.snapshot.error_message.clone();
        let request_id = self.next_request_id;
        self.submit(operation, generation);
        if self.pending.contains_key(&request_id) {
            self.snapshot.error_message = previous_error;
        }
    }

    fn handle_action(&mut self, action: UiAction) {
        if self.snapshot.busy || self.snapshot.git_version.is_none() {
            return;
        }
        self.snapshot.success_message = None;

        match action {
            UiAction::StartGitHubBrowserLogin => {
                self.github_auth_epoch = self.github_auth_epoch.saturating_add(1);
                self.submit(GitOperation::StartGitHubBrowserLogin, 0);
            }
            UiAction::CheckGitHubAuth => {
                if self.auth_check_request.is_some() {
                    return;
                }
                self.github_auth_epoch = self.github_auth_epoch.saturating_add(1);
                self.snapshot.error_message = None;
                self.request_github_auth_check();
            }
            UiAction::OpenRepository(path) => {
                self.generation = self.generation.saturating_add(1);
                self.snapshot.active_repo = Some(path.clone());
                self.clear_repository_view();
                self.submit(GitOperation::InspectRepository(path), self.generation);
            }
            UiAction::Refresh => {
                if let Some(path) = self.snapshot.active_repo.clone() {
                    self.snapshot.file_diff = None;
                    self.clear_collaboration_state();
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
                self.snapshot.active_repo = None;
                self.clear_repository_view();
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
                self.snapshot.active_repo = None;
                self.clear_repository_view();
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
            action @ (UiAction::Fetch
            | UiAction::Pull
            | UiAction::Push
            | UiAction::CreateGitHubRepository { .. }
            | UiAction::PublishBranch(_)
            | UiAction::SetUpstream { .. }
            | UiAction::CreateBranch(_)
            | UiAction::SwitchBranch(_)
            | UiAction::LoadMoreHistory
            | UiAction::LoadCommitDetail(_)
            | UiAction::CompareReferences { .. }) => {
                self.handle_collaboration_action(action);
            }
            action @ (UiAction::CreateStash { .. }
            | UiAction::LoadStashDetail(_)
            | UiAction::ApplyStash(_)
            | UiAction::DropStash(_)
            | UiAction::MergeBranch(_)
            | UiAction::RebaseBranch(_)
            | UiAction::RevertCommit(_)
            | UiAction::OpenConflictFile(_)
            | UiAction::ContinueIntegration
            | UiAction::AbortIntegration
            | UiAction::DiscardFile(_)
            | UiAction::DeleteBranch(_)) => {
                self.handle_recovery_action(action);
            }
            action @ (UiAction::SetPullPolicy(_) | UiAction::SetExternalEditor(_)) => {
                self.handle_preference_action(action);
            }
        }
    }

    fn handle_collaboration_action(&mut self, action: UiAction) {
        let Some(repository) = self.snapshot.active_repo.clone() else {
            return;
        };
        let Some(status) = self.snapshot.repository.as_ref() else {
            return;
        };

        let operation = match action {
            UiAction::Fetch => {
                if self.snapshot.remotes.is_empty() {
                    self.snapshot.error_message =
                        Some("Configura un remoto en Git antes de hacer Fetch.".into());
                    return;
                }
                GitOperation::Fetch(repository)
            }
            UiAction::Pull => {
                if status.upstream.is_none() || !self.current_branch_is_local(status) {
                    self.snapshot.error_message = Some(
                        "Configura o publica el upstream de esta rama antes de hacer Pull.".into(),
                    );
                    return;
                }
                GitOperation::Pull {
                    repository,
                    policy: self.snapshot.pull_policy,
                }
            }
            UiAction::Push => {
                let Some(upstream) = status.upstream.clone() else {
                    self.snapshot.error_message = Some(
                        "Configura o publica el upstream de esta rama antes de hacer Push.".into(),
                    );
                    return;
                };
                if status.has_head != Some(true) || !self.current_branch_is_local(status) {
                    self.snapshot.error_message =
                        Some("Push requiere una rama local actual con al menos un commit.".into());
                    return;
                }
                GitOperation::Push {
                    repository,
                    local_branch: status.branch.clone(),
                    upstream,
                }
            }
            UiAction::CreateGitHubRepository { name, visibility } => {
                let name = name.trim();
                if self.snapshot.github_auth_status != GitHubAuthStatus::Authenticated {
                    self.snapshot.error_message = Some(
                        "Inicia sesión en GitHub y verifica la sesión antes de crear un repositorio."
                            .into(),
                    );
                    return;
                }
                if !is_valid_github_repository_name(name) {
                    self.snapshot.error_message = Some(
                        "El nombre debe ser un slug de 1–100 caracteres: letras, números, punto, guion o guion bajo."
                            .into(),
                    );
                    return;
                }
                if status.has_head != Some(true)
                    || status.branch.is_empty()
                    || !self.current_branch_is_local(status)
                {
                    self.snapshot.error_message = Some(
                        "Crea al menos un commit y selecciona una rama local para publicar.".into(),
                    );
                    return;
                }
                if self
                    .snapshot
                    .remotes
                    .iter()
                    .any(|remote| remote.name == "origin")
                {
                    self.snapshot.error_message =
                        Some("Ya existe un remoto origin; Gitzi no lo reemplazará.".into());
                    return;
                }
                GitOperation::CreateGitHubRepository {
                    repository,
                    name: name.to_owned(),
                    visibility,
                }
            }
            UiAction::PublishBranch(remote) => {
                if status.upstream.is_some() {
                    self.snapshot.error_message = Some(
                        "Esta rama ya tiene upstream. Usa Push para enviar sus commits.".into(),
                    );
                    return;
                }
                if status.has_head != Some(true) || !self.current_branch_is_local(status) {
                    self.snapshot.error_message = Some(
                        "Publicar requiere una rama local actual con al menos un commit.".into(),
                    );
                    return;
                }
                if !self.snapshot.remotes.iter().any(|item| item.name == remote) {
                    self.snapshot.error_message = Some(
                        "El remoto elegido ya no está disponible. Actualiza las referencias."
                            .into(),
                    );
                    return;
                }
                if status.branch.is_empty() {
                    self.snapshot.error_message = Some(
                        "No se puede publicar una rama sin nombre. Crea un commit primero.".into(),
                    );
                    return;
                }
                GitOperation::PublishBranch {
                    repository,
                    remote,
                    local_branch: status.branch.clone(),
                }
            }
            UiAction::SetUpstream {
                local_branch,
                remote_branch,
            } => {
                let local_exists =
                    self.snapshot.branches.iter().any(|branch| {
                        branch.kind == BranchKind::Local && branch.name == local_branch
                    });
                let remote_exists = self.snapshot.branches.iter().any(|branch| {
                    branch.kind == BranchKind::Remote && branch.name == remote_branch
                });
                if !local_exists || !remote_exists {
                    self.snapshot.error_message = Some(
                        "La rama local o remota ya no existe. Actualiza la lista de ramas.".into(),
                    );
                    return;
                }
                GitOperation::SetUpstream {
                    repository,
                    local_branch,
                    remote_branch,
                }
            }
            UiAction::CreateBranch(name) => {
                if name.trim().is_empty() {
                    self.snapshot.error_message = Some("Escribe el nombre de la rama.".into());
                    return;
                }
                if status.has_head != Some(true) {
                    self.snapshot.error_message =
                        Some("Crea el primer commit antes de crear otra rama.".into());
                    return;
                }
                GitOperation::CreateBranch { repository, name }
            }
            UiAction::SwitchBranch(name) => {
                let branch = self
                    .snapshot
                    .branches
                    .iter()
                    .find(|branch| branch.kind == BranchKind::Local && branch.name == name);
                if branch.is_none() {
                    self.snapshot.error_message = Some(
                        "La rama local ya no está disponible. Actualiza la lista de ramas.".into(),
                    );
                    return;
                }
                if status.branch == name {
                    return;
                }
                GitOperation::SwitchBranch { repository, name }
            }
            UiAction::LoadMoreHistory => {
                if !self.snapshot.history_has_more {
                    return;
                }
                GitOperation::LoadHistory {
                    repository,
                    offset: self.snapshot.history.len(),
                    limit: 30,
                }
            }
            UiAction::LoadCommitDetail(commit_id) => {
                if !self
                    .snapshot
                    .history
                    .iter()
                    .any(|commit| commit.id == commit_id)
                {
                    self.snapshot.error_message =
                        Some("Ese commit ya no está en la página cargada.".into());
                    return;
                }
                self.snapshot.commit_detail = None;
                GitOperation::LoadCommitDetail {
                    repository,
                    commit_id,
                }
            }
            UiAction::CompareReferences {
                base_ref,
                target_ref,
            } => {
                let available = |reference: &str| {
                    self.snapshot
                        .branches
                        .iter()
                        .any(|branch| branch.name == reference)
                };
                if base_ref == target_ref || !available(&base_ref) || !available(&target_ref) {
                    self.snapshot.error_message =
                        Some("Elige dos referencias distintas de la lista de ramas actual.".into());
                    return;
                }
                self.snapshot.ref_comparison = None;
                GitOperation::CompareReferences {
                    repository,
                    base_ref,
                    target_ref,
                }
            }
            _ => return,
        };
        self.submit(operation, self.generation);
    }

    fn handle_preference_action(&mut self, action: UiAction) {
        let mut preferences = settings::Preferences {
            pull_policy: self.snapshot.pull_policy,
            external_editor: self.snapshot.external_editor.clone(),
        };
        match action {
            UiAction::SetPullPolicy(policy) => preferences.pull_policy = policy,
            UiAction::SetExternalEditor(editor) => {
                if editor
                    .as_ref()
                    .is_some_and(|path| path.as_os_str().is_empty())
                {
                    self.snapshot.error_message = Some("Indica la ruta del editor externo.".into());
                    return;
                }
                preferences.external_editor = editor;
            }
            _ => return,
        }
        match settings::save_preferences(&preferences) {
            Ok(()) => {
                self.snapshot.pull_policy = preferences.pull_policy;
                self.snapshot.external_editor = preferences.external_editor;
                self.snapshot.error_message = None;
            }
            Err(error) => self.snapshot.error_message = Some(error),
        }
    }

    fn handle_recovery_action(&mut self, action: UiAction) {
        let Some(repository) = self.snapshot.active_repo.clone() else {
            return;
        };
        let Some(status) = self.snapshot.repository.clone() else {
            return;
        };

        let is_merge = matches!(&action, UiAction::MergeBranch(_));
        let operation = match action {
            UiAction::CreateStash {
                message,
                include_untracked,
            } => {
                if status.files.is_empty() {
                    self.snapshot.error_message =
                        Some("No hay cambios para guardar en un stash.".into());
                    return;
                }
                GitOperation::CreateStash {
                    repository,
                    message,
                    include_untracked,
                }
            }
            UiAction::LoadStashDetail(oid) => {
                if !self.snapshot.stashes.iter().any(|stash| stash.oid == oid) {
                    self.snapshot.error_message =
                        Some("Ese stash ya no está disponible. Actualiza.".into());
                    return;
                }
                self.snapshot.stash_detail = None;
                GitOperation::LoadStashDetail {
                    repository,
                    stash_oid: oid,
                }
            }
            UiAction::ApplyStash(oid) => {
                if !self.snapshot.stashes.iter().any(|stash| stash.oid == oid) {
                    self.snapshot.error_message =
                        Some("Ese stash ya no está disponible. Actualiza.".into());
                    return;
                }
                GitOperation::ApplyStash {
                    repository,
                    stash_oid: oid,
                }
            }
            UiAction::DropStash(oid) => {
                if !self.snapshot.stashes.iter().any(|stash| stash.oid == oid) {
                    self.snapshot.error_message =
                        Some("Ese stash ya no está disponible. Actualiza.".into());
                    return;
                }
                GitOperation::DropStash {
                    repository,
                    stash_oid: oid,
                }
            }
            UiAction::MergeBranch(branch) | UiAction::RebaseBranch(branch) => {
                if status.has_head != Some(true) || status.branch.is_empty() {
                    self.snapshot.error_message =
                        Some("La integración requiere una rama actual con commit.".into());
                    return;
                }
                if status.integration.is_some() || !status.files.is_empty() {
                    self.snapshot.error_message = Some(
                        "Guarda o resuelve los cambios antes de iniciar otra integración.".into(),
                    );
                    return;
                }
                let valid_target = self.snapshot.branches.iter().any(|item| {
                    item.kind == BranchKind::Local && item.name == branch && !item.current
                });
                if !valid_target {
                    self.snapshot.error_message =
                        Some("Elige una rama local existente que no sea la actual.".into());
                    return;
                }
                if is_merge {
                    GitOperation::MergeBranch { repository, branch }
                } else {
                    GitOperation::RebaseBranch { repository, branch }
                }
            }
            UiAction::RevertCommit(commit_id) => {
                if status.has_head != Some(true) || status.branch.is_empty() {
                    self.snapshot.error_message =
                        Some("Revertir requiere una rama actual con commit.".into());
                    return;
                }
                if status.integration.is_some() || !status.files.is_empty() {
                    self.snapshot.error_message =
                        Some("Guarda o resuelve los cambios antes de revertir.".into());
                    return;
                }
                if !self
                    .snapshot
                    .history
                    .iter()
                    .any(|commit| commit.id == commit_id)
                {
                    self.snapshot.error_message =
                        Some("Ese commit ya no está en el historial cargado.".into());
                    return;
                }
                GitOperation::RevertCommit {
                    repository,
                    commit_id,
                }
            }
            UiAction::OpenConflictFile(path) => {
                let conflicted = self.file_status(&path).is_some_and(is_unmerged);
                if !conflicted {
                    self.snapshot.error_message =
                        Some("El archivo ya no tiene un conflicto activo.".into());
                    return;
                }
                let Some(editor) = self.snapshot.external_editor.clone() else {
                    self.snapshot.error_message =
                        Some("Configura un editor externo en Preferencias.".into());
                    return;
                };
                GitOperation::OpenConflictFile {
                    repository,
                    path,
                    editor,
                }
            }
            UiAction::ContinueIntegration => {
                if status.integration.is_none() {
                    self.snapshot.error_message =
                        Some("No hay una integración en curso para continuar.".into());
                    return;
                }
                if status.files.iter().any(is_unmerged) {
                    self.snapshot.error_message = Some(
                        "Resuelve y prepara todos los archivos en conflicto antes de continuar."
                            .into(),
                    );
                    return;
                }
                GitOperation::ContinueIntegration(repository)
            }
            UiAction::AbortIntegration => {
                if status.integration.is_none() {
                    self.snapshot.error_message =
                        Some("No hay una integración en curso para abortar.".into());
                    return;
                }
                GitOperation::AbortIntegration(repository)
            }
            UiAction::DiscardFile(path) => {
                let Some(file) = status.files.iter().find(|file| file.path == path) else {
                    self.snapshot.error_message =
                        Some("El archivo ya no tiene cambios. Actualiza.".into());
                    return;
                };
                if status.integration.is_some() || is_unmerged(file) {
                    self.snapshot.error_message =
                        Some("Resuelve o aborta la integración antes de descartar cambios.".into());
                    return;
                }
                let Some(has_head) = status.has_head else {
                    self.snapshot.error_message = Some(
                        "No se pudo determinar si el repositorio tiene commits. Actualiza.".into(),
                    );
                    return;
                };
                GitOperation::DiscardFile {
                    repository,
                    path,
                    has_head,
                }
            }
            UiAction::DeleteBranch(name) => {
                let Some(branch) = self
                    .snapshot
                    .branches
                    .iter()
                    .find(|item| item.kind == BranchKind::Local && item.name == name)
                else {
                    self.snapshot.error_message =
                        Some("La rama local ya no existe. Actualiza.".into());
                    return;
                };
                if branch.current || name == status.branch {
                    self.snapshot.error_message =
                        Some("No se puede eliminar la rama actual.".into());
                    return;
                }
                GitOperation::DeleteBranch { repository, name }
            }
            _ => return,
        };
        self.snapshot.file_diff = None;
        self.submit(operation, self.generation);
    }

    fn file_status(&self, path: &std::path::Path) -> Option<&FileStatus> {
        self.snapshot
            .repository
            .as_ref()?
            .files
            .iter()
            .find(|file| file.path == path)
    }

    fn current_branch_is_local(&self, status: &crate::domain::RepositoryStatus) -> bool {
        self.snapshot.branches.iter().any(|branch| {
            branch.kind == BranchKind::Local && branch.current && branch.name == status.branch
        })
    }

    fn receive_responses(&mut self) {
        while let Some(response) = self.worker.try_recv() {
            if self
                .auth_check_request
                .is_some_and(|(id, _)| id == response.id)
            {
                let (_, auth_epoch) = self
                    .auth_check_request
                    .take()
                    .expect("auth check was checked above");
                self.snapshot.github_auth_checking = false;
                self.append_commands(response.commands);
                if auth_epoch == self.github_auth_epoch && !self.github_login_process_active {
                    match response.result {
                        Ok(GitResult::GitHubAuthChecked { status }) => {
                            self.snapshot.github_auth_status = status;
                        }
                        Err(error) if self.snapshot.error_message.is_none() => {
                            self.snapshot.error_message = Some(error.message);
                        }
                        _ => {}
                    }
                }
                continue;
            }

            if self
                .status_refresh_request
                .as_ref()
                .is_some_and(|(id, _)| *id == response.id)
            {
                let (_, requested_path) = self
                    .status_refresh_request
                    .take()
                    .expect("status request was checked above");
                self.snapshot.status_refreshing = false;
                let is_current = response.generation == self.generation
                    && self.snapshot.active_repo.as_ref() == Some(&requested_path);
                if is_current {
                    self.snapshot.status_checked_at = Some(SystemTime::now());
                    match response.result {
                        Ok(GitResult::RepositoryStatusRefreshed { path, status })
                            if path == requested_path =>
                        {
                            self.snapshot.repository = Some(status);
                        }
                        Err(error) if self.snapshot.error_message.is_none() => {
                            self.snapshot.error_message = Some(error.message);
                        }
                        _ => {}
                    }
                }
                continue;
            }

            if let Ok(GitResult::GitHubLoginFinished { status }) = &response.result {
                self.update_commands(response.commands);
                self.github_login_process_active = false;
                self.snapshot.github_auth_status = status.clone();
                if status == &GitHubAuthStatus::Authenticated {
                    self.snapshot.error_message = None;
                    self.snapshot.success_message =
                        Some("Sesión de GitHub iniciada correctamente.".into());
                } else {
                    self.snapshot.error_message = Some(
                        "GitHub CLI terminó sin completar el inicio de sesión. Puedes intentarlo de nuevo.".into(),
                    );
                }
                continue;
            }

            let pending = self.pending.remove(&response.id);
            let is_current = pending
                .as_ref()
                .is_some_and(|request| request.generation == response.generation);
            let refresh_after_error = pending.and_then(|request| request.refresh_after_error);
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
                Ok(GitResult::GitHubLoginStarted) => {
                    self.github_login_process_active = true;
                    self.snapshot.github_auth_status = GitHubAuthStatus::LoginStarted;
                }
                Ok(GitResult::GitHubAuthChecked { status }) => {
                    if !self.github_login_process_active {
                        self.snapshot.github_auth_status = status;
                    }
                }
                Ok(GitResult::GitHubRepositoryCreated {
                    repository,
                    remote_url,
                }) if self.snapshot.active_repo.as_ref() == Some(&repository) => {
                    self.refresh_after_mutation(repository);
                    self.snapshot.success_message = Some(format!(
                        "Repositorio creado y commits publicados: {remote_url}"
                    ));
                }
                Ok(GitResult::RepositoryInspected { path, status }) => {
                    if self.snapshot.active_repo.as_ref() != Some(&path) {
                        self.clear_collaboration_state();
                    }
                    self.snapshot.active_repo = Some(path.clone());
                    self.snapshot.repository = Some(status);
                    self.snapshot.status_checked_at = Some(SystemTime::now());
                    self.snapshot.file_diff = None;
                    settings::remember_repository(&path, &mut self.snapshot.recent_repositories);
                    self.submit_preserving_error(
                        GitOperation::ListBranches(path.clone()),
                        self.generation,
                    );
                    self.submit_preserving_error(
                        GitOperation::LoadHistory {
                            repository: path.clone(),
                            offset: 0,
                            limit: 30,
                        },
                        self.generation,
                    );
                    self.submit_preserving_error(GitOperation::ListStashes(path), self.generation);
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
                Ok(GitResult::BranchesListed {
                    repository,
                    branches,
                    remotes,
                }) if self.snapshot.active_repo.as_ref() == Some(&repository) => {
                    self.snapshot.branches = branches;
                    self.snapshot.remotes = remotes;
                }
                Ok(GitResult::HistoryLoaded {
                    repository,
                    offset,
                    commits,
                    has_more,
                }) if self.snapshot.active_repo.as_ref() == Some(&repository) => {
                    if offset == 0 {
                        self.snapshot.history = commits;
                    } else if offset == self.snapshot.history.len() {
                        self.snapshot.history.extend(commits);
                    }
                    self.snapshot.history_has_more = has_more;
                }
                Ok(GitResult::StashesListed {
                    repository,
                    stashes,
                }) if self.snapshot.active_repo.as_ref() == Some(&repository) => {
                    if self
                        .snapshot
                        .stash_detail
                        .as_ref()
                        .is_some_and(|detail| !stashes.iter().any(|stash| stash.oid == detail.oid))
                    {
                        self.snapshot.stash_detail = None;
                    }
                    self.snapshot.stashes = stashes;
                }
                Ok(GitResult::StashDetailLoaded { repository, detail })
                    if self.snapshot.active_repo.as_ref() == Some(&repository)
                        && self
                            .snapshot
                            .stashes
                            .iter()
                            .any(|stash| stash.oid == detail.oid) =>
                {
                    self.snapshot.stash_detail = Some(detail);
                }
                Ok(GitResult::CommitLoaded { repository, detail })
                    if self.snapshot.active_repo.as_ref() == Some(&repository) =>
                {
                    self.snapshot.commit_detail = Some(detail);
                }
                Ok(GitResult::ReferencesCompared {
                    repository,
                    comparison,
                }) if self.snapshot.active_repo.as_ref() == Some(&repository) => {
                    self.snapshot.ref_comparison = Some(comparison);
                }
                Ok(GitResult::BranchCreated { repository })
                | Ok(GitResult::BranchSwitched { repository })
                | Ok(GitResult::UpstreamSet { repository })
                | Ok(GitResult::Fetched { repository })
                | Ok(GitResult::Pulled { repository })
                | Ok(GitResult::Pushed { repository })
                | Ok(GitResult::BranchPublished { repository }) => {
                    self.refresh_after_mutation(repository);
                }
                Ok(GitResult::StashCreated { repository })
                | Ok(GitResult::StashApplied { repository })
                | Ok(GitResult::StashDropped { repository })
                | Ok(GitResult::IntegrationCompleted { repository })
                | Ok(GitResult::IntegrationContinued { repository })
                | Ok(GitResult::IntegrationAborted { repository })
                | Ok(GitResult::FileDiscarded { repository })
                | Ok(GitResult::BranchDeleted { repository }) => {
                    self.refresh_after_mutation(repository);
                }
                Ok(GitResult::ExternalFileOpened { .. }) => {}
                Err(error) => {
                    self.snapshot.git_checked = true;
                    self.snapshot.error_message =
                        Some(self.snapshot.success_message.take().map_or(
                            error.message.clone(),
                            |success| {
                                format!(
                                    "{success}. No se pudo actualizar la vista del repositorio: {}",
                                    error.message
                                )
                            },
                        ));
                    if let Some(repository) = refresh_after_error {
                        self.refresh_after_error(repository);
                    }
                }
                Ok(_) => {}
            }
        }
    }

    fn refresh_after_mutation(&mut self, repository: std::path::PathBuf) {
        if self.snapshot.active_repo.as_ref() == Some(&repository) {
            self.snapshot.file_diff = None;
            self.clear_collaboration_state();
            self.submit(GitOperation::InspectRepository(repository), self.generation);
        }
    }

    fn refresh_after_error(&mut self, repository: PathBuf) {
        if self.snapshot.active_repo.as_ref() == Some(&repository) {
            self.snapshot.file_diff = None;
            self.clear_collaboration_state();
            self.submit_preserving_error(
                GitOperation::InspectRepository(repository),
                self.generation,
            );
        }
    }

    fn clear_collaboration_state(&mut self) {
        self.snapshot.branches.clear();
        self.snapshot.remotes.clear();
        self.snapshot.history.clear();
        self.snapshot.history_has_more = false;
        self.snapshot.commit_detail = None;
        self.snapshot.ref_comparison = None;
        self.snapshot.stashes.clear();
        self.snapshot.stash_detail = None;
    }

    fn clear_repository_view(&mut self) {
        self.snapshot.repository = None;
        self.snapshot.status_refreshing = false;
        self.snapshot.status_checked_at = None;
        self.snapshot.file_diff = None;
        self.clear_collaboration_state();
    }

    fn refresh_repository_status_when_due(&mut self) {
        let Some(repository) = self.snapshot.active_repo.clone() else {
            return;
        };
        if self.snapshot.busy
            || self.status_refresh_request.is_some()
            || self.snapshot.repository.is_none()
            || self
                .last_status_refresh_started
                .is_some_and(|started| started.elapsed() < STATUS_REFRESH_INTERVAL)
        {
            return;
        }

        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        let request = GitRequest {
            id,
            generation: self.generation,
            operation: GitOperation::RefreshRepositoryStatus(repository.clone()),
        };
        self.last_status_refresh_started = Some(Instant::now());
        if self.worker.submit(request).is_ok() {
            self.status_refresh_request = Some((id, repository));
            self.snapshot.status_refreshing = true;
        }
    }

    fn append_commands(&mut self, commands: Vec<CommandEntry>) {
        self.snapshot.command_log.extend(commands);
        self.trim_command_log();
    }

    fn update_commands(&mut self, commands: Vec<CommandEntry>) {
        for command in commands {
            if let Some(existing) = self
                .snapshot
                .command_log
                .iter_mut()
                .find(|entry| entry.id == command.id)
            {
                *existing = command;
            } else {
                self.snapshot.command_log.push(command);
            }
        }
        self.trim_command_log();
    }

    fn trim_command_log(&mut self) {
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
        } else if self.snapshot.github_auth_checking {
            ctx.request_repaint_after(Duration::from_millis(200));
        } else if self.github_login_process_active {
            ctx.request_repaint_after(Duration::from_secs(1));
        } else if self.snapshot.status_refreshing {
            ctx.request_repaint_after(Duration::from_millis(100));
        } else if self.snapshot.repository.is_some() {
            let wait = self
                .last_status_refresh_started
                .map_or(STATUS_REFRESH_INTERVAL, |started| {
                    STATUS_REFRESH_INTERVAL.saturating_sub(started.elapsed())
                });
            ctx.request_repaint_after(wait.max(Duration::from_millis(100)));
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
        self.refresh_repository_status_when_due();
        self.request_pending_repaint(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.ui_state.terminal_visible() {
            egui::Panel::bottom("command_terminal")
                .resizable(true)
                .default_size(164.0)
                .size_range(116.0..=228.0)
                .show(ui, |ui| {
                    ui::render_terminal(ui, &mut self.ui_state, &self.snapshot);
                });
        }
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(action) = ui::render(ui, &mut self.ui_state, &self.snapshot) {
                self.handle_action(action);
            }
        });
    }
}
