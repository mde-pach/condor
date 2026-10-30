//! The bundled plugins' test cases (condor-plugins `tests/`, synced with the
//! plugins). Each case is a directory of Python files. A line ending with
//! `# dead: a, b` must be reported for those names; nothing else may be.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn cases_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../condor_python/plugins/tests/python")
}

/// Case directories: `<plugin>/<case>/`.
fn cases() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for plugin in sorted_dirs(&cases_root()) {
        out.extend(sorted_dirs(&plugin));
    }
    out
}

fn sorted_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn python_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            python_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "py") {
            out.push(path);
        }
    }
}

/// (file relative to the case, line, name) for each `# dead:` marker.
fn expected(case: &Path) -> BTreeSet<(String, u64, String)> {
    let mut files = Vec::new();
    python_files(case, &mut files);
    let mut out = BTreeSet::new();
    for file in files {
        let rel = file.strip_prefix(case).unwrap().to_string_lossy().replace('\\', "/");
        for (i, line) in std::fs::read_to_string(&file).unwrap().lines().enumerate() {
            if let Some((_, names)) = line.split_once("# dead:") {
                for name in names.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                    out.insert((rel.clone(), i as u64 + 1, name.to_string()));
                }
            }
        }
    }
    out
}

fn reported(case: &Path) -> BTreeSet<(String, u64, String)> {
    let out = Command::new(env!("CARGO_BIN_EXE_condor"))
        .args(["--format", "json", "."])
        .current_dir(case)
        .output()
        .unwrap();
    assert!(out.stderr.is_empty(), "{}: {}", case.display(), String::from_utf8_lossy(&out.stderr));
    let findings: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    findings
        .iter()
        .map(|f| {
            let path = f["path"].as_str().unwrap().replace('\\', "/").trim_start_matches("./").to_string();
            (path, f["line"].as_u64().unwrap(), f["name"].as_str().unwrap().to_string())
        })
        .collect()
}

#[test]
fn plugin_cases() {
    let all = cases();
    assert!(all.len() > 20, "plugin cases missing under {}", cases_root().display());
    let mut failures = Vec::new();
    for case in &all {
        let (want, got) = (expected(case), reported(case));
        if want != got {
            let name = case.strip_prefix(cases_root()).unwrap().display().to_string();
            let missed: Vec<_> = want.difference(&got).collect();
            let extra: Vec<_> = got.difference(&want).collect();
            failures.push(format!("{name}\n  not reported: {missed:?}\n  reported, not expected: {extra:?}"));
        }
    }
    assert!(failures.is_empty(), "{} of {} cases fail:\n{}", failures.len(), all.len(), failures.join("\n"));
}
