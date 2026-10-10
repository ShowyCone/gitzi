use crate::domain::{
    BranchInfo, BranchKind, CommandEntry, CommandStatus, CommitDetail, CommitFileChange,
    CommitSummary, DiffArea, FileDiff, RefComparison, RemoteInfo, RepositoryStatus,
};
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MAX_CAPTURE_BYTES: usize = 1024 * 1024;
static LOG_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitOperation {
    DetectGit,
    InspectRepository(PathBuf),
    InitializeRepository {
        path: PathBuf,
    },
    CloneRepository {
        source: String,
        destination: PathBuf,
    },
    ReadDiff {
        repository: PathBuf,
        path: PathBuf,
        area: DiffArea,
        untracked: bool,
    },
    StageFile {
        repository: PathBuf,
        path: PathBuf,
    },
    UnstageFile {
        repository: PathBuf,
        path: PathBuf,
        has_head: bool,
    },
    Commit {
        repository: PathBuf,
        message: String,
    },
    ListBranches(PathBuf),
    CreateBranch {
        repository: PathBuf,
        name: String,
    },
    SwitchBranch {
        repository: PathBuf,
        name: String,
    },
    SetUpstream {
        repository: PathBuf,
        local_branch: String,
        remote_branch: String,
    },
    Fetch(PathBuf),
    PullFastForward(PathBuf),
    Push {
        repository: PathBuf,
        upstream: String,
        local_branch: String,
    },
    PublishBranch {
        repository: PathBuf,
        remote: String,
        local_branch: String,
    },
    LoadHistory {
        repository: PathBuf,
        offset: usize,
        limit: usize,
    },
    LoadCommitDetail {
        repository: PathBuf,
        commit_id: String,
    },
    CompareReferences {
        repository: PathBuf,
        base_ref: String,
        target_ref: String,
    },
}

