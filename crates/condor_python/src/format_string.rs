//! Port of CPython's `MarkupIterator` (`Objects/stringlib/unicode_format.h`),
//! the parser behind `string.Formatter().parse`.

/// The field names of a `str.format` template, or `None` if the template is
/// invalid (Python raises `ValueError`). Fields without a name, such as `{}`,
/// are skipped, as vulture skips falsy names.
pub fn field_names(s: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = s.chars().collect();
    let mut names = Vec::new();
    let mut pos = 0;
    let end = chars.len();
    while pos < end {
        // Literal text up to the first '{' or '}'.
        let mut c = '\0';
        let mut markup_follows = false;
        while pos < end {
            c = chars[pos];
            pos += 1;
            if c == '{' || c == '}' {
                markup_follows = true;
                break;
            }
        }
        let at_end = pos >= end;
        if c == '}' && markup_follows && (at_end || chars[pos] != c) {
            return None; // Single '}' encountered
        }
        if at_end && c == '{' && markup_follows {
            return None; // Single '{' encountered
        }
        if !at_end && markup_follows && chars[pos] == c {
            pos += 1; // escaped "{{" or "}}"
            continue;
        }
        if !markup_follows {
            continue;
        }
        // Field name, terminated by '}', ':' or '!'.
        let name_start = pos;
        let mut c = '\0';
        while pos < end {
            c = chars[pos];
            pos += 1;
            match c {
                '{' => return None, // unexpected '{' in field name
                '[' => {
                    while pos < end && chars[pos] != ']' {
                        pos += 1;
                    }
                    continue;
                }
                '}' | ':' | '!' => break,
                _ => continue,
            }
        }
        let name: String = chars[name_start..pos.saturating_sub(1).max(name_start)].iter().collect();
        if c == '!' || c == ':' {
            if c == '!' {
                if pos >= end {
                    return None; // end of string while looking for conversion specifier
                }
                pos += 1; // the conversion character
                if pos < end {
                    let next = chars[pos];
                    pos += 1;
                    if next == '}' {
                        names.push(name);
                        continue;
                    }
                    if next != ':' {
                        return None; // expected ':' after conversion specifier
                    }
                }
            }
            let mut count = 1;
            let mut closed = false;
            while pos < end {
                let ch = chars[pos];
                pos += 1;
                if ch == '{' {
                    count += 1;
                } else if ch == '}' {
                    count -= 1;
                    if count == 0 {
                        closed = true;
                        break;
                    }
                }
            }
            if !closed {
                return None; // unmatched '{' in format spec
            }
        } else if c != '}' {
            return None; // expected '}' before end of string
        }
        names.push(name);
    }
    Some(names.into_iter().filter(|n| !n.is_empty()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let f = |s: &str| field_names(s);
        assert_eq!(f("{a} {b.c[0]} {}"), Some(vec!["a".into(), "b.c[0]".into()]));
        assert_eq!(f("{{a}} {b!r:>{w}}"), Some(vec!["b".into()]));
        assert_eq!(f("{a"), None);
        assert_eq!(f("a}"), None);
        assert_eq!(f("{a!}"), None);
        assert_eq!(f("{a[}"), None);
        assert_eq!(f("{a:{b}"), None);
        assert_eq!(f("{a!r"), None);
        assert_eq!(f("{a!r}{b!s:x}"), Some(vec!["a".into(), "b".into()]));
    }
}
