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

use crate::rules::{TAG_CLASS_ATTR, TAG_FIELD};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    pub plugin: Meta,
    #[serde(default)]
    pub used: Vec<UsedRule>,
    #[serde(default)]
    pub fields_used: Vec<FieldsRule>,
    #[serde(default)]
    pub parameters_used: Vec<ParametersRule>,
    #[serde(default)]
    pub injected_by_name: Vec<InjectedRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The plugin applies when the project imports one of these modules
    /// (top-level names). Empty: always.
    #[serde(default)]
    pub when_imported: Vec<String>,
}

/// Definitions that are used: functions, methods and classes matching every
/// selector given.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsedRule {
    #[serde(default)]
    pub decorated_by: Vec<String>,
    #[serde(default)]
    pub names: Vec<String>,
    /// Methods of classes inheriting from these.
    #[serde(default)]
    pub in_classes_inheriting: Vec<String>,
    /// Classes inheriting from these.
    #[serde(default)]
    pub inherits: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Fields {
    #[default]
    Annotated,
    Assigned,
    All,
}

/// Attributes of matching classes (and their subclasses) that are used.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldsRule {
    #[serde(default)]
    pub decorated_by: Vec<String>,
    /// Class names (for example Django's `Meta` option classes).
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub inherits: Vec<String>,
    /// `annotated` (`x: int`), `assigned` (`X = 1`) or `all`.
    #[serde(default)]
    pub fields: Fields,
}

/// Functions whose parameters are not reported: their signature is imposed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParametersRule {
    #[serde(default)]
    pub decorated_by: Vec<String>,
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub in_classes_inheriting: Vec<String>,
}