#[derive(Clone, Debug)]
pub struct GitRequest {
    pub id: u64,
    pub generation: u64,
    pub operation: GitOperation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitResult {
    GitDetected {
        version: String,
    },
    RepositoryInspected {
        path: PathBuf,
        status: RepositoryStatus,
    },
    RepositoryInitialized {
        path: PathBuf,
    },
    RepositoryCloned {
        path: PathBuf,
    },
    DiffLoaded {
        diff: FileDiff,
    },
    FileStaged {
        path: PathBuf,
    },
    FileUnstaged {
        path: PathBuf,
    },
    CommitCreated {
        path: PathBuf,
    },
    BranchesListed {
        repository: PathBuf,
        branches: Vec<BranchInfo>,
        remotes: Vec<RemoteInfo>,
    },
    BranchCreated {
        repository: PathBuf,
    },
    BranchSwitched {
        repository: PathBuf,
    },
    UpstreamSet {
        repository: PathBuf,
    },
    Fetched {
        repository: PathBuf,
    },
    Pulled {
        repository: PathBuf,
    },
    Pushed {
        repository: PathBuf,
    },
    BranchPublished {
        repository: PathBuf,
    },
    HistoryLoaded {
        repository: PathBuf,
        offset: usize,
        commits: Vec<CommitSummary>,
        has_more: bool,
    },
    CommitLoaded {
        repository: PathBuf,
        detail: CommitDetail,
    },
    ReferencesCompared {
        repository: PathBuf,
        comparison: RefComparison,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitError {
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct GitResponse {
    pub id: u64,
    pub generation: u64,
    pub result: Result<GitResult, GitError>,
    pub commands: Vec<CommandEntry>,
}

pub(crate) fn handle_request(request: GitRequest) -> GitResponse {
    let (result, commands) = match request.operation {
        GitOperation::DetectGit => {
            let (entry, output) = run_git_from_current_dir(&[OsString::from("--version")]);
            let result = match output {
                Ok(output) if output.stdout.truncated => Err(GitError {
                    message: "La salida de git --version superó el límite de captura de 1 MiB."
                        .into(),
                }),
                Ok(output) if output.status.success() => Ok(GitResult::GitDetected {
                    version: String::from_utf8_lossy(&output.stdout.bytes)
                        .trim()
                        .to_owned(),
                }),
                Ok(output) => Err(command_error(
                    "No se pudo detectar Git",
                    &entry,
                    &output.stderr,
                )),
                Err(error) => Err(GitError {
                    message: format!(
                        "No se pudo ejecutar Git. Instala Git y comprueba que esté en PATH. {error}"
                    ),
                }),
            };
            (result, vec![entry])
        }
        GitOperation::InspectRepository(path) => {
            let args = [
                "--no-optional-locks",
                "status",
                "--porcelain=v2",
                "--branch",
                "-z",
                "--untracked-files=all",
            ]
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
            let (entry, output) = run_git(&path, &args);
            let mut result = match output {
                Ok(output) if output.stdout.truncated => Err(GitError {
                    message: "La salida de git status superó el límite de captura de 1 MiB; el estado está incompleto.".into(),
                }),
                Ok(output) if output.status.success() => match parse_status_output(&output.stdout) {
                    Ok(status) => Ok(GitResult::RepositoryInspected { path: path.clone(), status }),
                    Err(error) => Err(error),
                },
                Ok(output) => Err(command_error("No se pudo consultar el repositorio; comprueba que la ruta sea válida y que Git tenga acceso", &entry, &output.stderr)),
                Err(error) => Err(GitError { message: format!("No se pudo ejecutar Git para consultar {}. Comprueba que Git esté instalado y que la ruta sea accesible. {error}", path.display()) }),
            };
            let mut commands = vec![entry];
            if let Ok(GitResult::RepositoryInspected { status, .. }) = &mut result {
                status.last_fetch_at = fetch_timestamp(&path, &mut commands);
            }
            (result, commands)
        }
        GitOperation::InitializeRepository { path } => {
            let args = vec!["init".into(), "--".into(), path.as_os_str().to_owned()];
            let (entry, output) = run_git_from_current_dir(&args);
            let result =
                successful_mutation(output, &entry, "No se pudo inicializar el repositorio")
                    .map(|()| GitResult::RepositoryInitialized { path });
            (result, vec![entry])
        }
        GitOperation::CloneRepository {
            source,
            destination,
        } => {
            let args = vec![
                "clone".into(),
                "--".into(),
                source.into(),
                destination.as_os_str().to_owned(),
            ];
            let (entry, output) = run_git_from_current_dir(&args);
            let result = successful_mutation(output, &entry, "No se pudo clonar el repositorio")
                .map(|()| GitResult::RepositoryCloned { path: destination });
            (result, vec![entry])
        }
        GitOperation::ReadDiff {
            repository,
            path,
            area,
            untracked,
        } => {
            let mut args = vec![
                OsString::from("--no-optional-locks"),
                OsString::from("--literal-pathspecs"),
                OsString::from("diff"),
            ];
            if untracked {
                args.extend(
                    [
                        "--no-index",
                        "--no-ext-diff",
                        "--no-textconv",
                        "--no-color",
                        "--",
                        "/dev/null",
                    ]
                    .into_iter()
                    .map(OsString::from),
                );
                args.push(path.as_os_str().to_owned());
            } else {
                args.extend(
                    ["--no-ext-diff", "--no-textconv", "--no-color"]
                        .into_iter()
                        .map(OsString::from),
                );
                if area == DiffArea::Staged {
                    args.push("--cached".into());
                }
                args.push("--".into());
                args.push(path.as_os_str().to_owned());
            }
            let (mut entry, output) = run_git(&repository, &args);
            let result = match output {
                Ok(output)
                    if output.status.success()
                        || (untracked
                            && output.status.code() == Some(1)
                            && output.stderr.bytes.is_empty()) =>
                {
                    if untracked && output.status.code() == Some(1) {
                        entry.status = CommandStatus::SucceededWithDifferences;
                    }
                    let text = capture_text(&output.stdout);
                    let binary = text
                        .lines()
                        .any(|line| line.starts_with("Binary files ") && line.contains(" differ"))
                        || text.contains("GIT binary patch");
                    Ok(GitResult::DiffLoaded {
                        diff: FileDiff {
                            repository,
                            path,
                            area,
                            text,
                            binary,
                            truncated: output.stdout.truncated,
                        },
                    })
                }
                Ok(output) => Err(command_error(
                    "No se pudo leer el diff",
                    &entry,
                    &output.stderr,
                )),
                Err(error) => Err(GitError {
                    message: format!("No se pudo ejecutar Git para leer el diff. {error}"),
                }),
            };
            (result, vec![entry])
        }
        GitOperation::StageFile { repository, path } => {
            let args = vec![
                "--literal-pathspecs".into(),
                "add".into(),
                "--".into(),
                path.as_os_str().to_owned(),
            ];
            let (entry, output) = run_git(&repository, &args);
            let result = successful_mutation(output, &entry, "No se pudo preparar el archivo")
                .map(|()| GitResult::FileStaged { path });
            (result, vec![entry])
        }
        GitOperation::UnstageFile {
            repository,
            path,
            has_head,
        } => {
            let args = if has_head {
                vec![
                    "--literal-pathspecs".into(),
                    "restore".into(),
                    "--staged".into(),
                    "--".into(),
                    path.as_os_str().to_owned(),
                ]
            } else {
                vec![
                    "--literal-pathspecs".into(),
                    "rm".into(),
                    "--cached".into(),
                    "--".into(),
                    path.as_os_str().to_owned(),
                ]
            };
            let (entry, output) = run_git(&repository, &args);
            let result =
                successful_mutation(output, &entry, "No se pudo quitar el archivo del índice")
                    .map(|()| GitResult::FileUnstaged { path });
            (result, vec![entry])
        }
        GitOperation::Commit {
            repository,
            message,
        } => {
            let args = vec!["commit".into(), "--file=-".into()];
            let (entry, output) = run_git_with_input(&repository, &args, Some(message.as_bytes()));
            let result = successful_mutation(output, &entry, "No se pudo crear el commit")
                .map(|()| GitResult::CommitCreated { path: repository });
            (result, vec![entry])
        }
        operation @ (GitOperation::ListBranches(_)
        | GitOperation::CreateBranch { .. }
        | GitOperation::SwitchBranch { .. }
        | GitOperation::SetUpstream { .. }
        | GitOperation::Fetch(_)
        | GitOperation::PullFastForward(_)
        | GitOperation::Push { .. }
        | GitOperation::PublishBranch { .. }
        | GitOperation::LoadHistory { .. }
        | GitOperation::LoadCommitDetail { .. }
        | GitOperation::CompareReferences { .. }) => handle_collaboration(operation),
    };
    GitResponse {
        id: request.id,
        generation: request.generation,
        result,
        commands,
    }
}

fn handle_collaboration(
    operation: GitOperation,
) -> (Result<GitResult, GitError>, Vec<CommandEntry>) {
    let repository = match &operation {
        GitOperation::ListBranches(path)
        | GitOperation::Fetch(path)
        | GitOperation::PullFastForward(path)
        | GitOperation::Push {
            repository: path, ..
        } => path.clone(),
        GitOperation::CreateBranch { repository, .. }
        | GitOperation::SwitchBranch { repository, .. }
        | GitOperation::SetUpstream { repository, .. }
        | GitOperation::PublishBranch { repository, .. }
        | GitOperation::LoadHistory { repository, .. }
        | GitOperation::LoadCommitDetail { repository, .. }
        | GitOperation::CompareReferences { repository, .. } => repository.clone(),
        _ => unreachable!(),
    };
    let mut commands = Vec::new();
    let (result, command_result) = match operation {
        GitOperation::ListBranches(_) => {
            let args = strings(&[
                "for-each-ref",
                "--format=%(refname)%09%(upstream:short)",
                "refs/heads",
                "refs/remotes",
            ]);
            let (entry, output) = run_git(&repository, &args);
            commands.push(entry);
            let parsed = output.map_err(process_error).and_then(|out| {
                require_success(out, &commands[0], "No se pudieron listar las ramas")
                    .and_then(|out| parse_branches(&out.stdout.bytes))
            });
            let rem_args = strings(&["remote"]);
            let (remote_entry, remote_out) = run_git(&repository, &rem_args);
            commands.push(remote_entry);
            let parsed_remotes = remote_out.map_err(process_error).and_then(|out| {
                require_success(out, &commands[1], "No se pudieron listar los remotos")
                    .map(|out| parse_remotes(&out.stdout.bytes))
            });
            let current_args = strings(&["branch", "--show-current"]);
            let (current_entry, current_out) = run_git(&repository, &current_args);
            commands.push(current_entry);
            let current = current_out
                .ok()
                .filter(|out| out.status.success())
                .map(|out| String::from_utf8_lossy(&out.stdout.bytes).trim().to_owned());
            let result = parsed.and_then(|mut branches| {
                parsed_remotes.map(|remotes| {
                    for branch in &mut branches {
                        branch.current = branch.kind == BranchKind::Local
                            && current.as_deref() == Some(branch.name.as_str());
                    }
                    (branches, remotes)
                })
            });
            (
                result.map(|(branches, remotes)| GitResult::BranchesListed {
                    repository: repository.clone(),
                    branches,
                    remotes,
                }),
                Ok(()),
            )
        }
        GitOperation::CreateBranch { name, .. } if !name.starts_with('-') => one_mutation(
            &repository,
            strings(&["switch", "--create", &name]),
            "No se pudo crear la rama",
            GitResult::BranchCreated {
                repository: repository.clone(),
            },
            &mut commands,
        ),
        GitOperation::SwitchBranch { name, .. } if !name.starts_with('-') => one_mutation(
            &repository,
            strings(&["switch", "--", &name]),
            "No se pudo cambiar de rama",
            GitResult::BranchSwitched {
                repository: repository.clone(),
            },
            &mut commands,
        ),
        GitOperation::CreateBranch { .. } | GitOperation::SwitchBranch { .. } => (
            Err(GitError {
                message: "El nombre de rama no puede comenzar con guion.".into(),
            }),
            Ok(()),
        ),
        GitOperation::SetUpstream {
            local_branch,
            remote_branch,
            ..
        } => one_mutation(
            &repository,
            strings(&[
                "branch",
                &format!("--set-upstream-to={remote_branch}"),
                "--",
                &local_branch,
            ]),
            "No se pudo configurar el upstream",
            GitResult::UpstreamSet {
                repository: repository.clone(),
            },
            &mut commands,
        ),
        GitOperation::Fetch(_) => sync_operation(
            &repository,
            strings(&["fetch", "--all"]),
            "No se pudo consultar los remotos",
            GitResult::Fetched {
                repository: repository.clone(),
            },
            &mut commands,
        ),
        GitOperation::PullFastForward(_) => sync_operation(
            &repository,
            strings(&["pull", "--ff-only"]),
            "No se pudo integrar los cambios mediante avance rápido",
            GitResult::Pulled {
                repository: repository.clone(),
            },
            &mut commands,
        ),
        GitOperation::Push {
            upstream,
            local_branch,
            ..
        } => {
            let result =
                push_current_upstream(&repository, &upstream, &local_branch, &mut commands).map(
                    |()| GitResult::Pushed {
                        repository: repository.clone(),
                    },
                );
            (result, Ok(()))
        }
        GitOperation::PublishBranch {
            remote,
            local_branch,
            ..
        } => {
            let result = publish_current_branch(&repository, &remote, &local_branch, &mut commands)
                .map(|()| GitResult::BranchPublished {
                    repository: repository.clone(),
                });
            (result, Ok(()))
        }
        GitOperation::LoadHistory { offset, limit, .. } => {
            let requested = limit.clamp(1, 50);
            let count = requested.saturating_add(1);
            let args = strings(&[
                "log",
                "--no-show-signature",
                "--format=format:%H%x00%an%x00%at%x00%s%x00",
                "--skip",
                &offset.to_string(),
                "-n",
                &count.to_string(),
                "--all",
            ]);
            let (entry, out) = run_git(&repository, &args);
            commands.push(entry);
            let result = out.map_err(process_error).and_then(|out| {
                require_success(out, &commands[0], "No se pudo cargar el historial")
                    .and_then(|out| parse_history(&out.stdout, requested))
            });
            (
                result.map(|(commits, has_more)| GitResult::HistoryLoaded {
                    repository: repository.clone(),
                    offset,
                    commits,
                    has_more,
                }),
                Ok(()),
            )
        }
        GitOperation::LoadCommitDetail { commit_id, .. } => {
            let resolved = resolve_commit(&repository, &commit_id, &mut commands);
            let mut args = strings(&[
                "show",
                "-s",
                "--no-show-signature",
                "--no-ext-diff",
                "--no-textconv",
                "--format=format:%H%x00%an%x00%at%x00%s%x00%B%x00",
                "--no-renames",
                "",
            ]);
            let oid = match resolved {
                Ok(oid) => oid,
                Err(error) => return (Err(error), commands),
            };
            *args.last_mut().unwrap() = OsString::from(&oid);
            let (summary_entry, summary_out) = run_git(&repository, &args);
            commands.push(summary_entry);
            let detail = summary_out.map_err(process_error).and_then(|out| {
                require_success(out, &commands[1], "No se pudo cargar el commit")
                    .and_then(|out| parse_commit_detail(&out.stdout.bytes))
            });
            args = strings(&[
                "diff-tree",
                "--root",
                "--no-commit-id",
                "--name-status",
                "--no-renames",
                "-r",
                "-z",
                &oid,
            ]);
            let (files_entry, files_out) = run_git(&repository, &args);
            commands.push(files_entry);
            let files = files_out.map_err(process_error).and_then(|out| {
                require_success(
                    out,
                    &commands[2],
                    "No se pudieron listar los archivos del commit",
                )
                .and_then(|out| parse_changed_files(&out.stdout.bytes))
            });
            let result = detail.and_then(|mut detail| {
                files.map(|files| {
                    detail.changed_files = files;
                    detail
                })
            });
            (
                result.map(|detail| GitResult::CommitLoaded {
                    repository: repository.clone(),
                    detail,
                }),
                Ok(()),
            )
        }
        GitOperation::CompareReferences {
            base_ref,
            target_ref,
            ..
        } => {
            let base = resolve_commit(&repository, &base_ref, &mut commands);
            let target = resolve_commit(&repository, &target_ref, &mut commands);
            let compared = base
                .and_then(|base_oid| target.map(|target_oid| (base_oid, target_oid)))
                .and_then(|(base_oid, target_oid)| {
                    compare_oids(
                        &repository,
                        &base_ref,
                        &target_ref,
                        &base_oid,
                        &target_oid,
                        &mut commands,
                    )
                });
            (
                compared.map(|comparison| GitResult::ReferencesCompared {
                    repository: repository.clone(),
                    comparison,
                }),
                Ok(()),
            )
        }
        _ => unreachable!(),
    };
    let result = result.and_then(|result| command_result.map(|()| result));
    (result, commands)
}

fn strings(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
fn require_success(
    output: Captured,
    entry: &CommandEntry,
    context: &str,
) -> Result<Captured, GitError> {
    if output.stdout.truncated || output.stderr.truncated {
        return Err(GitError {
            message: format!(
                "{context}: la salida superó el límite de captura de 1 MiB y está incompleta."
            ),
        });
    }
    if output.status.success() {
        Ok(output)
    } else {
        Err(command_error(context, entry, &output.stderr))
    }
}
fn process_error(message: String) -> GitError {
    GitError { message }
}
fn one_mutation(
    repository: &Path,
    args: Vec<OsString>,
    context: &str,
    result: GitResult,
    commands: &mut Vec<CommandEntry>,
) -> (Result<GitResult, GitError>, Result<(), GitError>) {
    let (entry, output) = run_git(repository, &args);
    commands.push(entry);
    let done = output
        .map_err(process_error)
        .and_then(|out| require_success(out, &commands[0], context).map(|_| ()));
    (done.as_ref().map(|_| result).map_err(Clone::clone), done)
}
fn sync_operation(
    repository: &Path,
    args: Vec<OsString>,
    context: &str,
    result: GitResult,
    commands: &mut Vec<CommandEntry>,
) -> (Result<GitResult, GitError>, Result<(), GitError>) {
    let (entry, output) = run_git_with_terminal_prompt(repository, &args);
    commands.push(entry);
    let done = output
        .map_err(process_error)
        .and_then(|out| require_success(out, &commands[0], context).map(|_| ()));
    if done.is_ok()
        && matches!(
            args.first().and_then(|arg| arg.to_str()),
            Some("fetch" | "pull")
        )
    {
        let _ = fetch_timestamp(repository, commands);
    }
    (done.as_ref().map(|_| result).map_err(Clone::clone), done)
}

fn run_recorded(
    repository: &Path,
    args: &[OsString],
    context: &str,
    terminal_prompt_disabled: bool,
    commands: &mut Vec<CommandEntry>,
) -> Result<Captured, GitError> {
    let (entry, output) = if terminal_prompt_disabled {
        run_git_with_terminal_prompt(repository, args)
    } else {
        run_git(repository, args)
    };
    commands.push(entry);
    output
        .map_err(process_error)
        .and_then(|output| require_success(output, commands.last().unwrap(), context))
}

fn push_current_upstream(
    repository: &Path,
    upstream: &str,
    local_branch: &str,
    commands: &mut Vec<CommandEntry>,
) -> Result<(), GitError> {
    let local_ref = format!("refs/heads/{local_branch}");
    run_recorded(
        repository,
        &strings(&["check-ref-format", &local_ref]),
        "El nombre de la rama local no es válido",
        false,
        commands,
    )?;
    let current = run_recorded(
        repository,
        &strings(&["branch", "--show-current"]),
        "No se pudo identificar la rama actual",
        false,
        commands,
    )?;
    let current = String::from_utf8_lossy(&current.stdout.bytes)
        .trim()
        .to_owned();
    if current != local_branch {
        return Err(GitError {
            message: format!(
                "La rama actual ({current}) no coincide con la rama solicitada ({local_branch}); no se hizo Push."
            ),
        });
    }

    let upstream_format =
        "--format=%(upstream:remotename)%00%(upstream:remoteref)%00%(upstream:short)";
    let tracking_ref = format!("refs/heads/{local_branch}");
    let tracking = run_recorded(
        repository,
        &strings(&["for-each-ref", upstream_format, &tracking_ref]),
        "No se pudo resolver el upstream de la rama",
        false,
        commands,
    )?;
    let fields = String::from_utf8_lossy(&tracking.stdout.bytes)
        .split('\0')
        .map(str::trim)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if fields.len() != 3 || fields[0].is_empty() || fields[1].is_empty() || fields[2] != upstream {
        return Err(GitError {
            message: "El upstream solicitado no coincide con la referencia configurada para la rama; no se hizo Push.".into(),
        });
    }
    let remote = &fields[0];
    let remote_ref = &fields[1];
    let remote_branch = remote_ref
        .strip_prefix("refs/heads/")
        .ok_or_else(|| GitError {
            message: "El upstream no apunta a una rama de heads remota; no se hizo Push.".into(),
        })?;
    if remote_branch.is_empty() {
        return Err(GitError {
            message: "El upstream no contiene un nombre de rama remota; no se hizo Push.".into(),
        });
    }
    let head = run_recorded(
        repository,
        &strings(&[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("refs/heads/{local_branch}^{{commit}}"),
        ]),
        "La rama actual todavía no tiene un commit que publicar",
        false,
        commands,
    )?;
    let head_oid = String::from_utf8_lossy(&head.stdout.bytes)
        .trim()
        .to_owned();
    if head_oid.is_empty() || !head_oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GitError {
            message: "Git no devolvió un commit válido para la rama actual; no se publicó.".into(),
        });
    }

    let remotes = run_recorded(
        repository,
        &strings(&["remote"]),
        "No se pudieron consultar los remotos configurados",
        false,
        commands,
    )?;
    let remote_exists = String::from_utf8_lossy(&remotes.stdout.bytes)
        .lines()
        .any(|configured| configured.trim() == remote);
    if !remote_exists {
        return Err(GitError {
            message: format!("El remoto configurado ({remote}) ya no existe; no se hizo Push."),
        });
    }
    let remote_probe = format!("refs/remotes/{remote}/gitzi-probe");
    run_recorded(
        repository,
        &strings(&["check-ref-format", &remote_probe]),
        "El nombre del remoto configurado no es válido",
        false,
        commands,
    )?;

    run_recorded(
        repository,
        &strings(&["check-ref-format", remote_ref]),
        "El upstream no es una referencia remota válida",
        false,
        commands,
    )?;
    let refspec = format!("{head_oid}:{remote_ref}");
    let mirror_override = format!("remote.{remote}.mirror=false");
    run_recorded(
        repository,
        &strings(&[
            "-c",
            &mirror_override,
            "push",
            "--no-follow-tags",
            "--",
            remote,
            &refspec,
        ]),
        "No se pudo publicar los cambios",
        true,
        commands,
    )?;
    Ok(())
}

fn publish_current_branch(
    repository: &Path,
    remote: &str,
    local_branch: &str,
    commands: &mut Vec<CommandEntry>,
) -> Result<(), GitError> {
    let local_ref = format!("refs/heads/{local_branch}");
    run_recorded(
        repository,
        &strings(&["check-ref-format", &local_ref]),
        "El nombre de la rama local no es válido",
        false,
        commands,
    )?;
    let current = run_recorded(
        repository,
        &strings(&["branch", "--show-current"]),
        "No se pudo identificar la rama actual",
        false,
        commands,
    )?;
    let current = String::from_utf8_lossy(&current.stdout.bytes)
        .trim()
        .to_owned();
    if current != local_branch {
        return Err(GitError {
            message: format!(
                "La rama actual ({current}) no coincide con la rama solicitada ({local_branch}); no se publicó."
            ),
        });
    }
    let head = run_recorded(
        repository,
        &strings(&[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("refs/heads/{local_branch}^{{commit}}"),
        ]),
        "La rama actual todavía no tiene un commit que publicar",
        false,
        commands,
    )?;
    let head_oid = String::from_utf8_lossy(&head.stdout.bytes)
        .trim()
        .to_owned();
    if head_oid.is_empty() || !head_oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GitError {
            message: "Git no devolvió un commit válido para la rama actual; no se publicó.".into(),
        });
    }

    let remotes = run_recorded(
        repository,
        &strings(&["remote"]),
        "No se pudieron consultar los remotos configurados",
        false,
        commands,
    )?;
    let remote_exists = String::from_utf8_lossy(&remotes.stdout.bytes)
        .lines()
        .any(|configured| configured.trim() == remote);
    if !remote_exists {
        return Err(GitError {
            message: format!(
                "El remoto seleccionado ({remote}) no está configurado; no se publicó."
            ),
        });
    }
    let remote_probe = format!("refs/remotes/{remote}/gitzi-probe");
    run_recorded(
        repository,
        &strings(&["check-ref-format", &remote_probe]),
        "El nombre del remoto configurado no es válido",
        false,
        commands,
    )?;
    let remote_ref = format!("refs/heads/{local_branch}");
    run_recorded(
        repository,
        &strings(&["check-ref-format", &remote_ref]),
        "La rama publicada no es una referencia remota válida",
        false,
        commands,
    )?;

    let mirror_override = format!("remote.{remote}.mirror=false");
    let refspec = format!("{head_oid}:{remote_ref}");
    run_recorded(
        repository,
        &strings(&[
            "-c",
            &mirror_override,
            "push",
            "--no-follow-tags",
            "--",
            remote,
            &refspec,
        ]),
        "No se pudo publicar la rama",
        true,
        commands,
    )?;
    let remote_key = format!("branch.{local_branch}.remote");
    run_recorded(
        repository,
        &strings(&["config", "--local", "--replace-all", &remote_key, remote]),
        "La rama fue publicada, pero no se pudo configurar su remoto upstream",
        false,
        commands,
    )?;
    let merge_key = format!("branch.{local_branch}.merge");
    run_recorded(
        repository,
        &strings(&[
            "config",
            "--local",
            "--replace-all",
            &merge_key,
            &remote_ref,
        ]),
        "La rama fue publicada, pero no se pudo configurar su referencia upstream",
        false,
        commands,
    )?;
    Ok(())
}

fn parse_branches(bytes: &[u8]) -> Result<Vec<BranchInfo>, GitError> {
    let mut result = Vec::new();
    for line in bytes.split(|b| *b == b'\n') {
        let mut pair = line.splitn(2, |b| *b == b'\t');
        let Some(raw_name) = pair.next() else {
            continue;
        };
        let raw_name = String::from_utf8_lossy(raw_name).trim().to_owned();
        let (kind, name) = if let Some(name) = raw_name.strip_prefix("refs/heads/") {
            (BranchKind::Local, name.to_owned())
        } else if let Some(name) = raw_name.strip_prefix("refs/remotes/") {
            (BranchKind::Remote, name.to_owned())
        } else {
            continue;
        };
        let upstream = String::from_utf8_lossy(pair.next().unwrap_or_default())
            .trim()
            .to_owned();
        if name.is_empty() {
            continue;
        }
        if kind == BranchKind::Remote && name.ends_with("/HEAD") {
            continue;
        }
        result.push(BranchInfo {
            name,
            kind,
            current: false,
            upstream: (!upstream.is_empty()).then_some(upstream),
        });
    }
    Ok(result)
}
fn parse_remotes(bytes: &[u8]) -> Vec<RemoteInfo> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| RemoteInfo {
            name: name.to_owned(),
        })
        .collect()
}
fn parse_history(
    stream: &CapturedStream,
    limit: usize,
) -> Result<(Vec<CommitSummary>, bool), GitError> {
    if stream.truncated {
        return Err(GitError { message: "El historial superó el límite de captura de 1 MiB; no se muestran resultados incompletos.".into() });
    }
    let fields = stream.bytes.split(|b| *b == 0).collect::<Vec<_>>();
    let (records, remainder) = fields.as_chunks::<4>();
    if remainder.len() != 1 {
        return Err(GitError {
            message: "La respuesta de git log no tiene el formato esperado.".into(),
        });
    }
    let records = records
        .iter()
        .filter(|record| !record[0].is_empty())
        .collect::<Vec<_>>();
    let has_more = records.len() > limit;
    let commits = records
        .into_iter()
        .take(limit)
        .map(|record| {
            let id = String::from_utf8_lossy(record[0]).trim().to_owned();
            let author = String::from_utf8_lossy(record[1]).into_owned();
            let seconds = String::from_utf8_lossy(record[2])
                .trim()
                .parse::<u64>()
                .unwrap_or(0);
            CommitSummary {
                id,
                author,
                authored_at: UNIX_EPOCH + Duration::from_secs(seconds),
                subject: String::from_utf8_lossy(record[3])
                    .trim_end_matches(['\r', '\n'])
                    .to_owned(),
            }
        })
        .collect();
    Ok((commits, has_more))
}
fn parse_commit_detail(bytes: &[u8]) -> Result<CommitDetail, GitError> {
    let fields = bytes.split(|b| *b == 0).collect::<Vec<_>>();
    if fields.len() < 5 {
        return Err(GitError {
            message: "No se pudo interpretar el detalle del commit.".into(),
        });
    }
    let seconds = String::from_utf8_lossy(fields[2])
        .trim()
        .parse::<u64>()
        .unwrap_or(0);
    let summary = CommitSummary {
        id: String::from_utf8_lossy(fields[0]).into_owned(),
        author: String::from_utf8_lossy(fields[1]).into_owned(),
        authored_at: UNIX_EPOCH + Duration::from_secs(seconds),
        subject: String::from_utf8_lossy(fields[3]).to_string(),
    };
    let message = String::from_utf8_lossy(fields[4])
        .trim_matches(['\r', '\n'])
        .to_owned();
    Ok(CommitDetail {
        summary,
        message,
        changed_files: Vec::new(),
    })
}
fn parse_changed_files(bytes: &[u8]) -> Result<Vec<CommitFileChange>, GitError> {
    let fields = bytes
        .split(|b| *b == 0)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let status = String::from_utf8_lossy(fields[index]).into_owned();
        index += 1;
        let path = fields.get(index).ok_or_else(|| GitError {
            message: "La lista de archivos del commit está incompleta.".into(),
        })?;
        files.push(CommitFileChange {
            status,
            path: super::parsers::path_from_bytes(path),
        });
        index += 1;
    }
    Ok(files)
}
fn resolve_commit(
    repository: &Path,
    reference: &str,
    commands: &mut Vec<CommandEntry>,
) -> Result<String, GitError> {
    let arg = format!("{reference}^{{commit}}");
    let args = strings(&["rev-parse", "--verify", "--end-of-options", &arg]);
    let (entry, output) = run_git(repository, &args);
    commands.push(entry);
    let captured = output.map_err(process_error).and_then(|out| {
        require_success(
            out,
            commands.last().unwrap(),
            "La referencia no identifica un commit",
        )
        .map(|out| String::from_utf8_lossy(&out.stdout.bytes).trim().to_owned())
    })?;
    if captured.is_empty() || !captured.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(GitError {
            message: "Git devolvió un identificador de commit inválido.".into(),
        });
    }
    Ok(captured)
}
fn compare_oids(
    repository: &Path,
    base_ref: &str,
    target_ref: &str,
    base_oid: &str,
    target_oid: &str,
    commands: &mut Vec<CommandEntry>,
) -> Result<RefComparison, GitError> {
    let args = strings(&[
        "rev-list",
        "--left-right",
        "--count",
        &format!("{base_oid}...{target_oid}"),
    ]);
    let (entry, output) = run_git(repository, &args);
    commands.push(entry);
    let counts = output.map_err(process_error).and_then(|out| {
        require_success(
            out,
            commands.last().unwrap(),
            "No se pudieron comparar las referencias",
        )
        .map(|out| String::from_utf8_lossy(&out.stdout.bytes).into_owned())
    })?;
    let mut counts = counts.split_whitespace().map(|v| {
        v.parse::<u32>().map_err(|_| GitError {
            message: "Git devolvió contadores de commits inválidos.".into(),
        })
    });
    let behind = counts.next().ok_or_else(|| GitError {
        message: "Falta el contador de commits atrasados.".into(),
    })??;
    let ahead = counts.next().ok_or_else(|| GitError {
        message: "Falta el contador de commits adelantados.".into(),
    })??;
    let args = strings(&[
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        base_oid,
        target_oid,
    ]);
    let (entry, output) = run_git(repository, &args);
    commands.push(entry);
    let output = output.map_err(process_error).and_then(|out| {
        if !out.status.success() {
            Err(command_error(
                "No se pudo comparar las referencias",
                commands.last().unwrap(),
                &out.stderr,
            ))
        } else if out.stderr.truncated {
            Err(GitError {
                message: "La salida de error del diff superó el límite de captura.".into(),
            })
        } else {
            Ok(out.stdout)
        }
    })?;
    Ok(RefComparison {
        base_ref: base_ref.to_owned(),
        target_ref: target_ref.to_owned(),
        commits_ahead: ahead,
        commits_behind: behind,
        diff: capture_text(&output),
        truncated: output.truncated,
    })
}
fn fetch_timestamp(repository: &Path, commands: &mut Vec<CommandEntry>) -> Option<SystemTime> {
    let args = strings(&["rev-parse", "--git-path", "FETCH_HEAD"]);
    let (entry, output) = run_git(repository, &args);
    commands.push(entry);
    let path = output
        .ok()
        .filter(|out| out.status.success() && !out.stdout.truncated)
        .map(|out| PathBuf::from(String::from_utf8_lossy(&out.stdout.bytes).trim().to_owned()))?;
    let path = if path.is_absolute() {
        path
    } else {
        repository.join(path)
    };
    std::fs::metadata(path).ok()?.modified().ok()
}

