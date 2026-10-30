//! Whole-project aggregation: which definitions are unused, in vulture's order.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::item::{Item, ItemKind};

/// Where a log line goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// Everything one file contributes to the analysis. Files are independent, so
/// they can be scanned in parallel (and, later, re-scanned one at a time).
#[derive(Clone, Debug, Default)]
pub struct FileScan {
    /// Definitions and unreachable code, in the order vulture records them.
    pub items: Vec<Item>,
    /// Names counted as used, in the order vulture records them.
    pub uses: Vec<String>,
    /// Output lines produced while scanning (errors, and verbose logs).
    pub log: Vec<(Stream, String)>,
    /// The file could not be read or parsed (exit code 1).
    pub invalid_input: bool,
    /// Classes defined in the file, for rules that follow class hierarchies.
    pub classes: Vec<ClassFacts>,
    /// Functions and methods defined in the file.
    pub functions: Vec<FunctionFacts>,
    /// Top-level names of the modules the file imports.
    pub imports: Vec<String>,
    /// Imported names: local name -> dotted name it refers to (relative
    /// imports keep their dots), for following re-exports.
    pub aliases: Vec<(String, String)>,
    /// The file has a star import, so names it uses may come from anywhere.
    pub star_import: bool,
    /// String arguments of calls matching `ScanOptions::call_hints`, which
    /// count as names requested from a framework.
    pub requested: Vec<String>,
    /// Entries of a `--make-whitelist` file found in this file.
    pub whitelist_entries: Vec<WhitelistEntry>,
    /// Every name the file defines, including those not reported (ignored,
    /// `# noqa`, or used by a framework), for checking whitelist entries.
    pub defined_names: Vec<String>,
    /// The scanned file.
    pub file: Option<std::sync::Arc<std::path::Path>>,
}

/// A class. Names of bases and decorators are dotted and resolved through
/// the file's imports where possible (`BaseModel` imported from `pydantic`
/// is `pydantic.BaseModel`).
#[derive(Clone, Debug, Default)]
pub struct ClassFacts {
    pub name: String,
    pub bases: Vec<String>,
    pub decorators: Vec<Decorator>,
    /// Index of the class's item in `FileScan::items`, if it was defined.
    pub item: Option<usize>,
    /// The enclosing class (index in `FileScan::classes`), for nested classes.
    pub class: Option<usize>,
    /// Items for the attributes its body defines that have a tag.
    pub fields: Vec<usize>,
}

/// A function or method.
#[derive(Clone, Debug, Default)]
pub struct FunctionFacts {
    pub name: String,
    /// The class whose body defines it (index in `FileScan::classes`), for
    /// methods.
    pub class: Option<usize>,
    pub decorators: Vec<Decorator>,
    /// Index of the function's item in `FileScan::items`, if it was defined.
    pub item: Option<usize>,
    /// Parameter names, and the indices of the parameters' items.
    pub params: Vec<(String, Option<usize>)>,
}

/// A decorator: its resolved dotted name (without `@`) and the keyword
/// arguments given as literals (`autouse=True` is `("autouse", "true")`).
#[derive(Clone, Debug, Default)]
pub struct Decorator {
    pub name: String,
    pub keywords: Vec<(String, String)>,
    /// The decorator's first name comes from an import (`app` in
    /// `from myapp import app; @app.route`), not from the file itself.
    pub imported: bool,
}

/// A line written by `--make-whitelist`: `name  # unused <type> (<path>:<line>)`.
#[derive(Clone, Debug)]
pub struct WhitelistEntry {
    pub name: String,
    /// The definition's path as written in the entry.
    pub path: String,
    /// The line of the entry in the whitelist file.
    pub line: u32,
}

/// The project-wide result of scanning all files.
#[derive(Default)]
pub struct Analysis {
    pub scans: Vec<FileScan>,
}

impl Analysis {
    pub fn push(&mut self, scan: FileScan) {
        self.scans.push(scan);
    }

    pub fn invalid_input(&self) -> bool {
        self.scans.iter().any(|s| s.invalid_input)
    }

    pub fn used_names(&self) -> FxHashSet<&str> {
        self.scans.iter().flat_map(|s| s.uses.iter().map(String::as_str)).collect()
    }

    pub fn defined(&self, kind: ItemKind) -> impl Iterator<Item = &Item> {
        self.scans.iter().flat_map(|s| s.items.iter()).filter(move |i| i.kind == kind)
    }

