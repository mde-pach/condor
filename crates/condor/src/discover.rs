//! File collection (vulture `utils.get_modules`).

use std::path::{Path, PathBuf};

use condor_core::paths::resolve;

/// The modules to scan, in vulture's order, or the error vulture exits with
/// (status 1) before scanning anything.
pub fn get_modules(paths: &[String], cwd: &Path) -> Result<Vec<PathBuf>, String> {
    let mut modules = Vec::new();
    for p in paths {
        let path = resolve(Path::new(p), cwd);
        if path.is_file() {
            if path.extension().is_some_and(|e| e == "pyc") {
                return Err(format!("Error: *.pyc files are not supported: {}", path.display()));
            }
            modules.push(path);
        } else if path.is_dir() {
            rglob_py(&path, &mut modules);
        } else {
            return Err(format!("Error: {} could not be found.", path.display()));
        }
    }
    Ok(modules)
}

/// The `*.py` files under `dir`, in sorted order. Hidden directories are
/// included and symlinked directories are not followed, as in vulture; paths
/// ignored by the git repository's `.gitignore` files and
/// `.git/info/exclude`, and by `.ignore` files, are skipped. As in ripgrep,
/// `.gitignore` files apply only inside a git repository, so ignore files in
/// directories above the repository have no effect.
fn rglob_py(dir: &Path, out: &mut Vec<PathBuf>) {
    let walker = ignore::WalkBuilder::new(dir)
        .hidden(false)
        .parents(true)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(false)
        .ignore(true)
        .require_git(true)
        .follow_links(false)
        .build_parallel();
    let found = std::sync::Mutex::new(Vec::new());
    walker.run(|| {
        let found = &found;
        let mut local = Vec::new();
        Box::new(move |entry| {
            if let Ok(entry) = entry {
                let path = entry.path();
                if entry.depth() > 0
                    && entry.file_name().as_encoded_bytes().ends_with(b".py")
                    && !entry.file_type().is_some_and(|t| t.is_dir())
                    && path.is_file()
                {
                    local.push(path.to_path_buf());
                }
            }
            if !local.is_empty() {
                found.lock().unwrap().append(&mut local);
            }
            ignore::WalkState::Continue
        })
    });
    // Component-wise order is the order of a walk visiting each directory's
    // entries sorted by name.
    let mut found = found.into_inner().unwrap();
    found.sort();
    out.extend(found);
}
