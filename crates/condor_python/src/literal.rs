//! Condition evaluation for reachability (vulture `utils._safe_eval`), built
//! on the truthiness of `ast.literal_eval`.

use ruff_python_ast::{BoolOp, Expr, Number, Operator, UnaryOp};

struct Value {
    truthy: bool,
    hashable: bool,
}

/// `utils.condition_is_always_false`.
pub fn always_false(e: &Expr) -> bool {
    !safe_eval(e, true)
}

/// `utils.condition_is_always_true`.
pub fn always_true(e: &Expr) -> bool {
    safe_eval(e, false)
}

fn safe_eval(e: &Expr, default: bool) -> bool {
    match e {
        // Every operand is evaluated: no short circuit.
        Expr::BoolOp(b) => {
            let results: Vec<bool> = b.values.iter().map(|v| safe_eval(v, default)).collect();
            match b.op {
                BoolOp::And => results.iter().all(|&r| r),
                BoolOp::Or => results.iter().any(|&r| r),
            }
        }
        Expr::UnaryOp(u) if u.op == UnaryOp::Not => !safe_eval(&u.operand, !default),
        // Unhashable set members or dict keys make vulture crash with a
        // TypeError; condor falls back to the default instead.
        _ => literal(e).map_or(default, |v| v.truthy),
    }
}

fn constant(truthy: bool) -> Option<Value> {
    Some(Value { truthy, hashable: true })
}

/// `ast.literal_eval`, reduced to the truthiness and hashability of the value.
/// `None` means the node is not a literal (`ValueError`) or the value cannot be
/// built (`TypeError`).
fn literal(e: &Expr) -> Option<Value> {
    match e {
        Expr::StringLiteral(s) => constant(!s.value.is_empty()),
        Expr::BytesLiteral(b) => constant(!b.value.is_empty()),
        Expr::NumberLiteral(n) => constant(number_truthy(&n.value)),
        Expr::BooleanLiteral(b) => constant(b.value),
        Expr::NoneLiteral(_) => constant(false),
        Expr::EllipsisLiteral(_) => constant(true),
        Expr::Tuple(t) => {
            let elts: Option<Vec<Value>> = t.elts.iter().map(literal).collect();
            let elts = elts?;
            Some(Value { truthy: !elts.is_empty(), hashable: elts.iter().all(|v| v.hashable) })
        }
        Expr::List(l) => {
            let elts: Option<Vec<Value>> = l.elts.iter().map(literal).collect();
            Some(Value { truthy: !elts?.is_empty(), hashable: false })
        }
        Expr::Set(s) => {
            let elts: Option<Vec<Value>> = s.elts.iter().map(literal).collect();
            let elts = elts?;
            if !elts.iter().all(|v| v.hashable) {
                return None;
            }
            Some(Value { truthy: !elts.is_empty(), hashable: false })
        }
        Expr::Dict(d) => {
            let mut n = 0;
            for item in &d.items {
                let key = literal(item.key.as_ref()?)?;
                literal(&item.value)?;
                if !key.hashable {
                    return None;
                }
                n += 1;
            }
            Some(Value { truthy: n > 0, hashable: false })
        }
        Expr::Call(c) => {
            let is_set = matches!(&*c.func, Expr::Name(n) if n.id.as_str() == "set");
            if is_set && c.arguments.args.is_empty() && c.arguments.keywords.is_empty() {
                Some(Value { truthy: false, hashable: false })
            } else {
                None
            }
        }
        Expr::UnaryOp(_) => signed_num(e).map(|(re, im)| Value { truthy: re != 0.0 || im != 0.0, hashable: true }),
        Expr::BinOp(b) if matches!(b.op, Operator::Add | Operator::Sub) => {
            let (left, left_im) = signed_num(&b.left)?;
            if left_im != 0.0 || is_complex(&b.left) {
                return None;
            }
            let Expr::NumberLiteral(r) = &*b.right else { return None };
            let Number::Complex { real, imag } = r.value else { return None };
            let (re, im) = if b.op == Operator::Add { (left + real, imag) } else { (left - real, -imag) };
            Some(Value { truthy: re != 0.0 || im != 0.0 || re.is_nan() || im.is_nan(), hashable: true })
        }
        _ => None,
    }
}

fn is_complex(e: &Expr) -> bool {
    let inner = match e {
        Expr::UnaryOp(u) => &*u.operand,
        other => other,
    };
    matches!(inner, Expr::NumberLiteral(n) if matches!(n.value, Number::Complex { .. }))
}

/// `_convert_signed_num`: a numeric constant, optionally with one unary `+`/`-`.
/// Returns the value as `(real, imag)`; only zero-ness matters, so integers are
/// mapped to 0.0 or 1.0.
fn signed_num(e: &Expr) -> Option<(f64, f64)> {
    match e {
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::UAdd | UnaryOp::USub) => {
            let Expr::NumberLiteral(n) = &*u.operand else { return None };
            let (re, im) = number_parts(&n.value);
            Some(if u.op == UnaryOp::USub { (-re, -im) } else { (re, im) })
        }
        Expr::NumberLiteral(n) => Some(number_parts(&n.value)),
        _ => None,
    }
}

fn number_parts(n: &Number) -> (f64, f64) {
    match n {
        Number::Int(i) => (if *i == ruff_python_ast::Int::ZERO { 0.0 } else { 1.0 }, 0.0),
        Number::Float(f) => (*f, 0.0),
        Number::Complex { real, imag } => (*real, *imag),
    }
}

fn number_truthy(n: &Number) -> bool {
    let (re, im) = number_parts(n);
    re != 0.0 || im != 0.0 || re.is_nan() || im.is_nan()
}
