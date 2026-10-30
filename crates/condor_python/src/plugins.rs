//! Plugins: declarative rules that teach condor how libraries use code
//! (docs/plugins.md). A plugin is a TOML or JSON file with the same schema.
//!
//! Names in selectors are dotted names as the code imports them
//! (`pydantic.BaseModel`), with glob wildcards. condor resolves names through
//! each file's imports, so `from pydantic import BaseModel` and
//! `import pydantic as pd; pd.BaseModel` both match `pydantic.BaseModel`.
//! Names brought in by star imports (`from pydantic import *`) are not
//! resolved, so they do not match.

use std::path::Path;

use condor_core::fnmatch::PatternSet;
use condor_core::{Analysis, ClassFacts, Decorator, FunctionFacts};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Deserialize;

use crate::rules::{TAG_CLASS_ATTR, TAG_CLASS_ATTR_CALL, TAG_FIELD};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    /// Editor support for JSON plugins (`"$schema": ".../schema.json"`).
    #[serde(default, rename = "$schema")]
    pub schema: Option<String>,
    pub plugin: Meta,
    #[serde(default)]
    pub used: Vec<UsedRule>,
    #[serde(default)]
    pub fields_used: Vec<FieldsRule>,
    #[serde(default)]
    pub parameters_used: Vec<ParametersRule>,
    #[serde(default)]
    pub injected_by_name: Vec<InjectedRule>,
    #[serde(default)]
    pub variables_used: Vec<VariablesRule>,
}

/// Module and class variables a framework reads by name (`pytestmark`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariablesRule {
    #[serde(deserialize_with = "selectors")]
    pub names: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The plugin applies when the project imports one of these modules
    /// (top-level names). Empty: always.
    #[serde(default, deserialize_with = "selectors")]
    pub when_imported: Vec<String>,
}

/// Definitions that are used: functions, methods and classes matching every
/// selector given.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsedRule {
    #[serde(default, deserialize_with = "selectors")]
    pub decorated_by: Vec<String>,
    #[serde(default, deserialize_with = "selectors")]
    pub names: Vec<String>,
    /// Methods of classes inheriting from these.
    #[serde(default, deserialize_with = "selectors")]
    pub in_classes_inheriting: Vec<String>,
    /// Methods of classes listing one of these among their own bases (not
    /// their subclasses).
    #[serde(default, deserialize_with = "selectors")]
    pub in_classes_based_on: Vec<String>,
    /// Classes inheriting from these.
    #[serde(default, deserialize_with = "selectors")]
    pub inherits: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Fields {
    #[default]
    Annotated,
    Assigned,
    /// Assigned a call (`name = models.CharField()`).
    Calls,
    All,
}

impl Fields {
    /// Whether an attribute with scanner tag `tag` is selected.
    fn selects(self, tag: u8) -> bool {
        match tag {
            TAG_FIELD => matches!(self, Fields::Annotated | Fields::All),
            TAG_CLASS_ATTR => matches!(self, Fields::Assigned | Fields::All),
            TAG_CLASS_ATTR_CALL => matches!(self, Fields::Assigned | Fields::Calls | Fields::All),
            _ => false,
        }
    }
}

/// Attributes of matching classes (and their subclasses) that are used.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldsRule {
    #[serde(default, deserialize_with = "selectors")]
    pub decorated_by: Vec<String>,
    /// Class names (for example Django's `Meta` option classes).
    #[serde(default, deserialize_with = "selectors")]
    pub names: Vec<String>,
    #[serde(default, deserialize_with = "selectors")]
    pub inherits: Vec<String>,
    /// `annotated` (`x: int`), `assigned` (`X = 1`), `calls` (`x = f()`) or
    /// `all`.
    #[serde(default)]
    pub fields: Fields,
    /// Only attributes with these names (the options a framework reads).
    #[serde(default, deserialize_with = "selectors")]
    pub attributes: Vec<String>,
}

