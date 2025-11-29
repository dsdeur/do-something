use anyhow::Result;
use git2::Repository;
use std::path;

pub fn git_root() -> Option<std::path::PathBuf> {
    let repo = Repository::discover(".").ok()?;
    repo.workdir().map(|p| p.to_path_buf())
}

pub fn resolve_path(input: &str) -> Result<std::path::PathBuf> {
    let expanded = shellexpand::tilde(input);
    let res = path::absolute(expanded.as_ref())?;
    Ok(res)
}
