use marker_api::{
    ast::{
        expr::{
            ArrayExpr, AsExpr, AssignExpr, AwaitExpr, BinaryOpExpr, BinaryOpKind, BlockExpr,
            BoolLitExpr, BreakExpr, CallExpr, CaptureKind, CharLitExpr, ClosureExpr, ClosureParam,
            CommonExprData, ConstExpr, ContinueExpr, CtorExpr, CtorField, ExprKind, ExprPrecedence,
            FieldExpr, FloatLitExpr, FloatSuffix, ForExpr, IfExpr, IndexExpr, IntLitExpr,
            IntSuffix, LetExpr, LoopExpr, MatchArm, MatchExpr, MethodExpr, PathExpr,
            QuestionMarkExpr, RangeExpr, RefExpr, ReturnExpr, StrLitData, StrLitExpr, TupleExpr,
            UnaryOpExpr, UnaryOpKind, UnstableExpr, WhileExpr,
        },
        pat::PatKind,
        Ident, Safety, Syncness,
    },
    CtorBlocker,
};
use rustc_hash::FxHashMap;
use rustc_hir as hir;
use std::str::FromStr;

use super::MarkerConverterInner;

impl<'ast, 'tcx> MarkerConverterInner<'ast, 'tcx> {
    #[must_use]
    pub fn to_expr_from_block(&self, block: &hir::Block<'tcx>) -> ExprKind<'ast> {
        let id = self.to_expr_id(block.hir_id);
        if let Some(expr) = self.exprs.borrow().get(&id) {
            return *expr;
        }

        let data = CommonExprData::new(id, self.to_span_id(block.span));
        let expr = ExprKind::Block(self.alloc(self.to_block_expr(
            data,
            block,
            None,
            Syncness::Sync,
            CaptureKind::Default,
        )));

        self.exprs.borrow_mut().insert(id, expr);
        expr
    }

    #[must_use]
    pub fn to_exprs(&self, exprs: &[hir::Expr<'tcx>]) -> &'ast [ExprKind<'ast>] {
        self.alloc_slice(exprs.iter().map(|expr| self.to_expr(expr)))
    }

    #[must_use]
    pub fn to_expr(&self, expr: &hir::Expr<'tcx>) -> ExprKind<'ast> {
        let id = self.to_expr_id(expr.hir_id);
        if let Some(expr) = self.exprs.borrow().get(&id) {
            return *expr;
        }

        let data = CommonExprData::new(id, self.to_span_id(expr.span));
        let expr = match &expr.kind {
            hir::ExprKind::Lit(spanned_lit) => self.to_expr_from_lit_kind(data, &spanned_lit.node),
            hir::ExprKind::Binary(op, left, right) => {
                ExprKind::BinaryOp(self.alloc(BinaryOpExpr::new(
                    data,
                    self.to_expr(left),
                    self.to_expr(right),
                    self.to_bin_op_kind(op),
                )))
            }
            hir::ExprKind::Unary(op, expr) => ExprKind::UnaryOp(self.alloc(UnaryOpExpr::new(
                data,
                self.to_expr(expr),
                self.to_unary_op_kind(*op),
            ))),
            hir::ExprKind::AddrOf(_kind, muta, inner) => ExprKind::Ref(self.alloc(RefExpr::new(
                data,
                self.to_expr(inner),
                self.to_mutability(*muta),
            ))),
            hir::ExprKind::Block(block, label) => {
                let mut e = None;
                // if let-chains sadly break rustfmt for this method. This should
                // work well enough in the mean time
                if let [local, ..] = block.stmts {
                    if let hir::StmtKind::Local(local) = local.kind {
                        if let hir::LocalSource::AssignDesugar(_) = local.source {
                            e = Some(ExprKind::Assign(
                                self.alloc(self.to_assign_expr_from_desugar(block)),
                            ));
                        }
                    }
                }

                if let Some(e) = e {
                    e
                } else {
                    ExprKind::Block(self.alloc(self.to_block_expr(
                        data,
                        block,
                        *label,
                        Syncness::Sync,
                        CaptureKind::Default,
                    )))
                }
            }
            hir::ExprKind::Call(operand, args) => match &operand.kind {
                hir::ExprKind::Path(hir::QPath::LangItem(
                    hir::LangItem::RangeInclusiveNew,
                    _,
                    _,
                )) => ExprKind::Range(self.alloc({
                    RangeExpr::new(
                        data,
                        Some(self.to_expr(&args[0])),
                        Some(self.to_expr(&args[1])),
                        true,
                    )
                })),
                hir::ExprKind::Path(
                    qpath @ hir::QPath::Resolved(
                        None,
                        hir::Path {
                            // The correct def resolution is done by `to_qpath_from_expr`
                            res: hir::def::Res::Def(hir::def::DefKind::Ctor(_, _), _),
                            ..
                        },
                    ),
                ) => {
                    let fields = self.alloc_slice(args.iter().enumerate().map(|(index, expr)| {
                        CtorField::new(
                            self.to_span_id(expr.span),
                            Ident::new(
                                self.to_symbol_id_for_num(
                                    u32::try_from(index).expect("a index over 2^32 is unexpected"),
                                ),
                                self.to_span_id(rustc_span::DUMMY_SP),
                            ),
                            self.to_expr(expr),
                        )
                    }));
                    ExprKind::Ctor(self.alloc(CtorExpr::new(
                        data,
                        self.to_qpath_from_expr(qpath, expr),
                        fields,
                        None,
                    )))
                }

                _ => ExprKind::Call(self.alloc(CallExpr::new(
                    data,
                    self.to_expr(operand),
                    self.to_exprs(args),
                ))),
            },
            hir::ExprKind::MethodCall(method, receiver, args, _span) => {
                ExprKind::Method(self.alloc({
                    MethodExpr::new(
                        data,
                        self.to_expr(receiver),
                        self.to_path_segment(method),
                        self.to_exprs(args),
                    )
                }))
            }
            hir::ExprKind::Path(
                path @ hir::QPath::Resolved(
                    None,
                    hir::Path {
                        res: hir::def::Res::Def(hir::def::DefKind::Ctor(_, _), ..),
                        ..
                    },
                ),
            ) => ExprKind::Ctor(self.alloc(CtorExpr::new(
                data,
                self.to_qpath_from_expr(path, expr),
                &[],
                None,
            ))),
            hir::ExprKind::Path(qpath) => ExprKind::Path(
                self.alloc(PathExpr::new(data, self.to_qpath_from_expr(qpath, expr))),
            ),
            hir::ExprKind::Tup(exprs) => {
                ExprKind::Tuple(self.alloc(TupleExpr::new(data, self.to_exprs(exprs))))
            }
            hir::ExprKind::Array(exprs) => {
                ExprKind::Array(self.alloc(ArrayExpr::new(data, self.to_exprs(exprs), None)))
            }
            hir::ExprKind::Repeat(expr, hir::ArrayLen::Body(anon_const)) => {
                ExprKind::Array(self.alloc(ArrayExpr::new(
                    data,
                    self.alloc_slice([self.to_expr(expr)]),
                    Some(self.to_const_expr(*anon_const)),
                )))
            }
            hir::ExprKind::Struct(path, fields, base) => match path {
                hir::QPath::LangItem(hir::LangItem::RangeFull, _, _) => {
                    ExprKind::Range(self.alloc(RangeExpr::new(data, None, None, false)))
                }
                hir::QPath::LangItem(hir::LangItem::RangeFrom, _, _) => {
                    ExprKind::Range(self.alloc(RangeExpr::new(
                        data,
                        Some(self.to_expr(fields[0].expr)),
                        None,
                        false,
                    )))
                }
                hir::QPath::LangItem(hir::LangItem::RangeTo, _, _) => ExprKind::Range(self.alloc(
                    RangeExpr::new(data, None, Some(self.to_expr(fields[0].expr)), false),
                )),
                hir::QPath::LangItem(hir::LangItem::Range, _, _) => ExprKind::Range(self.alloc({
                    RangeExpr::new(
                        data,
                        Some(self.to_expr(fields[0].expr)),
                        Some(self.to_expr(fields[1].expr)),
                        false,
                    )
                })),
                hir::QPath::LangItem(hir::LangItem::RangeToInclusive, _, _) => {
                    ExprKind::Range(self.alloc(RangeExpr::new(
                        data,
                        None,
                        Some(self.to_expr(fields[0].expr)),
                        true,
                    )))
                }
                _ => {
                    let ctor_fields = self.alloc_slice(fields.iter().map(|field| {
                        CtorField::new(
                            self.to_span_id(field.span),
                            self.to_ident(field.ident),
                            self.to_expr(field.expr),
                        )
                    }));

                    ExprKind::Ctor(self.alloc({
                        CtorExpr::new(
                            data,
                            self.to_qpath_from_expr(path, expr),
                            ctor_fields,
                            base.map(|expr| self.to_expr(expr)),
                        )
                    }))
                }
            },
            hir::ExprKind::Index(operand, index) => ExprKind::Index(self.alloc(IndexExpr::new(
                data,
                self.to_expr(operand),
                self.to_expr(index),
            ))),
            hir::ExprKind::Field(operand, field) => ExprKind::Field(self.alloc(FieldExpr::new(
                data,
                self.to_expr(operand),
                self.to_ident(*field),
            ))),
            hir::ExprKind::If(scrutinee, then, els) => ExprKind::If(self.alloc(IfExpr::new(
                data,
                self.to_expr(scrutinee),
                self.to_expr(then),
                els.map(|els| self.to_expr(els)),
            ))),
            hir::ExprKind::Let(lets) => self.to_let_expr(lets, expr.hir_id),
            hir::ExprKind::Match(_scrutinee, _arms, hir::MatchSource::ForLoopDesugar) => {
                ExprKind::For(self.alloc(self.to_for_from_desugar(expr)))
            }
            hir::ExprKind::Match(
                scrutinee,
                arms,
                hir::MatchSource::Normal | hir::MatchSource::FormatArgs,
            ) => ExprKind::Match(self.alloc(MatchExpr::new(
                data,
                self.to_expr(scrutinee),
                self.to_match_arms(arms),
            ))),
            hir::ExprKind::Match(
                _scrutinee,
                [_early_return, _continue],
                hir::MatchSource::TryDesugar,
            ) => ExprKind::QuestionMark(self.alloc(self.to_try_expr_from_desugar(expr))),
            hir::ExprKind::Match(_scrutinee, [_awaitee_arm], hir::MatchSource::AwaitDesugar) => {
                ExprKind::Await(self.alloc(self.to_await_expr_from_desugar(expr)))
            }
            hir::ExprKind::Assign(assignee, value, _span) => {
                ExprKind::Assign(self.alloc(AssignExpr::new(
                    data,
                    PatKind::Place(self.to_expr(assignee), CtorBlocker::new()),
                    self.to_expr(value),
                    None,
                )))
            }
            hir::ExprKind::AssignOp(op, assignee, value) => {
                ExprKind::Assign(self.alloc(AssignExpr::new(
                    data,
                    PatKind::Place(self.to_expr(assignee), CtorBlocker::new()),
                    self.to_expr(value),
                    Some(self.to_bin_op_kind(op)),
                )))
            }
            hir::ExprKind::Break(dest, expr) => ExprKind::Break(self.alloc(BreakExpr::new(
                data,
                dest.label.map(|label| self.to_ident(label.ident)),
                self.to_expr_id(dest.target_id.expect("rustc would have errored")),
                expr.map(|expr| self.to_expr(expr)),
            ))),
            hir::ExprKind::Continue(dest) => ExprKind::Continue(self.alloc(ContinueExpr::new(
                data,
                dest.label.map(|label| self.to_ident(label.ident)),
                self.to_expr_id(dest.target_id.expect("rustc would have errored")),
            ))),
            hir::ExprKind::Ret(expr) => ExprKind::Return(
                self.alloc(ReturnExpr::new(data, expr.map(|expr| self.to_expr(expr)))),
            ),
            hir::ExprKind::Loop(block, label, source, _span) => match source {
                hir::LoopSource::Loop => ExprKind::Loop(self.alloc(LoopExpr::new(
                    data,
                    label.map(|label| self.to_ident(label.ident)),
                    self.to_expr_from_block(block),
                ))),
                hir::LoopSource::While => {
                    ExprKind::While(self.alloc(self.to_while_loop_from_desugar(expr)))
                }
                hir::LoopSource::ForLoop => unreachable!("is desugared at a higher node level"),
            },
            hir::ExprKind::Closure(closure) => self.to_expr_from_closure(data, expr, closure),
            hir::ExprKind::Cast(expr, ty) => {
                ExprKind::As(self.alloc(AsExpr::new(data, self.to_expr(expr), self.to_syn_ty(ty))))
            }
            // `DropTemps` is an rustc internal construct to tweak the drop
            // order during HIR lowering. Marker can for now ignore this and
            // convert the inner expression directly
            hir::ExprKind::DropTemps(inner) => return self.to_expr(inner),
            hir::ExprKind::Err(..) => unreachable!("would have triggered a rustc error"),
            _ => {
                eprintln!("skipping not implemented expr at: {:?}", expr.span);
                ExprKind::Unstable(self.alloc({
                    UnstableExpr::new(
                        data,
                        ExprPrecedence::Unstable(i32::from(expr.precedence().order())),
                    )
                }))
            }
        };

        // Here `expr.id()` has to be used as the key, as some desugar expressions
        // use a different id, than the one stored in the local variable `id`
        self.exprs.borrow_mut().insert(expr.id(), expr);
        expr
    }

    #[must_use]
    fn to_block_expr(
        &self,
        data: CommonExprData<'ast>,
        block: &hir::Block<'tcx>,
        label: Option<rustc_ast::Label>,
        syncness: Syncness,
        capture_kind: CaptureKind,
    ) -> BlockExpr<'ast> {
        let stmts: Vec<_> = block
            .stmts
            .iter()
            .filter_map(|stmt| self.to_stmt(stmt))
            .collect();
        let stmts = self.alloc_slice(stmts);
        let safety = match block.rules {
            hir::BlockCheckMode::DefaultBlock => Safety::Safe,
            hir::BlockCheckMode::UnsafeBlock(_) => Safety::Unsafe,
        };
        BlockExpr::new(
            data,
            stmts,
            block.expr.map(|expr| self.to_expr(expr)),
            label.map(|label| self.to_ident(label.ident)),
            safety,
            syncness,
            capture_kind,
        )
    }

    #[must_use]
    fn to_expr_from_lit_kind(
        &self,
        data: CommonExprData<'ast>,
        lit_kind: &rustc_ast::LitKind,
    ) -> ExprKind<'ast> {
        match &lit_kind {
            rustc_ast::LitKind::Str(sym, kind) => ExprKind::StrLit(self.alloc({
                StrLitExpr::new(
                    data,
                    matches!(kind, rustc_ast::StrStyle::Raw(_)),
                    StrLitData::Sym(self.to_symbol_id(*sym)),
                )
            })),
            rustc_ast::LitKind::ByteStr(bytes, kind) => ExprKind::StrLit(self.alloc({
                StrLitExpr::new(
                    data,
                    matches!(kind, rustc_ast::StrStyle::Raw(_)),
                    StrLitData::Bytes(self.alloc_slice(bytes.iter().copied()).into()),
                )
            })),
            // Still unstable see: https://github.com/rust-lang/rust/issues/105723
            rustc_ast::LitKind::CStr(_, _) => {
                ExprKind::Unstable(self.alloc(UnstableExpr::new(data, ExprPrecedence::Lit)))
            }
            rustc_ast::LitKind::Byte(value) => {
                ExprKind::IntLit(self.alloc(IntLitExpr::new(data, u128::from(*value), None)))
            }
            rustc_ast::LitKind::Char(value) => {
                ExprKind::CharLit(self.alloc(CharLitExpr::new(data, *value)))
            }
            rustc_ast::LitKind::Int(value, kind) => {
                let suffix = match kind {
                    rustc_ast::LitIntType::Signed(rustc_ast::IntTy::Isize) => {
                        Some(IntSuffix::Isize)
                    }
                    rustc_ast::LitIntType::Signed(rustc_ast::IntTy::I8) => Some(IntSuffix::I8),
                    rustc_ast::LitIntType::Signed(rustc_ast::IntTy::I16) => Some(IntSuffix::I16),
                    rustc_ast::LitIntType::Signed(rustc_ast::IntTy::I32) => Some(IntSuffix::I32),
                    rustc_ast::LitIntType::Signed(rustc_ast::IntTy::I64) => Some(IntSuffix::I64),
                    rustc_ast::LitIntType::Signed(rustc_ast::IntTy::I128) => Some(IntSuffix::I128),
                    rustc_ast::LitIntType::Unsigned(rustc_ast::UintTy::Usize) => {
                        Some(IntSuffix::Usize)
                    }
                    rustc_ast::LitIntType::Unsigned(rustc_ast::UintTy::U8) => Some(IntSuffix::U8),
                    rustc_ast::LitIntType::Unsigned(rustc_ast::UintTy::U16) => Some(IntSuffix::U16),
                    rustc_ast::LitIntType::Unsigned(rustc_ast::UintTy::U32) => Some(IntSuffix::U32),
                    rustc_ast::LitIntType::Unsigned(rustc_ast::UintTy::U64) => Some(IntSuffix::U64),
                    rustc_ast::LitIntType::Unsigned(rustc_ast::UintTy::U128) => {
                        Some(IntSuffix::U128)
                    }
                    rustc_ast::LitIntType::Unsuffixed => None,
                };
                ExprKind::IntLit(self.alloc(IntLitExpr::new(data, *value, suffix)))
            }
            rustc_ast::LitKind::Float(lit_sym, kind) => {
                let suffix = match kind {
                    rustc_ast::LitFloatType::Suffixed(rustc_ast::ast::FloatTy::F32) => {
                        Some(FloatSuffix::F32)
                    }
                    rustc_ast::LitFloatType::Suffixed(rustc_ast::ast::FloatTy::F64) => {
                        Some(FloatSuffix::F64)
                    }
                    rustc_ast::LitFloatType::Unsuffixed => None,
                };
                let value = f64::from_str(lit_sym.as_str())
                    .expect("rustc should have validated the literal");
                ExprKind::FloatLit(self.alloc(FloatLitExpr::new(data, value, suffix)))
            }
            rustc_ast::LitKind::Bool(value) => {
                ExprKind::BoolLit(self.alloc(BoolLitExpr::new(data, *value)))
            }
            rustc_ast::LitKind::Err => unreachable!("would have triggered a rustc error"),
        }
    }

    #[must_use]
    fn to_bin_op_kind(&self, op: &hir::BinOp) -> BinaryOpKind {
        match op.node {
            hir::BinOpKind::Add => BinaryOpKind::Add,
            hir::BinOpKind::Sub => BinaryOpKind::Sub,
            hir::BinOpKind::Mul => BinaryOpKind::Mul,
            hir::BinOpKind::Div => BinaryOpKind::Div,
            hir::BinOpKind::Rem => BinaryOpKind::Rem,
            hir::BinOpKind::And => BinaryOpKind::And,
            hir::BinOpKind::Or => BinaryOpKind::Or,
            hir::BinOpKind::BitXor => BinaryOpKind::BitXor,
            hir::BinOpKind::BitAnd => BinaryOpKind::BitAnd,
            hir::BinOpKind::BitOr => BinaryOpKind::BitOr,
            hir::BinOpKind::Shl => BinaryOpKind::Shl,
            hir::BinOpKind::Shr => BinaryOpKind::Shr,
            hir::BinOpKind::Eq => BinaryOpKind::Eq,
            hir::BinOpKind::Lt => BinaryOpKind::Lesser,
            hir::BinOpKind::Le => BinaryOpKind::LesserEq,
            hir::BinOpKind::Ne => BinaryOpKind::NotEq,
            hir::BinOpKind::Ge => BinaryOpKind::GreaterEq,
            hir::BinOpKind::Gt => BinaryOpKind::Greater,
        }
    }

    #[must_use]
    fn to_unary_op_kind(&self, op: hir::UnOp) -> UnaryOpKind {
        match op {
            hir::UnOp::Neg => UnaryOpKind::Neg,
            hir::UnOp::Not => UnaryOpKind::Not,
            hir::UnOp::Deref => UnaryOpKind::Deref,
        }
    }

    #[must_use]
    fn to_match_arms(&self, arms: &[hir::Arm<'tcx>]) -> &'ast [MatchArm<'ast>] {
        self.alloc_slice(arms.iter().map(|arm| self.to_match_arm(arm)))
    }

    #[must_use]
    fn to_match_arm(&self, arm: &hir::Arm<'tcx>) -> MatchArm<'ast> {
        let guard = match &arm.guard {
            Some(hir::Guard::If(expr)) => Some(self.to_expr(expr)),
            Some(hir::Guard::IfLet(lets)) => Some(self.to_let_expr(lets, arm.hir_id)),
            None => None,
        };
        MatchArm::new(
            self.to_span_id(arm.span),
            self.to_pat(arm.pat),
            guard,
            self.to_expr(arm.body),
        )
    }

    fn to_expr_from_closure(
        &self,
        data: CommonExprData<'ast>,
        _expr: &hir::Expr<'tcx>,
        closure: &hir::Closure<'tcx>,
    ) -> ExprKind<'ast> {
        let body_id = closure.body;
        let body = self.rustc_cx.hir().body(body_id);
        match body.generator_kind {
            Some(hir::GeneratorKind::Async(hir::AsyncGeneratorKind::Fn)) => {
                if let hir::ExprKind::Block(block, None) = body.value.kind
                    && let Some(temp_drop) = block.expr
                    && let hir::ExprKind::DropTemps(inner_block) = temp_drop.kind
                {
                    return self.with_body(body_id, || self.to_expr(inner_block));
                }

                unreachable!("`async fn` body desugar always has the same structure")
            }
            Some(hir::GeneratorKind::Async(hir::AsyncGeneratorKind::Block)) => {
                let block_expr = body.value;
                if let hir::ExprKind::Block(block, None) = block_expr.kind {
                    let api_block_expr = self.with_body(body_id, || {
                        self.to_block_expr(
                            CommonExprData::new(
                                self.to_expr_id(block_expr.hir_id),
                                self.to_span_id(block_expr.span),
                            ),
                            block,
                            None,
                            Syncness::Async,
                            self.to_capture_kind(closure.capture_clause),
                        )
                    });
                    return ExprKind::Block(self.alloc(api_block_expr));
                }
                unreachable!("`async` block desugar always has the same structure")
            }
            Some(
                hir::GeneratorKind::Async(hir::AsyncGeneratorKind::Closure)
                | hir::GeneratorKind::Gen,
            ) => ExprKind::Unstable(self.alloc(UnstableExpr::new(data, ExprPrecedence::Closure))),
            None => ExprKind::Closure(self.alloc(self.to_closure_expr(data, closure))),
        }
    }

    fn to_closure_expr(
        &self,
        data: CommonExprData<'ast>,
        closure: &hir::Closure<'tcx>,
    ) -> ClosureExpr<'ast> {
        let fn_decl = closure.fn_decl;

        let body_id = closure.body;
        let body = self.rustc_cx.hir().body(body_id);
        let params =
            self.with_body(body_id, || {
                self.alloc_slice(body.params.iter().zip(fn_decl.inputs.iter()).map(
                    |(param, ty)| {
                        // Rustc automatically substitutes the infer type, if a closure
                        // parameter has no type declaration.
                        let param_ty = if matches!(ty.kind, hir::TyKind::Infer)
                            && param.pat.span.contains(ty.span)
                        {
                            None
                        } else {
                            Some(self.to_syn_ty(ty))
                        };
                        ClosureParam::new(
                            self.to_span_id(param.span),
                            self.to_pat(param.pat),
                            param_ty,
                        )
                    },
                ))
            });

        let return_ty = if let hir::FnRetTy::Return(rust_ty) = fn_decl.output {
            Some(self.to_syn_ty(rust_ty))
        } else {
            None
        };

        ClosureExpr::new(
            data,
            self.to_capture_kind(closure.capture_clause),
            params,
            return_ty,
            self.to_body_id(closure.body),
        )
    }

    fn to_capture_kind(&self, capture: hir::CaptureBy) -> CaptureKind {
        match capture {
            rustc_ast::CaptureBy::Value => CaptureKind::Move,
            rustc_ast::CaptureBy::Ref => CaptureKind::Default,
        }
    }

    #[must_use]
    fn to_let_expr(&self, lets: &hir::Let<'tcx>, id: hir::HirId) -> ExprKind<'ast> {
        let data = CommonExprData::new(self.to_expr_id(id), self.to_span_id(lets.span));
        ExprKind::Let(self.alloc(LetExpr::new(
            data,
            self.to_pat(lets.pat),
            self.to_expr(lets.init),
        )))
    }

    /// Rustc desugars assignments with tuples, arrays and structs in the assignee as a block, which
    /// consists of a `let` statement, that assigns the value expression to temporary variables
    /// called `lhs` which are then assigned to the appropriate local variables
    ///
    /// The "Show HIR" option on the [Playground] is a great resource to understand how this
    /// desugaring works. Here is a simple example to illustrate the current desugar:
    ///
    /// ```
    /// # let mut a = 0;
    /// # let mut b = 0;
    /// // This expression
    /// [a, b] = [1, 2];
    /// // Is desugared to:
    /// {
    ///     let [lhs, lhs] = [1, 2];
    ///     a = lhs;
    ///     b = lhs;
    /// };
    /// // Note that both `lhs` have different IDs
    /// ```
    ///
    /// [Playground]: <https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=aea16a442e31ca5e7bed1040e8960d4e>
    #[must_use]
    fn to_assign_expr_from_desugar(&self, block: &hir::Block<'tcx>) -> AssignExpr<'ast> {
        let lhs_map: FxHashMap<_, _> = block
            .stmts
            .iter()
            .skip(1)
            .map(|stmt| {
                if let hir::StmtKind::Expr(expr) = stmt.kind
                    && let hir::ExprKind::Assign(assign_expr, value, _span) = expr.kind
                    && let hir::ExprKind::Path(hir::QPath::Resolved(None, path)) = value.kind
                    && let hir::def::Res::Local(local_id) = path.res
                {
                    (local_id, self.to_expr(assign_expr))
                } else {
                    unreachable!("unexpected statement while resugaring {stmt:?}")
                }
            })
            .collect();
        if let [local, ..] = block.stmts
            && let hir::StmtKind::Local(local) = local.kind
            && let hir::LocalSource::AssignDesugar(_) = local.source
        {
            AssignExpr::new(
                CommonExprData::new(self.to_expr_id(local.hir_id), self.to_span_id(local.span)),
                self.to_pat_with_hls(local.pat, &lhs_map),
                self.to_expr(local.init.unwrap()),
                None
            )
        } else {
            unreachable!("assignment expr desugar always has a local as the first statement")
        }
    }

    fn to_try_expr_from_desugar(&self, try_desugar: &hir::Expr<'tcx>) -> QuestionMarkExpr<'ast> {
        if let hir::ExprKind::Match(scrutinee, [_ret, _con], hir::MatchSource::TryDesugar) =
            try_desugar.kind
        {
            if let hir::ExprKind::Call(_try_path, [tested_expr]) = scrutinee.kind {
                return QuestionMarkExpr::new(
                    CommonExprData::new(
                        self.to_expr_id(try_desugar.hir_id),
                        self.to_span_id(try_desugar.span),
                    ),
                    self.to_expr(tested_expr),
                );
            }
        }

        unreachable!("try desugar always has the same structure")
    }

    /// The "Show HIR" option on the [Playground] is a great resource to understand how this
    /// desugaring works. Here is a simple example to illustrate the current desugar:
    ///
    /// ```
    /// # let mut a = 0;
    /// # let cond = false;
    /// // This expression
    /// while cond {
    ///     a += 1;
    /// }
    /// // Is desugared to:
    /// loop {
    ///     if cond {
    ///         a += 1;
    ///     } else {
    ///         break;
    ///     }
    /// }
    /// ```
    ///
    /// [Playground]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=b642324278a27a71e80720f24b29d7ee
    #[must_use]
    fn to_while_loop_from_desugar(&self, loop_expr: &hir::Expr<'tcx>) -> WhileExpr<'ast> {
        if let hir::ExprKind::Loop(block, label, hir::LoopSource::While, _loop_head) =
            loop_expr.kind
        {
            if let Some(expr) = block.expr {
                if let hir::ExprKind::If(cond, then, Some(_)) = expr.kind {
                    let data = CommonExprData::new(
                        self.to_expr_id(loop_expr.hir_id),
                        self.to_span_id(loop_expr.span),
                    );
                    return WhileExpr::new(
                        data,
                        label.map(|label| self.to_ident(label.ident)),
                        self.to_expr(cond),
                        self.to_expr(then),
                    );
                }
            }
        }

        unreachable!("while loop desugar always has the same structure")
    }

    /// The "Show HIR" option on the [Playground] is a great resource to understand how this
    /// desugaring works. Here is a simple example to illustrate the current desugar:
    ///
    /// ```
    /// # let range = 0..10;
    /// // This expression
    /// for _ in range { /* body */ }
    /// // Is desugared to:
    /// match IntoIterator::into_iter(range) {
    ///     mut iter => loop {
    ///         match Iterator::next(&mut iter) {
    ///             None => break,
    ///             Some(_) => { /* body */ }
    ///         }
    ///     },
    /// };
    /// ```
    ///
    /// [Playground]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=9f11727fd0d9124ca1434936b745d495
    #[must_use]
    fn to_for_from_desugar(&self, into_match: &hir::Expr<'tcx>) -> ForExpr<'ast> {
        if let hir::ExprKind::Match(into_scrutinee, [mut_iter_arm], hir::MatchSource::ForLoopDesugar) = into_match.kind
            && let hir::ExprKind::Call(_into_iter_path, [iter_expr]) = &into_scrutinee.kind
            && let loop_expr = mut_iter_arm.body
            && let hir::ExprKind::Loop(block, label, hir::LoopSource::ForLoop, _loop_head) = loop_expr.kind
            && let [stmt] = block.stmts
            && let hir::StmtKind::Expr(none_some_match) = stmt.kind
            && let hir::ExprKind::Match(_, [_none, some_arm], hir::MatchSource::ForLoopDesugar) = none_some_match.kind
            && let hir::PatKind::Struct(_some, [field], false) = &some_arm.pat.kind
        {
            let pat = self.to_pat(field.pat);
            let iter_expr = self.to_expr(iter_expr);
            let body = self.to_expr(some_arm.body);
            let data = CommonExprData::new(
                self.to_expr_id(loop_expr.hir_id),
                self.to_resugared_span_id(into_match.span)
            );
            return ForExpr::new(
                data,
                label.map(|label| self.to_ident(label.ident)),
                pat,
                iter_expr,
                body
            );
        }

        unreachable!("for loop desugar always has the same structure")
    }

    /// The "Show HIR" option on the [Playground] is a great resource to understand how this
    /// desugaring works. This desugar looks super scary and it is, but luckily, marker only
    /// needs to extract the argument for the `IntoFuture::into_future(<arg>)` call.
    ///
    /// ```ignore
    /// # async fn foo() -> u8 {
    /// #     16
    /// # }
    /// # async fn bar() -> u8 {
    ///     foo().await;
    /// # }
    ///
    /// # async fn bar() -> u8 {
    ///     match IntoFuture::into_future(foo()) {
    ///         mut __awaitee => loop {
    ///             match unsafe {
    ///                 core::future::poll(core::pin::Pin::new_unchecked(&mut __awaitee))
    ///                 core::future::get_context(_task_context)
    ///             } {
    ///                 core::task::pool::Pool::Ready {  0: result } => break result,
    ///                 core::task::pool::Pool::Pending {} => {},
    ///             }
    ///             _task_context = yield ()
    ///         }
    ///     }
    /// # }
    /// ```
    ///
    /// [Playground]: <https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=9589cb3ee8264bace959c3dbd9759d98>
    #[must_use]
    fn to_await_expr_from_desugar(&self, await_expr: &hir::Expr<'tcx>) -> AwaitExpr<'ast> {
        if let hir::ExprKind::Match(into_scrutinee, [_awaitee_arm], hir::MatchSource::AwaitDesugar) = await_expr.kind
            && let hir::ExprKind::Call(_into_future_path, [future_expr]) = &into_scrutinee.kind
        {
            return AwaitExpr::new(
                CommonExprData::new(
                    self.to_expr_id(await_expr.hir_id),
                    self.to_span_id(await_expr.span),
                ),
                self.to_expr(future_expr),
            );
        }

        unreachable!("await desugar always has the same structure")
    }

    pub fn to_const_expr(&self, anon: hir::AnonConst) -> ConstExpr<'ast> {
        let body = self.rustc_cx.hir().body(anon.body);
        self.with_body(body.id(), || ConstExpr::new(self.to_expr(body.value)))
    }
}
