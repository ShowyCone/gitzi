use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_RECENT_REPOSITORIES: usize = 12;

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

#[cfg(test)]
mod tests {
    use super::unique_paths;
    use std::path::PathBuf;

    #[test]
    fn unique_paths_preserves_first_seen_order_and_limits_results() {
        let paths = ["one", "two", "one"].into_iter().map(PathBuf::from);
        assert_eq!(
            unique_paths(paths),
            [PathBuf::from("one"), PathBuf::from("two")]
        );
    }
}
