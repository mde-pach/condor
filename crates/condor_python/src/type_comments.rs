//! Names used in type comments.
//!
//! vulture parses `# type: ...` comments and counts the names in them as used.
//! ruff's parser treats them as ordinary comments, so condor finds them in the
//! token stream. Only the uses matter for dead code; where CPython would reject
//! a misplaced type comment, condor does not (spec/divergences.md).

use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_python_ast::{Expr, ExprContext};
use ruff_text_size::Ranged;

/// CPython's tokenizer test: `#`, spaces/tabs, `type:`, spaces/tabs. Returns the
/// text after the prefix, or `None` for other comments and `# type: ignore`.
fn type_comment_body(comment: &str) -> Option<&str> {
    let rest = comment.strip_prefix('#')?;
    let body = rest.trim_start_matches([' ', '\t']).strip_prefix("type:")?.trim_start_matches([' ', '\t']);
    let ignore = body
        .strip_prefix("ignore")
        .is_some_and(|after| after.chars().next().is_none_or(|c| c.is_ascii() && !c.is_ascii_alphanumeric()));
    (!ignore).then_some(body)
}

/// The names used by every type comment in the file, in source order.
/// Comments that do not parse contribute nothing.
pub fn used_names(source: &str, tokens: &Tokens) -> Vec<String> {
    let mut names = Vec::new();
    if !source.contains("type:") {
        return names;
    }
    for tok in tokens.iter().filter(|t| t.kind() == TokenKind::Comment) {
        let Some(body) = type_comment_body(&source[tok.range()]) else { continue };
        for expr in parse(body) {
            collect_uses(&expr, &mut names);
        }
    }
    names
}

/// An expression (`int`), or a function signature (`(int, *str) -> None`).
fn parse(text: &str) -> Vec<Expr> {
    if let Ok(parsed) = ruff_python_parser::parse_expression(text.trim()) {
        return vec![*parsed.into_syntax().body];
    }
    let Some((args, returns)) = text.trim().rsplit_once("->") else { return Vec::new() };
    let mut exprs = Vec::new();
    // `(a, *b, **c)` parses as the arguments of a call.
    if let Ok(call) = ruff_python_parser::parse_expression(&format!("_{}", args.trim())) {
        if let Expr::Call(call) = *call.into_syntax().body {
            exprs.extend(call.arguments.args.into_iter());
            exprs.extend(call.arguments.keywords.into_iter().map(|k| k.value));
        }
    }
    if let Ok(ret) = ruff_python_parser::parse_expression(returns.trim()) {
        exprs.push(*ret.into_syntax().body);
    }
    exprs
}

fn collect_uses(e: &Expr, out: &mut Vec<String>) {
    use ruff_python_ast::visitor::{self, Visitor};
    struct V<'o>(&'o mut Vec<String>);
    impl<'b> Visitor<'b> for V<'_> {
        fn visit_expr(&mut self, e: &'b Expr) {
            visitor::walk_expr(self, e);
            match e {
                Expr::Name(n) if n.ctx == ExprContext::Load && !matches!(n.id.as_str(), "object" | "self") => {
                    self.0.push(n.id.to_string())
                }
                Expr::Attribute(a) if a.ctx == ExprContext::Load => self.0.push(a.attr.to_string()),
                _ => {}
            }
        }
    }
    V(out).visit_expr(e);
}