struct CapturedStream {
    bytes: Vec<u8>,
    truncated: bool,
}

struct Captured {
    stdout: CapturedStream,
    stderr: CapturedStream,
    status: std::process::ExitStatus,
}

fn parse_status_output(stdout: &CapturedStream) -> Result<RepositoryStatus, GitError> {
    if stdout.truncated {
        return Err(GitError { message: "La salida de git status superó el límite de captura de 1 MiB; el estado está incompleto.".into() });
    }
    super::parsers::parse_status(&stdout.bytes).map_err(|error| GitError {
        message: format!("No se pudo interpretar la respuesta de Git: {error}"),
    })
}

fn successful_mutation(
    output: Result<Captured, String>,
    entry: &CommandEntry,
    context: &str,
) -> Result<(), GitError> {
    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(command_error(context, entry, &output.stderr)),
        Err(error) => Err(GitError {
            message: format!("{context}. {error}"),
        }),
    }
}

fn run_git_from_current_dir(args: &[OsString]) -> (CommandEntry, Result<Captured, String>) {
    match std::env::current_dir() {
        Ok(cwd) => run_git(&cwd, args),
        Err(error) => {
            let message = format!(
                "No se pudo obtener el directorio de trabajo actual para ejecutar Git: {error}"
            );
            failed_without_process(args, message)
        }
    }
}

