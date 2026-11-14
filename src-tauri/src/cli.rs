//! `noground` on the command line — the app's pipeline, driven by args.
//!
//! The macOS bundle ships a single binary. Settings installs a symlink
//! `~/.local/bin/noground` → the binary inside Noground.app, so invoking the
//! CLI is just the app binary running under a different name. The dispatch
//! reads argv[0]: launched from inside a `.app` bundle it's the app, launched
//! through the (lowercase-named) symlink it's the CLI — no second bundled
//! binary, which Tauri can't ship in a universal macOS build anyway. Dev
//! builds only enter CLI mode via an explicit `cli` first argument, since
//! `tauri dev` runs the same binary name bare.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};

use crate::{
    imaging,
    models,
    pipeline,
    session::{LoadedSession, SessionCache},
    settings::{OutputFormat, Settings},
};

/// Name of the symlink Settings installs — and therefore of the CLI on $PATH.
pub const LINK_NAME: &str = "noground";

/// True when this process was launched as the CLI (see module docs).
pub fn should_run_cli() -> bool {
    if std::env::args_os().nth(1).is_some_and(|a| a == "cli") {
        return true;
    }
    // The cargo binary is `noground` in both the bundle and the symlink, so
    // the basename alone can't tell the two launches apart — where argv[0]
    // lives can: LaunchServices starts the app from inside its .app bundle,
    // the installed symlink lives outside it. Debug builds are always GUI
    // unless told otherwise — `tauri dev` would otherwise launch into CLI
    // mode and exit.
    #[cfg(not(debug_assertions))]
    {
        let argv0 = std::env::args_os()
            .next()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let named_as_cli = Path::new(&argv0).file_name() == Some(Path::new(LINK_NAME).file_name().unwrap());
        let launched_from_bundle = argv0.contains(".app/contents/macos");
        if named_as_cli && !launched_from_bundle {
            return true;
        }
    }
    false
}

const HELP: &str = "\
Noground — local background removal. Drop-in replacement for the app: same
model, same output naming (`photo.jpg` → `photo-noground.png`).

USAGE:
    noground [OPTIONS] <PATH>...

ARGS:
    <PATH>...    Image files or folders (folders are not recursed into)

