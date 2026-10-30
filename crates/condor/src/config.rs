//! `[tool.vulture]` configuration and its merge with the command line
//! (vulture `config.py`).

use std::path::Path;

use crate::args::{CliArgs, Exit};

/// The merged configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub paths: Vec<String>,
    pub exclude: Vec<String>,
    pub ignore_decorators: Vec<String>,
    pub ignore_names: Vec<String>,
    pub make_whitelist: bool,
    pub min_confidence: i64,
    pub sort_by_size: bool,
    pub verbose: bool,
    pub config: String,
    pub format: String,
    pub no_report: Vec<String>,
    pub only: Vec<String>,
    /// Plugin files or directories added to the bundled plugins.
    pub plugins: Vec<String>,
    /// Names used by the project's entry points (`[project.scripts]`, ...).
    pub entry_point_names: Vec<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            paths: Vec::new(),
            exclude: Vec::new(),
            ignore_decorators: Vec::new(),
            ignore_names: Vec::new(),
            make_whitelist: false,
            min_confidence: 0,
            sort_by_size: false,
            verbose: false,
            config: "pyproject.toml".into(),
            format: "text".into(),
            no_report: Vec::new(),
            only: Vec::new(),
            plugins: Vec::new(),
            entry_point_names: Vec::new(),
        }
    }
}

/// A value from the config file, typed like vulture's defaults.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
    List(Vec<toml::Value>),
    Other(&'static str),
}

/// vulture's `InputError`: printed to stderr, exit code 2.
#[derive(Debug, PartialEq)]
pub struct InputError(pub String);

/// The type name vulture expects for each key (`type(DEFAULTS[key]).__name__`).
pub fn expected_type(key: &str) -> Option<&'static str> {
    Some(match key {
        "config" | "format" => "str",
        "no_report" | "only" | "plugins" => "list",
        "min_confidence" => "int",
        "paths" | "exclude" | "ignore_decorators" | "ignore_names" => "list",
        "make_whitelist" | "sort_by_size" | "verbose" => "bool",
        _ => return None,
    })
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Str(_) => "str",
        Value::Int(_) => "int",
        Value::Bool(_) => "bool",
        Value::List(_) => "list",
        Value::Other(t) => t,
    }
}

/// `_check_input_config`: unknown keys and exact type checks, in order.
pub fn check_input_config(data: &[(String, Value)]) -> Result<(), InputError> {
    for (key, value) in data {
        let Some(expected) = expected_type(key) else {
            return Err(InputError(format!("Unknown configuration key: {key}")));
        };
        if type_name(value) != expected {
            return Err(InputError(format!("Data type for {key} must be '{expected}'")));
        }
    }
    Ok(())
}

fn convert(v: &toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::Str(s.clone()),
        toml::Value::Integer(i) => Value::Int(*i),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Array(a) => Value::List(a.clone()),
        toml::Value::Float(_) => Value::Other("float"),
        toml::Value::Datetime(_) => Value::Other("datetime"),
        toml::Value::Table(_) => Value::Other("dict"),
    }
}

/// The `[tool.vulture]` table, then `[tool.condor]` (whose keys win),
/// type-checked. A TOML syntax error is returned as `Err(Err(message))`.
pub fn parse_toml(content: &str) -> Result<Vec<(String, Value)>, Result<InputError, String>> {
    let doc: toml::Table = content.parse().map_err(|e: toml::de::Error| Err(e.to_string()))?;
    let mut settings = Vec::new();
    for section in ["vulture", "condor"] {
        if let Some(toml::Value::Table(t)) = doc.get("tool").and_then(|t| t.get(section)) {
            settings.extend(t.iter().map(|(k, v)| (k.clone(), convert(v))));
        }
    }
    check_input_config(&settings).map_err(Ok)?;
    Ok(settings)
}

/// Names used by entry points: the object path after `:` in
/// `[project.scripts]`, `[project.gui-scripts]`, `[project.entry-points.*]`
/// and `[tool.poetry.scripts]` (`pkg.cli:main` uses `main`).
pub fn entry_point_names(content: &str) -> Vec<String> {
    let Ok(doc) = content.parse::<toml::Table>() else { return Vec::new() };
    let mut tables: Vec<&toml::Table> = Vec::new();
    if let Some(project) = doc.get("project").and_then(|p| p.as_table()) {
        for key in ["scripts", "gui-scripts"] {
            tables.extend(project.get(key).and_then(|t| t.as_table()));
        }
        if let Some(groups) = project.get("entry-points").and_then(|t| t.as_table()) {
            tables.extend(groups.values().filter_map(|g| g.as_table()));
        }
    }
    tables.extend(doc.get("tool").and_then(|t| t.get("poetry")).and_then(|p| p.get("scripts")).and_then(|t| t.as_table()));
    let mut names = Vec::new();
    for table in tables {
        for value in table.values() {
            let target = match value {
                toml::Value::String(s) => s.as_str(),
                toml::Value::Table(t) => t.get("reference").or(t.get("callable")).and_then(|v| v.as_str()).unwrap_or(""),
                _ => "",
            };
            let object = target.split('[').next().unwrap_or("").trim();
            if let Some((_, path)) = object.split_once(':') {
                names.extend(path.split('.').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()));
            }
        }
    }
    names
}

