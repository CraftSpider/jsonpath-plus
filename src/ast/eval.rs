use super::*;
use crate::eval::{EvalCtx, EvalErr, Type};
use either::Either;
use std::borrow::Cow;

use crate::utils::ValueExt;
use serde_json::Value;

pub trait ValExt {
    fn ty(&self) -> Type;

    fn try_bool(&self) -> Result<bool, EvalErr>;
    fn try_f64(&self) -> Result<f64, EvalErr>;
    fn try_i64(&self) -> Result<i64, EvalErr>;
    fn try_str(&self) -> Result<&str, EvalErr>;
}

impl ValExt for Value {
    fn ty(&self) -> Type {
        match self {
            Value::Null => Type::Null,
            Value::Bool(_) => Type::Bool,
            Value::Number(num) => {
                let is_int = num.is_i64() || num.is_u64();
                let is_float = num.is_f64();
                match (is_int, is_float) {
                    (true, false) => Type::Int,
                    (false, true) => Type::Float,
                    (_, _) => Type::Number,
                }
            },
            Value::String(_) => Type::String,
            Value::Array(_) => Type::Array,
            Value::Object(_) => Type::Object,
        }
    }

    fn try_bool(&self) -> Result<bool, EvalErr> {
        self.as_bool().ok_or(EvalErr::MismatchedTypes {
            expected: Type::Bool,
            found: self.ty(),
        })
    }

    fn try_f64(&self) -> Result<f64, EvalErr> {
        self.as_f64().ok_or(EvalErr::MismatchedTypes {
            expected: Type::Float,
            found: self.ty(),
        })
    }

    fn try_i64(&self) -> Result<i64, EvalErr> {
        self.as_i64().ok_or(EvalErr::MismatchedTypes {
            expected: Type::Int,
            found: self.ty(),
        })
    }

    fn try_str(&self) -> Result<&str, EvalErr> {
        self.as_str().ok_or(EvalErr::MismatchedTypes {
            expected: Type::String,
            found: self.ty(),
        })
    }
}

fn flatten_recur<'a>(collect: &mut Vec<&'a Value>, a: &'a Value) {
    collect.push(a);
    a.iter().for_each(|a| flatten_recur(collect, a));
}

pub(crate) trait Eval {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr>;
}

impl Eval for Path {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr> {
        for op in &self.segments {
            op.eval(ctx)?;
        }
        if self.tilde.is_some() {
            Err(EvalErr::Unsupported(
                "Tilde at the top level isn't yet supported due to API design questions. Please \
                raise an issue with your use case".to_string()
            ))
        } else {
            Ok(())
        }
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
        false
    }
}

