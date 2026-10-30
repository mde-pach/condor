//! Random edit sequences: after every step, the long-running modes report
//! exactly what a full scan reports. Deterministic: seeds 1 to
//! `CONDOR_TEST_SEEDS` (default 8).

use std::path::{Path, PathBuf};

use condor::engine::Engine;
use condor::query::Engines;
use condor::run::{Output, run, select};

/// xorshift64*: enough randomness for test inputs, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

const FILES: [&str; 6] = ["a.py", "b.py", "pkg/__init__.py", "pkg/c.py", "pkg/d.py", "tests/test_e.py"];
const NAMES: [&str; 5] = ["alpha", "beta", "gamma", "delta", "omega"];
const CLASSES: [&str; 3] = ["Model", "Color", "Thing"];

/// A random module built from snippets that define and use shared names, so
/// an edit in one file changes what is unused in others.
fn module(rng: &mut Rng, syntax_errors: bool) -> String {
    let mut out = String::new();
    for _ in 0..rng.below(5) {
        let (n, m, c) = (rng.pick(&NAMES), rng.pick(&NAMES), rng.pick(&CLASSES));
        let snippet = match rng.below(13) {
            0 => format!("def {n}():\n    return {m}()\n"),
            1 => format!("def {n}(x):\n    return {n}(x)\n"),
            2 => format!("from pydantic import BaseModel\n\n\nclass {c}(BaseModel):\n    {n}: int\n"),
            3 => format!("import enum\n\n\nclass {c}(enum.Enum):\n    {n} = 1\n"),
            4 => format!("class {c}:\n    def {n}(self):\n        return self.{n}()\n\n    def {m}(self, unused_{m}):\n        return {c}\n"),
            5 => format!("from pkg.c import {n}\n"),
            6 => format!("{n}()\n"),
            7 => format!("print({c}.{n})\n"),
            8 => format!("{n} = 1  # noqa: V107\n"),
            9 => format!("import pytest\n\n\n@pytest.fixture\ndef {n}():\n    return 1\n\n\ndef test_{m}({n}):\n    pass\n"),
            10 => format!("def {n}():\n    return 1\n    {m}()\n"),
            11 if syntax_errors => "def broken(:\n".to_string(),
            _ => format!("from .d import {n} as {m}\nprint({m})\n"),
        };
        out.push_str(&snippet);
        out.push('\n');
    }
    out
}

fn seeds() -> std::ops::RangeInclusive<u64> {
    1..=std::env::var("CONDOR_TEST_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(8)
}

fn project(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    condor_core::paths::resolve(&dir, &dir)
}

fn write(dir: &Path, file: &str, text: &str) {
    let path = dir.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn oneshot(dir: &Path) -> (i32, String, String) {
    let mut out = Output::default();
    let code = run(&[".".to_string()], dir, &mut out);
    (code, String::from_utf8(out.stdout).unwrap(), String::from_utf8(out.stderr).unwrap())
}

#[test]
fn query_matches_a_full_scan_after_every_edit() {
    for seed in seeds() {
        let dir = project(&format!("incremental_query_{seed}"));
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut engines = Engines::default();
        for step in 0..60 {
            let file = FILES[rng.below(FILES.len())];
            match rng.below(10) {
                0 => {
                    let _ = std::fs::remove_file(dir.join(file));
                }
                1 => {
                    let ignore = rng.pick(&NAMES);
                    write(&dir, "pyproject.toml", &format!("[tool.condor]\nignore_names = [\"{ignore}\"]\n"));
                }
                2 => {
                    let _ = std::fs::remove_file(dir.join("pyproject.toml"));
                }
                3 => {
                    // Same size, different content.
                    if let Ok(text) = std::fs::read_to_string(dir.join(file)) {
                        let swapped: String = text.chars().map(|c| match c { 'a' => 'o', 'o' => 'a', c => c }).collect();
                        write(&dir, file, &swapped);
                    }
                }
                _ => write(&dir, file, &module(&mut rng, true)),
            }
            let mut out = Output::default();
            let code = engines.query(&[], &dir, &mut out);
            let got = (code, String::from_utf8(out.stdout).unwrap(), String::from_utf8(out.stderr).unwrap());
            assert_eq!(got, oneshot(&dir), "seed {seed}, step {step}");
        }
    }
}

#[test]
fn editor_buffers_match_a_full_scan_of_their_contents() {
    for seed in seeds() {
        let disk = project(&format!("incremental_disk_{seed}"));
        let mirror = project(&format!("incremental_mirror_{seed}"));
        let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03));
        let config = condor::config::Config { paths: vec![".".into()], ..Default::default() };
        let mut engine = Engine::new(config.clone(), &disk).unwrap();
        for step in 0..60 {
            let file = FILES[rng.below(FILES.len())];
            match rng.below(6) {
                // An unsaved buffer (new files included).
                0 | 1 => {
                    let text = module(&mut rng, false);
                    engine.set_overlay(disk.join(file), Some(text.clone()));
                    write(&mirror, file, &text);
                }
                // The buffer is closed: the file on disk counts again.
                2 => {
                    engine.set_overlay(disk.join(file), None);
                    match std::fs::read_to_string(disk.join(file)) {
                        Ok(text) => write(&mirror, file, &text),
                        Err(_) => {
                            let _ = std::fs::remove_file(mirror.join(file));
                        }
                    }
                }
                // Saved on disk; an open buffer still wins.
                _ => {
                    let text = module(&mut rng, false);
                    write(&disk, file, &text);
                    if engine.overlay(&disk.join(file)).is_none() {
                        write(&mirror, file, &text);
                    }
                }
            }
            engine.refresh(true).unwrap();
            let analysis = engine.analyze(&mut Output::default(), true).unwrap();
            let got: Vec<String> =
                select(&analysis, &config, &disk).iter().map(|i| i.report(&disk, false) + "\n").collect();
            let (_, expected, _) = oneshot(&mirror);
            assert_eq!(got.join(""), expected, "seed {seed}, step {step}");
        }
    }
}
