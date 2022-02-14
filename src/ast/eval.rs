use super::*;
use crate::eval::EvalCtx;
use std::borrow::Cow;

use serde_json::Value;

fn flatten_recur<'a>(collect: &mut Vec<&'a Value>, a: &'a Value) {
    collect.push(a);
    match a {
        Value::Array(v) => v.iter().for_each(|a| flatten_recur(collect, a)),
        Value::Object(m) => m.values().for_each(|a| flatten_recur(collect, a)),
        _ => (),
    }
}

impl Path {
    pub(crate) fn has_parent(&self) -> bool {
        for op in &self.segments {
            let result = match op {
                Segment::Dot(_, RawSelector::Parent(_))
                | Segment::Recursive(_, Some(RawSelector::Parent(_)))
                | Segment::Bracket(_, BracketSelector::Parent(_)) => true,
                Segment::Bracket(_, BracketSelector::Path(p)) => p.has_parent(),
                Segment::Bracket(_, BracketSelector::Filter(f)) => f.has_parent(),
                _ => false,
            };

            if result {
                return true;
            }
        }
        return false;
    }

    pub(crate) fn eval(&self, ctx: &mut EvalCtx<'_>) {
        for op in &self.segments {
            op.eval(ctx);
        }
        if self.tilde.is_some() {
            unimplemented!(
                "Tilde at the top level isn't yet supported due to API design questions. Please \
                raise an issue with your use case"
            )
        }
    }
}

impl Segment {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        match self {
            Segment::Dot(_, op) => op.eval(ctx),
            Segment::Bracket(_, op) => op.eval(ctx),
            Segment::Recursive(_, op) => {
                ctx.apply_matched(|_, a| {
                    let mut all = Vec::new();
                    flatten_recur(&mut all, a);
                    all
                });
                if let Some(inner) = op {
                    inner.eval(ctx);
                }
            }
        }
    }
}

impl RawSelector {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        match self {
            RawSelector::Wildcard(_) => ctx.apply_matched(|_, a| match a {
                Value::Array(v) => v.iter().collect(),
                Value::Object(m) => m.values().collect(),
                _ => vec![],
            }),
            RawSelector::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a));
            }
            RawSelector::Name(name) => ctx.apply_matched(|_, a| match a {
                Value::Object(m) => m.get(name.as_str()),
                _ => None,
            }),
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

fn idx_handle(val: i64, slice: &[Value]) -> Option<usize> {
    if val < 0 {
        slice.len().checked_sub(val.abs() as usize)
    } else {
        Some(val as usize)
    }
}

fn range(slice: &[Value], start: usize, end: usize) -> &[Value] {
    if start > end || start > slice.len() {
        &[]
    } else if end >= slice.len() {
        &slice[start..]
    } else {
        &slice[start..end]
    }
}

impl StepRange {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        let start = self.start.as_ref().map_or(0, |i| i.as_int());
        let end = self.end.as_ref().map_or(i64::MAX, |i| i.as_int());
        let step = self.step.as_ref().map_or(1, |i| i.as_int().get());

        let (rev, step) = step_handle(step);

        ctx.apply_matched(|_, a| match a {
            Value::Array(v) => {
                let start = idx_handle(start, v).unwrap_or(0);
                let end = idx_handle(end, v).unwrap_or(0);

                let iter = range(v, start, end).iter();

                if rev {
                    iter.rev().step_by(step).collect()
                } else {
                    iter.step_by(step).collect()
                }
            }
            _ => vec![],
        });
    }
}

impl Range {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        let start = self.start.as_ref().map_or(0, |i| i.as_int());
        let end = self.end.as_ref().map_or(i64::MAX, |i| i.as_int());

        ctx.apply_matched(|_, a| match a {
            Value::Array(v) => {
                let start = idx_handle(start, v).unwrap_or(0);
                let end = idx_handle(end, v).unwrap_or(0);

                range(v, start, end).iter().collect()
            }
            _ => vec![],
        });
    }
}

impl UnionComponent {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        match self {
            UnionComponent::StepRange(step_range) => step_range.eval(ctx),
            UnionComponent::Range(range) => range.eval(ctx),
            UnionComponent::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a));
            }
            UnionComponent::Path(path) => {
                path.eval_match(ctx);
            }
            UnionComponent::Filter(filter) => {
                filter.eval(ctx);
            }
            UnionComponent::Literal(lit) => {
                lit.eval(ctx);
            }
        }
    }
}

impl BracketSelector {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        match self {
            BracketSelector::Union(components) => {
                let mut new_matched = Vec::new();
                for component in components {
                    let mut new_ctx = ctx.child_ctx();
                    component.eval(&mut new_ctx);
                    new_matched.extend(new_ctx.into_matched());
                }
                ctx.set_matched(new_matched);
            }
            BracketSelector::StepRange(step_range) => step_range.eval(ctx),
            BracketSelector::Range(range) => range.eval(ctx),
            BracketSelector::Wildcard(_) => ctx.apply_matched(|_, a| match a {
                Value::Array(v) => v.iter().collect(),
                Value::Object(m) => m.values().collect(),
                _ => vec![],
            }),
            BracketSelector::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a));
            }
            BracketSelector::Path(path) => {
                path.eval_match(ctx);
            }
            BracketSelector::Filter(filter) => {
                filter.eval(ctx);
            }
            BracketSelector::Literal(lit) => {
                lit.eval(ctx);
            }
        }
    }
}

impl BracketLit {
    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        match self {
            BracketLit::Int(i) => ctx.apply_matched(|_, a| match a {
                Value::Array(v) => idx_handle(i.as_int(), v).and_then(|idx| v.get(idx)),
                _ => None,
            }),
            BracketLit::String(s) => ctx.apply_matched(|_, a| match a {
                Value::Object(m) => m.get(s.as_str()),
                _ => None,
            }),
        }
    }
}

