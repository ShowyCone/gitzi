use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::egui;

use crate::domain::{
    AppSnapshot, BranchInfo, BranchKind, CommandEntry, CommandStatus, DiffArea, FileStatus,
    GitHubAuthStatus, GitHubRepositoryVisibility, IntegrationKind, PullPolicy, UiAction,
};

#[derive(Clone, Debug)]
enum Confirmation {
    DropStash {
        oid: String,
        label: String,
    },
    DiscardFile(PathBuf),
    DeleteBranch(String),
    AbortIntegration {
        kind: String,
    },
    CreateGitHubRepository {
        name: String,
        visibility: GitHubRepositoryVisibility,
        repository: PathBuf,
        branch: String,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum WorkspaceTab {
    #[default]
    CodeControl,
    History,
    Branches,
    Integrate,
    Repository,
}

impl WorkspaceTab {
    const ALL: [Self; 5] = [
        Self::CodeControl,
        Self::History,
        Self::Branches,
        Self::Integrate,
        Self::Repository,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::CodeControl => "Control de código",
            Self::History => "Historial",
            Self::Branches => "Ramas",
            Self::Integrate => "Integrar",
            Self::Repository => "Repositorio",
        }
    }
}

pub struct UiState {
    active_tab: WorkspaceTab,
    terminal_visible: bool,
    repository_path: String,
    clone_source: String,
    clone_destination: String,
    active_repository: Option<PathBuf>,
    selected_file: Option<PathBuf>,
    selected_area: DiffArea,
    requested_diff: Option<(PathBuf, PathBuf, DiffArea)>,
    commit_message: String,
    branch_name: String,
    selected_local_branch: String,
    selected_integration_branch: String,
    selected_remote: String,
    selected_remote_branch: String,
    selected_commit: Option<String>,
    compare_base: String,
    compare_target: String,
    copied_command: Option<u64>,
    editor_path: String,
    observed_editor: Option<Option<PathBuf>>,
    stash_message: String,
    stash_include_untracked: bool,
    selected_stash: Option<String>,
    confirmation: Option<Confirmation>,
    github_repository_name: String,
    github_repository_visibility: GitHubRepositoryVisibility,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_tab: WorkspaceTab::CodeControl,
            terminal_visible: true,
            repository_path: String::new(),
            clone_source: String::new(),
            clone_destination: String::new(),
            active_repository: None,
            selected_file: None,
            selected_area: DiffArea::Unstaged,
            requested_diff: None,
            commit_message: String::new(),
            branch_name: String::new(),
            selected_local_branch: String::new(),
            selected_integration_branch: String::new(),
            selected_remote: String::new(),
            selected_remote_branch: String::new(),
            selected_commit: None,
            compare_base: String::new(),
            compare_target: String::new(),
            copied_command: None,
            editor_path: String::new(),
            observed_editor: None,
            stash_message: String::new(),
            stash_include_untracked: false,
            selected_stash: None,
            confirmation: None,
            github_repository_name: String::new(),
            github_repository_visibility: GitHubRepositoryVisibility::default(),
        }
    }
}

impl UiState {
    pub fn terminal_visible(&self) -> bool {
        self.terminal_visible
    }
}

pub fn render(ui: &mut egui::Ui, state: &mut UiState, snapshot: &AppSnapshot) -> Option<UiAction> {
    if state.active_repository != snapshot.active_repo {
        state.active_repository = snapshot.active_repo.clone();
        state.selected_file = None;
        state.requested_diff = None;
        state.selected_commit = None;
        state.selected_stash = None;
        state.confirmation = None;
    }
    let available = snapshot.git_checked && snapshot.git_version.is_some();
    let can_act = available && !snapshot.busy;
    let mut action = None;
    if state.observed_editor.as_ref() != Some(&snapshot.external_editor) {
        state.editor_path = snapshot
            .external_editor
            .as_ref()
            .map_or_else(String::new, |path| path.display().to_string());
        state.observed_editor = Some(snapshot.external_editor.clone());
    }

    apply_theme(ui);
    ui.vertical(|ui| {
        render_repository_header(ui, state, snapshot, can_act, &mut action);
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            let width = ((ui.available_width() - 32.0) / 5.0).max(90.0);
            for tab in WorkspaceTab::ALL {
                let response = ui.add_sized(
                    [width, 36.0],
                    egui::Button::selectable(state.active_tab == tab, tab.label()),
                );
                if response.clicked() {
                    state.active_tab = tab;
                }
            }
        });
        if let Some(message) = &snapshot.error_message {
            render_message(ui, "Error", message, ui.visuals().error_fg_color);
        }
        if let Some(message) = &snapshot.success_message {
            render_message(
                ui,
                "Completado",
                message,
                egui::Color32::from_rgb(145, 230, 190),
            );
        }
        if state.active_tab == WorkspaceTab::CodeControl {
            egui::ScrollArea::vertical()
                .id_salt(("gitzi_tab_content", state.active_tab))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    render_code_control(ui, state, snapshot, can_act, &mut action);
                });
        } else {
            egui::ScrollArea::vertical()
                .id_salt(("gitzi_tab_content", state.active_tab))
                .auto_shrink([false, false])
                .show(ui, |ui| match state.active_tab {
                    WorkspaceTab::CodeControl => unreachable!(),
                    WorkspaceTab::History => {
                        render_history_tab(ui, state, snapshot, can_act, &mut action)
                    }
                    WorkspaceTab::Branches => {
                        render_branches_tab(ui, state, snapshot, can_act, &mut action)
                    }
                    WorkspaceTab::Integrate => {
                        render_integrate_tab(ui, state, snapshot, can_act, &mut action)
                    }
                    WorkspaceTab::Repository => {
                        render_repository_tab(ui, state, snapshot, can_act, &mut action)
                    }
                });
        }
        render_confirmation(ui, state, snapshot, can_act, &mut action);
    });
    if matches!(
        action.as_ref(),
        Some(
            UiAction::Refresh
                | UiAction::CreateStash { .. }
                | UiAction::ApplyStash(_)
                | UiAction::DropStash(_)
        )
    ) {
        state.selected_stash = None;
    }
    action
}
/// Render the real command output as one read-only terminal transcript.
/// The app worker publishes entries after each operation completes; this view
/// intentionally does not imply that output is streamed while a process runs.
pub fn render_terminal(ui: &mut egui::Ui, state: &mut UiState, snapshot: &AppSnapshot) {
    apply_theme(ui);
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(13, 17, 23))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(49, 61, 75)))
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Terminal de actividad").strong());
                ui.label(
                    egui::RichText::new(format!("{} comandos", snapshot.command_log.len())).weak(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Ocultar").clicked() {
                        state.terminal_visible = false;
                    }
                });
            });
            ui.add_space(3.0);
            egui::ScrollArea::both()
                .id_salt("command_terminal_transcript")
                .max_height(174.0)
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if snapshot.command_log.is_empty() {
                        ui.label(
                            egui::RichText::new("Los comandos y sus resultados aparecerán aquí.")
                                .weak(),
                        );
                    }
                    for entry in &snapshot.command_log {
                        terminal_entry(ui, state, entry);
                    }
                });
        });
}

