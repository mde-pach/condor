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
    /// Log lines vulture prints on stdout go to stderr, so that stdout only
    /// holds the report (JSON and SARIF).
    pub logs_to_stderr: bool,
    /// Errors about files that could not be analysed.
    pub errors: Vec<String>,
}

impl Output {
    pub fn line(&mut self, stream: Stream, text: &str) {
        let buf = match stream {
            Stream::Stdout => &mut self.stdout,
            Stream::Stderr => &mut self.stderr,
        };
        buf.extend_from_slice(text.as_bytes());
        buf.push(b'\n');
    }

    /// A log line (verbose output, configuration banner).
    pub fn log(&mut self, stream: Stream, text: &str) {
        let stream = if self.logs_to_stderr { Stream::Stderr } else { stream };
        self.line(stream, text);
    }

    /// An error about an input file.
    pub fn error(&mut self, text: &str) {
        self.errors.push(text.to_string());
        self.line(Stream::Stderr, text);
    }

    fn scan_log(&mut self, scan: &FileScan) {
        for (stream, text) in &scan.log {
            match stream {
                Stream::Stderr => self.error(text),
                Stream::Stdout => self.log(*stream, text),
            }
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

/// What one module contributes.
#[allow(clippy::large_enum_variant)] // nearly every module is `Scanned`
pub enum ModuleResult {
    Excluded,
    Scanned(FileScan),
    /// Not decodable: vulture logs an error and continues.
    Unreadable,
    /// Not readable at all (a directory named `x.py`, permissions): vulture
    /// crashes with a traceback at this point.
    Crash(String),
}

/// Scan one module; `source` replaces the file's content when given.
pub fn scan_module(
    lang: &Python,
    module: &Path,
    source: Option<&str>,
    exclude: &PatternSet,
    opts: &ScanOptions,
) -> ModuleResult {
    if exclude.matches_normcase(&module.to_string_lossy()) {
        return ModuleResult::Excluded;
    }
    if let Some(source) = source {
        return ModuleResult::Scanned(lang.scan(source, module, opts));
    }
    match std::fs::read(module) {
        Ok(bytes) => match condor_python::read::decode_source(&bytes) {
            Ok(source) => ModuleResult::Scanned(lang.scan(&source, module, opts)),
            Err(_) => ModuleResult::Unreadable,
        },
        Err(e) => ModuleResult::Crash(e.to_string()),
    }
}

/// Write what vulture prints for `module` and add its scan to `analysis`.
/// Returns the exit code if vulture would have crashed.
pub fn add_module(
    module: &Path,
    result: &ModuleResult,
    opts: &ScanOptions,
    analysis: &mut Analysis,
    out: &mut Output,
) -> Result<(), i32> {
    let shown = module.display();
    if opts.verbose {
        let verb = if matches!(result, ModuleResult::Excluded) { "Excluded" } else { "Scanning" };
        out.log(Stream::Stdout, &format!("{verb}: {shown}"));
    }
    match result {
        ModuleResult::Excluded => {}
        ModuleResult::Scanned(scan) => {
            out.scan_log(scan);
            analysis.push(scan.clone());
        }
        ModuleResult::Unreadable => {
            out.error(&format!("Error: Could not read file {shown} - \nTry to change the encoding to UTF-8."));
            analysis.push(FileScan { invalid_input: true, ..FileScan::default() });
        }
        ModuleResult::Crash(err) => {
            out.error(&format!("Error: {shown}: {err}"));
            return Err(INVALID_INPUT);
        }
    }
    Ok(())
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
    let results: Vec<ModuleResult> =
        modules.par_iter().map(|module| scan_module(lang, module, None, exclude, opts)).collect();
    for (module, result) in modules.iter().zip(&results) {
        add_module(module, result, opts, analysis, out)?;
    }
    add_whitelists(lang, exclude, opts, analysis, out, |path, source| lang.scan(source, Path::new(path), opts));
    Ok(())
}

/// Add the bundled whitelists for the modules the analysis imports; `scan`
/// scans one of them.
pub fn add_whitelists(
    lang: &Python,
    exclude: &PatternSet,
    opts: &ScanOptions,
    analysis: &mut Analysis,
    out: &mut Output,
    mut scan: impl FnMut(&str, &str) -> FileScan,
) {
    for (path, source) in lang.implicit_sources(analysis) {
        if exclude.matches_normcase(&path) {
            if opts.verbose {
                out.log(Stream::Stdout, &format!("Excluded whitelist: {path}"));
            }
            continue;
        }
        let Some(source) = source else { continue };
        if opts.verbose {
            out.log(Stream::Stdout, &format!("Included whitelist: {path}"));
        }
        let scan = scan(&path, source);
        out.scan_log(&scan);
        analysis.push(scan);
    }
}

/// Once every file is scanned: entry points, the language's project-wide
/// rules, and stale whitelist entries.
pub fn finish(lang: &Python, analysis: &mut Analysis, config: &Config, cwd: &Path) {
    // First, while items keep the indices the scan gave them.
    lang.link(analysis);
    // An entry point uses its object in its module's file; if that file is
    // not scanned, any definition with its names.
    for entry in &config.entry_points {
        let mut found = false;
        for scan in &mut analysis.scans {
            let Some(file) = scan.file.as_deref() else { continue };
            if condor_core::paths::module_names(file).contains(&entry.module) {
                found = true;
                scan.items.retain(|i| !entry.path.contains(&i.name));
            }
        }
        if !found {
            analysis.push(FileScan { uses: entry.path.clone(), ..FileScan::default() });
        }
    }
    analysis.check_whitelist_entries(cwd);
}

/// The unused code to report, after `--min-confidence`, `--only` and
/// `--no-report`.
pub fn select<'a>(analysis: &'a Analysis, config: &Config, cwd: &Path) -> Vec<&'a Item> {
    let only: Vec<ItemKind> = config.only.iter().filter_map(|k| ItemKind::parse(k)).collect();
    let no_report = exclude_patterns(&config.no_report);
    analysis
        .unused_code(config.min_confidence, config.sort_by_size)
        .into_iter()
        .filter(|i| only.is_empty() || only.contains(&i.kind))
        .filter(|i| config.no_report.is_empty() || !no_report.matches_normcase(&resolve(&i.file, cwd).to_string_lossy()))
        .collect()
}

/// Print the unused code and return the exit code.
pub fn report(analysis: &Analysis, config: &Config, cwd: &Path, out: &mut Output) -> i32 {
    report_where(analysis, config, cwd, out, |_| true)
}

/// [`report`], for the items `keep` accepts.
pub fn report_where(
    analysis: &Analysis,
    config: &Config,
    cwd: &Path,
    out: &mut Output,
    keep: impl Fn(&Item) -> bool,
) -> i32 {
    if !(0..=100).contains(&config.min_confidence) {
        out.line(Stream::Stderr, "Error: --min-confidence must be between 0 and 100.");
        return INVALID_INPUT;
    }
    let items: Vec<&Item> = select(analysis, config, cwd).into_iter().filter(|i| keep(i)).collect();
    match config.format.as_str() {
        "json" => out.line(Stream::Stdout, &crate::output::json(&items, cwd)),
        "sarif" => {
            let log = crate::output::sarif(&items, cwd, &out.errors);
            out.line(Stream::Stdout, &log);
        }
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

/// The project's plugin files (`plugins` in the configuration: files, or
/// directories searched for `.toml` and `.json` files; relative to the
/// configuration file, or the current directory).
pub fn plugin_files(config: &Config, cwd: &Path) -> Result<Vec<PathBuf>, String> {
    fn walk(dir: &Path, entry: &str, out: &mut Vec<PathBuf>) -> Result<(), String> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| format!("Error: cannot read plugin directory {entry}: {e}"))?
            .flatten()
            .map(|e| e.path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, entry, out)?;
            } else if condor_python::plugins::format_of(&path).is_some()
                && path.file_name().is_some_and(|n| n != "schema.json")
            {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    for entry in &config.plugins {
        let path = cwd.join(entry);
        if path.is_dir() {
            walk(&path, entry, &mut files)?;
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

/// The Python front end with the bundled plugins and the project's own. A
/// project plugin with the name of a bundled one replaces it.
pub fn load_python(config: &Config, cwd: &Path) -> Result<Python, String> {
    let mut python = Python::bundled();
    let mut own: Vec<String> = Vec::new();
    for file in plugin_files(config, cwd)? {
        let content = std::fs::read_to_string(&file)
            .map_err(|e| format!("Error: cannot read plugin {}: {e}", file.display()))?;
        let plugin = condor_python::plugins::parse(&file.to_string_lossy(), &content)
            .map_err(|e| format!("Error: invalid plugin {e}"))?;
        let name = plugin.plugin.name.clone();
        if own.contains(&name) {
            return Err(format!("Error: two plugins are named \"{name}\" ({})", file.display()));
        }
        python.plugins.retain(|p| p.plugin.name != name);
        own.push(name);
        python.plugins.push(plugin);
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
    out.logs_to_stderr = config.format != "text";
    if let Some(b) = banner {
        out.log(Stream::Stdout, &b);
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

pub fn emit(exit: Exit, out: &mut Output) -> i32 {
    out.stdout.extend_from_slice(exit.stdout.as_bytes());
    out.stderr.extend_from_slice(exit.stderr.as_bytes());
    exit.code
}

/// Stack size of the threads that parse and scan files. Parsing recurses
/// on nesting, and the default 2 MiB overflows (aborting the process) on
/// code nested a few thousand levels deep; the memory is only reserved.
const SCAN_STACK: usize = 512 << 20;

/// Entry point of the binary.
pub fn main() -> i32 {
    let _ = rayon::ThreadPoolBuilder::new().stack_size(SCAN_STACK).build_global();
    let argv: Vec<String> = std::env::args_os().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
    // Without a working directory, relative paths have no meaning: vulture
    // fails here too (and "." must never become "/").
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("Error: cannot determine the current directory: {e}");
            return INVALID_INPUT;
        }
    };
    let mut out = Output::default();
    // Commands come first; scan a directory with one of these names as `./server`.
    let code = match argv.first().map(String::as_str) {
        Some("server") => return crate::server::main(&argv[1..], &cwd),
        #[cfg(unix)]
        Some("daemon") if argv.len() > 1 => {
            eprintln!("Error: condor daemon takes no arguments; condor query starts it");
            return INVALID_CMDLINE;
        }
        #[cfg(unix)]
        Some("daemon") => return crate::daemon::main(&cwd),
        Some("query") => crate::query::main(&argv[1..], &cwd, &mut out),
        _ => run(&argv, &cwd, &mut out),
    };
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
