use super::*;
use crate::eval::EvalCtx;
use std::borrow::Cow;

use crate::json::{Json, JsonArray, JsonObject, JsonNumber, JsonRef};

fn flatten_recur<'a, T: Json>(collect: &mut Vec<&'a T>, a: &'a T) {
    collect.push(a);
    match a.as_ref() {
        JsonRef::Array(v) => v.iter().for_each(|a| flatten_recur(collect, a)),
        JsonRef::Object(m) => m.values().for_each(|a| flatten_recur(collect, a)),
        _ => (),
    }
}

impl Path {
    pub(crate) fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
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
    fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
        match self {
            Segment::Dot(_, op) => op.eval(ctx),
            Segment::Bracket(_, op) => op.eval(ctx),
            Segment::Recursive(_, op) => {
                // Ensure that apply_matched doesn't add incorrect parent relationships.
                // We need to do this work anyways
                ctx.prepopulate_parents();
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

impl RecursiveOp {
    fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
        match self {
            RecursiveOp::Raw(inner) => inner.eval(ctx),
            RecursiveOp::Bracket(_, inner) => inner.eval(ctx),
        }
    }
}

impl RawSelector {
    fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
        match self {
            RawSelector::Wildcard(_) => ctx.apply_matched(|_, a| match a.as_ref() {
                JsonRef::Array(v) => v.iter().collect(),
                JsonRef::Object(m) => m.values().collect(),
                _ => vec![],
            }),
            RawSelector::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a).map(|a| vec![a]).unwrap_or_default());
            }
            RawSelector::Name(name) => ctx.apply_matched(|_, a| match a.as_ref() {
                JsonRef::Object(m) => m.get(name.as_str()).map(|a| vec![a]).unwrap_or_default(),
                _ => vec![],
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

fn idx_handle<T: Json>(val: i64, slice: &impl JsonArray<T>) -> Option<usize> {
    if val < 0 {
        slice.len().checked_sub(val.abs() as usize)
    } else {
        Some(val as usize)
    }
}

fn range<T: Json>(slice: &T::Array, start: usize, end: usize) -> Vec<&T> {
    if start > end || start > slice.len() {
        vec![]
    } else if end >= slice.len() {
        slice.iter().skip(start).collect()
    } else {
        slice.iter().skip(start).take(end - start).collect()
    }
}

impl StepRange {
    fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
        let start = self.start.as_ref().map_or(0, |i| i.as_int());
        let end = self.end.as_ref().map_or(i64::MAX, |i| i.as_int());
        let step = self.step.as_ref().map_or(1, |i| i.as_int().get());

        let (rev, step) = step_handle(step);

        ctx.apply_matched(|_, a| match a.as_ref() {
            JsonRef::Array(v) => {
                let start = idx_handle(start, v).unwrap_or(0);
                let end = idx_handle(end, v).unwrap_or(0);

                let vec = range(v, start, end);

                if rev {
                    vec.into_iter().rev().step_by(step).collect()
                } else {
                    vec.into_iter().step_by(step).collect()
                }
            }
            _ => vec![],
        });
    }
}

impl Range {
    fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
        let start = self.start.as_ref().map_or(0, |i| i.as_int());
        let end = self.end.as_ref().map_or(i64::MAX, |i| i.as_int());

        ctx.apply_matched(|_, a| match a.as_ref() {
            JsonRef::Array(v) => {
                let start = idx_handle(start, v).unwrap_or(0);
                let end = idx_handle(end, v).unwrap_or(0);

                range(v, start, end).into_iter().collect()
            }
            _ => vec![],
        });
    }
}

impl UnionComponent {
    fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
        match self {
            UnionComponent::StepRange(step_range) => step_range.eval(ctx),
            UnionComponent::Range(range) => range.eval(ctx),
            UnionComponent::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a).map(|a| vec![a]).unwrap_or_default());
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
    fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
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
            BracketSelector::Wildcard(_) => ctx.apply_matched(|_, a| match a.as_ref() {
                JsonRef::Array(v) => v.iter().collect(),
                JsonRef::Object(m) => m.values().collect(),
                _ => vec![],
            }),
            BracketSelector::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a).map(|a| vec![a]).unwrap_or_default());
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
    fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
        match self {
            BracketLit::Int(i) => ctx.apply_matched(|_, a| match a.as_ref() {
                JsonRef::Array(v) => idx_handle(i.as_int(), v)
                    .and_then(|idx| v.get(idx))
                    .map(|a| vec![a])
                    .unwrap_or_default(),
                _ => vec![],
            }),
            BracketLit::String(s) => ctx.apply_matched(|_, a| match a.as_ref() {
                JsonRef::Object(m) => m.get(s.as_str()).map(|a| vec![a]).unwrap_or_default(),
                _ => vec![],
            }),
        }
    }
}

impl SubPath {
    fn eval_expr<'a, T: Json>(&self, ctx: &EvalCtx<'a, T>, a: &'a T) -> Option<Cow<'a, T>> {
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
                Cow::Owned(T::from_idx(ctx.idx_of(matched[0])?))
            } else {
                Cow::Borrowed(matched[0])
            };

