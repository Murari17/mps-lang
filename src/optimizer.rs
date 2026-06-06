use crate::ast::{Program, Stmt, Expr, Literal, BinOp, MatchCase, FStringPart};

pub struct Optimizer;

impl Optimizer {
    pub fn optimize(program: Program) -> Program {
        Program {
            statements: Self::optimize_block(program.statements),
        }
    }

    fn optimize_block(stmts: Vec<Stmt>) -> Vec<Stmt> {
        let mut optimized = Vec::new();
        
        // First, recursively optimize all individual statements in the block.
        let mut folded_stmts = Vec::new();
        for stmt in stmts {
            folded_stmts.push(Self::fold_statement(stmt));
        }

        // Second, perform dead-code elimination in this block.
        // We will scan for unused local variable declarations.
        let mut var_references = std::collections::HashMap::new();
        
        // Scan for references to all variables
        for stmt in &folded_stmts {
            Self::find_var_references(stmt, &mut var_references);
        }

        for stmt in folded_stmts {
            let is_terminator = matches!(stmt, Stmt::ReturnStmt(_) | Stmt::RaiseStmt(_));
            match stmt {
                Stmt::IfStmt { condition, then_branch, else_branch } => {
                    match condition {
                        Expr::Literal(Literal::Bool(true)) => {
                            // Inline then_branch directly
                            optimized.extend(then_branch);
                        }
                        Expr::Literal(Literal::Bool(false)) => {
                            // Inline else_branch if it exists
                            if let Some(eb) = else_branch {
                                optimized.extend(eb);
                            }
                        }
                        other_cond => {
                            optimized.push(Stmt::IfStmt {
                                condition: other_cond,
                                then_branch,
                                else_branch,
                            });
                        }
                    }
                }
                Stmt::WhileStmt { condition, body } => {
                    match condition {
                        Expr::Literal(Literal::Bool(false)) => {
                            // Eliminate loop completely!
                        }
                        other_cond => {
                            optimized.push(Stmt::WhileStmt {
                                condition: other_cond,
                                body,
                            });
                        }
                    }
                }
                Stmt::VariableDecl { name, is_const, var_type, init } => {
                    // Check if variable is unused and pure
                    let is_unused = !var_references.contains_key(&name);
                    let is_pure = match &init {
                        Some(expr) => Self::is_pure_expr(expr),
                        None => true,
                    };

                    if is_unused && is_pure {
                        // Eliminate unused, pure variable declaration!
                    } else {
                        optimized.push(Stmt::VariableDecl {
                            name,
                            is_const,
                            var_type,
                            init,
                        });
                    }
                }
                other => optimized.push(other),
            }

            if is_terminator {
                break;
            }
        }

        optimized
    }

