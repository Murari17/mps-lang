use crate::ast::{Program, Stmt, Expr, Literal, BinOp, UnaryOp, Type, TraitMethodSignature, FStringPart};
use std::collections::HashMap;
use colored::*;

pub struct TypeChecker {
    source_code: String,
    filename: String,
    scopes: Vec<HashMap<String, Type>>,
    traits: HashMap<String, Vec<TraitMethodSignature>>,
    classes: HashMap<String, ClassTypeInfo>,
    functions: HashMap<String, (Vec<Type>, Type)>,
    loop_depth: usize,
}

struct ClassTypeInfo {
    #[allow(dead_code)]
    base_class: Option<String>,
    fields: HashMap<String, Type>,
    methods: HashMap<String, (Vec<Type>, Type)>,
}

impl TypeChecker {
    pub fn new(source_code: String, filename: String) -> Self {
        let mut functions = HashMap::new();
        // Register builtins
        functions.insert("print".to_string(), (vec![Type::Void], Type::Void));
        functions.insert("mps_print".to_string(), (vec![Type::Void], Type::Void));
        functions.insert("mps_println".to_string(), (vec![Type::Void], Type::Void));
        functions.insert("mps_input".to_string(), (vec![Type::String], Type::String));
        functions.insert("mps_to_int".to_string(), (vec![Type::Void], Type::Int));
        functions.insert("mps_to_float".to_string(), (vec![Type::Void], Type::Float));
        functions.insert("mps_to_string".to_string(), (vec![Type::Void], Type::String));
        functions.insert("mps_to_bool".to_string(), (vec![Type::Void], Type::Bool));
        functions.insert("mps_abs".to_string(), (vec![Type::Float], Type::Float));
        functions.insert("mps_sqrt".to_string(), (vec![Type::Float], Type::Float));
        functions.insert("mps_pow".to_string(), (vec![Type::Float, Type::Float], Type::Float));
        functions.insert("mps_floor".to_string(), (vec![Type::Float], Type::Int));
        functions.insert("mps_ceil".to_string(), (vec![Type::Float], Type::Int));
        functions.insert("mps_round".to_string(), (vec![Type::Float], Type::Int));
        functions.insert("mps_min".to_string(), (vec![Type::Float, Type::Float], Type::Float));
        functions.insert("mps_max".to_string(), (vec![Type::Float, Type::Float], Type::Float));
        functions.insert("mps_clamp".to_string(), (vec![Type::Float, Type::Float, Type::Float], Type::Float));
        functions.insert("mps_sin".to_string(), (vec![Type::Float], Type::Float));
        functions.insert("mps_cos".to_string(), (vec![Type::Float], Type::Float));
        functions.insert("mps_tan".to_string(), (vec![Type::Float], Type::Float));
        functions.insert("mps_str_len".to_string(), (vec![Type::String], Type::Int));
        functions.insert("mps_str_upper".to_string(), (vec![Type::String], Type::String));
        functions.insert("mps_str_lower".to_string(), (vec![Type::String], Type::String));
        functions.insert("mps_str_trim".to_string(), (vec![Type::String], Type::String));
        functions.insert("mps_str_contains".to_string(), (vec![Type::String, Type::String], Type::Bool));
        functions.insert("mps_str_starts_with".to_string(), (vec![Type::String, Type::String], Type::Bool));
        functions.insert("mps_str_ends_with".to_string(), (vec![Type::String, Type::String], Type::Bool));
        functions.insert("mps_str_replace".to_string(), (vec![Type::String, Type::String, Type::String], Type::String));
        functions.insert("mps_str_concat".to_string(), (vec![Type::String, Type::String], Type::String));
        functions.insert("mps_file_read".to_string(), (vec![Type::String], Type::String));
        functions.insert("mps_file_write".to_string(), (vec![Type::String, Type::String], Type::Void));
        functions.insert("mps_file_append".to_string(), (vec![Type::String, Type::String], Type::Void));
        functions.insert("mps_file_exists".to_string(), (vec![Type::String], Type::Bool));
        functions.insert("mps_exit".to_string(), (vec![Type::Int], Type::Void));
        functions.insert("mps_sleep".to_string(), (vec![Type::Int], Type::Void));
        functions.insert("mps_env".to_string(), (vec![Type::String], Type::String));
        functions.insert("map".to_string(), (vec![Type::PyObject, Type::Void], Type::PyObject));
        functions.insert("filter".to_string(), (vec![Type::PyObject, Type::Void], Type::PyObject));
        functions.insert("reduce".to_string(), (vec![Type::PyObject, Type::Void, Type::Void], Type::PyObject));
        functions.insert("len".to_string(), (vec![Type::PyObject], Type::Int));

        let mut global_scope = HashMap::new();
        global_scope.insert("MPS_PI".to_string(), Type::Float);
        global_scope.insert("MPS_E".to_string(), Type::Float);
        Self {
            source_code,
            filename,
            scopes: vec![global_scope],
            traits: HashMap::new(),
            classes: HashMap::new(),
            functions,
            loop_depth: 0,
        }
    }