            Some(matched)
        } else {
            None
        }
    }

    fn eval_match<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
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
                    .map(|a| Cow::Owned(T::from_idx(ctx.idx_of(a).unwrap())))
                    .collect::<Vec<_>>()
            } else {
                matched.into_iter().map(Cow::Borrowed).collect()
            };

            matched
                .into_iter()
                .flat_map(|mat| match a.as_ref() {
                    JsonRef::Array(v) => {
                        let idx = match (*mat).as_ref() {
                            JsonRef::Number(n) => idx_handle(n.as_i64().unwrap(), v),
                            _ => None,
                        };
                        idx.and_then(|i| v.get(i))
                            .map(|a| vec![a])
                            .unwrap_or_default()
                    }
                    JsonRef::Object(m) => {
                        let idx = match (*mat).as_ref() {
                            JsonRef::String(s) => Some(s.to_string()),
                            JsonRef::Number(n) => Some(n.to_string()),
                            _ => None,
                        };

                        idx.and_then(|i| m.get(&i))
                            .map(|a| vec![a])
                            .unwrap_or_default()
                    }
                    _ => vec![],
                })
                .collect()
        });
    }
}

impl Filter {
    fn eval<'a, T: Json>(&self, ctx: &mut EvalCtx<'a, T>) {
        ctx.apply_matched(|ctx, a| match a.as_ref() {
            JsonRef::Array(v) => v
                .iter()
                .filter(|&a| {
                    self.inner
                        .eval_expr(ctx, a)
                        .map_or(false, |c| c.as_bool() == Some(true))
                })
                .collect(),
            JsonRef::Object(m) => m
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
    fn eval_expr<'a, T: Json>(&self, ctx: &EvalCtx<'a, T>, val: &'a T) -> Option<Cow<'a, T>> {
        match self {
            FilterExpr::Unary(op, inner) => {
                let inner = inner.eval_expr(ctx, val)?;

                match op {
                    UnOp::Neg(_) => match (*inner).as_ref() {
                        JsonRef::Number(n) => {
                            let out = n
                                .as_i64()
                                .map(|i| T::from_i64(-i))
                                .or_else(|| n.as_u64().map(|i| T::from_i64(-(i as i64))))
                                .or_else(|| n.as_f64().map(|f| T::from_f64(-f)));
                            Some(Cow::Owned(out.unwrap()))
                        }
                        _ => None,
                    },
                    UnOp::Not(_) => match (*inner).as_ref() {
                        JsonRef::Bool(b) => Some(Cow::Owned(T::from_bool(!b))),
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
                        Some(Cow::Owned(T::from_bool(lhs && rhs)))
                    }
                    BinOp::Or(_) => {
                        let lhs = lhs.as_bool()?;
                        let rhs = rhs.as_bool()?;
                        Some(Cow::Owned(T::from_bool(lhs || rhs)))
                    }

                    BinOp::Eq(_) => Some(Cow::Owned(T::from_bool(lhs == rhs))),
                    BinOp::Le(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(T::from_bool(lhs <= rhs)))
                    }
                    BinOp::Lt(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(T::from_bool(lhs < rhs)))
                    }
                    BinOp::Gt(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(T::from_bool(lhs > rhs)))
                    }
                    BinOp::Ge(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(T::from_bool(lhs >= rhs)))
                    }

                    BinOp::Add(_) => {
                        if lhs.is_f64() && rhs.is_f64() {
                            let lhs = lhs.as_f64()?;
                            let rhs = rhs.as_f64()?;

                            Some(Cow::Owned(T::from_f64(lhs + rhs)))
                        } else if lhs.is_string() && rhs.is_string() {
                            let lhs = lhs.as_str()?;
                            let rhs = rhs.as_str()?;

                            Some(Cow::Owned(T::from_str(format!("{lhs}{rhs}"))))
                        } else {
                            None
                        }
                    }
                    BinOp::Sub(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(T::from_f64(lhs - rhs)))
                    }
                    BinOp::Mul(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(T::from_f64(lhs * rhs)))
                    }
                    BinOp::Div(_) => {
                        let lhs = lhs.as_f64()?;
                        let rhs = rhs.as_f64()?;

                        Some(Cow::Owned(T::from_f64(lhs / rhs)))
                    }
                    BinOp::Rem(_) => {
                        let lhs = lhs.as_i64()?;
                        let rhs = rhs.as_i64()?;

                        Some(Cow::Owned(T::from_i64(lhs % rhs)))
                    }
                }
            }
            FilterExpr::Path(path) => path.eval_expr(ctx, val),
            FilterExpr::Lit(lit) => Some(Cow::Owned(match lit {
                ExprLit::Int(i) => T::from_i64(i.as_int()),
                ExprLit::String(s) => T::from_str(s.as_str().to_owned()),
                ExprLit::Bool(b) => T::from_bool(b.as_bool()),
                ExprLit::Null(_) => T::null(),
            })),
            FilterExpr::Parens(_, inner) => inner.eval_expr(ctx, val),
        }
    }
}
