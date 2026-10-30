//! `condor query [OPTIONS] [FILES]`: the command line's report for the whole
//! project (the configured paths, or `.`), limited to `FILES` when given.
//! Answered by a background `condor daemon` that keeps the analysis between
//! calls; without one, the project is scanned in-process.

use std::path::Path;

use condor_core::Stream;
use condor_core::paths::resolve;

use crate::args;
use crate::config::{self, Config, Outcome};
use crate::engine::Engine;
use crate::run::{self, INVALID_CMDLINE, INVALID_INPUT, Output};

/// Engines kept by a daemon, one per configuration (at most a few).
#[derive(Default)]
pub struct Engines {
    engines: Vec<Engine>,
}

impl Engines {
    /// Answer `condor query ARGV` in `cwd`.
    pub fn query(&mut self, argv: &[String], cwd: &Path, out: &mut Output) -> i32 {
        let mut cli = match args::parse(argv) {
            Ok(cli) => cli,
            Err(exit) => return run::emit(exit, out),
        };
        let files: Vec<_> = cli.paths.take().unwrap_or_default().iter().map(|f| resolve(Path::new(f), cwd)).collect();
        cli.default_paths = vec![".".into()];
        let config = match config::make_config(cli, cwd, None) {
            Outcome::Config(config, banner) => {
                if let Some(b) = banner {
                    out.line(Stream::Stdout, &b);
                }
                config
            }
            Outcome::InputError(msg, banner) => {
                if let Some(b) = banner {
                    out.line(Stream::Stdout, &b);
                }
                out.line(Stream::Stderr, &msg);
                return INVALID_CMDLINE;
            }
            Outcome::Exit(exit) => return run::emit(exit, out),
        };
        let engine = match self.get(config, cwd) {
            Ok(engine) => engine,
            Err(msg) => {
                out.line(Stream::Stderr, &msg);
                return INVALID_CMDLINE;
            }
        };
        if let Err(msg) = engine.refresh() {
            out.line(Stream::Stderr, &msg);
            return INVALID_INPUT;
        }
        let analysis = match engine.analyze(out, true) {
            Ok(analysis) => analysis,
            Err(code) => return code,
        };
        run::report_where(&analysis, &engine.config, cwd, out, |item| {
            files.is_empty() || files.iter().any(|f| item.file.starts_with(f))
        })
    }

    /// The engine for `config`, built if needed.
    pub fn get(&mut self, config: Config, cwd: &Path) -> Result<&mut Engine, String> {
        let found = self.engines.iter().position(|e| e.config == config && e.cwd == cwd && !e.plugins_changed());
        let index = match found {
            Some(index) => index,
            None => {
                let engine = Engine::new(config, cwd)?;
                if self.engines.len() >= 4 {
                    self.engines.remove(0);
                }
                self.engines.push(engine);
                self.engines.len() - 1
            }
        };
        Ok(&mut self.engines[index])
    }
}

/// `condor query`: ask the daemon, starting it if needed, or scan
/// in-process if that fails.
pub fn main(argv: &[String], cwd: &Path, out: &mut Output) -> i32 {
    #[cfg(unix)]
    if let Some(code) = crate::daemon::ask(argv, cwd, out) {
        return code;
    }
    Engines::default().query(argv, cwd, out)
}