fn failed_without_process(
    args: &[OsString],
    message: String,
) -> (CommandEntry, Result<Captured, String>) {
    let started_at = SystemTime::now();
    let display = command_display("git", args);
    let entry = CommandEntry {
        id: LOG_ID.fetch_add(1, Ordering::Relaxed),
        command_display: redact_text(&display),
        cwd: None,
        started_at,
        duration: Duration::ZERO,
        status: CommandStatus::Failed,
        exit_code: None,
        stdout: String::new(),
        stderr: redact_text(&message),
    };
    (entry, Err(message))
}

fn run_git(cwd: &Path, args: &[OsString]) -> (CommandEntry, Result<Captured, String>) {
    run_git_with_input(cwd, args, None)
}
fn run_git_with_terminal_prompt(
    cwd: &Path,
    args: &[OsString],
) -> (CommandEntry, Result<Captured, String>) {
    run_git_with_options(cwd, args, None, true)
}

fn run_git_with_input(
    cwd: &Path,
    args: &[OsString],
    input: Option<&[u8]>,
) -> (CommandEntry, Result<Captured, String>) {
    run_git_with_options(cwd, args, input, false)
}

fn run_git_with_options(
    cwd: &Path,
    args: &[OsString],
    input: Option<&[u8]>,
    disable_terminal_prompt: bool,
) -> (CommandEntry, Result<Captured, String>) {
    let started_at = SystemTime::now();
    let timer = Instant::now();
    let display = command_display("git", args);
    let mut command = Command::new("git");
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.stdin(if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    command.current_dir(cwd);
    if disable_terminal_prompt {
        command.env("GIT_TERMINAL_PROMPT", "0");
    }
    let launched = command.spawn();
    let outcome = match launched {
        Ok(mut child) => {
            let stdout = child.stdout.take().expect("piped stdout");
            let stderr = child.stderr.take().expect("piped stderr");
            let out_thread = thread::spawn(move || read_limited(stdout));
            let err_thread = thread::spawn(move || read_limited(stderr));
            let write_input = if let Some(input) = input {
                match child.stdin.take().expect("piped stdin").write_all(input) {
                    Ok(()) => Ok(()),
                    Err(error) => Err(format!("Error enviando entrada a Git: {error}")),
                }
            } else {
                Ok(())
            };
            let status = child.wait();
            let out = out_thread
                .join()
                .unwrap_or_else(|_| Err("No se pudo leer stdout".into()));
            let err = err_thread
                .join()
                .unwrap_or_else(|_| Err("No se pudo leer stderr".into()));
            match (status, out, err, write_input) {
                (Ok(status), Ok(stdout), Ok(stderr), Ok(())) => Ok(Captured {
                    stdout,
                    stderr,
                    status,
                }),
                (Err(error), _, _, _) => Err(format!("Error esperando el proceso: {error}")),
                (_, Err(error), _, _) | (_, _, Err(error), _) | (_, _, _, Err(error)) => Err(error),
            }
        }
        Err(error) => Err(format!("{error}")),
    };
    let duration = timer.elapsed();
    let (status, exit_code, stdout, stderr) = match &outcome {
        Ok(output) => (
            if output.status.success() {
                CommandStatus::Succeeded
            } else {
                CommandStatus::Failed
            },
            output.status.code(),
            redact_text(&capture_text(&output.stdout)),
            redact_text(&capture_text(&output.stderr)),
        ),
        Err(error) => (
            CommandStatus::Failed,
            None,
            String::new(),
            redact_text(error),
        ),
    };
    let entry = CommandEntry {
        id: LOG_ID.fetch_add(1, Ordering::Relaxed),
        command_display: redact_text(&display),
        cwd: Some(cwd.to_path_buf()),
        started_at,
        duration,
        status,
        exit_code,
        stdout,
        stderr,
    };
    (entry, outcome)
}

fn read_limited<R: Read>(mut reader: R) -> Result<CapturedStream, String> {
    let mut kept = Vec::with_capacity(MAX_CAPTURE_BYTES.min(8192));
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| format!("Error leyendo salida de Git: {error}"))?;
        if count == 0 {
            break;
        }
        let remaining = MAX_CAPTURE_BYTES.saturating_sub(kept.len());
        kept.extend_from_slice(&buffer[..count.min(remaining)]);
        truncated |= count > remaining;
        // Continue draining even after the cap so Git cannot block on a full pipe.
    }
    Ok(CapturedStream {
        bytes: kept,
        truncated,
    })
}

