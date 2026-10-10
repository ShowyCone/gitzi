//! Asynchronous, read-only Git queries used by the application.

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
    use crate::domain::{CommandStatus, FileStatus};
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
        assert_eq!(entry.cwd, None);
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
            worktree_status: 'M'
        }));
        assert!(status.files.contains(&FileStatus {
            path: PathBuf::from("new file.txt"),
            index_status: '?',
            worktree_status: '?'
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
        let root = root_directory.path();
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
        assert_eq!(response.commands[0].cwd.as_deref(), Some(root));
        assert_eq!(response.commands[0].status, CommandStatus::Failed);
        assert!(response.commands[0].exit_code.is_some_and(|code| code != 0));
        if !response.commands[0].stderr.trim().is_empty() {
            assert!(error.message.contains(response.commands[0].stderr.trim()));
        } else {
            assert!(error.message.contains("Código de salida"));
        }
    }
}