    /// vulture's `unused_<kind>` properties: deduplicated on
    /// `(file, first line, name)` (first occurrence wins), filtered by the used
    /// names and sorted by lowercase name. Unreachable code and stale
    /// whitelist entries are neither deduplicated nor filtered.
    pub fn unused(&self, kind: ItemKind, used: &FxHashSet<&str>) -> Vec<&Item> {
        if matches!(kind, ItemKind::UnreachableCode | ItemKind::StaleWhitelistEntry) {
            return self.defined(kind).collect();
        }
        let mut seen = FxHashSet::default();
        let mut items: Vec<&Item> = self
            .defined(kind)
            .filter(|i| seen.insert(i.key()))
            .filter(|i| !used.contains(i.name.as_str()))
            .collect();
        // vulture iterates a hash set before this stable sort, so the relative
        // order of names that only differ in case is unspecified; condor sorts
        // them by name for determinism.
        items.sort_by_cached_key(|i| (py_lower(&i.name), i.name.clone()));
        items
    }

    /// Check `--make-whitelist` entries against the definitions. An entry is
    /// stale when the file it names was scanned but no longer defines the
    /// name. A stale entry is reported, and its use no longer counts, so it
    /// cannot hide unrelated dead code with the same name.
    pub fn check_whitelist_entries(&mut self, cwd: &std::path::Path) {
        let named: FxHashSet<String> =
            self.scans.iter().flat_map(|s| s.whitelist_entries.iter().map(|e| e.name.clone())).collect();
        if named.is_empty() {
            return;
        }
        let defined: FxHashSet<(&std::path::Path, &str)> = self
            .scans
            .iter()
            .filter_map(|s| s.file.as_deref().map(|f| (f, s)))
            .flat_map(|(f, s)| s.defined_names.iter().filter(|n| named.contains(*n)).map(move |n| (f, n.as_str())))
            .collect();
        let scanned: FxHashSet<&std::path::Path> = self.scans.iter().filter_map(|s| s.file.as_deref()).collect();
        // Entry paths are as `--make-whitelist` wrote them: relative to the
        // directory it ran in, which may be this one or the whitelist's.
        let mut stale: Vec<(usize, WhitelistEntry)> = Vec::new();
        for (index, scan) in self.scans.iter().enumerate() {
            let Some(file) = scan.file.as_deref() else { continue };
            let bases = [Some(cwd), file.parent()];
            for entry in &scan.whitelist_entries {
                let target = bases
                    .iter()
                    .flatten()
                    .map(|base| crate::paths::resolve(std::path::Path::new(&entry.path), base))
                    .find(|p| scanned.contains(p.as_path()));
                if let Some(target) = target
                    && !defined.contains(&(target.as_path(), entry.name.as_str())) {
                        stale.push((index, entry.clone()));
                    }
            }
        }
        for (index, entry) in stale {
            let scan = &mut self.scans[index];
            let file = scan.file.clone().expect("checked above");
            scan.items.push(Item {
                name: entry.name.clone(),
                kind: ItemKind::StaleWhitelistEntry,
                file,
                first_line: entry.line,
                last_line: entry.line,
                message: Some(format!("stale whitelist entry '{}': {} no longer defines it", entry.name, entry.path)),
                confidence: 100,
                tag: 0,
            });
            if let Some(pos) = scan.uses.iter().position(|u| *u == entry.name) {
                scan.uses.remove(pos);
            }
        }
    }

    /// `Vulture.get_unused_code`.
    pub fn unused_code(&self, min_confidence: i64, sort_by_size: bool) -> Vec<&Item> {
        let used = self.used_names();
        let mut all: Vec<&Item> = ItemKind::ALL
            .iter()
            .flat_map(|&k| self.unused(k, &used))
            .filter(|i| i64::from(i.confidence) >= min_confidence)
            .collect();
        let mut lower_paths: FxHashMap<*const std::path::Path, String> = FxHashMap::default();
        for i in &all {
            lower_paths
                .entry(std::sync::Arc::as_ptr(&i.file))
                .or_insert_with(|| py_lower(&i.file.to_string_lossy()));
        }
        let path_key = |i: &Item| lower_paths[&std::sync::Arc::as_ptr(&i.file)].clone();
        if sort_by_size {
            all.sort_by_cached_key(|i| (i.size(), path_key(i), i.first_line));
        } else {
            all.sort_by_cached_key(|i| (path_key(i), i.first_line));
        }
        all
    }
}

/// Python's `str.lower()` (full Unicode lowercase mapping, final-sigma aware).
pub fn py_lower(s: &str) -> String {
    s.to_lowercase()
}