OPTIONS:
    -o, --out <DIR>      Write results here (default: next to each source image)
    -m, --model <KEY>    bria-rmbg | birefnet-portrait | isnet-general-use | u2net
                         (default: the model selected in the app)
    -f, --format <FMT>   png | jpeg (default: the app's output format)
        --overwrite      Re-process images whose results already exist
        --list-models    List the models and whether they're installed
    -q, --quiet          Print only failures and the summary
    -h, --help           Print this help
    -V, --version        Print version
";

struct Args {
    inputs: Vec<String>,
    out_dir: Option<String>,
    model: Option<String>,
    format: Option<OutputFormat>,
    overwrite: bool,
    quiet: bool,
    list_models: bool,
}

enum Parsed {
    Run(Args),
    Help,
    Version,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Parsed, String> {
    let _argv0 = args.next();
    let mut rest: Vec<String> = args.collect();
    // tolerate `noground cli …` — the explicit dev-only dispatch form
    if rest.first().map(String::as_str) == Some("cli") {
        rest.remove(0);
    }

    let mut parsed = Args {
        inputs: Vec::new(),
        out_dir: None,
        model: None,
        format: None,
        overwrite: false,
        quiet: false,
        list_models: false,
    };

    let mut values = rest.into_iter();
    while let Some(arg) = values.next() {
        let mut next_value =
            |flag: &str| values.next().ok_or_else(|| format!("{flag} needs a value"));
        match arg.as_str() {
            "-h" | "--help" => return Ok(Parsed::Help),
            "-V" | "--version" => return Ok(Parsed::Version),
            "-o" | "--out" => parsed.out_dir = Some(next_value("--out")?),
            "-m" | "--model" => parsed.model = Some(next_value("--model")?),
            "-f" | "--format" => {
                let value = next_value("--format")?;
                parsed.format = Some(match value.as_str() {
                    "png" => OutputFormat::Png,
                    "jpeg" | "jpg" => OutputFormat::Jpeg,
                    _ => return Err(format!("unknown format “{value}” — use png or jpeg")),
                });
            }
            "--overwrite" => parsed.overwrite = true,
            "-q" | "--quiet" => parsed.quiet = true,
            "--list-models" => parsed.list_models = true,
            _ if arg.starts_with('-') => return Err(format!("unknown option “{arg}”")),
            _ => parsed.inputs.push(arg),
        }
    }
    Ok(Parsed::Run(parsed))
}

/// Entry point for CLI mode. Returns the process exit code.
pub fn run() -> i32 {
    let args = match parse_args(std::env::args()) {
        Ok(Parsed::Help) => {
            print!("{HELP}");
            return 0;
        }
        Ok(Parsed::Version) => {
            println!("noground {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        Ok(Parsed::Run(args)) => args,
        Err(usage) => {
            eprintln!("noground: {usage}\n\nTry `noground --help`.");
            return 2;
        }
    };

    match run_job(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("noground: {e:#}");
            1
        }
    }
}

fn run_job(args: Args) -> Result<i32> {
    if args.list_models {
        for m in models::MODELS {
            let installed = models::model_candidates(m.key)
                .into_iter()
                .any(|p| p.exists());
            println!(
                "{:<20} {:<9} {}",
                m.key,
                if installed { "installed" } else { "not installed" },
                m.label,
            );
        }
        return Ok(0);
    }
    if args.inputs.is_empty() {
        return Err(anyhow!("no input files given"));
    }

    // Start from the app's settings so a bare `noground photo.jpg` behaves
    // exactly like dropping the file in the app, then apply flag overrides.
    let app_settings = app_settings();
    let model_key = args.model.unwrap_or_else(|| app_settings.model.clone());
    let spec = models::spec(&model_key)?;
    let Some(model_path) = models::model_candidates(spec.key).into_iter().find(|p| p.exists())
    else {
        return Err(anyhow!(
            "model “{}” is not installed — download it in the Noground app \
             (Settings → Model) or pick another with --model",
            spec.label
        ));
    };

    let files = pipeline::expand_inputs(&args.inputs)?;
    if files.is_empty() {
        return Err(anyhow!("no images found in the given paths"));
    }

    let settings = Settings {
        model: model_key,
        output_dir: args.out_dir.clone(),
        overwrite: args.overwrite,
        format: args.format.unwrap_or(app_settings.format),
    };

    // A single dropped folder gets a sibling `<folder>-noground` dir in the
    // app; the CLI keeps that parity unless --out says otherwise.
    let batch_out_dir = if args.out_dir.is_none() {
        single_dir_out(&args.inputs)
    } else {
        None
    };

    if !args.quiet {
        eprintln!("loading {} …", spec.label);
    }
    let cache = SessionCache::default();
    let loaded = cache.get_or_load(
        spec.key,
        &model_path,
        models::is_managed_path(&model_path).then_some(spec),
    )?;

    let total = files.len();
    let mut done = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    for (index, input) in files.iter().enumerate() {
        let out_path = match pipeline::compute_output_path(input, &batch_out_dir, &settings) {
            Ok(p) => p,
            Err(message) => {
                failed += 1;
                eprintln!("noground: {}: {message}", input.display());
                continue;
            }
        };
        if !settings.overwrite && out_path.exists() {
            skipped += 1;
            if !args.quiet {
                eprintln!("[{}/{}] {} (already exists, skipped)", index + 1, total, out_path.display());
            }
            continue;
        }
        match process_one(&loaded, spec, input, &out_path, settings.format) {
            Ok(()) => {
                done += 1;
                if !args.quiet {
                    eprintln!("[{}/{}] {} → {}", index + 1, total, input.display(), out_path.display());
                }
            }
            Err(message) => {
                failed += 1;
                eprintln!("noground: {}: {message}", input.display());
            }
        }
    }

    let mut parts = Vec::new();
    if done > 0 {
        parts.push(format!("{done} done"));
    }
    if skipped > 0 {
        parts.push(format!("{skipped} skipped"));
    }
    if failed > 0 {
        parts.push(format!("{failed} failed"));
    }
    println!("{}", parts.join(", "));

    Ok(i32::from(failed > 0))
}

fn process_one(
    loaded: &LoadedSession,
    spec: &models::ModelSpec,
    input: &Path,
    out_path: &Path,
    format: OutputFormat,
) -> Result<(), String> {
    let img = imaging::load_oriented(input).map_err(|e| e.to_string())?;
    let tensor = imaging::preprocess(&img, spec);
    let mask_data = loaded.infer(tensor, spec.input_size)?;
    let s = spec.input_size as usize;
    let mask = imaging::postprocess(&mask_data, s * s, img.width(), img.height(), spec)
        .map_err(|e| e.to_string())?;
    let composed = imaging::compose(&img, &mask);
    imaging::encode_and_write(&composed, out_path, format).map_err(|e| e.to_string())
}

/// The app's single-dropped-folder rule: a sibling `<name>-noground` output
/// directory. Only meaningful when --out is unset (checked by the caller).
fn single_dir_out(inputs: &[String]) -> Option<PathBuf> {
    if inputs.len() != 1 {
        return None;
    }
    let dir = Path::new(&inputs[0]);
    if !dir.is_dir() {
        return None;
    }
    let name = dir.file_name()?.to_string_lossy();
    dir.parent()?.join(format!("{name}-noground")).into()
}

fn app_settings() -> Settings {
    match models::app_support_dir() {
        Some(dir) => Settings::load(&dir.join("settings.json")),
        None => Settings::default(),
    }
}

// --- the Settings-managed symlink -------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    /// symlink exists and points at the running binary
    pub installed: bool,
    pub link_path: String,
    /// what the symlink points at, for display; null when nothing is installed
    pub target: Option<String>,
    /// a fresh shell would find `noground` on its PATH
    pub bin_dir_on_path: bool,
    /// debug build: invoking the link opens the app, CLI mode needs `noground cli`
    pub dev_build: bool,
}

pub fn link_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".local/bin").join(LINK_NAME))
}

