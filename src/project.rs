use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Locate the repository root by walking up from `start` looking for `.git`.
pub fn find_git_root(start: &Path) -> Result<Option<PathBuf>> {
    let mut dir = Some(start.to_path_buf());
    while let Some(d) = dir {
        if d.join(".git").exists() {
            return Ok(Some(d));
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    Ok(None)
}

/// Resolve the effective git dir (handles worktrees where `.git` is a file).
/// Returns (repo_root, git_dir, exclude_path).
pub fn resolve_git_dirs(root: &Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let git_path = root.join(".git");
    if git_path.is_dir() {
        let exclude = git_path.join("info").join("exclude");
        return Ok((root.to_path_buf(), git_path, exclude));
    }
    if git_path.is_file() {
        // Worktree: `.git` contains "gitdir: <path>"
        let content = std::fs::read_to_string(&git_path)
            .with_context(|| format!("cannot read {}", git_path.display()))?;
        let line = content.lines().find_map(|l| l.strip_prefix("gitdir:"));
        let Some(raw) = line else {
            bail!("{} is a file but has no `gitdir:` line", git_path.display());
        };
        let gitdir = expand_tilde(Path::new(raw.trim()));
        let gitdir = if gitdir.is_absolute() {
            gitdir
        } else {
            root.join(gitdir)
        };
        let exclude = gitdir.join("info").join("exclude");
        return Ok((root.to_path_buf(), gitdir, exclude));
    }
    bail!("{} is neither a directory nor a gitfile", git_path.display())
}

/// Normalize a repo-relative path (e.g. `.pi`, `.github/x.md`).
/// Returns an absolute path, verifying it stays inside `root`.
pub fn repo_path(root: &Path, raw: &str) -> Result<PathBuf> {
    let p = expand_tilde(Path::new(raw));
    let p_abs = p.is_absolute();
    let full = if p_abs { p.clone() } else { root.join(&p) };
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    // Lexical containment check: strip root prefix and ensure no ".." escapes.
    let rel = full.strip_prefix(&canonical_root).map_err(|_| {
        anyhow::anyhow!("path `{}` is outside the repository root `{}`", raw, root.display())
    })?;
    if rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        bail!("path `{}` escapes the repository root via `..`", raw);
    }
    // Preserve the original (non-canonicalized) form for symlink creation.
    Ok(if p_abs { p } else { root.join(p) })
}

/// Resolve a target (--to) path. Relative paths are relative to `root`.
/// `~` is expanded to home. Returns the path with `..` normalized lexically
/// (the target itself does not need to exist yet).
pub fn resolve_target(root: &Path, raw: &str) -> Result<PathBuf> {
    let p = expand_tilde(Path::new(raw));
    let abs = if p.is_absolute() { p } else { root.join(p) };
    Ok(lexical_normalize(&abs))
}

pub fn expand_tilde(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if s == "~" {
        return home_dir().unwrap_or_else(|| p.to_path_buf());
    }
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    p.to_path_buf()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Lexically normalize a path (resolve `.` and `..` without touching the FS).
pub fn lexical_normalize(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out: Vec<Component> = Vec::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if out.last().is_some_and(|l| *l != Component::ParentDir) {
                    out.pop();
                } else {
                    out.push(c);
                }
            }
            other => out.push(other),
        }
    }
    out.iter().collect()
}
