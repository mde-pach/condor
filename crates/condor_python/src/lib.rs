//! Python front end for condor, reproducing vulture 2.16.

pub mod format_string;
pub mod literal;
pub mod noqa;
pub mod plugins;
pub mod read;
pub mod rules;
pub mod scanner;
pub mod type_comments;

use std::path::Path;
use std::sync::Arc;

use condor_core::{Analysis, FileScan, ItemKind, Language, ScanOptions};

/// vulture's bundled whitelists (MIT, copyright Jendrik Seipp; see
/// `whitelists/LICENSE-vulture.txt`), keyed by module name.
pub const BUNDLED_WHITELISTS: [(&str, &str); 13] = [
    ("argparse", include_str!("whitelists/argparse_whitelist.py")),
    ("ast", include_str!("whitelists/ast_whitelist.py")),
    ("collections", include_str!("whitelists/collections_whitelist.py")),
    ("ctypes", include_str!("whitelists/ctypes_whitelist.py")),
    ("enum", include_str!("whitelists/enum_whitelist.py")),
    ("logging", include_str!("whitelists/logging_whitelist.py")),
    ("pint", include_str!("whitelists/pint_whitelist.py")),
    ("socketserver", include_str!("whitelists/socketserver_whitelist.py")),
    ("ssl", include_str!("whitelists/ssl_whitelist.py")),
    ("string", include_str!("whitelists/string_whitelist.py")),
    ("sys", include_str!("whitelists/sys_whitelist.py")),
    ("threading", include_str!("whitelists/threading_whitelist.py")),
    ("unittest", include_str!("whitelists/unittest_whitelist.py")),
];

/// The Python front end, with the plugins that apply to the project.
pub struct Python {
    pub plugins: Vec<plugins::Plugin>,
}

impl Python {
    /// With the bundled plugins only.
    pub fn bundled() -> Python {
        Python { plugins: plugins::bundled() }
    }
}

impl Language for Python {
    fn scan(&self, source: &str, path: &Path, opts: &ScanOptions) -> FileScan {
        scanner::scan(source, Arc::from(path), opts)
    }

    /// For each distinct imported name, `whitelists/<name>_whitelist.py` and
    /// its source if vulture bundles it. vulture iterates a set here, so its
    /// order is unspecified; condor uses alphabetical order.
    fn implicit_sources(&self, analysis: &Analysis) -> Vec<(String, Option<&'static str>)> {
        let mut imported: Vec<&str> = analysis.defined(ItemKind::Import).map(|i| i.name.as_str()).collect();
        imported.sort_unstable();
        imported.dedup();
        imported
            .into_iter()
            .map(|name| {
                // vulture looks the file up on disk, so `from enum import Enum`
                // finds `enum_whitelist.py` on case-insensitive file systems
                // (the macOS and Windows defaults) but not on Linux.
                let matches = |n: &str| {
                    if cfg!(any(target_os = "macos", windows)) { n.eq_ignore_ascii_case(name) } else { n == name }
                };
                let source = BUNDLED_WHITELISTS.iter().find(|(n, _)| matches(n)).map(|(_, s)| *s);
                (format!("whitelists/{name}_whitelist.py"), source)
            })
            .collect()
    }

    fn link(&self, analysis: &mut Analysis) {
        plugins::apply(analysis, &self.plugins);
    }
}
