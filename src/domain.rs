use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RepositoryStatus {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub last_fetch_at: Option<SystemTime>,
    pub has_head: Option<bool>,
    pub integration: Option<IntegrationKind>,
    pub files: Vec<FileStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationKind {
    Merge,
    Rebase,
    Revert,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PullPolicy {
    #[default]
    FastForwardOnly,
    Merge,
    Rebase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchKind {
    Local,
    Remote,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchInfo {
    pub name: String,
    pub kind: BranchKind,
    pub current: bool,
    pub upstream: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteInfo {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitSummary {
    pub id: String,
    pub author: String,
    pub authored_at: SystemTime,
    pub subject: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitFileChange {
    pub status: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitDetail {
    pub summary: CommitSummary,
    pub message: String,
    pub changed_files: Vec<CommitFileChange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefComparison {
    pub base_ref: String,
    pub target_ref: String,
    pub commits_ahead: u32,
    pub commits_behind: u32,
    pub diff: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashEntry {
    pub oid: String,
    pub selector: String,
    pub message: String,
    pub created_at: SystemTime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashDetail {
    pub oid: String,
    pub diff: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileStatus {
    pub path: PathBuf,
    pub index_status: char,
    pub worktree_status: char,
    pub unmerged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffArea {
    Unstaged,
    Staged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDiff {
    pub repository: PathBuf,
    pub path: PathBuf,
    pub area: DiffArea,
    pub text: String,
    pub binary: bool,
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    Succeeded,
    SucceededWithDifferences,
    Failed,
}

#[derive(Clone, Debug)]
pub struct CommandEntry {
    pub id: u64,
    pub command_display: String,
    pub cwd: Option<PathBuf>,
    pub started_at: SystemTime,
    pub duration: Duration,
    pub status: CommandStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Debug, Default)]
pub struct AppSnapshot {
    pub git_checked: bool,
    pub git_version: Option<String>,
    pub active_repo: Option<PathBuf>,
    pub repository: Option<RepositoryStatus>,
    pub busy: bool,
    pub error_message: Option<String>,
    pub recent_repositories: Vec<PathBuf>,
    pub command_log: Vec<CommandEntry>,
    pub file_diff: Option<FileDiff>,
    pub branches: Vec<BranchInfo>,
    pub remotes: Vec<RemoteInfo>,
    pub history: Vec<CommitSummary>,
    pub history_has_more: bool,
    pub commit_detail: Option<CommitDetail>,
    pub ref_comparison: Option<RefComparison>,
    pub stashes: Vec<StashEntry>,
    pub stash_detail: Option<StashDetail>,
    pub pull_policy: PullPolicy,
    pub external_editor: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiAction {
    OpenRepository(PathBuf),
    Refresh,
    InitializeRepository(PathBuf),
    CloneRepository {
        source: String,
        destination: PathBuf,
    },
    LoadDiff {
        path: PathBuf,
        area: DiffArea,
    },
    StageFile(PathBuf),
    UnstageFile(PathBuf),
    Commit(String),
    Fetch,
    Pull,
    Push,
    PublishBranch(String),
    SetUpstream {
        local_branch: String,
        remote_branch: String,
    },
    CreateBranch(String),
    SwitchBranch(String),
    LoadMoreHistory,
    LoadCommitDetail(String),
    CompareReferences {
        base_ref: String,
        target_ref: String,
    },
    SetPullPolicy(PullPolicy),
    SetExternalEditor(Option<PathBuf>),
    OpenConflictFile(PathBuf),
    CreateStash {
        message: Option<String>,
        include_untracked: bool,
    },
    LoadStashDetail(String),
    ApplyStash(String),
    DropStash(String),
    MergeBranch(String),
    RebaseBranch(String),
    RevertCommit(String),
    ContinueIntegration,
    AbortIntegration,
    DiscardFile(PathBuf),
    DeleteBranch(String),
}
