//! Path helpers with Python `pathlib` semantics.

use std::path::{Component, Path, PathBuf};

/// `utils.format_path`: the path relative to `cwd` if it is lexically below it
/// (`PurePath.relative_to`), otherwise the path unchanged.
pub fn format_path<'a>(path: &'a Path, cwd: &Path) -> &'a Path {
    if path.is_absolute() {
        if let Ok(rel) = path.strip_prefix(cwd) {
            return if rel.as_os_str().is_empty() { Path::new(".") } else { rel };
        }
    }
    path
}

/// `Path.resolve()` (non-strict `os.path.realpath`): make absolute, resolve
/// symlinks of the components that exist, and normalise `.` and `..`.
pub fn resolve(path: &Path, cwd: &Path) -> PathBuf {
    let mut resolved = PathBuf::new();
    let mut budget = 64u32;
    let start = if path.is_absolute() { path.to_path_buf() } else { cwd.join(path) };
    join_realpath(&mut resolved, &start, &mut budget);
    if resolved.as_os_str().is_empty() {
        resolved.push("/");
    }
    resolved
}

fn join_realpath(resolved: &mut PathBuf, rest: &Path, budget: &mut u32) {
    for comp in rest.components() {
        match comp {
            Component::Prefix(p) => *resolved = PathBuf::from(p.as_os_str()),
            Component::RootDir => {
                resolved.clear();
                resolved.push(Component::RootDir.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(name) => {
                let candidate = resolved.join(name);
                match std::fs::read_link(&candidate) {
                    Ok(target) if *budget > 0 => {
                        *budget -= 1;
                        if target.is_absolute() {
                            resolved.clear();
                        }
                        join_realpath(resolved, &target, budget);
                    }
                    _ => *resolved = candidate,
                }
            }
        }
    }
}

/// `os.path.normcase`: lowercase on Windows, unchanged elsewhere.
pub fn normcase(s: &str) -> std::borrow::Cow<'_, str> {
    if cfg!(windows) {
        std::borrow::Cow::Owned(s.replace('/', "\\").to_lowercase())
    } else {
        std::borrow::Cow::Borrowed(s)
    }
}