const TRUNCATION_MARKER: &str = "\n[Salida truncada: límite de captura de 1 MiB]";

fn capture_text(capture: &CapturedStream) -> String {
    let mut text = decode_capture(&capture.bytes);
    if capture.truncated {
        let text_limit = MAX_CAPTURE_BYTES.saturating_sub(TRUNCATION_MARKER.len());
        if text.len() > text_limit {
            let mut boundary = text_limit;
            while !text.is_char_boundary(boundary) {
                boundary -= 1;
            }
            text.truncate(boundary);
        }
        text.push_str(TRUNCATION_MARKER);
    }
    text
}

fn decode_capture(bytes: &[u8]) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    if text.len() > MAX_CAPTURE_BYTES {
        let mut boundary = MAX_CAPTURE_BYTES;
        while !text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        text.truncate(boundary);
    }
    text
}

fn command_display(executable: &str, args: &[OsString]) -> String {
    std::iter::once(executable.to_owned())
        .chain(args.iter().map(|arg| arg.to_string_lossy().into_owned()))
        .map(|word| quote_shell_word(&word))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(windows)]
fn quote_shell_word(word: &str) -> String {
    if !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._/:\\-=".contains(c))
    {
        return word.to_owned();
    }
    format!("'{}'", word.replace('\'', "''"))
}