/// Functions whose parameters are not reported: their signature is imposed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParametersRule {
    #[serde(default, deserialize_with = "selectors")]
    pub decorated_by: Vec<String>,
    #[serde(default, deserialize_with = "selectors")]
    pub names: Vec<String>,
    #[serde(default, deserialize_with = "selectors")]
    pub in_classes_inheriting: Vec<String>,
    #[serde(default, deserialize_with = "selectors")]
    pub in_classes_based_on: Vec<String>,
}

/// Things a framework passes by parameter name, such as pytest fixtures.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InjectedRule {
    /// Functions that provide values (fixtures).
    pub providers: FunctionSelector,
    /// Keyword of the provider's decorator that renames it (`name=`).
    #[serde(default, deserialize_with = "string")]
    pub name_keyword: Option<String>,
    /// Keyword of the provider's decorator that makes it always used when
    /// true (`autouse=`).
    #[serde(default, deserialize_with = "string")]
    pub always_used_keyword: Option<String>,
    /// Functions that request providers through their parameters (tests).
    /// Providers request each other too.
    #[serde(default)]
    pub consumers: Consumers,
    /// Calls whose string arguments request providers (`usefixtures`).
    #[serde(default, deserialize_with = "selectors")]
    pub requested_by_calls: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionSelector {
    #[serde(default, deserialize_with = "selectors")]
    pub decorated_by: Vec<String>,
    #[serde(default, deserialize_with = "selectors")]
    pub names: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumers {
    /// Function names.
    #[serde(default, deserialize_with = "selectors")]
    pub names: Vec<String>,
    /// File patterns: a pattern with `/` is matched against the path, one
    /// without against the file name. Empty: every file.
    #[serde(default, deserialize_with = "selectors")]
    pub files: Vec<String>,
}

/// A selector list: given, it names at least one pattern.
fn selectors<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let list = Vec::<String>::deserialize(d)?;
    if list.is_empty() {
        return Err(serde::de::Error::custom("an empty list selects nothing; leave the key out instead"));
    }
    Ok(list)
}

/// An optional string: absent, not `null`.
fn string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

/// The plugin formats, by file extension (case-insensitive).
pub fn format_of(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "toml" => Some("toml"),
        "json" => Some("json"),
        _ => None,
    }
}

/// Parse a plugin file; the format follows the extension (`.toml`, `.json`).
pub fn parse(path: &str, content: &str) -> Result<Plugin, String> {
    let plugin: Plugin = match format_of(Path::new(path)) {
        Some("json") => {
            let value: serde_json::Value = serde_json::from_str(content).map_err(|e| format!("{path}: {e}"))?;
            if !value.is_object() {
                return Err(format!("{path}: a plugin is a JSON object with a \"plugin\" key"));
            }
            serde_json::from_value(value).map_err(|e| format!("{path}: {e}"))?
        }
        Some(_) => toml::from_str(content).map_err(|e| format!("{path}: {e}"))?,
        None => return Err(format!("{path}: a plugin file ends with .toml or .json")),
    };
    validate(&plugin).map_err(|e| format!("{path}: {e}"))?;
    Ok(plugin)
}