    fn fold_statement(stmt: Stmt) -> Stmt {
        match stmt {
            Stmt::FunctionDecl { name, type_params, params, return_type, body, is_async, decorators } => {
                Stmt::FunctionDecl {
                    name,
                    type_params,
                    params,
                    return_type,
                    body: Self::optimize_block(body),
                    is_async,
                    decorators,
                }
            }
            Stmt::ClassDecl { name, base_class, members } => {
                Stmt::ClassDecl {
                    name,
                    base_class,
                    members: Self::optimize_block(members),
                }
            }
            Stmt::TraitDecl { name, methods } => {
                Stmt::TraitDecl { name, methods }
            }
            Stmt::VariableDecl { name, is_const, var_type, init } => {
                Stmt::VariableDecl {
                    name,
                    is_const,
                    var_type,
                    init: init.map(|e| Self::fold_expression(e)),
                }
            }
            Stmt::AssignStmt { lhs, value } => {
                Stmt::AssignStmt {
                    lhs: Self::fold_expression(lhs),
                    value: Self::fold_expression(value),
                }
            }
            Stmt::IfStmt { condition, then_branch, else_branch } => {
                Stmt::IfStmt {
                    condition: Self::fold_expression(condition),
                    then_branch: Self::optimize_block(then_branch),
                    else_branch: else_branch.map(|eb| Self::optimize_block(eb)),
                }
            }
            Stmt::WhileStmt { condition, body } => {
                Stmt::WhileStmt {
                    condition: Self::fold_expression(condition),
                    body: Self::optimize_block(body),
                }
            }
            Stmt::ForStmt { var_name, iterable, body } => {
                Stmt::ForStmt {
                    var_name,
                    iterable: Self::fold_expression(iterable),
                    body: Self::optimize_block(body),
                }
            }
            Stmt::TryCatchStmt { try_branch, catch_var, catch_branch, finally_branch } => {
                Stmt::TryCatchStmt {
                    try_branch: Self::optimize_block(try_branch),
                    catch_var,
                    catch_branch: Self::optimize_block(catch_branch),
                    finally_branch: finally_branch.map(|fb| Self::optimize_block(fb)),
                }
            }
            Stmt::RaiseStmt(expr) => Stmt::RaiseStmt(Self::fold_expression(expr)),
            Stmt::MatchStmt { value, cases } => {
                Stmt::MatchStmt {
                    value: Self::fold_expression(value),
                    cases: cases.into_iter().map(|c| MatchCase {
                        pattern: c.pattern,
                        body: Self::optimize_block(c.body),
                    }).collect(),
                }
            }
            Stmt::TupleUnpack { vars, init } => {
                Stmt::TupleUnpack {
                    vars,
                    init: Self::fold_expression(init),
                }
            }
            Stmt::ExprStmt(expr) => Stmt::ExprStmt(Self::fold_expression(expr)),
            Stmt::ReturnStmt(opt_expr) => Stmt::ReturnStmt(opt_expr.map(|e| Self::fold_expression(e))),
            other => other,
        }
    }

