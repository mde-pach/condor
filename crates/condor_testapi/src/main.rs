//! JSON bridge used by `tests/upstream`: the stand-in `vulture` Python package
//! sends one request on stdin and reads one response on stdout. It exposes
//! condor's internals with the shape of vulture's Python API so that vulture's
//! own test suite can run against condor.

use std::io::Read;
use std::path::{Path, PathBuf};

use condor::args::{self, CliArgs};
use condor::config::{self, Outcome, Value as ConfigValue};
use condor::run::{self, Output};
use condor_core::fnmatch::PatternSet;
use condor_core::{Analysis, Item, ItemKind, ScanOptions};
use serde_json::{Value, json};

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).expect("read request");
    let req: Value = serde_json::from_str(&input).expect("valid JSON request");
    let resp = match req["op"].as_str().unwrap_or_default() {
        "session" => session(&req),
        "condition" => condition(&req),
        "decorator_name" => decorator_name(&req),
        "format_path" => format_path(&req),
        "parse_args" => parse_args(&req),
        "parse_toml" => parse_toml(&req),
        "check_input_config" => check_input_config(&req),
        "make_config" => make_config(&req),
        "constants" => constants(),
        "noqa_search" => noqa_search(&req),
        "explain" => explain(&req),
        op => json!({"error": format!("unknown op {op}")}),
    };
    println!("{resp}");
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

fn item_json(item: &Item, cwd: &Path, sort_by_size: bool) -> Value {
    json!({
        "name": item.name,
        "typ": item.kind.as_str(),
        "filename": item.file.to_string_lossy(),
        "first_lineno": item.first_line,
        "last_lineno": item.last_line,
        "message": item.message(),
        "confidence": item.confidence,
        "size": item.size(),
        "report": item.report(cwd, sort_by_size),
        "whitelist": item.whitelist_string(cwd),
    })
}

fn out_json(out: &Output) -> Value {
    json!({
        "stdout": String::from_utf8_lossy(&out.stdout),
        "stderr": String::from_utf8_lossy(&out.stderr),
    })
}

/// A `Vulture` object: replay its `scan`/`scavenge` calls, then answer a query.
fn session(req: &Value) -> Value {
    let cwd = PathBuf::from(req["cwd"].as_str().unwrap_or("."));
    let python = condor_python::Python::bundled();
    let opts = ScanOptions {
        call_hints: condor_python::plugins::call_hints(&python.plugins),
        verbose: req["verbose"].as_bool().unwrap_or(false),
        cwd: cwd.clone(),
        ignore_names: PatternSet::new(&strings(&req["ignore_names"])),
        ignore_decorators: PatternSet::new(&strings(&req["ignore_decorators"])),
    };
    let mut analysis = Analysis::default();
    let mut outputs = Vec::new();
    let mut exit = Value::Null;
    for action in req["actions"].as_array().into_iter().flatten() {
        let mut out = Output::default();
        if let Some(scan) = action.get("scan") {
            // `Path("")` is `.`.
            let filename = match scan["filename"].as_str().unwrap_or("") {
                "" => ".",
                f => f,
            };
            let result = run::scan_source(scan["code"].as_str().unwrap_or(""), filename, &opts);
            for (stream, text) in &result.log {
                let buf = match stream {
                    condor_core::Stream::Stdout => &mut out.stdout,
                    condor_core::Stream::Stderr => &mut out.stderr,
                };
                buf.extend_from_slice(text.as_bytes());
                buf.push(b'\n');
            }
            analysis.push(result);
        } else if let Some(sc) = action.get("scavenge") {
            match condor::discover::get_modules(&strings(&sc["paths"]), &cwd) {
                Ok(modules) => {
                    let exclude = run::exclude_patterns(&strings(&sc["exclude"]));
                    if let Err(code) = run::scavenge(&python, &modules, &exclude, &opts, &mut analysis, &mut out) {
                        exit = json!({"code": code});
                    }
                }
                Err(msg) => exit = json!({"code": 1, "message": msg}),
            }
        }
        outputs.push(out_json(&out));
        if !exit.is_null() {
            break;
        }
    }

    run::finish(&python, &mut analysis, &config::Config::default(), &cwd);
    let used = analysis.used_names();
    let mut defined = serde_json::Map::new();
    let mut unused = serde_json::Map::new();
    for kind in ItemKind::ALL {
        let key = kind.as_str().to_string();
        defined.insert(key.clone(), analysis.defined(kind).map(|i| item_json(i, &cwd, false)).collect());
        unused.insert(key, analysis.unused(kind, &used).into_iter().map(|i| item_json(i, &cwd, false)).collect());
    }
    let mut used_names: Vec<&str> = used.into_iter().collect();
    used_names.sort_unstable();
    let exit_code = if analysis.invalid_input() { 1 } else { 0 };

    let query = &req["query"];
    let query_result = if let Some(q) = query.get("unused_code") {
        let min = q["min_confidence"].as_i64().unwrap_or(0);
        if !(0..=100).contains(&min) {
            json!({"value_error": "min_confidence must be between 0 and 100."})
        } else {
            let sort = q["sort_by_size"].as_bool().unwrap_or(false);
            json!({"items": analysis.unused_code(min, sort).into_iter().map(|i| item_json(i, &cwd, sort)).collect::<Vec<_>>()})
        }
    } else if let Some(q) = query.get("report") {
        let cfg = config::Config {
            make_whitelist: q["make_whitelist"].as_bool().unwrap_or(false),
            min_confidence: q["min_confidence"].as_i64().unwrap_or(0),
            sort_by_size: q["sort_by_size"].as_bool().unwrap_or(false),
            ..config::Config::default()
        };
        let mut out = Output::default();
        let code = run::report(&analysis, &cfg, &cwd, &mut out);
        json!({"code": code, "output": out_json(&out)})
    } else {
        Value::Null
    };

    json!({
        "outputs": outputs,
        "exit": exit,
        "defined": defined,
        "unused": unused,
        "used_names": used_names,
        "exit_code": exit_code,
        "query": query_result,
    })
}

