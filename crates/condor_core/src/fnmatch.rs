//! Port of Python 3.14 `fnmatch.translate`, compiled to a Rust regex.
//!
//! Python joins interior `*fixed` pairs with atomic groups `(?>.*?fixed)`.
//! Every such group is followed by another star, so committing to the
//! earliest match never changes whether the whole pattern matches, and a
//! plain lazy `.*?` is equivalent.

use regex::Regex;

use crate::paths::normcase;

/// A compiled glob pattern with `fnmatch` semantics.
#[derive(Clone, Debug)]
pub struct Pattern {
    regex: Regex,
}

impl Pattern {
    pub fn new(pat: &str) -> Pattern {
        let regex = Regex::new(&translate(pat)).expect("fnmatch translation is always valid");
        Pattern { regex }
    }

    /// `fnmatch.fnmatchcase`.
    pub fn matches_case(&self, name: &str) -> bool {
        self.regex.is_match(name)
    }
}

/// A list of patterns, as used by `--exclude`, `--ignore-names` and
/// `--ignore-decorators`.
#[derive(Clone, Debug, Default)]
pub struct PatternSet {
    patterns: Vec<Pattern>,
    raw: Vec<String>,
}

impl PatternSet {
    pub fn new<S: AsRef<str>>(patterns: &[S]) -> PatternSet {
        PatternSet {
            patterns: patterns.iter().map(|p| Pattern::new(p.as_ref())).collect(),
            raw: patterns.iter().map(|p| p.as_ref().to_string()).collect(),
        }
    }

    /// vulture's `_match(name, patterns, case=True)`: `fnmatchcase`.
    pub fn matches_case(&self, name: &str) -> bool {
        self.patterns.iter().any(|p| p.matches_case(name))
    }

    /// vulture's `_match(name, patterns, case=False)`: `fnmatch`, which applies
    /// `os.path.normcase` to both sides (a no-op on POSIX).
    pub fn matches_normcase(&self, name: &str) -> bool {
        if cfg!(windows) {
            let name = normcase(name);
            self.raw.iter().any(|p| Pattern::new(&normcase(p)).matches_case(&name))
        } else {
            self.matches_case(name)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

pub fn translate(pat: &str) -> String {
    let pat: Vec<char> = pat.chars().collect();
    let n = pat.len();
    let mut res = String::from("^(?s:");
    let mut i = 0;
    while i < n {
        let c = pat[i];
        i += 1;
        match c {
            '*' => {
                res.push_str(".*?");
                while i < n && pat[i] == '*' {
                    i += 1;
                }
            }
            '?' => res.push('.'),
            '[' => {
                let mut j = i;
                if j < n && pat[j] == '!' {
                    j += 1;
                }
                if j < n && pat[j] == ']' {
                    j += 1;
                }
                while j < n && pat[j] != ']' {
                    j += 1;
                }
                if j >= n {
                    res.push_str("\\[");
                    continue;
                }
                let stuff = python_set_body(&pat, i, j);
                i = j + 1;
                if stuff.is_empty() {
                    res.push_str("[a&&b]");
                } else if stuff == "!" {
                    res.push('.');
                } else {
                    res.push_str(&rust_class(&stuff));
                }
            }
            _ => res.push_str(&regex::escape(&c.to_string())),
        }
    }
    // A trailing lazy star must still reach the end.
    res.push_str(")\\z");
    res
}

/// The set body exactly as Python builds it (before the set-operation and
/// leading-character escapes), using Python's escaping conventions.
fn python_set_body(pat: &[char], start: usize, j: usize) -> String {
    let slice = |a: usize, b: usize| -> String { pat[a..b].iter().collect() };
    let stuff = slice(start, j);
    if !stuff.contains('-') {
        return stuff.replace('\\', "\\\\");
    }
    let mut i = start;
    let mut chunks: Vec<String> = Vec::new();
    let mut k = if pat[i] == '!' { i + 2 } else { i + 1 };
    loop {
        let found = (k..j).find(|&x| pat[x] == '-');
        let Some(pos) = found else { break };
        chunks.push(slice(i, pos));
        i = pos + 1;
        k = pos + 3;
    }
    let chunk = slice(i, j);
    if !chunk.is_empty() {
        chunks.push(chunk);
    } else if let Some(last) = chunks.last_mut() {
        last.push('-');
    }
    let mut k = chunks.len().saturating_sub(1);
    while k > 0 {
        let prev_last = chunks[k - 1].chars().last();
        let cur_first = chunks[k].chars().next();
        if let (Some(a), Some(b)) = (prev_last, cur_first)
            && a > b {
                let mut merged: String = {
                    let mut p = chunks[k - 1].clone();
                    p.pop();
                    p
                };
                merged.extend(chunks[k].chars().skip(1));
                chunks[k - 1] = merged;
                chunks.remove(k);
            }
        k -= 1;
    }
    chunks
        .iter()
        .map(|s| s.replace('\\', "\\\\").replace('-', "\\-"))
        .collect::<Vec<_>>()
        .join("-")
}

/// Convert a Python set body into an equivalent Rust regex class.
fn rust_class(stuff: &str) -> String {
    let chars: Vec<char> = stuff.chars().collect();
    let mut out = String::from("[");
    let mut idx = 0;
    if chars[0] == '!' {
        out.push('^');
        idx = 1;
    }
    let lit = |c: char, out: &mut String| {
        if c.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(c);
    };
    while idx < chars.len() {
        let c = chars[idx];
        if c == '\\' && idx + 1 < chars.len() {
            lit(chars[idx + 1], &mut out);
            idx += 2;
        } else if c == '-' && idx > 0 && idx + 1 < chars.len() {
            out.push('-');
            idx += 1;
        } else {
            lit(c, &mut out);
            idx += 1;
        }
    }
    out.push(']');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(pat: &str, name: &str) -> bool {
        Pattern::new(pat).matches_case(name)
    }

    #[test]
    fn patterns_used_by_options() {
        assert!(m("*.py", "/src/a/b.py"));
        assert!(m("visit_*", "visit_Name"));
        assert!(!m("visit_*", "xvisit_"));
        assert!(m("@app.route", "@app.route"));
        assert!(!m("@app.route", "@appxroute"));
        assert!(m("@require_*", "@require_login"));
        assert!(m("*/tests/*", "/repo/tests/test_a.py"));
        assert!(m("[ab]_test.py", "a_test.py"));
        assert!(!m("[!ab]_test.py", "a_test.py"));
    }
}