fn validate(p: &Plugin) -> Result<(), String> {
    if p.plugin.name.trim().is_empty() {
        return Err("the plugin needs a name".into());
    }
    if let Some(m) = p.plugin.when_imported.iter().find(|m| m.contains('.') || m.trim().is_empty()) {
        return Err(format!("when_imported takes top-level module names (\"{}\" for \"{m}\")", m.split('.').next().unwrap_or(m)));
    }
    let patterns = p
        .used
        .iter()
        .flat_map(|r| [&r.decorated_by, &r.names, &r.in_classes_inheriting, &r.in_classes_based_on, &r.inherits])
        .chain(p.fields_used.iter().flat_map(|r| [&r.decorated_by, &r.names, &r.inherits, &r.attributes]))
        .chain(p.parameters_used.iter().flat_map(|r| [&r.decorated_by, &r.names, &r.in_classes_inheriting, &r.in_classes_based_on]))
        .chain(p.variables_used.iter().map(|r| &r.names))
        .chain(p.injected_by_name.iter().flat_map(|r| {
            [&r.providers.decorated_by, &r.providers.names, &r.consumers.names, &r.consumers.files, &r.requested_by_calls]
        }))
        .flatten();
    for pattern in patterns {
        if let Some(problem) = pattern_problem(pattern) {
            return Err(format!("invalid pattern \"{pattern}\": {problem}"));
        }
    }
    for r in &p.used {
        let methods = !r.in_classes_inheriting.is_empty() || !r.in_classes_based_on.is_empty();
        if r.decorated_by.is_empty() && r.names.is_empty() && !methods && r.inherits.is_empty() {
            return Err("a [[used]] rule needs at least one selector".into());
        }
        if !r.inherits.is_empty() && methods {
            return Err("a [[used]] rule selects classes (inherits) or methods (in_classes_inheriting, in_classes_based_on), not both".into());
        }
    }
    for r in &p.fields_used {
        if r.decorated_by.is_empty() && r.inherits.is_empty() && r.names.is_empty() {
            return Err("a [[fields_used]] rule needs decorated_by, inherits or names".into());
        }
    }
    for r in &p.parameters_used {
        if r.decorated_by.is_empty() && r.names.is_empty() && r.in_classes_inheriting.is_empty() && r.in_classes_based_on.is_empty() {
            return Err("a [[parameters_used]] rule needs at least one selector".into());
        }
    }
    for r in &p.injected_by_name {
        if r.providers.decorated_by.is_empty() && r.providers.names.is_empty() {
            return Err("an [[injected_by_name]] rule needs providers".into());
        }
    }
    Ok(())
}

/// Why a glob pattern cannot be what was meant (an unclosed `[`, which
/// fnmatch would take literally).
fn pattern_problem(pattern: &str) -> Option<&'static str> {
    if pattern.trim().is_empty() {
        return Some("empty");
    }
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            // `[]...]` and `[!]...]` include a literal `]`.
            if chars.peek() == Some(&'!') {
                chars.next();
            }
            if chars.peek() == Some(&']') {
                chars.next();
            }
            if !chars.any(|c| c == ']') {
                return Some("unclosed [");
            }
        }
    }
    None
}

mod bundled {
    include!(concat!(env!("OUT_DIR"), "/bundled_plugins.rs"));
}

/// The plugins shipped with condor (a pinned snapshot of condor-plugins).
pub fn bundled() -> Vec<Plugin> {
    bundled::PLUGINS
        .iter()
        .map(|(path, content)| parse(path, content).unwrap_or_else(|e| panic!("bundled plugin is invalid: {e}")))
        .collect()
}

/// The paths of the bundled plugin files.
pub fn bundled_paths() -> impl Iterator<Item = &'static str> {
    bundled::PLUGINS.iter().map(|(path, _)| *path)
}

/// Every `requested_by_calls` pattern, for the scanner.
pub fn call_hints(plugins: &[Plugin]) -> PatternSet {
    let patterns: Vec<&String> =
        plugins.iter().flat_map(|p| &p.injected_by_name).flat_map(|r| &r.requested_by_calls).collect();
    PatternSet::new(&patterns)
}

/// Dotted-name selectors with glob wildcards.
struct Selector {
    patterns: PatternSet,
    raw: Vec<String>,
}

impl Selector {
    fn new(raw: &[String]) -> Selector {
        Selector { patterns: PatternSet::new(raw), raw: raw.to_vec() }
    }

    fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    fn matches(&self, name: &str) -> bool {
        self.patterns.matches_case(name.trim_start_matches('@'))
    }

    fn any_decorator(&self, decorators: &[Decorator]) -> bool {
        decorators.iter().any(|d| self.matches(&d.name))
    }
}

/// A class: (scan index, index in `FileScan::classes`).
type ClassId = (usize, usize);

/// The project's classes and their bases, resolved through each file's
/// imports to the class they name.
struct Hierarchy<'a> {
    analysis: &'a Analysis,
    /// Classes by name, per file, in definition order.
    local: Vec<FxHashMap<&'a str, Vec<usize>>>,
    /// Imported names, per file (local name -> dotted name).
    aliases: Vec<FxHashMap<&'a str, &'a str>>,
    /// Classes by name, across the project.
    global: FxHashMap<&'a str, Vec<ClassId>>,
    /// Files by module name (`pkg.models`).
    modules: FxHashMap<String, Vec<usize>>,
    paths: FxHashMap<&'a Path, usize>,
    /// The project classes each class's bases resolve to.
    bases: FxHashMap<ClassId, Vec<ClassId>>,
}

