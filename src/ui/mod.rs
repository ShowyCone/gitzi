use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use eframe::egui;

use crate::domain::{AppSnapshot, CommandEntry, CommandStatus, UiAction};

#[derive(Default)]
pub struct UiState {
    repository_path: String,
    command_log_open: bool,
    copied_command: Option<u64>,
}

pub fn render(ui: &mut egui::Ui, state: &mut UiState, snapshot: &AppSnapshot) -> Option<UiAction> {
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
                });
            });
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
                            .num_columns(2)
                            .spacing(egui::vec2(14.0, 5.0))
                            .show(ui, |ui| {
                                for file in &repository.files {
                                    let status =
                                        format!("{}{}", file.index_status, file.worktree_status);
                                    ui.monospace(egui::RichText::new(status).strong());
                                    ui.label(file.path.display().to_string());
                                    ui.end_row();
                                }
                            });
                    });
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

fn loading_state(ui: &mut egui::Ui, text: &str) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(text);
        });
    });
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
                    CommandStatus::Failed => "Falló",
                })
                .color(match entry.status {
                    CommandStatus::Succeeded => ui.visuals().strong_text_color(),
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