fn terminal_entry(ui: &mut egui::Ui, state: &mut UiState, entry: &CommandEntry) {
    let (label, color) = match entry.status {
        CommandStatus::Running => ("EN CURSO", egui::Color32::from_rgb(111, 174, 222)),
        CommandStatus::Succeeded => ("OK", egui::Color32::from_rgb(117, 220, 173)),
        CommandStatus::SucceededWithDifferences => {
            ("OK · cambios", egui::Color32::from_rgb(255, 199, 102))
        }
        CommandStatus::Failed => ("ERROR", ui.visuals().error_fg_color),
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new("›")
                .color(egui::Color32::from_rgb(111, 214, 174))
                .strong(),
        );
        ui.monospace(
            egui::RichText::new(&entry.command_display)
                .color(egui::Color32::from_rgb(218, 229, 239)),
        );
        if ui
            .add_sized(
                [82.0, 40.0],
                egui::Button::new(if state.copied_command == Some(entry.id) {
                    "Copiado"
                } else {
                    "Copiar"
                }),
            )
            .clicked()
        {
            ui.ctx().copy_text(entry.command_display.clone());
            state.copied_command = Some(entry.id);
        }
        ui.label(egui::RichText::new(label).color(color).strong());
        if let Some(code) = entry.exit_code {
            ui.label(
                egui::RichText::new(format!("código {code} · {} ms", entry.duration.as_millis()))
                    .weak(),
            );
        }
        ui.label(egui::RichText::new(format_command_time(entry.started_at)).weak());
    });
    if let Some(cwd) = &entry.cwd {
        ui.label(
            egui::RichText::new(format!("  en {}", cwd.display()))
                .weak()
                .monospace(),
        );
    }
    if !entry.stdout.is_empty() {
        ui.label(
            egui::RichText::new(&entry.stdout)
                .monospace()
                .color(egui::Color32::from_rgb(194, 207, 220)),
        );
    }
    if !entry.stderr.is_empty() {
        ui.label(
            egui::RichText::new(&entry.stderr)
                .monospace()
                .color(ui.visuals().error_fg_color),
        );
    }
    ui.add_space(5.0);
}

fn choose_folder(title: &str, current_path: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().set_title(title);
    let mut current = PathBuf::from(current_path);
    while !current.is_dir() && current.pop() {}
    if current.is_dir() {
        dialog = dialog.set_directory(current);
    }
    dialog.pick_folder()
}

fn format_command_time(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let seconds_of_day = seconds % 86_400;
    format!(
        "{:02}:{:02}:{:02} UTC",
        seconds_of_day / 3_600,
        (seconds_of_day / 60) % 60,
        seconds_of_day % 60
    )
}

fn render_github_login(
    ui: &mut egui::Ui,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("GitHub").strong());
            let (status, color) = if snapshot.github_auth_checking {
                ("Comprobando sesión…", ui.visuals().weak_text_color())
            } else {
                match &snapshot.github_auth_status {
                    GitHubAuthStatus::Unknown => (
                        "Estado sin verificar",
                        ui.visuals().weak_text_color(),
                    ),
                    GitHubAuthStatus::Authenticated => {
                        ("Sesión iniciada", egui::Color32::from_rgb(117, 220, 173))
                    }
                    GitHubAuthStatus::Unauthenticated => {
                        ("Sin sesión", ui.visuals().warn_fg_color)
                    }
                    GitHubAuthStatus::CliUnavailable => {
                        ("GitHub CLI no disponible", ui.visuals().warn_fg_color)
                    }
                    GitHubAuthStatus::LoginStarted => (
                        "Autorización en curso",
                        ui.visuals().weak_text_color(),
                    ),
                }
            };
            egui::Frame::new()
                .fill(color.gamma_multiply(0.12))
                .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.30)))
                .inner_margin(egui::Margin::symmetric(7, 2))
                .corner_radius(egui::CornerRadius::same(5))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(status).color(color).strong());
                });

            let start = ui.add_enabled(
                can_act
                    && action.is_none()
                    && snapshot.github_auth_status != GitHubAuthStatus::LoginStarted,
                egui::Button::new("Iniciar sesión"),
            );
            if start.clicked() {
                *action = Some(UiAction::StartGitHubBrowserLogin);
            }
            let verify = ui.add_enabled(
                can_act && action.is_none() && !snapshot.github_auth_checking,
                egui::Button::new("Verificar sesión"),
            );
            if verify.clicked() {
                *action = Some(UiAction::CheckGitHubAuth);
            }
        });
        ui.label(
            egui::RichText::new(
            "GitHub CLI abre el navegador externo y copia un código de un solo uso. Gitzi no abre otra consola. Al terminar, pulsa Verificar sesión.",
            )
            .weak(),
        );
    });
}

fn render_github_repository(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading(egui::RichText::new("Crear repositorio desde commits").size(16.0));
        if snapshot.active_repo.is_none() {
            ui.label("Abre un repositorio para publicar sus commits en GitHub.");
            return;
        }

        ui.horizontal_wrapped(|ui| {
            ui.label("Nombre");
            ui.add(
                egui::TextEdit::singleline(&mut state.github_repository_name)
                    .hint_text("nombre-del-repositorio")
                    .desired_width(250.0),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Visibilidad");
            ui.selectable_value(
                &mut state.github_repository_visibility,
                GitHubRepositoryVisibility::Private,
                "Privado",
            );
            ui.selectable_value(
                &mut state.github_repository_visibility,
                GitHubRepositoryVisibility::Public,
                "Público",
            );
        });
        if state.github_repository_visibility == GitHubRepositoryVisibility::Public {
            ui.label(
                egui::RichText::new("El repositorio y los commits serán públicos.")
                    .color(ui.visuals().warn_fg_color),
            );
        }

        let reason = github_repository_disabled_reason(snapshot, &state.github_repository_name);
        if let Some(reason) = reason {
            ui.label(egui::RichText::new(reason).weak());
        }
        let create = ui.add_enabled(
            can_act && action.is_none() && reason.is_none(),
            egui::Button::new("Continuar…"),
        );
        if create.clicked()
            && let (Some(repository), Some(status)) =
                (snapshot.active_repo.clone(), snapshot.repository.as_ref())
        {
            state.confirmation = Some(Confirmation::CreateGitHubRepository {
                name: state.github_repository_name.trim().to_owned(),
                visibility: state.github_repository_visibility,
                repository,
                branch: status.branch.clone(),
            });
        }
    });
}

fn github_repository_disabled_reason(snapshot: &AppSnapshot, name: &str) -> Option<&'static str> {
    if snapshot.github_auth_status != GitHubAuthStatus::Authenticated {
        return Some("Inicia sesión en GitHub y verifica la sesión para continuar.");
    }
    if snapshot.active_repo.is_none() {
        return Some("Abre un repositorio local.");
    }
    let Some(repository) = snapshot.repository.as_ref() else {
        return Some("Espera a que Gitzi termine de consultar el repositorio.");
    };
    if repository.has_head != Some(true) {
        return Some("Crea al menos un commit antes de publicar.");
    }
    if repository.branch.trim().is_empty() {
        return Some("HEAD está separado; cambia a una rama local para publicar.");
    }
    let current_local_branch = snapshot.branches.iter().any(|branch| {
        branch.kind == BranchKind::Local && branch.current && branch.name == repository.branch
    });
    if !current_local_branch {
        return Some(if snapshot.busy {
            "Cargando ramas locales; espera antes de continuar."
        } else {
            "No se confirmó una rama local actual; revisa si HEAD está separado."
        });
    }
    if snapshot
        .remotes
        .iter()
        .any(|remote| remote.name == "origin")
    {
        return Some("El repositorio ya tiene un remoto llamado origin.");
    }
    if !crate::domain::is_valid_github_repository_name(name.trim()) {
        return Some("Escribe un nombre válido para el repositorio.");
    }
    None
}

