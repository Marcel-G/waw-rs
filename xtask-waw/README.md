# xtask-waw

This crate is a small wrapper around [xtask-wasm](https://github.com/rustminded/xtask-wasm) that produces an additional AudioWorklet initialisation shim for use with [waw-rs](https://github.com/Marcel-G/waw-rs).

It is based on the [`xtask` concept](https://github.com/matklad/cargo-xtask/), instead of using
external tooling like [`wasm-pack`](https://github.com/rustwasm/wasm-pack).

## Minimum Supported Rust Version

This crate requires **Rust 1.58.1** at a minimum because there is a security
issue on a function we use from std in previous version
(see [cve-2022-21658](https://groups.google.com/g/rustlang-security-announcements/c/R1fZFDhnJVQ)).

## Setup

The best way to add xtask-waw to your project is to create a workspace
with two packages: your project's package and the xtask package.

### Create a project using xtask

* Create a new directory that will contains the two package of your project
  and the workspace's `Cargo.toml`:

  ```console
  mkdir my-project
  cd my-project
  touch Cargo.toml
  ```

* Create the project package and the xtask package using `cargo new`:

  ```console
  cargo new my-project
  cargo new xtask
  ```

* Open the workspace's `Cargo.toml` and add the following:

  ```toml
  [workspace]
  default-members = ["my-project"]
  members = [
      "my-project",
      "xtask",
  ]
  ```

* Create a `.cargo/config.toml` file and add the following content:

  ```toml
  [alias]
  xtask = "run --package xtask --"
  ```

The directory layout should look like this:

```console
project
├── .cargo
│   └── config.toml
├── Cargo.toml
├── my-project
│   ├── Cargo.toml
│   └── src
│       └── ...
└── xtask
    ├── Cargo.toml
    └── src
        └── main.rs
```

And now you can run your xtask package using:

```console
cargo xtask
```

You can find more information about xtask
[here](https://github.com/matklad/cargo-xtask/).

### Use xtask-waw as a dependency

Finally, add the following to the xtask package's `Cargo.toml`:

```toml
[dependencies]
xtask-waw = { git = "https://github.com/Marcel-G/waw-rs" }
```

## Usage

This library gives you three structs:

* [`Dist`](https://docs.rs/xtask-wasm/latest/xtask_waw/dist/struct.Dist.html) - Generate a distributed package for Wasm.

They all implement [`clap::Parser`](https://docs.rs/clap/latest/clap/trait.Parser.html)
allowing them to be added easily to an existing CLI implementation and are
flexible enough to be customized for most use-cases.

You can find further information for each type at their documentation level.

## Examples

### A basic implementation

```rust
use std::process::Command;
use xtask_waw::{anyhow::Result, clap};

#[derive(clap::Parser)]
enum Opt {
    Dist(xtask_waw::Dist),
}


fn main() -> Result<()> {
    let opt: Opt = clap::Parser::parse();

    match opt {
        Opt::Dist(dist) => {
            dist
                .dist_dir_path("dist")
                .app_name("my-project")
                .run_in_workspace(true)
                .run("my-project")?;
        }
    }

    Ok(())
}
```