/// Things a framework passes by parameter name, such as pytest fixtures.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InjectedRule {
    /// Functions that provide values (fixtures).
    pub providers: FunctionSelector,
    /// Keyword of the provider's decorator that renames it (`name=`).
    #[serde(default)]
    pub name_keyword: Option<String>,
    /// Keyword of the provider's decorator that makes it always used when
    /// true (`autouse=`).
    #[serde(default)]
    pub always_used_keyword: Option<String>,
    /// Functions that request providers through their parameters (tests).
    /// Providers request each other too.
    #[serde(default)]
    pub consumers: Consumers,
    /// Calls whose string arguments request providers (`usefixtures`).
    #[serde(default)]
    pub requested_by_calls: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionSelector {
    #[serde(default)]
    pub decorated_by: Vec<String>,
    #[serde(default)]
    pub names: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumers {
    /// Function names.
    #[serde(default)]
    pub names: Vec<String>,
    /// File patterns: a pattern with `/` is matched against the path, one
    /// without against the file name. Empty: every file.
    #[serde(default)]
    pub files: Vec<String>,
}

/// Parse a plugin file; the format follows the extension (`.toml`, `.json`).
pub fn parse(path: &str, content: &str) -> Result<Plugin, String> {
    let plugin: Plugin = if path.ends_with(".json") {
        serde_json::from_str(content).map_err(|e| format!("{path}: {e}"))?
    } else {
        toml::from_str(content).map_err(|e| format!("{path}: {e}"))?
    };
    validate(&plugin).map_err(|e| format!("{path}: {e}"))?;
    Ok(plugin)
}

fn validate(p: &Plugin) -> Result<(), String> {
    for r in &p.used {
        if r.decorated_by.is_empty() && r.names.is_empty() && r.in_classes_inheriting.is_empty() && r.inherits.is_empty() {
            return Err("a [[used]] rule needs at least one selector".into());
        }
        if !r.inherits.is_empty() && !r.in_classes_inheriting.is_empty() {
            return Err("a [[used]] rule selects classes (inherits) or methods (in_classes_inheriting), not both".into());
        }
    }
    for r in &p.fields_used {
        if r.decorated_by.is_empty() && r.inherits.is_empty() && r.names.is_empty() {
            return Err("a [[fields_used]] rule needs decorated_by, inherits or names".into());
        }
    }
    for r in &p.parameters_used {
        if r.decorated_by.is_empty() && r.names.is_empty() && r.in_classes_inheriting.is_empty() {
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

/// Classes by name, for following bases across files.
struct Hierarchy<'a> {
    classes: FxHashMap<&'a str, Vec<&'a ClassFacts>>,
}

impl<'a> Hierarchy<'a> {
    fn new(analysis: &'a Analysis) -> Hierarchy<'a> {
        let mut classes: FxHashMap<&str, Vec<&ClassFacts>> = FxHashMap::default();
        for c in analysis.scans.iter().flat_map(|s| &s.classes) {
            classes.entry(c.name.as_str()).or_default().push(c);
        }
        Hierarchy { classes }
    }

    /// Whether a class named `name` matches `direct` itself or through any
    /// of its bases defined in the project.
    fn reaches(&self, name: &str, direct: &dyn Fn(&ClassFacts) -> bool, memo: &mut FxHashMap<String, bool>) -> bool {
        if let Some(&known) = memo.get(name) {
            return known;
        }
        memo.insert(name.to_string(), false); // cycles
        let found = self.classes.get(name).into_iter().flatten().any(|c| {
            direct(c)
                || c.bases.iter().any(|b| self.reaches(b.rsplit('.').next().unwrap_or(b), direct, memo))
        });
        memo.insert(name.to_string(), found);
        found
    }
}

/// A class selector over the hierarchy: `decorated_by` on the class or a
/// project base, `inherits` on any base along the way.
struct ClassMatcher<'h, 'a> {
    hierarchy: &'h Hierarchy<'a>,
    decorated_by: Selector,
    inherits: Selector,
    names: Selector,
    memo: FxHashMap<String, bool>,
}

impl<'h, 'a> ClassMatcher<'h, 'a> {
    fn new(hierarchy: &'h Hierarchy<'a>, decorated_by: &[String], inherits: &[String]) -> Self {
        ClassMatcher {
            hierarchy,
            decorated_by: Selector::new(decorated_by),
            inherits: Selector::new(inherits),
            names: Selector::new(&[]),
            memo: FxHashMap::default(),
        }
    }

    fn with_names(mut self, names: &[String]) -> Self {
        self.names = Selector::new(names);
        self
    }

    fn matches(&mut self, class: &str) -> bool {
        if !self.names.is_empty() && self.names.matches(class) {
            return true;
        }
        let (decorated_by, inherits) = (&self.decorated_by, &self.inherits);
        if decorated_by.is_empty() && inherits.is_empty() {
            return false;
        }
        let direct = |c: &ClassFacts| {
            (!decorated_by.is_empty() && decorated_by.any_decorator(&c.decorators))
                || (!inherits.is_empty() && c.bases.iter().any(|b| inherits.matches(b)))
        };
        self.hierarchy.reaches(class, &direct, &mut self.memo)
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
    let imported: FxHashSet<&str> = analysis.scans.iter().flat_map(|s| s.imports.iter().map(String::as_str)).collect();
    let active: Vec<&Plugin> = plugins
        .iter()
        .filter(|p| p.plugin.when_imported.is_empty() || p.plugin.when_imported.iter().any(|m| imported.contains(m.as_str())))
        .collect();
    if active.is_empty() {
        return;
    }
    let hierarchy = Hierarchy::new(analysis);
    // (scan index, item index) pairs to remove.
    let mut remove: FxHashSet<(usize, usize)> = FxHashSet::default();

    for plugin in &active {
        for rule in &plugin.used {
            let decorated_by = Selector::new(&rule.decorated_by);
            let names = Selector::new(&rule.names);
            let mut in_classes = ClassMatcher::new(&hierarchy, &[], &rule.in_classes_inheriting);
            let mut classes = ClassMatcher::new(&hierarchy, &[], &rule.inherits);
            for (si, scan) in analysis.scans.iter().enumerate() {
                if rule.inherits.is_empty() {
                    for f in &scan.functions {
                        let ok = (decorated_by.is_empty() || decorated_by.any_decorator(&f.decorators))
                            && (names.is_empty() || names.matches(&f.name))
                            && (rule.in_classes_inheriting.is_empty()
                                || f.class.as_deref().is_some_and(|c| in_classes.matches(c)));
                        if ok {
                            remove.extend(f.item.map(|i| (si, i)));
                        }
                    }
                }
                if rule.in_classes_inheriting.is_empty() {
                    for c in &scan.classes {
                        let ok = (decorated_by.is_empty() || decorated_by.any_decorator(&c.decorators))
                            && (names.is_empty() || names.matches(&c.name))
                            && (rule.inherits.is_empty() || classes.matches(&c.name));
                        if ok {
                            remove.extend(c.item.map(|i| (si, i)));
                        }
                    }
                }
            }
        }

        for rule in &plugin.fields_used {
            let mut matcher = ClassMatcher::new(&hierarchy, &rule.decorated_by, &rule.inherits).with_names(&rule.names);
            for (si, scan) in analysis.scans.iter().enumerate() {
                for (ii, item) in scan.items.iter().enumerate() {
                    let wanted = match item.tag {
                        TAG_FIELD => rule.fields != Fields::Assigned,
                        TAG_CLASS_ATTR => rule.fields != Fields::Annotated,
                        _ => false,
                    };
                    if wanted && item.context.as_deref().is_some_and(|c| matcher.matches(c)) {
                        remove.insert((si, ii));
                    }
                }
            }
        }

        for rule in &plugin.parameters_used {
            let decorated_by = Selector::new(&rule.decorated_by);
            let names = Selector::new(&rule.names);
            let mut in_classes = ClassMatcher::new(&hierarchy, &[], &rule.in_classes_inheriting);
            for (si, scan) in analysis.scans.iter().enumerate() {
                for f in &scan.functions {
                    let ok = (decorated_by.is_empty() || decorated_by.any_decorator(&f.decorators))
                        && (names.is_empty() || names.matches(&f.name))
                        && (rule.in_classes_inheriting.is_empty()
                            || f.class.as_deref().is_some_and(|c| in_classes.matches(c)));
                    if ok {
                        remove.extend(f.params.iter().filter_map(|(_, i)| i.map(|i| (si, i))));
                    }
                }
            }
        }

        for rule in &plugin.injected_by_name {
            apply_injected(analysis, rule, &mut remove);
        }
    }

    for (si, scan) in analysis.scans.iter_mut().enumerate() {
        let mut index = 0;
        scan.items.retain(|_| {
            let keep = !remove.contains(&(si, index));
            index += 1;
            keep
        });
    }
}

fn apply_injected(analysis: &Analysis, rule: &InjectedRule, remove: &mut FxHashSet<(usize, usize)>) {
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
}
