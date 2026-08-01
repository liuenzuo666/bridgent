use crate::exclude;
use crate::fsutil;
use crate::project;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Clone)]
pub struct PathStatus {
    pub path: String,
    pub kind: String, // "symlink" | "dangling" | "real_dir" | "real_file" | "missing" | "conflict"
    pub target: Option<String>,
    pub excluded: bool,
    pub exclude_marked: bool,
    pub note: Option<String>,
}

/// Status for a single repo-relative path.
pub fn path_status(root: &Path, exclude_path: &Path, rel: &str) -> Result<PathStatus> {
    let path = project::repo_path(root, rel)?;
    let (excluded, marked) = exclude::entry_state(exclude_path, rel)?;
    let mut st = PathStatus {
        path: rel.to_string(),
        kind: "missing".into(),
        target: None,
        excluded,
        exclude_marked: marked,
        note: None,
    };
    match std::fs::symlink_metadata(&path) {
        Ok(m) if m.file_type().is_symlink() => {
            let t = fsutil::read_link(&path)?;
            let t_abs = if t.is_absolute() {
                project::lexical_normalize(&t)
            } else {
                project::lexical_normalize(&path.parent().unwrap_or(root).join(&t))
            };
            let dangling = !t_abs.exists();
            st.kind = if dangling { "dangling".into() } else { "symlink".into() };
            st.target = Some(t_abs.display().to_string());
            if dangling {
                st.note = Some("target does not exist (dangling)".into());
            }
        }
        Ok(m) => {
            st.kind = if m.is_dir() { "real_dir".into() } else { "real_file".into() };
            st.note = Some("not a symlink; bridgent will not touch it".into());
        }
        Err(_) => {
            if st.excluded {
                st.note = Some("excluded but missing (link broken?)".into());
            }
        }
    }
    Ok(st)
}

/// Scan the repo root's first level for symlinks (without assuming which
/// paths are managed by bridgent).
pub fn scan_links(root: &Path, exclude_path: &Path) -> Result<Vec<PathStatus>> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(root)?;
    for e in entries {
        let e = e?;
        if !fsutil::is_link(&e.path()) {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        out.push(path_status(root, exclude_path, &name)?);
    }
    Ok(out)
}