    pub fn fold_expression(expr: Expr) -> Expr {
        match expr {
            Expr::Binary { op, left, right } => {
                let left_folded = Self::fold_expression(*left);
                let right_folded = Self::fold_expression(*right);

                match (&left_folded, &right_folded) {
                    (Expr::Literal(Literal::Int(a)), Expr::Literal(Literal::Int(b))) => {
                        match op {
                            BinOp::Add => Expr::Literal(Literal::Int(a + b)),
                            BinOp::Sub => Expr::Literal(Literal::Int(a - b)),
                            BinOp::Mul => Expr::Literal(Literal::Int(a * b)),
                            BinOp::Div => {
                                if *b != 0 {
                                    Expr::Literal(Literal::Int(a / b))
                                } else {
                                    Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                                }
                            }
                            BinOp::Percent => {
                                if *b != 0 {
                                    Expr::Literal(Literal::Int(a % b))
                                } else {
                                    Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                                }
                            }
                            BinOp::Eq => Expr::Literal(Literal::Bool(a == b)),
                            BinOp::Ne => Expr::Literal(Literal::Bool(a != b)),
                            BinOp::Lt => Expr::Literal(Literal::Bool(a < b)),
                            BinOp::Le => Expr::Literal(Literal::Bool(a <= b)),
                            BinOp::Gt => Expr::Literal(Literal::Bool(a > b)),
                            BinOp::Ge => Expr::Literal(Literal::Bool(a >= b)),
                            BinOp::Pow | BinOp::And | BinOp::Or => {
                                Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                            }
                        }
                    }
                    (Expr::Literal(Literal::Float(a)), Expr::Literal(Literal::Float(b))) => {
                        match op {
                            BinOp::Add => Expr::Literal(Literal::Float(a + b)),
                            BinOp::Sub => Expr::Literal(Literal::Float(a - b)),
                            BinOp::Mul => Expr::Literal(Literal::Float(a * b)),
                            BinOp::Div => {
                                if *b != 0.0 {
                                    Expr::Literal(Literal::Float(a / b))
                                } else {
                                    Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                                }
                            }
                            BinOp::Percent => {
                                if *b != 0.0 {
                                    Expr::Literal(Literal::Float(a % b))
                                } else {
                                    Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                                }
                            }
                            BinOp::Eq => Expr::Literal(Literal::Bool(a == b)),
                            BinOp::Ne => Expr::Literal(Literal::Bool(a != b)),
                            BinOp::Lt => Expr::Literal(Literal::Bool(a < b)),
                            BinOp::Le => Expr::Literal(Literal::Bool(a <= b)),
                            BinOp::Gt => Expr::Literal(Literal::Bool(a > b)),
                            BinOp::Ge => Expr::Literal(Literal::Bool(a >= b)),
                            BinOp::Pow | BinOp::And | BinOp::Or => {
                                Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                            }
                        }
                    }
                    (Expr::Literal(Literal::Int(a)), Expr::Literal(Literal::Float(b))) => {
                        let af = *a as f64;
                        match op {
                            BinOp::Add => Expr::Literal(Literal::Float(af + b)),
                            BinOp::Sub => Expr::Literal(Literal::Float(af - b)),
                            BinOp::Mul => Expr::Literal(Literal::Float(af * b)),
                            BinOp::Div => {
                                if *b != 0.0 {
                                    Expr::Literal(Literal::Float(af / b))
                                } else {
                                    Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                                }
                            }
                            BinOp::Percent => {
                                if *b != 0.0 {
                                    Expr::Literal(Literal::Float(af % b))
                                } else {
                                    Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                                }
                            }
                            BinOp::Eq => Expr::Literal(Literal::Bool(af == *b)),
                            BinOp::Ne => Expr::Literal(Literal::Bool(af != *b)),
                            BinOp::Lt => Expr::Literal(Literal::Bool(af < *b)),
                            BinOp::Le => Expr::Literal(Literal::Bool(af <= *b)),
                            BinOp::Gt => Expr::Literal(Literal::Bool(af > *b)),
                            BinOp::Ge => Expr::Literal(Literal::Bool(af >= *b)),
                            BinOp::Pow | BinOp::And | BinOp::Or => {
                                Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                            }
                        }
                    }
                    (Expr::Literal(Literal::Float(a)), Expr::Literal(Literal::Int(b))) => {
                        let bf = *b as f64;
                        match op {
                            BinOp::Add => Expr::Literal(Literal::Float(a + bf)),
                            BinOp::Sub => Expr::Literal(Literal::Float(a - bf)),
                            BinOp::Mul => Expr::Literal(Literal::Float(a * bf)),
                            BinOp::Div => {
                                if bf != 0.0 {
                                    Expr::Literal(Literal::Float(a / bf))
                                } else {
                                    Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                                }
                            }
                            BinOp::Percent => {
                                if bf != 0.0 {
                                    Expr::Literal(Literal::Float(a % bf))
                                } else {
                                    Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                                }
                            }
                            BinOp::Eq => Expr::Literal(Literal::Bool(*a == bf)),
                            BinOp::Ne => Expr::Literal(Literal::Bool(*a != bf)),
                            BinOp::Lt => Expr::Literal(Literal::Bool(*a < bf)),
                            BinOp::Le => Expr::Literal(Literal::Bool(*a <= bf)),
                            BinOp::Gt => Expr::Literal(Literal::Bool(*a > bf)),
                            BinOp::Ge => Expr::Literal(Literal::Bool(*a >= bf)),
                            BinOp::Pow | BinOp::And | BinOp::Or => {
                                Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                            }
                        }
                    }
                    (Expr::Literal(Literal::Bool(a)), Expr::Literal(Literal::Bool(b))) => {
                        match op {
                            BinOp::Eq => Expr::Literal(Literal::Bool(a == b)),
                            BinOp::Ne => Expr::Literal(Literal::Bool(a != b)),
                            _ => Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                        }
                    }
                    _ => Expr::Binary { op, left: Box::new(left_folded), right: Box::new(right_folded) }
                }
            }
            Expr::Call { name, type_args, args } => {
                Expr::Call {
                    name,
                    type_args,
                    args: args.into_iter().map(|arg| Self::fold_expression(arg)).collect(),
                }
            }
            Expr::MemberAccess { object, member } => {
                Expr::MemberAccess {
                    object: Box::new(Self::fold_expression(*object)),
                    member,
                }
            }
            Expr::MemberCall { object, method, args } => {
                Expr::MemberCall {
                    object: Box::new(Self::fold_expression(*object)),
                    method,
                    args: args.into_iter().map(|arg| Self::fold_expression(arg)).collect(),
                }
            }
            Expr::Subscript { object, index } => {
                Expr::Subscript {
                    object: Box::new(Self::fold_expression(*object)),
                    index: Box::new(Self::fold_expression(*index)),
                }
            }
            Expr::ListLiteral(elements) => {
                Expr::ListLiteral(elements.into_iter().map(|e| Self::fold_expression(e)).collect())
            }
            Expr::DictLiteral(pairs) => {
                Expr::DictLiteral(pairs.into_iter().map(|(k, v)| (Self::fold_expression(k), Self::fold_expression(v))).collect())
            }
            Expr::TupleLiteral(elements) => {
                Expr::TupleLiteral(elements.into_iter().map(|e| Self::fold_expression(e)).collect())
            }
            Expr::SuperCall { method, args } => {
                Expr::SuperCall {
                    method,
                    args: args.into_iter().map(|e| Self::fold_expression(e)).collect(),
                }
            }
            Expr::Lambda { params, return_type, body } => {
                Expr::Lambda {
                    params,
                    return_type,
                    body: Box::new(Self::fold_expression(*body)),
                }
            }
            Expr::AwaitExpr(expr) => {
                Expr::AwaitExpr(Box::new(Self::fold_expression(*expr)))
            }
            Expr::Unary { op, operand } => {
                Expr::Unary {
                    op,
                    operand: Box::new(Self::fold_expression(*operand)),
                }
            }
            Expr::OptionalMemberAccess { object, member } => {
                Expr::OptionalMemberAccess {
                    object: Box::new(Self::fold_expression(*object)),
                    member,
                }
            }
            Expr::OptionalMemberCall { object, method, args } => {
                Expr::OptionalMemberCall {
                    object: Box::new(Self::fold_expression(*object)),
                    method,
                    args: args.into_iter().map(|arg| Self::fold_expression(arg)).collect(),
                }
            }
            Expr::FString { parts } => {
                Expr::FString {
                    parts: parts.into_iter().map(|part| match part {
                        FStringPart::Text(t) => FStringPart::Text(t),
                        FStringPart::Expr(e) => FStringPart::Expr(Box::new(Self::fold_expression(*e))),
                    }).collect(),
                }
            }
            Expr::Slice { object, start, end } => {
                Expr::Slice {
                    object: Box::new(Self::fold_expression(*object)),
                    start: start.map(|e| Box::new(Self::fold_expression(*e))),
                    end: end.map(|e| Box::new(Self::fold_expression(*e))),
                }
            }
            Expr::ListComprehension { element, var_name, iterable } => {
                Expr::ListComprehension {
                    element: Box::new(Self::fold_expression(*element)),
                    var_name,
                    iterable: Box::new(Self::fold_expression(*iterable)),
                }
            }
            other => other,
        }
    }