#[cfg(not(windows))]
fn quote_shell_word(word: &str) -> String {
    if !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._/:\\-=".contains(c))
    {
        return word.to_owned();
    }
    format!("'{}'", word.replace('\'', "'\\''"))
}

fn redact_text(text: &str) -> String {
    // Mask credentials in common URL forms before either logs or errors are exposed.
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(scheme_pos) = remaining.find("://") {
        let after_scheme = scheme_pos + 3;
        result.push_str(&remaining[..after_scheme]);
        let tail = &remaining[after_scheme..];
        let end = tail
            .find(['/', '?', '#', ' ', '\n', '\r'])
            .unwrap_or(tail.len());
        let authority = &tail[..end];
        if let Some(at) = authority.rfind('@') {
            result.push_str("***@");
            result.push_str(&authority[at + 1..]);
        } else {
            result.push_str(authority);
        }
        remaining = &tail[end..];
    }
    result.push_str(remaining);
    result
}

fn command_error(context: &str, entry: &CommandEntry, stderr: &CapturedStream) -> GitError {
    let detail = redact_text(&capture_text(stderr)).trim().to_owned();
    let suffix = if detail.is_empty() {
        format!("Código de salida: {:?}.", entry.exit_code)
    } else {
        detail
    };
    GitError {
        message: format!("{context}. {suffix}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::{
        fs,
        process::Command as TestCommand,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_REPO: AtomicU64 = AtomicU64::new(0);

    struct TempPath(PathBuf);
    impl TempPath {
        fn new() -> Self {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!(
                    "gitzi-t3-{}-{}",
                    std::process::id(),
                    NEXT_REPO.fetch_add(1, Ordering::Relaxed)
                ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TempPath {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn raw_git(repo: &Path, args: &[&str]) -> String {
        let out = TestCommand::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }
    fn commit(repo: &Path, file: &str, body: &str, message: &str) {
        fs::write(repo.join(file), body).unwrap();
        raw_git(repo, &["add", "--", file]);
        raw_git(repo, &["commit", "-m", message]);
    }
    fn run(operation: GitOperation) -> GitResponse {
        handle_request(GitRequest {
            id: 900,
            generation: 7,
            operation,
        })
    }

    #[test]
    fn output_capture_is_capped_while_input_is_fully_drained() {
        let input = vec![b'x'; MAX_CAPTURE_BYTES * 2 + 17];
        let mut reader = Cursor::new(input);
        let captured = read_limited(&mut reader).unwrap();
        assert_eq!(captured.bytes.len(), MAX_CAPTURE_BYTES);
        assert!(captured.truncated);
        assert_eq!(reader.position() as usize, MAX_CAPTURE_BYTES * 2 + 17);
    }

    #[test]
    fn exact_limit_is_not_marked_truncated() {
        let input = vec![b'x'; MAX_CAPTURE_BYTES];
        let captured = read_limited(Cursor::new(input)).unwrap();
        assert_eq!(captured.bytes.len(), MAX_CAPTURE_BYTES);
        assert!(!captured.truncated);
    }

    #[test]
    fn collaboration_round_trip_with_real_bare_remote() {
        let temp = TempPath::new();
        let root = temp.0.as_path();
        let bare = root.join("remote.git");
        let seed = root.join("seed");
        let left = root.join("clone left");
        let right = root.join("clone right");
        fs::create_dir_all(&bare).unwrap();
        assert!(
            TestCommand::new("git")
                .args(["init", "--bare", bare.to_str().unwrap()])
                .status()
                .unwrap()
                .success()
        );
        fs::create_dir_all(&seed).unwrap();
        raw_git(&seed, &["init"]);
        raw_git(&seed, &["switch", "--create", "main"]);
        raw_git(&seed, &["config", "user.name", "Gitzi Test"]);
        raw_git(&seed, &["config", "user.email", "gitzi@example.test"]);
        commit(&seed, "base.txt", "base\n", "initial");
        raw_git(&seed, &["remote", "add", "origin", bare.to_str().unwrap()]);
        raw_git(&seed, &["push", "--set-upstream", "origin", "main"]);
        assert!(
            TestCommand::new("git")
                .args([
                    "--git-dir",
                    bare.to_str().unwrap(),
                    "symbolic-ref",
                    "HEAD",
                    "refs/heads/main"
                ])
                .status()
                .unwrap()
                .success()
        );
        raw_git(
            root,
            &["clone", bare.to_str().unwrap(), left.to_str().unwrap()],
        );
        raw_git(
            root,
            &["clone", bare.to_str().unwrap(), right.to_str().unwrap()],
        );
        for repo in [&left, &right] {
            raw_git(repo, &["config", "user.name", "Gitzi Test"]);
            raw_git(repo, &["config", "user.email", "gitzi@example.test"]);
        }

        let listed = run(GitOperation::ListBranches(left.clone()));
        match listed.result.unwrap() {
            GitResult::BranchesListed {
                branches, remotes, ..
            } => {
                assert!(
                    branches
                        .iter()
                        .any(|b| b.name == "main" && b.kind == BranchKind::Local && b.current)
                );
                assert!(
                    branches
                        .iter()
                        .any(|b| b.name == "origin/main" && b.kind == BranchKind::Remote)
                );
                assert_eq!(
                    remotes,
                    vec![RemoteInfo {
                        name: "origin".into()
                    }]
                );
            }
            other => panic!("{other:?}"),
        }

        // Create a remote branch first so SetUpstream associates only an existing ref.
        raw_git(&seed, &["switch", "--create", "topic"]);
        commit(&seed, "topic.txt", "topic\n", "topic seed");
        raw_git(&seed, &["push", "--set-upstream", "origin", "topic"]);
        assert!(matches!(
            run(GitOperation::Fetch(left.clone())).result.unwrap(),
            GitResult::Fetched { .. }
        ));
        assert!(matches!(
            run(GitOperation::CreateBranch {
                repository: left.clone(),
                name: "topic".into()
            })
            .result
            .unwrap(),
            GitResult::BranchCreated { .. }
        ));
        assert!(matches!(
            run(GitOperation::SetUpstream {
                repository: left.clone(),
                local_branch: "topic".into(),
                remote_branch: "origin/topic".into()
            })
            .result
            .unwrap(),
            GitResult::UpstreamSet { .. }
        ));
        assert!(matches!(
            run(GitOperation::SwitchBranch {
                repository: left.clone(),
                name: "main".into()
            })
            .result
            .unwrap(),
            GitResult::BranchSwitched { .. }
        ));

        // Publish with mirror=true must preserve remote refs outside this branch.
        raw_git(&seed, &["switch", "--create", "mirror-only", "main"]);
        commit(&seed, "mirror-only.txt", "keep\n", "remote-only branch");
        raw_git(&seed, &["push", "--set-upstream", "origin", "mirror-only"]);
        let mirror_only_oid = raw_git(&bare, &["rev-parse", "refs/heads/mirror-only"]);
        raw_git(&left, &["config", "remote.origin.mirror", "true"]);

        // Publish establishes upstream explicitly, then Push follows that upstream.
        assert!(matches!(
            run(GitOperation::CreateBranch {
                repository: left.clone(),
                name: "feature/published".into()
            })
            .result
            .unwrap(),
            GitResult::BranchCreated { .. }
        ));
        let publish_oid = raw_git(&left, &["rev-parse", "refs/heads/feature/published"]);
        let publish = run(GitOperation::PublishBranch {
            repository: left.clone(),
            remote: "origin".into(),
            local_branch: "feature/published".into(),
        });
        assert!(matches!(
            &publish.result,
            Ok(GitResult::BranchPublished { .. })
        ));
        let publish_commands = publish
            .commands
            .iter()
            .map(|entry| entry.command_display.as_str())
            .collect::<Vec<_>>();
        assert!(publish_commands.iter().any(|command| command.contains(&format!("-c remote.origin.mirror=false push --no-follow-tags -- origin {publish_oid}:refs/heads/feature/published"))));
        assert!(publish_commands.iter().any(|command| {
            command.contains("config --local --replace-all branch.feature/published.remote origin")
        }));
        assert!(publish_commands.iter().any(|command| command.contains("config --local --replace-all branch.feature/published.merge refs/heads/feature/published")));
        assert!(
            publish_commands
                .iter()
                .all(|command| !command.contains("force"))
        );
        assert_eq!(
            raw_git(&bare, &["rev-parse", "refs/heads/feature/published"]),
            publish_oid
        );
        assert_eq!(
            raw_git(&bare, &["rev-parse", "refs/heads/mirror-only"]),
            mirror_only_oid
        );
        assert_eq!(
            raw_git(&left, &["config", "--get", "remote.origin.mirror"]),
            "true"
        );
        assert_eq!(
            raw_git(
                &left,
                &["config", "--get", "branch.feature/published.remote"]
            ),
            "origin"
        );
        assert_eq!(
            raw_git(
                &left,
                &["config", "--get", "branch.feature/published.merge"]
            ),
            "refs/heads/feature/published"
        );
        commit(&left, "published.txt", "published\n", "published commit");

        // A configured matching default or remote push refspec must not add branches.
        raw_git(&seed, &["switch", "--create", "other", "main"]);
        commit(&seed, "other-base.txt", "base\n", "other base");
        raw_git(&seed, &["push", "--set-upstream", "origin", "other"]);
        assert!(matches!(
            run(GitOperation::Fetch(left.clone())).result.unwrap(),
            GitResult::Fetched { .. }
        ));
        raw_git(&left, &["branch", "other", "origin/other"]);
        raw_git(&left, &["switch", "other"]);
        commit(&left, "other-local.txt", "private\n", "other local only");
        let other_oid_before = raw_git(&bare, &["rev-parse", "refs/heads/other"]);
        raw_git(&left, &["config", "push.default", "matching"]);
        raw_git(
            &left,
            &[
                "config",
                "remote.origin.push",
                "refs/heads/other:refs/heads/other",
            ],
        );
        raw_git(&left, &["config", "remote.origin.mirror", "true"]);
        raw_git(&left, &["switch", "feature/published"]);

        let feature_push_oid = raw_git(&left, &["rev-parse", "refs/heads/feature/published"]);
        let feature_push = run(GitOperation::Push {
            repository: left.clone(),
            upstream: "origin/feature/published".into(),
            local_branch: "feature/published".into(),
        });
        assert!(matches!(
            feature_push.result.unwrap(),
            GitResult::Pushed { .. }
        ));
        assert!(
            feature_push
                .commands
                .last()
                .unwrap()
                .command_display
                .contains(&format!(
                    "origin {feature_push_oid}:refs/heads/feature/published"
                ))
        );
        assert_eq!(
            raw_git(&bare, &["rev-parse", "refs/heads/feature/published"]),
            feature_push_oid
        );
        assert_eq!(
            raw_git(&bare, &["rev-parse", "refs/heads/other"]),
            other_oid_before
        );
        assert_eq!(
            raw_git(&left, &["config", "--get", "remote.origin.mirror"]),
            "true"
        );

        // A normal fast-forward exchange uses fetch then pull --ff-only.
        run(GitOperation::SwitchBranch {
            repository: left.clone(),
            name: "main".into(),
        });
        commit(&left, "left.txt", "left\n", "left main change");
        let main_push_oid = raw_git(&left, &["rev-parse", "refs/heads/main"]);
        let main_push = run(GitOperation::Push {
            repository: left.clone(),
            upstream: "origin/main".into(),
            local_branch: "main".into(),
        });
        assert!(matches!(
            main_push.result.unwrap(),
            GitResult::Pushed { .. }
        ));
        assert!(
            main_push
                .commands
                .last()
                .unwrap()
                .command_display
                .contains(&format!("origin {main_push_oid}:refs/heads/main"))
        );
        assert_eq!(
            raw_git(&bare, &["rev-parse", "refs/heads/main"]),
            main_push_oid
        );
        assert!(matches!(
            run(GitOperation::Fetch(right.clone())).result.unwrap(),
            GitResult::Fetched { .. }
        ));
        assert!(matches!(
            run(GitOperation::PullFastForward(right.clone()))
                .result
                .unwrap(),
            GitResult::Pulled { .. }
        ));

        // Concurrent commits make the later push fail; it must remain a plain push.
        commit(&right, "right.txt", "right\n", "right concurrent change");
        raw_git(&right, &["push"]);
        commit(
            &left,
            "diverged.txt",
            "diverged\n",
            "left concurrent change",
        );
        let rejected = run(GitOperation::Push {
            repository: left.clone(),
            upstream: "origin/main".into(),
            local_branch: "main".into(),
        });
        assert!(rejected.result.is_err());
        let push_log = rejected.commands.last().unwrap().command_display.as_str();
        assert!(push_log.contains("push --no-follow-tags -- origin "));
        assert!(push_log.contains(":refs/heads/main"));
        assert!(!push_log.contains("HEAD:"));
        assert!(!push_log.contains("force"));

        // History pages, detail, and ref comparison use real commits and refs.
        for n in 0..4 {
            commit(
                &left,
                &format!("page-{n}.txt"),
                &format!("{n}\n"),
                &format!("page {n}"),
            );
        }
        let history = run(GitOperation::LoadHistory {
            repository: left.clone(),
            offset: 0,
            limit: 2,
        });
        let commit_id = match history.result.unwrap() {
            GitResult::HistoryLoaded {
                commits, has_more, ..
            } => {
                assert_eq!(commits.len(), 2);
                assert!(has_more);
                commits[0].id.clone()
            }
            other => panic!("{other:?}"),
        };
        let detail = run(GitOperation::LoadCommitDetail {
            repository: left.clone(),
            commit_id,
        });
        match detail.result.unwrap() {
            GitResult::CommitLoaded { detail, .. } => {
                assert!(!detail.message.is_empty());
                assert_eq!(detail.changed_files.len(), 1);
            }
            other => panic!("{other:?}"),
        }
        let comparison = run(GitOperation::CompareReferences {
            repository: left.clone(),
            base_ref: "main~1".into(),
            target_ref: "main".into(),
        });
        match comparison.result.unwrap() {
            GitResult::ReferencesCompared { comparison, .. } => {
                assert_eq!(comparison.commits_ahead, 1);
                assert_eq!(comparison.commits_behind, 0);
                assert!(comparison.diff.contains("page-"));
            }
            other => panic!("{other:?}"),
        }
        let status = run(GitOperation::InspectRepository(left.clone()));
        match status.result.unwrap() {
            GitResult::RepositoryInspected { status, .. } => {
                assert!(status.last_fetch_at.is_some())
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn truncated_status_stdout_is_never_parsed_and_is_marked_in_log() {
        let stdout = CapturedStream {
            bytes: b"# branch.head main\0".to_vec(),
            truncated: true,
        };
        assert!(
            parse_status_output(&stdout)
                .unwrap_err()
                .message
                .contains("superó el límite")
        );
        let logged = capture_text(&stdout);
        assert!(logged.contains("Salida truncada"));
        assert!(logged.len() <= MAX_CAPTURE_BYTES);
    }
}
