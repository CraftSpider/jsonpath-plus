use std::borrow::Cow;
use super::*;
use crate::eval::EvalCtx;

use serde_json::Value;

impl Path {
    pub(crate) fn eval(&self, ctx: &mut EvalCtx<'_>) {
        for op in &self.children {
            op.eval(ctx)
        }
        if self.tilde.is_some() {
            todo!()
        }
    }
}

impl Operator {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        match self {
            Operator::Dot(_, op) => op.eval(ctx),
            Operator::Bracket(_, op) => op.eval(ctx),
            Operator::Recursive(_, op) => op.eval(ctx),
        }
    }
}

impl DotIdent {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        match self {
            DotIdent::Wildcard(_) => {
                ctx.apply_matched(|_, a| match a {
                    Value::Array(a) => a.iter().collect(),
                    Value::Object(m) => m.values().collect(),
                    _ => vec![]
                })
            }
            DotIdent::Parent(_) => {
                ctx.apply_matched(|ctx, a| {
                    ctx.parent_of(a)
                        .map(|a| vec![a])
                        .unwrap_or_default()
                })
            }
            DotIdent::Name(name) => {
                ctx.apply_matched(|_, a| match a {
                    Value::Object(m) => m.get(name.as_str())
                        .map(|a| vec![a])
                        .unwrap_or_default(),
                    _ => vec![],
                })
            }
        }
    }
}

fn step_handle(val: i64) -> (bool, usize) {
    if val < 0 {
        (true, val.abs() as usize)
    } else {
        (false, val as usize)
    }
}

fn idx_handle(val: i64, slice: &[Value]) -> usize {
    if val < 0 {
        slice.len() - val.abs() as usize
    } else {
        val as usize
    }
}

impl BracketInner {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        match self {
            BracketInner::StepRange(start, _, end, _, step) => {
                let start = start.as_ref().map(|i| i.val).unwrap_or(0);
                let end = end.as_ref().map(|i| i.val).unwrap_or(i64::MAX);
                let step = step.as_ref().map(|i| i.val.get()).unwrap_or(1);

                let (rev, step) = step_handle(step);

                ctx.apply_matched(|_, a| match a {
                    Value::Array(v) => {
                        let start = idx_handle(start, v);
                        let end = idx_handle(end, v);

                        let iter = v[start..end].iter();

                        if rev {
                            iter.rev().step_by(step).collect()
                        } else {
                            iter.step_by(step).collect()
                        }
                    }
                    _ => vec![],
                })
            }
            BracketInner::Range(start, _, end) => {
                let start = start.as_ref().map(|i| i.val).unwrap_or(0);
                let end = end.as_ref().map(|i| i.val).unwrap_or(i64::MAX);

                ctx.apply_matched(|_, a| match a {
                    Value::Array(v) => {
                        let start = idx_handle(start, v);
                        let end = idx_handle(end, v);

                        v[start..end].iter().collect()
                    }
                    _ => vec![],
                })
            }
            BracketInner::Wildcard(_) => {
                ctx.apply_matched(|_, a| match a {
                    Value::Array(v) => v.iter().collect(),
                    Value::Object(m) => m.values().collect(),
                    _ => vec![]
                })
            }
            BracketInner::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a).map(|a| vec![a]).unwrap_or_default())
            }
            BracketInner::Path(path) => {
                path.eval_match(ctx);
            }
            BracketInner::Filter(filter) => {
                filter.eval(ctx);
            }
            BracketInner::Literal(lit) => {
                lit.eval(ctx);
            }
        }
    }
}

impl BracketLit {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        match self {
            BracketLit::Int(i) => {
                ctx.apply_matched(|_, a| match a {
                    Value::Array(v) => v.get(idx_handle(i.val, v)).map(|a| vec![a]).unwrap_or_default(),
                    Value::Object(m) => m.get(&i.val.to_string()).map(|a| vec![a]).unwrap_or_default(),
                    _ => vec![]
                })
            }
            BracketLit::String(s) => {
                ctx.apply_matched(|_, a| match a {
                    Value::Object(m) => m.get(s.as_str()).map(|a| vec![a]).unwrap_or_default(),
                    _ => vec![]
                })
            }
        }
    }
}

impl SubPath {
    fn eval_expr<'a>(&self, ctx: &EvalCtx<'a>, a: &'a Value) -> Option<Cow<'a, Value>> {
        let relative = match self.kind {
            PathKind::Root(_) => false,
            PathKind::Relative(_) => true,
        };

        let new_root = if relative {
            a
        } else {
            ctx.root()
        };

        let mut new_ctx = EvalCtx::new_parents(new_root, ctx.all_parents().clone());
        for op in &self.children {
            op.eval(&mut new_ctx)
        }
        let matched = new_ctx.into_matched();

        if matched.len() != 1 {
            None
        } else {
            let matched = if self.tilde.is_some() {
                Cow::Owned(ctx.idx_of(matched[0])?.into())
            } else {
                Cow::Borrowed(matched[0])
            };

            Some(matched)
        }
    }

