use crate::exclude;
use crate::fsutil;
use crate::project;
use anyhow::{bail, Context, Result};
use std::path::Path;

pub struct LinkOpts {
    pub yes: bool,
    pub no_exclude: bool,
    pub dry_run: bool,
    pub force: bool,
    pub kind: Kind,
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Kind {
    Dir,
    File,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Dir => "dir",
            Kind::File => "file",
        }
    }
}

/// Core `link` operation. Idempotent.
/// Returns a human-readable description of what was done.
pub fn link(root: &Path, exclude_path: &Path, rel: &str, target_raw: &str, opts: &LinkOpts) -> Result<String> {
    let path = project::repo_path(root, rel)?;
    let target = project::resolve_target(root, target_raw)?;
    let rel_norm = rel.trim_start_matches("./").trim_start_matches('/');

    // ----- Case: path already exists -----
    let md = std::fs::symlink_metadata(&path);
    match md {
        Ok(m) if m.file_type().is_symlink() => {
            let current = fsutil::read_link(&path)?;
            let current_abs = if current.is_absolute() {
                project::lexical_normalize(&current)
            } else {
                project::lexical_normalize(&path.parent().unwrap_or(root).join(&current))
            };
            if current_abs == target {
                let mut msg = format!("link {} -> {} already correct", path.display(), target.display());
                if !opts.no_exclude
                    && exclude::add_entry(exclude_path, rel_norm)? {
                        msg.push_str("; exclude entry added");
                    }
                return Ok(msg);
            }
            if !opts.force {
                bail!(
                    "{} is a symlink pointing to `{}`, not `{}`. Use --force to replace it.",
                    path.display(),
                    current_abs.display(),
                    target.display()
                );
            }
            eprintln!("info: replacing symlink {} (was -> {})", path.display(), current_abs.display());
            do_remove_link(&path, opts)?;
        }
        Ok(m) => {
            // Real file or directory exists at the link location.
            let is_dir = m.is_dir();
            if is_dir && is_empty_dir(&path)? {
                eprintln!("info: removing empty directory {}", path.display());
                if !opts.dry_run {
                    std::fs::remove_dir(&path)
                        .with_context(|| format!("cannot remove empty dir {}", path.display()))?;
                }
            } else if !target_exists(&target)? {
                // Non-empty real object; target free. Move it into place.
                if !opts.yes && !confirm(&format!(
                    "{} exists and is not a symlink. Move it to {} (data is preserved)?",
                    path.display(),
                    target.display()
                ))? {
                    bail!("aborted by user");
                }
                eprintln!("info: moving {} -> {}", path.display(), target.display());
                if !opts.dry_run {
                    fsutil::move_path(&path, &target)?;
                }
            } else {
                bail!(
                    "conflict: {} is a real {} and {} already exists. Refusing to touch either. \
                     Move one aside manually, then retry.",
                    path.display(),
                    if is_dir { "directory" } else { "file" },
                    target.display()
                );
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // ----- Case: path does not exist -----
            if !target_exists(&target)? {
                eprintln!("info: creating {} ({})", target.display(), opts.kind.as_str());
                if !opts.dry_run {
                    create_target(&target, opts.kind)?;
                }
            }
        }
        Err(e) => bail!("cannot stat {}: {}", path.display(), e),
    }

    // ----- Create the link -----
    if !opts.dry_run {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                // e.g. linking `.github/copilot-instructions.md` when `.github` is absent
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("cannot create {}", parent.display()))?;
            }
        }
        fsutil::create_link(&path, &target)
            .with_context(|| format!("cannot link {} -> {}", path.display(), target.display()))?;
    }

    // ----- Exclude -----
    let mut msg = format!("linked {} -> {}", path.display(), target.display());
    if opts.no_exclude {
        msg.push_str(" (exclude skipped)");
    } else if exclude::add_entry(exclude_path, rel_norm)? {
        msg.push_str(&format!("; excluded `{}`", rel_norm));
    } else {
        msg.push_str("; exclude entry already present");
    }
    Ok(msg)
}

/// Core `unlink` operation. Data at the target is preserved unless `purge`.
pub fn unlink(root: &Path, exclude_path: &Path, rel: &str, purge: bool, yes: bool, dry_run: bool) -> Result<String> {
    let path = project::repo_path(root, rel)?;
    let md = std::fs::symlink_metadata(&path)
        .with_context(|| format!("{} does not exist", path.display()))?;
    if !md.file_type().is_symlink() {
        bail!(
            "{} is a real {}; bridgent never removes real data. Move/delete it manually.",
            path.display(),
            if md.is_dir() { "directory" } else { "file" }
        );
    }
    let target = fsutil::read_link(&path)?;
    let target_abs = if target.is_absolute() {
        project::lexical_normalize(&target)
    } else {
        project::lexical_normalize(&path.parent().unwrap_or(root).join(&target))
    };

    let mut msg = format!("removed link {} (target `{}` preserved)", path.display(), target_abs.display());

    if purge {
        if !yes && !confirm(&format!(
            "--purge: also delete the target data at {}? This cannot be undone.",
            target_abs.display()
        ))? {
            bail!("aborted by user");
        }
        if !dry_run {
            fsutil::remove_tree(&target_abs)
                .with_context(|| format!("cannot remove {}", target_abs.display()))?;
        }
        msg.push_str(&format!("; purged target {}", target_abs.display()));
    }

    if !dry_run {
        fsutil::remove_link(&path)?;
        if exclude::remove_entry(exclude_path, rel.trim_start_matches("./").trim_start_matches('/'))? {
            msg.push_str("; exclude entry removed");
        }
    }
    Ok(msg)
}

fn do_remove_link(path: &Path, opts: &LinkOpts) -> Result<()> {
    if !opts.dry_run {
        fsutil::remove_link(path)?;
    }
    Ok(())
}

fn create_target(target: &Path, kind: Kind) -> Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    match kind {
        Kind::Dir => std::fs::create_dir_all(target)
            .with_context(|| format!("cannot create {}", target.display())),
        Kind::File => {
            std::fs::File::create(target)
                .with_context(|| format!("cannot create {}", target.display()))?;
            Ok(())
        }
    }
}

fn target_exists(target: &Path) -> Result<bool> {
    match std::fs::symlink_metadata(target) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => bail!("cannot stat {}: {}", target.display(), e),
    }
}

fn is_empty_dir(p: &Path) -> Result<bool> {
    let mut it = std::fs::read_dir(p)?;
    Ok(it.next().is_none())
}

pub fn confirm(prompt: &str) -> Result<bool> {
    use std::io::Write;
    print!("{} [y/N] ", prompt);
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(matches!(line.trim().to_lowercase().as_str(), "y" | "yes"))
}