fn parse_expr(req: &Value) -> Option<ruff_python_ast_expr::Expr> {
    ruff_python_ast_expr::parse(req["expr"].as_str()?)
}

/// Expression parsing goes through the condor_python dependency's parser.
mod ruff_python_ast_expr {
    pub use condor_python::scanner::PyExpr as Expr;

    pub fn parse(src: &str) -> Option<Expr> {
        condor_python::scanner::parse_expression(src)
    }
}

fn condition(req: &Value) -> Value {
    let Some(e) = parse_expr(req) else { return json!({"error": "parse"}) };
    json!({
        "always_true": condor_python::literal::always_true(&e),
        "always_false": condor_python::literal::always_false(&e),
    })
}

fn decorator_name(req: &Value) -> Value {
    let Some(e) = parse_expr(req) else { return json!({"error": "parse"}) };
    json!({"name": condor_python::scanner::decorator_name(&e)})
}

fn format_path(req: &Value) -> Value {
    let path = PathBuf::from(req["path"].as_str().unwrap_or(""));
    let cwd = PathBuf::from(req["cwd"].as_str().unwrap_or("."));
    json!({"path": condor_core::paths::format_path(&path, &cwd).to_string_lossy()})
}

fn cli_dict(cli: &CliArgs) -> Value {
    let mut m = serde_json::Map::new();
    if let Some(v) = &cli.paths {
        m.insert("paths".into(), json!(v));
    }
    if let Some(v) = &cli.exclude {
        m.insert("exclude".into(), json!(v));
    }
    if let Some(v) = &cli.ignore_decorators {
        m.insert("ignore_decorators".into(), json!(v));
    }
    if let Some(v) = &cli.ignore_names {
        m.insert("ignore_names".into(), json!(v));
    }
    if let Some(v) = cli.make_whitelist {
        m.insert("make_whitelist".into(), json!(v));
    }
    if let Some(v) = cli.min_confidence {
        m.insert("min_confidence".into(), json!(v));
    }
    if let Some(v) = cli.sort_by_size {
        m.insert("sort_by_size".into(), json!(v));
    }
    m.insert("config".into(), json!(cli.config));
    if let Some(v) = cli.verbose {
        m.insert("verbose".into(), json!(v));
    }
    Value::Object(m)
}

fn exit_json(e: &args::Exit) -> Value {
    json!({"exit": {"code": e.code, "stdout": e.stdout, "stderr": e.stderr}})
}

/// For `harness/plant.py`: scan a project as `condor ARGV` does and list the
/// items the plugins remove, with the rule that removes each.
fn explain(req: &Value) -> Value {
    let cwd = PathBuf::from(req["cwd"].as_str().unwrap_or("."));
    let cli = match args::parse(&strings(&req["argv"])) {
        Ok(cli) => cli,
        Err(e) => return exit_json(&e),
    };
    let Outcome::Config(config, _) = config::make_config(cli, &cwd, None) else {
        return json!({"error": "invalid configuration"});
    };
    let result = (|| {
        let modules = condor::discover::get_modules(&config.paths, &cwd)?;
        let python = run::load_python(&config, &cwd)?;
        let opts = run::scan_options(&python, &config, &cwd);
        let exclude = run::exclude_patterns(&config.exclude);
        let mut analysis = Analysis::default();
        let mut out = Output::default();
        let _ = run::scavenge(&python, &modules, &exclude, &opts, &mut analysis, &mut out);
        Ok::<_, String>(condor_python::plugins::explain(&analysis, &python.plugins))
    })();
    match result {
        Ok(removed) => Value::Array(
            removed
                .iter()
                .map(|(item, why)| {
                    json!({
                        "path": condor_core::paths::format_path(&item.file, &cwd).to_string_lossy(),
                        "line": item.first_line,
                        "name": item.name,
                        "kind": item.kind.as_str(),
                        "rule": why,
                    })
                })
                .collect(),
        ),
        Err(e) => json!({"error": e}),
    }
}

