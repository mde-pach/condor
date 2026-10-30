//! The `condor` command line, reproducing vulture 2.16's.

pub mod args;
pub mod config;
#[cfg(unix)]
pub mod daemon;
pub mod discover;
pub mod engine;
pub mod output;
pub mod query;
pub mod run;
pub mod server;

/// The program name shown in usage and error messages.
pub const PROG: &str = "condor";

/// `--version` output.
pub fn version_string() -> String {
    format!("condor {} (vulture 2.16 compatible)", env!("CARGO_PKG_VERSION"))
}
