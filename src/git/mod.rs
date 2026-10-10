//! Serialized Git queries and mutations used by the application.

mod parsers;
mod runner;

use std::{
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread,
};

pub use runner::{GitError, GitOperation, GitRequest, GitResponse, GitResult};

const REQUEST_QUEUE_CAPACITY: usize = 32;

/// A single background worker. Submissions never wait for queue capacity.
pub struct GitWorker {
    requests: SyncSender<GitRequest>,
    responses: Receiver<GitResponse>,
}

impl GitWorker {
    pub fn spawn() -> GitWorker {
        let (request_tx, request_rx) = mpsc::sync_channel(REQUEST_QUEUE_CAPACITY);
        let (response_tx, response_rx) = mpsc::channel();
        thread::Builder::new()
            .name("gitzi-git-worker".into())
            .spawn(move || {
                while let Ok(request) = request_rx.recv() {
                    let response = runner::handle_request(request);
                    if response_tx.send(response).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to start Git worker thread");
        GitWorker {
            requests: request_tx,
            responses: response_rx,
        }
    }

    pub fn submit(&self, request: GitRequest) -> Result<(), GitError> {
        self.requests
            .try_send(request)
            .map_err(|error| match error {
                TrySendError::Full(_) => GitError {
                    message: "La cola de Git está llena; inténtalo de nuevo.".into(),
                },
                TrySendError::Disconnected(_) => GitError {
                    message: "El trabajador de Git ya no está disponible.".into(),
                },
            })
    }

    pub fn try_recv(&self) -> Option<GitResponse> {
        self.responses.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CommandStatus, DiffArea, FileStatus};
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    };

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(test_name: &str) -> Self {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("gitzi-test-repositories")
                .join(format!(
                    "{}-{test_name}-{}",
                    std::process::id(),
                    NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
                ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn temp_repo() -> TestDirectory {
        let directory = TestDirectory::new("repo-con espacio-ñ");
        let path = directory.path();
        let init = Command::new("git")
            .args(["init", "-q"])
            .current_dir(path)
            .output()
            .unwrap();
        assert!(
            init.status.success(),
            "{}",
            String::from_utf8_lossy(&init.stderr)
        );
        let config = Command::new("git")
            .args(["config", "user.name", "Gitzi Test"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(config.success());
        let config = Command::new("git")
            .args(["config", "user.email", "gitzi-test@example.invalid"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(config.success());
        directory
    }

    fn response(worker: &GitWorker, id: u64, operation: GitOperation) -> GitResponse {
        worker
            .submit(GitRequest {
                id,
                generation: 1,
                operation,
            })
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(response) = worker.try_recv() {
                return response;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "worker timed out"
            );
            thread::yield_now();
        }
    }

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn configure_and_commit(root: &Path, filename: &str, body: &str) {
        git(root, &["config", "user.name", "Gitzi Test"]);
        git(
            root,
            &["config", "user.email", "gitzi-test@example.invalid"],
        );
        fs::write(root.join(filename), body).unwrap();
        git(root, &["add", "--", filename]);
        git(root, &["commit", "-m", "initial"]);
    }

    #[test]
    fn detects_git_and_records_the_real_invocation() {
        let worker = GitWorker::spawn();
        worker
            .submit(GitRequest {
                id: 10,
                generation: 4,
                operation: GitOperation::DetectGit,
            })
            .unwrap();
        let start = Instant::now();
        let response = loop {
            if let Some(response) = worker.try_recv() {
                break response;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "worker timed out"
            );
            thread::yield_now();
        };
        assert_eq!((response.id, response.generation), (10, 4));
        assert!(matches!(response.result, Ok(GitResult::GitDetected { .. })));
        assert_eq!(response.commands.len(), 1);
        let entry = &response.commands[0];
        assert_eq!(entry.command_display, "git --version");
        assert_eq!(
            entry.cwd.as_deref(),
            std::env::current_dir().ok().as_deref()
        );
        assert_eq!(entry.status, CommandStatus::Succeeded);
        assert!(entry.stdout.contains("git version"));
    }

    #[test]
    fn inspects_real_repository_and_preserves_status_and_paths() {
        let root_directory = temp_repo();
        let root = root_directory.path();
        let staged = root.join("staged ñ.txt");
        fs::write(&staged, "staged\n").unwrap();
        assert!(
            Command::new("git")
                .args(["add", "--", "staged ñ.txt"])
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
        fs::write(&staged, "staged and modified\n").unwrap();
        let untracked = root.join("new file.txt");
        fs::write(&untracked, "new\n").unwrap();
        let index_path = root.join(".git").join("index");
        let index_before = fs::read(&index_path).unwrap();

        let worker = GitWorker::spawn();
        worker
            .submit(GitRequest {
                id: 11,
                generation: 9,
                operation: GitOperation::InspectRepository(root.to_path_buf()),
            })
            .unwrap();
        let start = Instant::now();
        let response = loop {
            if let Some(response) = worker.try_recv() {
                break response;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "worker timed out"
            );
            thread::yield_now();
        };
        let status = match response.result.unwrap() {
            GitResult::RepositoryInspected { path, status } => {
                assert_eq!(path.as_path(), root);
                status
            }
            other => panic!("unexpected result: {other:?}"),
        };
        let branch = Command::new("git")
            .args(["symbolic-ref", "--short", "HEAD"])
            .current_dir(root)
            .output()
            .unwrap();
        assert_eq!(
            status.branch,
            String::from_utf8_lossy(&branch.stdout).trim().to_owned()
        );
        assert!(status.upstream.is_none());
        assert!(status.files.contains(&FileStatus {
            path: PathBuf::from("staged ñ.txt"),
            index_status: 'A',
            worktree_status: 'M',
            unmerged: false,
        }));
        assert!(status.files.contains(&FileStatus {
            path: PathBuf::from("new file.txt"),
            index_status: '?',
            worktree_status: '?',
            unmerged: false,
        }));
        assert_eq!(
            fs::read(&index_path).unwrap(),
            index_before,
            "status query changed the repository index"
        );
        let entry = &response.commands[0];
        assert_eq!(entry.cwd.as_deref(), Some(root));
        assert!(
            entry
                .command_display
                .contains("--no-optional-locks status --porcelain=v2")
        );
        assert_eq!(entry.status, CommandStatus::Succeeded);
    }

    #[test]
    fn invalid_repository_returns_actionable_error_and_record() {
        let root_directory = TestDirectory::new("not-a-repo");
        let root = root_directory.path().join("not-a-directory");
        fs::write(&root, "temporary file used as an invalid cwd").unwrap();
        let worker = GitWorker::spawn();
        worker
            .submit(GitRequest {
                id: 12,
                generation: 1,
                operation: GitOperation::InspectRepository(root.to_path_buf()),
            })
            .unwrap();
        let start = Instant::now();
        let response = loop {
            if let Some(response) = worker.try_recv() {
                break response;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "worker timed out"
            );
            thread::yield_now();
        };
        let error = response.result.unwrap_err();
        assert_eq!(response.commands[0].cwd.as_deref(), Some(root.as_path()));
        assert_eq!(response.commands[0].status, CommandStatus::Failed);
        assert!(response.commands[0].exit_code.is_none_or(|code| code != 0));
        if !response.commands[0].stderr.trim().is_empty() {
            assert!(error.message.contains(response.commands[0].stderr.trim()));
        } else {
            assert!(error.message.contains("Código de salida"));
        }
    }

    #[test]
    fn initializes_and_clones_repositories() {
        let base = TestDirectory::new("init-clone");
        let initialized = base.path().join("repo inicial");
        let worker = GitWorker::spawn();
        let init = response(
            &worker,
            20,
            GitOperation::InitializeRepository {
                path: initialized.clone(),
            },
        );
        assert!(matches!(
            init.result,
            Ok(GitResult::RepositoryInitialized { .. })
        ));
        assert!(init.commands[0].command_display.contains("init --"));
        let inspected = response(
            &worker,
            21,
            GitOperation::InspectRepository(initialized.clone()),
        );
        match inspected.result.unwrap() {
            GitResult::RepositoryInspected { status, .. } => {
                assert_eq!(status.has_head, Some(false))
            }
            other => panic!("unexpected result: {other:?}"),
        }

        configure_and_commit(initialized.as_path(), "hello.txt", "hello\n");
        let cloned = base.path().join("cloned");
        let cloned_response = response(
            &worker,
            22,
            GitOperation::CloneRepository {
                source: initialized.to_string_lossy().into_owned(),
                destination: cloned.clone(),
            },
        );
        assert!(matches!(
            cloned_response.result,
            Ok(GitResult::RepositoryCloned { .. })
        ));
        let inspected = response(&worker, 23, GitOperation::InspectRepository(cloned));
        match inspected.result.unwrap() {
            GitResult::RepositoryInspected { status, .. } => {
                assert_eq!(status.has_head, Some(true))
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn reads_untracked_binary_and_separate_index_and_worktree_diffs() {
        let directory = temp_repo();
        let root = directory.path();
        configure_and_commit(root, "tracked file.txt", "base\n");
        let tracked = PathBuf::from("tracked file.txt");
        fs::write(root.join(&tracked), "staged change\n").unwrap();
        let worker = GitWorker::spawn();
        let staged = response(
            &worker,
            30,
            GitOperation::StageFile {
                repository: root.to_path_buf(),
                path: tracked.clone(),
            },
        );
        assert!(staged.result.is_ok());
        fs::write(root.join(&tracked), "worktree change\n").unwrap();
        let staged_diff = response(
            &worker,
            31,
            GitOperation::ReadDiff {
                repository: root.to_path_buf(),
                path: tracked.clone(),
                area: DiffArea::Staged,
                untracked: false,
            },
        );
        let unstaged_diff = response(
            &worker,
            32,
            GitOperation::ReadDiff {
                repository: root.to_path_buf(),
                path: tracked,
                area: DiffArea::Unstaged,
                untracked: false,
            },
        );
        let get_text = |r: GitResponse| match r.result.unwrap() {
            GitResult::DiffLoaded { diff } => diff.text,
            other => panic!("unexpected result: {other:?}"),
        };
        assert!(get_text(staged_diff).contains("staged change"));
        assert!(get_text(unstaged_diff).contains("worktree change"));

        fs::write(root.join("binary.dat"), [0, 1, 2, 255]).unwrap();
        let binary = response(
            &worker,
            33,
            GitOperation::ReadDiff {
                repository: root.to_path_buf(),
                path: "binary.dat".into(),
                area: DiffArea::Unstaged,
                untracked: true,
            },
        );
        match binary.result.unwrap() {
            GitResult::DiffLoaded { diff } => assert!(diff.binary),
            other => panic!("unexpected result: {other:?}"),
        }
        assert_eq!(binary.commands[0].exit_code, Some(1));
        assert_eq!(
            binary.commands[0].status,
            CommandStatus::SucceededWithDifferences
        );
    }

    #[test]
    fn unborn_unstage_preserves_file_and_commit_uses_index_only() {
        let directory = temp_repo();
        let root = directory.path();
        let path = PathBuf::from("archivo con espacios.txt");
        fs::write(root.join(&path), "first\n").unwrap();
        let worker = GitWorker::spawn();
        assert!(
            response(
                &worker,
                40,
                GitOperation::StageFile {
                    repository: root.to_path_buf(),
                    path: path.clone()
                }
            )
            .result
            .is_ok()
        );
        let unstage = response(
            &worker,
            41,
            GitOperation::UnstageFile {
                repository: root.to_path_buf(),
                path: path.clone(),
                has_head: false,
            },
        );
        assert!(unstage.result.is_ok());
        assert_eq!(fs::read_to_string(root.join(&path)).unwrap(), "first\n");

        configure_and_commit(root, "base.txt", "base\n");
        fs::write(root.join(&path), "staged version\n").unwrap();
        git(root, &["add", "--", "archivo con espacios.txt"]);
        fs::write(root.join(&path), "unstaged version\n").unwrap();
        let commit = response(
            &worker,
            42,
            GitOperation::Commit {
                repository: root.to_path_buf(),
                message: "index only\n".into(),
            },
        );
        assert!(matches!(commit.result, Ok(GitResult::CommitCreated { .. })));
        assert!(commit.commands[0].command_display.contains("--file=-"));
        assert!(!commit.commands[0].command_display.contains("index only"));
        assert_eq!(
            fs::read_to_string(root.join(&path)).unwrap(),
            "unstaged version\n"
        );
        let show = Command::new("git")
            .args(["show", "HEAD:archivo con espacios.txt"])
            .current_dir(root)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&show.stdout), "staged version\n");
        let status = response(
            &worker,
            43,
            GitOperation::InspectRepository(root.to_path_buf()),
        );
        match status.result.unwrap() {
            GitResult::RepositoryInspected { status, .. } => assert!(
                status
                    .files
                    .iter()
                    .any(|file| file.path == path && file.worktree_status == 'M')
            ),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn unstaging_a_staged_rename_preserves_the_worktree_file_and_edits() {
        let directory = temp_repo();
        let root = directory.path();
        configure_and_commit(root, "old name.txt", "one\ntwo\nthree\nfour\n");
        let old_path = PathBuf::from("old name.txt");
        let new_path = PathBuf::from("new name.txt");
        fs::rename(root.join(&old_path), root.join(&new_path)).unwrap();
        let edited = "one\ntwo modified\nthree\nfour\n";
        fs::write(root.join(&new_path), edited).unwrap();
        git(root, &["add", "-A", "--"]);

        let worker = GitWorker::spawn();
        let before = response(
            &worker,
            50,
            GitOperation::InspectRepository(root.to_path_buf()),
        );
        match before.result.unwrap() {
            GitResult::RepositoryInspected { status, .. } => assert!(
                status
                    .files
                    .iter()
                    .any(|file| file.path == new_path && file.index_status == 'R')
            ),
            other => panic!("unexpected result: {other:?}"),
        }
        let unstage = response(
            &worker,
            51,
            GitOperation::UnstageFile {
                repository: root.to_path_buf(),
                path: new_path.clone(),
                has_head: true,
            },
        );
        assert!(unstage.result.is_ok());
        assert!(root.join(&new_path).exists());
        assert!(!root.join(&old_path).exists());
        assert_eq!(fs::read_to_string(root.join(&new_path)).unwrap(), edited);

        let after = response(
            &worker,
            52,
            GitOperation::InspectRepository(root.to_path_buf()),
        );
        match after.result.unwrap() {
            GitResult::RepositoryInspected { status, .. } => {
                assert!(
                    status
                        .files
                        .iter()
                        .any(|file| file.path == old_path && file.index_status == 'D')
                );
                assert!(status.files.iter().any(|file| file.path == new_path
                    && file.index_status == '?'
                    && file.worktree_status == '?'));
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
