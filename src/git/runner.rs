use crate::domain::{CommandEntry, CommandStatus, RepositoryStatus};
use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Instant, SystemTime},
};

const MAX_CAPTURE_BYTES: usize = 1024 * 1024;
static LOG_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitOperation {
    DetectGit,
    InspectRepository(PathBuf),
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
            let (entry, output) = run_git(None, &[OsString::from("--version")]);
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
            let (entry, output) = run_git(Some(&path), &args);
            let result = match output {
                Ok(output) if output.stdout.truncated => Err(GitError {
                    message: "La salida de git status superó el límite de captura de 1 MiB; el estado está incompleto.".into(),
                }),
                Ok(output) if output.status.success() => match parse_status_output(&output.stdout) {
                    Ok(status) => Ok(GitResult::RepositoryInspected { path, status }),
                    Err(error) => Err(error),
                },
                Ok(output) => Err(command_error("No se pudo consultar el repositorio; comprueba que la ruta sea válida y que Git tenga acceso", &entry, &output.stderr)),
                Err(error) => Err(GitError { message: format!("No se pudo ejecutar Git para consultar {}. Comprueba que Git esté instalado y que la ruta sea accesible. {error}", path.display()) }),
            };
            (result, vec![entry])
        }
    };
    GitResponse {
        id: request.id,
        generation: request.generation,
        result,
        commands,
    }
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

fn run_git(cwd: Option<&Path>, args: &[OsString]) -> (CommandEntry, Result<Captured, String>) {
    let started_at = SystemTime::now();
    let timer = Instant::now();
    let display = command_display("git", args);
    let mut command = Command::new("git");
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let launched = command.spawn();
    let outcome = match launched {
        Ok(mut child) => {
            let stdout = child.stdout.take().expect("piped stdout");
            let stderr = child.stderr.take().expect("piped stderr");
            let out_thread = thread::spawn(move || read_limited(stdout));
            let err_thread = thread::spawn(move || read_limited(stderr));
            let status = child.wait();
            let out = out_thread
                .join()
                .unwrap_or_else(|_| Err("No se pudo leer stdout".into()));
            let err = err_thread
                .join()
                .unwrap_or_else(|_| Err("No se pudo leer stderr".into()));
            match (status, out, err) {
                (Ok(status), Ok(stdout), Ok(stderr)) => Ok(Captured {
                    stdout,
                    stderr,
                    status,
                }),
                (Err(error), _, _) => Err(format!("Error esperando el proceso: {error}")),
                (_, Err(error), _) | (_, _, Err(error)) => Err(error),
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
        cwd: cwd.map(Path::to_path_buf),
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
