//! Language-level helpers for condor's rules beyond vulture's. Knowledge of
//! specific libraries lives in plugins (`plugins.rs`).

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, ExprContext, Parameters, Stmt};
use rustc_hash::FxHashMap;

/// An attribute annotated in a class body (`name: int`).
pub const TAG_FIELD: u8 = 1;
/// An attribute assigned in a class body (`NAME = 1`).
pub const TAG_CLASS_ATTR: u8 = 2;
/// An attribute assigned a call in a class body (`name = models.CharField()`).
pub const TAG_CLASS_ATTR_CALL: u8 = 3;

/// Whether `body` (or `params`) binds `name` in the scope itself (not in
/// nested functions or classes).
pub fn binds_name(body: &[Stmt], params: Option<&Parameters>, name: &str) -> bool {
    if params.is_some_and(|p| p.iter().any(|a| a.name().as_str() == name)) {
        return true;
    }
    struct Binds<'n> {
        name: &'n str,
        found: bool,
    }
    impl<'b> Visitor<'b> for Binds<'_> {
        fn visit_stmt(&mut self, stmt: &'b Stmt) {
            match stmt {
                Stmt::FunctionDef(f) => self.found |= f.name.as_str() == self.name,
                Stmt::ClassDef(c) => self.found |= c.name.as_str() == self.name,
                Stmt::Import(i) => {
                    self.found |= i.names.iter().any(|a| {
                        a.asname.as_ref().map_or(a.name.split('.').next() == Some(self.name), |n| n.as_str() == self.name)
                    })
                }
                Stmt::ImportFrom(i) => {
                    self.found |= i
                        .names
                        .iter()
                        .any(|a| a.asname.as_ref().unwrap_or(&a.name).as_str() == self.name)
                }
                _ => visitor::walk_stmt(self, stmt),
            }
        }
        fn visit_expr(&mut self, e: &'b Expr) {
            if let Expr::Name(n) = e {
                self.found |= n.ctx == ExprContext::Store && n.id.as_str() == self.name;
            }
            visitor::walk_expr(self, e);
        }
    }
    let mut v = Binds { name, found: false };
    v.visit_body(body);
    v.found
}

/// The file's imports: local name -> dotted name it refers to, the
/// top-level names of the imported modules, and whether there is a star
/// import. Relative imports are not resolved.
pub fn collect_imports(body: &[Stmt]) -> (FxHashMap<String, String>, Vec<String>, bool) {
    struct Imports {
        aliases: FxHashMap<String, String>,
        modules: Vec<String>,
        star: bool,
    }
    impl<'b> Visitor<'b> for Imports {
        fn visit_stmt(&mut self, stmt: &'b Stmt) {
            match stmt {
                Stmt::Import(i) => {
                    for a in &i.names {
                        let full = a.name.as_str();
                        let top = full.split('.').next().unwrap_or(full);
                        self.modules.push(top.to_string());
                        match &a.asname {
                            Some(alias) => self.aliases.insert(alias.to_string(), full.to_string()),
                            None => self.aliases.insert(top.to_string(), top.to_string()),
                        };
                    }
                }
                // Relative imports keep their dots (`.models.Base`), so they
                // can be told apart from top-level modules and resolved to a
                // project file.
                Stmt::ImportFrom(i) if i.level > 0 => {
                    let dots = ".".repeat(i.level as usize);
                    let prefix = match &i.module {
                        Some(module) => format!("{dots}{module}."),
                        None => dots,
                    };
                    for a in &i.names {
                        if a.name.as_str() == "*" {
                            self.star = true;
                            continue;
                        }
                        let local = a.asname.as_ref().unwrap_or(&a.name);
                        self.aliases.insert(local.to_string(), format!("{prefix}{}", a.name));
                    }
                }
                Stmt::ImportFrom(i) => {
                    let Some(module) = &i.module else { return };
                    self.modules.push(module.split('.').next().unwrap_or(module).to_string());
                    for a in &i.names {
                        if a.name.as_str() == "*" {
                            self.star = true;
                            continue;
                        }
                        let local = a.asname.as_ref().unwrap_or(&a.name);
                        self.aliases.insert(local.to_string(), format!("{module}.{}", a.name));
                    }
                }
                _ => visitor::walk_stmt(self, stmt),
            }
        }
    }
    let mut v = Imports { aliases: FxHashMap::default(), modules: Vec::new(), star: false };
    v.visit_body(body);
    v.modules.sort_unstable();
    v.modules.dedup();
    (v.aliases, v.modules, v.star)
}
