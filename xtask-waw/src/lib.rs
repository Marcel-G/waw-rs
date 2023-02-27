pub use xtask_wasm::{
    anyhow, cargo_metadata, cargo_metadata::camino, clap, metadata, package, xtask_command, Target,
};

mod dist;

pub use dist::*;

#[cfg(feature = "wasm-opt")]
#[cfg_attr(docsrs, doc(cfg(feature = "wasm-opt")))]
pub use ::xtask_wasm::WasmOpt;