impl Eval for Segment {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr> {
        match self {
            Segment::Dot(_, op) => op.eval(ctx),
            Segment::Bracket(_, op) => op.eval(ctx),
            Segment::Recursive(_, op) => {
                ctx.apply_matched(|_, a| {
                    let mut all = Vec::new();
                    flatten_recur(&mut all, a);
                    all
                })?;
                if let Some(inner) = op {
                    inner.eval(ctx)
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl Eval for RawSelector {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr> {
        match self {
            RawSelector::Wildcard(_) => ctx.apply_matched(|_, a| a.iter()),
            RawSelector::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a))
            }
            RawSelector::Name(name) => ctx.apply_matched(|_, a| match a {
                Value::Object(m) => m.get(name.as_str()),
                _ => None,
            }),
        }
    }
}

fn step_handle(val: i64) -> (bool, u64) {
    (val < 0, val.unsigned_abs())
}

fn idx_handle(val: i64, slice: &[Value]) -> Result<usize, EvalErr> {
    let is_neg = val.is_negative();
    let val = usize::try_from(val.unsigned_abs())
        .map_err(|_| EvalErr::out_of_range::<_, usize>(val))?;
    if is_neg {
        slice.len().checked_sub(val).ok_or(EvalErr::BadIdx)
    } else {
        Ok(val)
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

impl Eval for StepRange {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr> {
        let start = self.start.as_ref().map_or(0, |i| i.as_int());
        let end = self.end.as_ref().map_or(i64::MAX, |i| i.as_int());
        let step = self.step.as_ref().map_or(1, |i| i.as_int().get());

        let (rev, step) = step_handle(step);
        let step = usize::try_from(step)
            .map_err(|_| EvalErr::out_of_range::<_, usize>(step))?;

        ctx.apply_matched(|_, a| match a {
            Value::Array(v) => {
                let start = idx_handle(start, v).unwrap_or(0);
                let end = idx_handle(end, v).unwrap_or(0);

                let iter = range(v, start, end).iter();

                if rev {
                    Either::Left(iter.rev().step_by(step))
                } else {
                    Either::Right(iter.step_by(step))
                }
            }
            _ => Either::Right([].iter().step_by(1)),
        })
    }
}

impl Eval for Range {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr>{
        let start = self.start.as_ref().map_or(0, |i| i.as_int());
        let end = self.end.as_ref().map_or(i64::MAX, |i| i.as_int());

        ctx.apply_matched(|_, a| match a {
            Value::Array(v) => {
                let start = idx_handle(start, v).unwrap_or(0);
                let end = idx_handle(end, v).unwrap_or(0);

                range(v, start, end)
            }
            _ => &[],
        })
    }
}

impl Eval for UnionComponent {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr> {
        match self {
            UnionComponent::StepRange(step_range) => step_range.eval(ctx),
            UnionComponent::Range(range) => range.eval(ctx),
            UnionComponent::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a))
            }
            UnionComponent::Path(path) => {
                path.eval_match(ctx)
            }
            UnionComponent::Filter(filter) => {
                filter.eval(ctx)
            }
            UnionComponent::Literal(lit) => {
                lit.eval(ctx)
            }
        }
    }
}

impl Eval for BracketSelector {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr> {
        match self {
            BracketSelector::Union(components) => {
                let mut new_matched = Vec::new();
                let old_matched = ctx.get_matched().to_owned();
                for component in components {
                    ctx.set_matched(old_matched.clone());
                    component.eval(ctx)?;
                    new_matched.extend(ctx.get_matched());
                }
                ctx.set_matched(new_matched);
                Ok(())
            }
            BracketSelector::StepRange(step_range) => step_range.eval(ctx),
            BracketSelector::Range(range) => range.eval(ctx),
            BracketSelector::Wildcard(_) => {
                ctx.apply_matched(|_, a| a.iter())
            },
            BracketSelector::Parent(_) => {
                ctx.apply_matched(|ctx, a| ctx.parent_of(a))
            }
            BracketSelector::Path(path) => {
                path.eval_match(ctx)
            }
            BracketSelector::Filter(filter) => {
                filter.eval(ctx)
            }
            BracketSelector::Literal(lit) => {
                lit.eval(ctx)
            }
        }
    }
}

impl Eval for BracketLit {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr> {
        match self {
            BracketLit::Int(i) => ctx.apply_matched(|_, a| match a {
                Value::Array(v) => idx_handle(i.as_int(), v).ok().and_then(|idx| v.get(idx)),
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
        false
    }

    fn eval_expr<'a>(&self, ctx: &EvalCtx<'a, '_>, a: &'a Value) -> Result<Cow<'a, Value>, EvalErr> {
        let relative = match self.kind {
            PathKind::Root(_) => false,
            PathKind::Relative(_) => true,
        };

        let new_root = if relative { a } else { ctx.root() };

        let mut new_ctx = EvalCtx::new_parents(new_root, ctx.all_parents());
        for op in &self.segments {
            op.eval(&mut new_ctx)?;
        }
        let matched = new_ctx.into_matched();

        match matched.len() {
            0 => Err(EvalErr::MatchedNone),
            1 => {
                let matched = if self.tilde.is_some() {
                    Cow::Owned(ctx.idx_of(matched[0]).ok_or(EvalErr::BadIdx)?.into())
                } else {
                    Cow::Borrowed(matched[0])
                };

                Ok(matched)
            }
            _ => Err(EvalErr::MatchedMany),
        }
    }

