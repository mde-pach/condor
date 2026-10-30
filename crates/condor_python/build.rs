//! Embed the bundled plugin files (`plugins/`, a snapshot of condor-plugins).

use std::path::{Path, PathBuf};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "toml" || e == "json") && path.file_name() != Some("schema.json".as_ref()) {
            out.push(path);
        }
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins");
    // Only the rule files; `tests/` holds the cases for `plugin_cases.rs`.
    let rules = root.join("python");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&rules, &mut files);
    let mut code = String::from("pub static PLUGINS: &[(&str, &str)] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        code.push_str(&format!("    ({rel:?}, include_str!({:?})),\n", f.display().to_string()));
    }
    code.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bundled_plugins.rs");
    std::fs::write(out, code).unwrap();
}