fn this_binary() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.canonicalize().ok()
}

fn link_target(link: &Path) -> Option<PathBuf> {
    let target = fs::read_link(link).ok()?;
    target.canonicalize().ok()
}

pub fn status() -> CliStatus {
    let link = link_path();
    let target = link.as_ref().and_then(|p| fs::read_link(p).ok());
    let installed = link
        .as_ref()
        .zip(this_binary().as_ref())
        .is_some_and(|(p, exe)| link_target(p).is_some_and(|t| &t == exe));
    CliStatus {
        installed,
        link_path: link
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        target: target.map(|t| t.display().to_string()),
        bin_dir_on_path: bin_dir_on_path(),
        dev_build: cfg!(debug_assertions),
    }
}

/// Create (or re-point) `~/.local/bin/noground` at the running binary. The
/// well-known name is ours to take; a directory in the way is never touched.
pub fn install() -> Result<CliStatus> {
    let exe = std::env::current_exe().context("cannot locate the running binary")?;
    let link = link_path().context("no home directory")?;
    let bin_dir = link.parent().context("link path has no parent")?;
    fs::create_dir_all(bin_dir)?;

    match fs::symlink_metadata(&link) {
        Ok(meta) if meta.is_dir() => {
            return Err(anyhow!(
                "{} exists and is a directory — remove it first",
                link.display()
            ));
        }
        Ok(_) => {
            let _ = fs::remove_file(&link);
        }
        Err(_) => {}
    }
    std::os::unix::fs::symlink(&exe, &link)
        .with_context(|| format!("failed to create {}", link.display()))?;
    Ok(status())
}

/// Remove the symlink, but only when it points at this binary — a link we
/// didn't create is not ours to delete.
pub fn uninstall() -> Result<CliStatus> {
    let link = link_path().context("no home directory")?;
    if fs::symlink_metadata(&link).is_err() {
        return Ok(status());
    }
    let ours = this_binary()
        .zip(link_target(&link))
        .is_some_and(|(exe, target)| exe == target);
    if !ours {
        return Err(anyhow!(
            "{} does not point at this Noground build — remove it manually if it isn't yours",
            link.display()
        ));
    }
    fs::remove_file(&link)?;
    Ok(status())
}

/// Is `~/.local/bin` reachable from a fresh shell? GUI apps inherit a
/// minimal PATH, so checking the env alone isn't enough — the usual shell
/// configs are where a new Terminal would actually pick the dir up.
fn bin_dir_on_path() -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let bin = home.join(".local/bin");
    if std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d == bin)) {
        return true;
    }
    [".zshenv", ".zprofile", ".zshrc", ".profile", ".bashrc"]
        .iter()
        .any(|rc| {
            fs::read_to_string(home.join(rc))
                .map(|text| text.contains(".local/bin"))
                .unwrap_or(false)
        })
}
