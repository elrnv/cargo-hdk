#[macro_use]
extern crate anyhow;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use anyhow::{Context, Result};

use clap::{AppSettings, Parser};
use log::*;

use cargo_metadata::{camino::Utf8PathBuf, Message, MetadataCommand, Package};

const ABOUT: &str = "
cargo-hdk is a cargo subcommand to compile and install a Houdini plugin written in Rust and C++.";

#[derive(Parser, Debug)]
#[clap(author, about = ABOUT, name = "cargo-hdk")]
struct Opt {
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,

    /// Arguments for the 'cargo build' step. These are ignored when the '--hdk-only' flag is used.
    #[clap(name = "BUILD ARGS")]
    build_args: Vec<String>,

    /// Skip the 'cargo build` step. Build only the HDK plugin.
    #[clap(short = 'k', long)]
    hdk_only: bool,

    /// Remove artifacts created by the build process including the HDK plugin.
    ///
    /// To clean the HDK build only, use the '--hdk-only' flag in combination with this flag.
    #[clap(long)]
    clean: bool,

    /// Pass arguments to CMake configuration.
    ///
    /// For instance to use Ninja as the cmake generator, use '--cmake "-G Ninja"'.
    #[clap(short, long, default_value = "")]
    cmake: String,

    /// Path to the HDK plugin relative to the root of the crate. This must be a Unicode path.
    #[clap(short, long, default_value = "./hdk")]
    hdk_path: Utf8PathBuf,

    /// Path prefix to the automatically generated files containing the Rust output directories
    /// ('OUT_DIR') of the crate being built as well as any additional dependencies specified by
    /// '--deps'.
    ///
    /// Note that this path is relative to the hdk build directory unless this path is absolute.
    ///
    /// These files can be loaded directly into CMake variables using the CMake 'file' command.
    ///
    /// These files are saved in the following format: '<hdk build directory>/<out_dir_file_prefix>.txt'.
    ///
    /// For example if the prefix is 'rust/out_dir_', '--hdk-path' is './hdk' then for a dependent
    /// crate named 'pkg', the 'OUT_DIR' file will be saved as './hdk/build_release/rust/out_dir_pkg.txt'.
    ///
    /// If multiple versions of the same dependency are found, the last one built is the one that
    /// will have an associated 'OUT_DIR' file.
    #[clap(long, default_value = "rust/out_dir_")]
    out_dir_file_prefix: String,

    /// The list of dependency names for which to produce an 'OUT_DIR' file.
    #[clap(long, default_value = "hdkrs")]
    deps: Vec<String>,
}

pub fn init_logging(level: Option<log::Level>) {
    if let Some(level) = level {
        let mut builder = env_logger::Builder::new();
        builder.filter(None, level.to_level_filter());
        builder.format_timestamp(None).format_module_path(false);
        builder.init();
    }
}

