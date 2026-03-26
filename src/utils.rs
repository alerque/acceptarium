// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::{Path, PathBuf};

#[cfg(feature = "git")]
pub(crate) fn discover_project_root(cwd: &Path) -> PathBuf {
    use git2::Repository;
    let git_repo = Repository::discover(cwd).ok();
    let git_root = git_repo
        .as_ref()
        .and_then(|repo| repo.workdir().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(&cwd));
    walk_to_root_or_config(cwd, &git_root)
}

#[cfg(not(feature = "git"))]
pub(crate) fn discover_project_root(cwd: &Path) -> PathBuf {
    walk_to_root_or_config(cwd, &PathBuf::from("/"))
}

fn walk_to_root_or_config(cwd: &Path, root: &PathBuf) -> PathBuf {
    let mut current = cwd.to_path_buf();
    loop {
        let config = current.join("acceptarium.toml");
        if config.exists() {
            return current;
        }
        if current == *root {
            break;
        }
        if !current.pop() {
            break;
        }
    }
    root.clone()
}