    fn eval_match(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr> {
        let relative = match self.kind {
            PathKind::Root(_) => false,
            PathKind::Relative(_) => true,
        };

        ctx.set_matched(ctx.apply_matched_ref(|ctx, a| {
            let new_root = if relative { a } else { ctx.root() };

            let mut new_ctx = EvalCtx::new_parents(new_root, ctx.all_parents());
            for op in &self.segments {
                op.eval(&mut new_ctx)?;
            }

            let id = self.tilde.is_some();

            Ok(new_ctx
                .into_matched()
                .into_iter()
                .map(move |a| {
                    if id {
                        Cow::Owned(ctx.idx_of(a).unwrap().into())
                    } else {
                        Cow::Borrowed(a)
                    }
                })
                .flat_map(move |mat| match a {
                    Value::Array(v) => {
                        let idx = match &*mat {
                            Value::Number(n) => idx_handle(n.as_i64().unwrap(), v).ok(),
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
                }))
        })?);
        Ok(())
    }
}

impl Filter {
    fn has_parent(&self) -> bool {
        self.inner.has_parent()
    }
}

impl Eval for Filter {
    fn eval(&self, ctx: &mut EvalCtx<'_, '_>) -> Result<(), EvalErr> {
        ctx.set_matched(ctx.apply_matched_ref(|ctx, a| {
            Ok(a.iter().filter(|&a| {
                self.inner
                    .eval_expr(ctx, a)
                    .map_or(false, |c| c.as_bool() == Some(true))
            }))
        })?);
        Ok(())
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

    fn eval_unary<'a>(
        &self,
        ctx: &EvalCtx<'a, '_>,
        val: &'a Value,
        op: &UnOp,
        inner: &FilterExpr,
    ) -> Result<Cow<'a, Value>, EvalErr> {
        let inner = inner.eval_expr(ctx, val)?;

        match op {
            UnOp::Neg(_) => match &*inner {
                Value::Number(n) => {
                    let out = n
                        .as_i64()
                        .map(|i| Value::from(-i))
                        .or_else(|| n.as_f64().map(|f| Value::from(-f)))
                        .ok_or_else(|| EvalErr::out_of_range::<_, i64>(n.as_u64().unwrap()))?;
                    Ok(Cow::Owned(out))
                }
                val => Err(EvalErr::MismatchedTypes { expected: Type::Number, found: val.ty() }),
            },
            UnOp::Not(_) => match &*inner {
                Value::Bool(b) => Ok(Cow::Owned(Value::from(!b))),
                val => Err(EvalErr::MismatchedTypes {
                    expected: Type::Bool,
                    found: val.ty(),
                }),
            },
        }
    }

    fn eval_expr<'a>(&self, ctx: &EvalCtx<'a, '_>, val: &'a Value) -> Result<Cow<'a, Value>, EvalErr> {
        match self {
            FilterExpr::Unary(op, inner) => {
                self.eval_unary(ctx, val, op, inner)
            }
            FilterExpr::Binary(lhs, op, rhs) => {
                let lhs = lhs.eval_expr(ctx, val)?;
                let rhs = rhs.eval_expr(ctx, val)?;

                match op {
                    BinOp::And(_) => {
                        let lhs = lhs.try_bool()?;
                        let rhs = rhs.try_bool()?;
                        Ok(Cow::Owned(Value::Bool(lhs && rhs)))
                    }
                    BinOp::Or(_) => {
                        let lhs = lhs.try_bool()?;
                        let rhs = rhs.try_bool()?;
                        Ok(Cow::Owned(Value::Bool(lhs || rhs)))
                    }

                    BinOp::Eq(_) => Ok(Cow::Owned(Value::Bool(lhs == rhs))),
                    BinOp::Le(_) => {
                        let lhs = lhs.try_f64()?;
                        let rhs = rhs.try_f64()?;

                        Ok(Cow::Owned(Value::Bool(lhs <= rhs)))
                    }
                    BinOp::Lt(_) => {
                        let lhs = lhs.try_f64()?;
                        let rhs = rhs.try_f64()?;

                        Ok(Cow::Owned(Value::Bool(lhs < rhs)))
                    }
                    BinOp::Gt(_) => {
                        let lhs = lhs.try_f64()?;
                        let rhs = rhs.try_f64()?;

                        Ok(Cow::Owned(Value::Bool(lhs > rhs)))
                    }
                    BinOp::Ge(_) => {
                        let lhs = lhs.try_f64()?;
                        let rhs = rhs.try_f64()?;

                        Ok(Cow::Owned(Value::Bool(lhs >= rhs)))
                    }

                    BinOp::Add(_) => {
                        if lhs.is_f64() && rhs.is_f64() {
                            let lhs = lhs.try_f64()?;
                            let rhs = rhs.try_f64()?;

                            Ok(Cow::Owned(Value::from(lhs + rhs)))
                        } else if lhs.is_i64() && rhs.is_i64() {
                            let lhs = lhs.try_i64()?;
                            let rhs = rhs.try_i64()?;

                            Ok(Cow::Owned(Value::from(lhs + rhs)))
                        } else if lhs.is_string() && rhs.is_string() {
                            let lhs = lhs.try_str()?;
                            let rhs = rhs.try_str()?;

                            Ok(Cow::Owned(Value::String(format!("{lhs}{rhs}"))))
                        } else {
                            let err = if lhs.ty() == rhs.ty() {
                                EvalErr::Unsupported("Adding non-numeric or non-string types".to_string())
                            } else {
                                EvalErr::Unsupported("Adding two values of different type".to_string())
                            };
                            Err(err)
                        }
                    }
                    BinOp::Sub(_) => {
                        let lhs = lhs.try_f64()?;
                        let rhs = rhs.try_f64()?;

                        Ok(Cow::Owned(Value::from(lhs - rhs)))
                    }
                    BinOp::Mul(_) => {
                        let lhs = lhs.try_f64()?;
                        let rhs = rhs.try_f64()?;

                        Ok(Cow::Owned(Value::from(lhs * rhs)))
                    }
                    BinOp::Div(_) => {
                        let lhs = lhs.try_f64()?;
                        let rhs = rhs.try_f64()?;

                        Ok(Cow::Owned(Value::from(lhs / rhs)))
                    }
                    BinOp::Rem(_) => {
                        let lhs = lhs.try_i64()?;
                        let rhs = rhs.try_i64()?;

                        Ok(Cow::Owned(Value::from(lhs % rhs)))
                    }
                }
            }
            FilterExpr::Path(path) => path.eval_expr(ctx, val),
            FilterExpr::Lit(lit) => Ok(Cow::Owned(match lit {
                ExprLit::Int(i) => Value::from(i.as_int()),
                ExprLit::String(s) => Value::from(s.as_str()),
                ExprLit::Bool(b) => Value::from(b.as_bool()),
                ExprLit::Null(_) => Value::Null,
            })),
            FilterExpr::Parens(_, inner) => inner.eval_expr(ctx, val),
        }
    }
}
