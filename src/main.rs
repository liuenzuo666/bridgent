mod doctor;
mod exclude;
mod fsutil;
mod link;
mod project;
mod status;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "bridgent",
    version,
    about = "Stateless bridge for agent config dirs: symlink + git exclude, no config files, no assumptions"
)]
struct Cli {
    /// Repo root; defaults to walking up from cwd to find `.git`
    #[arg(long, global = true)]
    project_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Link a repo path to a managed target: move data (if any), create symlink, write exclude
    Link {
        /// Repo-relative path to manage (e.g. `.pi`, `.github/copilot-instructions.md`)
        path: String,
        /// Where the data lives (absolute, or relative to repo root; `~` ok)
        #[arg(long)]
        to: String,
        /// Skip writing to .git/info/exclude
        #[arg(long)]
        no_exclude: bool,
        /// Overwrite an existing symlink pointing elsewhere
        #[arg(long)]
        force: bool,
        /// Type of the managed object when it must be created fresh (dir|file)
        #[arg(long, value_enum, default_value_t = link::Kind::Dir)]
        kind: link::Kind,
        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
        /// Show what would happen without changing anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove the symlink (target data preserved); --purge deletes target data too
    Unlink {
        /// Repo-relative path of the symlink to remove
        path: String,
        /// Also delete the data at the link target (double confirmation unless --yes)
        #[arg(long)]
        purge: bool,
        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
        /// Show what would happen without changing anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Report state of one or more paths (default: all top-level symlinks)
    Status {
        /// Paths to inspect; empty = scan repo root for symlinks
        paths: Vec<String>,
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Environment checks: git repo form, symlink capability, exclude writability
    Doctor,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let start = cli.project_dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let start = start.canonicalize().unwrap_or(start);

    match &cli.command {
        Commands::Link { path, to, no_exclude, force, yes, dry_run, kind } => {
            // link can work without a git repo when --no-exclude is given (cwd becomes the root)
            let (root, exclude) = match project::find_git_root(&start)? {
                Some(root) => {
                    let (_r, _g, ex) = project::resolve_git_dirs(&root)?;
                    (root, ex)
                }
                None if *no_exclude => {
                    eprintln!("warning: no git repo found; linking without exclude");
                    (start.clone(), PathBuf::new())
                }
                None => bail!(
                    "no git repository found from `{}` (use --project-dir, or --no-exclude to link without exclude)",
                    start.display()
                ),
            };
            if *dry_run {
                eprintln!("info: dry run, nothing will be written");
            }
            let opts = link::LinkOpts { yes: *yes, no_exclude: *no_exclude, dry_run: *dry_run, force: *force, kind: *kind };
            let msg = link::link(&root, &exclude, path, to, &opts)?;
            println!("{}", msg);
        }
        Commands::Unlink { path, purge, yes, dry_run } => {
            let (root, _gitdir, exclude) = require_git(&start)?;
            if *dry_run {
                eprintln!("info: dry run, nothing will be written");
            }
            let msg = link::unlink(&root, &exclude, path, *purge, *yes, *dry_run)?;
            println!("{}", msg);
        }
        Commands::Status { paths, json } => {
            let (root, _gitdir, exclude) = require_git(&start)?;
            let items: Vec<status::PathStatus> = if paths.is_empty() {
                status::scan_links(&root, &exclude)?
            } else {
                let mut v = Vec::new();
                for p in paths {
                    v.push(status::path_status(&root, &exclude, p)?);
                }
                v
            };
            if *json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                if items.is_empty() {
                    println!("no symlinks at repo root (nothing managed by bridgent)");
                    return Ok(());
                }
                for s in &items {
                    let excl = match (s.excluded, s.exclude_marked) {
                        (true, true) => "excluded (bridgent)",
                        (true, false) => "excluded (manual)",
                        (false, _) => "NOT excluded",
                    };
                    let target = s.target.as_deref().unwrap_or("-");
                    let note = s.note.as_deref().unwrap_or("");
                    println!("{:<10} {:<10} -> {:<40} {} {}", s.path, s.kind, target, excl, note);
                }
            }
        }
        Commands::Doctor => {
            let r = doctor::doctor(&start)?;
            match (&r.git_root, &r.exclude_writable) {
                (Some(root), Some(w)) => {
                    println!("repo root:   {}", root);
                    println!("git dir:     {}", r.git_dir.as_deref().unwrap_or("-"));
                    println!("exclude:     {}", r.exclude_path.as_deref().unwrap_or("-"));
                    println!("exclude dir: {}", if *w { "present" } else { "MISSING (git repo incomplete?)" });
                }
                _ => {
                    println!("no git repository found from {}", start.display());
                    println!("note: link/unlink/status need a git repo (unless --no-exclude is used)");
                }
            }
            println!("symlinks:    {}", if r.symlink_ok { "OK" } else { "FAILED" });
            if !r.platform_note.is_empty() {
                println!("note: {}", r.platform_note);
            }
        }
    }
    Ok(())
}

fn require_git(start: &std::path::Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let root = project::find_git_root(start)?
        .with_context(|| format!("no git repository found from `{}` (use --project-dir, or link with --no-exclude in a non-git dir)", start.display()))?;
    project::resolve_git_dirs(&root)
}
