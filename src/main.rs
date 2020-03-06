use structopt::{clap::AppSettings, StructOpt};
use std::path::PathBuf;
use std::process::Command;
use std::env;

const ABOUT: &str = "
cargo-hdk is a cargo subcommand to compile C++ code defining an HDK interface for a Houdini plugin. This subcommand runs 'cargo build' with the provided arguments followed by a CMake build of the HDK plugin.";

#[derive(StructOpt, Debug)]
#[structopt(author, about = ABOUT, name = "cargo-hdk")]
struct Opt {
    /// Arguments for the 'cargo build' step. These are igonored if `--hdk-only'
    /// is specified.
    #[structopt(name = "BUILD ARGS")]
    build_args: Vec<String>,

    /// Skip the 'cargo build` step. Build only the HDK plugin.
    #[structopt(short = "k", long)]
    hdk_only: bool,

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

fn main() {
    use terminal_size::{ terminal_size, Width };
    let app = Opt::clap().set_term_width(if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    }).setting(AppSettings::AllowLeadingHyphen);

    let opts = Opt::from_clap(&app.get_matches());

    if !opts.hdk_only {
        let build_args = if opts.build_args.first().map(|x| x.as_str()) == Some("hdk") {
            &opts.build_args[1..]
        } else {
            opts.build_args.as_slice()
        };
        Command::new(env!("CARGO"))
            .arg("build")
            .args(build_args)
            .status()
            .expect("Cargo build failed");
    }

    let hfs = env::var("HFS").ok().or_else(|| {
        // Try some typical installation paths:
        for version in &["18.5", "18.0", "17.5", "17.0"] {
            let hfs_path = format!("/opt/hfs{}", version);
            if PathBuf::from(&hfs_path).exists() {
                return Some(hfs_path);
            }
        }
        None
    }).expect("Couldn't find HFS. Please source 'houdini_setup' from houdini's installation directory or set the 'HFS' environment variable to the Houdini installation path.");

    env::set_var("HFS", hfs);

    let build_type = opts.build_args.iter().find(|&x| x == "--release")
        .map(|_| "Release").unwrap_or_else(|| "Debug");

    let root_dir = env!("CARGO_MANIFEST_DIR");

    let build_dir = PathBuf::from(root_dir).join(opts.hdk_path).join("build");

    // Build if it doesn't exist
    match std::fs::create_dir(&build_dir) {
        Err(err) if err.kind() != std::io::ErrorKind::AlreadyExists => {
            panic!("Failed to create build directory: {:?}", &build_dir);
        }
        _ => {}
    }

    let cur_dir = env::current_dir().expect("Failed to get current directory");
    env::set_current_dir(&build_dir).expect(&format!("Failed to set current directory: {:?}", &build_dir));

    let mut cmake_args = Vec::new();
    if !opts.cmake.is_empty() {
        if opts.cmake.starts_with("[") && opts.cmake.ends_with("]") {
            cmake_args = opts.cmake[1..opts.cmake.len()-1].split_whitespace().collect();
        } else {
            eprintln!("WARNING: cmake args must be surrounded with square brackets '[' and ']'.");
        };
    }

    Command::new("cmake")
        .arg("..")
        .args(&cmake_args)
        .arg(&format!("-DCMAKE_BUILD_TYPE={}", build_type))
        .status()
        .expect("Failed to configure CMake.");

    Command::new("cmake")
        .arg("--build")
        .arg(".")
        .status()
        .expect("Failed to configure CMake.");

    env::set_current_dir(cur_dir).expect("Failed to reset current directory");
}