// Run the cargo build (or clean) command and return the output directories to cache for each
// dependency (including the crate being compiled).
fn cargo_build(opts: &Opt, package: &Package) -> Result<Vec<(String, Utf8PathBuf)>> {
    info!("Building Rust code using cargo.");

    let build_args = if opts.build_args.first().map(|x| x.as_str()) == Some("hdk") {
        &opts.build_args[1..]
    } else {
        opts.build_args.as_slice()
    };

    if opts.clean {
        let status = Command::new(env!("CARGO"))
            .arg("clean")
            .args(build_args)
            .status()
            .context("Cargo clean failed")?;

        if !status.success() {
            return Err(anyhow!("Rust clean failed"));
        }
        Ok(Vec::new())
    } else {
        // First build the crate with the standard build args.
        let out = Command::new(env!("CARGO"))
            .args(&["build", "--message-format=json"])
            .args(build_args)
            .stderr(Stdio::inherit())
            .stdout(Stdio::piped())
            .output()
            .context("Cargo build failed")?;

        if !out.status.success() {
            return Err(anyhow!("Rust build failed"));
        }

        let reader = std::io::BufReader::new(out.stdout.as_slice());
        let mut out_dir_deps = Vec::new();
        for message in Message::parse_stream(reader) {
            if let Message::BuildScriptExecuted(script) = message.unwrap() {
                trace!(
                    "Checking if a build script package id {} is {}",
                    &script.package_id.repr,
                    &package.id
                );
                if script.package_id == package.id {
                    out_dir_deps.push((package.name.clone(), script.out_dir.clone()));
                    continue;
                }
                for dep in &opts.deps {
                    trace!(
                        "Checking if a build script package id {} contains {}",
                        &script.package_id.repr,
                        &dep
                    );
                    if script.package_id.repr.contains(dep) {
                        out_dir_deps.push((dep.clone(), script.out_dir.clone()));
                        continue;
                    }
                }
            }
        }

        Ok(out_dir_deps)
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

    // Remember current working directory.
    let orig_cur_dir = env::current_dir().context("Failed to get current directory")?;
    info!("Looking for a parent directory containing the `Cargo.toml` manifest file.");

    let metadata = MetadataCommand::new().exec()?;
    let package = metadata
        .root_package()
        .context("Failed to find crate root")?;

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

    let build_dir = package
        .manifest_path
        .parent()
        .context("Failed to find manifest directory")?
        .join(&opts.hdk_path)
        .join(&format!("build_{}", build_type.to_lowercase()));

    // Do the CMake clean

    if opts.clean {
        // Clean the build artifacts.
        if let Err(e) = fs::remove_dir_all(&build_dir) {
            warn!("Failed to remove {}: {}", build_dir, e);
        }

        return Ok(());
    } else {
        debug!("Creating the build directory: {:?}.", build_dir);

        // Create build directory if it doesn't exist
        match fs::create_dir(&build_dir) {
            Err(err) if err.kind() != std::io::ErrorKind::AlreadyExists => {
                bail!("Failed to create build directory: {:?}", &build_dir);
            }
            _ => {}
        }
    }

    // Do the Cargo build/clean

    // Cargo build with a custom target directory set to the cmake build directory.
    if !opts.hdk_only {
        // Cache the out_dir in a file so that the C++ code can be built without running cargo later.
        let out_dir_deps = cargo_build(&opts, &package)?;
        for (dep, out_dir) in out_dir_deps {
            use std::io::Write;
            let out_dir_path = build_dir.join(format!("{}{}.txt", &opts.out_dir_file_prefix, dep));
            // Build directory structure for out_dir_path.
            let out_dir_path_dir = out_dir_path
                .parent()
                .expect(&format!("Invalid 'OUT_DIR' path: {}", out_dir_path));
            if !out_dir_path_dir.exists() {
                fs::create_dir_all(out_dir_path_dir).expect(&format!(
                    "Failed to create 'OUT_DIR' path directory: {}",
                    out_dir_path_dir
                ));
            }

            let mut out_dir_file = fs::File::create(out_dir_path.clone()).context(format!(
                "Failed to create the OUT_DIR file: {}",
                out_dir_path
            ))?;
            write!(out_dir_file, "{}", out_dir)?;
            // Close the file at the end of the scope.
        }
    }

    if opts.clean {
        return Ok(());
    }

    // Do the CMake build

    env::set_current_dir(&build_dir)
        .with_context(|| format!("Failed to set current directory: {:?}", &build_dir))?;

    debug!("Parsing cmake args.");

    let mut cmake_args = Vec::new();
    if !opts.cmake.is_empty() {
        // Strip square brackets if they are there for backwards compatibility.
        let arg_str = if opts.cmake.starts_with('[') && opts.cmake.ends_with(']') {
            &opts.cmake[1..opts.cmake.len() - 1]
        } else {
            &opts.cmake[..]
        };

        // Quick and dirty quoation sensitive parser
        let mut cur_arg = String::new();
        let mut in_quoted_text = None;
        for c in arg_str.chars() {
            if let Some(quotation_char) = in_quoted_text {
                if c == quotation_char {
                    // End quoted text
                    in_quoted_text = None;
                    continue; // Consume the qutation mark
                }
            } else if c == '\"' || c == '\'' {
                in_quoted_text = Some(c);
                continue;
            } else if c.is_whitespace() {
                if !cmake_args.is_empty() {
                    let finished_arg = std::mem::replace(&mut cur_arg, String::new());
                    cmake_args.push(finished_arg);
                }
                continue;
            }
            cur_arg.push(c);
        }
        // Push any remainder arg to cmake_args.
        if !cur_arg.is_empty() {
            cmake_args.push(cur_arg);
        }
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
