//! The `condor` command: vulture's `main()`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use condor_core::fnmatch::PatternSet;
use condor_core::paths::resolve;
use condor_core::{Analysis, FileScan, Item, ItemKind, Language, ScanOptions, Stream};
use condor_python::Python;
use rayon::prelude::*;

use crate::args::{self, Exit};
use crate::config::{self, Config, Outcome};
use crate::discover;

/// Exit codes (vulture `utils.ExitCode`).
pub const NO_DEAD_CODE: i32 = 0;
pub const INVALID_INPUT: i32 = 1;
pub const INVALID_CMDLINE: i32 = 2;
pub const DEAD_CODE: i32 = 3;

/// Collected output. vulture writes to stdout and stderr as it goes; condor
/// buffers per file so files can be scanned in parallel, and writes in the
/// same order.
#[derive(Default)]
pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Output {
    fn line(&mut self, stream: Stream, text: &str) {
        let buf = match stream {
            Stream::Stdout => &mut self.stdout,
            Stream::Stderr => &mut self.stderr,
        };
        buf.extend_from_slice(text.as_bytes());
        buf.push(b'\n');
    }

    fn scan_log(&mut self, scan: &FileScan) {
        for (stream, text) in &scan.log {
            self.line(*stream, text);
        }
    }
}

/// `prepare_pattern`: a pattern without wildcards matches anywhere.
pub fn exclude_patterns(exclude: &[String]) -> PatternSet {
    let prepared: Vec<String> = exclude
        .iter()
        .map(|p| if p.contains(['*', '?', '[']) { p.clone() } else { format!("*{p}*") })
        .collect();
    PatternSet::new(&prepared)
}

/// What one module contributes, computed in parallel.
enum ModuleResult {
    Excluded,
    Scanned(FileScan),
    /// Not decodable: vulture logs an error and continues.
    Unreadable,
    /// Not readable at all (a directory named `x.py`, permissions): vulture
    /// crashes with a traceback at this point.
    Crash(String),
}

/// Scan `modules` and the bundled whitelists, writing logs to `out`. Returns
/// the analysis, or the exit code if vulture would have crashed.
pub fn scavenge(
    lang: &Python,
    modules: &[PathBuf],
    exclude: &PatternSet,
    opts: &ScanOptions,
    analysis: &mut Analysis,
    out: &mut Output,
) -> Result<(), i32> {
    let results: Vec<ModuleResult> = modules
        .par_iter()
        .map(|module| {
            if exclude.matches_normcase(&module.to_string_lossy()) {
                return ModuleResult::Excluded;
            }
            match std::fs::read(module) {
                Ok(bytes) => match condor_python::read::decode_source(&bytes) {
                    Ok(source) => ModuleResult::Scanned(lang.scan(&source, module, opts)),
                    Err(_) => ModuleResult::Unreadable,
                },
                Err(e) => ModuleResult::Crash(e.to_string()),
            }
        })
        .collect();

    for (module, result) in modules.iter().zip(results) {
        let shown = module.display();
        match result {
            ModuleResult::Excluded => {
                if opts.verbose {
                    out.line(Stream::Stdout, &format!("Excluded: {shown}"));
                }
            }
            ModuleResult::Scanned(scan) => {
                if opts.verbose {
                    out.line(Stream::Stdout, &format!("Scanning: {shown}"));
                }
                out.scan_log(&scan);
                analysis.push(scan);
            }
            ModuleResult::Unreadable => {
                if opts.verbose {
                    out.line(Stream::Stdout, &format!("Scanning: {shown}"));
                }
                out.line(
                    Stream::Stderr,
                    &format!("Error: Could not read file {shown} - \nTry to change the encoding to UTF-8."),
                );
                analysis.push(FileScan { invalid_input: true, ..FileScan::default() });
            }
            ModuleResult::Crash(err) => {
                if opts.verbose {
                    out.line(Stream::Stdout, &format!("Scanning: {shown}"));
                }
                out.line(Stream::Stderr, &format!("Error: {shown}: {err}"));
                return Err(INVALID_INPUT);
            }
        }
    }

    for (path, source) in lang.implicit_sources(analysis) {
        if exclude.matches_normcase(&path) {
            if opts.verbose {
                out.line(Stream::Stdout, &format!("Excluded whitelist: {path}"));
            }
            continue;
        }
        let Some(source) = source else { continue };
        if opts.verbose {
            out.line(Stream::Stdout, &format!("Included whitelist: {path}"));
        }
        let scan = lang.scan(source, Path::new(&path), opts);
        out.scan_log(&scan);
        analysis.push(scan);
    }
    Ok(())
}

/// Once every file is scanned: entry points, the language's project-wide
/// rules, and stale whitelist entries.
pub fn finish(lang: &Python, analysis: &mut Analysis, config: &Config, cwd: &Path) {
    if !config.entry_point_names.is_empty() {
        analysis.push(FileScan { uses: config.entry_point_names.clone(), ..FileScan::default() });
    }
    lang.link(analysis);
    analysis.check_whitelist_entries(cwd);
}