    fn is_pure_expr(expr: &Expr) -> bool {
        match expr {
            Expr::Literal(_) => true,
            Expr::Identifier(_) => true,
            Expr::Binary { left, right, .. } => Self::is_pure_expr(left) && Self::is_pure_expr(right),
            Expr::MemberAccess { object, .. } => Self::is_pure_expr(object),
            Expr::Subscript { object, index } => Self::is_pure_expr(object) && Self::is_pure_expr(index),
            Expr::ListLiteral(elements) => elements.iter().all(|e| Self::is_pure_expr(e)),
            Expr::DictLiteral(pairs) => pairs.iter().all(|(k, v)| Self::is_pure_expr(k) && Self::is_pure_expr(v)),
            Expr::TupleLiteral(elements) => elements.iter().all(|e| Self::is_pure_expr(e)),
            Expr::Super => true,
            Expr::Unary { operand, .. } => Self::is_pure_expr(operand),
            Expr::OptionalMemberAccess { object, .. } => Self::is_pure_expr(object),
            Expr::FString { parts } => parts.iter().all(|part| match part {
                FStringPart::Text(_) => true,
                FStringPart::Expr(e) => Self::is_pure_expr(e),
            }),
            Expr::Slice { object, start, end } => {
                Self::is_pure_expr(object)
                    && start.as_ref().map_or(true, |e| Self::is_pure_expr(e))
                    && end.as_ref().map_or(true, |e| Self::is_pure_expr(e))
            }
            Expr::ListComprehension { element, iterable, .. } => {
                Self::is_pure_expr(element) && Self::is_pure_expr(iterable)
            }
            _ => false,
        }
    }

