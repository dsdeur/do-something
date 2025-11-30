use anyhow::Result;
use git2::Repository;
use std::path;

/// Find the current Git root directory
pub fn git_root() -> Option<std::path::PathBuf> {
    let repo = Repository::discover(".").ok()?;
    repo.workdir().map(|p| p.to_path_buf())
}

/// Resolve a given path, expanding `~` to the home directory and converting to an absolute path.
pub fn resolve_path(input: &str) -> Result<std::path::PathBuf> {
    let expanded = shellexpand::tilde(input);
    let res = path::absolute(expanded.as_ref())?;
    Ok(res)
}
