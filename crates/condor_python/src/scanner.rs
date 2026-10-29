//! The per-file scan: vulture's `Vulture.scan` and its `ast.NodeVisitor`.
//!
//! Nodes are visited the way vulture visits them: children first, in CPython's
//! `_fields` order (a `FunctionDef` visits `args, body, decorator_list,
//! returns, type_params`), then reachability, then the node's own handler.
//! Reachability needs children before parents; following CPython's order also
//! keeps unreachable-code items in vulture's order.

use std::path::Path;
use std::sync::{Arc, LazyLock};

use condor_core::fnmatch::PatternSet;
use condor_core::paths::{format_path, resolve};
use condor_core::{ClassFacts, Decorator, FileScan, FunctionFacts, Item, ItemKind, ScanOptions, Stream, WhitelistEntry};
use regex::Regex;
use ruff_python_ast::{
    Comprehension, ExceptHandler, Expr, ExprContext, InterpolatedStringElement, InterpolatedStringElements, Operator,
    Parameter, Parameters, Pattern, Stmt, StmtIf, TypeParam, TypeParams,
};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_source_file::LineIndex;
use ruff_text_size::{Ranged, TextSize};
use rustc_hash::FxHashSet;

use crate::format_string;
use crate::literal;
use crate::noqa::NoqaLines;
use crate::rules::{self, TAG_CLASS_ATTR, TAG_FIELD};
use crate::type_comments;

const DEFAULT_CONFIDENCE: u8 = 60;
const IGNORED_VARIABLE_NAMES: [&str; 2] = ["object", "self"];
const PYTEST_FUNCTION_NAMES: [&str; 4] = ["setup_module", "teardown_module", "setup_function", "teardown_function"];
const PYTEST_METHOD_NAMES: [&str; 4] = ["setup_class", "teardown_class", "setup_method", "teardown_method"];

static TEST_FILE_PATTERNS: LazyLock<PatternSet> =
    LazyLock::new(|| PatternSet::new(&["*/test/*", "*/tests/*", "*/test*.py", "*[-_]test.py"]));
static PERCENT_FORMAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%\((\w+)\)").unwrap());
static BRACKETS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\w*\]").unwrap());
// A line written by `--make-whitelist`.
static WHITELIST_ENTRY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#\s*unused\s+\w+\s+\((.+):(\d+)\)\s*$").unwrap());


fn is_special_name(name: &str) -> bool {
    name.starts_with("__") && name.ends_with("__")
}

/// The display form of a path in messages (`utils.format_path`).
fn display(path: &Path, cwd: &Path) -> String {
    let p = format_path(path, cwd);
    if p.as_os_str().is_empty() { ".".to_string() } else { p.display().to_string() }
}

/// Scan one decoded source file (vulture's `Vulture.scan`).
pub fn scan(source: &str, path: Arc<Path>, opts: &ScanOptions) -> FileScan {
    let mut out = FileScan::default();
    let noqa = NoqaLines::parse(source);
    let shown = display(&path, &opts.cwd);

    if source.contains('\0') {
        out.log.push((Stream::Stderr, format!("{shown}: source code cannot contain null bytes")));
        out.invalid_input = true;
        return out;
    }

    let source = crate::read::universal_newlines(source.to_string());
    let options = ParseOptions::from(Mode::Module).with_target_version(ruff_python_ast::PythonVersion::PY314);
    let parsed = parse_unchecked(&source, options);
    let idx = LineIndex::from_source_text(&source);
    let Some(module) = parsed.syntax().as_module() else { unreachable!("parsed in module mode") };

    // The first syntax error. Its wording is ruff's (docs/divergences.md).
    let error = parsed
        .errors()
        .iter()
        .map(|e| (e.location.start(), e.error.to_string()))
        .chain(parsed.unsupported_syntax_errors().iter().map(|e| (e.range.start(), e.to_string())))
        .min_by_key(|(off, _)| *off);
    if let Some((offset, msg)) = error {
        let line = idx.line_index(offset).get();
        out.log.push((Stream::Stderr, format!("{shown}:{line}: {msg}")));
        out.invalid_input = true;
        return out;
    }

    let resolved = resolve(&path, &opts.cwd);
    out.file = Some(path.clone());
    let (aliases, modules) = rules::collect_imports(&module.body);
    out.imports = modules;
    let mut s = Scanner {
        source: &source,
        idx: &idx,
        file: path.clone(),
        opts,
        is_test_file: TEST_FILE_PATTERNS.matches_normcase(&resolved.to_string_lossy()),
        is_init: path.file_name().is_some_and(|n| n == "__init__.py"),
        noqa,
        no_fall_through: FxHashSet::default(),
        scopes: Vec::new(),
        class_body: None,
        pending_tag: None,
        skip_params: None,
        aliases,
        param_owner: None,
        out,
    };
    s.visit_module(&module.body);
    for name in type_comments::used_names(&source, parsed.tokens()) {
        s.use_name(&name);
    }
    s.out
}

fn addr<T>(node: &T) -> usize {
    node as *const T as usize
}

/// A statement as CPython's AST has it. An `elif` clause of ruff's `StmtIf` is
/// a nested `If` in the `orelse` of the previous clause.
#[derive(Clone, Copy)]
enum StmtRef<'a> {
    Stmt(&'a Stmt),
    Elif(&'a StmtIf, usize),
}

struct Scanner<'a> {
    source: &'a str,
    idx: &'a LineIndex,
    file: Arc<Path>,
    opts: &'a ScanOptions,
    is_test_file: bool,
    is_init: bool,
    noqa: NoqaLines,
    /// Statements that cannot fall through, by address.
    no_fall_through: FxHashSet<usize>,
    /// Functions and classes whose body is being visited, innermost last.
    scopes: Vec<Scope>,
    /// The class whose body statements are being visited directly.
    class_body: Option<ClassBody>,
    /// Tag and context for the next definition (class fields).
    pending_tag: Option<(u8, String)>,
    /// While visiting a function's parameters: why they are not definitions.
    skip_params: Option<SkipParams>,
    /// Local name -> dotted name it was imported as.
    aliases: rustc_hash::FxHashMap<String, String>,
    /// While visiting a function's parameters: its index in `out.functions`.
    param_owner: Option<usize>,
    out: FileScan,
}

