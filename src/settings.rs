use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::domain::PullPolicy;

const MAX_RECENT_REPOSITORIES: usize = 12;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Preferences {
    pub pull_policy: PullPolicy,
    pub external_editor: Option<PathBuf>,
}

pub fn load_recent_repositories() -> Vec<PathBuf> {
    let Some(file) = settings_file() else {
        return Vec::new();
    };
    let Ok(contents) = fs::read_to_string(file) else {
        return Vec::new();
    };

    unique_paths(contents.lines().map(PathBuf::from))
}

pub fn remember_repository(path: &Path, recent: &mut Vec<PathBuf>) {
    recent.retain(|item| item != path);
    recent.insert(0, path.to_path_buf());
    recent.truncate(MAX_RECENT_REPOSITORIES);
    save_recent_repositories(recent);
}

pub fn load_preferences() -> Preferences {
    let Some(file) = preferences_file() else {
        return Preferences::default();
    };
    let Ok(contents) = fs::read_to_string(file) else {
        return Preferences::default();
    };
    parse_preferences(&contents)
}

pub fn save_preferences(preferences: &Preferences) -> Result<(), String> {
    let Some(file) = preferences_file() else {
        return Err("No se encontró una carpeta de configuración para Gitzi.".into());
    };
    let Some(parent) = file.parent() else {
        return Err("No se pudo determinar la carpeta de configuración de Gitzi.".into());
    };
    let editor = match &preferences.external_editor {
        Some(path) => path
            .to_str()
            .ok_or_else(|| "La ruta del editor no se puede guardar en texto.".to_owned())?,
        None => "",
    };
    if editor.contains('\n') || editor.contains('\r') {
        return Err("La ruta del editor no puede contener saltos de línea.".into());
    }
    fs::create_dir_all(parent)
        .map_err(|error| format!("No se pudo crear la configuración de Gitzi: {error}"))?;
    let pull_policy = match preferences.pull_policy {
        PullPolicy::FastForwardOnly => "ff-only",
        PullPolicy::Merge => "merge",
        PullPolicy::Rebase => "rebase",
    };
    let contents = format!("version=1\npull_policy={pull_policy}\nexternal_editor={editor}\n");
    fs::write(file, contents)
        .map_err(|error| format!("No se pudieron guardar las preferencias de Gitzi: {error}"))
}

fn unique_paths(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if !path.as_os_str().is_empty() && !unique.contains(&path) {
            unique.push(path);
        }
        if unique.len() == MAX_RECENT_REPOSITORIES {
            break;
        }
    }
    unique
}

fn save_recent_repositories(recent: &[PathBuf]) {
    let Some(file) = settings_file() else {
        return;
    };
    let Some(parent) = file.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }

    let contents = recent
        .iter()
        .filter_map(|path| path.to_str())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = fs::write(file, contents);
}

fn settings_file() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let root = std::env::var_os("APPDATA").map(PathBuf::from)?;

    #[cfg(target_os = "linux")]
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let root = std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))?;

    Some(root.join("Gitzi").join("recent-repositories.txt"))
}

fn preferences_file() -> Option<PathBuf> {
    settings_file().map(|file| file.with_file_name("preferences.txt"))
}

fn parse_preferences(contents: &str) -> Preferences {
    let mut preferences = Preferences::default();
    let mut supported_version = false;
    let mut editor = None;
    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match (key, value) {
            ("version", "1") => supported_version = true,
            ("pull_policy", "merge") => preferences.pull_policy = PullPolicy::Merge,
            ("pull_policy", "rebase") => preferences.pull_policy = PullPolicy::Rebase,
            ("pull_policy", "ff-only") => {
                preferences.pull_policy = PullPolicy::FastForwardOnly;
            }
            ("external_editor", value) if !value.trim().is_empty() => {
                editor = Some(PathBuf::from(value));
            }
            _ => {}
        }
    }
    if supported_version {
        preferences.external_editor = editor;
        preferences
    } else {
        Preferences::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{Preferences, parse_preferences, unique_paths};
    use crate::domain::PullPolicy;
    use std::path::PathBuf;

    #[test]
    fn unique_paths_preserves_first_seen_order_and_limits_results() {
        let paths = ["one", "two", "one"].into_iter().map(PathBuf::from);
        assert_eq!(
            unique_paths(paths),
            [PathBuf::from("one"), PathBuf::from("two")]
        );
    }

    #[test]
    fn preferences_parse_known_values_and_ignore_unknown_versions() {
        let preferences = parse_preferences(
            "version=1\npull_policy=rebase\nexternal_editor=C:/Tools/editor.exe\n",
        );
        assert_eq!(preferences.pull_policy, PullPolicy::Rebase);
        assert_eq!(
            preferences.external_editor,
            Some(PathBuf::from("C:/Tools/editor.exe"))
        );
        assert_eq!(
            parse_preferences("version=2\npull_policy=merge\n"),
            Preferences::default()
        );
        assert_eq!(
            parse_preferences("version=1\npull_policy=unknown\n"),
            Preferences::default()
        );
    }
}
