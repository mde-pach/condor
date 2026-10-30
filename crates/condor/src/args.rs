//! Command-line parsing that accepts what vulture's `argparse` parser accepts:
//! the same options, unambiguous prefixes of long options, `--opt=value`,
//! `--`, and positionals that look like negative numbers. Invalid command
//! lines exit with status 2.

use crate::PROG;

/// The values given on the command line. `None` means "not given", so the
/// config file or the default applies.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CliArgs {
    pub paths: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub ignore_decorators: Option<Vec<String>>,
    pub ignore_names: Option<Vec<String>>,
    pub make_whitelist: Option<bool>,
    pub min_confidence: Option<i64>,
    pub sort_by_size: Option<bool>,
    pub config: String,
    /// `--config` was given (a missing file is then an error).
    pub config_given: bool,
    pub verbose: Option<bool>,
    pub format: Option<String>,
    pub no_report: Option<Vec<String>>,
    pub only: Option<Vec<String>>,
    /// Paths used when neither the command line nor the configuration gives
    /// any (`.` for `condor query` and `condor server`).
    pub default_paths: Vec<String>,
}

/// Parsing stopped: print these and exit with `code`.
#[derive(Debug, PartialEq)]
pub struct Exit {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Help,
    Exclude,
    IgnoreDecorators,
    IgnoreNames,
    MakeWhitelist,
    MinConfidence,
    SortBySize,
    Config,
    Verbose,
    Version,
    Format,
    NoReport,
    Only,
}

impl Action {
    fn takes_value(self) -> bool {
        matches!(
            self,
            Action::Exclude
                | Action::IgnoreDecorators
                | Action::IgnoreNames
                | Action::MinConfidence
                | Action::Config
                | Action::Format
                | Action::NoReport
                | Action::Only
        )
    }

    /// `argument <names>` in error messages.
    fn names(self) -> &'static str {
        match self {
            Action::Help => "-h/--help",
            Action::Exclude => "--exclude",
            Action::IgnoreDecorators => "--ignore-decorators",
            Action::IgnoreNames => "--ignore-names",
            Action::MakeWhitelist => "--make-whitelist",
            Action::MinConfidence => "--min-confidence",
            Action::SortBySize => "--sort-by-size",
            Action::Config => "--config",
            Action::Verbose => "-v/--verbose",
            Action::Version => "--version",
            Action::Format => "--format",
            Action::NoReport => "--no-report",
            Action::Only => "--only",
        }
    }
}

/// Option strings in the order the parser registers them.
const OPTION_STRINGS: [(&str, Action); 15] = [
    ("-h", Action::Help),
    ("--help", Action::Help),
    ("--exclude", Action::Exclude),
    ("--ignore-decorators", Action::IgnoreDecorators),
    ("--ignore-names", Action::IgnoreNames),
    ("--make-whitelist", Action::MakeWhitelist),
    ("--min-confidence", Action::MinConfidence),
    ("--sort-by-size", Action::SortBySize),
    ("--config", Action::Config),
    ("-v", Action::Verbose),
    ("--verbose", Action::Verbose),
    ("--version", Action::Version),
    ("--format", Action::Format),
    ("--no-report", Action::NoReport),
    ("--only", Action::Only),
];

fn lookup(s: &str) -> Option<Action> {
    OPTION_STRINGS.iter().find(|(o, _)| *o == s).map(|(_, a)| *a)
}

/// One way to read an option-like argument: `(action, option string, sep,
/// explicit argument)`. `action` is `None` for an unknown option.
#[derive(Clone, Debug)]
struct OptionTuple {
    action: Option<Action>,
    option_string: String,
    sep: Option<String>,
    explicit_arg: Option<String>,
}

#[derive(Clone, Debug)]
enum Classified {
    Positional,
    Dashes,
    Optional(Vec<OptionTuple>),
}

fn tuple(action: Option<Action>, option_string: &str, sep: Option<&str>, explicit: Option<&str>) -> OptionTuple {
    OptionTuple {
        action,
        option_string: option_string.to_string(),
        sep: sep.map(str::to_string),
        explicit_arg: explicit.map(str::to_string),
    }
}