    fn find_var_references(stmt: &Stmt, refs: &mut std::collections::HashMap<String, usize>) {
        match stmt {
            Stmt::VariableDecl { init, .. } => {
                if let Some(expr) = init {
                    Self::find_var_references_in_expr(expr, refs);
                }
            }
            Stmt::AssignStmt { lhs, value } => {
                Self::find_var_references_in_expr(lhs, refs);
                Self::find_var_references_in_expr(value, refs);
            }
            Stmt::IfStmt { condition, then_branch, else_branch } => {
                Self::find_var_references_in_expr(condition, refs);
                for s in then_branch {
                    Self::find_var_references(s, refs);
                }
                if let Some(eb) = else_branch {
                    for s in eb {
                        Self::find_var_references(s, refs);
                    }
                }
            }
            Stmt::WhileStmt { condition, body } => {
                Self::find_var_references_in_expr(condition, refs);
                for s in body {
                    Self::find_var_references(s, refs);
                }
            }
            Stmt::ForStmt { iterable, body, .. } => {
                Self::find_var_references_in_expr(iterable, refs);
                for s in body {
                    Self::find_var_references(s, refs);
                }
            }
            Stmt::TryCatchStmt { try_branch, catch_branch, finally_branch, .. } => {
                for s in try_branch {
                    Self::find_var_references(s, refs);
                }
                for s in catch_branch {
                    Self::find_var_references(s, refs);
                }
                if let Some(fb) = finally_branch {
                    for s in fb {
                        Self::find_var_references(s, refs);
                    }
                }
            }
            Stmt::RaiseStmt(expr) => {
                Self::find_var_references_in_expr(expr, refs);
            }
            Stmt::TupleUnpack { init, .. } => {
                Self::find_var_references_in_expr(init, refs);
            }
            Stmt::MatchStmt { value, cases } => {
                Self::find_var_references_in_expr(value, refs);
                for c in cases {
                    for s in &c.body {
                        Self::find_var_references(s, refs);
                    }
                }
            }
            Stmt::ExprStmt(expr) => {
                Self::find_var_references_in_expr(expr, refs);
            }
            Stmt::ReturnStmt(opt_expr) => {
                if let Some(expr) = opt_expr {
                    Self::find_var_references_in_expr(expr, refs);
                }
            }
            Stmt::FunctionDecl { body, .. } => {
                for s in body {
                    Self::find_var_references(s, refs);
                }
            }
            Stmt::ClassDecl { members, .. } => {
                for s in members {
                    Self::find_var_references(s, refs);
                }
            }
            Stmt::TraitDecl { .. } => {}
            Stmt::PyImport { .. } => {}
            Stmt::Import { .. } | Stmt::FromImport { .. } => {}
            Stmt::BreakStmt | Stmt::ContinueStmt => {}
        }
    }