fn apply_theme(ui: &mut egui::Ui) {
    let mut visuals = egui::Visuals::dark();
    let canvas = egui::Color32::from_rgb(18, 23, 31);
    let panel = egui::Color32::from_rgb(27, 34, 45);
    let raised = egui::Color32::from_rgb(36, 45, 58);
    let accent = egui::Color32::from_rgb(111, 214, 174);

    visuals.panel_fill = canvas;
    visuals.window_fill = panel;
    visuals.faint_bg_color = panel;
    visuals.extreme_bg_color = egui::Color32::from_rgb(13, 17, 23);
    visuals.code_bg_color = egui::Color32::from_rgb(15, 20, 27);
    visuals.hyperlink_color = accent;
    visuals.warn_fg_color = egui::Color32::from_rgb(255, 199, 102);
    visuals.error_fg_color = egui::Color32::from_rgb(255, 133, 137);
    visuals.selection.bg_fill = egui::Color32::from_rgb(48, 94, 82);
    visuals.selection.stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.noninteractive.bg_fill = panel;
    visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(207, 216, 228));
    visuals.widgets.inactive.bg_fill = raised;
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(48, 61, 76);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(44, 82, 72);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    // egui 0.36 usa `active` también para widgets con foco.
    visuals.widgets.active.bg_stroke = egui::Stroke::new(2.0, accent);
    ui.ctx().set_visuals(visuals.clone());

    let style = ui.style_mut();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(7.0, 4.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.interact_size = egui::vec2(36.0, 36.0);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(13.5));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(20.0));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
}

fn render_file_row(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    file: &FileStatus,
    has_head: Option<bool>,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    let selected = state.selected_file.as_ref() == Some(&file.path);
    ui.horizontal(|ui| {
        let summary = if file.unmerged {
            "Conflicto".to_owned()
        } else if file.index_status == '?' && file.worktree_status == '?' {
            "Nuevo · sin seguimiento".to_owned()
        } else {
            format!(
                "Índice: {} · Árbol: {}",
                status_description(file.index_status),
                status_description(file.worktree_status)
            )
        };
        let status_width = 206.0;
        let select_width = (ui.available_width() - status_width - 132.0).max(120.0);
        let select = ui
            .add_sized(
                [select_width, 36.0],
                egui::Button::selectable(
                    selected,
                    egui::RichText::new(file.path.display().to_string()).strong(),
                )
                .truncate(),
            )
            .on_hover_text(file.path.display().to_string());
        if select.clicked() {
            state.selected_file = Some(file.path.clone());
            state.selected_area = preferred_area(file);
            state.requested_diff = None;
            if action.is_none() {
                let area = state.selected_area;
                *action = Some(request_diff(state, snapshot, file.path.clone(), area));
            }
        }
        ui.add_sized(
            [status_width, 36.0],
            egui::Label::new(egui::RichText::new(&summary).weak()).truncate(),
        )
        .on_hover_text(&summary);

        let stage = ui
            .add_enabled(can_act && has_unstaged(file), egui::Button::new("+"))
            .on_hover_text("Preparar este archivo para el commit");
        if stage.clicked() && action.is_none() {
            *action = Some(UiAction::StageFile(file.path.clone()));
        }
        let unstage = ui
            .add_enabled(
                can_act && has_staged(file) && has_head.is_some(),
                egui::Button::new("−"),
            )
            .on_hover_text("Quitar este archivo del área preparada");
        if unstage.clicked() && action.is_none() {
            *action = Some(UiAction::UnstageFile(file.path.clone()));
        }
        let discard = ui
            .add_enabled(
                can_act && !file.unmerged && (has_staged(file) || has_unstaged(file)),
                egui::Button::new("×"),
            )
            .on_hover_text("Descartar los cambios de este archivo…");
        if discard.clicked() && action.is_none() {
            state.confirmation = Some(Confirmation::DiscardFile(file.path.clone()));
        }
    });
    ui.separator();
}
fn status_description(status: char) -> &'static str {
    match status {
        '.' => "limpio",
        'A' => "añadido",
        'M' => "modificado",
        'D' => "eliminado",
        'R' => "renombrado",
        'C' => "copiado",
        'U' => "conflicto",
        '?' => "nuevo",
        _ => "cambiado",
    }
}

fn render_repository_header(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(egui::RichText::new("Gitzi").strong().size(20.0));
                if !snapshot.git_checked {
                    ui.spinner();
                    ui.label("Detectando Git…");
                } else if let Some(version) = &snapshot.git_version {
                    ui.label(egui::RichText::new(format!("Git {version}")).weak());
                } else {
                    ui.label(
                        egui::RichText::new("Git no está disponible")
                            .color(ui.visuals().error_fg_color),
                    );
                }
                ui.separator();
                if let Some(path) = &snapshot.active_repo {
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .filter(|name| !name.is_empty())
                        .unwrap_or("Repositorio");
                    ui.label(egui::RichText::new(name).strong());
                    ui.add_sized(
                        [160.0, 20.0],
                        egui::Label::new(egui::RichText::new(path.display().to_string()).weak())
                            .truncate(),
                    )
                    .on_hover_text(path.display().to_string());
                } else {
                    ui.label(egui::RichText::new("Sin repositorio abierto").weak());
                }
                if snapshot.busy {
                    ui.spinner();
                    ui.label("Trabajando…");
                }
                let open = ui.add_enabled(can_act, egui::Button::new("Abrir repositorio"));
                if open.clicked()
                    && let Some(path) =
                        choose_folder("Selecciona el repositorio", &state.repository_path)
                {
                    state.repository_path = path.display().to_string();
                    *action = Some(UiAction::OpenRepository(path));
                }
                ui.menu_button("Recientes", |menu| {
                    if snapshot.recent_repositories.is_empty() {
                        menu.label(egui::RichText::new("No hay repositorios recientes.").weak());
                    }
                    for path in &snapshot.recent_repositories {
                        let label = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .filter(|name| !name.is_empty())
                            .map(str::to_owned)
                            .unwrap_or_else(|| path.display().to_string());
                        let response = menu.button(label).on_hover_text(path.display().to_string());
                        if response.clicked() && can_act && action.is_none() {
                            state.repository_path = path.display().to_string();
                            *action = Some(UiAction::OpenRepository(path.clone()));
                            menu.close();
                        }
                    }
                });
                let refresh = ui.add_enabled(
                    can_act && snapshot.active_repo.is_some(),
                    egui::Button::new("Actualizar"),
                );
                if refresh.clicked() && action.is_none() {
                    *action = Some(UiAction::Refresh);
                }
                if !state.terminal_visible && ui.button("Mostrar terminal").clicked() {
                    state.terminal_visible = true;
                }
            });
            ui.horizontal_wrapped(|ui| {
                if let Some(repository) = &snapshot.repository {
                    ui.label(egui::RichText::new("Rama").weak());
                    ui.label(
                        egui::RichText::new(if repository.branch.is_empty() {
                            "Sin rama"
                        } else {
                            &repository.branch
                        })
                        .strong(),
                    );
                    ui.separator();
                    let clean = repository.files.is_empty();
                    let label = if clean { "Limpio" } else { "Con cambios" };
                    let color = if clean {
                        egui::Color32::from_rgb(117, 220, 173)
                    } else {
                        ui.visuals().warn_fg_color
                    };
                    ui.label(egui::RichText::new(label).strong().color(color));
                    ui.label(format!("{} archivos cambiados", repository.files.len()));
                    if repository.has_head == Some(false) {
                        ui.label(
                            egui::RichText::new("Sin commits").color(ui.visuals().warn_fg_color),
                        );
                    }
                    if let Some(checked_at) = snapshot.status_checked_at {
                        ui.label(egui::RichText::new("●").weak())
                            .on_hover_text(format!(
                                "Último intento de actualizar el estado: {}",
                                format_known_time(checked_at)
                            ));
                    }
                } else if snapshot.active_repo.is_some() && snapshot.busy {
                    ui.spinner();
                    ui.label("Consultando repositorio…");
                }
            });
        });
}
fn render_message(ui: &mut egui::Ui, title: &str, message: &str, color: egui::Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.10))
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.32)))
        .inner_margin(egui::Margin::symmetric(10, 5))
        .corner_radius(egui::CornerRadius::same(6))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(title).strong().color(color));
                ui.label(egui::RichText::new(message).color(color));
            });
        });
}