    fn is_compatible(&self, declared: &Type, inferred: &Type) -> bool {
        if declared == inferred {
            return true;
        }
        if *declared == Type::PyObject || *inferred == Type::PyObject {
            return true;
        }
        if let Type::Optional(_) = declared {
            if let Type::Optional(inner) = inferred {
                if **inner == Type::Void {
                    return true;
                }
            }
        }
        if let Type::Optional(inner) = declared {
            if **inner == *inferred {
                return true;
            }
        }
        false
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: String, ty: Type) -> Result<(), String> {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name) {
                return Err(format!("Redeclared variable '{}' in same scope", name));
            }
            scope.insert(name, ty);
            Ok(())
        } else {
            Err("No active scope".to_string())
        }
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn report_error(&self, code: &str, message: &str, pattern: &str) {
        let mut line_num = 1;
        let mut col_num = 1;
        let mut line_text = "".to_string();

        for (i, line) in self.source_code.lines().enumerate() {
            if let Some(col) = line.find(pattern) {
                line_num = i + 1;
                col_num = col + 1;
                line_text = line.to_string();
                break;
            }
        }

        eprintln!(
            "{} {}: {}",
            "error".red().bold(),
            code.bold(),
            message.bold()
        );
        eprintln!(
            "  {} {}:{}:{}",
            "-->".blue().bold(),
            self.filename,
            line_num,
            col_num
        );
        eprintln!("   {}", "|".blue().bold());
        eprintln!(
            "{:4} {} {}",
            line_num.to_string().blue().bold(),
            "|".blue().bold(),
            line_text
        );
        eprintln!(
            "   {} {}{}",
            "|".blue().bold(),
            " ".repeat(col_num - 1),
            "^".red().bold()
        );
        eprintln!();
    }

    pub fn typecheck_program(&mut self, program: &Program) -> Result<(), String> {
        // Pre-register traits and class signatures
        for stmt in &program.statements {
            match stmt {
                Stmt::TraitDecl { name, methods } => {
                    self.traits.insert(name.clone(), methods.clone());
                }
                Stmt::ClassDecl { name, base_class, members } => {
                    let mut fields = HashMap::new();
                    let mut methods = HashMap::new();
                    for member in members {
                        match member {
                            Stmt::VariableDecl { name, var_type, init, .. } => {
                                let ty = var_type.clone().unwrap_or_else(|| {
                                    if let Some(expr) = init {
                                        self.infer_expr_type(expr).unwrap_or(Type::Void)
                                    } else {
                                        Type::Void
                                    }
                                });
                                fields.insert(name.clone(), ty);
                            }
                            Stmt::FunctionDecl { name, params, return_type, .. } => {
                                let param_types = params.iter().map(|p| p.param_type.clone()).collect();
                                methods.insert(name.clone(), (param_types, return_type.clone()));
                            }
                            _ => {}
                        }
                    }
                    self.classes.insert(name.clone(), ClassTypeInfo {
                        base_class: base_class.clone(),
                        fields,
                        methods,
                    });
                }
                Stmt::FunctionDecl { name, params, return_type, .. } => {
                    let param_types = params.iter().map(|p| p.param_type.clone()).collect();
                    self.functions.insert(name.clone(), (param_types, return_type.clone()));
                }
                _ => {}
            }
        }

        // Validate all statements
        for stmt in &program.statements {
            if let Err(e) = self.check_statement(stmt) {
                return Err(e);
            }
        }
        Ok(())
    }

    fn check_statement(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::VariableDecl { name, var_type, init, .. } => {
                let inferred = if let Some(expr) = init {
                    self.infer_expr_type(expr)?
                } else {
                    var_type.clone().ok_or_else(|| format!("Variable '{}' must have type declaration or initializer", name))?
                };

                if let Some(declared) = var_type {
                    if !self.is_compatible(declared, &inferred) {
                        self.report_error("E002", &format!("Type mismatch: declared '{}' but initialized as '{}'", declared, inferred), name);
                        return Err(format!("Type mismatch for variable '{}'", name));
                    }
                }

                if let Err(e) = self.declare(name.clone(), inferred) {
                    self.report_error("E003", &e, name);
                    return Err(e);
                }
            }
            Stmt::AssignStmt { lhs, value } => {
                let lhs_t = self.infer_expr_type(lhs)?;
                let val_t = self.infer_expr_type(value)?;

                if !self.is_compatible(&lhs_t, &val_t) {
                    self.report_error("E002", &format!("Type mismatch in assignment: cannot assign '{}' to LHS of type '{}'", val_t, lhs_t), "=");
                    return Err("Assignment type mismatch".to_string());
                }
            }
            Stmt::IfStmt { condition, then_branch, else_branch } => {
                let cond_t = self.infer_expr_type(condition)?;
                if cond_t != Type::Bool && cond_t != Type::PyObject {
                    self.report_error("E004", "If condition must be a boolean", "if");
                    return Err("Condition type error".to_string());
                }
                self.enter_scope();
                for s in then_branch {
                    self.check_statement(s)?;
                }
                self.exit_scope();

                if let Some(eb) = else_branch {
                    self.enter_scope();
                    for s in eb {
                        self.check_statement(s)?;
                    }
                    self.exit_scope();
                }
            }
            Stmt::WhileStmt { condition, body } => {
                let cond_t = self.infer_expr_type(condition)?;
                if cond_t != Type::Bool && cond_t != Type::PyObject {
                    self.report_error("E004", "While condition must be a boolean", "while");
                    return Err("Condition type error".to_string());
                }
                self.enter_scope();
                self.loop_depth += 1;
                for s in body {
                    self.check_statement(s)?;
                }
                self.loop_depth -= 1;
                self.exit_scope();
            }
            Stmt::ForStmt { var_name, iterable, body } => {
                let _iter_t = self.infer_expr_type(iterable)?;
                self.enter_scope();
                // If it is a range, index is Int
                if let Expr::Call { name, .. } = iterable {
                    if name == "range" {
                        self.declare(var_name.clone(), Type::Int)?;
                    } else {
                        self.declare(var_name.clone(), Type::PyObject)?;
                    }
                } else {
                    self.declare(var_name.clone(), Type::PyObject)?;
                }
                self.loop_depth += 1;
                for s in body {
                    self.check_statement(s)?;
                }
                self.loop_depth -= 1;
                self.exit_scope();
            }
            Stmt::TryCatchStmt { try_branch, catch_var, catch_branch, finally_branch } => {
                self.enter_scope();
                for s in try_branch {
                    self.check_statement(s)?;
                }
                self.exit_scope();

                self.enter_scope();
                // catch variable represents an Error structure: { message: string, code: int }
                self.declare(catch_var.clone(), Type::Custom("Error".to_string()))?;
                for s in catch_branch {
                    self.check_statement(s)?;
                }
                self.exit_scope();

                if let Some(fb) = finally_branch {
                    self.enter_scope();
                    for s in fb {
                        self.check_statement(s)?;
                    }
                    self.exit_scope();
                }
            }
            Stmt::RaiseStmt(expr) => {
                self.infer_expr_type(expr)?;
            }
            Stmt::MatchStmt { value, cases } => {
                let _val_t = self.infer_expr_type(value)?;
                for case in cases {
                    self.enter_scope();
                    for s in &case.body {
                        self.check_statement(s)?;
                    }
                    self.exit_scope();
                }
            }
            Stmt::BreakStmt => {
                if self.loop_depth == 0 {
                    self.report_error("E007", "error: `break` used outside of a loop", "break");
                    return Err("`break` used outside of a loop".to_string());
                }
            }
            Stmt::ContinueStmt => {
                if self.loop_depth == 0 {
                    self.report_error("E007", "error: `continue` used outside of a loop", "continue");
                    return Err("`continue` used outside of a loop".to_string());
                }
            }
            Stmt::FunctionDecl { params, body, .. } => {
                self.enter_scope();
                for p in params {
                    self.declare(p.name.clone(), p.param_type.clone())?;
                }
                for s in body {
                    self.check_statement(s)?;
                }
                self.exit_scope();
            }
            Stmt::ClassDecl { name, base_class, members } => {
                // If it inherits from a trait, verify trait completeness!
                if let Some(parent) = base_class {
                    if let Some(trait_methods) = self.traits.get(parent).cloned() {
                        // Trait exists! Verify class implements all trait methods
                        let class_info = self.classes.get(name).unwrap();
                        for trait_m in trait_methods {
                            if let Some((param_types, ret_type)) = class_info.methods.get(&trait_m.name) {
                                // Match types
                                let expected_params: Vec<Type> = trait_m.params.iter().map(|p| p.param_type.clone()).collect();
                                if expected_params != *param_types || trait_m.return_type != *ret_type {
                                    self.report_error("E005", &format!("Method signature of '{}' does not match trait '{}'", trait_m.name, parent), name);
                                    return Err(format!("Trait method signature mismatch for '{}'", trait_m.name));
                                }
                            } else {
                                self.report_error("E006", &format!("Class '{}' missing trait method implementation '{}'", name, trait_m.name), name);
                                return Err(format!("Unimplemented trait method '{}'", trait_m.name));
                            }
                        }
                    }
                }

                self.enter_scope();
                self.declare("self".to_string(), Type::Custom(name.clone()))?;
                for s in members {
                    self.check_statement(s)?;
                }
                self.exit_scope();
            }
            Stmt::ExprStmt(expr) => {
                self.infer_expr_type(expr)?;
            }
            Stmt::ReturnStmt(opt_expr) => {
                if let Some(expr) = opt_expr {
                    self.infer_expr_type(expr)?;
                }
            }
            Stmt::Import { .. } | Stmt::FromImport { .. } => {
                // Already resolved before typechecking — nothing to check.
            }
            _ => {}
        }
        Ok(())
    }

    pub fn infer_expr_type(&self, expr: &Expr) -> Result<Type, String> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => Ok(Type::Int),
                Literal::Float(_) => Ok(Type::Float),
                Literal::String(_) => Ok(Type::String),
                Literal::Bool(_) => Ok(Type::Bool),
                Literal::Null => Ok(Type::Optional(Box::new(Type::Void))),
            },
            Expr::Unary { op, operand } => {
                let operand_t = self.infer_expr_type(operand)?;
                match op {
                    UnaryOp::Neg => {
                        if operand_t == Type::Int || operand_t == Type::Float {
                            Ok(operand_t)
                        } else {
                            Ok(Type::PyObject)
                        }
                    }
                    UnaryOp::Not => Ok(Type::Bool),
                }
            }
            Expr::FString { parts } => {
                for part in parts {
                    if let FStringPart::Expr(expr) = part {
                        self.infer_expr_type(expr)?;
                    }
                }
                Ok(Type::String)
            }
            Expr::OptionalMemberAccess { object, member } => {
                let obj_t = self.infer_expr_type(object)?;
                if !matches!(obj_t, Type::Optional(_)) && obj_t != Type::PyObject {
                    println!("warning: optional chaining '?.' used on non-optional type '{}'", obj_t);
                }
                let inner_t = match obj_t {
                    Type::Optional(inner) => *inner,
                    other => other,
                };
                if let Type::Custom(class_name) = &inner_t {
                    if class_name == "Error" {
                        return match member.as_str() {
                            "message" => Ok(Type::Optional(Box::new(Type::String))),
                            "code" => Ok(Type::Optional(Box::new(Type::Int))),
                            _ => Ok(Type::PyObject),
                        };
                    }
                    if let Some(info) = self.classes.get(class_name) {
                        if let Some(ty) = info.fields.get(member) {
                            return Ok(Type::Optional(Box::new(ty.clone())));
                        }
                    }
                }
                Ok(Type::PyObject)
            }
            Expr::OptionalMemberCall { object, method, args } => {
                let obj_t = self.infer_expr_type(object)?;
                if !matches!(obj_t, Type::Optional(_)) && obj_t != Type::PyObject {
                    println!("warning: optional chaining '?.' used on non-optional type '{}'", obj_t);
                }
                let inner_t = match obj_t {
                    Type::Optional(inner) => *inner,
                    other => other,
                };
                for arg in args {
                    self.infer_expr_type(arg)?;
                }
                if let Type::Custom(class_name) = &inner_t {
                    if class_name == "Matrix" {
                        if method == "get" {
                            return Ok(Type::Optional(Box::new(Type::Float)));
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "mul" {
                            return Ok(Type::Optional(Box::new(Type::Custom("Matrix".to_string()))));
                        }
                    }
                    if let Some(info) = self.classes.get(class_name) {
                        if let Some((_, ret_t)) = info.methods.get(method) {
                            return Ok(Type::Optional(Box::new(ret_t.clone())));
                        }
                    }
                }
                Ok(Type::PyObject)
            }
            Expr::Identifier(name) => {
                if let Some(t) = self.lookup(name) {
                    Ok(t)
                } else if self.functions.contains_key(name) {
                    let sig = self.functions.get(name).unwrap();
                    Ok(Type::Function {
                        params: sig.0.clone(),
                        return_type: Box::new(sig.1.clone()),
                    })
                } else {
                    self.report_error("E001", &format!("Undefined variable '{}'", name), name);
                    Err(format!("Undefined variable '{}'", name))
                }
            }
            Expr::Binary { op, left, right } => {
                let left_t = self.infer_expr_type(left)?;
                let right_t = self.infer_expr_type(right)?;
                match op {
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        Ok(Type::Bool)
                    }
                    _ => {
                        // String concatenation: string + anything = string
                        if *op == BinOp::Add && (left_t == Type::String || right_t == Type::String) {
                            Ok(Type::String)
                        } else if left_t == Type::Float || right_t == Type::Float {
                            Ok(Type::Float)
                        } else if left_t == Type::Int && right_t == Type::Int {
                            Ok(Type::Int)
                        } else {
                            Ok(Type::PyObject)
                        }
                    }
                }
            }
            Expr::Call { name, .. } => {
                if name == "print" || name == "mps_print" || name == "mps_println" {
                    return Ok(Type::Void);
                }
                if name == "Matrix" {
                    return Ok(Type::Custom("Matrix".to_string()));
                }
                if let Some((_, ret_t)) = self.functions.get(name) {
                    Ok(ret_t.clone())
                } else if self.classes.contains_key(name) {
                    Ok(Type::Custom(name.clone()))
                } else {
                    Ok(Type::PyObject)
                }
            }
            Expr::MemberAccess { object, member } => {
                let obj_t = self.infer_expr_type(object)?;
                if let Type::Custom(class_name) = &obj_t {
                    // Special-case Error type for try/catch
                    if class_name == "Error" {
                        return match member.as_str() {
                            "message" => Ok(Type::String),
                            "code" => Ok(Type::Int),
                            _ => Ok(Type::PyObject),
                        };
                    }
                    if let Some(info) = self.classes.get(class_name) {
                        if let Some(ty) = info.fields.get(member) {
                            return Ok(ty.clone());
                        }
                    }
                }
                Ok(Type::PyObject)
            }
            Expr::MemberCall { object, method, args } => {
                let obj_t = self.infer_expr_type(object)?;
                if let Type::Custom(class_name) = &obj_t {
                    if class_name == "Matrix" {
                        if method == "get" {
                            return Ok(Type::Float);
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "mul" {
                            return Ok(Type::Custom("Matrix".to_string()));
                        }
                    }
                    if let Some(info) = self.classes.get(class_name) {
                        if let Some((_, ret_t)) = info.methods.get(method) {
                            return Ok(ret_t.clone());
                        }
                    }
                }
                if obj_t == Type::String {
                    match method.as_str() {
                        "upper" | "lower" | "trim" => {
                            if args.len() != 0 {
                                return Err(format!("Method '{}' on string takes 0 arguments", method));
                            }
                            return Ok(Type::String);
                        }
                        "startswith" | "endswith" | "contains" => {
                            if args.len() != 1 {
                                return Err(format!("Method '{}' on string takes 1 argument", method));
                            }
                            return Ok(Type::Bool);
                        }
                        "replace" => {
                            if args.len() != 2 {
                                return Err(format!("Method 'replace' on string takes 2 arguments"));
                            }
                            return Ok(Type::String);
                        }
                        "split" => {
                            if args.len() > 1 {
                                return Err(format!("Method 'split' on string takes at most 1 argument"));
                            }
                            return Ok(Type::PyObject);
                        }
                        "join" => {
                            if args.len() != 1 {
                                return Err(format!("Method 'join' on string takes 1 argument"));
                            }
                            return Ok(Type::String);
                        }
                        _ => {
                            return Err(format!("String has no method '{}'", method));
                        }
                    }
                }
                if obj_t == Type::PyObject {
                    match method.as_str() {
                        "append" => {
                            if args.len() != 1 {
                                return Err(format!("Method 'append' on list takes 1 argument"));
                            }
                            return Ok(Type::Void);
                        }
                        "pop" => {
                            if args.len() > 1 {
                                return Err(format!("Method 'pop' on list takes at most 1 argument"));
                            }
                            return Ok(Type::PyObject);
                        }
                        "remove" => {
                            if args.len() != 1 {
                                return Err(format!("Method 'remove' on list takes 1 argument"));
                            }
                            return Ok(Type::Void);
                        }
                        "clear" => {
                            if args.len() != 0 {
                                return Err(format!("Method 'clear' takes 0 arguments"));
                            }
                            return Ok(Type::Void);
                        }
                        "length" => {
                            if args.len() != 0 {
                                return Err(format!("Method 'length' takes 0 arguments"));
                            }
                            return Ok(Type::Int);
                        }
                        "keys" | "values" => {
                            if args.len() != 0 {
                                return Err(format!("Method '{}' takes 0 arguments", method));
                            }
                            return Ok(Type::PyObject);
                        }
                        "get" => {
                            if args.len() < 1 || args.len() > 2 {
                                return Err(format!("Method 'get' on dict takes 1 or 2 arguments"));
                            }
                            return Ok(Type::PyObject);
                        }
                        "contains" => {
                            if args.len() != 1 {
                                return Err(format!("Method 'contains' on dict takes 1 argument"));
                            }
                            return Ok(Type::Bool);
                        }
                        _ => {}
                    }
                }
                Ok(Type::PyObject)
            }
            Expr::Subscript { object, .. } => {
                let obj_t = self.infer_expr_type(object)?;
                if obj_t == Type::Custom("Matrix".to_string()) {
                    Ok(Type::Float)
                } else if obj_t == Type::String {
                    Ok(Type::String)
                } else {
                    Ok(Type::PyObject)
                }
            }
            Expr::ListLiteral(_) => Ok(Type::PyObject),
            Expr::DictLiteral(_) => Ok(Type::PyObject),
            Expr::TupleLiteral(_) => Ok(Type::PyObject),
            Expr::SuperCall { .. } => {
                // Return parent method type or fallback
                Ok(Type::PyObject)
            }
            Expr::Lambda { params, return_type, .. } => {
                let param_types = params.iter().map(|p| p.param_type.clone()).collect();
                Ok(Type::Function {
                    params: param_types,
                    return_type: Box::new(return_type.clone()),
                })
            }
            Expr::AwaitExpr(expr) => {
                self.infer_expr_type(expr)
            }
            Expr::Super => Ok(Type::PyObject),
        }
    }
}
