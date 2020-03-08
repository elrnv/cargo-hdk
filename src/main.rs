#[macro_use]
extern crate anyhow;

use anyhow::{Context, Result};

use log::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use structopt::{clap::AppSettings, StructOpt};

const ABOUT: &str = "
cargo-hdk is a cargo subcommand to compile C++ code defining an HDK interface for a Houdini plugin. This subcommand runs 'cargo build' with the provided arguments followed by a CMake build of the HDK plugin.";

#[derive(StructOpt, Debug)]
#[structopt(author, about = ABOUT, name = "cargo-hdk")]
struct Opt {
    #[structopt(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    /// Arguments for the 'cargo build' step. These are mostly igonored if `--hdk-only' is
    /// specified.
    #[structopt(name = "BUILD ARGS")]
    build_args: Vec<String>,

    /// Skip the 'cargo build` step. Build only the HDK plugin.
    #[structopt(short = "k", long)]
    hdk_only: bool,

    /// Remove artifacts created by the build process including the HDK plugin.
    ///
    /// To clean the HDK build only, use the '--hdk-only' flag in combination with this flag.
    #[structopt(long)]
    clean: bool,

    /// Pass arguments to CMake configuration.
    ///
    /// Arguments are expected to be listed between brackets. For instance to use Ninja as the
    /// cmake generator, use '--cmake "[-G Ninja]"'.
    #[structopt(short, long, default_value = "")]
    cmake: String,

    /// Path to the HDK plugin relative to the root of the crate.
    #[structopt(short, long, default_value = "./hdk")]
    hdk_path: PathBuf,
}

pub fn init_logging(level: Option<log::Level>) {
    if let Some(level) = level {
        let mut builder = env_logger::Builder::new();
        builder.filter(None, level.to_level_filter());
        builder.format_timestamp(None).format_module_path(false);
        builder.init();
    }
}

fn main() -> Result<()> {
    use terminal_size::{terminal_size, Width};
    let app = Opt::clap()
        .set_term_width(if let Some((Width(w), _)) = terminal_size() {
            w as usize
        } else {
            80
        })
        .setting(AppSettings::AllowLeadingHyphen);

    let opts = Opt::from_clap(&app.get_matches());
    init_logging(opts.verbose.log_level());

    info!("Looking for a parent directory containing the `Cargo.toml` manifest file.");

    // Find crate root directory. This is not provided by cargo at the time of this writing.
    // Start at current directory and follow upstream until the root directory "/".
    let orig_cur_dir = env::current_dir().context("Failed to get current directory")?;
    let mut root_dir: &Path = &orig_cur_dir;
    while !root_dir.join("Cargo.toml").exists() || !root_dir.join("Cargo.toml").is_file() {
        root_dir = if let Some(parent) = root_dir.parent() {
            parent
        } else {
            bail!(
                "Couldn't find `Cargo.toml` in {:?} or any parent directory.",
                orig_cur_dir
            );
        }
    }

    if !opts.hdk_only {
        info!("Building rust code using cargo.");

        let build_args = if opts.build_args.first().map(|x| x.as_str()) == Some("hdk") {
            &opts.build_args[1..]
        } else {
            opts.build_args.as_slice()
        };

        if opts.clean {
            Command::new(env!("CARGO"))
                .arg("clean")
                .args(build_args)
                .status()
                .context("Cargo clean failed")?;
        } else {
            Command::new(env!("CARGO"))
                .arg("build")
                .args(build_args)
                .status()
                .context("Cargo build failed")?;
        }
    }

    info!("Looking for a Houdini installation.");

    let hfs = env::var("HFS").ok().or_else(|| {
        // Try some typical installation paths:
        for version in &["18.5", "18.0", "17.5", "17.0"] {
            let hfs_path = format!("/opt/hfs{}", version);
            info!("Using Houdini installation path {:?}", hfs_path);
            if Path::new(&hfs_path).exists() {
                return Some(hfs_path);
            }
        }
        None
    }).context("Couldn't find HFS. Please source 'houdini_setup' from houdini's installation directory or set the 'HFS' environment variable to the Houdini installation path.")?;

    env::set_var("HFS", &hfs);
    // Set the path variable to include hfs bin directory.
    // This is needed in case hserver needs to verify the license during a build.
    if let Some(path) = env::var_os("PATH") {
        let mut paths = env::split_paths(&path).collect::<Vec<_>>();
        paths.push(PathBuf::from(&hfs).join("bin"));
        let new_path = env::join_paths(paths)?;
        env::set_var("PATH", &new_path);
    }

    debug!("Determining build type.");

    let build_type = opts
        .build_args
        .iter()
        .find(|&x| x == "--release")
        .map(|_| "Release")
        .unwrap_or_else(|| "Debug");

    let build_dir = PathBuf::from(root_dir)
        .join(opts.hdk_path)
        .join(&format!("build_{}", build_type.to_lowercase()));

    if opts.clean {
        // Clean the build artifacts
        std::fs::remove_dir_all(&build_dir).with_context(|| {
            format!(
                "Failed to remove HDK build artifacts located in {:?}",
                &build_dir
            )
        })?;

        return Ok(());
    }

    debug!("Creating the build directory: {:?}.", build_dir);

    // Create build directory if it doesn't exist
    match std::fs::create_dir(&build_dir) {
        Err(err) if err.kind() != std::io::ErrorKind::AlreadyExists => {
            bail!("Failed to create build directory: {:?}", &build_dir);
        }
        _ => {}
    }

    env::set_current_dir(&build_dir)
        .with_context(|| format!("Failed to set current directory: {:?}", &build_dir))?;

    debug!("Parsing cmake args.");

    let mut cmake_args = Vec::new();
    if !opts.cmake.is_empty() {
        if opts.cmake.starts_with("[") && opts.cmake.ends_with("]") {
            cmake_args = opts.cmake[1..opts.cmake.len() - 1]
                .split_whitespace()
                .collect();
        } else {
            eprintln!("WARNING: cmake args must be surrounded with square brackets '[' and ']'.");
        };
    }

    info!("Configuring CMake.");

    Command::new("cmake")
        .arg("..")
        .args(&cmake_args)
        .arg(&format!("-DCMAKE_BUILD_TYPE={}", build_type))
        .status()
        .context("Failed to configure CMake.")?;

    info!("Building the C/C++ HDK plugin.");

    Command::new("cmake")
        .arg("--build")
        .arg(".")
        .status()
        .context("Failed to build HDK plugin.")?;

    env::set_current_dir(&orig_cur_dir)
        .with_context(|| format!("Failed to reset current directory: {:?}", &orig_cur_dir))?;

    Ok(())
}
