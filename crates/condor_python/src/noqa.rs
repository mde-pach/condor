//! `# noqa` handling (vulture `noqa.py`).

use std::sync::LazyLock;

use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};

// The flake8 regex, as used by vulture.
static NOQA_REGEXP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)# noqa(?::\s?(?P<codes>([A-Z]+[0-9]+(?:[,\s]+)?)+))?").unwrap()
});

/// flake8 codes vulture understands.
pub const NOQA_CODE_MAP: [(&str, &str); 2] = [("F401", "V104"), ("F841", "V107")];

/// `NOQA_REGEXP.search(line)`: `None` if there is no match, otherwise the
/// `codes` group.
pub fn search(line: &str) -> Option<Option<&str>> {
    NOQA_REGEXP.captures(line).map(|caps| caps.name("codes").map(|m| m.as_str()))
}

/// `_parse_error_codes`: no codes means `all`.
pub fn parse_error_codes(codes: Option<&str>) -> Vec<&str> {
    codes.unwrap_or("all").split(',').map(py_strip).collect()
}

/// Line numbers suppressed per code; `"all"` suppresses every code.
#[derive(Default, Debug)]
pub struct NoqaLines {
    lines: FxHashMap<String, FxHashSet<u32>>,
}

impl NoqaLines {
    /// `parse_noqa(code.splitlines())`. Line numbers follow Python's
    /// `str.splitlines`, which breaks on more characters than the parser does.
    pub fn parse(code: &str) -> NoqaLines {
        let mut noqa = NoqaLines::default();
        if !code.contains('#') {
            return noqa;
        }
        for (idx, line) in py_splitlines(code).into_iter().enumerate() {
            if !line.contains('#') {
                continue;
            }
            let Some(codes) = search(line) else { continue };
            for code in parse_error_codes(codes) {
                let code = NOQA_CODE_MAP.iter().find(|(f, _)| *f == code).map_or(code, |(_, v)| v);
                noqa.lines.entry(code.to_string()).or_default().insert(idx as u32 + 1);
            }
        }
        noqa
    }

    pub fn ignores(&self, line: u32, code: &str) -> bool {
        let has = |c: &str| self.lines.get(c).is_some_and(|s| s.contains(&line));
        has(code) || has("all")
    }
}

/// Python's `str.splitlines()` line boundaries.
pub fn py_splitlines(s: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let is_break = matches!(
            c,
            '\n' | '\r' | '\x0b' | '\x0c' | '\x1c' | '\x1d' | '\x1e' | '\u{85}' | '\u{2028}' | '\u{2029}'
        );
        if is_break {
            lines.push(&s[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r' {
                if let Some(&(j, '\n')) = chars.peek() {
                    chars.next();
                    end = j + 1;
                }
            }
            start = end;
        }
    }
    if start < s.len() {
        lines.push(&s[start..]);
    }
    lines
}

/// Python's `str.strip()` with no arguments (Unicode whitespace).
pub fn py_strip(s: &str) -> &str {
    s.trim_matches(is_py_whitespace)
}

/// `str.isspace()` for a single character.
pub fn is_py_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0b' | '\x0c' | '\r' | '\x1c' | '\x1d' | '\x1e' | '\x1f' | ' ' | '\u{85}')
        || (c as u32 > 0x7f && c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        let n = NoqaLines::parse("x = 1  # noqa: F841, V101\nimport os  # NOQA\ny  # noqa:V102 V103\n");
        assert!(n.ignores(1, "V107"));
        assert!(n.ignores(1, "V101"));
        assert!(!n.ignores(1, "V103"));
        assert!(n.ignores(2, "V104"));
        assert!(!n.ignores(3, "V102"));
    }
}