fn render_code_control(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    let (Some(_path), Some(repository)) = (&snapshot.active_repo, &snapshot.repository) else {
        empty_state(
            ui,
            "Abre un repositorio",
            "Elige una carpeta desde el encabezado para revisar cambios y sincronizar.",
        );
        return;
    };
    if state
        .selected_file
        .as_ref()
        .is_none_or(|selected| !repository.files.iter().any(|file| &file.path == selected))
    {
        state.selected_file = repository.files.first().map(|file| file.path.clone());
    }
    let has_upstream = repository.upstream.is_some();
    let has_current_local_branch = snapshot.branches.iter().any(|branch| {
        branch.kind == BranchKind::Local && branch.current && branch.name == repository.branch
    });
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if has_upstream {
                ui.label(egui::RichText::new(format!("{} commits por subir", repository.ahead)).strong());
                ui.separator();
                ui.label(egui::RichText::new(format!("{} commits por bajar", repository.behind)).strong());
                ui.label(egui::RichText::new("frente a refs locales conocidas; puede cambiar tras Fetch").weak());
            } else {
                ui.label(egui::RichText::new("Sin upstream").strong().color(ui.visuals().warn_fg_color));
                ui.label(egui::RichText::new("Publica esta rama o configura un upstream en más opciones.").weak());
            }
        });
        ui.horizontal_wrapped(|ui| {
            let fetch = ui.add_enabled(can_act && !snapshot.remotes.is_empty(), egui::Button::new("Fetch"));
            if fetch.clicked() && action.is_none() { *action = Some(UiAction::Fetch); }
            let pull_label = if has_upstream {
                format!("Bajar {} commits", repository.behind)
            } else { "Bajar cambios".to_owned() };
            let pull = ui.add_enabled(can_act && has_upstream && has_current_local_branch, egui::Button::new(pull_label))
                .on_hover_text("Pull: integra los commits del upstream según la política seleccionada");
            if pull.clicked() && action.is_none() { *action = Some(UiAction::Pull); }
            let push_label = if has_upstream {
                format!("Subir {} commits", repository.ahead)
            } else { "Subir cambios".to_owned() };
            let push = ui.add_enabled(can_act && has_upstream && has_current_local_branch, egui::Button::new(push_label))
                .on_hover_text("Push: publica los commits locales en el upstream");
            if push.clicked() && action.is_none() { *action = Some(UiAction::Push); }
            if !has_upstream && !snapshot.remotes.is_empty() {
                let remotes: Vec<String> = snapshot.remotes.iter().map(|remote| remote.name.clone()).collect();
                if !remotes.iter().any(|name| name == &state.selected_remote) { state.selected_remote.clone_from(&remotes[0]); }
                egui::ComboBox::from_id_salt("publish_remote").selected_text(&state.selected_remote).show_ui(ui, |ui| {
                    for remote in &remotes { ui.selectable_value(&mut state.selected_remote, remote.clone(), remote); }
                });
                let publish = ui.add_enabled(can_act && action.is_none() && !has_upstream && has_current_local_branch && repository.has_head == Some(true), egui::Button::new("Publicar rama y configurar upstream"));
                if publish.clicked() && action.is_none() { *action = Some(UiAction::PublishBranch(state.selected_remote.clone())); }
            } else if !has_upstream {
                ui.label(
                    egui::RichText::new("Añade un remoto para poder usar Fetch o publicar.")
                        .weak(),
                );
            }
        });
        if !has_upstream && !snapshot.remotes.is_empty() {
            ui.label(egui::RichText::new("Pull y Push se habilitan al publicar la rama o asociar un upstream existente.").weak());
        }
        ui.collapsing("Más opciones de sincronización", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Política de Pull");
                let mut policy = snapshot.pull_policy;
                egui::ComboBox::from_id_salt("pull_policy").selected_text(match policy {
                    PullPolicy::FastForwardOnly => "Solo avance rápido",
                    PullPolicy::Merge => "Merge",
                    PullPolicy::Rebase => "Rebase",
                }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut policy, PullPolicy::FastForwardOnly, "Solo avance rápido");
                    ui.selectable_value(&mut policy, PullPolicy::Merge, "Merge");
                    ui.selectable_value(&mut policy, PullPolicy::Rebase, "Rebase");
                });
                if policy != snapshot.pull_policy && can_act && action.is_none() { *action = Some(UiAction::SetPullPolicy(policy)); }
                ui.label(egui::RichText::new("Define cómo integrar al usar Pull.").weak());
            });
            let local: Vec<&BranchInfo> = snapshot.branches.iter().filter(|branch| branch.kind == BranchKind::Local).collect();
            let remote: Vec<&BranchInfo> = snapshot.branches.iter().filter(|branch| branch.kind == BranchKind::Remote).collect();
            if !local.is_empty() && !remote.is_empty() {
                let current_local = local.iter().find(|branch| branch.current).map(|branch| branch.name.as_str()).unwrap_or(&repository.branch);
                if !local.iter().any(|branch| branch.name == state.selected_local_branch) { state.selected_local_branch = current_local.to_owned(); }
                if !remote.iter().any(|branch| branch.name == state.selected_remote_branch) { state.selected_remote_branch = remote[0].name.clone(); }
                ui.horizontal_wrapped(|ui| {
                    egui::ComboBox::from_label("Rama local").selected_text(&state.selected_local_branch).show_ui(ui, |ui| {
                        for branch in &local { ui.selectable_value(&mut state.selected_local_branch, branch.name.clone(), &branch.name); }
                    });
                    egui::ComboBox::from_label("Upstream remoto existente").selected_text(&state.selected_remote_branch).show_ui(ui, |ui| {
                        for branch in &remote { ui.selectable_value(&mut state.selected_remote_branch, branch.name.clone(), &branch.name); }
                    });
                    let set = ui.add_enabled(can_act && action.is_none(), egui::Button::new("Configurar upstream"));
                    if set.clicked() && action.is_none() { *action = Some(UiAction::SetUpstream { local_branch: state.selected_local_branch.clone(), remote_branch: state.selected_remote_branch.clone() }); }
                });
            } else {
                ui.label(egui::RichText::new("Para asociar un upstream existente, hace falta una rama local y una remota conocidas.").weak());
            }
            ui.label(egui::RichText::new(format!("Último Fetch conocido: {}", repository.last_fetch_at.map_or_else(|| "sin dato".to_owned(), format_known_time))).weak());
        });
    });
    ui.add_space(5.0);
    if repository.files.is_empty() {
        ui.label(egui::RichText::new("No hay cambios locales sin commit.").weak());
    } else {
        render_commit(ui, state, snapshot, can_act, action);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Archivos cambiados · {}", repository.files.len()))
                    .strong(),
            );
            ui.label(egui::RichText::new("Selecciona una fila para revisar su diff.").weak());
        });
        for file in &repository.files {
            render_file_row(
                ui,
                state,
                snapshot,
                file,
                repository.has_head,
                can_act,
                action,
            );
        }
    }
    render_file_review(ui, state, snapshot, &repository.files, can_act, action);
}
fn render_repository_tab(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Configuración del repositorio");
        render_github_login(ui, snapshot, can_act, action);
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Inicializar repositorio").strong());
        ui.horizontal_wrapped(|ui| {
            ui.label("Ruta");
            let field_width = (ui.available_width() - 250.0).clamp(140.0, 540.0);
            ui.add(
                egui::TextEdit::singleline(&mut state.repository_path)
                    .hint_text("Elige una carpeta para inicializar")
                    .desired_width(field_width)
                    .id_source("repository_path"),
            );
            let choose = ui.add_sized([132.0, 40.0], egui::Button::new("Elegir carpeta…"));
            if choose.clicked()
                && let Some(path) = choose_folder(
                    "Selecciona una carpeta para inicializar",
                    &state.repository_path,
                )
            {
                state.repository_path = path.display().to_string();
            }
        });
        let selected_path = PathBuf::from(state.repository_path.trim());
        let existing_repo = if state.repository_path.trim().is_empty() {
            None
        } else {
            git_repository_for_path(&selected_path)
        };
        ui.horizontal_wrapped(|ui| {
            let initialize = ui.add_enabled(
                can_act
                    && action.is_none()
                    && !state.repository_path.trim().is_empty()
                    && existing_repo.is_none(),
                egui::Button::new("Inicializar"),
            );
            if initialize.clicked() && action.is_none() {
                *action = Some(UiAction::InitializeRepository(selected_path.clone()));
            }
            if let Some(repository_path) = &existing_repo {
                ui.label(
                    egui::RichText::new(format!(
                        "Esta ruta ya está dentro del repositorio {}.",
                        repository_path.display()
                    ))
                    .color(ui.visuals().warn_fg_color),
                );
            } else {
                initialize.on_hover_text("Crear un repositorio Git en esta ruta");
            }
        });
        ui.separator();
        ui.label(egui::RichText::new("Clonar repositorio").strong());
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.clone_source)
                    .hint_text("URL o ruta de origen")
                    .desired_width((ui.available_width() - 20.0).clamp(220.0, 590.0)),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.clone_destination)
                    .hint_text("Destino local")
                    .desired_width((ui.available_width() - 290.0).clamp(180.0, 440.0)),
            );
            let choose = ui.add_sized([132.0, 40.0], egui::Button::new("Elegir carpeta…"));
            if choose.clicked()
                && let Some(path) =
                    choose_folder("Selecciona el destino del clon", &state.clone_destination)
            {
                state.clone_destination = path.display().to_string();
            }
            let clone = ui.add_enabled(
                can_act
                    && !state.clone_source.trim().is_empty()
                    && !state.clone_destination.trim().is_empty(),
                egui::Button::new("Clonar"),
            );
            if clone.clicked() && action.is_none() {
                *action = Some(UiAction::CloneRepository {
                    source: state.clone_source.clone(),
                    destination: PathBuf::from(&state.clone_destination),
                });
            }
        });
    });
    ui.add_space(8.0);
    render_github_repository(ui, state, snapshot, can_act, action);
    ui.add_space(8.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new("Editor externo para conflictos").strong());
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.editor_path)
                    .hint_text("Ruta del ejecutable")
                    .desired_width(260.0),
            );
            let save = ui.add_enabled(
                can_act
                    && action.is_none()
                    && state.editor_path.trim()
                        != snapshot
                            .external_editor
                            .as_ref()
                            .map_or("", |path| path.to_str().unwrap_or("")),
                egui::Button::new("Guardar editor"),
            );
            if save.clicked() {
                let path = state.editor_path.trim();
                *action = Some(UiAction::SetExternalEditor(
                    (!path.is_empty()).then(|| PathBuf::from(path)),
                ));
            }
            let clear = ui.add_enabled(
                can_act && action.is_none() && snapshot.external_editor.is_some(),
                egui::Button::new("Quitar"),
            );
            if clear.clicked() {
                state.editor_path.clear();
                *action = Some(UiAction::SetExternalEditor(None));
            }
        });
        if snapshot.external_editor.is_none() {
            ui.label(
                egui::RichText::new("Sin editor configurado: abrir conflictos no está disponible.")
                    .weak(),
            );
        }
    });
}
fn git_repository_for_path(path: &std::path::Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();
    if current.is_file() {
        current.pop();
    }
    loop {
        let git_marker = current.join(".git");
        if git_marker.is_dir() || git_marker.is_file() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn render_history_tab(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    if snapshot.active_repo.is_none() {
        empty_state(ui, "Abre un repositorio", "El historial aparecerá aquí.");
        return;
    }
    ui.heading("Historial");
    if snapshot.history.is_empty() {
        if snapshot.busy {
            loading_state(ui, "Cargando historial…");
        } else {
            ui.label(egui::RichText::new("No hay commits para mostrar.").weak());
        }
    }
    egui::ScrollArea::vertical()
        .id_salt("history_list")
        .max_height(220.0)
        .show(ui, |ui| {
            for commit in &snapshot.history {
                ui.horizontal_wrapped(|ui| {
                    let selected = state.selected_commit.as_deref() == Some(&commit.id);
                    let response = ui.selectable_label(
                        selected,
                        format!("{} · {}", short_id(&commit.id), commit.subject),
                    );
                    if response.clicked() && action.is_none() && can_act {
                        state.selected_commit = Some(commit.id.clone());
                        *action = Some(UiAction::LoadCommitDetail(commit.id.clone()));
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {}",
                            commit.author,
                            format_known_time(commit.authored_at)
                        ))
                        .weak(),
                    );
                });
            }
        });
    if snapshot.history_has_more {
        let more = ui.add_enabled(
            can_act && action.is_none(),
            egui::Button::new("Cargar más commits"),
        );
        if more.clicked() && action.is_none() {
            *action = Some(UiAction::LoadMoreHistory);
        }
    }
    if let Some(detail) = &snapshot.commit_detail {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Commit {} · {}",
                    short_id(&detail.summary.id),
                    detail.summary.subject
                ))
                .strong(),
            );
            ui.label(format!(
                "Autor: {} · {}",
                detail.summary.author,
                format_known_time(detail.summary.authored_at)
            ));
            egui::ScrollArea::vertical()
                .id_salt("commit_message")
                .max_height(120.0)
                .show(ui, |ui| ui.monospace(&detail.message));
            ui.label(format!(
                "Archivos modificados ({})",
                detail.changed_files.len()
            ));
            egui::ScrollArea::both()
                .id_salt("commit_changed_files")
                .max_height(140.0)
                .show(ui, |ui| {
                    for file in &detail.changed_files {
                        ui.label(format!("{}  {}", file.status, file.path.display()));
                    }
                });
        });
    } else if state.selected_commit.is_some() && snapshot.busy {
        loading_state(ui, "Cargando detalle del commit…");
    }
    let refs: Vec<String> = snapshot
        .branches
        .iter()
        .map(|branch| branch.name.clone())
        .collect();
    if refs.is_empty() {
        ui.label(egui::RichText::new("Carga ramas para comparar referencias.").weak());
        return;
    }
    if !refs.iter().any(|name| name == &state.compare_base) {
        state.compare_base.clone_from(&refs[0]);
    }
    if !refs.iter().any(|name| name == &state.compare_target) {
        state.compare_target = refs.get(1).unwrap_or(&refs[0]).clone();
    }
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_label("Base")
            .selected_text(&state.compare_base)
            .show_ui(ui, |ui| {
                for reference in &refs {
                    ui.selectable_value(&mut state.compare_base, reference.clone(), reference);
                }
            });
        egui::ComboBox::from_label("Objetivo")
            .selected_text(&state.compare_target)
            .show_ui(ui, |ui| {
                for reference in &refs {
                    ui.selectable_value(&mut state.compare_target, reference.clone(), reference);
                }
            });
        let compare = ui.add_enabled(
            can_act && action.is_none() && state.compare_base != state.compare_target,
            egui::Button::new("Comparar"),
        );
        if compare.clicked() && action.is_none() {
            *action = Some(UiAction::CompareReferences {
                base_ref: state.compare_base.clone(),
                target_ref: state.compare_target.clone(),
            });
        }
    });
    if let Some(comparison) = snapshot.ref_comparison.as_ref().filter(|comparison| {
        comparison.base_ref == state.compare_base && comparison.target_ref == state.compare_target
    }) {
        ui.label(format!(
            "{} → {} · ↑ {} commits · ↓ {} commits",
            comparison.base_ref,
            comparison.target_ref,
            comparison.commits_ahead,
            comparison.commits_behind
        ));
        if comparison.truncated {
            ui.label(
                egui::RichText::new("Diff truncado; el contenido está incompleto.")
                    .color(ui.visuals().warn_fg_color),
            );
        }
        egui::ScrollArea::both()
            .id_salt("ref_comparison")
            .max_height(230.0)
            .show(ui, |ui| ui.monospace(&comparison.diff));
    } else {
        ui.label(
            egui::RichText::new("Pulsa Comparar para cargar el resultado de estas referencias.")
                .weak(),
        );
    }
}