    fn find_var_references_in_expr(expr: &Expr, refs: &mut std::collections::HashMap<String, usize>) {
        match expr {
            Expr::Identifier(name) => {
                let count = refs.entry(name.clone()).or_insert(0);
                *count += 1;
            }
            Expr::Binary { left, right, .. } => {
                Self::find_var_references_in_expr(left, refs);
                Self::find_var_references_in_expr(right, refs);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    Self::find_var_references_in_expr(arg, refs);
                }
            }
            Expr::MemberAccess { object, .. } => {
                Self::find_var_references_in_expr(object, refs);
            }
            Expr::MemberCall { object, args, .. } => {
                Self::find_var_references_in_expr(object, refs);
                for arg in args {
                    Self::find_var_references_in_expr(arg, refs);
                }
            }
            Expr::Subscript { object, index } => {
                Self::find_var_references_in_expr(object, refs);
                Self::find_var_references_in_expr(index, refs);
            }
            Expr::ListLiteral(elements) => {
                for e in elements {
                    Self::find_var_references_in_expr(e, refs);
                }
            }
            Expr::DictLiteral(pairs) => {
                for (k, v) in pairs {
                    Self::find_var_references_in_expr(k, refs);
                    Self::find_var_references_in_expr(v, refs);
                }
            }
            Expr::TupleLiteral(elements) => {
                for e in elements {
                    Self::find_var_references_in_expr(e, refs);
                }
            }
            Expr::SuperCall { args, .. } => {
                for arg in args {
                    Self::find_var_references_in_expr(arg, refs);
                }
            }
            Expr::Lambda { body, .. } => {
                Self::find_var_references_in_expr(body, refs);
            }
            Expr::AwaitExpr(expr) => {
                Self::find_var_references_in_expr(expr, refs);
            }
            Expr::Super => {}
            Expr::Literal(_) => {}
            Expr::Unary { operand, .. } => {
                Self::find_var_references_in_expr(operand, refs);
            }
            Expr::OptionalMemberAccess { object, .. } => {
                Self::find_var_references_in_expr(object, refs);
            }
            Expr::OptionalMemberCall { object, args, .. } => {
                Self::find_var_references_in_expr(object, refs);
                for arg in args {
                    Self::find_var_references_in_expr(arg, refs);
                }
            }
            Expr::FString { parts } => {
                for part in parts {
                    if let FStringPart::Expr(e) = part {
                        Self::find_var_references_in_expr(e, refs);
                    }
                }
            }
            Expr::Slice { object, start, end } => {
                Self::find_var_references_in_expr(object, refs);
                if let Some(s) = start {
                    Self::find_var_references_in_expr(s, refs);
                }
                if let Some(e) = end {
                    Self::find_var_references_in_expr(e, refs);
                }
            }
            Expr::ListComprehension { element, iterable, .. } => {
                Self::find_var_references_in_expr(element, refs);
                Self::find_var_references_in_expr(iterable, refs);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Literal, Expr, BinOp};

    #[test]
    fn test_constant_folding_int() {
        let expr = Expr::Binary {
            op: BinOp::Add,
            left: Box::new(Expr::Literal(Literal::Int(10))),
            right: Box::new(Expr::Binary {
                op: BinOp::Mul,
                left: Box::new(Expr::Literal(Literal::Int(20))),
                right: Box::new(Expr::Literal(Literal::Int(2))),
            }),
        };

        let optimized = Optimizer::fold_expression(expr);
        assert_eq!(optimized, Expr::Literal(Literal::Int(50)));
    }

    #[test]
    fn test_constant_folding_float() {
        let expr = Expr::Binary {
            op: BinOp::Add,
            left: Box::new(Expr::Literal(Literal::Float(2.5))),
            right: Box::new(Expr::Literal(Literal::Int(2))),
        };

        let optimized = Optimizer::fold_expression(expr);
        assert_eq!(optimized, Expr::Literal(Literal::Float(4.5)));
    }

    #[test]
    fn test_constant_comparisons() {
        let expr = Expr::Binary {
            op: BinOp::Lt,
            left: Box::new(Expr::Literal(Literal::Int(10))),
            right: Box::new(Expr::Literal(Literal::Int(20))),
        };

        let optimized = Optimizer::fold_expression(expr);
        assert_eq!(optimized, Expr::Literal(Literal::Bool(true)));
    }

    #[test]
    fn test_optimizer_dead_code_elimination() {
        // let x = 10 (not referenced) should be optimized out of program if DCE is run,
        // or a simple return stmt after another return stmt should be removed as dead code.
        let program = crate::ast::Program {
            statements: vec![
                Stmt::ReturnStmt(Some(Expr::Literal(Literal::Int(1)))),
                Stmt::ExprStmt(Expr::Identifier("unreachable".to_string())),
            ]
        };
        let optimized = Optimizer::optimize(program);
        // The second statement after the return should be eliminated.
        assert_eq!(optimized.statements.len(), 1);
    }

    #[test]
    fn test_optimizer_if_true_folding() {
        let program = crate::ast::Program {
            statements: vec![
                Stmt::IfStmt {
                    condition: Expr::Literal(Literal::Bool(true)),
                    then_branch: vec![Stmt::ReturnStmt(Some(Expr::Literal(Literal::Int(42))))],
                    else_branch: Some(vec![Stmt::ReturnStmt(Some(Expr::Literal(Literal::Int(0))))]),
                }
            ]
        };
        let optimized = Optimizer::optimize(program);
        // If true should fold to just the then_branch statements
        assert_eq!(optimized.statements.len(), 1);
        assert!(matches!(optimized.statements[0], Stmt::ReturnStmt(_)));
    }
}
