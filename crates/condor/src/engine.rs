//! Incremental analysis for the long-running modes (`condor server`,
//! `condor daemon`): the project is scanned once, then only the files that
//! changed are scanned again.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use condor_core::fnmatch::PatternSet;
use condor_core::{Analysis, FileScan, Language, ScanOptions};
use condor_python::Python;
use rayon::prelude::*;
use rustc_hash::FxHashMap;

use crate::config::Config;
use crate::discover;
use crate::run::{self, ModuleResult, Output};

/// What a file looked like when it was scanned.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Stamp {
    Missing,
    Disk { modified: Option<SystemTime>, len: u64, inode: u64 },
    /// Contents given by an editor, by generation.
    Overlay(u64),
}

fn stamp(path: &Path) -> Stamp {
    let Ok(meta) = std::fs::metadata(path) else { return Stamp::Missing };
    #[cfg(unix)]
    let inode = std::os::unix::fs::MetadataExt::ino(&meta);
    #[cfg(not(unix))]
    let inode = 0;
    Stamp::Disk { modified: meta.modified().ok(), len: meta.len(), inode }
}

/// One project's analysis, kept up to date.
pub struct Engine {
    pub config: Config,
    pub cwd: PathBuf,
    lang: Python,
    opts: ScanOptions,
    exclude: PatternSet,
    plugins: Vec<(PathBuf, Stamp)>,
    modules: Vec<PathBuf>,
    scans: FxHashMap<PathBuf, (Stamp, ModuleResult)>,
    whitelists: FxHashMap<String, FileScan>,
    overlays: FxHashMap<PathBuf, (u64, String)>,
    generation: u64,
}

impl Engine {
    pub fn new(config: Config, cwd: &Path) -> Result<Engine, String> {
        let lang = run::load_python(&config, cwd)?;
        let opts = run::scan_options(&lang, &config, cwd);
        let exclude = run::exclude_patterns(&config.exclude);
        let plugins = run::plugin_files(&config, cwd)?.into_iter().map(|p| { let s = stamp(&p); (p, s) }).collect();
        Ok(Engine {
            config,
            cwd: cwd.to_path_buf(),
            lang,
            opts,
            exclude,
            plugins,
            modules: Vec::new(),
            scans: FxHashMap::default(),
            whitelists: FxHashMap::default(),
            overlays: FxHashMap::default(),
            generation: 0,
        })
    }

    /// Whether a project plugin file changed since the engine was built.
    pub fn plugins_changed(&self) -> bool {
        let now = run::plugin_files(&self.config, &self.cwd).unwrap_or_default();
        now.len() != self.plugins.len() || self.plugins.iter().zip(&now).any(|((p, s), n)| p != n || *s != stamp(n))
    }

    /// Use `text` instead of the file's content (an editor's unsaved
    /// buffer), or the file again with `None`. `path` must be resolved.
    pub fn set_overlay(&mut self, path: PathBuf, text: Option<String>) {
        match text {
            Some(text) => {
                self.generation += 1;
                self.overlays.insert(path, (self.generation, text));
            }
            None => {
                self.overlays.remove(&path);
            }
        }
    }

    pub fn overlay(&self, path: &Path) -> Option<&str> {
        self.overlays.get(path).map(|(_, t)| t.as_str())
    }

    /// Find the project's modules again and scan those that changed.
    pub fn refresh(&mut self) -> Result<(), String> {
        self.modules = discover::get_modules(&self.config.paths, &self.cwd)?;
        let current = |m: &PathBuf| match self.overlays.get(m) {
            Some((generation, _)) => Stamp::Overlay(*generation),
            None => stamp(m),
        };
        let fresh: Vec<(PathBuf, Stamp, ModuleResult)> = self
            .modules
            .par_iter()
            .filter_map(|m| {
                let now = current(m);
                match self.scans.get(m) {
                    Some((before, _)) if *before == now => None,
                    _ => {
                        let source = self.overlays.get(m).map(|(_, t)| t.as_str());
                        Some((m.clone(), now, run::scan_module(&self.lang, m, source, &self.exclude, &self.opts)))
                    }
                }
            })
            .collect();
        for (module, now, result) in fresh {
            self.scans.insert(module, (now, result));
        }
        if self.scans.len() > self.modules.len() {
            let keep: rustc_hash::FxHashSet<&PathBuf> = self.modules.iter().collect();
            self.scans.retain(|m, _| keep.contains(m));
        }
        Ok(())
    }

    /// The whole-project analysis, writing what the command line prints to
    /// `out`. With `strict`, a file that cannot be read stops the analysis
    /// with an exit code, as on the command line; otherwise it is skipped.
    pub fn analyze(&mut self, out: &mut Output, strict: bool) -> Result<Analysis, i32> {
        let mut analysis = Analysis::default();
        for module in &self.modules {
            let (_, result) = &self.scans[module];
            match run::add_module(module, result, &self.opts, &mut analysis, out) {
                Err(code) if strict => return Err(code),
                _ => {}
            }
        }
        let (lang, opts, whitelists) = (&self.lang, &self.opts, &mut self.whitelists);
        run::add_whitelists(lang, &self.exclude, opts, &mut analysis, out, |path, source| {
            whitelists.entry(path.to_string()).or_insert_with(|| lang.scan(source, Path::new(path), opts)).clone()
        });
        run::finish(&self.lang, &mut analysis, &self.config, &self.cwd);
        Ok(analysis)
    }
}
