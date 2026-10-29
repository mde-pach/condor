//! Language-agnostic core of condor.
//!
//! A language front end turns one source file into a [`FileScan`]: the items it
//! defines and the names it uses. [`Analysis`] combines the scans of all files
//! and answers the whole-project question "which definitions are never used".
//! Nothing here knows about Python.

pub mod fnmatch;
pub mod item;
pub mod paths;
pub mod project;

use std::path::Path;

pub use item::{Item, ItemKind};
pub use project::{Analysis, ClassFacts, Decorator, FileScan, FunctionFacts, Stream, WhitelistEntry};

/// What a language front end needs to know to scan a file.
#[derive(Clone, Debug, Default)]
pub struct ScanOptions {
    pub verbose: bool,
    /// The working directory, for `format_path` and relative paths.
    pub cwd: std::path::PathBuf,
    pub ignore_names: fnmatch::PatternSet,
    pub ignore_decorators: fnmatch::PatternSet,
    /// Calls whose string arguments name things a framework provides by name
    /// (e.g. `pytest.mark.usefixtures`), matched on resolved dotted names.
    pub call_hints: fnmatch::PatternSet,
}

/// A language front end.
pub trait Language: Sync {
    /// Scan one decoded source file.
    fn scan(&self, source: &str, path: &Path, opts: &ScanOptions) -> FileScan;

    /// Sources the language may add once all files are scanned (for Python:
    /// vulture's bundled whitelists for the imported modules). Returns the
    /// candidate paths in the order they are considered, with their source if
    /// the language ships one. The caller applies exclusions.
    fn implicit_sources(&self, analysis: &Analysis) -> Vec<(String, Option<&'static str>)>;

    /// Apply the language's project-wide rules once every file is scanned
    /// (for example, rules that follow class hierarchies across files).
    fn link(&self, analysis: &mut Analysis);
}