    fn eval_match(&self, ctx: &mut EvalCtx<'_>) {
        let relative = match self.kind {
            PathKind::Root(_) => false,
            PathKind::Relative(_) => true,
        };

        ctx.apply_matched(|ctx, a| {
            let new_root = if relative {
                a
            } else {
                ctx.root()
            };

            let mut new_ctx = EvalCtx::new_parents(new_root, ctx.all_parents().clone());
            for op in &self.children {
                op.eval(&mut new_ctx)
            }
            let matched = new_ctx.into_matched();

            let matched = if self.tilde.is_some() {
                matched.into_iter()
                    .map(|a| Cow::Owned(ctx.idx_of(a).unwrap().into()))
                    .collect::<Vec<_>>()
            } else {
                matched.into_iter()
                    .map(Cow::Borrowed)
                    .collect()
            };

            matched
                .into_iter()
                .flat_map(|mat| {
                    match a {
                        Value::Array(v) => {
                            let idx = match &*mat {
                                Value::Number(n) => Some(idx_handle(n.as_i64().unwrap(), v)),
                                _ => None,
                            };
                            idx
                                .and_then(|i| v.get(i))
                                .map(|a| vec![a])
                                .unwrap_or_default()
                        }
                        Value::Object(m) => {
                            let idx = match &*mat {
                                Value::String(s) => Some(s.to_string()),
                                Value::Number(n) => Some(n.to_string()),
                                _ => None,
                            };

                            idx
                                .and_then(|i| m.get(&i))
                                .map(|a| vec![a])
                                .unwrap_or_default()
                        }
                        _ => vec![]
                    }
                })
                .collect()
        })
    }
}

impl Filter {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        ctx.apply_matched(|ctx, a| {
            let result = self.inner.eval_expr(ctx, a)
                .map(|c| c.as_bool() == Some(true))
                .unwrap_or(false);

            if result {
                vec![a]
            } else {
                vec![]
            }
        })
    }
}

impl FilterExpr {
    fn eval_expr<'a>(&self, ctx: &EvalCtx<'a>, val: &'a Value) -> Option<Cow<'a, Value>> {
        match self {
            FilterExpr::Unary(op, inner) => {
                let inner = inner.eval_expr(ctx, val)?;

                match op {
                    UnOp::Neg(_) => {
                        match &*inner {
                            Value::Number(n) => {
                                let out = n.as_i64().map(|i| Value::from(-i))
                                    .or_else(|| n.as_u64().map(|i| Value::from(-(i as i64))))
                                    .or_else(|| n.as_f64().map(|f| Value::from(-f)));
                                Some(Cow::Owned(out.unwrap()))
                            }
                            _ => None,
                        }
                    }
                    UnOp::Not(_) => {
                        match &*inner {
                            Value::Bool(b) => {
                                Some(Cow::Owned(Value::from(!b)))
                            }
                            _ => None
                        }
                    }
                }
            }
            FilterExpr::Binary(lhs, op, rhs) => {
                let lhs = lhs.eval_expr(ctx, val)?;
                let rhs = rhs.eval_expr(ctx, val)?;

                match op {
                    BinOp::And(_) => {
                        let lhs = lhs.as_bool()?;
                        let rhs = rhs.as_bool()?;
                        Some(Cow::Owned(Value::Bool(lhs && rhs)))
                    }
                    BinOp::Or(_) => {
                        let lhs = lhs.as_bool()?;
                        let rhs = rhs.as_bool()?;
                        Some(Cow::Owned(Value::Bool(lhs || rhs)))
                    }

                    BinOp::Eq(_) => Some(Cow::Owned(Value::Bool(lhs == rhs))),
                    BinOp::Le(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(Value::Bool(lhs <= rhs)))
                    },
                    BinOp::Lt(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(Value::Bool(lhs < rhs)))
                    }
                    BinOp::Gt(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(Value::Bool(lhs > rhs)))
                    }
                    BinOp::Ge(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(Value::Bool(lhs >= rhs)))
                    }

                    BinOp::Add(_) => {
                        if lhs.is_f64() && rhs.is_f64() {
                            let lhs = lhs.as_f64()?;
                            let rhs = rhs.as_f64()?;

                            Some(Cow::Owned(Value::from(lhs + rhs)))
                        } else if lhs.is_string() && rhs.is_string() {
                            let lhs = lhs.as_str()?;
                            let rhs = rhs.as_str()?;

                            Some(Cow::Owned(Value::String(format!("{lhs}{rhs}"))))
                        } else {
                            None
                        }
                    }
                    BinOp::Sub(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(Value::from(lhs - rhs)))
                    }
                    BinOp::Mul(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(Value::from(lhs * rhs)))
                    }
                    BinOp::Div(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(Value::from(lhs / rhs)))
                    }

                    _ => todo!(),
                }
            }
            FilterExpr::Path(path) => {
                path.eval_expr(ctx, val)
            }
            FilterExpr::Lit(lit) => {
                Some(Cow::Owned(match lit {
                    ExprLit::Int(i) => Value::from(i.val),
                    ExprLit::Str(s) => Value::from(s.as_str()),
                    ExprLit::Bool(b) => Value::from(b.val),
                    ExprLit::Null(_) => Value::Null,
                }))
            }
            FilterExpr::Parens(_, inner) => {
                inner.eval_expr(ctx, val)
            }
        }
    }
}

impl RecursiveOp {
    fn eval(&self, _ctx: &mut EvalCtx<'_>) {
        todo!()
    }
}