struct Scope {
    name: String,
    is_class: bool,
    /// A function defined directly in a class body. Its name is not in scope
    /// inside its own body, so a bare name there refers to something else.
    is_method: bool,
    /// The body binds the scope's own name (`def data(): data = ...`), so a
    /// reference to it there is not a self-reference.
    shadowed: bool,
}

#[derive(Clone)]
struct ClassBody {
    name: String,
}

/// Why a function's parameters are not reported.
#[derive(Clone, Copy, PartialEq)]
enum SkipParams {
    /// The body only declares the signature (docstring, `...`,
    /// `raise NotImplementedError`).
    Stub,
}

#[derive(Clone, Copy)]
enum Ignore {
    None,
    Import,
    Class,
    Function,
    Method,
    Variable,
}

impl<'a> Scanner<'a> {
    fn line(&self, offset: TextSize) -> u32 {
        self.idx.line_index(offset).get() as u32
    }

    fn start_line(&self, node: &impl Ranged) -> u32 {
        self.line(node.start())
    }

    fn end_line(&self, node: &impl Ranged) -> u32 {
        self.line(node.end())
    }

    fn log(&mut self, msg: String) {
        if self.opts.verbose {
            self.out.log.push((Stream::Stdout, msg));
        }
    }

    fn use_name(&mut self, name: &str) {
        if self.opts.verbose {
            self.out.log.push((Stream::Stdout, format!("use name \"{name}\"")));
        }
        self.out.uses.push(name.to_string());
    }

    /// A reference to `name` by a name or attribute load. A function or class
    /// referring to itself from its own body (`f()` inside `def f`, `self.f`
    /// or `cls.f` inside method `f`, `C` inside `class C`) does not make it
    /// used.
    fn use_ref(&mut self, name: &str, receiver: Option<&Expr>) {
        let is_self_ref = match receiver {
            None => self.scopes.iter().any(|s| s.name == name && !s.shadowed && !s.is_method),
            Some(Expr::Name(n)) => {
                let r = n.id.as_str();
                (r == "self" || r == "cls" || self.scopes.iter().any(|s| s.is_class && s.name == r))
                    && self.scopes.iter().any(|s| s.is_method && !s.shadowed && s.name == name)
            }
            Some(_) => false,
        };
        if is_self_ref {
            self.log(format!("self reference \"{name}\""));
        } else {
            self.use_name(name);
        }
    }

    // ----- definitions ---------------------------------------------------

