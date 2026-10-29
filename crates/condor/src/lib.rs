//! The `condor` command line, reproducing vulture 2.16's.

pub mod args;
pub mod config;
pub mod discover;
pub mod output;
pub mod run;

/// The program name shown in usage and error messages.
pub const PROG: &str = "condor";

/// `--version` output.
pub fn version_string() -> String {
    format!("condor {} (vulture 2.16 compatible)", env!("CARGO_PKG_VERSION"))
}
