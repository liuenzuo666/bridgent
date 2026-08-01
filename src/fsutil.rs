use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Create a link at `link_path` pointing to `target`.
/// On Unix: symlink. On Windows: symlink, falling back to junction for dirs.
pub fn create_link(link_path: &Path, target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)
            .with_context(|| format!("cannot create symlink {} -> {}", link_path.display(), target.display()))?;
    }
    #[cfg(windows)]
    {
        let is_dir = target.is_dir();
        if let Err(e) = std::os::windows::fs::symlink_dir(target, link_path) {
            if is_dir {
                // Fallback: NTFS junction via mklink /J
                let status = std::process::Command::new("cmd")
                    .args(["/C", "mklink", "/J"])
                    .arg(link_path)
                    .arg(target)
                    .status();
                match status {
                    Ok(s) if s.success() => {}
                    _ => {
                        bail!(
                            "cannot create symlink {} -> {} (tried symlink and junction): {}",
                            link_path.display(),
                            target.display(),
                            e
                        )
                    }
                }
            } else {
                bail!(
                    "cannot create symlink {} -> {}: {}",
                    link_path.display(),
                    target.display(),
                    e
                )
            }
        }
    }
    Ok(())
}

/// Remove a link (symlink or junction). Safe: only removes the link itself,
/// never follows it. Returns false if the path was already gone.
pub fn remove_link(link_path: &Path) -> Result<bool> {
    let md = match std::fs::symlink_metadata(link_path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => bail!("cannot stat {}: {}", link_path.display(), e),
    };
    let res = if md.file_type().is_symlink() {
        #[cfg(windows)]
        {
            // On Windows, dir symlinks/junctions must be removed with remove_dir.
            let is_dir = md.file_type().is_dir(); // symlink_metadata: is_dir follows? No, it does not follow.
            let is_dir = std::fs::metadata(link_path).map(|m| m.is_dir()).unwrap_or(is_dir);
            if is_dir {
                std::fs::remove_dir(link_path)
            } else {
                std::fs::remove_file(link_path)
            }
        }
        #[cfg(unix)]
        {
            std::fs::remove_file(link_path)
        }
    } else {
        bail!("refusing to remove `{}`: not a symlink", link_path.display())
    };
    res.with_context(|| format!("cannot remove link {}", link_path.display()))?;
    Ok(true)
}

/// Read the target of a symlink without following it.
pub fn read_link(link_path: &Path) -> Result<PathBuf> {
    std::fs::read_link(link_path)
        .with_context(|| format!("cannot read link {}", link_path.display()))
}

/// Is `p` a symlink (not following)?
pub fn is_link(p: &Path) -> bool {
    std::fs::symlink_metadata(p)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Move `src` to `dst`. Prefers rename; falls back to copy+verify+remove
/// when crossing filesystems (EXDEV). Creates parent dirs of `dst`.
pub fn move_path(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create {}", parent.display()))?;
        }
    }
    match std::fs::rename(src, dst) {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {}
        Err(e) => bail!("cannot move {} -> {}: {}", src.display(), dst.display(), e),
    }
    // Cross-device fallback: copy then verify then remove source.
    eprintln!(
        "info: `{}` is on another filesystem, copying instead of renaming",
        src.display()
    );
    copy_tree(src, dst).with_context(|| format!("copy {} -> {} failed", src.display(), dst.display()))?;
    verify_tree(src, dst)?;
    remove_tree(src)?;
    Ok(())
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    let md = std::fs::symlink_metadata(src)?;
    if md.file_type().is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_tree(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else if md.file_type().is_symlink() {
        let target = std::fs::read_link(src)?;
        std::os::unix::fs::symlink(&target, dst)?;
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

fn verify_tree(src: &Path, dst: &Path) -> Result<()> {
    let md = std::fs::symlink_metadata(src)?;
    if md.file_type().is_dir() {
        let src_entries: Vec<_> = std::fs::read_dir(src)?.collect::<std::io::Result<_>>()?;
        let dst_entries: Vec<_> = std::fs::read_dir(dst)?.collect::<std::io::Result<_>>()?;
        if src_entries.len() != dst_entries.len() {
            bail!("verify failed: entry count mismatch for {}", src.display());
        }
        for e in src_entries {
            verify_tree(&e.path(), &dst.join(e.file_name()))?;
        }
    } else if md.file_type().is_symlink() {
        if std::fs::read_link(dst)? != std::fs::read_link(src)? {
            bail!("verify failed: symlink mismatch for {}", src.display());
        }
    } else if std::fs::metadata(src)?.len() != std::fs::metadata(dst)?.len() {
        bail!("verify failed: size mismatch for {}", src.display());
    }
    Ok(())
}

pub fn remove_tree(p: &Path) -> Result<()> {
    let md = std::fs::symlink_metadata(p)?;
    if md.file_type().is_dir() && !md.file_type().is_symlink() {
        std::fs::remove_dir_all(p)
            .with_context(|| format!("cannot remove {}", p.display()))?;
    } else {
        std::fs::remove_file(p)
            .with_context(|| format!("cannot remove {}", p.display()))?;
    }
    Ok(())
}
