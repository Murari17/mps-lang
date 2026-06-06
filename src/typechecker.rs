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
    generic_functions: HashMap<String, (Vec<String>, Vec<crate::ast::Param>, Type, Vec<Stmt>, bool, Vec<String>)>,
}

#[derive(Clone)]
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
        functions.insert("matrix_add".to_string(), (vec![Type::Custom("Matrix".to_string()), Type::Custom("Matrix".to_string())], Type::Custom("Matrix".to_string())));
        functions.insert("matrix_sub".to_string(), (vec![Type::Custom("Matrix".to_string()), Type::Custom("Matrix".to_string())], Type::Custom("Matrix".to_string())));
        functions.insert("matrix_scale".to_string(), (vec![Type::Custom("Matrix".to_string()), Type::Float], Type::Custom("Matrix".to_string())));
        functions.insert("matrix_sigmoid".to_string(), (vec![Type::Custom("Matrix".to_string())], Type::Custom("Matrix".to_string())));
        functions.insert("matrix_softmax".to_string(), (vec![Type::Custom("Matrix".to_string())], Type::Custom("Matrix".to_string())));
        functions.insert("matrix_exp".to_string(), (vec![Type::Custom("Matrix".to_string())], Type::Custom("Matrix".to_string())));
        functions.insert("matrix_log".to_string(), (vec![Type::Custom("Matrix".to_string())], Type::Custom("Matrix".to_string())));
        functions.insert("matrix_relu".to_string(), (vec![Type::Custom("Matrix".to_string())], Type::Custom("Matrix".to_string())));
        
        functions.insert("matrix32_add".to_string(), (vec![Type::Custom("Matrix32".to_string()), Type::Custom("Matrix32".to_string())], Type::Custom("Matrix32".to_string())));
        functions.insert("matrix32_sub".to_string(), (vec![Type::Custom("Matrix32".to_string()), Type::Custom("Matrix32".to_string())], Type::Custom("Matrix32".to_string())));
        functions.insert("matrix32_scale".to_string(), (vec![Type::Custom("Matrix32".to_string()), Type::Float32], Type::Custom("Matrix32".to_string())));
        functions.insert("matrix32_sigmoid".to_string(), (vec![Type::Custom("Matrix32".to_string())], Type::Custom("Matrix32".to_string())));
        functions.insert("matrix32_softmax".to_string(), (vec![Type::Custom("Matrix32".to_string())], Type::Custom("Matrix32".to_string())));
        functions.insert("matrix32_exp".to_string(), (vec![Type::Custom("Matrix32".to_string())], Type::Custom("Matrix32".to_string())));
        functions.insert("matrix32_log".to_string(), (vec![Type::Custom("Matrix32".to_string())], Type::Custom("Matrix32".to_string())));
        functions.insert("matrix32_relu".to_string(), (vec![Type::Custom("Matrix32".to_string())], Type::Custom("Matrix32".to_string())));

        functions.insert("tensor_sigmoid".to_string(), (vec![Type::Custom("Tensor".to_string())], Type::Custom("Tensor".to_string())));
        functions.insert("tensor_relu".to_string(), (vec![Type::Custom("Tensor".to_string())], Type::Custom("Tensor".to_string())));
        functions.insert("tensor_softmax".to_string(), (vec![Type::Custom("Tensor".to_string())], Type::Custom("Tensor".to_string())));
        functions.insert("tensor_exp".to_string(), (vec![Type::Custom("Tensor".to_string())], Type::Custom("Tensor".to_string())));
        functions.insert("tensor_log".to_string(), (vec![Type::Custom("Tensor".to_string())], Type::Custom("Tensor".to_string())));

        functions.insert("tensor32_sigmoid".to_string(), (vec![Type::Custom("Tensor32".to_string())], Type::Custom("Tensor32".to_string())));
        functions.insert("tensor32_relu".to_string(), (vec![Type::Custom("Tensor32".to_string())], Type::Custom("Tensor32".to_string())));
        functions.insert("tensor32_softmax".to_string(), (vec![Type::Custom("Tensor32".to_string())], Type::Custom("Tensor32".to_string())));
        functions.insert("tensor32_exp".to_string(), (vec![Type::Custom("Tensor32".to_string())], Type::Custom("Tensor32".to_string())));
        functions.insert("tensor32_log".to_string(), (vec![Type::Custom("Tensor32".to_string())], Type::Custom("Tensor32".to_string())));

        functions.insert("mps_random".to_string(), (vec![], Type::Float));
        functions.insert("mps_randint".to_string(), (vec![Type::Int, Type::Int], Type::Int));
        functions.insert("mps_random_seed".to_string(), (vec![Type::Int], Type::Void));

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
            generic_functions: HashMap::new(),
        }
    }

    fn is_compatible(&self, declared: &Type, inferred: &Type) -> bool {
        if declared == inferred {
            return true;
        }
        if *declared == Type::PyObject || *inferred == Type::PyObject {
            return true;
        }
        if (*declared == Type::Float && *inferred == Type::Float32) || (*declared == Type::Float32 && *inferred == Type::Float) {
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
                Stmt::FunctionDecl { name, type_params, params, return_type, body, is_async, decorators } => {
                    if !type_params.is_empty() {
                        self.generic_functions.insert(name.clone(), (type_params.clone(), params.clone(), return_type.clone(), body.clone(), *is_async, decorators.clone()));
                    } else {
                        let param_types = params.iter().map(|p| p.param_type.clone()).collect();
                        self.functions.insert(name.clone(), (param_types, return_type.clone()));
                    }
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
            Stmt::FunctionDecl { type_params, params, body, .. } => {
                if !type_params.is_empty() {
                    return Ok(());
                }
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
            Stmt::TupleUnpack { vars, init } => {
                let _init_t = self.infer_expr_type(init)?;
                for var_name in vars {
                    self.declare(var_name.clone(), Type::PyObject)?;
                }
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
                    let is_matrix = class_name == "Matrix" || class_name == "Matrix32" || class_name.starts_with("Matrix<");
                    let is_tensor = class_name == "Tensor" || class_name == "Tensor32" || class_name.starts_with("Tensor<");
                    if is_matrix {
                        let is_f32 = class_name == "Matrix32" || class_name.contains("float32");
                        let elem_t = if is_f32 { Type::Float32 } else { Type::Float };
                        if method == "get" {
                            return Ok(Type::Optional(Box::new(elem_t)));
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "mul" {
                            return Ok(Type::Optional(Box::new(Type::Custom(class_name.clone()))));
                        }
                    }
                    if is_tensor {
                        let is_f32 = class_name == "Tensor32" || class_name.contains("float32");
                        let elem_t = if is_f32 { Type::Float32 } else { Type::Float };
                        if method == "get" {
                            return Ok(Type::Optional(Box::new(elem_t)));
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "shape" || method == "strides" {
                            return Ok(Type::PyObject);
                        } else if method == "sigmoid" || method == "relu" || method == "softmax" || method == "exp" || method == "log" {
                            return Ok(Type::Custom(class_name.clone()));
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
                    BinOp::And | BinOp::Or => Ok(Type::Bool),
                    BinOp::Pow => Ok(Type::Float),
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        Ok(Type::Bool)
                    }
                    _ => {
                        let is_tensor_left = match &left_t { Type::Custom(c) => c == "Tensor" || c == "Tensor32" || c.starts_with("Tensor<"), _ => false };
                        let is_tensor_right = match &right_t { Type::Custom(c) => c == "Tensor" || c == "Tensor32" || c.starts_with("Tensor<"), _ => false };
                        if is_tensor_left && is_tensor_right {
                            let is_f32_l = match &left_t { Type::Custom(c) => c == "Tensor32" || c.contains("float32"), _ => false };
                            let is_f32_r = match &right_t { Type::Custom(c) => c == "Tensor32" || c.contains("float32"), _ => false };
                            if is_f32_l || is_f32_r {
                                return Ok(Type::Custom("Tensor32".to_string()));
                            } else {
                                return Ok(Type::Custom("Tensor".to_string()));
                            }
                        }
                        // String concatenation: string + anything = string
                        if *op == BinOp::Add && (left_t == Type::String || right_t == Type::String) {
                            Ok(Type::String)
                        } else if left_t == Type::Float || right_t == Type::Float {
                            Ok(Type::Float)
                        } else if left_t == Type::Float32 || right_t == Type::Float32 {
                            Ok(Type::Float32)
                        } else if left_t == Type::Int && right_t == Type::Int {
                            Ok(Type::Int)
                        } else {
                            Ok(Type::PyObject)
                        }
                    }
                }
            }
            Expr::Call { name, type_args, args } => {
                if name == "print" || name == "mps_print" || name == "mps_println" {
                    return Ok(Type::Void);
                }
                if name == "Matrix" {
                    return Ok(Type::Custom("Matrix".to_string()));
                }
                if name == "Matrix32" {
                    return Ok(Type::Custom("Matrix32".to_string()));
                }
                if name == "Tensor" {
                    return Ok(Type::Custom("Tensor".to_string()));
                }
                if name == "Tensor32" {
                    return Ok(Type::Custom("Tensor32".to_string()));
                }
                
                // Generic template call
                if self.generic_functions.contains_key(name) {
                    let (type_params, params, return_type, body, _is_async, _decorators) = self.generic_functions.get(name).unwrap().clone();
                    if type_params.len() != type_args.len() {
                        return Err(format!("Generic function '{}' expects {} type arguments, but {} were provided", name, type_params.len(), type_args.len()));
                    }
                    let mut mapping = HashMap::new();
                    for (param, arg) in type_params.iter().zip(type_args.iter()) {
                        mapping.insert(param.clone(), arg.clone());
                    }
                    
                    let concrete_params: Vec<Type> = params.iter().map(|p| substitute_type(&p.param_type, &mapping)).collect();
                    let concrete_return = substitute_type(&return_type, &mapping);
                    let concrete_body: Vec<Stmt> = body.iter().map(|s| substitute_stmt(s, &mapping)).collect();
                    
                    if args.len() != concrete_params.len() {
                        return Err(format!("Function '{}' expects {} arguments, but {} were provided", name, concrete_params.len(), args.len()));
                    }
                    for (arg, param_t) in args.iter().zip(concrete_params.iter()) {
                        let arg_t = self.infer_expr_type(arg)?;
                        if !self.is_compatible(param_t, &arg_t) {
                            return Err(format!("Type mismatch in call to '{}': expected '{}' but found '{}'", name, param_t, arg_t));
                        }
                    }

                    let concrete_name = format!("{}_{}", name, type_args.iter().map(sanitize_type_name).collect::<Vec<_>>().join("_"));

                    // Dry-run typecheck concrete function body
                    let mut dry_run = TypeChecker::new(self.source_code.clone(), self.filename.clone());
                    dry_run.traits = self.traits.clone();
                    dry_run.classes = self.classes.clone();
                    dry_run.functions = self.functions.clone();
                    dry_run.generic_functions = self.generic_functions.clone();
                    
                    dry_run.functions.insert(concrete_name, (concrete_params.clone(), concrete_return.clone()));
                    
                    dry_run.enter_scope();
                    for (p_decl, p_concrete) in params.iter().zip(concrete_params.iter()) {
                        dry_run.declare(p_decl.name.clone(), p_concrete.clone())?;
                    }
                    for stmt in &concrete_body {
                        dry_run.check_statement(stmt)?;
                    }
                    dry_run.exit_scope();
                    
                    return Ok(concrete_return);
                }

                if let Some((param_types, ret_t)) = self.functions.get(name) {
                    if args.len() != param_types.len() {
                        return Err(format!("Function '{}' expects {} arguments, but {} were provided", name, param_types.len(), args.len()));
                    }
                    for (arg, param_t) in args.iter().zip(param_types.iter()) {
                        let arg_t = self.infer_expr_type(arg)?;
                        if !self.is_compatible(param_t, &arg_t) {
                            return Err(format!("Type mismatch in call to '{}': expected '{}' but found '{}'", name, param_t, arg_t));
                        }
                    }
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
                    let is_matrix = class_name == "Matrix" || class_name == "Matrix32" || class_name.starts_with("Matrix<");
                    let is_tensor = class_name == "Tensor" || class_name == "Tensor32" || class_name.starts_with("Tensor<");
                    if is_matrix {
                        let is_f32 = class_name == "Matrix32" || class_name.contains("float32");
                        let elem_t = if is_f32 { Type::Float32 } else { Type::Float };
                        if method == "get" {
                            return Ok(elem_t);
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "mul" {
                            return Ok(Type::Custom(class_name.clone()));
                        }
                    }
                    if is_tensor {
                        let is_f32 = class_name == "Tensor32" || class_name.contains("float32");
                        let elem_t = if is_f32 { Type::Float32 } else { Type::Float };
                        if method == "get" {
                            return Ok(elem_t);
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "shape" || method == "strides" {
                            return Ok(Type::PyObject);
                        } else if method == "sigmoid" || method == "relu" || method == "softmax" || method == "exp" || method == "log" {
                            return Ok(Type::Custom(class_name.clone()));
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
                        "length" => {
                            if args.len() != 0 {
                                return Err(format!("Method 'length' on string takes 0 arguments"));
                            }
                            return Ok(Type::Int);
                        }
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
                match &obj_t {
                    Type::Custom(class_name) => {
                        if class_name == "Matrix" || class_name.starts_with("Matrix<") {
                            let is_f32 = class_name.contains("float32");
                            if is_f32 { Ok(Type::Float32) } else { Ok(Type::Float) }
                        } else if class_name == "Matrix32" {
                            Ok(Type::Float32)
                        } else if class_name == "Tensor" || class_name.starts_with("Tensor<") {
                            let is_f32 = class_name.contains("float32");
                            if is_f32 { Ok(Type::Float32) } else { Ok(Type::Float) }
                        } else if class_name == "Tensor32" {
                            Ok(Type::Float32)
                        } else {
                            Ok(Type::PyObject)
                        }
                    }
                    Type::String => Ok(Type::String),
                    _ => Ok(Type::PyObject),
                }
            }
            Expr::Slice { object, start, end } => {
                let obj_t = self.infer_expr_type(object)?;
                if let Some(s) = start {
                    let st = self.infer_expr_type(s)?;
                    if st != Type::Int {
                        return Err(format!("Slice index must be an integer, found '{}'", st));
                    }
                }
                if let Some(e) = end {
                    let et = self.infer_expr_type(e)?;
                    if et != Type::Int {
                        return Err(format!("Slice index must be an integer, found '{}'", et));
                    }
                }
                if obj_t == Type::String {
                    Ok(Type::String)
                } else {
                    Ok(Type::PyObject)
                }
            }
            Expr::ListComprehension { element: _, var_name: _, iterable } => {
                let _iter_t = self.infer_expr_type(iterable)?;
                Ok(Type::PyObject)
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

pub fn sanitize_type_name(ty: &Type) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Float => "float".to_string(),
        Type::Float32 => "float32".to_string(),
        Type::String => "string".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Void => "void".to_string(),
        Type::PyObject => "PyObject".to_string(),
        Type::Custom(s) => {
            s.replace("<", "_").replace(">", "").replace(",", "_").replace(" ", "")
        }
        Type::Optional(inner) => format!("optional_{}", sanitize_type_name(inner)),
        Type::Function { params, return_type } => {
            let params_str: Vec<String> = params.iter().map(sanitize_type_name).collect();
            format!("fn_{}_{}", params_str.join("_"), sanitize_type_name(return_type))
        }
    }
}

pub fn substitute_type(ty: &Type, mapping: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Custom(s) => {
            if let Some(concrete) = mapping.get(s) {
                concrete.clone()
            } else if s.contains("<") {
                let mut replaced = s.clone();
                for (k, v) in mapping {
                    let target_less = format!("<{}>", k);
                    let replacement_less = format!("<{}>", v);
                    replaced = replaced.replace(&target_less, &replacement_less);

                    let target_comma_left = format!("<{},", k);
                    let replacement_comma_left = format!("<{},", v);
                    replaced = replaced.replace(&target_comma_left, &replacement_comma_left);

                    let target_comma_right = format!("{},>", k);
                    let replacement_comma_right = format!("{},>", v);
                    replaced = replaced.replace(&target_comma_right, &replacement_comma_right);

                    let target_comma_both = format!(" ,{},", k);
                    let replacement_comma_both = format!(" ,{},", v);
                    replaced = replaced.replace(&target_comma_both, &replacement_comma_both);
                    replaced = replaced.replace(&format!(",{}", k), &format!(",{}", v));
                    replaced = replaced.replace(&format!("{},", k), &format!("{},", v));
                }
                Type::Custom(replaced)
            } else {
                Type::Custom(s.clone())
            }
        }
        Type::Optional(inner) => Type::Optional(Box::new(substitute_type(inner, mapping))),
        Type::Function { params, return_type } => {
            let new_params = params.iter().map(|p| substitute_type(p, mapping)).collect();
            let new_return = Box::new(substitute_type(return_type, mapping));
            Type::Function { params: new_params, return_type: new_return }
        }
        other => other.clone(),
    }
}

fn substitute_expr(expr: &Expr, mapping: &HashMap<String, Type>) -> Expr {
    match expr {
        Expr::Literal(lit) => Expr::Literal(lit.clone()),
        Expr::Identifier(s) => Expr::Identifier(s.clone()),
        Expr::Unary { op, operand } => Expr::Unary {
            op: *op,
            operand: Box::new(substitute_expr(operand, mapping)),
        },
        Expr::Binary { op, left, right } => Expr::Binary {
            op: *op,
            left: Box::new(substitute_expr(left, mapping)),
            right: Box::new(substitute_expr(right, mapping)),
        },
        Expr::Call { name, type_args, args } => {
            let new_type_args = type_args.iter().map(|t| substitute_type(t, mapping)).collect();
            let new_args = args.iter().map(|a| substitute_expr(a, mapping)).collect();
            Expr::Call {
                name: name.clone(),
                type_args: new_type_args,
                args: new_args,
            }
        }
        Expr::MemberAccess { object, member } => Expr::MemberAccess {
            object: Box::new(substitute_expr(object, mapping)),
            member: member.clone(),
        },
        Expr::MemberCall { object, method, args } => Expr::MemberCall {
            object: Box::new(substitute_expr(object, mapping)),
            method: method.clone(),
            args: args.iter().map(|a| substitute_expr(a, mapping)).collect(),
        },
        Expr::OptionalMemberAccess { object, member } => Expr::OptionalMemberAccess {
            object: Box::new(substitute_expr(object, mapping)),
            member: member.clone(),
        },
        Expr::OptionalMemberCall { object, method, args } => Expr::OptionalMemberCall {
            object: Box::new(substitute_expr(object, mapping)),
            method: method.clone(),
            args: args.iter().map(|a| substitute_expr(a, mapping)).collect(),
        },
        Expr::Subscript { object, index } => Expr::Subscript {
            object: Box::new(substitute_expr(object, mapping)),
            index: Box::new(substitute_expr(index, mapping)),
        },
        Expr::ListLiteral(exprs) => Expr::ListLiteral(
            exprs.iter().map(|e| substitute_expr(e, mapping)).collect()
        ),
        Expr::DictLiteral(pairs) => Expr::DictLiteral(
            pairs.iter().map(|(k, v)| (substitute_expr(k, mapping), substitute_expr(v, mapping))).collect()
        ),
        Expr::TupleLiteral(exprs) => Expr::TupleLiteral(
            exprs.iter().map(|e| substitute_expr(e, mapping)).collect()
        ),
        Expr::SuperCall { method, args } => Expr::SuperCall {
            method: method.clone(),
            args: args.iter().map(|a| substitute_expr(a, mapping)).collect(),
        },
        Expr::Lambda { params, return_type, body } => {
            let new_params = params.iter().map(|p| crate::ast::Param {
                name: p.name.clone(),
                param_type: substitute_type(&p.param_type, mapping),
            }).collect();
            Expr::Lambda {
                params: new_params,
                return_type: substitute_type(return_type, mapping),
                body: Box::new(substitute_expr(body, mapping)),
            }
        }
        Expr::AwaitExpr(e) => Expr::AwaitExpr(Box::new(substitute_expr(e, mapping))),
        Expr::FString { parts } => {
            let new_parts = parts.iter().map(|p| match p {
                FStringPart::Text(t) => FStringPart::Text(t.clone()),
                FStringPart::Expr(e) => FStringPart::Expr(Box::new(substitute_expr(e, mapping))),
            }).collect();
            Expr::FString { parts: new_parts }
        }
        Expr::Super => Expr::Super,
        Expr::Slice { object, start, end } => Expr::Slice {
            object: Box::new(substitute_expr(object, mapping)),
            start: start.as_ref().map(|s| Box::new(substitute_expr(s, mapping))),
            end: end.as_ref().map(|e| Box::new(substitute_expr(e, mapping))),
        },
        Expr::ListComprehension { element, var_name, iterable } => Expr::ListComprehension {
            element: Box::new(substitute_expr(element, mapping)),
            var_name: var_name.clone(),
            iterable: Box::new(substitute_expr(iterable, mapping)),
        },
    }
}

pub fn substitute_stmt(stmt: &Stmt, mapping: &HashMap<String, Type>) -> Stmt {
    match stmt {
        Stmt::FunctionDecl { name, type_params, params, return_type, body, is_async, decorators } => {
            let new_params = params.iter().map(|p| crate::ast::Param {
                name: p.name.clone(),
                param_type: substitute_type(&p.param_type, mapping),
            }).collect();
            let new_return = substitute_type(return_type, mapping);
            let new_body = body.iter().map(|s| substitute_stmt(s, mapping)).collect();
            Stmt::FunctionDecl {
                name: name.clone(),
                type_params: type_params.clone(),
                params: new_params,
                return_type: new_return,
                body: new_body,
                is_async: *is_async,
                decorators: decorators.clone(),
            }
        }
        Stmt::ClassDecl { name, base_class, members } => {
            let new_members = members.iter().map(|s| substitute_stmt(s, mapping)).collect();
            Stmt::ClassDecl {
                name: name.clone(),
                base_class: base_class.clone(),
                members: new_members,
            }
        }
        Stmt::VariableDecl { name, is_const, var_type, init } => {
            let new_type = var_type.as_ref().map(|t| substitute_type(t, mapping));
            let new_init = init.as_ref().map(|e| substitute_expr(e, mapping));
            Stmt::VariableDecl {
                name: name.clone(),
                is_const: *is_const,
                var_type: new_type,
                init: new_init,
            }
        }
        Stmt::AssignStmt { lhs, value } => Stmt::AssignStmt {
            lhs: substitute_expr(lhs, mapping),
            value: substitute_expr(value, mapping),
        },
        Stmt::IfStmt { condition, then_branch, else_branch } => Stmt::IfStmt {
            condition: substitute_expr(condition, mapping),
            then_branch: then_branch.iter().map(|s| substitute_stmt(s, mapping)).collect(),
            else_branch: else_branch.as_ref().map(|eb| eb.iter().map(|s| substitute_stmt(s, mapping)).collect()),
        },
        Stmt::WhileStmt { condition, body } => Stmt::WhileStmt {
            condition: substitute_expr(condition, mapping),
            body: body.iter().map(|s| substitute_stmt(s, mapping)).collect(),
        },
        Stmt::ForStmt { var_name, iterable, body } => Stmt::ForStmt {
            var_name: var_name.clone(),
            iterable: substitute_expr(iterable, mapping),
            body: body.iter().map(|s| substitute_stmt(s, mapping)).collect(),
        },
        Stmt::TryCatchStmt { try_branch, catch_var, catch_branch, finally_branch } => Stmt::TryCatchStmt {
            try_branch: try_branch.iter().map(|s| substitute_stmt(s, mapping)).collect(),
            catch_var: catch_var.clone(),
            catch_branch: catch_branch.iter().map(|s| substitute_stmt(s, mapping)).collect(),
            finally_branch: finally_branch.as_ref().map(|fb| fb.iter().map(|s| substitute_stmt(s, mapping)).collect()),
        },
        Stmt::RaiseStmt(e) => Stmt::RaiseStmt(substitute_expr(e, mapping)),
        Stmt::MatchStmt { value, cases } => {
            let new_cases = cases.iter().map(|case| {
                crate::ast::MatchCase {
                    pattern: case.pattern.clone(),
                    body: case.body.iter().map(|s| substitute_stmt(s, mapping)).collect(),
                }
            }).collect();
            Stmt::MatchStmt {
                value: substitute_expr(value, mapping),
                cases: new_cases,
            }
        }
        Stmt::TupleUnpack { vars, init } => Stmt::TupleUnpack {
            vars: vars.clone(),
            init: substitute_expr(init, mapping),
        },
        Stmt::ExprStmt(e) => Stmt::ExprStmt(substitute_expr(e, mapping)),
        Stmt::ReturnStmt(opt_e) => Stmt::ReturnStmt(opt_e.as_ref().map(|e| substitute_expr(e, mapping))),
        Stmt::TraitDecl { .. } | Stmt::PyImport { .. } | Stmt::Import { .. } | Stmt::FromImport { .. } | Stmt::BreakStmt | Stmt::ContinueStmt => stmt.clone(),
    }
}