fn render_branches_tab(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    let Some(repository) = &snapshot.repository else {
        empty_state(
            ui,
            "Abre un repositorio",
            "La gestión de ramas aparecerá aquí.",
        );
        return;
    };
    ui.heading("Ramas");
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.branch_name)
                .hint_text("Nombre de nueva rama")
                .desired_width(220.0),
        );
        let create = ui.add_enabled(
            can_act
                && action.is_none()
                && !state.branch_name.trim().is_empty()
                && repository.has_head == Some(true),
            egui::Button::new("Crear y cambiar"),
        );
        if create.clicked() && action.is_none() {
            *action = Some(UiAction::CreateBranch(state.branch_name.trim().to_owned()));
        }
    });
    if repository.has_head == Some(false) {
        ui.label(egui::RichText::new("Crea el primer commit antes de crear otra rama.").weak());
    }
    if snapshot.branches.is_empty() {
        if snapshot.busy {
            loading_state(ui, "Cargando ramas…");
        } else {
            ui.label(egui::RichText::new("No hay ramas disponibles.").weak());
        }
    } else {
        egui::ScrollArea::vertical()
            .id_salt("branches_list")
            .max_height(350.0)
            .show(ui, |ui| {
                for branch in &snapshot.branches {
                    ui.horizontal_wrapped(|ui| {
                        let kind = match branch.kind {
                            BranchKind::Local => "Local",
                            BranchKind::Remote => "Remota",
                        };
                        ui.label(format!(
                            "{kind} · {}{}",
                            branch.name,
                            if branch.current { " · actual" } else { "" }
                        ));
                        if let Some(upstream) = &branch.upstream {
                            ui.label(egui::RichText::new(format!("→ {upstream}")).weak());
                        }
                        if branch.kind == BranchKind::Local && !branch.current {
                            let switch = ui.add_enabled(
                                can_act && action.is_none(),
                                egui::Button::new("Cambiar"),
                            );
                            if switch.clicked() && action.is_none() {
                                *action = Some(UiAction::SwitchBranch(branch.name.clone()));
                            }
                            let delete = ui.add_enabled(
                                can_act && action.is_none(),
                                egui::Button::new("Eliminar…"),
                            );
                            if delete.clicked() {
                                state.confirmation =
                                    Some(Confirmation::DeleteBranch(branch.name.clone()));
                            }
                        }
                    });
                }
            });
    }
}

