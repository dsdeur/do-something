use anyhow::Result;
use git2::Repository;

use std::env;
use std::path::{self, Path, PathBuf};

/// Find the current Git root directory
pub fn git_root() -> Option<PathBuf> {
    let repo = Repository::discover(".").ok()?;
    repo.workdir().map(|p| p.to_path_buf())
}

/// Resolve a given path, expanding `~` to the home directory and converting to an absolute path.
pub fn resolve_path(input: &str) -> Result<PathBuf> {
    let expanded = shellexpand::tilde(input);
    let res = path::absolute(expanded.as_ref())?;
    Ok(res)
}

/// Collapse a path to use `~` for the home directory if applicable
pub fn collapse_to_tilde(path: &Path) -> String {
    if let Some(home) = env::home_dir()
        && path.starts_with(&home)
    {
        let rest = path.strip_prefix(&home).unwrap();
        return format!("~/{}", rest.display());
    }
    path.display().to_string()
}