/// How many re-exports to follow (`from .base import Base` in a package).
const MAX_REEXPORTS: u8 = 8;

impl<'a> Hierarchy<'a> {
    fn new(analysis: &'a Analysis) -> Hierarchy<'a> {
        let mut hierarchy = Hierarchy {
            analysis,
            local: Vec::with_capacity(analysis.scans.len()),
            aliases: Vec::with_capacity(analysis.scans.len()),
            global: FxHashMap::default(),
            modules: FxHashMap::default(),
            paths: FxHashMap::default(),
            bases: FxHashMap::default(),
        };
        for (si, scan) in analysis.scans.iter().enumerate() {
            let mut names: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
            for (ci, c) in scan.classes.iter().enumerate() {
                names.entry(c.name.as_str()).or_default().push(ci);
                hierarchy.global.entry(c.name.as_str()).or_default().push((si, ci));
            }
            hierarchy.local.push(names);
            hierarchy.aliases.push(scan.aliases.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect());
            if let Some(file) = scan.file.as_deref() {
                hierarchy.paths.insert(file, si);
                for module in condor_core::paths::module_names(file) {
                    hierarchy.modules.entry(module).or_default().push(si);
                }
            }
        }
        let mut bases = FxHashMap::default();
        for (si, scan) in analysis.scans.iter().enumerate() {
            for (ci, c) in scan.classes.iter().enumerate() {
                let resolved: Vec<ClassId> =
                    c.bases.iter().flat_map(|b| hierarchy.resolve(si, b, Some(ci), 0)).collect();
                if !resolved.is_empty() {
                    bases.insert((si, ci), resolved);
                }
            }
        }
        hierarchy.bases = bases;
        hierarchy
    }

    fn class(&self, (si, ci): ClassId) -> &'a ClassFacts {
        &self.analysis.scans[si].classes[ci]
    }

    /// The class named `name` that file `si` exports: a top-level one, else
    /// any of that name in the file.
    fn named(&self, si: usize, name: &str) -> Vec<ClassId> {
        let Some(found) = self.local[si].get(name) else { return Vec::new() };
        let top: Vec<ClassId> =
            found.iter().filter(|&&ci| self.analysis.scans[si].classes[ci].class.is_none()).map(|&ci| (si, ci)).collect();
        if top.is_empty() { found.iter().map(|&ci| (si, ci)).collect() } else { top }
    }

    /// The class a bare base name in class `before` of file `si` refers to:
    /// the nearest one of that name defined before it (each test function
    /// may define its own `class C`), else any in the file.
    fn named_before(&self, si: usize, name: &str, before: Option<usize>) -> Vec<ClassId> {
        let Some(found) = self.local[si].get(name) else { return Vec::new() };
        match before.and_then(|b| found.iter().rev().find(|&&ci| ci < b)) {
            Some(&ci) => vec![(si, ci)],
            None => self.named(si, name),
        }
    }

    fn by_name(&self, name: &str) -> Vec<ClassId> {
        self.global.get(name).cloned().unwrap_or_default()
    }