fn render_integrate_tab(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    let Some(repository) = &snapshot.repository else {
        empty_state(
            ui,
            "Abre un repositorio",
            "Merge, rebase, revert y stash aparecerán aquí.",
        );
        return;
    };
    ui.heading("Integrar y recuperar");
    render_recovery(ui, state, snapshot, repository, can_act, action);
}
fn short_id(id: &str) -> &str {
    id.get(..id.len().min(12)).unwrap_or(id)
}

fn render_recovery(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    repository: &crate::domain::RepositoryStatus,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    let local: Vec<&BranchInfo> = snapshot
        .branches
        .iter()
        .filter(|branch| branch.kind == BranchKind::Local)
        .collect();
    ui.collapsing("Integración y recuperación", |ui| {
        let conflicts: Vec<&FileStatus> = repository.files.iter().filter(|file| file.unmerged || file.index_status == 'U' || file.worktree_status == 'U').collect();
        if let Some(kind) = repository.integration {
            ui.label(egui::RichText::new(format!("Operación en curso: {}", match kind { IntegrationKind::Merge => "Merge", IntegrationKind::Rebase => "Rebase", IntegrationKind::Revert => "Revert" })).strong());
            if conflicts.is_empty() {
                ui.label("No quedan conflictos detectados. Puedes continuar o abortar la operación.");
            } else {
                ui.label(egui::RichText::new(format!("Conflictos sin resolver: {}. Selecciona Preparar para marcar un archivo resuelto.", conflicts.len())).color(ui.visuals().warn_fg_color));
                for file in &conflicts {
                    ui.horizontal_wrapped(|ui| {
                        ui.monospace(file.path.display().to_string());
                        let editor = ui.add_enabled(can_act && action.is_none() && snapshot.external_editor.is_some(), egui::Button::new("Abrir en editor"));
                        if editor.clicked() { *action = Some(UiAction::OpenConflictFile(file.path.clone())); }
                        let resolved = ui.add_enabled(can_act && action.is_none(), egui::Button::new("Preparar como resuelto"));
                        if resolved.clicked() { *action = Some(UiAction::StageFile(file.path.clone())); }
                    });
                }
            }
            ui.horizontal(|ui| {
                let continue_op = ui.add_enabled(can_act && action.is_none() && conflicts.is_empty(), egui::Button::new("Continuar integración"));
                if continue_op.clicked() { *action = Some(UiAction::ContinueIntegration); }
                let abort = ui.add_enabled(can_act && action.is_none(), egui::Button::new("Abortar integración…"));
                if abort.clicked() {
                    state.confirmation = Some(Confirmation::AbortIntegration {
                        kind: match kind { IntegrationKind::Merge => "merge", IntegrationKind::Rebase => "rebase", IntegrationKind::Revert => "revert" }.to_owned(),
                    });
                }
            });
        } else {
            ui.label("No hay una integración en curso.");
            let clean = repository.files.is_empty();
            let ready = can_act && action.is_none() && clean && repository.has_head == Some(true);
            if local.is_empty() {
                ui.label("No hay ramas locales para integrar.");
            } else {
                let branches: Vec<&BranchInfo> = local.iter().copied().filter(|branch| !branch.current).collect();
                if branches.is_empty() { ui.label("No hay otra rama local disponible."); }
                else {
                    if !branches.iter().any(|branch| branch.name == state.selected_integration_branch) { state.selected_integration_branch = branches[0].name.clone(); }
                    egui::ComboBox::from_label("Rama local de origen").selected_text(&state.selected_integration_branch).show_ui(ui, |ui| {
                        for branch in &branches { ui.selectable_value(&mut state.selected_integration_branch, branch.name.clone(), &branch.name); }
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(ready, egui::Button::new("Merge rama")).clicked() { *action = Some(UiAction::MergeBranch(state.selected_integration_branch.clone())); }
                        if ui.add_enabled(ready, egui::Button::new("Rebase sobre rama")).clicked() { *action = Some(UiAction::RebaseBranch(state.selected_integration_branch.clone())); }
                    });
                }
            }
            if !clean { ui.label("Merge, Rebase y Revert requieren un árbol de trabajo limpio."); }
            let selected = snapshot.history.iter().find(|commit| state.selected_commit.as_deref() == Some(&commit.id));
            ui.horizontal_wrapped(|ui| {
                ui.label(selected.map_or("Selecciona un commit del historial para revertir.".to_owned(), |commit| format!("Commit seleccionado: {} · {}", short_id(&commit.id), commit.subject)));
                let revert = ui.add_enabled(ready && selected.is_some(), egui::Button::new("Revertir commit seleccionado"));
                if revert.clicked() && let Some(commit) = selected {
                    *action = Some(UiAction::RevertCommit(commit.id.clone()));
                }
            });
        }
    });

    ui.collapsing("Stash", |ui| {
        ui.label("Guarda temporalmente cambios sin crear un commit.");
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.stash_message)
                    .hint_text("Mensaje opcional")
                    .desired_width(240.0),
            );
            ui.checkbox(
                &mut state.stash_include_untracked,
                "Incluir archivos sin seguimiento",
            );
            let has_changes = !repository.files.is_empty();
            if ui
                .add_enabled(
                    can_act && action.is_none() && has_changes,
                    egui::Button::new("Crear stash"),
                )
                .clicked()
            {
                *action = Some(UiAction::CreateStash {
                    message: (!state.stash_message.trim().is_empty())
                        .then(|| state.stash_message.trim().to_owned()),
                    include_untracked: state.stash_include_untracked,
                });
            }
        });
        if snapshot.stashes.is_empty() {
            ui.label(if snapshot.busy {
                "Cargando stashes…"
            } else {
                "No hay stashes en este repositorio."
            });
        } else {
            egui::ScrollArea::vertical()
                .id_salt("stash_list")
                .max_height(150.0)
                .show(ui, |ui| {
                    for stash in &snapshot.stashes {
                        ui.horizontal_wrapped(|ui| {
                            let label = format!("{} · {}", stash.selector, stash.message);
                            let selected = state.selected_stash.as_deref() == Some(&stash.oid);
                            if ui.selectable_label(selected, label.clone()).clicked() {
                                if !selected {
                                    state.selected_stash = Some(stash.oid.clone());
                                }
                                if action.is_none() {
                                    *action = Some(UiAction::LoadStashDetail(stash.oid.clone()));
                                }
                            }
                            if ui
                                .add_enabled(
                                    can_act && action.is_none(),
                                    egui::Button::new("Aplicar sin eliminar"),
                                )
                                .clicked()
                            {
                                *action = Some(UiAction::ApplyStash(stash.oid.clone()));
                            }
                            if ui
                                .add_enabled(
                                    can_act && action.is_none(),
                                    egui::Button::new("Eliminar…"),
                                )
                                .clicked()
                            {
                                state.confirmation = Some(Confirmation::DropStash {
                                    oid: stash.oid.clone(),
                                    label,
                                });
                            }
                        });
                    }
                });
            if let Some(detail) = snapshot
                .stash_detail
                .as_ref()
                .filter(|detail| state.selected_stash.as_deref() == Some(&detail.oid))
            {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(egui::RichText::new("Detalle del stash seleccionado").strong());
                    if detail.truncated {
                        ui.label(
                            egui::RichText::new("Diff truncado; el contenido está incompleto.")
                                .color(ui.visuals().warn_fg_color),
                        );
                    }
                    egui::ScrollArea::both()
                        .id_salt("stash_detail")
                        .max_height(220.0)
                        .show(ui, |ui| ui.monospace(&detail.diff));
                });
            } else if state.selected_stash.is_some() && snapshot.busy {
                loading_state(ui, "Cargando detalle del stash…");
            }
        }
    });
}

