use crate::domain::{FileStatus, RepositoryStatus};
use std::path::PathBuf;

pub(super) fn parse_status(bytes: &[u8]) -> Result<RepositoryStatus, String> {
    let mut status = RepositoryStatus::default();
    let mut fields = bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    while let Some(field) = fields.next() {
        if field.starts_with(b"# ") {
            let header = String::from_utf8_lossy(field);
            if let Some(value) = header.strip_prefix("# branch.head ") {
                status.branch = value.to_owned();
            } else if let Some(value) = header.strip_prefix("# branch.oid ") {
                status.has_head = if value.is_empty() {
                    None
                } else {
                    Some(value != "(initial)" && !value.bytes().all(|byte| byte == b'0'))
                };
            } else if let Some(value) = header.strip_prefix("# branch.upstream ") {
                status.upstream = Some(value.to_owned());
            } else if let Some(value) = header.strip_prefix("# branch.ab ") {
                for part in value.split_whitespace() {
                    if let Some(number) = part.strip_prefix('+') {
                        status.ahead = number.parse().unwrap_or(0);
                    }
                    if let Some(number) = part.strip_prefix('-') {
                        status.behind = number.parse().unwrap_or(0);
                    }
                }
            }
            continue;
        }
        match field.first().copied() {
            Some(b'1') => {
                let record = split_fields(field, 9)?;
                status.files.push(FileStatus {
                    path: path_from_bytes(record[8]),
                    index_status: record[1][0] as char,
                    worktree_status: record[1][1] as char,
                    unmerged: false,
                });
            }
            Some(b'2') => {
                let record = split_fields(field, 10)?;
                let path = path_from_bytes(record[9]);
                // The next NUL-delimited field is the source path. FileStatus tracks the current path.
                let _original_path = fields.next();
                status.files.push(FileStatus {
                    path,
                    index_status: record[1][0] as char,
                    worktree_status: record[1][1] as char,
                    unmerged: false,
                });
            }
            Some(b'u') => {
                let record = split_fields(field, 11)?;
                status.files.push(FileStatus {
                    path: path_from_bytes(record[10]),
                    index_status: record[1][0] as char,
                    worktree_status: record[1][1] as char,
                    unmerged: true,
                });
            }
            Some(b'?') => {
                let path = field
                    .strip_prefix(b"? ")
                    .ok_or_else(|| "registro untracked inválido".to_owned())?;
                status.files.push(FileStatus {
                    path: path_from_bytes(path),
                    index_status: '?',
                    worktree_status: '?',
                    unmerged: false,
                });
            }
            Some(b'!') => {} // ignored records are absent unless explicitly requested
            Some(other) => {
                return Err(format!(
                    "tipo de registro porcelain desconocido: {}",
                    other as char
                ));
            }
            None => {}
        }
    }
    if status.branch.is_empty() {
        return Err("Git no devolvió la rama actual".into());
    }
    Ok(status)
}

fn split_fields(record: &[u8], expected: usize) -> Result<Vec<&[u8]>, String> {
    let fields = record
        .splitn(expected, |byte| *byte == b' ')
        .collect::<Vec<_>>();
    if fields.len() != expected
        || fields
            .iter()
            .take(expected - 1)
            .any(|field| field.is_empty())
    {
        return Err("registro de archivo porcelain v2 inválido".into());
    }
    Ok(fields)
}

#[cfg(unix)]
pub(super) fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
pub(super) fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_upstream_counts_and_paths_with_spaces() {
        let mut input = b"# branch.oid abc\0# branch.head feature/x\0# branch.upstream origin/feature/x\0# branch.ab +3 -2\0? ".to_vec();
        input.extend_from_slice("archivo nuevo ñ.txt".as_bytes());
        input.push(0);
        let status = parse_status(&input).unwrap();
        assert_eq!(status.branch, "feature/x");
        assert_eq!(status.upstream.as_deref(), Some("origin/feature/x"));
        assert_eq!((status.ahead, status.behind), (3, 2));
        assert_eq!(status.files[0].path, PathBuf::from("archivo nuevo ñ.txt"));
    }

    #[test]
    fn identifies_head_from_branch_oid() {
        assert_eq!(
            parse_status(b"# branch.oid (initial)\0# branch.head main\0")
                .unwrap()
                .has_head,
            Some(false)
        );
        assert_eq!(
            parse_status(
                b"# branch.oid 0000000000000000000000000000000000000000\0# branch.head main\0"
            )
            .unwrap()
            .has_head,
            Some(false)
        );
        assert_eq!(
            parse_status(b"# branch.oid abc123\0# branch.head main\0")
                .unwrap()
                .has_head,
            Some(true)
        );
        assert_eq!(
            parse_status(b"# branch.head main\0").unwrap().has_head,
            None
        );
        assert_eq!(
            parse_status(b"# branch.oid \0# branch.head main\0")
                .unwrap()
                .has_head,
            None
        );
    }

    #[test]
    fn parses_unmerged_records_and_preserves_both_status_characters() {
        let status = parse_status(b"# branch.oid abc123\0# branch.head main\0u UU N... 100644 100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 3333333333333333333333333333333333333333 conflicted file.txt\0").unwrap();
        assert_eq!(status.files.len(), 1);
        assert_eq!(status.files[0].path, PathBuf::from("conflicted file.txt"));
        assert_eq!(
            (
                status.files[0].index_status,
                status.files[0].worktree_status
            ),
            ('U', 'U')
        );
        assert!(status.files[0].unmerged);
    }

    #[test]
    fn parses_staged_unstaged_deleted_and_renamed_records() {
        let input = b"# branch.head main\0# branch.ab +0 -0\x001 A. N... 100644 100644 100644 aaa bbb staged.txt\x001 .M N... 100644 100644 100644 aaa bbb work tree.txt\x001 D. N... 100644 000000 000000 aaa bbb gone.txt\x002 R. N... 100644 100644 100644 aaa bbb R100 new name.txt\0old name.txt\0";
        let status = parse_status(input).unwrap();
        assert_eq!(status.files.len(), 4);
        assert_eq!(
            (
                status.files[0].index_status,
                status.files[0].worktree_status
            ),
            ('A', '.')
        );
        assert_eq!(
            (
                status.files[1].index_status,
                status.files[1].worktree_status
            ),
            ('.', 'M')
        );
        assert_eq!(
            (
                status.files[2].index_status,
                status.files[2].worktree_status
            ),
            ('D', '.')
        );
        assert_eq!(status.files[3].path, PathBuf::from("new name.txt"));
        assert_eq!(status.files[3].index_status, 'R');
    }
}
