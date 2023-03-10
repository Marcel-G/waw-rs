use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{self},
};
use xtask_wasm::{clap, Target};

/// A helper to generate the distributed package.
///
/// # Usage
///
/// ```rust,no_run
/// use std::process;
/// use xtask_wasm::{anyhow::Result, clap};
///
/// #[derive(clap::Parser)]
/// enum Opt {
///     Dist(xtask_wasm::Dist),
/// }
///
/// fn main() -> Result<()> {
///     let opt: Opt = clap::Parser::parse();
///
///     match opt {
///         Opt::Dist(dist) => {
///             log::info!("Generating package...");
///
///             dist
///                 .static_dir_path("my-project/static")
///                 .app_name("my-project")
///                 .run_in_workspace(true)
///                 .run("my-project")?;
///         }
///     }
///
///     Ok(())
/// }
/// ```
///
/// In this example, we added a `dist` subcommand to build and package the
/// `my-project` crate. It will run the [`default_build_command`](crate::default_build_command)
/// at the workspace root, copy the content of the `project/static` directory,
/// generate JS bindings and output two files: `project.js` and `project.wasm`
/// into the dist directory.
#[non_exhaustive]
#[derive(Debug, Default, clap::Parser)]
#[clap(
    about = "Generate the distributed package.",
    long_about = "Generate the distributed package.\n\
			It will build and package the project for WASM."
)]
#[group(skip)]
pub struct Dist {
    #[clap(flatten)]
    wasm: xtask_wasm::Dist,
    #[clap(long)]
    shared_memory: bool,
}

impl Dist {
    /// Set the command used by the build process.
    ///
    /// The default command is the result of the [`default_build_command`].
    pub fn build_command(mut self, command: process::Command) -> Self {
        self.wasm.build_command = command;
        self
    }

    /// Sets shared memory build flags to enabled
    ///
    /// A couple of steps are necessary to get this build working which makes it slightly
    /// nonstandard compared to most other builds.
    pub fn shared_memory(mut self, enabled: bool) -> Self {
        self.shared_memory = enabled;
        self
    }

    /// Set the directory for the generated artifacts.
    ///
    /// The default for debug build is `target/debug/dist` and
    /// `target/release/dist` for the release build.
    pub fn dist_dir_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.wasm.dist_dir_path = Some(path.into());
        self
    }

    /// Set the directory for the static artifacts (like `index.html`).
    pub fn static_dir_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.wasm.static_dir_path = Some(path.into());
        self
    }

    /// Set the resulting package name.
    ///
    /// The default is `app`.
    pub fn app_name(mut self, app_name: impl Into<String>) -> Self {
        self.wasm.app_name = Some(app_name.into());
        self
    }

    /// Set the dist process current directory as the workspace root.
    pub fn run_in_workspace(mut self, res: bool) -> Self {
        self.wasm.run_in_workspace = res;
        self
    }

    /// Set if this is a release or debug build
    pub fn release(mut self, res: bool) -> Self {
        self.wasm.release = res;
        self
    }

    /// Set if typescript .d.ts should be output
    pub fn typescript(mut self, res: bool) -> Self {
        self.wasm.typescript = res;
        self
    }

    /// Set the wasm target
    pub fn target(mut self, res: Target) -> Self {
        self.wasm.target = Some(res);
        self
    }

    /// Build the given package for Wasm.
    ///
    /// This will generate JS bindings via [`wasm-bindgen`](https://docs.rs/wasm-bindgen/latest/wasm_bindgen/)
    /// and copy files from a given static directory if any to finally return
    /// the paths of the generated artifacts with [`DistResult`].
    ///
    /// Wasm optimizations can be achieved using [`crate::WasmOpt`] if the
    /// feature `wasm-opt` is enabled.
    pub fn run(mut self, package_name: &str) -> Result<DistResult> {
        // Run xtask-wasm build

        if self.shared_memory {
            let build_command = &mut self.wasm.build_command;

            // First, the Rust standard library needs to be recompiled with atomics
            // enabled. to do that we use Cargo's unstable `-Zbuild-std` feature.
            build_command.env(
                "RUSTFLAGS",
                "-C target-feature=+atomics,+bulk-memory,+mutable-globals",
            );
            // Next we need to compile everything with the `atomics` and `bulk-memory`
            // features enabled, ensuring that LLVM will generate atomic instructions,
            // shared memory, passive segments, etc.
            build_command.args(["-Z", "build-std=std,panic_abort"]);
        }

        let result = self
            .wasm
            .target(Target::Web)
            .typescript(true)
            .run(package_name)?;

        let worklet_entry = result
            .dist_dir
            .join(format!("{package_name}.worklet.entry.js"));
        let worklet_final = result.dist_dir.join(format!("{package_name}.worklet.js"));

        fs::write(
            &worklet_entry,
            generate_worklet_entry(result.js.strip_prefix(&result.dist_dir)?),
        )?;

        fs::copy(&worklet_entry, &worklet_final)?;
        fs::remove_file(&worklet_entry)?;

        Ok(DistResult {
            dist_dir: result.dist_dir,
            main_js: result.js,
            wasm: result.wasm,
            worklet_js: worklet_final,
        })
    }
}

/// Provides paths of the generated dist artifacts.
#[derive(Debug)]
pub struct DistResult {
    /// Directory containing the generated artifacts.
    pub dist_dir: PathBuf,
    /// JS output generated by wasm-bindgen for the main thread.
    pub main_js: PathBuf,
    /// Wasm output generated by wasm-bindgen. (shared for main & audio worklet)
    pub wasm: PathBuf,
    /// Wasm output generated by wasm-bindgen for the audio worklet.
    pub worklet_js: PathBuf,
}

fn generate_worklet_entry(main_js: &Path) -> String {
    let boilerplate = include_str!("./worklet.entry.js");
    let path = main_js.to_str().unwrap();

    format!(
        "
        import * as bindgen from \"./{path}\";
        {boilerplate}
    "
    )
}