fn render_confirmation(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    let Some(confirmation) = state.confirmation.clone() else {
        return;
    };
    let mut open = true;
    egui::Window::new("Confirmar operación")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            let mut confirmation_valid = can_act;
            match &confirmation {
                Confirmation::DropStash { label, .. } => {
                    ui.label(format!("Se eliminará este stash de forma permanente: {label}"));
                    ui.label("Esta acción no se puede deshacer desde Gitzi.");
                }
                Confirmation::DiscardFile(path) => {
                    ui.label("Se eliminarán los cambios preparados y sin preparar de este archivo:");
                    ui.monospace(path.display().to_string());
                    ui.label("Los cambios de este archivo no se conservarán.");
                }
                Confirmation::DeleteBranch(name) => {
                    ui.label("Se solicitará eliminar esta rama local (sin forzar):");
                    ui.monospace(name);
                    ui.label("Git rechazará el borrado si la rama no está fusionada.");
                }
                Confirmation::AbortIntegration { kind } => {
                    ui.label(format!("Se abortará la operación de {kind} en curso."));
                    ui.label("Se descartará la integración y los cambios preparados y sin preparar realizados al resolver conflictos.");
                }
                Confirmation::CreateGitHubRepository {
                    name,
                    visibility,
                    repository,
                    branch,
                } => {
                    let visibility_label = match visibility {
                        GitHubRepositoryVisibility::Private => "privado",
                        GitHubRepositoryVisibility::Public => "público",
                    };
                    ui.label(format!(
                        "Se creará un repositorio {visibility_label} llamado {name} en la cuenta activa de GitHub."
                    ));
                    ui.label(format!(
                        "Se añadirá origin y se publicarán los commits de la rama actual ({branch})."
                    ));
                    ui.label("Los cambios que aún no tienen commit permanecerán solo en este equipo.");
                    if *visibility == GitHubRepositoryVisibility::Public {
                        ui.label(
                            egui::RichText::new("El repositorio será visible públicamente.")
                                .color(ui.visuals().warn_fg_color)
                                .strong(),
                        );
                    }

                    let current_branch = snapshot.repository.as_ref().map(|status| &status.branch);
                    if snapshot.active_repo.as_ref() != Some(repository)
                        || current_branch.is_none_or(|current| current != branch)
                    {
                        confirmation_valid = false;
                        ui.label(
                            egui::RichText::new(
                                "El repositorio o la rama cambió. Cierra esta confirmación y revisa los datos.",
                            )
                            .color(ui.visuals().error_fg_color),
                        );
                    } else if let Some(reason) = github_repository_disabled_reason(snapshot, name) {
                        confirmation_valid = false;
                        ui.label(
                            egui::RichText::new(reason).color(ui.visuals().error_fg_color),
                        );
                    }
                }
            }
            ui.horizontal(|ui| {
                let label = if matches!(&confirmation, Confirmation::CreateGitHubRepository { .. }) {
                    "Crear y publicar"
                } else {
                    "Confirmar operación"
                };
                let confirm = ui.add_enabled(
                    confirmation_valid && action.is_none(),
                    egui::Button::new(label),
                );
                if confirm.clicked() {
                    *action = Some(match confirmation {
                        Confirmation::DropStash { oid, .. } => UiAction::DropStash(oid),
                        Confirmation::DiscardFile(path) => UiAction::DiscardFile(path),
                        Confirmation::DeleteBranch(name) => UiAction::DeleteBranch(name),
                        Confirmation::AbortIntegration { .. } => UiAction::AbortIntegration,
                        Confirmation::CreateGitHubRepository {
                            name, visibility, ..
                        } => UiAction::CreateGitHubRepository {
                            name,
                            visibility,
                        },
                    });
                    state.confirmation = None;
                }
                if ui.button("Cancelar").clicked() { state.confirmation = None; }
            });
        });
    if !open {
        state.confirmation = None;
    }
}

