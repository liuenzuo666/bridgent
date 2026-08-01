use crate::project;
use anyhow::Result;
use std::path::Path;

pub struct DoctorReport {
    pub git_root: Option<String>,
    pub git_dir: Option<String>,
    pub exclude_path: Option<String>,
    pub exclude_writable: Option<bool>,
    pub symlink_ok: bool,
    pub platform_note: String,
}

pub fn doctor(start: &Path) -> Result<DoctorReport> {
    let mut r = DoctorReport {
        git_root: None,
        git_dir: None,
        exclude_path: None,
        exclude_writable: None,
        symlink_ok: true,
        platform_note: String::new(),
    };

    match project::find_git_root(start)? {
        Some(root) => {
            let (_root, gitdir, exclude) = project::resolve_git_dirs(&root)?;
            r.git_root = Some(root.display().to_string());
            r.git_dir = Some(gitdir.display().to_string());
            r.exclude_path = Some(exclude.display().to_string());
            // writable check
            let parent = exclude.parent().unwrap();
            r.exclude_writable = Some(parent.exists());
        }
        None => {
            #[cfg(windows)]
            {
                r.platform_note = "Windows: symlinks need Developer Mode or admin; dirs fall back to junctions".into();
            }
            return Ok(r);
        }
    }

    // Symlink capability probe in a temp dir inside the repo (cleaned up after).
    let root = Path::new(r.git_root.as_ref().unwrap());
    let probe_dir = root.join(".bridgent-probe-XXXXXX");
    let probe_dir = mkdtemp_like(&probe_dir);
    if let Some(dir) = probe_dir {
        let link = dir.join("probe");
        match crate::fsutil::create_link(&link, Path::new("probe-target")) {
            Ok(()) => {
                let _ = std::fs::remove_file(&link);
                r.symlink_ok = true;
            }
            Err(e) => {
                r.symlink_ok = false;
                r.platform_note = format!("symlink creation failed: {}", e);
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
    Ok(r)
}

fn mkdtemp_like(template: &Path) -> Option<std::path::PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    for _ in 0..100 {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = template.with_extension(format!("{}-{}", std::process::id(), n));
        match std::fs::create_dir(&dir) {
            Ok(()) => return Some(dir),
            Err(_) => continue,
        }
    }
    None
}