    /// The project classes a base written in file `si` (in class `before`)
    /// refers to. A base imported from outside the project resolves to
    /// nothing (selectors match its dotted name directly).
    fn resolve(&self, si: usize, base: &str, before: Option<usize>, depth: u8) -> Vec<ClassId> {
        if depth > MAX_REEXPORTS {
            return Vec::new();
        }
        if let Some(rest) = base.strip_prefix('.') {
            // Relative import: `.models.Base` is `Base` in `./models.py`.
            let level = 1 + rest.len() - rest.trim_start_matches('.').len();
            let rest = rest.trim_start_matches('.');
            let (module, name) = rest.rsplit_once('.').unwrap_or(("", rest));
            let Some(mut dir) = self.analysis.scans[si].file.as_deref().and_then(Path::parent) else {
                return self.by_name(name);
            };
            for _ in 1..level {
                dir = dir.parent().unwrap_or(dir);
            }
            let module_dir = module.split('.').filter(|p| !p.is_empty()).fold(dir.to_path_buf(), |d, p| d.join(p));
            let candidates = if module.is_empty() {
                vec![dir.join("__init__.py")]
            } else {
                vec![module_dir.with_extension("py"), module_dir.join("__init__.py")]
            };
            for file in &candidates {
                if let Some(&target) = self.paths.get(file.as_path()) {
                    return self.exported(target, name, depth);
                }
            }
            // A module that was not scanned.
            return self.by_name(name);
        }
        match base.rsplit_once('.') {
            None => {
                let found = self.named_before(si, base, before);
                if !found.is_empty() {
                    found
                } else if self.analysis.scans[si].star_import {
                    // Brought in by `from m import *`: any project class.
                    self.by_name(base)
                } else {
                    // A builtin, or a name the file does not define.
                    Vec::new()
                }
            }
            Some((module, name)) => match self.modules.get(module) {
                Some(files) => files.iter().flat_map(|&f| self.exported(f, name, depth)).collect(),
                None => Vec::new(),
            },
        }
    }

    /// The class module file `si` provides as `name`: defined there, or
    /// imported there (a re-export such as `from .base import Base` in a
    /// package's `__init__.py`).
    fn exported(&self, si: usize, name: &str, depth: u8) -> Vec<ClassId> {
        let found = self.named(si, name);
        if !found.is_empty() {
            return found;
        }
        if let Some(target) = self.aliases[si].get(name) {
            return self.resolve(si, target, None, depth + 1);
        }
        if self.analysis.scans[si].star_import { self.by_name(name) } else { Vec::new() }
    }

    /// Whether class `start` matches `direct` itself or through any of its
    /// project bases. Iterative, so deep hierarchies cannot overflow.
    fn reaches(&self, start: ClassId, direct: &dyn Fn(ClassId) -> bool, memo: &mut FxHashMap<ClassId, bool>) -> bool {
        let mut stack = vec![(start, false)];
        let mut open: FxHashSet<ClassId> = FxHashSet::default();
        while let Some((node, expanded)) = stack.pop() {
            if memo.contains_key(&node) {
                continue;
            }
            let bases = self.bases.get(&node).map(Vec::as_slice).unwrap_or_default();
            if expanded {
                // A base still open is a cycle: it adds nothing.
                let found = bases.iter().any(|b| memo.get(b) == Some(&true));
                memo.insert(node, found);
                open.remove(&node);
            } else if direct(node) {
                memo.insert(node, true);
            } else if open.insert(node) {
                stack.push((node, true));
                stack.extend(bases.iter().filter(|b| !memo.contains_key(b) && !open.contains(b)).map(|&b| (b, false)));
            }
        }
        memo.get(&start).copied().unwrap_or(false)
    }
}

/// Where a plugin's rules apply. Rules that follow class hierarchies apply
/// anywhere, since bases are resolved. Others (decorator patterns such as
/// `*.get`, class names such as `Meta`) apply in files that import the
/// plugin's modules, or to decorators whose object is imported (`from
/// myapp import app; @app.route`), so a same-named local helper elsewhere
/// is not taken for the framework.
struct Gate<'p> {
    open: Vec<bool>,
    modules: &'p [String],
}