fn format_known_time(time: std::time::SystemTime) -> String {
    let Ok(duration) = time.duration_since(UNIX_EPOCH) else {
        return "fecha anterior a 1970".to_owned();
    };
    let seconds = duration.as_secs();
    let days = (seconds / 86_400) as i64;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let seconds_of_day = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        seconds_of_day / 3_600,
        (seconds_of_day / 60) % 60
    )
}

fn loading_state(ui: &mut egui::Ui, text: &str) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(text);
        });
    });
}

fn render_file_review(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    files: &[FileStatus],
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    let Some(path) = state.selected_file.clone() else {
        return;
    };
    let Some(file) = files.iter().find(|file| file.path == path) else {
        return;
    };

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Revisar archivo");
            ui.label(egui::RichText::new(path.display().to_string()).weak());
        });
        ui.horizontal(|ui| {
            let unstaged = ui.add_enabled(
                can_act && has_unstaged(file),
                egui::Button::selectable(state.selected_area == DiffArea::Unstaged, "Sin preparar"),
            );
            if unstaged.clicked() {
                state.selected_area = DiffArea::Unstaged;
                if can_act && action.is_none() {
                    *action = Some(request_diff(
                        state,
                        snapshot,
                        path.clone(),
                        DiffArea::Unstaged,
                    ));
                } else {
                    state.requested_diff = None;
                }
            }
            let staged = ui.add_enabled(
                can_act && has_staged(file),
                egui::Button::selectable(state.selected_area == DiffArea::Staged, "Preparado"),
            );
            if staged.clicked() {
                state.selected_area = DiffArea::Staged;
                if can_act && action.is_none() {
                    *action = Some(request_diff(
                        state,
                        snapshot,
                        path.clone(),
                        DiffArea::Staged,
                    ));
                } else {
                    state.requested_diff = None;
                }
            }
            let load = ui.add_enabled(
                can_act && area_available(file, state.selected_area),
                egui::Button::new("Cargar diff"),
            );
            if load.clicked() && action.is_none() {
                *action = Some(request_diff(
                    state,
                    snapshot,
                    path.clone(),
                    state.selected_area,
                ));
            }
        });

        if !area_available(file, state.selected_area) {
            ui.label(egui::RichText::new("No hay cambios en esta área para el archivo.").weak());
            return;
        }
        match snapshot.file_diff.as_ref().filter(|diff| {
            snapshot.active_repo.as_ref() == Some(&diff.repository)
                && diff.path == path
                && diff.area == state.selected_area
                && state
                    .requested_diff
                    .as_ref()
                    .is_some_and(|(repository, file, area)| {
                        repository == &diff.repository && file == &diff.path && *area == diff.area
                    })
        }) {
            Some(diff) if diff.binary => {
                ui.label(
                    egui::RichText::new("Archivo binario · Git no proporciona diff de texto.")
                        .strong()
                        .color(ui.visuals().warn_fg_color),
                );
                if diff.truncated {
                    ui.label(
                        egui::RichText::new("La salida del diff está truncada.")
                            .color(ui.visuals().warn_fg_color),
                    );
                }
            }
            Some(diff) => {
                if diff.truncated {
                    ui.label(
                        egui::RichText::new("Diff truncado: la salida está incompleta.")
                            .color(ui.visuals().warn_fg_color),
                    );
                }
                egui::ScrollArea::both()
                    .id_salt(("file_diff", path.clone(), state.selected_area as u8))
                    .max_height(160.0)
                    .show(ui, |ui| {
                        ui.monospace(&diff.text);
                    });
            }
            None => {
                ui.label(egui::RichText::new("Carga el diff para revisar este archivo.").weak());
            }
        }
    });
}

fn render_commit(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    let has_staged = snapshot
        .repository
        .as_ref()
        .is_some_and(|repository| repository.files.iter().any(has_staged));
    let has_conflicts = snapshot.repository.as_ref().is_some_and(|repository| {
        repository
            .files
            .iter()
            .any(|file| file.unmerged || file.index_status == 'U' || file.worktree_status == 'U')
    });
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(egui::Margin::symmetric(8, 5))
        .corner_radius(egui::CornerRadius::same(6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Commit").strong());
                let input_width = (ui.available_width() - 166.0).max(160.0);
                ui.add_sized(
                    [input_width, 56.0],
                    egui::TextEdit::multiline(&mut state.commit_message)
                        .hint_text("Mensaje del commit")
                        .desired_rows(2),
                );
                let commit = ui.add_enabled(
                    can_act
                        && has_staged
                        && !has_conflicts
                        && !state.commit_message.trim().is_empty(),
                    egui::Button::new("Crear commit"),
                );
                if commit.clicked() && action.is_none() {
                    *action = Some(UiAction::Commit(state.commit_message.clone()));
                }
            });
            if has_conflicts {
                ui.label(
                    egui::RichText::new("Resuelve los conflictos antes de crear un commit.")
                        .color(ui.visuals().warn_fg_color),
                );
            }
            if !has_staged {
                ui.label(
                    egui::RichText::new("Prepara al menos un archivo para crear un commit.").weak(),
                );
            }
        });
}

fn preferred_area(file: &FileStatus) -> DiffArea {
    if has_unstaged(file) {
        DiffArea::Unstaged
    } else {
        DiffArea::Staged
    }
}

fn has_unstaged(file: &FileStatus) -> bool {
    file.worktree_status != '.'
}

fn has_staged(file: &FileStatus) -> bool {
    file.index_status != '.' && file.index_status != '?'
}

fn area_available(file: &FileStatus, area: DiffArea) -> bool {
    match area {
        DiffArea::Unstaged => has_unstaged(file),
        DiffArea::Staged => has_staged(file),
    }
}

fn request_diff(
    state: &mut UiState,
    snapshot: &AppSnapshot,
    path: PathBuf,
    area: DiffArea,
) -> UiAction {
    if let Some(repository) = &snapshot.active_repo {
        state.requested_diff = Some((repository.clone(), path.clone(), area));
    } else {
        state.requested_diff = None;
    }
    UiAction::LoadDiff { path, area }
}

fn empty_state(ui: &mut egui::Ui, title: &str, detail: &str) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).strong());
            ui.label(egui::RichText::new(detail).weak());
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{has_staged, has_unstaged};
    use crate::domain::FileStatus;
    use std::path::PathBuf;

    #[test]
    fn status_sentinels_distinguish_clean_and_untracked_files() {
        let clean = FileStatus {
            path: PathBuf::from("clean.txt"),
            index_status: '.',
            worktree_status: '.',
            unmerged: false,
        };
        let untracked = FileStatus {
            path: PathBuf::from("new.txt"),
            index_status: '?',
            worktree_status: '?',
            unmerged: false,
        };

        assert!(!has_staged(&clean));
        assert!(!has_unstaged(&clean));
        assert!(!has_staged(&untracked));
        assert!(has_unstaged(&untracked));
    }

    #[test]
    fn staged_and_unstaged_changes_can_coexist() {
        let partially_staged = FileStatus {
            path: PathBuf::from("changed.txt"),
            index_status: 'M',
            worktree_status: 'M',
            unmerged: false,
        };

        assert!(has_staged(&partially_staged));
        assert!(has_unstaged(&partially_staged));
    }
}