fn strings(list: &[toml::Value], key: &str) -> Result<Vec<String>, String> {
    list.iter()
        .map(|v| match v {
            toml::Value::String(s) => Ok(s.clone()),
            // vulture accepts any list element and fails later with a TypeError.
            other => Err(format!("{key}: expected a list of strings, found {other}")),
        })
        .collect()
}

/// What `make_config` produced.
pub enum Outcome {
    /// The configuration, and the verbose "Reading configuration" line.
    Config(Config, Option<String>),
    /// vulture's `InputError` (exit status 2), with the verbose line if it was
    /// printed before the error.
    InputError(String, Option<String>),
    /// Any other early exit.
    Exit(Exit),
}

/// `make_config`: parse the command line, read the config file, merge (the
/// command line wins), apply defaults and check that there are paths.
/// `toml_content` replaces reading the file (vulture's `tomlfile` parameter).
pub fn make_config(cli: CliArgs, cwd: &Path, toml_content: Option<(&str, String)>) -> Outcome {
    let exit = |code: i32, stderr: String| Outcome::Exit(Exit { code, stdout: String::new(), stderr });
    let mut entry_points = Vec::new();
    let (settings, detected) = match toml_content {
        Some((label, content)) => match parse_toml(&content) {
            Ok(s) => {
                entry_points = entry_point_names(&content);
                (s, Some(label.to_string()))
            }
            Err(Ok(InputError(msg))) => return Outcome::InputError(msg, None),
            Err(Err(msg)) => return exit(1, format!("Error: invalid TOML in {label}: {msg}\n")),
        },
        None => {
            let toml_path = condor_core::paths::resolve(Path::new(&cli.config), cwd);
            if toml_path.is_file() {
                let content = match std::fs::read(&toml_path) {
                    Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                    Err(e) => return exit(1, format!("Error: could not read {}: {e}\n", toml_path.display())),
                };
                let label = toml_path.display().to_string();
                match parse_toml(&content) {
                    Ok(s) => {
                        entry_points = entry_point_names(&content);
                        (s, Some(label))
                    }
                    Err(Ok(InputError(msg))) => return Outcome::InputError(msg, None),
                    Err(Err(msg)) => return exit(1, format!("Error: invalid TOML in {label}: {msg}\n")),
                }
            } else {
                (Vec::new(), None)
            }
        }
    };

    let mut config = Config { entry_point_names: entry_points, ..Config::default() };
    for (key, value) in &settings {
        let list = |v: &Value| match v {
            Value::List(l) => strings(l, key),
            _ => unreachable!("type-checked"),
        };
        let res = match (key.as_str(), value) {
            ("paths", v) => list(v).map(|l| config.paths = l),
            ("exclude", v) => list(v).map(|l| config.exclude = l),
            ("ignore_decorators", v) => list(v).map(|l| config.ignore_decorators = l),
            ("ignore_names", v) => list(v).map(|l| config.ignore_names = l),
            ("make_whitelist", Value::Bool(b)) => Ok(config.make_whitelist = *b),
            ("min_confidence", Value::Int(i)) => Ok(config.min_confidence = *i),
            ("sort_by_size", Value::Bool(b)) => Ok(config.sort_by_size = *b),
            ("verbose", Value::Bool(b)) => Ok(config.verbose = *b),
            ("config", Value::Str(s)) => Ok(config.config = s.clone()),
            ("format", Value::Str(s)) => Ok(config.format = s.clone()),
            ("no_report", v) => list(v).map(|l| config.no_report = l),
            ("only", v) => list(v).map(|l| config.only = l),
            ("plugins", v) => list(v).map(|l| config.plugins = l),
            _ => Ok(()),
        };
        if let Err(msg) = res {
            return exit(1, format!("Error: {msg}\n"));
        }
    }
    // The command line overrides the file.
    if let Some(v) = cli.paths {
        config.paths = v;
    }
    if let Some(v) = cli.exclude {
        config.exclude = v;
    }
    if let Some(v) = cli.ignore_decorators {
        config.ignore_decorators = v;
    }
    if let Some(v) = cli.ignore_names {
        config.ignore_names = v;
    }
    if let Some(v) = cli.make_whitelist {
        config.make_whitelist = v;
    }
    if let Some(v) = cli.min_confidence {
        config.min_confidence = v;
    }
    if let Some(v) = cli.sort_by_size {
        config.sort_by_size = v;
    }
    if let Some(v) = cli.verbose {
        config.verbose = v;
    }
    if let Some(v) = cli.format {
        config.format = v;
    }
    if let Some(v) = cli.no_report {
        config.no_report = v;
    }
    if let Some(v) = cli.only {
        config.only = v;
    }
    config.config = cli.config.clone();
    if !matches!(config.format.as_str(), "text" | "json" | "sarif") {
        return Outcome::InputError(format!("Unknown format: {} (expected text, json or sarif)", config.format), None);
    }
    if let Some(bad) = config.only.iter().find(|k| condor_core::ItemKind::parse(k).is_none()) {
        return Outcome::InputError(format!("Unknown kind in only: {bad}"), None);
    }

    let banner = match &detected {
        Some(path) if config.verbose => Some(format!("Reading configuration from {path}")),
        _ => None,
    };
    if config.paths.is_empty() {
        config.paths = cli.default_paths;
    }
    if config.paths.is_empty() {
        return Outcome::InputError("Please pass at least one file or directory".into(), banner);
    }
    Outcome::Config(config, banner)
}