/// `ArgumentParser._parse_optional`.
fn parse_optional(arg: &str) -> Option<Vec<OptionTuple>> {
    if !arg.starts_with('-') {
        return None;
    }
    if let Some(a) = lookup(arg) {
        return Some(vec![tuple(Some(a), arg, None, None)]);
    }
    if arg.chars().count() == 1 {
        return None;
    }
    if let Some((opt, explicit)) = arg.split_once('=')
        && let Some(a) = lookup(opt) {
            return Some(vec![tuple(Some(a), opt, Some("="), Some(explicit))]);
        }
    let tuples = option_tuples(arg);
    if !tuples.is_empty() {
        return Some(tuples);
    }
    // `-\.?\d`: looks like a negative number, and no option does.
    let rest = &arg[1..];
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    if rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    if arg.contains(' ') {
        return None;
    }
    Some(vec![tuple(None, arg, None, None)])
}

/// `ArgumentParser._get_option_tuples` (abbreviations allowed).
fn option_tuples(arg: &str) -> Vec<OptionTuple> {
    let mut result = Vec::new();
    let second = arg.chars().nth(1);
    if second == Some('-') {
        let (prefix, sep, explicit) = match arg.split_once('=') {
            Some((p, e)) => (p, Some("="), Some(e)),
            None => (arg, None, None),
        };
        for (o, a) in OPTION_STRINGS {
            if o.starts_with(prefix) {
                result.push(tuple(Some(a), o, sep, explicit));
            }
        }
    } else {
        let short: String = arg.chars().take(2).collect();
        let short_explicit: String = arg.chars().skip(2).collect();
        for (o, a) in OPTION_STRINGS {
            if o == short {
                result.push(tuple(Some(a), o, Some(""), Some(&short_explicit)));
            } else if o.starts_with(arg) {
                result.push(tuple(Some(a), o, None, None));
            }
        }
    }
    result
}

fn usage() -> String {
    format!("usage: {PROG} [options] [PATH ...]\n")
}

fn error(msg: &str) -> Exit {
    Exit { code: 2, stdout: String::new(), stderr: format!("{}{PROG}: error: {msg}\n", usage()) }
}

fn action_error(action: Action, msg: &str) -> Exit {
    error(&format!("argument {}: {msg}", action.names()))
}

/// `int(s)` as Python reads a base-10 integer (surrounding whitespace, a
/// sign, `_` between digits), saturated to `i64`.
pub fn py_int(s: &str) -> Option<i64> {
    let s = s.trim();
    let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
    if digits.is_empty() || digits.starts_with('_') || digits.ends_with('_') || digits.contains("__") {
        return None;
    }
    if !digits.chars().all(|c| c.is_ascii_digit() || c == '_') {
        return None;
    }
    let value: i128 = digits.replace('_', "").chars().take(20).collect::<String>().parse().ok()?;
    let value = if s.starts_with('-') { -value } else { value };
    Some(value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64)
}

/// `ArgumentParser.parse_args` for vulture's parser.
pub fn parse(argv: &[String]) -> Result<CliArgs, Exit> {
    // Classify every argument: 'A' positional, 'O' option-like, '-' for `--`.
    let mut kinds: Vec<Classified> = Vec::with_capacity(argv.len());
    let mut after_dashes = false;
    for arg in argv {
        if after_dashes {
            kinds.push(Classified::Positional);
        } else if arg == "--" {
            after_dashes = true;
            kinds.push(Classified::Dashes);
        } else {
            kinds.push(match parse_optional(arg) {
                Some(t) => Classified::Optional(t),
                None => Classified::Positional,
            });
        }
    }
    let pattern: Vec<char> = kinds
        .iter()
        .map(|k| match k {
            Classified::Positional => 'A',
            Classified::Dashes => '-',
            Classified::Optional(_) => 'O',
        })
        .collect();

    let mut out = CliArgs { config: "pyproject.toml".to_string(), ..CliArgs::default() };
    let mut extras: Vec<String> = Vec::new();
    let mut paths_pending = true;
    let option_indices: Vec<usize> = (0..argv.len()).filter(|&i| pattern[i] == 'O').collect();
    let max_option = option_indices.last().copied();

    // The `PATH` positional (`nargs="*"`): consumes every 'A' and '-' from
    // `start`, dropping the first `--`.
    let consume_positionals = |start: usize, out: &mut CliArgs, pending: &mut bool| -> usize {
        if !*pending {
            return start;
        }
        let count = pattern[start..].iter().take_while(|&&c| c == 'A' || c == '-').count();
        let mut args: Vec<String> = argv[start..start + count].to_vec();
        if pattern[start..start + count].contains(&'-')
            && let Some(pos) = args.iter().position(|a| a == "--") {
                args.remove(pos);
            }
        *pending = false;
        // An empty match leaves the default (the config file's paths).
        if !args.is_empty() {
            out.paths = Some(args);
        }
        start + count
    };

    let mut start = 0;
    if let Some(max_option) = max_option {
        while start <= max_option {
            let next_option = (start..=max_option).find(|i| pattern[*i] == 'O').unwrap_or(max_option + 1);
            if start != next_option {
                let end = consume_positionals(start, &mut out, &mut paths_pending);
                if end > start {
                    start = end;
                    continue;
                }
                start = end;
            }
            if pattern[start] != 'O' {
                extras.extend(argv[start..next_option].iter().cloned());
                start = next_option;
            }
            start = consume_optional(argv, &pattern, &kinds, start, &mut out, &mut extras)?;
        }
    }
    let stop = consume_positionals(start, &mut out, &mut paths_pending);
    extras.extend(argv[stop..].iter().cloned());
    if !extras.is_empty() {
        return Err(error(&format!("unrecognized arguments: {}", extras.join(" "))));
    }
    Ok(out)
}

