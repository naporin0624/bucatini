use std::{
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use clap::{Parser, Subcommand};

mod license_entry;
mod merge;
mod normalize;
mod render;
mod sort;

/// Kept in one place so the install hint below (`XtaskError::Spawn`) can't drift from
/// what CI actually installs (`.github/workflows/ci.yml`'s own `cargo install` step).
const CARGO_BUNDLE_LICENSES_VERSION: &str = "4.2.0";

#[derive(Debug, thiserror::Error)]
enum XtaskError {
    #[error(
        "failed to spawn `{command}`: {source}\n\
         install it with: cargo install cargo-bundle-licenses --version {version} --locked",
        version = CARGO_BUNDLE_LICENSES_VERSION
    )]
    Spawn {
        command: String,
        source: std::io::Error,
    },
    #[error("`{command}` exited with an error:\n{stderr}")]
    Subprocess { command: String, stderr: String },
    #[error("`{command}` output was not valid UTF-8: {source}")]
    Utf8 {
        command: String,
        source: std::string::FromUtf8Error,
    },
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error at {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error(
        "license artifact is stale: {0} does not match freshly generated output; run `cargo xtask gen-licenses`"
    )]
    Stale(String),
}

#[derive(Parser)]
#[command(name = "xtask")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Regenerate assets/third-party-licenses.json and THIRD-PARTY-NOTICES.
    GenLicenses {
        /// Regenerate into memory and byte-compare against the committed files instead of
        /// writing them; exits nonzero if either file is stale.
        #[arg(long)]
        check: bool,
    },
}

struct Artifacts {
    json: String,
    notices: String,
}

fn project_root() -> PathBuf {
    // xtask -> repo root. CARGO_MANIFEST_DIR is resolved at compile time, so this works
    // regardless of the caller's current directory.
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root
}

fn run_cargo_bundle_licenses() -> Result<normalize::CargoBundleOutput, XtaskError> {
    let command = "cargo bundle-licenses";
    // `--features gui` pulls in eframe/tray-icon and their transitive deps: the GUI binary
    // is what actually renders this manifest in its licenses viewer, so its dependency set
    // must be covered, not just the CLI's smaller default-feature set.
    let output = Command::new("cargo")
        .args(["bundle-licenses", "--format", "json", "--features", "gui"])
        .output()
        .map_err(|source| XtaskError::Spawn {
            command: command.to_string(),
            source,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(XtaskError::Subprocess {
            command: command.to_string(),
            stderr,
        });
    }

    let stdout = String::from_utf8(output.stdout).map_err(|source| XtaskError::Utf8 {
        command: command.to_string(),
        source,
    })?;
    serde_json::from_str(&stdout).map_err(XtaskError::Json)
}

fn read_appendix(root: &Path) -> Result<Vec<license_entry::LicenseEntry>, XtaskError> {
    let path = root.join("licenses/appendix.json");
    let raw = std::fs::read_to_string(&path).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(XtaskError::Json)
}

fn build_artifacts(root: &Path) -> Result<Artifacts, XtaskError> {
    let scanned = run_cargo_bundle_licenses()?;
    let scanner_entries = normalize::normalize(scanned);
    let appendix_entries = read_appendix(root)?;
    let merged = merge::merge(scanner_entries, appendix_entries);
    let sorted = sort::sort_entries(merged);

    let mut json = serde_json::to_string_pretty(&sorted).map_err(XtaskError::Json)?;
    json.push('\n');
    let notices = render::render_notices(&sorted);

    Ok(Artifacts { json, notices })
}

fn check_matches(path: &Path, expected: &str) -> Result<(), XtaskError> {
    let actual = std::fs::read_to_string(path).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })?;
    if actual != expected {
        return Err(XtaskError::Stale(path.display().to_string()));
    }
    Ok(())
}

fn gen_licenses(check: bool) -> Result<(), XtaskError> {
    let root = project_root();
    let artifacts = build_artifacts(&root)?;
    let assets_path = root.join("assets/third-party-licenses.json");
    let notices_path = root.join("THIRD-PARTY-NOTICES");

    if check {
        check_matches(&assets_path, &artifacts.json)?;
        check_matches(&notices_path, &artifacts.notices)?;
        println!("license artifacts are up to date");
        return Ok(());
    }

    if let Some(parent) = assets_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| XtaskError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }
    std::fs::write(&assets_path, &artifacts.json).map_err(|source| XtaskError::Io {
        path: assets_path.display().to_string(),
        source,
    })?;
    std::fs::write(&notices_path, &artifacts.notices).map_err(|source| XtaskError::Io {
        path: notices_path.display().to_string(),
        source,
    })?;
    println!("wrote {}", assets_path.display());
    println!("wrote {}", notices_path.display());
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::GenLicenses { check } => gen_licenses(check),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
