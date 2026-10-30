//! The `condor` command on small projects written to a temporary directory.

use std::path::{Path, PathBuf};

use condor::run::{Output, run};

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    dir
}

fn condor(dir: &Path, args: &[&str]) -> (i32, String) {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut out = Output::default();
    let code = run(&argv, dir, &mut out);
    (code, String::from_utf8(out.stdout).unwrap())
}

#[test]
fn i4_entry_points_are_used() {
    let dir = project(
        "entry_points",
        &[
            ("pyproject.toml", "[project.scripts]\ntool = \"pkg.cli:main\"\n[project.entry-points.'x.plugins']\np = \"pkg.cli:Plugin.load\"\n"),
            ("pkg/cli.py", "def main():\n    pass\n\nclass Plugin:\n    def load(self):\n        pass\n\ndef unused():\n    pass\n"),
        ],
    );
    let (code, out) = condor(&dir, &["pkg"]);
    assert_eq!(code, 3);
    assert_eq!(out, "pkg/cli.py:8: unused function 'unused' (60% confidence)\n");
}

#[test]
fn i8_json_and_sarif() {
    let dir = project("formats", &[("m.py", "def f():\n    pass\n")]);
    let (code, out) = condor(&dir, &["--format", "json", "m.py"]);
    assert_eq!(code, 3);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["name"], "f");
    assert_eq!(v[0]["kind"], "function");
    assert_eq!(v[0]["line"], 1);
    let (_, out) = condor(&dir, &["--format", "sarif", "m.py"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["runs"][0]["results"][0]["ruleId"], "V103");
    assert_eq!(condor(&dir, &["--format", "xml", "m.py"]).0, 2);
}

#[test]
fn i9_gitignore() {
    let dir = project(
        "gitignore",
        &[
            (".git/HEAD", "ref: refs/heads/main\n"),
            (".gitignore", ".venv/\nbuild/\n"),
            (".venv/lib/dep.py", "def in_venv():\n    pass\n"),
            ("build/gen.py", "def generated():\n    pass\n"),
            (".hidden/mod.py", "def hidden():\n    pass\n"),
            ("src/app.py", "def app():\n    pass\n"),
        ],
    );
    let (_, out) = condor(&dir, &["."]);
    assert_eq!(
        out,
        ".hidden/mod.py:1: unused function 'hidden' (60% confidence)\nsrc/app.py:1: unused function 'app' (60% confidence)\n"
    );
}

#[test]
fn i10_no_report_still_counts_uses() {
    let dir = project(
        "no_report",
        &[("lib.py", "def helper():\n    pass\n"), ("scripts/run.py", "from lib import helper\nhelper()\ndef local():\n    pass\n")],
    );
    let (code, out) = condor(&dir, &["--no-report", "scripts", "."]);
    assert_eq!((code, out.as_str()), (0, ""));
    let (_, out) = condor(&dir, &["--exclude", "scripts", "."]);
    assert_eq!(out, "lib.py:1: unused function 'helper' (60% confidence)\n");
}

#[test]
fn i11_only_kinds() {
    let dir = project("only", &[("m.py", "import os\nx = 1\ndef f():\n    pass\n")]);
    let (_, out) = condor(&dir, &["--only", "function,V104", "m.py"]);
    assert_eq!(out, "m.py:1: unused import 'os' (90% confidence)\nm.py:3: unused function 'f' (60% confidence)\n");
    assert_eq!(condor(&dir, &["--only", "nonsense", "m.py"]).0, 2);
}

#[test]
fn condor_config_section() {
    let dir = project(
        "config_section",
        &[
            ("pyproject.toml", "[tool.vulture]\nignore_names = [\"f\"]\n[tool.condor]\nonly = [\"import\"]\n"),
            ("m.py", "import os\ndef f():\n    pass\ndef g():\n    pass\n"),
        ],
    );
    let (_, out) = condor(&dir, &["m.py"]);
    assert_eq!(out, "m.py:1: unused import 'os' (90% confidence)\n");
}

#[test]
fn project_plugins_in_toml_and_json() {
    let dir = project(
        "plugins",
        &[
            ("pyproject.toml", "[tool.condor]\nplugins = [\"rules\"]\n"),
            (
                "rules/web.toml",
                "[plugin]\nname = \"myweb\"\nwhen_imported = [\"myweb\"]\n\n[[used]]\ndecorated_by = [\"myweb.handler\"]\n",
            ),
            (
                "rules/models.json",
                r#"{"plugin": {"name": "mymodels"}, "fields_used": [{"inherits": ["mymodels.Model"], "fields": "all"}]}"#,
            ),
            (
                "app.py",
                "import myweb\nimport mymodels\n\n@myweb.handler\ndef index():\n    pass\n\nclass User(mymodels.Model):\n    email = 1\n\ndef dead():\n    pass\n\nprint(User)\n",
            ),
        ],
    );
    let (code, out) = condor(&dir, &["app.py"]);
    assert_eq!((code, out.as_str()), (3, "app.py:11: unused function 'dead' (60% confidence)\n"));
}

#[test]
fn invalid_plugin_is_an_error() {
    let dir = project(
        "bad_plugin",
        &[("pyproject.toml", "[tool.condor]\nplugins = [\"bad.toml\"]\n"), ("bad.toml", "[plugin]\nname = \"x\"\n[[used]]\n"), ("m.py", "")],
    );
    assert_eq!(condor(&dir, &["m.py"]).0, 2);
}

fn condor_err(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut out = Output::default();
    let code = run(&argv, dir, &mut out);
    (code, String::from_utf8(out.stdout).unwrap(), String::from_utf8(out.stderr).unwrap())
}