fn consume_optional(
    argv: &[String],
    pattern: &[char],
    kinds: &[Classified],
    start: usize,
    out: &mut CliArgs,
    extras: &mut Vec<String>,
) -> Result<usize, Exit> {
    let Classified::Optional(tuples) = &kinds[start] else { unreachable!() };
    if tuples.len() > 1 {
        let matches: Vec<&str> = tuples.iter().map(|t| t.option_string.as_str()).collect();
        return Err(error(&format!("ambiguous option: {} could match {}", argv[start], matches.join(", "))));
    }
    let OptionTuple { mut action, mut option_string, mut sep, mut explicit_arg } = tuples[0].clone();
    let mut taken: Vec<(Action, Vec<String>)> = Vec::new();
    let stop;
    loop {
        let Some(act) = action else {
            extras.push(argv[start].clone());
            return Ok(start + 1);
        };
        if let Some(explicit) = explicit_arg.clone() {
            let single_dash = option_string.chars().nth(1) != Some('-');
            if !act.takes_value() && single_dash && !explicit.is_empty() {
                if sep.as_deref().is_some_and(|s| !s.is_empty()) || explicit.starts_with('-') {
                    return Err(action_error(act, &format!("ignored explicit argument '{explicit}'")));
                }
                taken.push((act, Vec::new()));
                let first: String = explicit.chars().take(1).collect();
                let candidate = format!("-{first}");
                if let Some(next) = lookup(&candidate) {
                    action = Some(next);
                    option_string = candidate;
                    let rest: String = explicit.chars().skip(1).collect();
                    if rest.is_empty() {
                        sep = None;
                        explicit_arg = None;
                    } else if let Some(r) = rest.strip_prefix('=') {
                        sep = Some("=".into());
                        explicit_arg = Some(r.to_string());
                    } else {
                        sep = Some(String::new());
                        explicit_arg = Some(rest);
                    }
                    continue;
                }
                extras.push(format!("-{explicit}"));
                stop = start + 1;
                break;
            } else if act.takes_value() {
                taken.push((act, vec![explicit]));
                stop = start + 1;
                break;
            } else {
                return Err(action_error(act, &format!("ignored explicit argument '{explicit}'")));
            }
        }
        let next = start + 1;
        if act.takes_value() {
            if pattern.get(next) != Some(&'A') {
                return Err(action_error(act, "expected one argument"));
            }
            taken.push((act, vec![argv[next].clone()]));
            stop = next + 1;
        } else {
            taken.push((act, Vec::new()));
            stop = next;
        }
        break;
    }
    for (act, values) in taken {
        apply(act, values, out)?;
    }
    Ok(stop)
}

fn apply(act: Action, values: Vec<String>, out: &mut CliArgs) -> Result<(), Exit> {
    let csv = |v: &str| v.split(',').map(str::to_string).collect::<Vec<_>>();
    match act {
        Action::Help => return Err(Exit { code: 0, stdout: help(), stderr: String::new() }),
        Action::Version => {
            return Err(Exit { code: 0, stdout: format!("{}\n", crate::version_string()), stderr: String::new() });
        }
        Action::Exclude => out.exclude = Some(csv(&values[0])),
        Action::IgnoreDecorators => out.ignore_decorators = Some(csv(&values[0])),
        Action::IgnoreNames => out.ignore_names = Some(csv(&values[0])),
        Action::MakeWhitelist => out.make_whitelist = Some(true),
        Action::SortBySize => out.sort_by_size = Some(true),
        Action::Verbose => out.verbose = Some(true),
        Action::Config => {
            out.config = values[0].clone();
            out.config_given = true;
        }
        Action::Format => out.format = Some(values[0].clone()),
        Action::NoReport => out.no_report = Some(csv(&values[0])),
        Action::Only => out.only = Some(csv(&values[0])),
        Action::MinConfidence => match py_int(&values[0]) {
            Some(n) => out.min_confidence = Some(n),
            None => {
                return Err(action_error(act, &format!("invalid int value: '{}'", values[0])));
            }
        },
    }
    Ok(())
}

