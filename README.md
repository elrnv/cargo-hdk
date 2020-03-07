# `cargo-hdk`

A subcommand of cargo for building Houdini plugins based on the Houdini Development Kit (HDK).

The purpose of this command line tool is to simplify building Rust plugins for Houdini using the
HDK.

[![On crates.io](https://img.shields.io/crates/v/cargo-hdk.svg)](https://crates.io/crates/cargo-hdk)
[![Build status](https://travis-ci.org/elrnv/cargo-hdk.svg?branch=master)](https://travis-ci.org/elrnv/cargo-hdk)

# Features

Build a CMake based HDK plugin in a subdirectory (default is `$CARGO_MANIFEST_DIR/hdk` where
`$CARGO_MANIFEST_DIR` is the crate root directory containing the `Cargo.toml` file) containing the
`CMakeLists.txt` and the source code. The actual build artifacts are stored in a designated `build`
subdirectory (for debug builds the complete build path is `$CARGO_MANIFEST_DIR/hdk/build_debug`).

# Usage

To build the HDK plugin located in `$CARGO_MANIFEST_DIR/hdk`, simply run

```
cargo hdk
```

For release builds use

```
cargo hdk --release
```

To use a different CMake generator like Ninja, use the `--cmake` option

```
cargo hdk --cmake '[-G Ninja]'
```

All arguments are expected to be within `[` and `]` brackets to avoid ambiguity with arguments
passed directly to the `cargo build` command.

Note that specifying the CMake generator is required on the first build only. Subsequent builds will
use the cached generator, unless `cargo hdk --clean` is run, which clears all build artifacts.

