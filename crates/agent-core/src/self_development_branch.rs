//! Pure issue-to-branch mapping; adapters perform no work here.
const MAX: usize = 256;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchRequest {
    pub issue_id: String,
    pub candidate_id: String,
    pub version: String,
    pub base_sha: String,
    pub policy: String,
    pub root: String,
    pub project_id: String,
    pub branch: String,
    pub protected_branch: bool,
    pub policy_allowed: bool,
    pub issue_present: bool,
}
impl BranchRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        i: &str,
        c: &str,
        v: &str,
        b: &str,
        p: &str,
        root: &str,
        project: &str,
        branch: &str,
        protected: bool,
        allowed: bool,
        present: bool,
    ) -> Result<Self, BranchError> {
        if [i, c, v, b, p, root, project, branch]
            .iter()
            .any(|x| x.is_empty() || x.len() > MAX)
        {
            return Err(BranchError::InvalidIdentity);
        }
        Ok(Self {
            issue_id: i.into(),
            candidate_id: c.into(),
            version: v.into(),
            base_sha: b.into(),
            policy: p.into(),
            root: root.into(),
            project_id: project.into(),
            branch: branch.into(),
            protected_branch: protected,
            policy_allowed: allowed,
            issue_present: present,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchError {
    InvalidIdentity,
    IssueMissing,
    PolicyDenied,
    ProtectedBranch,
    RootNotAllowed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupTarget {
    Unknown,
    ExpiredRegistered,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupAction {
    Preserve,
    CleanupRegistered,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchMapping {
    branch: String,
    key: String,
}
impl BranchMapping {
    pub fn create(r: BranchRequest) -> Result<Self, BranchError> {
        if !r.issue_present {
            return Err(BranchError::IssueMissing);
        }
        if !r.policy_allowed {
            return Err(BranchError::PolicyDenied);
        }
        if r.protected_branch {
            return Err(BranchError::ProtectedBranch);
        }
        if !r.root.starts_with("/srv/hank-worktrees") {
            return Err(BranchError::RootNotAllowed);
        }
        let key = digest(&format!(
            "{}|{}|{}|{}|{}",
            r.issue_id, r.candidate_id, r.version, r.base_sha, r.project_id
        ));
        Ok(Self {
            branch: r.branch,
            key,
        })
    }
    pub fn branch(&self) -> &str {
        &self.branch
    }
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn cleanup(&self, target: CleanupTarget) -> CleanupAction {
        match target {
            CleanupTarget::Unknown => CleanupAction::Preserve,
            CleanupTarget::ExpiredRegistered => CleanupAction::CleanupRegistered,
        }
    }
}
fn digest(v: &str) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in v.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3)
    }
    format!("{h:016x}")
}