impl<'p> Gate<'p> {
    fn new(analysis: &Analysis, plugin: &'p Plugin) -> Gate<'p> {
        let modules = &plugin.plugin.when_imported;
        let open = analysis
            .scans
            .iter()
            .map(|s| modules.is_empty() || s.imports.iter().any(|m| modules.contains(m)))
            .collect();
        Gate { open, modules }
    }

    fn file(&self, si: usize) -> bool {
        self.open[si]
    }

    fn decorator(&self, si: usize, d: &Decorator) -> bool {
        self.open[si] || d.imported || self.modules.iter().any(|m| d.name.strip_prefix(m.as_str()).is_some_and(|r| r.starts_with('.')))
    }

    /// `selector` matches one of `decorators` in file `si`.
    fn decorated(&self, si: usize, selector: &Selector, decorators: &[Decorator]) -> bool {
        decorators.iter().any(|d| selector.matches(&d.name) && self.decorator(si, d))
    }
}

/// A class selector over the hierarchy: `decorated_by` on the class or a
/// project base, `inherits` on any base along the way, `names` on the class
/// itself. With `direct_only`, `inherits` looks at the class's own bases.
struct ClassMatcher<'h, 'a> {
    hierarchy: &'h Hierarchy<'a>,
    gate: &'h Gate<'h>,
    decorated_by: Selector,
    inherits: Selector,
    names: Selector,
    direct_only: bool,
    memo: FxHashMap<ClassId, bool>,
}

impl<'h, 'a> ClassMatcher<'h, 'a> {
    fn new(hierarchy: &'h Hierarchy<'a>, gate: &'h Gate<'h>, decorated_by: &[String], inherits: &[String]) -> Self {
        ClassMatcher {
            hierarchy,
            gate,
            decorated_by: Selector::new(decorated_by),
            inherits: Selector::new(inherits),
            names: Selector::new(&[]),
            direct_only: false,
            memo: FxHashMap::default(),
        }
    }

    fn with_names(mut self, names: &[String]) -> Self {
        self.names = Selector::new(names);
        self
    }

    fn direct_only(mut self) -> Self {
        self.direct_only = true;
        self
    }

    /// Whether class `id` matches, by name or through its hierarchy.
    fn matches(&mut self, id: ClassId) -> bool {
        if !self.names.is_empty() && self.names.matches(&self.hierarchy.class(id).name) && self.gate.file(id.0) {
            return true;
        }
        if self.decorated_by.is_empty() && self.inherits.is_empty() {
            return false;
        }
        self.matches_id(id)
    }

