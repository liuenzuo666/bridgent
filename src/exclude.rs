use anyhow::{Context, Result};
use std::path::Path;

/// Marker comment prefix for entries written by bridgent.
const MARKER: &str = "# bridgent: ";

/// Ensure an exclude entry `/<path>` exists at the end of the file,
/// preceded by its marker comment. Idempotent: if an entry for the path
/// already exists (with or without marker), nothing is appended.
/// Preserves all existing content byte-for-byte.
pub fn add_entry(exclude_path: &Path, rel: &str) -> Result<bool> {
    let content = read_or_empty(exclude_path)?;
    let entry = format!("/{}", rel.trim_start_matches('/'));
    let already = content.lines().any(|l| {
        let t = l.trim_end();
        t == entry || t == format!("{}/", entry)
    });
    if already {
        return Ok(false);
    }
    let mut out = content;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!("{}{}\n{}\n", MARKER, rel, entry));
    write(exclude_path, &out)?;
    Ok(true)
}

/// Remove the marker comment and the entry for `rel` if both exist.
/// User-written entries (no marker) are never removed.
/// Returns true if anything was removed.
pub fn remove_entry(exclude_path: &Path, rel: &str) -> Result<bool> {
    let content = read_or_empty(exclude_path)?;
    let entry = format!("/{}", rel.trim_start_matches('/'));
    let marker = format!("{}{}", MARKER, rel);
    let lines: Vec<&str> = content.lines().collect();
    let mut kept: Vec<&str> = Vec::with_capacity(lines.len());
    let mut removed = false;
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if l.trim_end() == marker || l.trim_end() == entry {
            removed = true;
            i += 1;
            continue;
        }
        kept.push(l);
        i += 1;
    }
    if !removed {
        return Ok(false);
    }
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    write(exclude_path, &out)?;
    Ok(true)
}

/// Query the state of an entry in the exclude file.
/// Returns (present, marked_by_bridgent).
pub fn entry_state(exclude_path: &Path, rel: &str) -> Result<(bool, bool)> {
    let content = read_or_empty(exclude_path)?;
    let entry = format!("/{}", rel.trim_start_matches('/'));
    let marker = format!("{}{}", MARKER, rel);
    let mut present = false;
    let mut marked = false;
    for l in content.lines() {
        let t = l.trim_end();
        if t == entry {
            present = true;
        }
        if t == marker {
            marked = true;
        }
    }
    Ok((present, marked))
}

fn read_or_empty(p: &Path) -> Result<String> {
    match std::fs::read_to_string(p) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e).with_context(|| format!("cannot read {}", p.display())),
    }
}

fn write(p: &Path, content: &str) -> Result<()> {
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    std::fs::write(p, content).with_context(|| format!("cannot write {}", p.display()))
}