    fn ignored_by(&self, ignore: Ignore, name: &str) -> bool {
        let pytest = |names: &[&str]| (names.contains(&name) || name.starts_with("test_")) && self.is_test_file;
        match ignore {
            Ignore::None => false,
            Ignore::Import => self.is_init || name == "*",
            Ignore::Class => self.is_test_file && name.contains("Test"),
            Ignore::Function => pytest(&PYTEST_FUNCTION_NAMES),
            Ignore::Method => is_special_name(name) || pytest(&PYTEST_METHOD_NAMES),
            Ignore::Variable => {
                IGNORED_VARIABLE_NAMES.contains(&name)
                    || (name.starts_with('_') && !name.starts_with("__"))
                    || is_special_name(name)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn define(
        &mut self,
        kind: ItemKind,
        name: &str,
        first_line: u32,
        last_line: u32,
        message: Option<String>,
        confidence: u8,
        ignore: Ignore,
    ) -> Option<usize> {
        let ignored = self.ignored_by(ignore, name)
            || self.opts.ignore_names.matches_case(name)
            || self.noqa.ignores(first_line, kind.noqa_code());
        if ignored {
            self.log(format!("Ignoring {} \"{name}\"", kind.as_str()));
            self.pending_tag = None;
            return None;
        }
        self.log(format!("define {} \"{name}\"", kind.as_str()));
        let (tag, context) = self.pending_tag.take().unzip();
        self.out.items.push(Item {
            name: name.to_string(),
            kind,
            file: self.file.clone(),
            first_line,
            last_line,
            message,
            confidence,
            tag: tag.unwrap_or(0),
            context,
        });
        Some(self.out.items.len() - 1)
    }

    fn define_variable(&mut self, name: &str, node: &impl Ranged, confidence: u8) -> Option<usize> {
        let (first, last) = (self.start_line(node), self.end_line(node));
        self.define(ItemKind::Variable, name, first, last, None, confidence, Ignore::Variable)
    }

    fn report_unreachable(&mut self, name: &str, first_line: u32, last_line: u32, message: String) {
        self.define(ItemKind::UnreachableCode, name, first_line, last_line, Some(message), 100, Ignore::None);
    }

    /// A dotted name as written, resolved through the file's imports.
    fn resolve_name(&self, written: &str) -> String {
        let (first, rest) = written.split_once('.').map_or((written, None), |(f, r)| (f, Some(r)));
        match (self.aliases.get(first), rest) {
            (Some(full), Some(rest)) => format!("{full}.{rest}"),
            (Some(full), None) => full.clone(),
            (None, _) => written.to_string(),
        }
    }

    fn decorator_facts(&self, decorators: &[ruff_python_ast::Decorator]) -> Vec<Decorator> {
        decorators
            .iter()
            .map(|d| {
                let name = decorator_name(&d.expression);
                let mut keywords = Vec::new();
                if let Expr::Call(c) = &d.expression {
                    for k in &c.arguments.keywords {
                        let value = match &k.value {
                            Expr::BooleanLiteral(b) => b.value.to_string(),
                            Expr::StringLiteral(s) => s.value.to_str().to_string(),
                            _ => continue,
                        };
                        if let Some(arg) = &k.arg {
                            keywords.push((arg.to_string(), value));
                        }
                    }
                }
                Decorator { name: self.resolve_name(name.trim_start_matches('@')), keywords }
            })
            .collect()
    }

    // ----- statements as CPython sees them ---------------------------------

    /// `lines.get_first_line_number`: the first decorator, else the node.
    fn stmt_first_line(&self, s: StmtRef) -> u32 {
        match s {
            StmtRef::Stmt(stmt) => {
                let decorators = match stmt {
                    Stmt::FunctionDef(f) => f.decorator_list.first(),
                    Stmt::ClassDef(c) => c.decorator_list.first(),
                    _ => None,
                };
                match decorators {
                    Some(d) => self.start_line(&d.expression),
                    None => self.start_line(stmt),
                }
            }
            StmtRef::Elif(i, k) => self.start_line(&i.elif_else_clauses[k]),
        }
    }

    fn stmt_last_line(&self, s: StmtRef) -> u32 {
        match s {
            StmtRef::Stmt(stmt) => self.end_line(stmt),
            StmtRef::Elif(i, _) => self.end_line(i),
        }
    }

    fn stmt_id(s: StmtRef) -> usize {
        match s {
            StmtRef::Stmt(stmt) => addr(stmt),
            StmtRef::Elif(i, k) => addr(&i.elif_else_clauses[k]),
        }
    }

    /// `statement.__class__.__name__.lower()`.
    fn stmt_class_name(s: StmtRef) -> &'static str {
        let StmtRef::Stmt(stmt) = s else { return "if" };
        match stmt {
            Stmt::FunctionDef(f) if f.is_async => "asyncfunctiondef",
            Stmt::FunctionDef(_) => "functiondef",
            Stmt::ClassDef(_) => "classdef",
            Stmt::Return(_) => "return",
            Stmt::Delete(_) => "delete",
            Stmt::TypeAlias(_) => "typealias",
            Stmt::Assign(_) => "assign",
            Stmt::AugAssign(_) => "augassign",
            Stmt::AnnAssign(_) => "annassign",
            Stmt::For(f) if f.is_async => "asyncfor",
            Stmt::For(_) => "for",
            Stmt::While(_) => "while",
            Stmt::If(_) => "if",
            Stmt::With(w) if w.is_async => "asyncwith",
            Stmt::With(_) => "with",
            Stmt::Match(_) => "match",
            Stmt::Raise(_) => "raise",
            Stmt::Try(t) if t.is_star => "trystar",
            Stmt::Try(_) => "try",
            Stmt::Assert(_) => "assert",
            Stmt::Import(_) => "import",
            Stmt::ImportFrom(_) => "importfrom",
            Stmt::Global(_) => "global",
            Stmt::Nonlocal(_) => "nonlocal",
            Stmt::Expr(_) => "expr",
            Stmt::Pass(_) => "pass",
            Stmt::Break(_) => "break",
            Stmt::Continue(_) => "continue",
            Stmt::IpyEscapeCommand(_) => "ipyescapecommand",
        }
    }

    fn refs(body: &'a [Stmt]) -> Vec<StmtRef<'a>> {
        body.iter().map(StmtRef::Stmt).collect()
    }

    /// The `orelse` of the `If` for clause `k` of `i` (`k == 0` is the `if`).
    fn orelse_refs(i: &'a StmtIf, k: usize) -> Vec<StmtRef<'a>> {
        match i.elif_else_clauses.get(k) {
            None => Vec::new(),
            Some(clause) if clause.test.is_some() => vec![StmtRef::Elif(i, k)],
            Some(clause) => Self::refs(&clause.body),
        }
    }

    // ----- reachability (vulture reachability.py) -------------------------

    fn can_fall_through(&self, s: StmtRef) -> bool {
        !self.no_fall_through.contains(&Self::stmt_id(s))
    }

    fn mark_no_fall_through(&mut self, s: StmtRef) {
        self.no_fall_through.insert(Self::stmt_id(s));
    }

    fn statements_can_fall_through(&mut self, stmts: &[StmtRef]) -> bool {
        for (i, &stmt) in stmts.iter().enumerate() {
            if !self.can_fall_through(stmt) {
                let rest = &stmts[i + 1..];
                // `raise NotImplementedError` followed by a bare `yield` is how
                // an abstract method is made a generator.
                if !rest.is_empty() && rest.iter().all(|s| is_bare_yield(*s)) {
                    return false;
                }
                if let Some(&next) = stmts.get(i + 1) {
                    let class_name = Self::stmt_class_name(stmt);
                    let first = self.stmt_first_line(next);
                    let last = self.stmt_last_line(*stmts.last().unwrap());
                    self.report_unreachable(class_name, first, last, format!("unreachable code after '{class_name}'"));
                }
                return false;
            }
        }
        true
    }

    fn else_can_fall_through(&mut self, orelse: &[StmtRef], condition_always_true: bool) -> bool {
        if orelse.is_empty() {
            return !condition_always_true;
        }
        self.statements_can_fall_through(orelse)
    }

    fn reach_if(&mut self, node: StmtRef<'a>, test: &Expr, body: &'a [Stmt], orelse: &[StmtRef<'a>]) {
        let body_refs = Self::refs(body);
        let (if_can, else_can);
        if literal::always_false(test) {
            let first = self.stmt_first_line(node);
            let last = self.end_line(body.last().unwrap());
            self.report_unreachable("if", first, last, "unsatisfiable 'if' condition".into());
            if_can = true;
            else_can = self.else_can_fall_through(orelse, false);
        } else if literal::always_true(test) {
            if_can = self.statements_can_fall_through(&body_refs);
            else_can = self.else_can_fall_through(orelse, true);
            if let (Some(&first), Some(&last)) = (orelse.first(), orelse.last()) {
                let (f, l) = (self.stmt_first_line(first), self.stmt_last_line(last));
                self.report_unreachable("else", f, l, "unreachable 'else' block".into());
            } else {
                let (f, l) = (self.stmt_first_line(node), self.stmt_last_line(node));
                self.report_unreachable("if", f, l, "redundant if-condition".into());
            }
        } else {
            if_can = self.statements_can_fall_through(&body_refs);
            else_can = self.else_can_fall_through(orelse, false);
        }
        if !(if_can || else_can) {
            self.mark_no_fall_through(node);
        }
    }

    fn reach_while(&mut self, stmt: &'a Stmt, test: &Expr, body: &'a [Stmt], orelse: &'a [Stmt]) {
        if literal::always_false(test) {
            let first = self.start_line(stmt);
            let last = self.end_line(body.last().unwrap());
            self.report_unreachable("while", first, last, "unsatisfiable 'while' condition".into());
        } else if literal::always_true(test) {
            if let (Some(first), Some(last)) = (orelse.first(), orelse.last()) {
                let (f, l) = (self.stmt_first_line(StmtRef::Stmt(first)), self.end_line(last));
                self.report_unreachable("else", f, l, "unreachable 'else' block".into());
            }
            if !body_has_break(body) {
                self.mark_no_fall_through(StmtRef::Stmt(stmt));
            }
        }
        self.statements_can_fall_through(&Self::refs(body));
    }

    fn reach_try(&mut self, stmt: &'a Stmt, body: &'a [Stmt], handlers: &'a [ExceptHandler], orelse: &'a [Stmt]) {
        let try_can = self.statements_can_fall_through(&Self::refs(body));
        if !try_can {
            if let (Some(first), Some(last)) = (orelse.first(), orelse.last()) {
                let (f, l) = (self.stmt_first_line(StmtRef::Stmt(first)), self.end_line(last));
                self.report_unreachable("else", f, l, "unreachable 'else' block".into());
            }
        }
        // `any()` stops at the first handler that can fall through.
        let mut any_except = false;
        for ExceptHandler::ExceptHandler(h) in handlers {
            if self.statements_can_fall_through(&Self::refs(&h.body)) {
                any_except = true;
                break;
            }
        }
        if !(try_can || any_except) {
            self.mark_no_fall_through(StmtRef::Stmt(stmt));
        }
    }

    // ----- visiting --------------------------------------------------------

    fn visit_module(&mut self, body: &'a [Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
        self.statements_can_fall_through(&Self::refs(body));
    }

    fn visit_body(&mut self, body: &'a [Stmt]) {
        body.iter().for_each(|s| self.visit_stmt(s))
    }

    fn visit_opt(&mut self, e: Option<&'a Expr>) {
        match e {
            Some(e) => self.visit_expr(e),
            None => {}
        }
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let me = StmtRef::Stmt(stmt);
        match stmt {
            Stmt::FunctionDef(f) => {
                let facts = FunctionFacts {
                    name: f.name.to_string(),
                    class: self.class_body.as_ref().map(|c| c.name.clone()),
                    decorators: self.decorator_facts(&f.decorator_list),
                    item: None,
                    params: Vec::new(),
                };
                self.out.functions.push(facts);
                let index = self.out.functions.len() - 1;
                self.skip_params = is_stub_body(&f.body).then_some(SkipParams::Stub);
                self.param_owner = Some(index);
                self.visit_parameters(&f.parameters);
                self.param_owner = None;
                self.skip_params = None;
                let class_body = self.class_body.take();
                let shadowed = rules::binds_name(&f.body, Some(&f.parameters), f.name.as_str());
                let is_method = class_body.is_some();
                self.scopes.push(Scope { name: f.name.to_string(), is_class: false, is_method, shadowed });
                self.visit_body(&f.body);
                self.scopes.pop();
                self.class_body = class_body;
                for d in &f.decorator_list {
                    self.visit_expr(&d.expression);
                }
                self.visit_opt(f.returns.as_deref());
                self.visit_type_params(f.type_params.as_deref());
                self.statements_can_fall_through(&Self::refs(&f.body));
                self.out.functions[index].item = self.handle_function_def(stmt, f);
            }
            Stmt::ClassDef(c) => {
                if let Some(args) = &c.arguments {
                    for a in &args.args {
                        self.visit_expr(a);
                    }
                    for k in &args.keywords {
                        self.visit_expr(&k.value);
                    }
                }
                let bases: Vec<String> = c
                    .arguments
                    .iter()
                    .flat_map(|a| a.args.iter())
                    .filter_map(dotted_name)
                    .map(|b| self.resolve_name(&b))
                    .collect();
                let outer = self.class_body.replace(ClassBody { name: c.name.to_string() });
                let shadowed = rules::binds_name(&c.body, None, c.name.as_str());
                self.scopes.push(Scope { name: c.name.to_string(), is_class: true, is_method: false, shadowed });
                self.visit_body(&c.body);
                self.scopes.pop();
                self.class_body = outer;
                for d in &c.decorator_list {
                    self.visit_expr(&d.expression);
                }
                self.visit_type_params(c.type_params.as_deref());
                let decorators = self.decorator_facts(&c.decorator_list);
                let item = self.handle_class_def(stmt, c);
                let class = self.class_body.as_ref().map(|b| b.name.clone());
                self.out.classes.push(ClassFacts { name: c.name.to_string(), bases, decorators, item, class });
            }
            Stmt::Return(r) => {
                self.visit_opt(r.value.as_deref());
                self.mark_no_fall_through(me);
            }
            Stmt::Delete(d) => d.targets.iter().for_each(|t| self.visit_expr(t)),
            Stmt::Assign(a) => {
                for t in &a.targets {
                    self.tag_class_attribute(t, TAG_CLASS_ATTR);
                    self.visit_expr(t);
                    self.pending_tag = None;
                }
                self.visit_expr(&a.value);
                self.handle_assign(a);
            }
            Stmt::TypeAlias(t) => {
                self.visit_expr(&t.name);
                self.visit_type_params(t.type_params.as_deref());
                self.visit_expr(&t.value);
            }
            Stmt::AugAssign(a) => {
                self.visit_expr(&a.target);
                self.visit_expr(&a.value);
            }
            Stmt::AnnAssign(a) => {
                self.tag_class_attribute(&a.target, TAG_FIELD);
                self.visit_expr(&a.target);
                self.pending_tag = None;
                self.visit_expr(&a.annotation);
                self.visit_opt(a.value.as_deref());
            }
            Stmt::For(f) => {
                self.visit_expr(&f.target);
                self.visit_expr(&f.iter);
                self.visit_body(&f.body);
                self.visit_body(&f.orelse);
                self.statements_can_fall_through(&Self::refs(&f.body));
            }
            Stmt::While(w) => {
                self.visit_expr(&w.test);
                self.visit_body(&w.body);
                self.visit_body(&w.orelse);
                self.reach_while(stmt, &w.test, &w.body, &w.orelse);
            }
            Stmt::If(i) => {
                self.visit_expr(&i.test);
                self.visit_body(&i.body);
                self.visit_elif_chain(i, 0);
                let orelse = Self::orelse_refs(i, 0);
                self.reach_if(me, &i.test, &i.body, &orelse);
            }
            Stmt::With(w) => {
                for item in &w.items {
                    self.visit_expr(&item.context_expr);
                    self.visit_opt(item.optional_vars.as_deref());
                }
                self.visit_body(&w.body);
                self.statements_can_fall_through(&Self::refs(&w.body));
            }
            Stmt::Match(m) => {
                self.visit_expr(&m.subject);
                for case in &m.cases {
                    self.visit_pattern(&case.pattern);
                    self.visit_opt(case.guard.as_deref());
                    self.visit_body(&case.body);
                }
            }
            Stmt::Raise(r) => {
                self.visit_opt(r.exc.as_deref());
                self.visit_opt(r.cause.as_deref());
                self.mark_no_fall_through(me);
            }
            Stmt::Try(t) => {
                self.visit_body(&t.body);
                for ExceptHandler::ExceptHandler(h) in &t.handlers {
                    self.visit_opt(h.type_.as_deref());
                    self.visit_body(&h.body);
                }
                self.visit_body(&t.orelse);
                self.visit_body(&t.finalbody);
                // vulture does not handle `TryStar`.
                if !t.is_star {
                    self.reach_try(stmt, &t.body, &t.handlers, &t.orelse);
                }
            }
            Stmt::Assert(a) => {
                self.visit_expr(&a.test);
                self.visit_opt(a.msg.as_deref());
            }
            Stmt::Import(i) => {
                let first = self.start_line(stmt);
                let last = self.end_line(stmt);
                for alias in &i.names {
                    self.add_alias(alias.name.as_str(), alias.asname.as_ref().map(|a| a.as_str()), first, last);
                }
            }
            Stmt::ImportFrom(i) => {
                if i.module.as_ref().is_none_or(|m| m.as_str() != "__future__") {
                    let first = self.start_line(stmt);
                    let last = self.end_line(stmt);
                    for alias in &i.names {
                        self.add_alias(alias.name.as_str(), alias.asname.as_ref().map(|a| a.as_str()), first, last);
                    }
                }
            }
            Stmt::Expr(e) => {
                self.visit_expr(&e.value);
                self.record_whitelist_entry(stmt, &e.value);
            }
            Stmt::Break(_) | Stmt::Continue(_) => self.mark_no_fall_through(me),
            Stmt::Global(_) | Stmt::Nonlocal(_) | Stmt::Pass(_) | Stmt::IpyEscapeCommand(_) => {}
        }
    }

    /// The `orelse` of clause `k - 1`: a nested `If` for an `elif`, or the
    /// `else` body.
    fn visit_elif_chain(&mut self, i: &'a StmtIf, k: usize) {
        let Some(clause) = i.elif_else_clauses.get(k) else { return };
        match &clause.test {
            Some(test) => {
                self.visit_expr(test);
                self.visit_body(&clause.body);
                self.visit_elif_chain(i, k + 1);
                let orelse = Self::orelse_refs(i, k + 1);
                self.reach_if(StmtRef::Elif(i, k), test, &clause.body, &orelse);
            }
            None => self.visit_body(&clause.body),
        }
    }

    fn visit_type_params(&mut self, params: Option<&'a TypeParams>) {
        let Some(params) = params else { return };
        for p in &params.type_params {
            match p {
                TypeParam::TypeVar(t) => {
                    self.visit_opt(t.bound.as_deref());
                    self.visit_opt(t.default.as_deref());
                }
                TypeParam::ParamSpec(t) => self.visit_opt(t.default.as_deref()),
                TypeParam::TypeVarTuple(t) => self.visit_opt(t.default.as_deref()),
            }
        }
    }

    /// CPython `arguments` order: `posonlyargs, args, vararg, kwonlyargs,
    /// kw_defaults, kwarg, defaults`.
    fn visit_parameters(&mut self, p: &'a Parameters) {
        for a in p.posonlyargs.iter().chain(&p.args) {
            self.visit_arg(&a.parameter);
        }
        if let Some(v) = &p.vararg {
            self.visit_arg(v);
        }
        for a in &p.kwonlyargs {
            self.visit_arg(&a.parameter);
        }
        for a in &p.kwonlyargs {
            self.visit_opt(a.default.as_deref());
        }
        if let Some(k) = &p.kwarg {
            self.visit_arg(k);
        }
        for a in p.posonlyargs.iter().chain(&p.args) {
            self.visit_opt(a.default.as_deref());
        }
    }

    fn visit_arg(&mut self, p: &'a Parameter) {
        self.visit_opt(p.annotation.as_deref());
        let item = match self.skip_params {
            Some(SkipParams::Stub) => {
                self.log(format!("Ignoring variable \"{}\" (stub parameter)", p.name));
                None
            }
            // `visit_arg`: function arguments are variables with 100% confidence.
            None => self.define_variable(p.name.as_str(), p, 100),
        };
        if let Some(owner) = self.param_owner {
            self.out.functions[owner].params.push((p.name.to_string(), item));
        }
    }

    fn visit_comprehensions(&mut self, generators: &'a [Comprehension]) {
        for g in generators {
            self.visit_expr(&g.target);
            self.visit_expr(&g.iter);
            for cond in &g.ifs {
                self.visit_expr(cond);
            }
        }
    }

    fn visit_interpolations(&mut self, elements: &'a InterpolatedStringElements) {
        for el in elements.iter() {
            if let InterpolatedStringElement::Interpolation(i) = el {
                self.visit_expr(&i.expression);
                if let Some(spec) = &i.format_spec {
                    self.visit_interpolations(&spec.elements);
                }
            }
        }
    }

    fn visit_pattern(&mut self, p: &'a Pattern) {
        match p {
            Pattern::MatchValue(v) => self.visit_expr(&v.value),
            Pattern::MatchSingleton(_) | Pattern::MatchStar(_) => {}
            Pattern::MatchSequence(s) => s.patterns.iter().for_each(|p| self.visit_pattern(p)),
            Pattern::MatchMapping(m) => {
                for k in &m.keys {
                    self.visit_expr(k);
                }
                for p in &m.patterns {
                    self.visit_pattern(p);
                }
            }
            Pattern::MatchClass(c) => {
                self.visit_expr(&c.cls);
                for p in &c.arguments.patterns {
                    self.visit_pattern(p);
                }
                for k in &c.arguments.keywords {
                    self.visit_pattern(&k.pattern);
                }
                // `visit_MatchClass`.
                for k in &c.arguments.keywords {
                    self.use_name(k.attr.as_str());
                }
            }
            Pattern::MatchAs(a) => {
                if let Some(p) = &a.pattern {
                    self.visit_pattern(p);
                }
            }
            Pattern::MatchOr(o) => o.patterns.iter().for_each(|p| self.visit_pattern(p)),
        }
    }

    fn visit_expr(&mut self, e: &'a Expr) {
        match e {
            Expr::BoolOp(b) => b.values.iter().for_each(|v| self.visit_expr(v)),
            Expr::Named(n) => {
                self.visit_expr(&n.target);
                self.visit_expr(&n.value);
            }
            Expr::BinOp(b) => {
                self.visit_expr(&b.left);
                self.visit_expr(&b.right);
                self.handle_bin_op(b);
            }
            Expr::UnaryOp(u) => self.visit_expr(&u.operand),
            Expr::Lambda(l) => {
                if let Some(p) = &l.parameters {
                    let (skip, owner) = (self.skip_params.take(), self.param_owner.take());
                    self.visit_parameters(p);
                    (self.skip_params, self.param_owner) = (skip, owner);
                }
                self.visit_expr(&l.body);
            }
            Expr::If(i) => {
                self.visit_expr(&i.test);
                self.visit_expr(&i.body);
                self.visit_expr(&i.orelse);
                if literal::always_false(&i.test) {
                    let (f, l) = (self.start_line(e), self.end_line(&*i.body));
                    self.report_unreachable("ternary", f, l, "unsatisfiable 'ternary' condition".into());
                } else if literal::always_true(&i.test) {
                    let (f, l) = (self.start_line(&*i.orelse), self.end_line(&*i.orelse));
                    self.report_unreachable("ternary", f, l, "unreachable 'else' expression".into());
                }
            }
            Expr::Dict(d) => {
                for item in &d.items {
                    self.visit_opt(item.key.as_ref());
                }
                for item in &d.items {
                    self.visit_expr(&item.value);
                }
            }
            Expr::Set(s) => s.elts.iter().for_each(|x| self.visit_expr(x)),
            Expr::ListComp(c) => {
                self.visit_expr(&c.elt);
                self.visit_comprehensions(&c.generators);
            }
            Expr::SetComp(c) => {
                self.visit_expr(&c.elt);
                self.visit_comprehensions(&c.generators);
            }
            Expr::Generator(c) => {
                self.visit_expr(&c.elt);
                self.visit_comprehensions(&c.generators);
            }
            Expr::DictComp(c) => {
                self.visit_opt(c.key.as_deref());
                self.visit_expr(&c.value);
                self.visit_comprehensions(&c.generators);
            }
            Expr::Await(a) => self.visit_expr(&a.value),
            Expr::Yield(y) => self.visit_opt(y.value.as_deref()),
            Expr::YieldFrom(y) => self.visit_expr(&y.value),
            Expr::Compare(c) => {
                // `operands` is `left` followed by the comparators.
                c.operands.iter().for_each(|x| self.visit_expr(x));
            }
            Expr::Call(c) => {
                self.visit_expr(&c.func);
                for a in &c.arguments.args {
                    self.visit_expr(a);
                }
                for k in &c.arguments.keywords {
                    self.visit_expr(&k.value);
                }
                self.handle_call(c);
            }
            Expr::FString(f) => {
                for el in f.value.elements() {
                    if let InterpolatedStringElement::Interpolation(i) = el {
                        self.visit_expr(&i.expression);
                        if let Some(spec) = &i.format_spec {
                            self.visit_interpolations(&spec.elements);
                        }
                    }
                }
            }
            Expr::TString(t) => {
                for ts in t.value.iter() {
                    self.visit_interpolations(&ts.elements);
                }
            }
            Expr::StringLiteral(_)
            | Expr::BytesLiteral(_)
            | Expr::NumberLiteral(_)
            | Expr::BooleanLiteral(_)
            | Expr::NoneLiteral(_)
            | Expr::EllipsisLiteral(_)
            | Expr::IpyEscapeCommand(_) => {}
            Expr::Attribute(a) => {
                self.visit_expr(&a.value);
                match a.ctx {
                    ExprContext::Store => {
                        let (f, l) = (self.start_line(e), self.end_line(e));
                        self.define(ItemKind::Attribute, a.attr.as_str(), f, l, None, DEFAULT_CONFIDENCE, Ignore::None);
                    }
                    ExprContext::Load => {
                        let receiver = if is_super_call(&a.value) { None } else { Some(&*a.value) };
                        match receiver {
                            None => self.use_name(a.attr.as_str()),
                            Some(r) => self.use_ref(a.attr.as_str(), Some(r)),
                        }
                    }
                    _ => {}
                }
            }
            Expr::Subscript(s) => {
                self.visit_expr(&s.value);
                self.visit_expr(&s.slice);
            }
            Expr::Starred(s) => self.visit_expr(&s.value),
            Expr::Name(n) => match n.ctx {
                ExprContext::Load | ExprContext::Del => {
                    if !IGNORED_VARIABLE_NAMES.contains(&n.id.as_str()) {
                        self.use_ref(n.id.as_str(), None);
                    }
                }
                ExprContext::Store => {
                    self.define_variable(n.id.as_str(), e, DEFAULT_CONFIDENCE);
                }
                ExprContext::Invalid => {}
            },
            Expr::List(l) => l.elts.iter().for_each(|x| self.visit_expr(x)),
            Expr::Tuple(t) => t.elts.iter().for_each(|x| self.visit_expr(x)),
            Expr::Slice(s) => {
                self.visit_opt(s.lower.as_deref());
                self.visit_opt(s.upper.as_deref());
                self.visit_opt(s.step.as_deref());
            }
        }
    }

    // ----- node handlers (vulture visit_* methods) -------------------------

    fn add_alias(&mut self, full_name: &str, asname: Option<&str>, first: u32, last: u32) {
        // Only the top-level module name is stored ("os.path" -> "os").
        let name = full_name.split('.').next().unwrap_or(full_name);
        self.define(ItemKind::Import, asname.unwrap_or(name), first, last, None, 90, Ignore::Import);
        if asname.is_some() {
            self.use_name(full_name);
        }
    }

    fn decorator_names(&self, decorators: &[ruff_python_ast::Decorator]) -> Vec<String> {
        decorators.iter().map(|d| decorator_name(&d.expression)).collect()
    }

    fn handle_class_def(&mut self, stmt: &Stmt, c: &ruff_python_ast::StmtClassDef) -> Option<usize> {
        let names = self.decorator_names(&c.decorator_list);
        if names.iter().any(|n| self.opts.ignore_decorators.matches_case(n)) {
            self.log(format!("Ignoring class \"{}\" (decorator whitelisted)", c.name));
            return None;
        }
        let (first, last) = (self.stmt_first_line(StmtRef::Stmt(stmt)), self.end_line(stmt));
        self.define(ItemKind::Class, c.name.as_str(), first, last, None, DEFAULT_CONFIDENCE, Ignore::Class)
    }

    fn handle_function_def(&mut self, stmt: &Stmt, f: &ruff_python_ast::StmtFunctionDef) -> Option<usize> {
        let names = self.decorator_names(&f.decorator_list);
        let first_arg = f.parameters.args.first().map(|a| a.parameter.name.as_str());
        let has = |d: &str| names.iter().any(|n| n == d);
        let kind = if has("@property") {
            ItemKind::Property
        } else if has("@staticmethod") || has("@classmethod") || first_arg == Some("self") {
            ItemKind::Method
        } else {
            ItemKind::Function
        };
        if names.iter().any(|n| self.opts.ignore_decorators.matches_case(n)) {
            self.log(format!("Ignoring {} \"{}\" (decorator whitelisted)", kind.as_str(), f.name));
            return None;
        }
        let ignore = match kind {
            ItemKind::Method => Ignore::Method,
            ItemKind::Function => Ignore::Function,
            _ => Ignore::None,
        };
        let (first, last) = (self.stmt_first_line(StmtRef::Stmt(stmt)), self.end_line(stmt));
        self.define(kind, f.name.as_str(), first, last, None, DEFAULT_CONFIDENCE, ignore)
    }

    fn handle_assign(&mut self, a: &ruff_python_ast::StmtAssign) {
        let assigns_all = a.targets.iter().any(|t| matches!(t, Expr::Name(n) if n.id.as_str() == "__all__"));
        let elts = match &*a.value {
            Expr::List(l) => &l.elts,
            Expr::Tuple(t) => &t.elts,
            _ => return,
        };
        if assigns_all {
            for elt in elts {
                if let Expr::StringLiteral(s) = elt {
                    self.use_name(s.value.to_str());
                }
            }
        }
    }

    /// `"%(my_var)s" % locals()`. vulture adds these with `|=`, which bypasses
    /// its verbose logging.
    fn handle_bin_op(&mut self, b: &ruff_python_ast::ExprBinOp) {
        if let Expr::StringLiteral(s) = &*b.left {
            if b.op == Operator::Mod && is_locals_call(&b.right) {
                for caps in PERCENT_FORMAT.captures_iter(s.value.to_str()) {
                    self.out.uses.push(caps[1].to_string());
                }
            }
        }
    }

    fn handle_call(&mut self, c: &ruff_python_ast::ExprCall) {
        let nargs = c.arguments.args.len();
        // Calls that name things a framework provides by name, such as
        // `@pytest.mark.usefixtures("db")` (from plugins).
        if !self.opts.call_hints.is_empty() {
            if let Some(callee) = dotted_name(&c.func) {
                if self.opts.call_hints.matches_case(&self.resolve_name(&callee)) {
                    for arg in &c.arguments.args {
                        if let Expr::StringLiteral(s) = arg {
                            self.out.requested.push(s.value.to_str().to_string());
                        }
                    }
                }
            }
        }
        if let Expr::Name(n) = &*c.func {
            let id = n.id.as_str();
            if (id == "getattr" && (2..=3).contains(&nargs)) || (id == "hasattr" && nargs == 2) {
                if let Expr::StringLiteral(s) = &c.arguments.args[1] {
                    self.use_name(s.value.to_str());
                }
            }
        }
        // `"{my_var}".format(**locals())`.
        if let Expr::Attribute(a) = &*c.func {
            if let Expr::StringLiteral(s) = &*a.value {
                let has_locals = c.arguments.keywords.iter().any(|k| k.arg.is_none() && is_locals_call(&k.value));
                if a.attr.as_str() == "format" && has_locals {
                    self.handle_new_format_string(s.value.to_str());
                }
            }
        }
    }

    /// Tag a name assigned directly in a class body, for the declarative
    /// class rules applied once all files are scanned.
    fn tag_class_attribute(&mut self, target: &Expr, tag: u8) {
        if let (Expr::Name(_), Some(class)) = (target, &self.class_body) {
            self.pending_tag = Some((tag, class.name.clone()));
        }
    }

    /// `name  # unused function (path:line)` or `_.name  # unused method (...)`.
    fn record_whitelist_entry(&mut self, stmt: &Stmt, value: &Expr) {
        let name = match value {
            Expr::Name(n) => n.id.as_str(),
            Expr::Attribute(a) if matches!(&*a.value, Expr::Name(n) if n.id.as_str() == "_") => a.attr.as_str(),
            _ => return,
        };
        let end = stmt.end().to_usize();
        let line_end = self.source[end..].find('\n').map_or(self.source.len(), |p| end + p);
        let Some(caps) = WHITELIST_ENTRY.captures(&self.source[end..line_end]) else { return };
        let entry = WhitelistEntry { name: name.to_string(), path: caps[1].to_string(), line: self.start_line(stmt) };
        self.out.whitelist_entries.push(entry);
    }

    fn handle_new_format_string(&mut self, s: &str) {
        let names = format_string::field_names(s).unwrap_or_default();
        for field_name in names {
            // "a[0][b].c[d].e" -> "a.c.e" -> ["a", "c", "e"]
            let stripped = BRACKETS.replace_all(&field_name, "");
            for var in stripped.split('.') {
                // `re.match(r"[a-zA-Z_][a-zA-Z0-9_]*", name)`: only the first
                // character matters.
                if var.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') {
                    self.use_name(var);
                }
            }
        }
    }
}

fn is_super_call(e: &Expr) -> bool {
    matches!(e, Expr::Call(c) if matches!(&*c.func, Expr::Name(n) if n.id.as_str() == "super"))
}

/// A body that only declares a signature: a docstring, `...` or
/// `raise NotImplementedError`. A `pass` body is not one: it is as often a
/// placeholder as an interface.
fn is_stub_body(body: &[Stmt]) -> bool {
    body.iter().all(|s| match s {
        Stmt::Expr(e) => matches!(&*e.value, Expr::StringLiteral(_) | Expr::EllipsisLiteral(_)),
        Stmt::Raise(r) => r.exc.as_deref().is_some_and(|e| {
            let e = match e {
                Expr::Call(c) => &*c.func,
                other => other,
            };
            matches!(e, Expr::Name(n) if n.id.as_str() == "NotImplementedError")
        }),
        _ => false,
    })
}

fn is_bare_yield(s: StmtRef) -> bool {
    matches!(s, StmtRef::Stmt(Stmt::Expr(e)) if matches!(&*e.value, Expr::Yield(y) if y.value.is_none()))
}

/// The dotted name of a base class (`Protocol`, `typing.Protocol`,
/// `Protocol[T]`).
fn dotted_name(e: &Expr) -> Option<String> {
    match e {
        Expr::Name(n) => Some(n.id.to_string()),
        Expr::Attribute(a) => dotted_name(&a.value).map(|v| format!("{v}.{}", a.attr)),
        Expr::Subscript(s) => dotted_name(&s.value),
        _ => None,
    }
}

fn is_locals_call(e: &Expr) -> bool {
    matches!(e, Expr::Call(c)
        if matches!(&*c.func, Expr::Name(n) if n.id.as_str() == "locals")
            && c.arguments.args.is_empty()
            && c.arguments.keywords.is_empty())
}

/// `utils.get_decorator_name`.
pub fn decorator_name(e: &Expr) -> String {
    let mut node = match e {
        Expr::Call(c) => &*c.func,
        other => other,
    };
    let mut parts = Vec::new();
    loop {
        match node {
            Expr::Attribute(a) => {
                parts.push(a.attr.as_str());
                node = &a.value;
            }
            Expr::Name(n) => {
                parts.push(n.id.as_str());
                break;
            }
            _ => {
                parts.clear();
                break;
            }
        }
    }
    parts.reverse();
    format!("@{}", parts.join("."))
}

/// `Reachability._body_has_break`.
fn body_has_break(stmts: &[Stmt]) -> bool {
    stmts.iter().any(stmt_has_break)
}

fn stmt_has_break(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Break(_) => true,
        // New loops and scopes have their own breaks.
        Stmt::While(_) | Stmt::For(_) | Stmt::FunctionDef(_) | Stmt::ClassDef(_) => false,
        // Every list field that holds AST nodes is searched: statement bodies,
        // but also handlers and match cases, whose bodies are lists too.
        Stmt::If(i) => {
            body_has_break(&i.body)
                || i.elif_else_clauses.first().is_some_and(|_| elif_has_break(i, 0))
        }
        Stmt::With(w) => body_has_break(&w.body),
        Stmt::Try(t) => {
            body_has_break(&t.body)
                || t.handlers.iter().any(|ExceptHandler::ExceptHandler(h)| body_has_break(&h.body))
                || body_has_break(&t.orelse)
                || body_has_break(&t.finalbody)
        }
        Stmt::Match(m) => m.cases.iter().any(|c| body_has_break(&c.body)),
        _ => false,
    }
}

fn elif_has_break(i: &StmtIf, k: usize) -> bool {
    let Some(clause) = i.elif_else_clauses.get(k) else { return false };
    body_has_break(&clause.body) || (clause.test.is_some() && elif_has_break(i, k + 1))
}

pub use ruff_python_ast::Expr as PyExpr;

/// Parse a standalone expression (used by the test bridge).
pub fn parse_expression(src: &str) -> Option<Expr> {
    ruff_python_parser::parse_expression(src).ok().map(|p| *p.into_syntax().body)
}