// ----- regressions found by the 0.2.0 audit --------------------------------

#[test]
fn machine_output_stays_parseable() {
    let dir = project("machine_output", &[("dir with space/m%#.py", "def f(): pass\n"), ("bad.py", "def (:\n")]);
    let (code, out, err) = condor_err(&dir, &["--format", "json", "-v", "."]);
    assert_eq!(code, 3);
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok(), "verbose lines stay off stdout");
    assert!(err.contains("Scanning:"));
    let (_, out, _) = condor_err(&dir, &["--format", "sarif", "."]);
    let log: serde_json::Value = serde_json::from_str(&out).unwrap();
    let run = &log["runs"][0];
    let location = &run["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"];
    assert_eq!(location["uri"], "dir%20with%20space/m%25%23.py");
    assert_eq!(location["uriBaseId"], "%SRCROOT%");
    assert_eq!(run["invocations"][0]["executionSuccessful"], false);
    assert!(run["invocations"][0]["toolExecutionNotifications"][0]["message"]["text"].as_str().unwrap().contains("bad.py"));
}

#[test]
fn option_errors() {
    let dir = project("option_errors", &[("m.py", "def f(): pass\n")]);
    assert_eq!(condor(&dir, &["--config", "missing.toml", "m.py"]).0, 2);
    assert_eq!(condor(&dir, &["--make-whitelist", "--format", "json", "m.py"]).0, 2);
    assert_eq!(condor(&dir, &["--only", "Function", "m.py"]).0, 3);
    assert_eq!(condor(&dir, &["--only", ",", "m.py"]).0, 2);
    std::fs::write(dir.join("pyproject.toml"), "[tool]\ncondor = \"x\"\n").unwrap();
    assert_eq!(condor(&dir, &["m.py"]).0, 2);
}

#[test]
fn plugin_files() {
    let rule = |name: &str, f: &str| format!("[plugin]\nname = \"{name}\"\n[[used]]\nnames = [\"{f}\"]\n");
    let dir = project(
        "plugin_files",
        &[
            ("proj/pyproject.toml", "[tool.condor]\nplugins = [\"rules\"]\n"),
            ("proj/rules/nested/a.TOML", &rule("a", "f")),
            ("proj/rules/schema.json", "{}"),
            ("proj/src/m.py", "def f(): pass\ndef g(): pass\n"),
        ],
    );
    // Relative to the configuration file, searched recursively.
    let (code, out) = condor(&dir, &["--config", "proj/pyproject.toml", "proj/src"]);
    assert_eq!((code, out.as_str()), (3, "proj/src/m.py:2: unused function 'g' (60% confidence)\n"));
    std::fs::write(dir.join("proj/rules/b.toml"), rule("a", "g")).unwrap();
    assert_eq!(condor(&dir, &["--config", "proj/pyproject.toml", "proj/src"]).0, 2, "two plugins named a");
}

#[test]
fn project_plugin_replaces_bundled_one() {
    let dir = project(
        "plugin_replace",
        &[
            ("pyproject.toml", "[tool.condor]\nplugins = [\"enum.toml\"]\n"),
            ("enum.toml", "[plugin]\nname = \"enum\"\n[[used]]\nnames = [\"nothing\"]\n"),
            ("m.py", "from enum import Enum\nclass E(Enum):\n    A = 1\nprint(E)\n"),
        ],
    );
    assert_eq!(condor(&dir, &["m.py"]).1, "m.py:3: unused variable 'A' (60% confidence)\n");
}

#[test]
fn entry_points_use_their_module() {
    let dir = project(
        "entry_module",
        &[
            ("pyproject.toml", "[project.scripts]\ntool = \"pkg.cli:main\"\n"),
            ("pkg/cli.py", "def main():\n    pass\n"),
            ("pkg/other.py", "def main():\n    pass\n"),
        ],
    );
    assert_eq!(condor(&dir, &["pkg"]).1, "pkg/other.py:1: unused function 'main' (60% confidence)\n");
}

#[test]
fn coding_cookie_in_a_latin1_comment() {
    let dir = project("cookie", &[]);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("e.py"), b"# -*- coding: latin-1 -*- Soci\xe9t\xe9\nx = 1\n").unwrap();
    assert_eq!(condor(&dir, &["e.py"]).0, 3);
}

/// The binary, for what only `main` sets up.
fn binary(dir: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_condor")).args(args).current_dir(dir).output().unwrap()
}

#[test]
fn deeply_nested_code_is_analysed() {
    let deep = format!("x = {}1{}\n", "(".repeat(5000), ")".repeat(5000));
    let dir = project("deep", &[("deep.py", &deep), ("other.py", "y = 1\n")]);
    assert_eq!(binary(&dir, &["."]).status.code(), Some(3));
}

#[test]
fn subcommand_options_are_checked() {
    let dir = project("subcommands", &[]);
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(binary(&dir, &["server", "--bogus"]).status.code(), Some(2));
    assert_eq!(binary(&dir, &["query", "missing.py"]).status.code(), Some(1));
}

#[cfg(unix)]
#[test]
fn missing_working_directory_is_an_error() {
    let dir = project("gone", &[]);
    std::fs::create_dir_all(&dir).unwrap();
    // Enter a directory, remove it, then start condor there.
    let script = format!("mkdir d && cd d && rmdir ../d && exec '{}' .", env!("CARGO_BIN_EXE_condor"));
    let out = std::process::Command::new("sh").arg("-c").arg(script).current_dir(&dir).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("current directory"));
}
