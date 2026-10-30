//! Path helpers with Python `pathlib` semantics.

use std::path::{Component, Path, PathBuf};

/// `utils.format_path`: the path relative to `cwd` if it is lexically below it
/// (`PurePath.relative_to`), otherwise the path unchanged.
pub fn format_path<'a>(path: &'a Path, cwd: &Path) -> &'a Path {
    if path.is_absolute()
        && let Ok(rel) = path.strip_prefix(cwd) {
            return if rel.as_os_str().is_empty() { Path::new(".") } else { rel };
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
                // Keep a Windows drive: `C:\` is the prefix `C:`, then the root.
                let prefix = match resolved.components().next() {
                    Some(Component::Prefix(p)) => Some(p.as_os_str().to_owned()),
                    _ => None,
                };
                resolved.clear();
                if let Some(prefix) = prefix {
                    resolved.push(prefix);
                }
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

/// The dotted module names a file can be imported as (`a/b/c.py`: `c`,
/// `b.c`, `a.b.c`; `a/b/__init__.py`: `b`, `a.b`).
pub fn module_names(file: &Path) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if stem != "__init__" {
        parts.push(stem);
    }
    for dir in file.parent().into_iter().flat_map(Path::ancestors).take(8) {
        let Some(name) = dir.file_name() else { break };
        parts.push(name.to_string_lossy().into_owned());
    }
    (1..=parts.len()).map(|n| parts[..n].iter().rev().cloned().collect::<Vec<_>>().join(".")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn resolve_keeps_the_drive() {
        let cwd = Path::new(r"D:\work\project");
        assert_eq!(resolve(Path::new("pkg\\m.py"), cwd), Path::new(r"D:\work\project\pkg\m.py"));
        assert_eq!(resolve(Path::new(r"C:\other\..\x.py"), cwd), Path::new(r"C:\x.py"));
        assert_eq!(format_path(&resolve(Path::new("m.py"), cwd), cwd), Path::new("m.py"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_normalises() {
        let cwd = Path::new("/nonexistent/project");
        assert_eq!(resolve(Path::new("a/../b/./m.py"), cwd), Path::new("/nonexistent/project/b/m.py"));
        assert_eq!(format_path(&resolve(Path::new("m.py"), cwd), cwd), Path::new("m.py"));
    }
}