fn parse_args(req: &Value) -> Value {
    match args::parse(&strings(&req["argv"])) {
        Ok(cli) => json!({"ok": cli_dict(&cli)}),
        Err(e) => exit_json(&e),
    }
}

fn config_value_json(v: &ConfigValue) -> Value {
    match v {
        ConfigValue::Str(s) => json!(s),
        ConfigValue::Int(i) => json!(i),
        ConfigValue::Bool(b) => json!(b),
        ConfigValue::List(l) => Value::Array(l.iter().map(|x| json!(x.to_string().trim_matches('"'))).collect()),
        ConfigValue::Other(t) => json!(t),
    }
}

fn parse_toml(req: &Value) -> Value {
    match config::parse_toml(req["content"].as_str().unwrap_or("")) {
        Ok(settings) => {
            let m: serde_json::Map<String, Value> =
                settings.iter().map(|(k, v)| (k.clone(), config_value_json(v))).collect();
            json!({"ok": m})
        }
        Err(Ok(config::InputError(msg))) => json!({"input_error": msg}),
        Err(Err(msg)) => json!({"toml_error": msg}),
    }
}

fn check_input_config(req: &Value) -> Value {
    let data: Vec<(String, ConfigValue)> = req["data"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(k, v)| {
            let value = match v {
                Value::String(s) => ConfigValue::Str(s.clone()),
                Value::Bool(b) => ConfigValue::Bool(*b),
                Value::Number(n) if n.is_i64() => ConfigValue::Int(n.as_i64().unwrap()),
                Value::Number(_) => ConfigValue::Other("float"),
                Value::Array(_) => ConfigValue::List(Vec::new()),
                Value::Object(_) => ConfigValue::Other("dict"),
                Value::Null => ConfigValue::Other("NoneType"),
            };
            (k.clone(), value)
        })
        .collect();
    match config::check_input_config(&data) {
        Ok(()) => json!({"ok": true}),
        Err(config::InputError(msg)) => json!({"input_error": msg}),
    }
}

fn make_config(req: &Value) -> Value {
    let cwd = PathBuf::from(req["cwd"].as_str().unwrap_or("."));
    let cli = match args::parse(&strings(&req["argv"])) {
        Ok(cli) => cli,
        Err(e) => return exit_json(&e),
    };
    let toml = req["toml"].as_str().map(|c| ("<tomlfile>", c.to_string()));
    match config::make_config(cli, &cwd, toml) {
        Outcome::Config(c, banner) => json!({
            "ok": {
                "paths": c.paths,
                "exclude": c.exclude,
                "ignore_decorators": c.ignore_decorators,
                "ignore_names": c.ignore_names,
                "make_whitelist": c.make_whitelist,
                "min_confidence": c.min_confidence,
                "sort_by_size": c.sort_by_size,
                "verbose": c.verbose,
                "config": c.config,
            },
            "stdout": banner.map(|b| format!("{b}\n")).unwrap_or_default(),
        }),
        Outcome::InputError(msg, banner) => json!({
            "input_error": msg,
            "stdout": banner.map(|b| format!("{b}\n")).unwrap_or_default(),
        }),
        Outcome::Exit(e) => exit_json(&e),
    }
}

fn constants() -> Value {
    let error_codes: serde_json::Map<String, Value> =
        ItemKind::ALL.iter().map(|k| (k.as_str().to_string(), json!(k.noqa_code()))).collect();
    let code_map: serde_json::Map<String, Value> =
        condor_python::noqa::NOQA_CODE_MAP.iter().map(|(f, v)| (f.to_string(), json!(v))).collect();
    json!({"error_codes": error_codes, "noqa_code_map": code_map})
}

fn noqa_search(req: &Value) -> Value {
    match condor_python::noqa::search(req["line"].as_str().unwrap_or("")) {
        None => json!({"match": false}),
        Some(codes) => json!({
            "match": true,
            "codes": codes,
            "parsed": condor_python::noqa::parse_error_codes(codes),
        }),
    }
}