pub fn help() -> String {
    format!(
        "{}
condor finds unused code in Python programs. It accepts vulture's options and
reads [tool.vulture] and [tool.condor] in pyproject.toml. Directories are
scanned without the files that .gitignore ignores.

positional arguments:
  PATH                  Python files or directories. For each directory, all
                        contained *.py files are analyzed.

commands:
  condor query [OPTIONS] [PATH ...]
                        Report the whole project (the configured paths, or .),
                        only for PATH if given. A background daemon keeps the
                        analysis, so later queries rescan only changed files.
  condor server [OPTIONS]
                        Language server on stdin and stdout, for editors.

options:
  -h, --help            show this help message and exit
  --exclude PATTERNS    Comma-separated list of path patterns to ignore. A
                        pattern without glob wildcards is treated as
                        *PATTERN*. Patterns are matched against absolute paths.
  --ignore-decorators PATTERNS
                        Comma-separated list of decorators; functions and
                        classes using them are ignored (e.g. \"@app.route\").
  --ignore-names PATTERNS
                        Comma-separated list of names to ignore.
  --make-whitelist      Report unused code as a whitelist module.
  --min-confidence MIN_CONFIDENCE
                        Minimum confidence (between 0 and 100) for code to be
                        reported as unused.
  --sort-by-size        Sort unused functions and classes by their size.
  --config CONFIG       Path to the pyproject.toml config file.
  --no-report PATTERNS  Comma-separated list of path patterns whose files are
                        scanned for uses but whose own findings are not
                        reported (same pattern rules as --exclude).
  --only KINDS          Comma-separated list of kinds to report: attribute,
                        class, function, import, method, property, variable,
                        unreachable_code, whitelist_entry (or their V codes).
  --format FORMAT       Output format: text (default), json or sarif.
  -v, --verbose         Log definitions and uses while scanning.
  --version             show program's version number and exit
",
        usage().trim_end()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<CliArgs, Exit> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    fn rejected(args: &[&str]) -> bool {
        p(args).is_err_and(|e| e.code == 2)
    }

    #[test]
    fn values() {
        assert_eq!(p(&["--exclude", "x,y", "a"]).unwrap().exclude, Some(vec!["x".into(), "y".into()]));
        assert_eq!(p(&["--exclude", "x", "a"]).unwrap().paths, Some(vec!["a".into()]));
        assert_eq!(p(&["a", "b", "--sort-by-size"]).unwrap().paths, Some(vec!["a".into(), "b".into()]));
        assert_eq!(p(&["--min", "5", "a"]).unwrap().min_confidence, Some(5));
        assert_eq!(p(&["--min-confidence=60", "a"]).unwrap().min_confidence, Some(60));
        assert_eq!(p(&["--", "-a", "b"]).unwrap().paths, Some(vec!["-a".into(), "b".into()]));
        assert_eq!(p(&["--exclude=a", "--exclude=b", "p"]).unwrap().exclude, Some(vec!["b".into()]));
        assert_eq!(p(&["--conf", "x", "p"]).unwrap().config, "x");
        assert_eq!(p(&["-v", "--make-whitelist", "p"]).unwrap().verbose, Some(true));
        assert_eq!(p(&[]).unwrap().paths, None);
        assert_eq!(p(&["--help"]).unwrap_err().code, 0);
        assert_eq!(p(&["--version"]).unwrap_err().code, 0);
    }

    #[test]
    fn invalid_command_lines_exit_2() {
        assert!(rejected(&["a", "--exclude", "x", "b"]));
        assert!(rejected(&["--i", "x", "a"]));
        assert!(rejected(&["--exclude"]));
        assert!(rejected(&["--min-confidence", "1.5"]));
        assert!(rejected(&["--invalid-argument", "a"]));
        assert!(rejected(&["--verbose=1", "a"]));
    }
}