/// Print the unused code and return the exit code.
pub fn report(analysis: &Analysis, config: &Config, cwd: &Path, out: &mut Output) -> i32 {
    if !(0..=100).contains(&config.min_confidence) {
        out.line(Stream::Stderr, "Error: --min-confidence must be between 0 and 100.");
        return INVALID_INPUT;
    }
    let only: Vec<ItemKind> = config.only.iter().filter_map(|k| ItemKind::parse(k)).collect();
    let no_report = exclude_patterns(&config.no_report);
    let items: Vec<&Item> = analysis
        .unused_code(config.min_confidence, config.sort_by_size)
        .into_iter()
        .filter(|i| only.is_empty() || only.contains(&i.kind))
        .filter(|i| config.no_report.is_empty() || !no_report.matches_normcase(&resolve(&i.file, cwd).to_string_lossy()))
        .collect();
    match config.format.as_str() {
        "json" => out.line(Stream::Stdout, &crate::output::json(&items, cwd)),
        "sarif" => out.line(Stream::Stdout, &crate::output::sarif(&items, cwd)),
        _ => {
            for item in &items {
                let line = if config.make_whitelist {
                    item.whitelist_string(cwd)
                } else {
                    item.report(cwd, config.sort_by_size)
                };
                out.line(Stream::Stdout, &line);
            }
        }
    }
    if !items.is_empty() {
        DEAD_CODE
    } else if analysis.invalid_input() {
        INVALID_INPUT
    } else {
        NO_DEAD_CODE
    }
}

/// The Python front end with the bundled plugins and the project's own
/// (`plugins` in the configuration: files, or directories of `.toml` and
/// `.json` files, relative to the current directory).
pub fn load_python(config: &Config, cwd: &Path) -> Result<Python, String> {
    let mut python = Python::bundled();
    for entry in &config.plugins {
        let path = cwd.join(entry);
        let mut files = Vec::new();
        if path.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(&path)
                .map_err(|e| format!("Error: cannot read plugin directory {entry}: {e}"))?
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "toml" || e == "json"))
                .collect();
            entries.sort();
            files.extend(entries);
        } else {
            files.push(path);
        }
        for file in files {
            let content = std::fs::read_to_string(&file)
                .map_err(|e| format!("Error: cannot read plugin {}: {e}", file.display()))?;
            let plugin = condor_python::plugins::parse(&file.to_string_lossy(), &content)
                .map_err(|e| format!("Error: invalid plugin {e}"))?;
            python.plugins.push(plugin);
        }
    }
    Ok(python)
}

pub fn scan_options(lang: &Python, config: &Config, cwd: &Path) -> ScanOptions {
    ScanOptions {
        call_hints: condor_python::plugins::call_hints(&lang.plugins),
        verbose: config.verbose,
        cwd: cwd.to_path_buf(),
        ignore_names: PatternSet::new(&config.ignore_names),
        ignore_decorators: PatternSet::new(&config.ignore_decorators),
    }
}

/// Run condor with `argv` (without the program name). Returns the exit code.
pub fn run(argv: &[String], cwd: &Path, out: &mut Output) -> i32 {
    let cli = match args::parse(argv) {
        Ok(cli) => cli,
        Err(exit) => return emit(exit, out),
    };
    let (config, banner) = match config::make_config(cli, cwd, None) {
        Outcome::Config(c, b) => (c, b),
        Outcome::InputError(msg, banner) => {
            if let Some(b) = banner {
                out.line(Stream::Stdout, &b);
            }
            out.line(Stream::Stderr, &msg);
            return INVALID_CMDLINE;
        }
        Outcome::Exit(exit) => return emit(exit, out),
    };
    if let Some(b) = banner {
        out.line(Stream::Stdout, &b);
    }
    let modules = match discover::get_modules(&config.paths, cwd) {
        Ok(m) => m,
        Err(msg) => {
            out.line(Stream::Stderr, &msg);
            return INVALID_INPUT;
        }
    };
    let python = match load_python(&config, cwd) {
        Ok(p) => p,
        Err(msg) => {
            out.line(Stream::Stderr, &msg);
            return INVALID_CMDLINE;
        }
    };
    let opts = scan_options(&python, &config, cwd);
    let exclude = exclude_patterns(&config.exclude);
    let mut analysis = Analysis::default();
    if let Err(code) = scavenge(&python, &modules, &exclude, &opts, &mut analysis, out) {
        return code;
    }
    finish(&python, &mut analysis, &config, cwd);
    report(&analysis, &config, cwd, out)
}

fn emit(exit: Exit, out: &mut Output) -> i32 {
    out.stdout.extend_from_slice(exit.stdout.as_bytes());
    out.stderr.extend_from_slice(exit.stderr.as_bytes());
    exit.code
}

/// Entry point of the binary.
pub fn main() -> i32 {
    let argv: Vec<String> = std::env::args_os().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut out = Output::default();
    let code = run(&argv, &cwd, &mut out);
    // Output is best effort: a closed pipe must not turn into a crash.
    let _ = std::io::stdout().lock().write_all(&out.stdout);
    let _ = std::io::stderr().lock().write_all(&out.stderr);
    code
}

/// Shared by the CLI and the test bridge: scan code given as strings.
pub fn scan_source(source: &str, path: &str, opts: &ScanOptions) -> FileScan {
    let path: Arc<Path> = Arc::from(Path::new(path));
    condor_python::scanner::scan(source, path, opts)
}
