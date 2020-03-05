# `cargo-hdk`

A subcommand of cargo for building Houdini plugins based on the Houdini Development Kit (HDK).

The purpose of this command line tool is to simplify building Rust plugins for Houdini using the
HDK.

# Features

Build a CMake based HDK plugin in a subdirectory (default is `./hdk`) containing the CMakeLists.txt
and the source code. The actual build artifacts are stored in a designated `build` subdirectory (so
by default this is `./hdk/build`).

# Usage

To build the HDK plugin located in `./hdk`, simply run

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