impl SubPath {
    pub(crate) fn has_parent(&self) -> bool {
        for op in &self.segments {
            let result = match op {
                Segment::Dot(_, RawSelector::Parent(_))
                | Segment::Recursive(_, Some(RawSelector::Parent(_)))
                | Segment::Bracket(_, BracketSelector::Parent(_)) => true,
                Segment::Bracket(_, BracketSelector::Path(p)) => p.has_parent(),
                Segment::Bracket(_, BracketSelector::Filter(f)) => f.has_parent(),
                _ => false,
            };

            if result {
                return true;
            }
        }
        return false;
    }

    fn eval_expr<'a>(&self, ctx: &EvalCtx<'a>, a: &'a Value) -> Option<Cow<'a, Value>> {
        let relative = match self.kind {
            PathKind::Root(_) => false,
            PathKind::Relative(_) => true,
        };

        let new_root = if relative { a } else { ctx.root() };

        let mut new_ctx = EvalCtx::new_parents(new_root, ctx.all_parents().clone());
        for op in &self.segments {
            op.eval(&mut new_ctx);
        }
        let matched = new_ctx.into_matched();

        if matched.len() == 1 {
            let matched = if self.tilde.is_some() {
                Cow::Owned(ctx.idx_of(matched[0])?.into())
            } else {
                Cow::Borrowed(matched[0])
            };

            Some(matched)
        } else {
            None
        }
    }

    fn eval_match(&self, ctx: &mut EvalCtx<'_>) {
        let relative = match self.kind {
            PathKind::Root(_) => false,
            PathKind::Relative(_) => true,
        };

        ctx.apply_matched(|ctx, a| {
            let new_root = if relative { a } else { ctx.root() };

            let mut new_ctx = EvalCtx::new_parents(new_root, ctx.all_parents().clone());
            for op in &self.segments {
                op.eval(&mut new_ctx);
            }
            let matched = new_ctx.into_matched();

            let matched = if self.tilde.is_some() {
                matched
                    .into_iter()
                    .map(|a| Cow::Owned(ctx.idx_of(a).unwrap().into()))
                    .collect::<Vec<_>>()
            } else {
                matched.into_iter().map(Cow::Borrowed).collect()
            };

            matched
                .into_iter()
                .flat_map(|mat| match a {
                    Value::Array(v) => {
                        let idx = match &*mat {
                            Value::Number(n) => idx_handle(n.as_i64().unwrap(), v),
                            _ => None,
                        };
                        idx.and_then(|i| v.get(i))
                    }
                    Value::Object(m) => {
                        let idx = match &*mat {
                            Value::String(s) => Some(s.to_string()),
                            Value::Number(n) => Some(n.to_string()),
                            _ => None,
                        };

                        idx.and_then(|i| m.get(&i))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
    }
}

impl Filter {
    fn has_parent(&self) -> bool {
        self.inner.has_parent()
    }

    fn eval(&self, ctx: &mut EvalCtx<'_>) {
        ctx.apply_matched(|ctx, a| match a {
            Value::Array(v) => v
                .iter()
                .filter(|&a| {
                    self.inner
                        .eval_expr(ctx, a)
                        .map_or(false, |c| c.as_bool() == Some(true))
                })
                .collect(),
            Value::Object(m) => m
                .values()
                .filter(|&a| {
                    self.inner
                        .eval_expr(ctx, a)
                        .map_or(false, |c| c.as_bool() == Some(true))
                })
                .collect(),
            _ => vec![],
        });
    }
}

impl FilterExpr {
    fn has_parent(&self) -> bool {
        match self {
            FilterExpr::Unary(_, inner) => inner.has_parent(),
            FilterExpr::Binary(left, _, right) => left.has_parent() || right.has_parent(),
            FilterExpr::Parens(_, inner) => inner.has_parent(),
            FilterExpr::Path(p) => p.has_parent(),
            _ => false,
        }
    }

    fn eval_expr<'a>(&self, ctx: &EvalCtx<'a>, val: &'a Value) -> Option<Cow<'a, Value>> {
        match self {
            FilterExpr::Unary(op, inner) => {
                let inner = inner.eval_expr(ctx, val)?;

                match op {
                    UnOp::Neg(_) => match &*inner {
                        Value::Number(n) => {
                            let out = n
                                .as_i64()
                                .map(|i| Value::from(-i))
                                .or_else(|| n.as_u64().map(|i| Value::from(-(i as i64))))
                                .or_else(|| n.as_f64().map(|f| Value::from(-f)));
                            Some(Cow::Owned(out.unwrap()))
                        }
                        _ => None,
                    },
                    UnOp::Not(_) => match &*inner {
                        Value::Bool(b) => Some(Cow::Owned(Value::from(!b))),
                        _ => None,
                    },
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
                    }
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
                    BinOp::Rem(_) => {
                        let lhs = lhs.as_i64()?;
                        let rhs = rhs.as_i64()?;

                        Some(Cow::Owned(Value::from(lhs % rhs)))
                    }
                }
            }
            FilterExpr::Path(path) => path.eval_expr(ctx, val),
            FilterExpr::Lit(lit) => Some(Cow::Owned(match lit {
                ExprLit::Int(i) => Value::from(i.as_int()),
                ExprLit::String(s) => Value::from(s.as_str()),
                ExprLit::Bool(b) => Value::from(b.as_bool()),
                ExprLit::Null(_) => Value::Null,
            })),
            FilterExpr::Parens(_, inner) => inner.eval_expr(ctx, val),
        }
    }
}