    fn matches_id(&mut self, id: ClassId) -> bool {
        let (hierarchy, gate, decorated_by, inherits) = (self.hierarchy, self.gate, &self.decorated_by, &self.inherits);
        let direct = |id: ClassId| {
            let c = hierarchy.class(id);
            (!decorated_by.is_empty() && gate.decorated(id.0, decorated_by, &c.decorators))
                || (!inherits.is_empty() && c.bases.iter().any(|b| inherits.matches(b)))
        };
        if self.direct_only {
            return direct(id);
        }
        hierarchy.reaches(id, &direct, &mut self.memo)
    }
}

fn file_matches(patterns: &[String], path: &Path) -> bool {
    if patterns.is_empty() {
        return true;
    }
    let full = path.to_string_lossy();
    let file = path.file_name().map(|f| f.to_string_lossy()).unwrap_or_default();
    patterns.iter().any(|p| {
        let pat = condor_core::fnmatch::Pattern::new(p);
        if p.contains('/') { pat.matches_case(&full) } else { pat.matches_case(&file) }
    })
}

/// Apply the active plugins once every file is scanned: remove the items
/// they declare used.
pub fn apply(analysis: &mut Analysis, plugins: &[Plugin]) {
    let remove = removals(analysis, plugins);
    if remove.is_empty() {
        return;
    }
    for (si, scan) in analysis.scans.iter_mut().enumerate() {
        let mut index = 0;
        scan.items.retain(|_| {
            let keep = !remove.contains_key(&(si, index));
            index += 1;
            keep
        });
    }
}

/// The items the plugins would remove, each with the rule that removes it
/// first (`django: used[3]`), in scan order. For reviewing the rules.
pub fn explain(analysis: &Analysis, plugins: &[Plugin]) -> Vec<(condor_core::Item, String)> {
    let remove = removals(analysis, plugins);
    let mut out: Vec<((usize, usize), condor_core::Item, String)> = remove
        .into_iter()
        .map(|((si, ii), (plugin, kind, rule))| {
            let item = analysis.scans[si].items[ii].clone();
            ((si, ii), item, format!("{}: {kind}[{rule}]", plugins[plugin].plugin.name))
        })
        .collect();
    out.sort_by_key(|(at, _, _)| *at);
    out.into_iter().map(|(_, item, why)| (item, why)).collect()
}

/// Why an item is removed: (plugin index, rule kind, rule index).
type Why = (usize, &'static str, usize);

/// The items to remove, by (scan index, item index).
fn removals(analysis: &Analysis, plugins: &[Plugin]) -> FxHashMap<(usize, usize), Why> {
    let mut remove: FxHashMap<(usize, usize), Why> = FxHashMap::default();
    let imported: FxHashSet<&str> = analysis.scans.iter().flat_map(|s| s.imports.iter().map(String::as_str)).collect();
    let active: Vec<(usize, &Plugin)> = plugins
        .iter()
        .enumerate()
        .filter(|(_, p)| p.plugin.when_imported.is_empty() || p.plugin.when_imported.iter().any(|m| imported.contains(m.as_str())))
        .collect();
    if active.is_empty() {
        return remove;
    }
    let hierarchy = Hierarchy::new(analysis);

    for &(pi, plugin) in &active {
        let gate = Gate::new(analysis, plugin);
        let mut mark = |at: (usize, usize), kind: &'static str, rule: usize| {
            remove.entry(at).or_insert((pi, kind, rule));
        };
        for (ri, rule) in plugin.used.iter().enumerate() {
            let decorated_by = Selector::new(&rule.decorated_by);
            let names = Selector::new(&rule.names);
            let mut in_classes = ClassMatcher::new(&hierarchy, &gate, &[], &rule.in_classes_inheriting);
            let mut based_on = ClassMatcher::new(&hierarchy, &gate, &[], &rule.in_classes_based_on).direct_only();
            let mut classes = ClassMatcher::new(&hierarchy, &gate, &[], &rule.inherits);
            let methods = !rule.in_classes_inheriting.is_empty() || !rule.in_classes_based_on.is_empty();
            for (si, scan) in analysis.scans.iter().enumerate() {
                // Without a class hierarchy to follow, the rule applies
                // where the framework is in use.
                let anchored = methods || !rule.inherits.is_empty();
                let names_ok = |name: &str| names.is_empty() || (names.matches(name) && (anchored || gate.file(si)));
                if rule.inherits.is_empty() {
                    for f in &scan.functions {
                        let ok = (decorated_by.is_empty() || gate.decorated(si, &decorated_by, &f.decorators))
                            && names_ok(&f.name)
                            && (rule.in_classes_inheriting.is_empty()
                                || f.class.is_some_and(|ci| in_classes.matches((si, ci))))
                            && (rule.in_classes_based_on.is_empty()
                                || f.class.is_some_and(|ci| based_on.matches((si, ci))));
                        if ok && let Some(i) = f.item {
                            mark((si, i), "used", ri);
                        }
                    }
                }
                if !methods {
                    for (ci, c) in scan.classes.iter().enumerate() {
                        let ok = (decorated_by.is_empty() || gate.decorated(si, &decorated_by, &c.decorators))
                            && names_ok(&c.name)
                            && (rule.inherits.is_empty() || classes.matches_id((si, ci)));
                        if ok && let Some(i) = c.item {
                            mark((si, i), "used", ri);
                        }
                    }
                }
            }
        }

        for (ri, rule) in plugin.fields_used.iter().enumerate() {
            let mut matcher =
                ClassMatcher::new(&hierarchy, &gate, &rule.decorated_by, &rule.inherits).with_names(&rule.names);
            let attributes = Selector::new(&rule.attributes);
            for (si, scan) in analysis.scans.iter().enumerate() {
                for (ci, class) in scan.classes.iter().enumerate() {
                    if class.fields.is_empty() || !matcher.matches((si, ci)) {
                        continue;
                    }
                    for &ii in &class.fields {
                        let item = &scan.items[ii];
                        if rule.fields.selects(item.tag) && (attributes.is_empty() || attributes.matches(&item.name)) {
                            mark((si, ii), "fields_used", ri);
                        }
                    }
                }
            }
        }

        for (ri, rule) in plugin.parameters_used.iter().enumerate() {
            let decorated_by = Selector::new(&rule.decorated_by);
            let names = Selector::new(&rule.names);
            let mut in_classes = ClassMatcher::new(&hierarchy, &gate, &[], &rule.in_classes_inheriting);
            let mut based_on = ClassMatcher::new(&hierarchy, &gate, &[], &rule.in_classes_based_on).direct_only();
            let anchored = !rule.in_classes_inheriting.is_empty() || !rule.in_classes_based_on.is_empty();
            for (si, scan) in analysis.scans.iter().enumerate() {
                for f in &scan.functions {
                    let ok = (decorated_by.is_empty() || gate.decorated(si, &decorated_by, &f.decorators))
                        && (names.is_empty() || (names.matches(&f.name) && (anchored || gate.file(si))))
                        && (rule.in_classes_inheriting.is_empty()
                            || f.class.is_some_and(|ci| in_classes.matches((si, ci))))
                        && (rule.in_classes_based_on.is_empty()
                            || f.class.is_some_and(|ci| based_on.matches((si, ci))));
                    if ok {
                        for i in f.params.iter().filter_map(|(_, i)| *i) {
                            mark((si, i), "parameters_used", ri);
                        }
                    }
                }
            }
        }

        for (ri, rule) in plugin.injected_by_name.iter().enumerate() {
            for at in injected(analysis, rule) {
                mark(at, "injected_by_name", ri);
            }
        }

        for (ri, rule) in plugin.variables_used.iter().enumerate() {
            let names = Selector::new(&rule.names);
            for (si, scan) in analysis.scans.iter().enumerate().filter(|(si, _)| gate.file(*si)) {
                for (ii, item) in scan.items.iter().enumerate() {
                    if item.kind == condor_core::ItemKind::Variable && names.matches(&item.name) {
                        mark((si, ii), "variables_used", ri);
                    }
                }
            }
        }
    }
    remove
}

/// The items an `injected_by_name` rule declares used.
fn injected(analysis: &Analysis, rule: &InjectedRule) -> Vec<(usize, usize)> {
    let mut remove = Vec::new();
    let providers_by = Selector::new(&rule.providers.decorated_by);
    let provider_names = Selector::new(&rule.providers.names);
    let consumer_names = Selector::new(&rule.consumers.names);
    let is_provider = |f: &FunctionFacts| {
        (!providers_by.is_empty() && providers_by.any_decorator(&f.decorators))
            || (!provider_names.is_empty() && provider_names.matches(&f.name))
    };
    let mut requested: FxHashSet<&str> = analysis.scans.iter().flat_map(|s| s.requested.iter().map(String::as_str)).collect();
    let mut providers: Vec<(usize, &FunctionFacts)> = Vec::new();
    for (si, scan) in analysis.scans.iter().enumerate() {
        let path = scan.file.as_deref();
        for f in &scan.functions {
            let provider = is_provider(f);
            let consumer = !consumer_names.is_empty()
                && consumer_names.matches(&f.name)
                && path.is_some_and(|p| file_matches(&rule.consumers.files, p));
            if provider || consumer {
                // The framework passes these parameters.
                remove.extend(f.params.iter().filter_map(|(_, i)| i.map(|i| (si, i))));
                requested.extend(f.params.iter().map(|(n, _)| n.as_str()));
            }
            if provider {
                providers.push((si, f));
            }
        }
    }
    for (si, f) in providers {
        let decorator = f.decorators.iter().find(|d| providers_by.matches(&d.name));
        let keyword = |k: &Option<String>| {
            k.as_ref().and_then(|k| decorator.and_then(|d| d.keywords.iter().find(|(name, _)| name == k)))
        };
        let always = keyword(&rule.always_used_keyword).is_some_and(|(_, v)| v == "true");
        let name = keyword(&rule.name_keyword).map_or(f.name.as_str(), |(_, v)| v.as_str());
        if always || requested.contains(name) {
            remove.extend(f.item.map(|i| (si, i)));
        }
    }
    remove
}
