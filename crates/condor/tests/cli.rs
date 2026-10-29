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
