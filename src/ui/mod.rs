use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::egui;

use crate::domain::{
    AppSnapshot, BranchInfo, BranchKind, CommandEntry, CommandStatus, DiffArea, FileStatus,
    UiAction,
};

pub struct UiState {
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
    selected_remote: String,
    selected_remote_branch: String,
    selected_commit: Option<String>,
    compare_base: String,
    compare_target: String,
    command_log_open: bool,
    copied_command: Option<u64>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
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
            selected_remote: String::new(),
            selected_remote_branch: String::new(),
            selected_commit: None,
            compare_base: String::new(),
            compare_target: String::new(),
            command_log_open: false,
            copied_command: None,
        }
    }
}

pub fn render(ui: &mut egui::Ui, state: &mut UiState, snapshot: &AppSnapshot) -> Option<UiAction> {
    if state.active_repository != snapshot.active_repo {
        state.active_repository = snapshot.active_repo.clone();
        state.selected_file = None;
        state.requested_diff = None;
        state.selected_commit = None;
    }
    let available = snapshot.git_checked && snapshot.git_version.is_some();
    let can_act = available && !snapshot.busy;
    let mut action = None;

    ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
    ui.visuals_mut().widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    ui.visuals_mut().widgets.hovered.corner_radius = egui::CornerRadius::same(6);

    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(egui::Margin::symmetric(18, 14))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("Gitzi").strong().size(24.0));
                ui.add_space(10.0);
                if snapshot.busy {
                    ui.spinner();
                    ui.label("Trabajando…");
                } else if !snapshot.git_checked {
                    ui.spinner();
                    ui.label("Detectando Git…");
                } else if let Some(version) = &snapshot.git_version {
                    ui.label(
                        egui::RichText::new(format!("Git detectado · {version}"))
                            .color(ui.visuals().weak_text_color()),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("Git no está disponible")
                            .color(ui.visuals().error_fg_color),
                    );
                }
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("Ruta del repositorio");
                let field_width = (ui.available_width() - 72.0).max(140.0);
                let field = ui.add(
                    egui::TextEdit::singleline(&mut state.repository_path)
                        .hint_text("Selecciona o escribe una ruta")
                        .desired_width(field_width)
                        .id_source("repository_path"),
                );
                field.on_hover_text("Ruta local del repositorio Git");
                let open = ui.add_enabled(
                    can_act && !state.repository_path.trim().is_empty(),
                    egui::Button::new("Abrir"),
                );
                if open.clicked() {
                    action = Some(UiAction::OpenRepository(PathBuf::from(
                        &state.repository_path,
                    )));
                }
            });
            ui.horizontal(|ui| {
                let initialize = ui
                    .add_enabled(
                        can_act && !state.repository_path.trim().is_empty(),
                        egui::Button::new("Inicializar"),
                    )
                    .on_hover_text("Crear un repositorio Git en esta ruta");
                if initialize.clicked() && action.is_none() {
                    action = Some(UiAction::InitializeRepository(PathBuf::from(
                        &state.repository_path,
                    )));
                }
            });
            ui.separator();
            ui.label(egui::RichText::new("Clonar repositorio").strong());
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.clone_source)
                        .hint_text("URL o ruta de origen")
                        .desired_width((ui.available_width() * 0.55).max(140.0)),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut state.clone_destination)
                        .hint_text("Destino local")
                        .desired_width((ui.available_width() - 72.0).max(140.0)),
                );
                let clone = ui.add_enabled(
                    can_act
                        && !state.clone_source.trim().is_empty()
                        && !state.clone_destination.trim().is_empty(),
                    egui::Button::new("Clonar"),
                );
                if clone.clicked() && action.is_none() {
                    action = Some(UiAction::CloneRepository {
                        source: state.clone_source.clone(),
                        destination: PathBuf::from(&state.clone_destination),
                    });
                }
            });
        });

    if !snapshot.recent_repositories.is_empty() {
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Recientes").strong());
            for path in &snapshot.recent_repositories {
                let label = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| path.display().to_string());
                let response = ui.add_enabled(can_act, egui::Button::new(label).frame(false));
                response.clone().on_hover_text(path.display().to_string());
                if response.clicked() && action.is_none() {
                    action = Some(UiAction::OpenRepository(path.clone()));
                }
            }
        });
    }

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.heading("Repositorio");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let refresh = ui.add_enabled(
                can_act && snapshot.active_repo.is_some(),
                egui::Button::new("Actualizar"),
            );
            if refresh.clicked() && action.is_none() {
                action = Some(UiAction::Refresh);
            }
        });
    });

    if let Some(message) = &snapshot.error_message {
        egui::Frame::new()
            .fill(ui.visuals().error_fg_color.gamma_multiply(0.10))
            .inner_margin(egui::Margin::symmetric(12, 9))
            .corner_radius(egui::CornerRadius::same(6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Error")
                            .strong()
                            .color(ui.visuals().error_fg_color),
                    );
                    ui.label(message);
                });
            });
        ui.add_space(8.0);
    }

    match (&snapshot.active_repo, &snapshot.repository) {
        (Some(path), Some(repository)) => {
            if state
                .selected_file
                .as_ref()
                .is_none_or(|selected| !repository.files.iter().any(|file| &file.path == selected))
            {
                state.selected_file = repository.files.first().map(|file| file.path.clone());
            }
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(
                            path.file_name()
                                .and_then(|name| name.to_str())
                                .filter(|name| !name.is_empty())
                                .unwrap_or("Repositorio"),
                        )
                        .strong()
                        .size(18.0),
                    );
                    ui.label(egui::RichText::new(path.display().to_string()).weak());
                });
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Rama").weak());
                    ui.label(
                        egui::RichText::new(if repository.branch.is_empty() {
                            "Sin rama"
                        } else {
                            &repository.branch
                        })
                        .strong(),
                    );
                    if let Some(upstream) = &repository.upstream {
                        ui.separator();
                        ui.label(egui::RichText::new("Upstream").weak());
                        ui.label(upstream);
                        ui.separator();
                        ui.label(format!("↑ {} · ↓ {}", repository.ahead, repository.behind));
                    }
                    if repository.has_head == Some(false) {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(
                                "Repositorio sin commits · primer commit disponible",
                            )
                            .color(ui.visuals().warn_fg_color),
                        );
                    }
                });
            });
            ui.add_space(8.0);
            render_collaboration(ui, state, snapshot, can_act, &mut action);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Archivos").strong());
                ui.label(egui::RichText::new(repository.files.len().to_string()).weak());
            });
            if repository.files.is_empty() {
                empty_state(
                    ui,
                    "Árbol de trabajo limpio",
                    "No hay archivos modificados.",
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("repository_files")
                    .max_height(250.0)
                    .show(ui, |ui| {
                        egui::Grid::new("file_status_grid")
                            .striped(true)
                            .num_columns(4)
                            .spacing(egui::vec2(14.0, 5.0))
                            .show(ui, |ui| {
                                for file in &repository.files {
                                    let selected = state.selected_file.as_ref() == Some(&file.path);
                                    let select = ui.selectable_label(
                                        selected,
                                        file.path.display().to_string(),
                                    );
                                    if select.clicked() {
                                        state.selected_file = Some(file.path.clone());
                                        state.selected_area = preferred_area(file);
                                        state.requested_diff = None;
                                        if action.is_none() {
                                            let area = state.selected_area;
                                            action = Some(request_diff(
                                                state,
                                                snapshot,
                                                file.path.clone(),
                                                area,
                                            ));
                                        }
                                    }
                                    let status =
                                        format!("{}{}", file.index_status, file.worktree_status);
                                    ui.monospace(egui::RichText::new(status).strong());
                                    if ui
                                        .add_enabled(
                                            can_act && has_unstaged(file),
                                            egui::Button::new("Preparar"),
                                        )
                                        .clicked()
                                        && action.is_none()
                                    {
                                        action = Some(UiAction::StageFile(file.path.clone()));
                                    }
                                    if ui
                                        .add_enabled(
                                            can_act
                                                && has_staged(file)
                                                && repository.has_head.is_some(),
                                            egui::Button::new("Deshacer preparación"),
                                        )
                                        .clicked()
                                        && action.is_none()
                                    {
                                        action = Some(UiAction::UnstageFile(file.path.clone()));
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                ui.add_space(8.0);
                render_file_review(ui, state, snapshot, &repository.files, can_act, &mut action);
                ui.add_space(10.0);
                render_commit(ui, state, snapshot, can_act, &mut action);
            }
        }
        (Some(path), None) => {
            ui.label(egui::RichText::new(path.display().to_string()).weak());
            if snapshot.busy {
                loading_state(ui, "Consultando repositorio…");
            } else {
                empty_state(
                    ui,
                    "Sin datos del repositorio",
                    "Actualiza para consultar el estado.",
                );
            }
        }
        (None, _) => {
            if snapshot.busy {
                loading_state(ui, "Preparando Git…");
            } else if !snapshot.git_checked {
                loading_state(ui, "Detectando Git…");
            } else if snapshot.git_version.is_none() {
                empty_state(
                    ui,
                    "Git no está disponible",
                    "Instala Git y vuelve a abrir Gitzi.",
                );
            } else {
                empty_state(
                    ui,
                    "Abre un repositorio",
                    "Escribe una ruta o elige un repositorio reciente.",
                );
            }
        }
    }

    ui.add_space(12.0);
    let log_response = egui::CollapsingHeader::new(
        egui::RichText::new(format!(
            "Registro de comandos ({})",
            snapshot.command_log.len()
        ))
        .strong(),
    )
    .id_salt("command_log")
    .default_open(state.command_log_open)
    .show(ui, |ui| {
        if snapshot.command_log.is_empty() {
            ui.label(egui::RichText::new("Aún no hay comandos registrados.").weak());
        } else {
            egui::ScrollArea::vertical()
                .id_salt("command_log_scroll")
                .max_height(220.0)
                .show(ui, |ui| {
                    for entry in snapshot.command_log.iter().rev() {
                        command_entry(ui, state, entry);
                        ui.add_space(6.0);
                    }
                });
        }
    });
    state.command_log_open = log_response.body_returned.is_some();
    action
}

fn render_collaboration(
    ui: &mut egui::Ui,
    state: &mut UiState,
    snapshot: &AppSnapshot,
    can_act: bool,
    action: &mut Option<UiAction>,
) {
    let Some(repository) = &snapshot.repository else {
        return;
    };
    let has_upstream = repository.upstream.is_some();
    let has_current_local_branch = snapshot.branches.iter().any(|branch| {
        branch.kind == BranchKind::Local && branch.current && branch.name == repository.branch
    });
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Colaboración");
        ui.horizontal_wrapped(|ui| {
            let fetch = ui.add_enabled(
                can_act && !snapshot.remotes.is_empty(),
                egui::Button::new("Fetch"),
            );
            if fetch.clicked() && action.is_none() {
                *action = Some(UiAction::Fetch);
            }
            if snapshot.remotes.is_empty() {
                ui.label(egui::RichText::new("Configura un remoto para poder hacer fetch.").weak());
            }
            let pull = ui.add_enabled(
                can_act && has_upstream && has_current_local_branch,
                egui::Button::new("Pull · avance rápido"),
            );
            if pull.clicked() && action.is_none() {
                *action = Some(UiAction::Pull);
            }
            let push = ui.add_enabled(
                can_act && has_upstream && has_current_local_branch,
                egui::Button::new("Push"),
            );
            if push.clicked() && action.is_none() {
                *action = Some(UiAction::Push);
            }
            if !has_upstream {
                ui.label(egui::RichText::new("Pull y Push requieren upstream. Publica esta rama o configura uno existente abajo.").weak());
            }
            if !has_current_local_branch {
                ui.label(egui::RichText::new("Pull y Push requieren una rama local actual; no están disponibles en HEAD separado o sin commits.").weak());
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Adelante/atrás respecto a referencias locales conocidas:");
            ui.label(format!("↑ {} · ↓ {}", repository.ahead, repository.behind));
            ui.label(egui::RichText::new("No es una consulta remota en tiempo real.").weak());
            ui.separator();
            ui.label("Último fetch conocido:");
            ui.label(repository.last_fetch_at.map_or_else(|| "sin dato".to_owned(), format_known_time));
        });

        ui.collapsing("Ramas y upstream", |ui| {
            ui.label(egui::RichText::new("Ramas conocidas localmente").strong());
            if snapshot.branches.is_empty() {
                if snapshot.busy { loading_state(ui, "Cargando ramas…"); }
                else { ui.label(egui::RichText::new("No hay ramas disponibles.").weak()); }
            } else {
                egui::ScrollArea::vertical().id_salt("branches_list").max_height(110.0).show(ui, |ui| {
                    for branch in &snapshot.branches {
                        ui.horizontal_wrapped(|ui| {
                            let kind = match branch.kind { BranchKind::Local => "Local", BranchKind::Remote => "Remota" };
                            ui.label(format!("{kind} · {}{}", branch.name, if branch.current { " · actual" } else { "" }));
                            if let Some(upstream) = &branch.upstream { ui.label(egui::RichText::new(format!("→ {upstream}")).weak()); }
                            if branch.kind == BranchKind::Local && !branch.current {
                                let switch = ui.add_enabled(can_act && action.is_none(), egui::Button::new("Cambiar"));
                                if switch.clicked() && action.is_none() { *action = Some(UiAction::SwitchBranch(branch.name.clone())); }
                            }
                        });
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut state.branch_name).hint_text("Nombre de nueva rama").desired_width(190.0));
                let create = ui.add_enabled(can_act && action.is_none() && !state.branch_name.trim().is_empty() && repository.has_head == Some(true), egui::Button::new("Crear y cambiar"));
                if create.clicked() && action.is_none() { *action = Some(UiAction::CreateBranch(state.branch_name.trim().to_owned())); }
            });
            if repository.has_head == Some(false) {
                ui.label(egui::RichText::new("Crea el primer commit antes de crear otra rama.").weak());
            }

            let remotes: Vec<String> = snapshot.remotes.iter().map(|remote| remote.name.clone()).collect();
            let remote_branches: Vec<&BranchInfo> = snapshot.branches.iter().filter(|branch| branch.kind == BranchKind::Remote).collect();
            if !remotes.is_empty() {
                if !remotes.iter().any(|name| name == &state.selected_remote) { state.selected_remote.clone_from(&remotes[0]); }
                egui::ComboBox::from_label("Remoto para publicar").selected_text(&state.selected_remote).show_ui(ui, |ui| {
                    for remote in &remotes { ui.selectable_value(&mut state.selected_remote, remote.clone(), remote); }
                });
                let publish = ui.add_enabled(can_act && action.is_none() && has_current_local_branch && repository.has_head == Some(true) && !has_upstream, egui::Button::new("Publicar rama actual y configurar upstream"));
                if publish.clicked() && action.is_none() { *action = Some(UiAction::PublishBranch(state.selected_remote.clone())); }
            } else { ui.label(egui::RichText::new("No hay remotos configurados; no se puede publicar.").weak()); }

            let local_branches: Vec<&BranchInfo> = snapshot.branches.iter().filter(|branch| branch.kind == BranchKind::Local).collect();
            let current_local = local_branches.iter().find(|branch| branch.current).map(|branch| branch.name.as_str()).unwrap_or(&repository.branch);
            if !local_branches.is_empty() && !remote_branches.is_empty() {
                if !local_branches.iter().any(|branch| branch.name == state.selected_local_branch) { state.selected_local_branch = current_local.to_owned(); }
                if !remote_branches.iter().any(|branch| branch.name == state.selected_remote_branch) { state.selected_remote_branch = remote_branches[0].name.clone(); }
                ui.horizontal_wrapped(|ui| {
                    egui::ComboBox::from_label("Rama local").selected_text(&state.selected_local_branch).show_ui(ui, |ui| {
                        for branch in &local_branches { ui.selectable_value(&mut state.selected_local_branch, branch.name.clone(), &branch.name); }
                    });
                    egui::ComboBox::from_label("Upstream remoto existente").selected_text(&state.selected_remote_branch).show_ui(ui, |ui| {
                        for branch in &remote_branches { ui.selectable_value(&mut state.selected_remote_branch, branch.name.clone(), &branch.name); }
                    });
                    let set = ui.add_enabled(can_act && action.is_none(), egui::Button::new("Configurar upstream"));
                    if set.clicked() && action.is_none() { *action = Some(UiAction::SetUpstream { local_branch: state.selected_local_branch.clone(), remote_branch: state.selected_remote_branch.clone() }); }
                });
            } else { ui.label(egui::RichText::new("Para asociar un upstream existente, primero debe haber una rama local y una remota conocidas.").weak()); }
        });

        ui.collapsing("Historial y comparación", |ui| {
            if snapshot.history.is_empty() {
                if snapshot.busy { loading_state(ui, "Cargando historial…"); }
                else { ui.label(egui::RichText::new("No hay commits para mostrar.").weak()); }
            }
            egui::ScrollArea::vertical().id_salt("history_list").max_height(180.0).show(ui, |ui| {
                for commit in &snapshot.history {
                    ui.horizontal_wrapped(|ui| {
                        let selected = state.selected_commit.as_deref() == Some(&commit.id);
                        let response = ui.selectable_label(selected, format!("{} · {}", short_id(&commit.id), commit.subject));
                        if response.clicked() && action.is_none() && can_act {
                            state.selected_commit = Some(commit.id.clone());
                            *action = Some(UiAction::LoadCommitDetail(commit.id.clone()));
                        }
                        ui.label(egui::RichText::new(format!("{} · {}", commit.author, format_known_time(commit.authored_at))).weak());
                    });
                }
            });
            if snapshot.history_has_more {
                let more = ui.add_enabled(can_act && action.is_none(), egui::Button::new("Cargar más commits"));
                if more.clicked() && action.is_none() { *action = Some(UiAction::LoadMoreHistory); }
            }
            if let Some(detail) = &snapshot.commit_detail {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(egui::RichText::new(format!("Commit {} · {}", short_id(&detail.summary.id), detail.summary.subject)).strong());
                    ui.label(format!("Autor: {} · {}", detail.summary.author, format_known_time(detail.summary.authored_at)));
                    ui.monospace(&detail.message);
                    ui.label(format!("Archivos modificados ({})", detail.changed_files.len()));
                    for file in &detail.changed_files { ui.label(format!("{}  {}", file.status, file.path.display())); }
                });
            } else if state.selected_commit.is_some() && snapshot.busy { loading_state(ui, "Cargando detalle del commit…"); }

            let refs: Vec<String> = snapshot.branches.iter().map(|branch| branch.name.clone()).collect();
            if refs.is_empty() { ui.label(egui::RichText::new("Cargar ramas para comparar referencias.").weak()); }
            else {
                if !refs.iter().any(|name| name == &state.compare_base) { state.compare_base.clone_from(&refs[0]); }
                if !refs.iter().any(|name| name == &state.compare_target) { state.compare_target = refs.get(1).unwrap_or(&refs[0]).clone(); }
                ui.horizontal_wrapped(|ui| {
                    egui::ComboBox::from_label("Base").selected_text(&state.compare_base).show_ui(ui, |ui| {
                        for reference in &refs { ui.selectable_value(&mut state.compare_base, reference.clone(), reference); }
                    });
                    egui::ComboBox::from_label("Objetivo").selected_text(&state.compare_target).show_ui(ui, |ui| {
                        for reference in &refs { ui.selectable_value(&mut state.compare_target, reference.clone(), reference); }
                    });
                    let compare = ui.add_enabled(can_act && action.is_none() && state.compare_base != state.compare_target, egui::Button::new("Comparar"));
                    if compare.clicked() && action.is_none() { *action = Some(UiAction::CompareReferences { base_ref: state.compare_base.clone(), target_ref: state.compare_target.clone() }); }
                });
                if let Some(comparison) = snapshot.ref_comparison.as_ref().filter(|comparison| {
                    comparison.base_ref == state.compare_base
                        && comparison.target_ref == state.compare_target
                }) {
                    ui.label(format!("{} → {} · ↑ {} commits · ↓ {} commits", comparison.base_ref, comparison.target_ref, comparison.commits_ahead, comparison.commits_behind));
                    if comparison.truncated { ui.label(egui::RichText::new("Diff truncado; el contenido está incompleto.").color(ui.visuals().warn_fg_color)); }
                    egui::ScrollArea::vertical().id_salt("ref_comparison").max_height(230.0).show(ui, |ui| ui.monospace(&comparison.diff));
                } else {
                    ui.label(egui::RichText::new("Pulsa Comparar para cargar el resultado de estas referencias.").weak());
                }
            }
        });
    });
}

fn short_id(id: &str) -> &str {
    id.get(..id.len().min(12)).unwrap_or(id)
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
                egui::ScrollArea::vertical()
                    .id_salt(("file_diff", path.clone(), state.selected_area as u8))
                    .max_height(320.0)
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
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new("Crear commit").strong());
        ui.add(
            egui::TextEdit::multiline(&mut state.commit_message)
                .hint_text("Mensaje del commit")
                .desired_rows(2),
        );
        let commit = ui.add_enabled(
            can_act && has_staged && !has_conflicts && !state.commit_message.trim().is_empty(),
            egui::Button::new("Crear commit"),
        );
        if commit.clicked() && action.is_none() {
            *action = Some(UiAction::Commit(state.commit_message.clone()));
        }
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

fn command_entry(ui: &mut egui::Ui, state: &mut UiState, entry: &CommandEntry) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(match entry.status {
                    CommandStatus::Succeeded => "Correcto",
                    CommandStatus::SucceededWithDifferences => "Correcto · con diferencias",
                    CommandStatus::Failed => "Falló",
                })
                .color(match entry.status {
                    CommandStatus::Succeeded | CommandStatus::SucceededWithDifferences => {
                        ui.visuals().strong_text_color()
                    }
                    CommandStatus::Failed => ui.visuals().error_fg_color,
                })
                .strong(),
            );
            if let Some(code) = entry.exit_code {
                ui.label(format!("Código {code}"));
            }
            ui.label(format!(
                "{} · {} ms",
                format_start_time(entry.started_at),
                entry.duration.as_millis()
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let copied = state.copied_command == Some(entry.id);
                if ui
                    .button(if copied { "Copiado" } else { "Copiar" })
                    .clicked()
                {
                    ui.ctx().copy_text(entry.command_display.clone());
                    state.copied_command = Some(entry.id);
                }
            });
        });
        ui.monospace(egui::RichText::new(&entry.command_display).size(12.0));
        ui.label(format!(
            "cwd: {}",
            entry
                .cwd
                .as_ref()
                .map_or_else(|| "—".to_owned(), |path| path.display().to_string())
        ));
        if !entry.stdout.is_empty() {
            ui.collapsing("Salida", |ui| {
                ui.monospace(&entry.stdout);
            });
        }
        if !entry.stderr.is_empty() {
            ui.collapsing("Error", |ui| {
                ui.monospace(egui::RichText::new(&entry.stderr).color(ui.visuals().error_fg_color));
            });
        }
    });
}

fn format_start_time(time: SystemTime) -> String {
    let seconds_since_epoch = time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let seconds_of_day = seconds_since_epoch % 86_400;
    format!(
        "{:02}:{:02}:{:02} UTC",
        seconds_of_day / 3_600,
        (seconds_of_day / 60) % 60,
        seconds_of_day % 60
    )
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
