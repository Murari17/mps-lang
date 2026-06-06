use crate::ast::{BinOp, UnaryOp, Expr, Literal, Program, Stmt, Type, Param, MatchPattern, FStringPart};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct MethodInfo {
    params: Vec<Param>,
    return_type: Type,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct ClassInfo {
    name: String,
    parent: Option<String>,
    fields: HashMap<String, Type>,
    methods: HashMap<String, MethodInfo>,
}

pub struct Codegen {
    var_env: HashMap<String, Type>,
    func_env: HashMap<String, Type>,
    classes: HashMap<String, ClassInfo>,
    py_imports: HashMap<String, String>,
    has_py_imports: bool,
    indent_level: usize,
    current_class: Option<String>,
    scope_py_vars: Vec<Vec<String>>,
    scope_matrix_vars: Vec<Vec<String>>,
    temp_counter: usize,
    lambdas_code: String,
    async_tasks_code: String,
    async_functions: std::collections::HashSet<String>,
    generic_templates: HashMap<String, (Vec<String>, Vec<Param>, Type, Vec<Stmt>)>,
    monomorphized_code: String,
    monomorphized_set: std::collections::HashSet<String>,
    pub session_defined_funcs: std::collections::HashSet<String>,
    pub session_defined_classes: std::collections::HashSet<String>,
    pub is_repl_session: bool,
}

impl Codegen {
    pub fn new() -> Self {
        let mut func_env = HashMap::new();
        func_env.insert("print".to_string(), Type::Void);
        func_env.insert("mps_print".to_string(), Type::Void);
        func_env.insert("mps_println".to_string(), Type::Void);
        func_env.insert("mps_input".to_string(), Type::String);
        func_env.insert("mps_to_int".to_string(), Type::Int);
        func_env.insert("mps_to_float".to_string(), Type::Float);
        func_env.insert("mps_to_string".to_string(), Type::String);
        func_env.insert("mps_to_bool".to_string(), Type::Bool);
        func_env.insert("mps_abs".to_string(), Type::Float);
        func_env.insert("mps_sqrt".to_string(), Type::Float);
        func_env.insert("mps_pow".to_string(), Type::Float);
        func_env.insert("mps_floor".to_string(), Type::Int);
        func_env.insert("mps_ceil".to_string(), Type::Int);
        func_env.insert("mps_round".to_string(), Type::Int);
        func_env.insert("mps_min".to_string(), Type::Float);
        func_env.insert("mps_max".to_string(), Type::Float);
        func_env.insert("mps_clamp".to_string(), Type::Float);
        func_env.insert("mps_sin".to_string(), Type::Float);
        func_env.insert("mps_cos".to_string(), Type::Float);
        func_env.insert("mps_tan".to_string(), Type::Float);
        func_env.insert("mps_str_len".to_string(), Type::Int);
        func_env.insert("mps_str_upper".to_string(), Type::String);
        func_env.insert("mps_str_lower".to_string(), Type::String);
        func_env.insert("mps_str_trim".to_string(), Type::String);
        func_env.insert("mps_str_contains".to_string(), Type::Bool);
        func_env.insert("mps_str_starts_with".to_string(), Type::Bool);
        func_env.insert("mps_str_ends_with".to_string(), Type::Bool);
        func_env.insert("mps_str_replace".to_string(), Type::String);
        func_env.insert("mps_str_concat".to_string(), Type::String);
        func_env.insert("mps_file_read".to_string(), Type::String);
        func_env.insert("mps_file_write".to_string(), Type::Void);
        func_env.insert("mps_file_append".to_string(), Type::Void);
        func_env.insert("mps_file_exists".to_string(), Type::Bool);
        func_env.insert("mps_exit".to_string(), Type::Void);
        func_env.insert("mps_sleep".to_string(), Type::Void);
        func_env.insert("mps_env".to_string(), Type::String);
        func_env.insert("map".to_string(), Type::PyObject);
        func_env.insert("filter".to_string(), Type::PyObject);
        func_env.insert("len".to_string(), Type::Int);
        func_env.insert("matrix_add".to_string(), Type::Custom("Matrix".to_string()));
        func_env.insert("matrix_relu".to_string(), Type::Custom("Matrix".to_string()));
        func_env.insert("matrix32_add".to_string(), Type::Custom("Matrix32".to_string()));
        func_env.insert("matrix32_relu".to_string(), Type::Custom("Matrix32".to_string()));
        func_env.insert("mps_random".to_string(), Type::Float);
        func_env.insert("mps_randint".to_string(), Type::Int);
        func_env.insert("mps_random_seed".to_string(), Type::Void);

        func_env.insert("matrix_sub".to_string(), Type::Custom("Matrix".to_string()));
        func_env.insert("matrix_scale".to_string(), Type::Custom("Matrix".to_string()));
        func_env.insert("matrix_sigmoid".to_string(), Type::Custom("Matrix".to_string()));
        func_env.insert("matrix_softmax".to_string(), Type::Custom("Matrix".to_string()));
        func_env.insert("matrix_exp".to_string(), Type::Custom("Matrix".to_string()));
        func_env.insert("matrix_log".to_string(), Type::Custom("Matrix".to_string()));

        func_env.insert("matrix32_sub".to_string(), Type::Custom("Matrix32".to_string()));
        func_env.insert("matrix32_scale".to_string(), Type::Custom("Matrix32".to_string()));
        func_env.insert("matrix32_sigmoid".to_string(), Type::Custom("Matrix32".to_string()));
        func_env.insert("matrix32_softmax".to_string(), Type::Custom("Matrix32".to_string()));
        func_env.insert("matrix32_exp".to_string(), Type::Custom("Matrix32".to_string()));
        func_env.insert("matrix32_log".to_string(), Type::Custom("Matrix32".to_string()));

        func_env.insert("tensor_sigmoid".to_string(), Type::Custom("Tensor".to_string()));
        func_env.insert("tensor_relu".to_string(), Type::Custom("Tensor".to_string()));
        func_env.insert("tensor_softmax".to_string(), Type::Custom("Tensor".to_string()));
        func_env.insert("tensor_exp".to_string(), Type::Custom("Tensor".to_string()));
        func_env.insert("tensor_log".to_string(), Type::Custom("Tensor".to_string()));

        func_env.insert("tensor32_sigmoid".to_string(), Type::Custom("Tensor32".to_string()));
        func_env.insert("tensor32_relu".to_string(), Type::Custom("Tensor32".to_string()));
        func_env.insert("tensor32_softmax".to_string(), Type::Custom("Tensor32".to_string()));
        func_env.insert("tensor32_exp".to_string(), Type::Custom("Tensor32".to_string()));
        func_env.insert("tensor32_log".to_string(), Type::Custom("Tensor32".to_string()));

        func_env.insert("tensor_zeros".to_string(), Type::Custom("Tensor".to_string()));
        func_env.insert("tensor_ones".to_string(), Type::Custom("Tensor".to_string()));
        func_env.insert("tensor_randn".to_string(), Type::Custom("Tensor".to_string()));

        func_env.insert("tensor32_zeros".to_string(), Type::Custom("Tensor32".to_string()));
        func_env.insert("tensor32_ones".to_string(), Type::Custom("Tensor32".to_string()));
        func_env.insert("tensor32_randn".to_string(), Type::Custom("Tensor32".to_string()));

        let mut var_env = HashMap::new();
        var_env.insert("MPS_PI".to_string(), Type::Float);
        var_env.insert("MPS_E".to_string(), Type::Float);
 
        Self {
            var_env,
            func_env,
            classes: HashMap::new(),
            py_imports: HashMap::new(),
            has_py_imports: false,
            indent_level: 0,
            current_class: None,
            scope_py_vars: Vec::new(),
            scope_matrix_vars: Vec::new(),
            temp_counter: 0,
            lambdas_code: String::new(),
            async_tasks_code: String::new(),
            async_functions: std::collections::HashSet::new(),
            generic_templates: HashMap::new(),
            monomorphized_code: String::new(),
            monomorphized_set: std::collections::HashSet::new(),
            session_defined_funcs: std::collections::HashSet::new(),
            session_defined_classes: std::collections::HashSet::new(),
            is_repl_session: false,
        }
    }

    pub fn has_py_imports(&self) -> bool {
        self.has_py_imports
    }

    fn enter_block(&mut self) {
        self.scope_py_vars.push(Vec::new());
        self.scope_matrix_vars.push(Vec::new());
    }

    fn exit_block(&mut self) -> String {
        let mut cleanups = String::new();
        if let Some(vars) = self.scope_py_vars.pop() {
            for v in vars.iter().rev() {
                cleanups.push_str(&format!("{}Py_XDECREF({});\n", self.indent(), v));
            }
        }
        if let Some(vars) = self.scope_matrix_vars.pop() {
            for v in vars.iter().rev() {
                let var_type = self.var_env.get(v);
                let is_m32 = var_type == Some(&Type::Custom("Matrix32".to_string()));
                let is_tensor = var_type == Some(&Type::Custom("Tensor".to_string()));
                let is_tensor32 = var_type == Some(&Type::Custom("Tensor32".to_string()));
                if is_tensor32 {
                    cleanups.push_str(&format!("{}tensor32_free({});\n", self.indent(), v));
                } else if is_tensor {
                    cleanups.push_str(&format!("{}tensor_free({});\n", self.indent(), v));
                } else if is_m32 {
                    cleanups.push_str(&format!("{}matrix32_free({});\n", self.indent(), v));
                } else {
                    cleanups.push_str(&format!("{}matrix_free({});\n", self.indent(), v));
                }
            }
        }
        cleanups
    }

    fn exit_all_scopes(&self, exclude_var: Option<&str>) -> String {
        let mut cleanups = String::new();
        for vars in self.scope_py_vars.iter().rev() {
            for v in vars.iter().rev() {
                if Some(v.as_str()) != exclude_var {
                    cleanups.push_str(&format!("{}Py_XDECREF({});\n", self.indent(), v));
                }
            }
        }
        for vars in self.scope_matrix_vars.iter().rev() {
            for v in vars.iter().rev() {
                if Some(v.as_str()) != exclude_var {
                    let var_type = self.var_env.get(v);
                    let is_m32 = var_type == Some(&Type::Custom("Matrix32".to_string()));
                    let is_tensor = var_type == Some(&Type::Custom("Tensor".to_string()));
                    let is_tensor32 = var_type == Some(&Type::Custom("Tensor32".to_string()));
                    if is_tensor32 {
                        cleanups.push_str(&format!("{}tensor32_free({});\n", self.indent(), v));
                    } else if is_tensor {
                        cleanups.push_str(&format!("{}tensor_free({});\n", self.indent(), v));
                    } else if is_m32 {
                        cleanups.push_str(&format!("{}matrix32_free({});\n", self.indent(), v));
                    } else {
                        cleanups.push_str(&format!("{}matrix_free({});\n", self.indent(), v));
                    }
                }
            }
        }
        cleanups
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn escape_string(s: &str) -> String {
        let mut escaped = String::new();
        for c in s.chars() {
            match c {
                '"' => escaped.push_str("\\\""),
                '\\' => escaped.push_str("\\\\"),
                '\n' => escaped.push_str("\\n"),
                '\t' => escaped.push_str("\\t"),
                '\r' => escaped.push_str("\\r"),
                _ => escaped.push(c),
            }
        }
        escaped
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
        if let Type::Optional(inner) = declared {
            if **inner == *inferred {
                return true;
            }
        }
        false
    }

    fn c_type(&self, t: &Type) -> String {
        match t {
            Type::Int => "int".to_string(),
            Type::Float => "double".to_string(),
            Type::Float32 => "float".to_string(),
            Type::String => "const char*".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Void => "void".to_string(),
            Type::PyObject => "PyObject*".to_string(),
            Type::Function { .. } => "void*".to_string(),
            Type::Optional(inner) => {
                let base_c = self.c_type(inner);
                if base_c.ends_with('*') {
                    base_c
                } else {
                    format!("{}*", base_c)
                }
            }
            Type::Custom(name) => {
                if name == "Matrix" {
                    "MPSMatrix*".to_string()
                } else if name == "Matrix32" {
                    "MPSMatrix32*".to_string()
                } else if name == "Tensor" {
                    "MPSTensor*".to_string()
                } else if name == "Tensor32" {
                    "MPSTensor32*".to_string()
                } else if name == "Error" {
                    "MPS_Error".to_string()
                } else if self.classes.contains_key(name) || name.starts_with("_args_") {
                    format!("{}*", name)
                } else {
                    name.clone()
                }
            }
        }
    }

    fn binary_op_symbol(op: BinOp) -> Option<&'static str> {
        match op {
            BinOp::Add => Some("+"),
            BinOp::Sub => Some("-"),
            BinOp::Mul => Some("*"),
            BinOp::Div => Some("/"),
            BinOp::Percent => Some("%"),
            BinOp::Eq => Some("=="),
            BinOp::Ne => Some("!="),
            BinOp::Lt => Some("<"),
            BinOp::Le => Some("<="),
            BinOp::Gt => Some(">"),
            BinOp::Ge => Some(">="),
            BinOp::Pow | BinOp::And | BinOp::Or | BinOp::MatMul => None,
        }
    }

    fn resolve_property(&self, class_name: &str, prop: &str) -> Option<(Type, String)> {
        if let Some(info) = self.classes.get(class_name) {
            if let Some(t) = info.fields.get(prop) {
                return Some((t.clone(), prop.to_string()));
            }
            if let Some(parent) = &info.parent {
                if let Some((t, path)) = self.resolve_property(parent, prop) {
                    return Some((t, format!("base.{}", path)));
                }
            }
        }
        None
    }

    fn resolve_method(&self, class_name: &str, method: &str) -> Option<(String, String)> {
        if let Some(info) = self.classes.get(class_name) {
            if info.methods.contains_key(method) {
                return Some((class_name.to_string(), "".to_string()));
            }
            if let Some(parent) = &info.parent {
                if let Some((defining_class, _)) = self.resolve_method(parent, method) {
                    return Some((defining_class, format!("({})", parent)));
                }
            }
        }
        None
    }

    fn get_constructor(&self, class_name: &str) -> Option<(String, MethodInfo)> {
        if let Some(info) = self.classes.get(class_name) {
            if let Some(init) = info.methods.get("init") {
                return Some((class_name.to_string(), init.clone()));
            }
            if let Some(parent) = &info.parent {
                return self.get_constructor(parent);
            }
        }
        None
    }

    fn get_direct_indices(&self, index_expr: &Expr) -> Option<Vec<Expr>> {
        match index_expr {
            Expr::TupleLiteral(elts) => {
                for elt in elts {
                    if let Ok(elt_t) = self.infer_type(elt) {
                        if elt_t != Type::Int {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                Some(elts.clone())
            }
            _ => {
                if let Ok(t) = self.infer_type(index_expr) {
                    if t == Type::Int {
                        return Some(vec![index_expr.clone()]);
                    }
                }
                None
            }
        }
    }

    fn infer_type(&self, expr: &Expr) -> Result<Type, String> {
        match expr {
            Expr::Slice { object, .. } => {
                let obj_t = self.infer_type(object)?;
                if obj_t == Type::String {
                    Ok(Type::String)
                } else {
                    Ok(Type::PyObject)
                }
            }
            Expr::ListComprehension { .. } => Ok(Type::PyObject),
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => Ok(Type::Int),
                Literal::Float(_) => Ok(Type::Float),
                Literal::String(_) => Ok(Type::String),
                Literal::Bool(_) => Ok(Type::Bool),
                Literal::Null => Ok(Type::Optional(Box::new(Type::Void))),
            },
            Expr::Unary { op, operand } => {
                let operand_t = self.infer_type(operand)?;
                match op {
                    UnaryOp::Neg => Ok(operand_t),
                    UnaryOp::Not => Ok(Type::Bool),
                }
            }
            Expr::FString { parts } => {
                for part in parts {
                    if let FStringPart::Expr(expr) = part {
                        self.infer_type(expr)?;
                    }
                }
                Ok(Type::String)
            }
            Expr::OptionalMemberAccess { object, member } => {
                let obj_t = self.infer_type(object)?;
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
                let obj_t = self.infer_type(object)?;
                let inner_t = match obj_t {
                    Type::Optional(inner) => *inner,
                    other => other,
                };
                for arg in args {
                    self.infer_type(arg)?;
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
                    } else if class_name == "Matrix32" {
                        if method == "get" {
                            return Ok(Type::Optional(Box::new(Type::Float32)));
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "mul" {
                            return Ok(Type::Optional(Box::new(Type::Custom("Matrix32".to_string()))));
                        }
                    }
                    if let Some(info) = self.classes.get(class_name) {
                        if let Some(method_info) = info.methods.get(method) {
                            return Ok(Type::Optional(Box::new(method_info.return_type.clone())));
                        }
                    }
                }
                Ok(Type::PyObject)
            }
            Expr::Identifier(name) => {
                if let Some(t) = self.var_env.get(name) {
                    Ok(t.clone())
                } else {
                    Err(format!("Type Error: Undefined variable '{}'", name))
                }
            }
            Expr::Binary { op, left, right } => {
                let left_t = self.infer_type(left)?;
                let right_t = self.infer_type(right)?;
                match op {
                    BinOp::And | BinOp::Or => Ok(Type::Bool),
                    BinOp::Pow => Ok(Type::Float),
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        Ok(Type::Bool)
                    }
                    BinOp::MatMul => {
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
                        let is_matrix_left = match &left_t { Type::Custom(c) => c == "Matrix" || c == "Matrix32" || c.starts_with("Matrix<"), _ => false };
                        let is_matrix_right = match &right_t { Type::Custom(c) => c == "Matrix" || c == "Matrix32" || c.starts_with("Matrix<"), _ => false };
                        if is_matrix_left && is_matrix_right {
                            let is_f32_l = match &left_t { Type::Custom(c) => c == "Matrix32" || c.contains("float32"), _ => false };
                            let is_f32_r = match &right_t { Type::Custom(c) => c == "Matrix32" || c.contains("float32"), _ => false };
                            if is_f32_l || is_f32_r {
                                return Ok(Type::Custom("Matrix32".to_string()));
                            } else {
                                return Ok(Type::Custom("Matrix".to_string()));
                            }
                        }
                        if let Type::Custom(class_name) = &left_t {
                            if let Some((def_class, _)) = self.resolve_method(class_name, "matmul") {
                                let def_info = self.classes.get(&def_class).unwrap();
                                let meth_info = def_info.methods.get("matmul").unwrap();
                                return Ok(meth_info.return_type.clone());
                            }
                        }
                        Ok(Type::PyObject)
                    }
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Percent => {
                        // String concatenation: string + anything = string
                        if *op == BinOp::Add && (left_t == Type::String || right_t == Type::String) {
                            Ok(Type::String)
                        } else if left_t == Type::Float || right_t == Type::Float {
                            Ok(Type::Float)
                        } else if left_t == Type::Int && right_t == Type::Int {
                            Ok(Type::Int)
                        } else if left_t == Type::PyObject || right_t == Type::PyObject {
                            Ok(Type::PyObject)
                        } else {
                            // Check for Tensor/Tensor32 binary ops
                            let is_tensor_left = match &left_t { Type::Custom(c) => c == "Tensor" || c == "Tensor32", _ => false };
                            let is_tensor_right = match &right_t { Type::Custom(c) => c == "Tensor" || c == "Tensor32", _ => false };
                            if is_tensor_left && is_tensor_right {
                                let is_f32_l = match &left_t { Type::Custom(c) => c == "Tensor32", _ => false };
                                let is_f32_r = match &right_t { Type::Custom(c) => c == "Tensor32", _ => false };
                                if is_f32_l || is_f32_r {
                                    return Ok(Type::Custom("Tensor32".to_string()));
                                } else {
                                    return Ok(Type::Custom("Tensor".to_string()));
                                }
                            }
                            // Check operator overloading!
                            if let Type::Custom(class_name) = &left_t {
                                let op_method = match op {
                                    BinOp::Add => Some("add"),
                                    BinOp::Sub => Some("sub"),
                                    BinOp::Mul => Some("mul"),
                                    BinOp::Div => Some("div"),
                                    BinOp::Percent => Some("mod"),
                                    _ => None,
                                };
                                if let Some(m_name) = op_method {
                                    if let Some((def_class, _)) = self.resolve_method(class_name, m_name) {
                                        let def_info = self.classes.get(&def_class).unwrap();
                                        let meth_info = def_info.methods.get(m_name).unwrap();
                                        return Ok(meth_info.return_type.clone());
                                    }
                                }
                            }
                            Err(format!(
                                "Type Error: Operator '{}' cannot be applied to types '{}' and '{}'",
                                op, left_t, right_t
                            ))
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
                if name == "Matrix32" {
                    return Ok(Type::Custom("Matrix32".to_string()));
                }
                if name == "Tensor" {
                    return Ok(Type::Custom("Tensor".to_string()));
                }
                if name == "Tensor32" {
                    return Ok(Type::Custom("Tensor32".to_string()));
                }
                // Generic function call — look up the template and return the substituted return type
                if let Expr::Call { type_args, .. } = expr {
                    if !type_args.is_empty() {
                        if let Some((type_params, _params, return_type, _body)) = self.generic_templates.get(name) {
                            if type_params.len() == type_args.len() {
                                let mut mapping = std::collections::HashMap::new();
                                for (param, arg) in type_params.iter().zip(type_args.iter()) {
                                    mapping.insert(param.clone(), arg.clone());
                                }
                                let concrete_return = crate::typechecker::substitute_type(return_type, &mapping);
                                return Ok(concrete_return);
                            }
                        }
                    }
                }
                if self.classes.contains_key(name) {
                    return Ok(Type::Custom(name.clone()));
                }
                if self.async_functions.contains(name) {
                    return Ok(Type::Custom(format!("_args_{}", name)));
                }
                if let Some(t) = self.var_env.get(name) {
                    if let Type::Function { return_type, .. } = t {
                        return Ok(*return_type.clone());
                    }
                }
                if let Some(t) = self.func_env.get(name) {
                    Ok(t.clone())
                } else {
                    Err(format!("Type Error: Undefined function '{}'", name))
                }
            }
            Expr::MemberAccess { object, member } => {
                let obj_t = self.infer_type(object)?;
                if let Type::Custom(class_name) = &obj_t {
                    // Special-case Error type for try/catch
                    if class_name == "Error" {
                        return match member.as_str() {
                            "message" => Ok(Type::String),
                            "code" => Ok(Type::Int),
                            _ => Err(format!("Type Error: Error has no property '{}'", member)),
                        };
                    }
                    if let Some((t, _)) = self.resolve_property(class_name, member) {
                        Ok(t)
                    } else {
                        Err(format!("Type Error: Class '{}' does not have property '{}'", class_name, member))
                    }
                } else if obj_t == Type::PyObject {
                    Ok(Type::PyObject)
                } else {
                    Err(format!("Type Error: Member access '.' only allowed on custom classes or Python objects, found '{:?}'", obj_t))
                }
            }
            Expr::MemberCall { object, method, .. } => {
                let obj_t = self.infer_type(object)?;
                if let Type::Custom(class_name) = &obj_t {
                    if class_name == "Matrix" {
                        if method == "get" {
                            return Ok(Type::Float);
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "mul" {
                            return Ok(Type::Custom("Matrix".to_string()));
                        } else {
                            return Err(format!("Type Error: Native Matrix does not have method '{}'", method));
                        }
                    } else if class_name == "Matrix32" {
                        if method == "get" {
                            return Ok(Type::Float32);
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "mul" {
                            return Ok(Type::Custom("Matrix32".to_string()));
                        } else {
                            return Err(format!("Type Error: Native Matrix32 does not have method '{}'", method));
                        }
                    } else if class_name == "Tensor" || class_name.starts_with("Tensor<") {
                        let is_f32 = class_name.contains("float32");
                        if method == "get" {
                            return Ok(if is_f32 { Type::Float32 } else { Type::Float });
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "shape" || method == "strides" {
                            return Ok(Type::PyObject);
                        } else if method == "sigmoid" || method == "relu" || method == "softmax" || method == "exp" || method == "log" || method == "reshape" || method == "transpose" || method == "squeeze" || method == "matmul" {
                            return Ok(Type::Custom(class_name.clone()));
                        } else {
                            return Err(format!("Type Error: Native Tensor does not have method '{}'", method));
                        }
                    } else if class_name == "Tensor32" {
                        if method == "get" {
                            return Ok(Type::Float32);
                        } else if method == "set" {
                            return Ok(Type::Void);
                        } else if method == "shape" || method == "strides" {
                            return Ok(Type::PyObject);
                        } else if method == "sigmoid" || method == "relu" || method == "softmax" || method == "exp" || method == "log" || method == "reshape" || method == "transpose" || method == "squeeze" || method == "matmul" {
                            return Ok(Type::Custom("Tensor32".to_string()));
                        } else {
                            return Err(format!("Type Error: Native Tensor32 does not have method '{}'", method));
                        }
                    }
                }
                if obj_t == Type::String {
                    match method.as_str() {
                        "length" => {
                            return Ok(Type::Int);
                        }
                        "upper" | "lower" | "trim" | "replace" | "join" => {
                            return Ok(Type::String);
                        }
                        "startswith" | "endswith" | "contains" => {
                            return Ok(Type::Bool);
                        }
                        "split" => {
                            return Ok(Type::PyObject);
                        }
                        _ => return Err(format!("Type Error: String has no method '{}'", method)),
                    }
                }
                if let Type::Custom(class_name) = obj_t {
                    if let Some((def_class, _)) = self.resolve_method(&class_name, method) {
                        let info = self.classes.get(&def_class).unwrap();
                        let meth_info = info.methods.get(method).unwrap();
                        Ok(meth_info.return_type.clone())
                    } else {
                        Err(format!("Type Error: Class '{}' does not have method '{}'", class_name, method))
                    }
                } else if obj_t == Type::PyObject {
                    match method.as_str() {
                        "append" | "remove" | "clear" => Ok(Type::Void),
                        "length" => Ok(Type::Int),
                        "contains" => Ok(Type::Bool),
                        _ => Ok(Type::PyObject),
                    }
                } else {
                    Err(format!("Type Error: Method call only allowed on custom classes or Python objects, found '{:?}'", obj_t))
                }
            }
            Expr::Subscript { object, .. } => {
                let obj_t = self.infer_type(object)?;
                if obj_t == Type::PyObject {
                    Ok(Type::PyObject)
                } else if let Type::Custom(ref class_name) = obj_t {
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
                        Err(format!("Type Error: Subscript indexing only supported on Python objects, Matrices, and Tensors, found '{:?}'", obj_t))
                    }
                } else if obj_t == Type::String {
                    Ok(Type::String)
                } else {
                    Err(format!("Type Error: Subscript indexing only supported on Python objects, found '{:?}'", obj_t))
                }
            }
            Expr::ListLiteral(_) => Ok(Type::PyObject),
            Expr::DictLiteral(_) => Ok(Type::PyObject),
            Expr::TupleLiteral(_) => Ok(Type::PyObject),
            Expr::SuperCall { method, .. } => {
                if let Some(class_name) = &self.current_class {
                    if let Some(info) = self.classes.get(class_name) {
                        if let Some(parent) = &info.parent {
                            if let Some((def_class, _)) = self.resolve_method(parent, method) {
                                let def_info = self.classes.get(&def_class).unwrap();
                                let meth_info = def_info.methods.get(method).unwrap();
                                return Ok(meth_info.return_type.clone());
                            }
                        }
                    }
                }
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
                let inner_t = self.infer_type(expr)?;
                if let Type::Custom(ref s) = inner_t {
                    if s.starts_with("_args_") {
                        let fn_name = &s[6..];
                        if let Some(t) = self.func_env.get(fn_name) {
                            return Ok(t.clone());
                        }
                    }
                }
                Ok(inner_t)
            }
            Expr::Super => Ok(Type::PyObject),
        }
    }

    pub fn transpile_program(&mut self, program: &Program) -> Result<String, String> {

        // Scan and register functions, classes, and Python imports in environment
        for stmt in &program.statements {
            match stmt {
                Stmt::FunctionDecl { name, type_params, params, return_type, body, is_async, .. } => {
                    if !type_params.is_empty() {
                        self.generic_templates.insert(name.clone(), (type_params.clone(), params.clone(), return_type.clone(), body.clone()));
                    } else {
                        self.func_env.insert(name.clone(), return_type.clone());
                        if *is_async {
                            self.async_functions.insert(name.clone());
                        }
                    }
                }
                Stmt::PyImport { library, alias } => {
                    let alias_name = alias.clone().unwrap_or_else(|| library.clone());
                    self.py_imports.insert(alias_name.clone(), library.clone());
                    self.var_env.insert(alias_name.clone(), Type::PyObject);
                    self.has_py_imports = true;
                }
                Stmt::ClassDecl { name, base_class, members } => {
                    let mut fields = HashMap::new();
                    let mut methods = HashMap::new();

                    for member in members {
                        match member {
                            Stmt::VariableDecl { name, var_type, init, .. } => {
                                let t = if let Some(declared) = var_type {
                                    declared.clone()
                                } else {
                                    self.infer_type(init.as_ref().unwrap())?
                                };
                                fields.insert(name.clone(), t);
                            }
                            Stmt::FunctionDecl { name, params, return_type, .. } => {
                                methods.insert(
                                    name.clone(),
                                    MethodInfo {
                                        params: params.clone(),
                                        return_type: return_type.clone(),
                                    },
                                );
                            }
                            _ => {}
                        }
                    }

                    self.classes.insert(
                        name.clone(),
                        ClassInfo {
                            name: name.clone(),
                            parent: base_class.clone(),
                            fields,
                            methods,
                        },
                    );
                }
                _ => {}
            }
        }

        // Global Python Object Declarations
        let mut py_globals = String::new();
        if self.has_py_imports {
            for alias_name in self.py_imports.keys() {
                py_globals.push_str(&format!("PyObject* {} = NULL;\n", alias_name));
            }
            py_globals.push_str("\n");
        }

        // Struct Forward Declarations
        let mut forward_decls = String::new();
        for class_name in self.classes.keys() {
            forward_decls.push_str(&format!("typedef struct {} {};\n", class_name, class_name));
        }
        forward_decls.push_str("\n");

        // Struct Layout Definitions
        let mut struct_definitions = String::new();
        let mut ordered_classes = Vec::new();
        let mut visited = HashMap::new();

        fn layout_class(
            name: &str,
            classes: &HashMap<String, ClassInfo>,
            ordered: &mut Vec<String>,
            visited: &mut HashMap<String, bool>,
        ) {
            if visited.contains_key(name) {
                return;
            }
            if let Some(info) = classes.get(name) {
                if let Some(parent) = &info.parent {
                    layout_class(parent, classes, ordered, visited);
                }
                visited.insert(name.to_string(), true);
                ordered.push(name.to_string());
            }
        }

        for class_name in self.classes.keys() {
            layout_class(class_name, &self.classes, &mut ordered_classes, &mut visited);
        }

        for class_name in &ordered_classes {
            let info = self.classes.get(class_name).unwrap();
            struct_definitions.push_str(&format!("struct {} {{\n", class_name));
            if let Some(parent) = &info.parent {
                if self.classes.contains_key(parent) {
                    struct_definitions.push_str(&format!("    {} base;\n", parent));
                }
            }
            for (f_name, f_type) in &info.fields {
                struct_definitions.push_str(&format!("    {} {};\n", self.c_type(f_type), f_name));
            }
            struct_definitions.push_str("};\n\n");
        }

        // Method Declarations and Instantiation Constructors declarations
        let mut forward_methods = String::new();
        for class_name in &ordered_classes {
            let info = self.classes.get(class_name).unwrap();
            
            // Constructor Declaration
            let mut init_args = Vec::new();
            if let Some((_, init_info)) = self.get_constructor(class_name) {
                for p in &init_info.params {
                    if p.name != "self" {
                        init_args.push(format!("{} {}", self.c_type(&p.param_type), p.name));
                    }
                }
            }
            forward_methods.push_str(&format!(
                "MPS_EXPORT {}* {}_new({});\n",
                class_name,
                class_name,
                init_args.join(", ")
            ));

            // Class methods Declarations
            for (m_name, m_info) in &info.methods {
                let mut method_args = Vec::new();
                for p in &m_info.params {
                    if p.name == "self" {
                        method_args.push(format!("{} self", self.c_type(&Type::Custom(class_name.clone()))));
                    } else {
                        method_args.push(format!("{} {}", self.c_type(&p.param_type), p.name));
                    }
                }
                forward_methods.push_str(&format!(
                    "MPS_EXPORT {} {}_{}({});\n",
                    self.c_type(&m_info.return_type),
                    class_name,
                    m_name,
                    method_args.join(", ")
                ));
            }
        }

        // Forward declare normal & async functions (skip generic templates)
        for stmt in &program.statements {
            if let Stmt::FunctionDecl { name, type_params, params, return_type, is_async, .. } = stmt {
                if !type_params.is_empty() {
                    continue; // Skip generic templates
                }
                let mut init_args = Vec::new();
                for p in params {
                    init_args.push(format!("{} {}", self.c_type(&p.param_type), p.name));
                }
                if *is_async {
                    forward_methods.push_str(&format!("typedef struct _args_{} _args_{};\n", name, name));
                    forward_methods.push_str(&format!("MPS_EXPORT {} _impl_{}({});\n", self.c_type(return_type), name, init_args.join(", ")));
                    forward_methods.push_str(&format!("MPS_EXPORT _args_{}* {}({});\n", name, name, init_args.join(", ")));
                } else {
                    forward_methods.push_str(&format!("MPS_EXPORT {} {}({});\n", self.c_type(return_type), name, init_args.join(", ")));
                }
            }
        }
        forward_methods.push_str("\n");

        let mut functions_code = String::new();
        let mut classes_code = String::new();
        let mut top_level_statements = Vec::new();

        // Transpilation of functions and classes
        for stmt in &program.statements {
            match stmt {
                Stmt::FunctionDecl { name, type_params, params, return_type, body, is_async, .. } => {
                    if !type_params.is_empty() {
                        continue; // Skip generic templates
                    }
                    if self.session_defined_funcs.contains(name) {
                        continue;
                    }
                    if *is_async {
                        // Transpile async task wrapper & structures
                        let mut arg_struct = format!("struct _args_{} {{\n", name);
                        for p in params {
                            arg_struct.push_str(&format!("    {} {};\n", self.c_type(&p.param_type), p.name));
                        }
                        if *return_type != Type::Void {
                            arg_struct.push_str(&format!("    {} _ret;\n", self.c_type(return_type)));
                        }
                        arg_struct.push_str("    MPS_Task _task;\n");
                        arg_struct.push_str("};\n\n");
                        
                        let mut task_fn = format!("void _task_wrapper_{}(void* arg) {{\n", name);
                        task_fn.push_str(&format!("    _args_{}* a = (_args_{}*)arg;\n", name, name));
                        if *return_type != Type::Void {
                            task_fn.push_str("    a->_ret = ");
                        } else {
                            task_fn.push_str("    ");
                        }
                        let call_args: Vec<String> = params.iter().map(|p| format!("a->{}", p.name)).collect();
                        task_fn.push_str(&format!("_impl_{}({});\n", name, call_args.join(", ")));
                        task_fn.push_str("}\n\n");
                        
                        let mut launcher_fn = format!(
                            "_args_{}* {}(",
                            name,
                            name
                        );
                        let param_strs: Vec<String> = params
                            .iter()
                            .map(|p| format!("{} {}", self.c_type(&p.param_type), p.name))
                            .collect();
                        launcher_fn.push_str(&param_strs.join(", "));
                        launcher_fn.push_str(") {\n");
                        launcher_fn.push_str(&format!("    _args_{}* a = malloc(sizeof(_args_{}));\n", name, name));
                        for p in params {
                            launcher_fn.push_str(&format!("    a->{} = {};\n", p.name, p.name));
                        }
                        launcher_fn.push_str(&format!("    a->_task.fn = _task_wrapper_{};\n", name));
                        launcher_fn.push_str("    a->_task.arg = a;\n");
                        launcher_fn.push_str("    mps_pool_submit(&a->_task);\n");
                        launcher_fn.push_str("    return a;\n");
                        launcher_fn.push_str("}\n\n");
                        
                        self.async_tasks_code.push_str(&arg_struct);
                        self.async_tasks_code.push_str(&task_fn);
                        self.async_tasks_code.push_str(&launcher_fn);
                        
                        let impl_name = format!("_impl_{}", name);
                        let func_code = self.transpile_function(&impl_name, params, return_type, body)?;
                        functions_code.push_str(&func_code);
                        functions_code.push_str("\n");
                    } else {
                        let func_code = self.transpile_function(name, params, return_type, body)?;
                        functions_code.push_str(&func_code);
                        functions_code.push_str("\n");
                    }
                }
                Stmt::ClassDecl { name, members, .. } => {
                    if self.session_defined_classes.contains(name) {
                        continue;
                    }
                    self.current_class = Some(name.clone());
                    
                    // Transpile methods
                    for member in members {
                        if let Stmt::FunctionDecl { name: m_name, params, return_type, body, .. } = member {
                            let prefixes = format!("{}_{}", name, m_name);
                            let method_code = self.transpile_class_method(&prefixes, params, return_type, body)?;
                            classes_code.push_str(&method_code);
                            classes_code.push_str("\n");
                        }
                    }

                    // Generate constructor implementation
                    let mut init_params = Vec::new();
                    let mut call_args = Vec::new();
                    let resolved_init = self.get_constructor(name);

                    if let Some((_, init_info)) = &resolved_init {
                        for p in &init_info.params {
                            if p.name != "self" {
                                init_params.push(format!("{} {}", self.c_type(&p.param_type), p.name));
                                call_args.push(p.name.clone());
                            }
                        }
                    }
                    
                    let mut new_impl = format!("{}* {}_new({}) {{\n", name, name, init_params.join(", "));
                    new_impl.push_str(&format!("    {}* self = malloc(sizeof({}));\n", name, name));
                    
                    if let Some((defining_class, _)) = resolved_init {
                        if defining_class == *name {
                            new_impl.push_str(&format!("    {}_init(self", name));
                        } else {
                            new_impl.push_str(&format!("    {}_init(({}*)self", defining_class, defining_class));
                        }
                        for arg in call_args {
                            new_impl.push_str(&format!(", {}", arg));
                        }
                        new_impl.push_str(");\n");
                    }
                    new_impl.push_str("    return self;\n");
                    new_impl.push_str("}\n\n");
                    classes_code.push_str(&new_impl);

                    self.current_class = None;
                }
                Stmt::PyImport { .. } | Stmt::Import { .. } | Stmt::FromImport { .. } => {}
                other => {
                    top_level_statements.push(other);
                }
            }
        }

        // Build the C file header now that we know if Python is needed
        let mut header = String::new();
        if !self.is_repl_session {
            header.push_str("#define MPS_RUNTIME_IMPL\n");
        }
        if self.has_py_imports {
            header.push_str("#define MPS_USE_PYTHON\n");
        }
        header.push_str("#ifndef MPS_EXPORT\n");
        header.push_str("#if defined(_WIN32) && defined(MPS_EMIT_SO)\n");
        header.push_str("#define MPS_EXPORT __declspec(dllexport)\n");
        header.push_str("#elif defined(MPS_EMIT_SO)\n");
        header.push_str("#define MPS_EXPORT __attribute__((visibility(\"default\")))\n");
        header.push_str("#else\n");
        header.push_str("#define MPS_EXPORT\n");
        header.push_str("#endif\n");
        header.push_str("#endif\n");
        header.push_str("#include \"runtime.h\"\n\n");

        let mut top_level_code = String::new();

        // Generate main function
        if (!top_level_statements.is_empty() || self.has_py_imports) && !self.is_repl_session {
            if self.func_env.contains_key("main") {
                return Err("Compilation Error: Cannot have both top-level statements and an explicit 'main' function definition.".into());
            }

            top_level_code.push_str("int main() {\n");
            self.indent_level += 1;
            self.enter_block();

            if self.has_py_imports {
                top_level_code.push_str(&format!("{}Py_Initialize();\n", self.indent()));
                
                // Initialize imports inside C main
                for (alias_name, lib_name) in &self.py_imports {
                    top_level_code.push_str(&format!(
                        "{}PyObject* {}_name = PyUnicode_DecodeFSDefault(\"{}\");\n",
                        self.indent(),
                        alias_name,
                        lib_name
                    ));
                    top_level_code.push_str(&format!(
                        "{}{} = PyImport_Import({}_name);\n",
                        self.indent(),
                        alias_name,
                        alias_name
                    ));
                    top_level_code.push_str(&format!(
                        "{}Py_DECREF({}_name);\n",
                        self.indent(),
                        alias_name
                    ));
                }
                top_level_code.push_str("\n");
            }

            for stmt in top_level_statements {
                let stmt_code = self.transpile_statement(stmt)?;
                top_level_code.push_str(&stmt_code);
            }

            let cleanups = self.exit_block();
            top_level_code.push_str(&cleanups);

            if self.has_py_imports {
                for alias_name in self.py_imports.keys() {
                    top_level_code.push_str(&format!("{}Py_XDECREF({});\n", self.indent(), alias_name));
                }
                top_level_code.push_str(&format!("{}Py_Finalize();\n", self.indent()));
            }

            top_level_code.push_str(&format!("{}return 0;\n", self.indent()));
            self.indent_level -= 1;
            top_level_code.push_str("}\n");
        }

        let mut output = header;
        output.push_str(&py_globals);
        output.push_str(&forward_decls);
        output.push_str(&struct_definitions);
        output.push_str(&forward_methods);
        output.push_str(&self.lambdas_code);
        output.push_str(&self.async_tasks_code);
        output.push_str(&self.monomorphized_code);
        output.push_str(&functions_code);
        output.push_str(&classes_code);
        output.push_str(&top_level_code);

        Ok(output)
    }

    fn transpile_function(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: &Type,
        body: &[Stmt],
    ) -> Result<String, String> {
        let mut func_decl = format!("{} {}(", self.c_type(return_type), name);
        let param_strs: Vec<String> = params
            .iter()
            .map(|p| format!("{} {}", self.c_type(&p.param_type), p.name))
            .collect();
        func_decl.push_str(&param_strs.join(", "));
        func_decl.push_str(") {\n");

        self.enter_block();
        let outer_var_env = self.var_env.clone();
        for p in params {
            self.var_env.insert(p.name.clone(), p.param_type.clone());
        }

        self.indent_level += 1;
        let mut body_code = String::new();
        for stmt in body {
            body_code.push_str(&self.transpile_statement(stmt)?);
        }
        let cleanups = self.exit_block();
        self.indent_level -= 1;
        self.var_env = outer_var_env;

        Ok(format!("{}{}{}{}}}\n", func_decl, body_code, cleanups, self.indent()))
    }

    fn transpile_class_method(
        &mut self,
        prefix_name: &str,
        params: &[Param],
        return_type: &Type,
        body: &[Stmt],
    ) -> Result<String, String> {
        let class_name = self.current_class.as_ref().unwrap().clone();
        let mut func_decl = format!("{} {}(", self.c_type(return_type), prefix_name);
        
        let param_strs: Vec<String> = params
            .iter()
            .map(|p| {
                if p.name == "self" {
                    format!("{} self", self.c_type(&Type::Custom(class_name.clone())))
                } else {
                    format!("{} {}", self.c_type(&p.param_type), p.name)
                }
            })
            .collect();
        func_decl.push_str(&param_strs.join(", "));
        func_decl.push_str(") {\n");

        self.enter_block();
        let outer_var_env = self.var_env.clone();
        for p in params {
            if p.name == "self" {
                self.var_env.insert("self".to_string(), Type::Custom(class_name.clone()));
            } else {
                self.var_env.insert(p.name.clone(), p.param_type.clone());
            }
        }

        self.indent_level += 1;
        let mut body_code = String::new();
        for stmt in body {
            body_code.push_str(&self.transpile_statement(stmt)?);
        }
        let cleanups = self.exit_block();
        self.indent_level -= 1;
        self.var_env = outer_var_env;

        Ok(format!("{}{}{}{}}}\n", func_decl, body_code, cleanups, self.indent()))
    }

    fn transpile_statement(&mut self, stmt: &Stmt) -> Result<String, String> {
        match stmt {
            Stmt::TupleUnpack { vars, init } => {
                let mut block_statements = Vec::new();
                self.enter_block();
                let init_code = self.flatten_expression(init, &mut block_statements)?;
                let cleanups = self.exit_block();

                self.temp_counter += 1;
                let temp_name = format!("_tmp_unpack_{}", self.temp_counter);

                let mut out = String::new();
                if !block_statements.is_empty() {
                    out.push_str(&format!("{}{{\n", self.indent()));
                    self.indent_level += 1;
                    for s in &block_statements {
                        out.push_str(&format!("{}{}\n", self.indent(), s));
                    }
                }

                out.push_str(&format!("{}PyObject* {} = to_py({});\n", self.indent(), temp_name, init_code));

                for (i, var_name) in vars.iter().enumerate() {
                    self.var_env.insert(var_name.clone(), Type::PyObject);
                    if let Some(scope) = self.scope_py_vars.last_mut() {
                        scope.push(var_name.clone());
                    }
                    out.push_str(&format!(
                        "{}PyObject* {} = PySequence_GetItem({}, {});\n",
                        self.indent(),
                        var_name,
                        temp_name,
                        i
                    ));
                }

                out.push_str(&format!("{}Py_XDECREF({});\n", self.indent(), temp_name));

                if !block_statements.is_empty() {
                    out.push_str(&cleanups);
                    self.indent_level -= 1;
                    out.push_str(&format!("{}}}\n", self.indent()));
                }

                Ok(out)
            }
            Stmt::VariableDecl { name, is_const, var_type, init } => {
                let inferred = if let Some(init_expr) = init {
                    let inf = self.infer_type(init_expr)?;
                    if let Some(declared) = var_type {
                        if !self.is_compatible(declared, &inf) {
                            return Err(format!(
                                "Type Error: Declared type '{}' for variable '{}' does not match initializer type '{}'",
                                declared, name, inf
                            ));
                        }
                    }
                    inf
                } else {
                    var_type.clone().ok_or_else(|| format!("Compilation Error: Variable '{}' has no type declaration and no initializer.", name))?
                };

                self.var_env.insert(name.clone(), inferred.clone());

                if inferred == Type::PyObject {
                    if let Some(scope) = self.scope_py_vars.last_mut() {
                        scope.push(name.clone());
                    }
                } else if inferred == Type::Custom("Matrix".to_string()) || inferred == Type::Custom("Matrix32".to_string())
                    || inferred == Type::Custom("Tensor".to_string()) || inferred == Type::Custom("Tensor32".to_string()) {
                    if let Some(scope) = self.scope_matrix_vars.last_mut() {
                        scope.push(name.clone());
                    }
                }

                let const_prefix = if *is_const { "const " } else { "" };
                if let Some(init_expr) = init {
                    let mut block_statements = Vec::new();
                    self.enter_block();
                    let init_code = self.flatten_expression(init_expr, &mut block_statements)?;
                    let is_matrix_or_tensor = inferred == Type::Custom("Matrix".to_string())
                        || inferred == Type::Custom("Matrix32".to_string())
                        || inferred == Type::Custom("Tensor".to_string())
                        || inferred == Type::Custom("Tensor32".to_string());
                    if is_matrix_or_tensor && init_code.starts_with("_tmp_") {
                        if let Some(scope) = self.scope_matrix_vars.last_mut() {
                            if let Some(pos) = scope.iter().position(|x| x == &init_code) {
                                scope.remove(pos);
                            }
                        }
                    }
                    let cleanups = self.exit_block();

                    if block_statements.is_empty() {
                        let mut out = format!(
                            "{}{}{} {} = {};\n",
                            self.indent(),
                            const_prefix,
                            self.c_type(&inferred),
                            name,
                            init_code
                        );
                        if inferred == Type::PyObject {
                            out.push_str(&format!("{}Py_XINCREF({});\n", self.indent(), name));
                        }
                        Ok(out)
                    } else {
                        let default_val = match inferred {
                            Type::Int => "0",
                            Type::Float => "0.0",
                            Type::String => "NULL",
                            Type::Bool => "false",
                            _ => "NULL",
                        };
                        let mut out = format!(
                            "{}{} {} = {};\n",
                            self.indent(),
                            self.c_type(&inferred),
                            name,
                            default_val
                        );
                        out.push_str(&format!("{}{{\n", self.indent()));
                        self.indent_level += 1;
                        for stmt in block_statements {
                            out.push_str(&format!("{}{}\n", self.indent(), stmt));
                        }
                        out.push_str(&format!("{}{} = {};\n", self.indent(), name, init_code));
                        if inferred == Type::PyObject {
                            out.push_str(&format!("{}Py_XINCREF({});\n", self.indent(), name));
                        }
                        out.push_str(&cleanups);
                        self.indent_level -= 1;
                        out.push_str(&format!("{}}}\n", self.indent()));
                        Ok(out)
                    }
                } else {
                    Ok(format!(
                        "{}{}{} {};\n",
                        self.indent(),
                        const_prefix,
                        self.c_type(&inferred),
                        name
                    ))
                }
            }
            Stmt::AssignStmt { lhs, value } => {
                match lhs {
                    Expr::Identifier(name) => {
                        let var_t = if let Some(t) = self.var_env.get(name) {
                            t.clone()
                        } else {
                            return Err(format!("Compilation Error: Variable '{}' is not declared in this scope", name));
                        };

                        let val_t = self.infer_type(value)?;
                        if !self.is_compatible(&var_t, &val_t) {
                            return Err(format!(
                                "Type Error: Cannot assign value of type '{}' to variable '{}' of type '{}'",
                                val_t, name, var_t
                            ));
                        }

                        let mut block_statements = Vec::new();
                        self.enter_block();
                        let val_code = self.flatten_expression(value, &mut block_statements)?;
                        let is_matrix_or_tensor = var_t == Type::Custom("Matrix".to_string())
                            || var_t == Type::Custom("Matrix32".to_string())
                            || var_t == Type::Custom("Tensor".to_string())
                            || var_t == Type::Custom("Tensor32".to_string());
                        if is_matrix_or_tensor && val_code.starts_with("_tmp_") {
                            if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                if let Some(pos) = scope.iter().position(|x| x == &val_code) {
                                    scope.remove(pos);
                                }
                            }
                        }
                        let cleanups = self.exit_block();

                        if block_statements.is_empty() {
                            let mut out = format!("{}{} = {};\n", self.indent(), name, val_code);
                            if var_t == Type::PyObject {
                                out.push_str(&format!("{}Py_XINCREF({});\n", self.indent(), name));
                            }
                            Ok(out)
                        } else {
                            let mut out = format!("{}{{\n", self.indent());
                            self.indent_level += 1;
                            for stmt in block_statements {
                                out.push_str(&format!("{}{}\n", self.indent(), stmt));
                            }
                            out.push_str(&format!("{}{} = {};\n", self.indent(), name, val_code));
                            if var_t == Type::PyObject {
                                out.push_str(&format!("{}Py_XINCREF({});\n", self.indent(), name));
                            }
                            out.push_str(&cleanups);
                            self.indent_level -= 1;
                            out.push_str(&format!("{}}}\n", self.indent()));
                            Ok(out)
                        }
                    }
                    Expr::MemberAccess { object, member } => {
                        let obj_t = self.infer_type(object)?;
                        let mut block_statements = Vec::new();
                        self.enter_block();
                        let obj_code = self.flatten_expression(object, &mut block_statements)?;
                        let val_code = self.flatten_expression(value, &mut block_statements)?;
                        let cleanups = self.exit_block();

                        if let Type::Custom(class_name) = obj_t {
                            if let Some((prop_t, access_path)) = self.resolve_property(&class_name, member) {
                                let val_t = self.infer_type(value)?;
                                if !self.is_compatible(&prop_t, &val_t) {
                                    return Err(format!(
                                        "Type Error: Cannot assign value of type '{}' to property '{}' of type '{}'",
                                        val_t, member, prop_t
                                    ));
                                }
                                if block_statements.is_empty() {
                                    Ok(format!("{}{}->{} = {};\n", self.indent(), obj_code, access_path, val_code))
                                } else {
                                    let mut out = format!("{}{{\n", self.indent());
                                    self.indent_level += 1;
                                    for stmt in block_statements {
                                        out.push_str(&format!("{}{}\n", self.indent(), stmt));
                                    }
                                    out.push_str(&format!("{}{}->{} = {};\n", self.indent(), obj_code, access_path, val_code));
                                    out.push_str(&cleanups);
                                    self.indent_level -= 1;
                                    out.push_str(&format!("{}}}\n", self.indent()));
                                    Ok(out)
                                }
                            } else {
                                Err(format!("Compilation Error: Property '{}' not found in class '{}'", member, class_name))
                            }
                        } else if obj_t == Type::PyObject {
                            if block_statements.is_empty() {
                                Ok(format!("{}PyObject_SetAttrString({}, \"{}\", to_py({}));\n", self.indent(), obj_code, member, val_code))
                            } else {
                                let mut out = format!("{}{{\n", self.indent());
                                self.indent_level += 1;
                                for stmt in block_statements {
                                    out.push_str(&format!("{}{}\n", self.indent(), stmt));
                                }
                                out.push_str(&format!("{}PyObject_SetAttrString({}, \"{}\", to_py({}));\n", self.indent(), obj_code, member, val_code));
                                out.push_str(&cleanups);
                                self.indent_level -= 1;
                                out.push_str(&format!("{}}}\n", self.indent()));
                                Ok(out)
                            }
                        } else {
                            Err(format!("Compilation Error: Member assignment on non-class instance of type '{:?}'", obj_t))
                        }
                    }
                    Expr::Subscript { object, index } => {
                        let obj_t = self.infer_type(object)?;
                        let mut block_statements = Vec::new();
                        self.enter_block();
                        let obj_code = self.flatten_expression(object, &mut block_statements)?;
                        
                        let is_tensor_direct = if let Type::Custom(ref class_name) = obj_t {
                            (class_name == "Tensor" || class_name.starts_with("Tensor<") || class_name == "Tensor32") && self.get_direct_indices(index).is_some()
                        } else {
                            false
                        };

                        let idx_info = if is_tensor_direct {
                            let elts = self.get_direct_indices(index).unwrap();
                            let mut idx_codes = Vec::new();
                            for elt in &elts {
                                idx_codes.push(self.flatten_expression(elt, &mut block_statements)?);
                            }
                            let idx_count = idx_codes.len();
                            let array_literal = format!("(const int[]){{{}}}", idx_codes.join(", "));
                            Some((idx_count, array_literal))
                        } else {
                            None
                        };

                        let index_code = if idx_info.is_none() {
                            Some(self.flatten_expression(index, &mut block_statements)?)
                        } else {
                            None
                        };

                        let val_code = self.flatten_expression(value, &mut block_statements)?;
                        let cleanups = self.exit_block();

                        if obj_t == Type::PyObject {
                            let idx_c = index_code.unwrap();
                            if block_statements.is_empty() {
                                Ok(format!("{}PyObject_SetItem({}, to_py({}), to_py({}));\n", self.indent(), obj_code, idx_c, val_code))
                            } else {
                                let mut out = format!("{}{{\n", self.indent());
                                self.indent_level += 1;
                                for stmt in block_statements {
                                    out.push_str(&format!("{}{}\n", self.indent(), stmt));
                                }
                                out.push_str(&format!("{}PyObject_SetItem({}, to_py({}), to_py({}));\n", self.indent(), obj_code, idx_c, val_code));
                                out.push_str(&cleanups);
                                self.indent_level -= 1;
                                out.push_str(&format!("{}}}\n", self.indent()));
                                Ok(out)
                            }
                        } else if let Type::Custom(ref class_name) = obj_t {
                            let is_tensor = class_name == "Tensor" || class_name.starts_with("Tensor<");
                            let is_tensor32 = class_name == "Tensor32";
                            if is_tensor || is_tensor32 {
                                let is_f32 = is_tensor32 || class_name.contains("float32");
                                let cast_type = if is_f32 { "float" } else { "double" };
                                if let Some((idx_count, array_literal)) = idx_info {
                                    let setter_fn = if is_f32 { "tensor32_set_direct" } else { "tensor_set_direct" };
                                    if block_statements.is_empty() {
                                        Ok(format!("{}{}({}, {}, {}, ({})({}));\n", self.indent(), setter_fn, obj_code, idx_count, array_literal, cast_type, val_code))
                                    } else {
                                        let mut out = format!("{}{{\n", self.indent());
                                        self.indent_level += 1;
                                        for stmt in block_statements {
                                            out.push_str(&format!("{}{}\n", self.indent(), stmt));
                                        }
                                        out.push_str(&format!("{}{}({}, {}, {}, ({})({}));\n", self.indent(), setter_fn, obj_code, idx_count, array_literal, cast_type, val_code));
                                        out.push_str(&cleanups);
                                        self.indent_level -= 1;
                                        out.push_str(&format!("{}}}\n", self.indent()));
                                        Ok(out)
                                    }
                                } else {
                                    let setter_fn = if is_f32 { "tensor32_set" } else { "tensor_set" };
                                    let idx_c = index_code.unwrap();
                                    if block_statements.is_empty() {
                                        Ok(format!("{}{}({}, to_py({}), ({})({}));\n", self.indent(), setter_fn, obj_code, idx_c, cast_type, val_code))
                                    } else {
                                        let mut out = format!("{}{{\n", self.indent());
                                        self.indent_level += 1;
                                        for stmt in block_statements {
                                            out.push_str(&format!("{}{}\n", self.indent(), stmt));
                                        }
                                        out.push_str(&format!("{}{}({}, to_py({}), ({})({}));\n", self.indent(), setter_fn, obj_code, idx_c, cast_type, val_code));
                                        out.push_str(&cleanups);
                                        self.indent_level -= 1;
                                        out.push_str(&format!("{}}}\n", self.indent()));
                                        Ok(out)
                                    }
                                }
                            } else {
                                Err(format!("Compilation Error: Subscript assignment on non-Python/Tensor object of type '{:?}'", obj_t))
                            }
                        } else {
                            Err(format!("Compilation Error: Subscript assignment on non-Python/Tensor object of type '{:?}'", obj_t))
                        }
                    }
                    _ => Err("Compilation Error: Invalid left-hand side of assignment".into())
                }
            }
            Stmt::IfStmt { condition, then_branch, else_branch } => {
                let cond_t = self.infer_type(condition)?;
                if cond_t != Type::Bool && cond_t != Type::PyObject {
                    return Err(format!("Type Error: If condition must be of type 'bool' or 'PyObject', found '{}'", cond_t));
                }

                let mut block_statements = Vec::new();
                self.enter_block();
                let cond_code = self.flatten_expression(condition, &mut block_statements)?;
                let cleanups_cond = self.exit_block();

                // If it is a PyObject, evaluate truthiness using CPython PyObject_IsTrue
                let final_cond_code = if cond_t == Type::PyObject {
                    format!("PyObject_IsTrue({})", cond_code)
                } else {
                    cond_code
                };

                let mut output = String::new();

                if !block_statements.is_empty() {
                    output.push_str(&format!("{}{{\n", self.indent()));
                    self.indent_level += 1;
                    for stmt in &block_statements {
                        output.push_str(&format!("{}{}\n", self.indent(), stmt));
                    }
                }

                output.push_str(&format!("{}if ({}) {{\n", self.indent(), final_cond_code));

                let outer_env = self.var_env.clone();
                self.indent_level += 1;
                self.enter_block();
                for s in then_branch {
                    output.push_str(&self.transpile_statement(s)?);
                }
                let cleanups_then = self.exit_block();
                output.push_str(&cleanups_then);
                self.indent_level -= 1;
                self.var_env = outer_env;

                output.push_str(&format!("{}}}", self.indent()));

                if let Some(else_stmts) = else_branch {
                    output.push_str(" else {\n");
                    let outer_env_else = self.var_env.clone();
                    self.indent_level += 1;
                    self.enter_block();
                    for s in else_stmts {
                        output.push_str(&self.transpile_statement(s)?);
                    }
                    let cleanups_else = self.exit_block();
                    output.push_str(&cleanups_else);
                    self.indent_level -= 1;
                    self.var_env = outer_env_else;
                    output.push_str(&format!("{}}}\n", self.indent()));
                } else {
                    output.push_str("\n");
                }

                if !block_statements.is_empty() {
                    output.push_str(&cleanups_cond);
                    self.indent_level -= 1;
                    output.push_str(&format!("{}}}\n", self.indent()));
                }

                Ok(output)
            }
            Stmt::WhileStmt { condition, body } => {
                let cond_t = self.infer_type(condition)?;
                if cond_t != Type::Bool && cond_t != Type::PyObject {
                    return Err(format!("Type Error: While condition must be of type 'bool' or 'PyObject', found '{}'", cond_t));
                }

                let mut block_statements = Vec::new();
                self.enter_block();
                let cond_code = self.flatten_expression(condition, &mut block_statements)?;
                let cleanups_cond = self.exit_block();

                let final_cond_code = if cond_t == Type::PyObject {
                    format!("PyObject_IsTrue({})", cond_code)
                } else {
                    cond_code
                };

                let mut output = String::new();

                if !block_statements.is_empty() {
                    output.push_str(&format!("{}{{\n", self.indent()));
                    self.indent_level += 1;
                    for stmt in &block_statements {
                        output.push_str(&format!("{}{}\n", self.indent(), stmt));
                    }
                }

                output.push_str(&format!("{}while ({}) {{\n", self.indent(), final_cond_code));

                let outer_env = self.var_env.clone();
                self.indent_level += 1;
                self.enter_block();
                for s in body {
                    output.push_str(&self.transpile_statement(s)?);
                }

                if !block_statements.is_empty() {
                    for stmt in &block_statements {
                        output.push_str(&format!("{}{}\n", self.indent(), stmt));
                    }
                }

                let cleanups_body = self.exit_block();
                output.push_str(&cleanups_body);
                self.indent_level -= 1;
                self.var_env = outer_env;

                output.push_str(&format!("{}}}\n", self.indent()));

                if !block_statements.is_empty() {
                    output.push_str(&cleanups_cond);
                    self.indent_level -= 1;
                    output.push_str(&format!("{}}}\n", self.indent()));
                }
                Ok(output)
            }
            Stmt::ForStmt { var_name, iterable, body } => {
                let mut is_range = false;
                let mut start_expr = None;
                let mut end_expr = None;
                let mut step_expr = None;
                
                if let Expr::Call { name, args, .. } = iterable {
                    if name == "range" {
                        is_range = true;
                        if args.len() == 1 {
                            start_expr = Some(Expr::Literal(Literal::Int(0)));
                            end_expr = Some(args[0].clone());
                        } else if args.len() == 2 {
                            start_expr = Some(args[0].clone());
                            end_expr = Some(args[1].clone());
                        } else if args.len() == 3 {
                            start_expr = Some(args[0].clone());
                            end_expr = Some(args[1].clone());
                            step_expr = Some(args[2].clone());
                        }
                    }
                }
                
                let mut out = String::new();
                
                if is_range {
                    let start = start_expr.unwrap();
                    let end = end_expr.unwrap();
                    
                    let mut block_statements = Vec::new();
                    self.enter_block();
                    let start_code = self.flatten_expression(&start, &mut block_statements)?;
                    let end_code = self.flatten_expression(&end, &mut block_statements)?;
                    let step_code = if let Some(step) = &step_expr {
                        Some(self.flatten_expression(step, &mut block_statements)?)
                    } else {
                        None
                    };
                    let cleanups_range = self.exit_block();
                    
                    if !block_statements.is_empty() {
                        out.push_str(&format!("{}{{\n", self.indent()));
                        self.indent_level += 1;
                        for stmt in &block_statements {
                            out.push_str(&format!("{}{}\n", self.indent(), stmt));
                        }
                    }
                    
                    let outer_env = self.var_env.clone();
                    self.var_env.insert(var_name.clone(), Type::Int);
                    
                    let step_incr = if let Some(sc) = step_code {
                        format!("{} += {}", var_name, sc)
                    } else {
                        format!("{}++", var_name)
                    };
                    
                    out.push_str(&format!(
                        "{}for (int {} = {}; {} < {}; {}) {{\n",
                        self.indent(),
                        var_name,
                        start_code,
                        var_name,
                        end_code,
                        step_incr
                    ));
                    
                    self.indent_level += 1;
                    self.enter_block();
                    for s in body {
                        out.push_str(&self.transpile_statement(s)?);
                    }
                    let cleanups_body = self.exit_block();
                    out.push_str(&cleanups_body);
                    self.indent_level -= 1;
                    self.var_env = outer_env;
                    
                    out.push_str(&format!("{}}}", self.indent()));
                    
                    if !block_statements.is_empty() {
                        out.push_str(&cleanups_range);
                        self.indent_level -= 1;
                        out.push_str(&format!("{}}}\n", self.indent()));
                    } else {
                        out.push_str("\n");
                    }
                } else {
                    // Standard Python-like iteration
                    let mut block_statements = Vec::new();
                    self.enter_block();
                    let iter_obj_code = self.flatten_expression(iterable, &mut block_statements)?;
                    let cleanups_iter = self.exit_block();
                    
                    if !block_statements.is_empty() {
                        out.push_str(&format!("{}{{\n", self.indent()));
                        self.indent_level += 1;
                        for stmt in &block_statements {
                            out.push_str(&format!("{}{}\n", self.indent(), stmt));
                        }
                    }
                    
                    self.temp_counter += 1;
                    let iter_name = format!("_iter_{}", self.temp_counter);
                    self.temp_counter += 1;
                    let item_name = format!("_item_{}", self.temp_counter);
                    
                    out.push_str(&format!("{}PyObject* {} = PyObject_GetIter(to_py({}));\n", self.indent(), iter_name, iter_obj_code));
                    out.push_str(&format!("{}if ({} != NULL) {{\n", self.indent(), iter_name));
                    self.indent_level += 1;
                    out.push_str(&format!("{}PyObject* {} = NULL;\n", self.indent(), item_name));
                    out.push_str(&format!("{}while (({} = PyIter_Next({})) != NULL) {{\n", self.indent(), item_name, iter_name));
                    
                    self.indent_level += 1;
                    self.enter_block();
                    let outer_env = self.var_env.clone();
                    self.var_env.insert(var_name.clone(), Type::PyObject);
                    out.push_str(&format!("{}PyObject* {} = {};\n", self.indent(), var_name, item_name));
                    
                    for s in body {
                        out.push_str(&self.transpile_statement(s)?);
                    }
                    
                    let cleanups_body = self.exit_block();
                    out.push_str(&cleanups_body);
                    out.push_str(&format!("{}Py_DECREF({});\n", self.indent(), item_name));
                    self.var_env = outer_env;
                    self.indent_level -= 1;
                    out.push_str(&format!("{}}}\n", self.indent()));
                    out.push_str(&format!("{}Py_DECREF({});\n", self.indent(), iter_name));
                    self.indent_level -= 1;
                    out.push_str(&format!("{}}}\n", self.indent()));
                    
                    if !block_statements.is_empty() {
                        out.push_str(&cleanups_iter);
                        self.indent_level -= 1;
                        out.push_str(&format!("{}}}\n", self.indent()));
                    }
                }
                Ok(out)
            }
            Stmt::TryCatchStmt { try_branch, catch_var, catch_branch, finally_branch } => {
                let mut out = String::new();
                out.push_str(&format!("{}{{\n", self.indent()));
                self.indent_level += 1;
                
                out.push_str(&format!("{}jmp_buf _local_buf;\n", self.indent()));
                out.push_str(&format!("{}jmp_buf* _prev_buf = mps_err_buf;\n", self.indent()));
                out.push_str(&format!("{}mps_err_buf = &_local_buf;\n", self.indent()));
                
                out.push_str(&format!("{}if (setjmp(_local_buf) == 0) {{\n", self.indent()));
                
                self.indent_level += 1;
                self.enter_block();
                for s in try_branch {
                    out.push_str(&self.transpile_statement(s)?);
                }
                let cleanups_try = self.exit_block();
                out.push_str(&cleanups_try);
                out.push_str(&format!("{}mps_err_buf = _prev_buf;\n", self.indent()));
                self.indent_level -= 1;
                
                out.push_str(&format!("{}}} else {{\n", self.indent()));
                
                self.indent_level += 1;
                out.push_str(&format!("{}mps_err_buf = _prev_buf;\n", self.indent()));
                out.push_str(&format!("{}MPS_Error {} = mps_last_error;\n", self.indent(), catch_var));
                
                self.enter_block();
                let outer_env = self.var_env.clone();
                self.var_env.insert(catch_var.clone(), Type::Custom("Error".to_string()));
                
                for s in catch_branch {
                    out.push_str(&self.transpile_statement(s)?);
                }
                let cleanups_catch = self.exit_block();
                out.push_str(&cleanups_catch);
                self.var_env = outer_env;
                self.indent_level -= 1;
                out.push_str(&format!("{}}}\n", self.indent()));
                
                if let Some(fb) = finally_branch {
                    out.push_str(&format!("{}// finally\n", self.indent()));
                    self.enter_block();
                    for s in fb {
                        out.push_str(&self.transpile_statement(s)?);
                    }
                    let cleanups_finally = self.exit_block();
                    out.push_str(&cleanups_finally);
                }
                
                self.indent_level -= 1;
                out.push_str(&format!("{}}}\n", self.indent()));
                Ok(out)
            }
            Stmt::RaiseStmt(expr) => {
                let mut block_statements = Vec::new();
                self.enter_block();
                let expr_code = self.flatten_expression(expr, &mut block_statements)?;
                let cleanups = self.exit_block();
                
                let mut out = String::new();
                if !block_statements.is_empty() {
                    out.push_str(&format!("{}{{\n", self.indent()));
                    self.indent_level += 1;
                    for stmt in &block_statements {
                        out.push_str(&format!("{}{}\n", self.indent(), stmt));
                    }
                }
                out.push_str(&format!("{}mps_raise(mps_to_string({}));\n", self.indent(), expr_code));
                if !cleanups.is_empty() {
                    out.push_str(&cleanups);
                }
                if !block_statements.is_empty() {
                    self.indent_level -= 1;
                    out.push_str(&format!("{}}}\n", self.indent()));
                }
                Ok(out)
            }
            Stmt::MatchStmt { value, cases } => {
                let mut block_statements = Vec::new();
                self.enter_block();
                let value_code = self.flatten_expression(value, &mut block_statements)?;
                let cleanups = self.exit_block();
                
                let value_t = self.infer_type(value)?;
                
                let mut out = String::new();
                if !block_statements.is_empty() {
                    out.push_str(&format!("{}{{\n", self.indent()));
                    self.indent_level += 1;
                    for stmt in &block_statements {
                        out.push_str(&format!("{}{}\n", self.indent(), stmt));
                    }
                }
                
                self.temp_counter += 1;
                let matched_var = format!("_match_val_{}", self.temp_counter);
                out.push_str(&format!("{}{} {} = {};\n", self.indent(), self.c_type(&value_t), matched_var, value_code));
                
                for (idx, case) in cases.iter().enumerate() {
                    let cond = match &case.pattern {
                        MatchPattern::Literal(lit) => {
                            let lit_code = match lit {
                                Literal::Int(n) => n.to_string(),
                                Literal::Float(f) => f.to_string(),
                                Literal::String(s) => format!("\"{}\"", Self::escape_string(s)),
                                Literal::Bool(b) => b.to_string(),
                                Literal::Null => "NULL".to_string(),
                            };
                            if value_t == Type::String && lit != &Literal::Null {
                                format!("strcmp({}, {}) == 0", matched_var, lit_code)
                            } else {
                                format!("{} == {}", matched_var, lit_code)
                            }
                        }
                        MatchPattern::Wildcard => "true".to_string(),
                    };
                    
                    if idx == 0 {
                        out.push_str(&format!("{}if ({}) {{\n", self.indent(), cond));
                    } else {
                        out.push_str(&format!("}} else if ({}) {{\n", cond));
                    }
                    
                    self.indent_level += 1;
                    self.enter_block();
                    for s in &case.body {
                        out.push_str(&self.transpile_statement(s)?);
                    }
                    let cleanups_case = self.exit_block();
                    out.push_str(&cleanups_case);
                    self.indent_level -= 1;
                }
                if !cases.is_empty() {
                    out.push_str(&format!("{}}}\n", self.indent()));
                }
                
                if !cleanups.is_empty() {
                    out.push_str(&cleanups);
                }
                if !block_statements.is_empty() {
                    self.indent_level -= 1;
                    out.push_str(&format!("{}}}\n", self.indent()));
                }
                Ok(out)
            }
            Stmt::ExprStmt(expr) => {
                let mut block_statements = Vec::new();
                self.enter_block();
                let expr_code = self.flatten_expression(expr, &mut block_statements)?;
                let cleanups = self.exit_block();

                if block_statements.is_empty() {
                    if expr_code.is_empty() {
                        Ok("".to_string())
                    } else {
                        Ok(format!("{}{};\n", self.indent(), expr_code))
                    }
                } else {
                    let mut out = format!("{}{{\n", self.indent());
                    self.indent_level += 1;
                    for stmt in block_statements {
                        out.push_str(&format!("{}{}\n", self.indent(), stmt));
                    }
                    if !expr_code.is_empty() {
                        out.push_str(&format!("{}{};\n", self.indent(), expr_code));
                    }
                    out.push_str(&cleanups);
                    self.indent_level -= 1;
                    out.push_str(&format!("{}}}\n", self.indent()));
                    Ok(out)
                }
            }
            Stmt::ReturnStmt(opt_expr) => {
                if let Some(expr) = opt_expr {
                    let mut block_statements = Vec::new();
                    self.enter_block();
                    let expr_code = self.flatten_expression(expr, &mut block_statements)?;
                    let cleanups_temp = self.exit_block();

                    let returned_var = match expr {
                        Expr::Identifier(name) => Some(name.as_str()),
                        _ => None,
                    };
                    let all_scope_cleanups = self.exit_all_scopes(returned_var);

                    if block_statements.is_empty() {
                        let mut out = all_scope_cleanups;
                        out.push_str(&format!("{}return {};\n", self.indent(), expr_code));
                        Ok(out)
                    } else {
                        let return_t = self.infer_type(expr)?;
                        if return_t == Type::PyObject {
                            let mut out = format!("{}{{\n", self.indent());
                            self.indent_level += 1;
                            for stmt in block_statements {
                                out.push_str(&format!("{}{}\n", self.indent(), stmt));
                            }
                            out.push_str(&format!("{}PyObject* _ret_val = {};\n", self.indent(), expr_code));
                            out.push_str(&cleanups_temp);
                            out.push_str(&all_scope_cleanups);
                            out.push_str(&format!("{}return _ret_val;\n", self.indent()));
                            self.indent_level -= 1;
                            out.push_str(&format!("{}}}\n", self.indent()));
                            Ok(out)
                        } else {
                            let mut out = format!("{}{{\n", self.indent());
                            self.indent_level += 1;
                            for stmt in block_statements {
                                out.push_str(&format!("{}{}\n", self.indent(), stmt));
                            }
                            let c_ret_t = self.c_type(&return_t);
                            out.push_str(&format!("{}{} _ret_val = {};\n", self.indent(), c_ret_t, expr_code));
                            out.push_str(&cleanups_temp);
                            out.push_str(&all_scope_cleanups);
                            out.push_str(&format!("{}return _ret_val;\n", self.indent()));
                            self.indent_level -= 1;
                            out.push_str(&format!("{}}}\n", self.indent()));
                            Ok(out)
                        }
                    }
                } else {
                    let all_scope_cleanups = self.exit_all_scopes(None);
                    let mut out = all_scope_cleanups;
                    out.push_str(&format!("{}return;\n", self.indent()));
                    Ok(out)
                }
            }
            Stmt::BreakStmt => {
                Ok(format!("{}break;\n", self.indent()))
            }
            Stmt::ContinueStmt => {
                Ok(format!("{}continue;\n", self.indent()))
            }
            Stmt::TraitDecl { .. } => Ok("".to_string()),
            Stmt::FunctionDecl { .. } | Stmt::ClassDecl { .. } | Stmt::PyImport { .. } | Stmt::Import { .. } | Stmt::FromImport { .. } => {
                Err("Internal Error: Declaration matched at statement transpilation level".into())
            }
        }
    }

    fn flatten_expression(&mut self, expr: &Expr, block_statements: &mut Vec<String>) -> Result<String, String> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(n) => Ok(n.to_string()),
                Literal::Float(f) => Ok(f.to_string()),
                Literal::String(s) => Ok(format!("\"{}\"", Self::escape_string(s))),
                Literal::Bool(b) => Ok(b.to_string()),
                Literal::Null => Ok("NULL".to_string()),
            },
            Expr::Unary { op, operand } => {
                let operand_code = self.flatten_expression(operand, block_statements)?;
                match op {
                    UnaryOp::Neg => Ok(format!("(-{})", operand_code)),
                    UnaryOp::Not => Ok(format!("(!{})", operand_code)),
                }
            }
            Expr::FString { parts } => {
                if parts.is_empty() {
                    return Ok("\"\"".to_string());
                }
                let mut current_code = None;
                for part in parts {
                    let part_code = match part {
                        FStringPart::Text(s) => {
                            format!("\"{}\"", Self::escape_string(s))
                        }
                        FStringPart::Expr(expr) => {
                            let expr_code = self.flatten_expression(expr, block_statements)?;
                            let expr_t = self.infer_type(expr)?;
                            if expr_t == Type::String {
                                expr_code
                            } else {
                                format!("mps_to_string({})", expr_code)
                            }
                        }
                    };
                    if let Some(prev_code) = current_code {
                        current_code = Some(format!("mps_str_concat({}, {})", prev_code, part_code));
                    } else {
                        current_code = Some(part_code);
                    }
                }
                Ok(current_code.unwrap())
            }
            Expr::OptionalMemberAccess { object, member } => {
                let obj_code = self.flatten_expression(object, block_statements)?;
                let obj_t = self.infer_type(object)?;
                let inner_t = match obj_t {
                    Type::Optional(inner) => *inner,
                    other => other,
                };
                
                let member_access = if let Type::Custom(class_name) = &inner_t {
                    if let Some((_, access_path)) = self.resolve_property(class_name, member) {
                        format!("->{}", access_path)
                    } else {
                        format!("->{}", member)
                    }
                } else {
                    format!("->{}", member)
                };
                
                self.temp_counter += 1;
                let temp_name = format!("_tmp_opt_access_{}", self.temp_counter);
                
                let res_t = self.infer_type(expr)?;
                let c_res_t = self.c_type(&res_t);
                
                block_statements.push(format!("{} {};", c_res_t, temp_name));
                block_statements.push(format!(
                    "if ({} != NULL) {{ {} = {}{}; }} else {{ {} = NULL; }}",
                    obj_code, temp_name, obj_code, member_access, temp_name
                ));
                
                Ok(temp_name)
            }
            Expr::OptionalMemberCall { object, method, args } => {
                let obj_code = self.flatten_expression(object, block_statements)?;
                let obj_t = self.infer_type(object)?;
                let inner_t = match obj_t {
                    Type::Optional(inner) => *inner,
                    other => other,
                };
                
                let mut arg_codes = Vec::new();
                if let Type::Custom(class_name) = &inner_t {
                    arg_codes.push(format!("({}*){}", class_name, obj_code));
                } else {
                    arg_codes.push(obj_code.clone());
                }
                for arg in args {
                    arg_codes.push(self.flatten_expression(arg, block_statements)?);
                }
                
                let method_call = if let Type::Custom(class_name) = &inner_t {
                    if let Some((defining_class, _)) = self.resolve_method(class_name, method) {
                        format!("{}_{}({})", defining_class, method, arg_codes.join(", "))
                    } else {
                        format!("(NULL)")
                    }
                } else {
                    format!("(NULL)")
                };
                
                self.temp_counter += 1;
                let temp_name = format!("_tmp_opt_call_{}", self.temp_counter);
                
                let res_t = self.infer_type(expr)?;
                let c_res_t = self.c_type(&res_t);
                
                block_statements.push(format!("{} {};", c_res_t, temp_name));
                block_statements.push(format!(
                    "if ({} != NULL) {{ {} = {}; }} else {{ {} = NULL; }}",
                    obj_code, temp_name, method_call, temp_name
                ));
                
                Ok(temp_name)
            }
            Expr::Identifier(name) => Ok(name.clone()),
            Expr::Binary { op, left, right } => {
                let left_t = self.infer_type(left)?;
                let right_t = self.infer_type(right)?;

                let is_tensor_left = match &left_t { Type::Custom(c) => c == "Tensor" || c == "Tensor32" || c.starts_with("Tensor<"), _ => false };
                let is_tensor_right = match &right_t { Type::Custom(c) => c == "Tensor" || c == "Tensor32" || c.starts_with("Tensor<"), _ => false };
                if is_tensor_left && is_tensor_right {
                    let is_f32_l = match &left_t { Type::Custom(c) => c == "Tensor32" || c.contains("float32"), _ => false };
                    let is_f32_r = match &right_t { Type::Custom(c) => c == "Tensor32" || c.contains("float32"), _ => false };
                    let is_f32 = is_f32_l || is_f32_r;
                    
                    let left_code = self.flatten_expression(left, block_statements)?;
                    let right_code = self.flatten_expression(right, block_statements)?;
                    
                    let op_fn = match op {
                        BinOp::Add => if is_f32 { "tensor32_add" } else { "tensor_add" },
                        BinOp::Sub => if is_f32 { "tensor32_sub" } else { "tensor_sub" },
                        BinOp::Mul => if is_f32 { "tensor32_mul" } else { "tensor_mul" },
                        BinOp::Div => if is_f32 { "tensor32_div" } else { "tensor_div" },
                        BinOp::MatMul => if is_f32 { "tensor32_matmul" } else { "tensor_matmul" },
                        _ => return Err(format!("Compilation Error: Operator '{}' not supported on Tensors", op)),
                    };
                    
                    self.temp_counter += 1;
                    let temp_name = format!("_tmp_{}", self.temp_counter);
                    let tensor_type = if is_f32 { "MPSTensor32*" } else { "MPSTensor*" };
                    block_statements.push(format!("{} {} = {}({}, {});", tensor_type, temp_name, op_fn, left_code, right_code));
                    self.var_env.insert(temp_name.clone(), Type::Custom(if is_f32 { "Tensor32".to_string() } else { "Tensor".to_string() }));
                    if let Some(scope) = self.scope_matrix_vars.last_mut() {
                        scope.push(temp_name.clone());
                    }
                    return Ok(temp_name);
                }

                if *op == BinOp::MatMul {
                    let is_matrix_left = match &left_t { Type::Custom(c) => c == "Matrix" || c == "Matrix32" || c.starts_with("Matrix<"), _ => false };
                    let is_matrix_right = match &right_t { Type::Custom(c) => c == "Matrix" || c == "Matrix32" || c.starts_with("Matrix<"), _ => false };
                    if is_matrix_left && is_matrix_right {
                        let is_f32_l = match &left_t { Type::Custom(c) => c == "Matrix32" || c.contains("float32"), _ => false };
                        let is_f32_r = match &right_t { Type::Custom(c) => c == "Matrix32" || c.contains("float32"), _ => false };
                        let is_f32 = is_f32_l || is_f32_r;
                        let left_code = self.flatten_expression(left, block_statements)?;
                        let right_code = self.flatten_expression(right, block_statements)?;
                        let op_fn = if is_f32 { "matrix32_mul" } else { "matrix_mul" };
                        
                        self.temp_counter += 1;
                        let temp_name = format!("_tmp_{}", self.temp_counter);
                        let matrix_type = if is_f32 { "MPSMatrix32*" } else { "MPSMatrix*" };
                        block_statements.push(format!("{} {} = {}({}, {});", matrix_type, temp_name, op_fn, left_code, right_code));
                        self.var_env.insert(temp_name.clone(), Type::Custom(if is_f32 { "Matrix32".to_string() } else { "Matrix".to_string() }));
                        if let Some(scope) = self.scope_matrix_vars.last_mut() {
                            scope.push(temp_name.clone());
                        }
                        return Ok(temp_name);
                    }
                    
                    let is_tensor_l = match &left_t { Type::Custom(c) => c == "Tensor" || c == "Tensor32" || c.starts_with("Tensor<"), _ => false };
                    let is_tensor_r = match &right_t { Type::Custom(c) => c == "Tensor" || c == "Tensor32" || c.starts_with("Tensor<"), _ => false };
                    if is_tensor_l && is_tensor_r {
                        let is_f32_l = match &left_t { Type::Custom(c) => c == "Tensor32" || c.contains("float32"), _ => false };
                        let is_f32_r = match &right_t { Type::Custom(c) => c == "Tensor32" || c.contains("float32"), _ => false };
                        let is_f32 = is_f32_l || is_f32_r;
                        let left_code = self.flatten_expression(left, block_statements)?;
                        let right_code = self.flatten_expression(right, block_statements)?;
                        let op_fn = if is_f32 { "tensor32_matmul" } else { "tensor_matmul" };
                        
                        self.temp_counter += 1;
                        let temp_name = format!("_tmp_{}", self.temp_counter);
                        let tensor_type = if is_f32 { "MPSTensor32*" } else { "MPSTensor*" };
                        block_statements.push(format!("{} {} = {}({}, {});", tensor_type, temp_name, op_fn, left_code, right_code));
                        self.var_env.insert(temp_name.clone(), Type::Custom(if is_f32 { "Tensor32".to_string() } else { "Tensor".to_string() }));
                        if let Some(scope) = self.scope_matrix_vars.last_mut() {
                            scope.push(temp_name.clone());
                        }
                        return Ok(temp_name);
                    }
                }

                if *op == BinOp::And || *op == BinOp::Or {
                    let left_code = self.flatten_expression(left, block_statements)?;
                    self.temp_counter += 1;
                    let temp_name = format!("_tmp_{}", self.temp_counter);
                    block_statements.push(format!("bool {} = false;", temp_name));
                    if *op == BinOp::And {
                        block_statements.push(format!("if ({}) {{", left_code));
                    } else {
                        block_statements.push(format!("if (!({})) {{", left_code));
                    }
                    let right_code = self.flatten_expression(right, block_statements)?;
                    block_statements.push(format!("{} = {};", temp_name, right_code));
                    block_statements.push("}".to_string());
                    return Ok(temp_name);
                }

                if *op == BinOp::Pow {
                    let left_code = self.flatten_expression(left, block_statements)?;
                    let right_code = self.flatten_expression(right, block_statements)?;
                    return Ok(format!("mps_pow({}, {})", left_code, right_code));
                }

                // String concatenation: string + anything → mps_str_concat
                if *op == BinOp::Add && (left_t == Type::String || right_t == Type::String) {
                    let left_code = self.flatten_expression(left, block_statements)?;
                    let right_code = self.flatten_expression(right, block_statements)?;
                    let left_coerced = if left_t == Type::String {
                        left_code
                    } else {
                        format!("mps_to_string({})", left_code)
                    };
                    let right_coerced = if right_t == Type::String {
                        right_code
                    } else {
                        format!("mps_to_string({})", right_code)
                    };
                    return Ok(format!("mps_str_concat({}, {})", left_coerced, right_coerced));
                }

                if *op == BinOp::Eq && left_t == Type::String {
                    let left_code = self.flatten_expression(left, block_statements)?;
                    let right_code = self.flatten_expression(right, block_statements)?;
                    return Ok(format!("(strcmp({}, {}) == 0)", left_code, right_code));
                }
                if *op == BinOp::Ne && left_t == Type::String {
                    let left_code = self.flatten_expression(left, block_statements)?;
                    let right_code = self.flatten_expression(right, block_statements)?;
                    return Ok(format!("(strcmp({}, {}) != 0)", left_code, right_code));
                }

                if let Type::Custom(class_name) = &left_t {
                    let op_method = match op {
                        BinOp::Add => Some("add"),
                        BinOp::Sub => Some("sub"),
                        BinOp::Mul => Some("mul"),
                        BinOp::Div => Some("div"),
                        BinOp::Percent => Some("mod"),
                        BinOp::Eq => Some("eq"),
                        BinOp::Ne => Some("ne"),
                        BinOp::Lt => Some("lt"),
                        BinOp::Le => Some("le"),
                        BinOp::Gt => Some("gt"),
                        BinOp::Ge => Some("ge"),
                        BinOp::MatMul => Some("matmul"),
                        BinOp::Pow | BinOp::And | BinOp::Or => None,
                    };
                    if let Some(method_name) = op_method {
                        if self.resolve_method(class_name, method_name).is_some() {
                            let rewrite = Expr::MemberCall {
                                object: left.clone(),
                                method: method_name.to_string(),
                                args: vec![*right.clone()],
                            };
                            return self.flatten_expression(&rewrite, block_statements);
                        }
                    }
                }

                let left_code = self.flatten_expression(left, block_statements)?;
                let right_code = self.flatten_expression(right, block_statements)?;
                let op_str = Self::binary_op_symbol(*op).unwrap();
                Ok(format!("({} {} {})", left_code, op_str, right_code))
            }
            Expr::Call { name, type_args, args } => {
                if name == "len" && args.len() == 1 {
                    let arg_code = self.flatten_expression(&args[0], block_statements)?;
                    let arg_t = self.infer_type(&args[0])?;
                    if arg_t == Type::String {
                        return Ok(format!("((int)strlen({}))", arg_code));
                    } else {
                        return Ok(format!("((int)PyObject_Size({}))", arg_code));
                    }
                }

                if name == "Matrix" {
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        arg_codes.push(self.flatten_expression(arg, block_statements)?);
                    }
                    return Ok(format!("matrix_new({})", arg_codes.join(", ")));
                }

                if name == "Matrix32" {
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        arg_codes.push(self.flatten_expression(arg, block_statements)?);
                    }
                    return Ok(format!("matrix32_new({})", arg_codes.join(", ")));
                }

                if name == "Tensor" {
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        arg_codes.push(self.flatten_expression(arg, block_statements)?);
                    }
                    return Ok(format!("tensor_new({})", arg_codes.join(", ")));
                }

                if name == "Tensor32" {
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        arg_codes.push(self.flatten_expression(arg, block_statements)?);
                    }
                    return Ok(format!("tensor32_new({})", arg_codes.join(", ")));
                }

                // Generic function call — monomorphize on the fly
                if !type_args.is_empty() {
                    if let Some((type_params, params, return_type, body)) = self.generic_templates.get(name).cloned() {
                        if type_params.len() == type_args.len() {
                            let mut mapping = std::collections::HashMap::new();
                            for (param, arg) in type_params.iter().zip(type_args.iter()) {
                                mapping.insert(param.clone(), arg.clone());
                            }
                            let concrete_name = format!("{}_{}", name, type_args.iter().map(|t| crate::typechecker::sanitize_type_name(t)).collect::<Vec<_>>().join("_"));

                            // Generate the concrete function if not already generated
                            if !self.monomorphized_set.contains(&concrete_name) {
                                self.monomorphized_set.insert(concrete_name.clone());
                                let concrete_params: Vec<Param> = params.iter().map(|p| Param {
                                    name: p.name.clone(),
                                    param_type: crate::typechecker::substitute_type(&p.param_type, &mapping),
                                }).collect();
                                let concrete_return = crate::typechecker::substitute_type(&return_type, &mapping);
                                let concrete_body: Vec<Stmt> = body.iter().map(|s| crate::typechecker::substitute_stmt(s, &mapping)).collect();

                                // Register in func_env for type inference
                                self.func_env.insert(concrete_name.clone(), concrete_return.clone());

                                let func_code = self.transpile_function(&concrete_name, &concrete_params, &concrete_return, &concrete_body)?;
                                self.monomorphized_code.push_str(&func_code);
                                self.monomorphized_code.push('\n');
                            }

                            // Generate the call
                            let mut arg_codes = Vec::new();
                            for arg in args {
                                arg_codes.push(self.flatten_expression(arg, block_statements)?);
                            }
                            let call_code = format!("{}({})", concrete_name, arg_codes.join(", "));
                            let return_t = self.func_env.get(&concrete_name).cloned().unwrap_or(Type::Void);
                            if return_t == Type::PyObject {
                                self.temp_counter += 1;
                                let temp_name = format!("_tmp_{}", self.temp_counter);
                                block_statements.push(format!("PyObject* {} = {};", temp_name, call_code));
                                if let Some(scope) = self.scope_py_vars.last_mut() {
                                    scope.push(temp_name.clone());
                                }
                                return Ok(temp_name);
                            } else if return_t == Type::Custom("Matrix".to_string()) {
                                self.temp_counter += 1;
                                let temp_name = format!("_tmp_{}", self.temp_counter);
                                block_statements.push(format!("MPSMatrix* {} = {};", temp_name, call_code));
                                if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                    scope.push(temp_name.clone());
                                }
                                return Ok(temp_name);
                            } else if return_t == Type::Custom("Matrix32".to_string()) {
                                self.temp_counter += 1;
                                let temp_name = format!("_tmp_{}", self.temp_counter);
                                block_statements.push(format!("MPSMatrix32* {} = {};", temp_name, call_code));
                                if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                    scope.push(temp_name.clone());
                                }
                                return Ok(temp_name);
                            } else if return_t == Type::Custom("Tensor".to_string()) {
                                self.temp_counter += 1;
                                let temp_name = format!("_tmp_{}", self.temp_counter);
                                block_statements.push(format!("MPSTensor* {} = {};", temp_name, call_code));
                                self.var_env.insert(temp_name.clone(), Type::Custom("Tensor".to_string()));
                                if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                    scope.push(temp_name.clone());
                                }
                                return Ok(temp_name);
                            } else if return_t == Type::Custom("Tensor32".to_string()) {
                                self.temp_counter += 1;
                                let temp_name = format!("_tmp_{}", self.temp_counter);
                                block_statements.push(format!("MPSTensor32* {} = {};", temp_name, call_code));
                                self.var_env.insert(temp_name.clone(), Type::Custom("Tensor32".to_string()));
                                if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                    scope.push(temp_name.clone());
                                }
                                return Ok(temp_name);
                            } else if return_t == Type::Void {
                                block_statements.push(format!("{};", call_code));
                                return Ok("".to_string());
                            } else {
                                return Ok(call_code);
                            }
                        }
                    }
                }

                // map(list, fn) → inline loop building new list
                if name == "map" && args.len() == 2 {
                    let list_code = self.flatten_expression(&args[0], block_statements)?;
                    let fn_code = self.flatten_expression(&args[1], block_statements)?;
                    self.temp_counter += 1;
                    let iter_name = format!("_map_iter_{}", self.temp_counter);
                    self.temp_counter += 1;
                    let item_name = format!("_map_item_{}", self.temp_counter);
                    self.temp_counter += 1;
                    let result_name = format!("_map_result_{}", self.temp_counter);
                    self.temp_counter += 1;
                    let idx_name = format!("_map_i_{}", self.temp_counter);

                    block_statements.push(format!("PyObject* {} = PyList_New(0);", result_name));
                    block_statements.push(format!("PyObject* {} = PyObject_GetIter(to_py({}));", iter_name, list_code));
                    block_statements.push(format!("PyObject* {} = NULL;", item_name));
                    block_statements.push(format!("(void){};  /* suppress unused */", idx_name));
                    block_statements.push(format!("while (({} = PyIter_Next({})) != NULL) {{", item_name, iter_name));
                    block_statements.push(format!("    PyObject* _mapped = to_py({}(mps_to_int({})));", fn_code, item_name));
                    block_statements.push(format!("    PyList_Append({}, _mapped);", result_name));
                    block_statements.push("    Py_DECREF(_mapped);".to_string());
                    block_statements.push(format!("    Py_DECREF({});", item_name));
                    block_statements.push("}".to_string());
                    block_statements.push(format!("Py_DECREF({});", iter_name));

                    if let Some(scope) = self.scope_py_vars.last_mut() {
                        scope.push(result_name.clone());
                    }
                    return Ok(result_name);
                }

                // filter(list, fn) → inline loop keeping matching elements
                if name == "filter" && args.len() == 2 {
                    let list_code = self.flatten_expression(&args[0], block_statements)?;
                    let fn_code = self.flatten_expression(&args[1], block_statements)?;
                    self.temp_counter += 1;
                    let iter_name = format!("_filt_iter_{}", self.temp_counter);
                    self.temp_counter += 1;
                    let item_name = format!("_filt_item_{}", self.temp_counter);
                    self.temp_counter += 1;
                    let result_name = format!("_filt_result_{}", self.temp_counter);

                    block_statements.push(format!("PyObject* {} = PyList_New(0);", result_name));
                    block_statements.push(format!("PyObject* {} = PyObject_GetIter(to_py({}));", iter_name, list_code));
                    block_statements.push(format!("PyObject* {} = NULL;", item_name));
                    block_statements.push(format!("while (({} = PyIter_Next({})) != NULL) {{", item_name, iter_name));
                    block_statements.push(format!("    if ({}(mps_to_int({}))) {{", fn_code, item_name));
                    block_statements.push(format!("        Py_INCREF({});", item_name));
                    block_statements.push(format!("        PyList_Append({}, {});", result_name, item_name));
                    block_statements.push("    }".to_string());
                    block_statements.push(format!("    Py_DECREF({});", item_name));
                    block_statements.push("}".to_string());
                    block_statements.push(format!("Py_DECREF({});", iter_name));

                    if let Some(scope) = self.scope_py_vars.last_mut() {
                        scope.push(result_name.clone());
                    }
                    return Ok(result_name);
                }

                if self.classes.contains_key(name) {
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        arg_codes.push(self.flatten_expression(arg, block_statements)?);
                    }
                    return Ok(format!("{}_new({})", name, arg_codes.join(", ")));
                }

                if self.py_imports.contains_key(name) {
                    return Ok(name.clone());
                }

                let mut arg_codes = Vec::new();
                for arg in args {
                    arg_codes.push(self.flatten_expression(arg, block_statements)?);
                }

                if let Some(t) = self.var_env.get(name) {
                    if let Type::Function { params, return_type } = t {
                        let ret_c = self.c_type(return_type);
                        let param_cs: Vec<String> = params.iter().map(|p| self.c_type(p)).collect();
                        let call_code = format!("(({} (*)({})){})({})", ret_c, param_cs.join(", "), name, arg_codes.join(", "));
                        if **return_type == Type::PyObject {
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("PyObject* {} = {};", temp_name, call_code));
                            if let Some(scope) = self.scope_py_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        } else if **return_type == Type::Custom("Matrix".to_string()) {
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("MPSMatrix* {} = {};", temp_name, call_code));
                            if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        } else if **return_type == Type::Custom("Matrix32".to_string()) {
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("MPSMatrix32* {} = {};", temp_name, call_code));
                            if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        } else {
                            return Ok(call_code);
                        }
                    }
                }
                
                let call_code = format!("{}({})", name, arg_codes.join(", "));
                let return_t = self.infer_type(expr)?;
                if return_t == Type::PyObject {
                    self.temp_counter += 1;
                    let temp_name = format!("_tmp_{}", self.temp_counter);
                    block_statements.push(format!("PyObject* {} = {};", temp_name, call_code));
                    if let Some(scope) = self.scope_py_vars.last_mut() {
                        scope.push(temp_name.clone());
                    }
                    Ok(temp_name)
                } else if return_t == Type::Custom("Matrix".to_string()) {
                    self.temp_counter += 1;
                    let temp_name = format!("_tmp_{}", self.temp_counter);
                    block_statements.push(format!("MPSMatrix* {} = {};", temp_name, call_code));
                    if let Some(scope) = self.scope_matrix_vars.last_mut() {
                        scope.push(temp_name.clone());
                    }
                    Ok(temp_name)
                } else if return_t == Type::Custom("Matrix32".to_string()) {
                    self.temp_counter += 1;
                    let temp_name = format!("_tmp_{}", self.temp_counter);
                    block_statements.push(format!("MPSMatrix32* {} = {};", temp_name, call_code));
                    if let Some(scope) = self.scope_matrix_vars.last_mut() {
                        scope.push(temp_name.clone());
                    }
                    Ok(temp_name)
                } else {
                    Ok(call_code)
                }
            }
            Expr::MemberAccess { object, member } => {
                let obj_t = self.infer_type(object)?;
                let obj_code = self.flatten_expression(object, block_statements)?;
                if let Type::Custom(class_name) = &obj_t {
                    // Special-case Error type (stack-allocated MPS_Error)
                    if class_name == "Error" {
                        return Ok(format!("{}.{}", obj_code, member));
                    }
                    if let Some((_, access_path)) = self.resolve_property(class_name, member) {
                        Ok(format!("{}->{}", obj_code, access_path))
                    } else {
                        Err(format!("Compilation Error: Property '{}' not found in class '{}'", member, class_name))
                    }
                } else if obj_t == Type::PyObject {
                    let call_code = format!("PyObject_GetAttrString({}, \"{}\")", obj_code, member);
                    self.temp_counter += 1;
                    let temp_name = format!("_tmp_{}", self.temp_counter);
                    block_statements.push(format!("PyObject* {} = {};", temp_name, call_code));
                    if let Some(scope) = self.scope_py_vars.last_mut() {
                        scope.push(temp_name.clone());
                    }
                    Ok(temp_name)
                } else {
                    Err(format!("Compilation Error: Member access on non-class instance of type '{:?}'", obj_t))
                }
            }
            Expr::MemberCall { object, method, args } => {
                let obj_t = self.infer_type(object)?;
                let obj_code = self.flatten_expression(object, block_statements)?;
                if obj_t == Type::String {
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        arg_codes.push(self.flatten_expression(arg, block_statements)?);
                    }
                    match method.as_str() {
                        "length" => return Ok(format!("mps_str_len({})", obj_code)),
                        "upper" => return Ok(format!("mps_str_upper({})", obj_code)),
                        "lower" => return Ok(format!("mps_str_lower({})", obj_code)),
                        "trim" => return Ok(format!("mps_str_trim({})", obj_code)),
                        "startswith" => return Ok(format!("mps_str_starts_with({}, {})", obj_code, arg_codes[0])),
                        "endswith" => return Ok(format!("mps_str_ends_with({}, {})", obj_code, arg_codes[0])),
                        "contains" => return Ok(format!("mps_str_contains({}, {})", obj_code, arg_codes[0])),
                        "replace" => return Ok(format!("mps_str_replace({}, {}, {})", obj_code, arg_codes[0], arg_codes[1])),
                        "split" => {
                            let sep = if arg_codes.is_empty() { "\" \"" } else { &arg_codes[0] };
                            let call_code = format!("mps_str_split({}, {})", obj_code, sep);
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("PyObject* {} = {};", temp_name, call_code));
                            if let Some(scope) = self.scope_py_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        }
                        "join" => {
                            return Ok(format!("mps_str_join({}, to_py({}))", obj_code, arg_codes[0]));
                        }
                        _ => return Err(format!("Compilation Error: String has no method '{}'", method)),
                    }
                }
                if let Type::Custom(class_name) = obj_t {
                    if class_name == "Matrix" {
                        let mut arg_codes = Vec::new();
                        for arg in args {
                            arg_codes.push(self.flatten_expression(arg, block_statements)?);
                        }
                        if method == "get" {
                            return Ok(format!("matrix_get({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "set" {
                            return Ok(format!("matrix_set({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "mul" {
                            let call_code = format!("matrix_mul({}, {})", obj_code, arg_codes.join(", "));
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("MPSMatrix* {} = {};", temp_name, call_code));
                            if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        } else {
                            return Err(format!("Compilation Error: Native Matrix does not have method '{}'", method));
                        }
                    } else if class_name == "Matrix32" {
                        let mut arg_codes = Vec::new();
                        for arg in args {
                            arg_codes.push(self.flatten_expression(arg, block_statements)?);
                        }
                        if method == "get" {
                            return Ok(format!("matrix32_get({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "set" {
                            return Ok(format!("matrix32_set({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "mul" {
                            let call_code = format!("matrix32_mul({}, {})", obj_code, arg_codes.join(", "));
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("MPSMatrix32* {} = {};", temp_name, call_code));
                            if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        } else {
                            return Err(format!("Compilation Error: Native Matrix32 does not have method '{}'", method));
                        }
                    } else if class_name == "Tensor" || class_name.starts_with("Tensor<") {
                        let mut arg_codes = Vec::new();
                        for arg in args {
                            arg_codes.push(self.flatten_expression(arg, block_statements)?);
                        }
                        if method == "get" {
                            return Ok(format!("tensor_get({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "set" {
                            block_statements.push(format!("tensor_set({}, {});", obj_code, arg_codes.join(", ")));
                            return Ok("".to_string());
                        } else if method == "shape" || method == "strides" {
                            return Ok(format!("tensor_{}({})", method, obj_code));
                        } else if method == "sigmoid" || method == "relu" || method == "softmax" || method == "exp" || method == "log" {
                            let call_code = format!("tensor_{}({})", method, obj_code);
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("MPSTensor* {} = {};", temp_name, call_code));
                            self.var_env.insert(temp_name.clone(), Type::Custom("Tensor".to_string()));
                            if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        } else if method == "reshape" || method == "transpose" || method == "squeeze" || method == "matmul" {
                            let call_code = match method.as_str() {
                                "reshape" => format!("tensor_reshape({}, to_py({}))", obj_code, arg_codes[0]),
                                "transpose" => format!("tensor_transpose({}, {}, {})", obj_code, arg_codes[0], arg_codes[1]),
                                "squeeze" => format!("tensor_squeeze({}, {})", obj_code, arg_codes[0]),
                                "matmul" => format!("tensor_matmul({}, {})", obj_code, arg_codes[0]),
                                _ => unreachable!(),
                            };
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("MPSTensor* {} = {};", temp_name, call_code));
                            self.var_env.insert(temp_name.clone(), Type::Custom("Tensor".to_string()));
                            if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        } else {
                            return Err(format!("Compilation Error: Native Tensor does not have method '{}'", method));
                        }
                    } else if class_name == "Tensor32" {
                        let mut arg_codes = Vec::new();
                        for arg in args {
                            arg_codes.push(self.flatten_expression(arg, block_statements)?);
                        }
                        if method == "get" {
                            return Ok(format!("tensor32_get({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "set" {
                            block_statements.push(format!("tensor32_set({}, {});", obj_code, arg_codes.join(", ")));
                            return Ok("".to_string());
                        } else if method == "shape" || method == "strides" {
                            return Ok(format!("tensor32_{}({})", method, obj_code));
                        } else if method == "sigmoid" || method == "relu" || method == "softmax" || method == "exp" || method == "log" {
                            let call_code = format!("tensor32_{}({})", method, obj_code);
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("MPSTensor32* {} = {};", temp_name, call_code));
                            self.var_env.insert(temp_name.clone(), Type::Custom("Tensor32".to_string()));
                            if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        } else if method == "reshape" || method == "transpose" || method == "squeeze" || method == "matmul" {
                            let call_code = match method.as_str() {
                                "reshape" => format!("tensor32_reshape({}, to_py({}))", obj_code, arg_codes[0]),
                                "transpose" => format!("tensor32_transpose({}, {}, {})", obj_code, arg_codes[0], arg_codes[1]),
                                "squeeze" => format!("tensor32_squeeze({}, {})", obj_code, arg_codes[0]),
                                "matmul" => format!("tensor32_matmul({}, {})", obj_code, arg_codes[0]),
                                _ => unreachable!(),
                            };
                            self.temp_counter += 1;
                            let temp_name = format!("_tmp_{}", self.temp_counter);
                            block_statements.push(format!("MPSTensor32* {} = {};", temp_name, call_code));
                            self.var_env.insert(temp_name.clone(), Type::Custom("Tensor32".to_string()));
                            if let Some(scope) = self.scope_matrix_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                            return Ok(temp_name);
                        } else {
                            return Err(format!("Compilation Error: Native Tensor32 does not have method '{}'", method));
                        }
                    }
                    if let Some((def_class, cast)) = self.resolve_method(&class_name, method) {
                        let mut arg_codes = Vec::new();
                        if cast.is_empty() {
                            arg_codes.push(obj_code);
                        } else {
                            arg_codes.push(format!("({}*){}", def_class, obj_code));
                        }
                        for arg in args {
                            arg_codes.push(self.flatten_expression(arg, block_statements)?);
                        }
                        Ok(format!("{}_{}({})", def_class, method, arg_codes.join(", ")))
                    } else {
                        Err(format!("Compilation Error: Method '{}' not found in class '{}'", method, class_name))
                    }
                } else if obj_t == Type::PyObject {
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        let arg_c = self.flatten_expression(arg, block_statements)?;
                        arg_codes.push(format!("to_py({})", arg_c));
                    }
                    let args_str = if arg_codes.is_empty() { "".to_string() } else { format!(", {}", arg_codes.join(", ")) };
                    let call_code = format!("py_call({}, \"{}\", {}{})", obj_code, method, args.len(), args_str);
                    
                    let method_name = method.as_str();
                    let (c_type_str, wrapped_call_code) = match method_name {
                        "length" => ("int".to_string(), format!("mps_to_int({})", call_code)),
                        "contains" => ("bool".to_string(), format!("mps_to_bool({})", call_code)),
                        "append" | "remove" | "clear" => ("void".to_string(), call_code),
                        _ => ("PyObject*".to_string(), call_code),
                    };

                    if c_type_str == "void" {
                        block_statements.push(format!("{};", wrapped_call_code));
                        Ok("".to_string())
                    } else {
                        self.temp_counter += 1;
                        let temp_name = format!("_tmp_{}", self.temp_counter);
                        block_statements.push(format!("{} {} = {};", c_type_str, temp_name, wrapped_call_code));
                        if c_type_str == "PyObject*" {
                            if let Some(scope) = self.scope_py_vars.last_mut() {
                                scope.push(temp_name.clone());
                            }
                        }
                        Ok(temp_name)
                    }
                } else {
                    Err(format!("Compilation Error: Method call on non-class instance of type '{:?}'", obj_t))
                }
            }
            Expr::Subscript { object, index } => {
                let obj_t = self.infer_type(object)?;
                let obj_code = self.flatten_expression(object, block_statements)?;
                if obj_t == Type::PyObject {
                    let index_code = self.flatten_expression(index, block_statements)?;
                    let call_code = format!("PyObject_GetItem({}, to_py({}))", obj_code, index_code);
                    self.temp_counter += 1;
                    let temp_name = format!("_tmp_{}", self.temp_counter);
                    block_statements.push(format!("PyObject* {} = {};", temp_name, call_code));
                    if let Some(scope) = self.scope_py_vars.last_mut() {
                        scope.push(temp_name.clone());
                    }
                    Ok(temp_name)
                } else if let Type::Custom(ref class_name) = obj_t {
                    if class_name == "Tensor" || class_name.starts_with("Tensor<") || class_name == "Tensor32" {
                        let is_f32 = class_name == "Tensor32" || class_name.contains("float32");
                        if let Some(elts) = self.get_direct_indices(index) {
                            let mut idx_codes = Vec::new();
                            for elt in &elts {
                                idx_codes.push(self.flatten_expression(elt, block_statements)?);
                            }
                            let idx_count = idx_codes.len();
                            let array_literal = format!("(const int[]){{{}}}", idx_codes.join(", "));
                            let getter_fn = if is_f32 { "tensor32_get_direct" } else { "tensor_get_direct" };
                            Ok(format!("{}({}, {}, {})", getter_fn, obj_code, idx_count, array_literal))
                        } else {
                            let index_code = self.flatten_expression(index, block_statements)?;
                            if is_f32 {
                                Ok(format!("tensor32_get({}, to_py({}))", obj_code, index_code))
                            } else {
                                Ok(format!("tensor_get({}, to_py({}))", obj_code, index_code))
                            }
                        }
                    } else if class_name == "Matrix" || class_name.starts_with("Matrix<") {
                        let index_code = self.flatten_expression(index, block_statements)?;
                        Ok(format!("{}->data[{}]", obj_code, index_code))
                    } else if class_name == "Matrix32" {
                        let index_code = self.flatten_expression(index, block_statements)?;
                        Ok(format!("{}->data[{}]", obj_code, index_code))
                    } else {
                        Err(format!("Compilation Error: Subscript indexing only supported on Python objects, found '{:?}'", obj_t))
                    }
                } else if obj_t == Type::String {
                    let index_code = self.flatten_expression(index, block_statements)?;
                    Ok(format!("mps_str_get_char({}, {})", obj_code, index_code))
                } else {
                    Err(format!("Compilation Error: Subscript indexing only supported on Python objects, found '{:?}'", obj_t))
                }
            }
            Expr::ListLiteral(exprs) => {
                let mut args = Vec::new();
                for e in exprs {
                    let e_code = self.flatten_expression(e, block_statements)?;
                    args.push(format!("to_py({})", e_code));
                }
                let call_code = format!("mps_list_new({}, {})", exprs.len(), args.join(", "));
                self.temp_counter += 1;
                let temp_name = format!("_tmp_{}", self.temp_counter);
                block_statements.push(format!("PyObject* {} = {};", temp_name, call_code));
                if let Some(scope) = self.scope_py_vars.last_mut() {
                    scope.push(temp_name.clone());
                }
                Ok(temp_name)
            }
            Expr::DictLiteral(pairs) => {
                let mut args = Vec::new();
                for (k, v) in pairs {
                    let k_code = self.flatten_expression(k, block_statements)?;
                    let v_code = self.flatten_expression(v, block_statements)?;
                    args.push(format!("to_py({}), to_py({})", k_code, v_code));
                }
                let call_code = format!("mps_dict_new({}, {})", pairs.len(), args.join(", "));
                self.temp_counter += 1;
                let temp_name = format!("_tmp_{}", self.temp_counter);
                block_statements.push(format!("PyObject* {} = {};", temp_name, call_code));
                if let Some(scope) = self.scope_py_vars.last_mut() {
                    scope.push(temp_name.clone());
                }
                Ok(temp_name)
            }
            Expr::TupleLiteral(exprs) => {
                let mut args = Vec::new();
                for e in exprs {
                    let e_code = self.flatten_expression(e, block_statements)?;
                    args.push(format!("to_py({})", e_code));
                }
                let call_code = format!("mps_tuple_new({}, {})", exprs.len(), args.join(", "));
                self.temp_counter += 1;
                let temp_name = format!("_tmp_{}", self.temp_counter);
                block_statements.push(format!("PyObject* {} = {};", temp_name, call_code));
                if let Some(scope) = self.scope_py_vars.last_mut() {
                    scope.push(temp_name.clone());
                }
                Ok(temp_name)
            }
            Expr::SuperCall { method, args } => {
                let parent = self.current_class.as_ref().and_then(|class_name| {
                    self.classes.get(class_name).and_then(|info| info.parent.clone())
                });
                if let Some(parent_name) = parent {
                    let mut arg_codes = Vec::new();
                    arg_codes.push(format!("({}*)self", parent_name));
                    for arg in args {
                        arg_codes.push(self.flatten_expression(arg, block_statements)?);
                    }
                    return Ok(format!("{}_{}({})", parent_name, method, arg_codes.join(", ")));
                }
                Err("Compilation Error: 'super' call only allowed inside class methods.".into())
            }
            Expr::Lambda { params, return_type, body } => {
                self.temp_counter += 1;
                let lambda_name = format!("_lambda_{}", self.temp_counter);
                
                let mut block_statements_inner = Vec::new();
                self.enter_block();
                let outer_var_env = self.var_env.clone();
                for p in params {
                    self.var_env.insert(p.name.clone(), p.param_type.clone());
                }
                let body_code = self.flatten_expression(body, &mut block_statements_inner)?;
                let cleanups = self.exit_block();
                self.var_env = outer_var_env;
                
                let param_strs: Vec<String> = params
                    .iter()
                    .map(|p| format!("{} {}", self.c_type(&p.param_type), p.name))
                    .collect();
                
                let mut l_func = format!(
                    "static inline {} {}({}) {{\n",
                    self.c_type(return_type),
                    lambda_name,
                    param_strs.join(", ")
                );
                for stmt in block_statements_inner {
                    l_func.push_str(&format!("    {}\n", stmt));
                }
                if !cleanups.is_empty() {
                    l_func.push_str(&cleanups);
                }
                l_func.push_str(&format!("    return {};\n", body_code));
                l_func.push_str("}\n\n");
                
                self.lambdas_code.push_str(&l_func);
                Ok(lambda_name)
            }
            Expr::AwaitExpr(inner) => {
                let task_handle = self.flatten_expression(inner, block_statements)?;
                block_statements.push(format!("mps_task_await(&{}->_task);", task_handle));
                let return_t = self.infer_type(expr)?;
                if return_t != Type::Void {
                    self.temp_counter += 1;
                    let temp_name = format!("_tmp_{}", self.temp_counter);
                    block_statements.push(format!("{} {} = {}->_ret;", self.c_type(&return_t), temp_name, task_handle));
                    block_statements.push(format!("free({});", task_handle));
                    Ok(temp_name)
                } else {
                    block_statements.push(format!("free({});", task_handle));
                    Ok("".to_string())
                }
            }
            Expr::Super => Ok("((PyObject*)self)".to_string()),
            Expr::Slice { object, start, end } => {
                let obj_code = self.flatten_expression(object, block_statements)?;
                let obj_t = self.infer_type(object)?;
                
                self.temp_counter += 1;
                let temp_name = format!("_tmp_slice_{}", self.temp_counter);
                
                if obj_t == Type::String {
                    let s_val = if let Some(s) = start {
                        self.flatten_expression(s, block_statements)?
                    } else {
                        "-1".to_string()
                    };
                    let e_val = if let Some(e) = end {
                        self.flatten_expression(e, block_statements)?
                    } else {
                        "-1".to_string()
                    };
                    block_statements.push(format!("const char* {} = mps_str_slice({}, {}, {});", temp_name, obj_code, s_val, e_val));
                    return Ok(temp_name);
                } else {
                    let s_obj = if let Some(s) = start {
                        let sc = self.flatten_expression(s, block_statements)?;
                        format!("to_py({})", sc)
                    } else {
                        "Py_None".to_string()
                    };
                    let e_obj = if let Some(e) = end {
                        let ec = self.flatten_expression(e, block_statements)?;
                        format!("to_py({})", ec)
                    } else {
                        "Py_None".to_string()
                    };
                    block_statements.push(format!("PyObject* _slice_spec_{} = PySlice_New({}, {}, NULL);", self.temp_counter, s_obj, e_obj));
                    block_statements.push(format!("PyObject* {} = PyObject_GetItem(to_py({}), _slice_spec_{});", temp_name, obj_code, self.temp_counter));
                    block_statements.push(format!("Py_XDECREF(_slice_spec_{});", self.temp_counter));
                    
                    if let Some(scope) = self.scope_py_vars.last_mut() {
                        scope.push(temp_name.clone());
                    }
                    return Ok(temp_name);
                }
            }
            Expr::ListComprehension { element, var_name, iterable } => {
                let iter_code = self.flatten_expression(iterable, block_statements)?;
                
                self.temp_counter += 1;
                let list_name = format!("_list_comp_{}", self.temp_counter);
                
                block_statements.push(format!("PyObject* {} = PyList_New(0);", list_name));
                
                if let Some(scope) = self.scope_py_vars.last_mut() {
                    scope.push(list_name.clone());
                }
                
                self.temp_counter += 1;
                let iter_name = format!("_iter_{}", self.temp_counter);
                self.temp_counter += 1;
                let item_name = format!("_item_{}", self.temp_counter);
                
                block_statements.push(format!("PyObject* {} = PyObject_GetIter(to_py({}));", iter_name, iter_code));
                block_statements.push(format!("if ({} != NULL) {{", iter_name));
                block_statements.push(format!("    PyObject* {} = NULL;", item_name));
                block_statements.push(format!("    while (({} = PyIter_Next({})) != NULL) {{", item_name, iter_name));
                
                let outer_var_env = self.var_env.clone();
                self.var_env.insert(var_name.clone(), Type::PyObject);
                self.enter_block();
                if let Some(scope) = self.scope_py_vars.last_mut() {
                    scope.push(var_name.clone());
                }
                
                let mut loop_body = Vec::new();
                loop_body.push(format!("PyObject* {} = {};", var_name, item_name));
                loop_body.push(format!("Py_XINCREF({});", var_name));
                
                let elem_code = self.flatten_expression(element, &mut loop_body)?;
                loop_body.push(format!("PyObject* _mapped_val = to_py({});", elem_code));
                loop_body.push(format!("PyList_Append({}, _mapped_val);", list_name));
                loop_body.push("Py_XDECREF(_mapped_val);".to_string());
                
                let cleanups_loop = self.exit_block();
                self.var_env = outer_var_env;
                
                for line in loop_body {
                    block_statements.push(format!("        {}", line));
                }
                for line in cleanups_loop.lines() {
                    block_statements.push(format!("        {}", line));
                }
                block_statements.push(format!("        Py_DECREF({});", item_name));
                block_statements.push("    }".to_string());
                block_statements.push(format!("    Py_DECREF({});", iter_name));
                block_statements.push("}".to_string());
                
                Ok(list_name)
            }
        }
    }

    #[allow(dead_code)]
    fn transpile_expression(&self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(n) => Ok(n.to_string()),
                Literal::Float(f) => Ok(f.to_string()),
                Literal::String(s) => Ok(format!("\"{}\"", Self::escape_string(s))),
                Literal::Bool(b) => Ok(b.to_string()),
                Literal::Null => Ok("NULL".to_string()),
            },
            Expr::Identifier(name) => Ok(name.clone()),
            Expr::Unary { op, operand } => {
                let operand_code = self.transpile_expression(operand)?;
                match op {
                    UnaryOp::Neg => Ok(format!("(-{})", operand_code)),
                    UnaryOp::Not => Ok(format!("(!{})", operand_code)),
                }
            }
            Expr::Binary { op, left, right } => {
                let left_t = self.infer_type(left)?;
                if let Type::Custom(class_name) = &left_t {
                    let op_method = match op {
                        BinOp::Add => Some("add"),
                        BinOp::Sub => Some("sub"),
                        BinOp::Mul => Some("mul"),
                        BinOp::Div => Some("div"),
                        BinOp::Percent => Some("mod"),
                        BinOp::Eq => Some("eq"),
                        BinOp::Ne => Some("ne"),
                        BinOp::Lt => Some("lt"),
                        BinOp::Le => Some("le"),
                        BinOp::Gt => Some("gt"),
                        BinOp::Ge => Some("ge"),
                        BinOp::MatMul => Some("matmul"),
                        BinOp::Pow | BinOp::And | BinOp::Or => None,
                    };
                    if let Some(method_name) = op_method {
                        if self.resolve_method(class_name, method_name).is_some() {
                            let rewrite = Expr::MemberCall {
                                object: Box::new(left.as_ref().clone()),
                                method: method_name.to_string(),
                                args: vec![right.as_ref().clone()],
                            };
                            return self.transpile_expression(&rewrite);
                        }
                    }
                }
                let left_code = self.transpile_expression(left)?;
                let right_code = self.transpile_expression(right)?;
                let right_t = self.infer_type(right)?;

                match op {
                    BinOp::And => Ok(format!("({} && {})", left_code, right_code)),
                    BinOp::Or => Ok(format!("({} || {})", left_code, right_code)),
                    BinOp::Pow => Ok(format!("mps_pow({}, {})", left_code, right_code)),
                    BinOp::Add => {
                        if left_t == Type::String || right_t == Type::String {
                            let left_coerced = if left_t == Type::String { left_code } else { format!("mps_to_string({})", left_code) };
                            let right_coerced = if right_t == Type::String { right_code } else { format!("mps_to_string({})", right_code) };
                            Ok(format!("mps_str_concat({}, {})", left_coerced, right_coerced))
                        } else {
                            Ok(format!("({} + {})", left_code, right_code))
                        }
                    }
                    BinOp::Sub => Ok(format!("({} - {})", left_code, right_code)),
                    BinOp::Mul => {
                        if left_t == Type::Custom("Matrix".to_string()) && right_t == Type::Custom("Matrix".to_string()) {
                            Ok(format!("matrix_mul({}, {})", left_code, right_code))
                        } else if left_t == Type::Custom("Matrix32".to_string()) && right_t == Type::Custom("Matrix32".to_string()) {
                            Ok(format!("matrix32_mul({}, {})", left_code, right_code))
                        } else {
                            Ok(format!("({} * {})", left_code, right_code))
                        }
                    }
                    BinOp::MatMul => {
                        if left_t == Type::Custom("Matrix".to_string()) && right_t == Type::Custom("Matrix".to_string()) {
                            Ok(format!("matrix_mul({}, {})", left_code, right_code))
                        } else if left_t == Type::Custom("Matrix32".to_string()) && right_t == Type::Custom("Matrix32".to_string()) {
                            Ok(format!("matrix32_mul({}, {})", left_code, right_code))
                        } else if left_t == Type::Custom("Tensor".to_string()) && right_t == Type::Custom("Tensor".to_string()) {
                            Ok(format!("tensor_matmul({}, {})", left_code, right_code))
                        } else if left_t == Type::Custom("Tensor32".to_string()) && right_t == Type::Custom("Tensor32".to_string()) {
                            Ok(format!("tensor32_matmul({}, {})", left_code, right_code))
                        } else {
                            Err(format!("Compilation Error: Operator '@' only supported on Matrix or Tensor types"))
                        }
                    }
                    BinOp::Div => Ok(format!("({} / {})", left_code, right_code)),
                    BinOp::Percent => Ok(format!("({} % {})", left_code, right_code)),
                    BinOp::Eq => {
                        if left_t == Type::String {
                            Ok(format!("(strcmp({}, {}) == 0)", left_code, right_code))
                        } else {
                            Ok(format!("({} == {})", left_code, right_code))
                        }
                    }
                    BinOp::Ne => {
                        if left_t == Type::String {
                            Ok(format!("(strcmp({}, {}) != 0)", left_code, right_code))
                        } else {
                            Ok(format!("({} != {})", left_code, right_code))
                        }
                    }
                    BinOp::Lt => Ok(format!("({} < {})", left_code, right_code)),
                    BinOp::Le => Ok(format!("({} <= {})", left_code, right_code)),
                    BinOp::Gt => Ok(format!("({} > {})", left_code, right_code)),
                    BinOp::Ge => Ok(format!("({} >= {})", left_code, right_code)),
                }
            }
            Expr::Call { name, type_args: _, args } => {
                if name == "len" && args.len() == 1 {
                    let arg_code = self.transpile_expression(&args[0])?;
                    let arg_t = self.infer_type(&args[0])?;
                    if arg_t == Type::String {
                        return Ok(format!("((int)strlen({}))", arg_code));
                    } else {
                        return Ok(format!("((int)PyObject_Size({}))", arg_code));
                    }
                }

                if name == "print" || name == "mps_print" {
                    let arg_c = self.transpile_expression(&args[0])?;
                    let arg_t = self.infer_type(&args[0])?;
                    let print_macro = match arg_t {
                        Type::Int => "print_int",
                        Type::Float => "print_float",
                        Type::String => "print_string",
                        Type::Bool => "print_bool",
                        Type::PyObject => "print_py",
                        _ => "print_string",
                    };
                    Ok(format!("{}({})", print_macro, arg_c))
                } else if name == "mps_println" {
                    let arg_c = self.transpile_expression(&args[0])?;
                    let arg_t = self.infer_type(&args[0])?;
                    let print_macro = match arg_t {
                        Type::Int => "println_int",
                        Type::Float => "println_float",
                        Type::String => "println_string",
                        Type::Bool => "println_bool",
                        Type::PyObject => "println_py",
                        _ => "println_string",
                    };
                    Ok(format!("{}({})", print_macro, arg_c))
                } else {
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        arg_codes.push(self.transpile_expression(arg)?);
                    }
                    if self.classes.contains_key(name) {
                        Ok(format!("{}_new({})", name, arg_codes.join(", ")))
                    } else {
                        Ok(format!("{}({})", name, arg_codes.join(", ")))
                    }
                }
            }
            Expr::MemberAccess { object, member } => {
                let obj_t = self.infer_type(object)?;
                if let Type::Custom(class_name) = &obj_t {
                    if class_name == "Error" {
                        let obj_code = self.transpile_expression(object)?;
                        return Ok(format!("{}.{}", obj_code, member));
                    }
                    if let Some((_, access_path)) = self.resolve_property(class_name, member) {
                        let obj_code = self.transpile_expression(object)?;
                        Ok(format!("{}->{}", obj_code, access_path))
                    } else {
                        Err(format!("Compilation Error: Property '{}' not found in class '{}'", member, class_name))
                    }
                } else if obj_t == Type::PyObject {
                    let obj_code = self.transpile_expression(object)?;
                    Ok(format!("PyObject_GetAttrString({}, \"{}\")", obj_code, member))
                } else {
                    Err(format!("Compilation Error: Member access on non-class instance of type '{:?}'", obj_t))
                }
            }
            Expr::MemberCall { object, method, args } => {
                let obj_t = self.infer_type(object)?;
                if obj_t == Type::String {
                    let obj_code = self.transpile_expression(object)?;
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        arg_codes.push(self.transpile_expression(arg)?);
                    }
                    match method.as_str() {
                        "length" => return Ok(format!("mps_str_len({})", obj_code)),
                        "upper" => return Ok(format!("mps_str_upper({})", obj_code)),
                        "lower" => return Ok(format!("mps_str_lower({})", obj_code)),
                        "trim" => return Ok(format!("mps_str_trim({})", obj_code)),
                        "startswith" => return Ok(format!("mps_str_starts_with({}, {})", obj_code, arg_codes[0])),
                        "endswith" => return Ok(format!("mps_str_ends_with({}, {})", obj_code, arg_codes[0])),
                        "contains" => return Ok(format!("mps_str_contains({}, {})", obj_code, arg_codes[0])),
                        "replace" => return Ok(format!("mps_str_replace({}, {}, {})", obj_code, arg_codes[0], arg_codes[1])),
                        "split" => {
                            let sep = if arg_codes.is_empty() { "\" \"" } else { &arg_codes[0] };
                            return Ok(format!("mps_str_split({}, {})", obj_code, sep));
                        }
                        "join" => {
                            return Ok(format!("mps_str_join({}, to_py({}))", obj_code, arg_codes[0]));
                        }
                        _ => return Err(format!("Compilation Error: String has no method '{}'", method)),
                    }
                }
                if let Type::Custom(class_name) = obj_t {
                    if class_name == "Matrix" {
                        let obj_code = self.transpile_expression(object)?;
                        let mut arg_codes = Vec::new();
                        for arg in args {
                            arg_codes.push(self.transpile_expression(arg)?);
                        }
                        if method == "get" {
                            return Ok(format!("matrix_get({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "set" {
                            return Ok(format!("matrix_set({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "mul" {
                            return Ok(format!("matrix_mul({}, {})", obj_code, arg_codes.join(", ")));
                        } else {
                            return Err(format!("Compilation Error: Native Matrix does not have method '{}'", method));
                        }
                    } else if class_name == "Matrix32" {
                        let obj_code = self.transpile_expression(object)?;
                        let mut arg_codes = Vec::new();
                        for arg in args {
                            arg_codes.push(self.transpile_expression(arg)?);
                        }
                        if method == "get" {
                            return Ok(format!("matrix32_get({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "set" {
                            return Ok(format!("matrix32_set({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "mul" {
                            return Ok(format!("matrix32_mul({}, {})", obj_code, arg_codes.join(", ")));
                        } else {
                            return Err(format!("Compilation Error: Native Matrix32 does not have method '{}'", method));
                        }
                    } else if class_name == "Tensor" || class_name.starts_with("Tensor<") {
                        let obj_code = self.transpile_expression(object)?;
                        let mut arg_codes = Vec::new();
                        for arg in args {
                            arg_codes.push(self.transpile_expression(arg)?);
                        }
                        if method == "get" {
                            return Ok(format!("tensor_get({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "set" {
                            return Ok(format!("tensor_set({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "shape" || method == "strides" {
                            return Ok(format!("tensor_{}({})", method, obj_code));
                        } else if method == "sigmoid" || method == "relu" || method == "softmax" || method == "exp" || method == "log" {
                            return Ok(format!("tensor_{}({})", method, obj_code));
                        } else if method == "reshape" {
                            return Ok(format!("tensor_reshape({}, to_py({}))", obj_code, arg_codes[0]));
                        } else if method == "transpose" {
                            return Ok(format!("tensor_transpose({}, {}, {})", obj_code, arg_codes[0], arg_codes[1]));
                        } else if method == "squeeze" {
                            return Ok(format!("tensor_squeeze({}, {})", obj_code, arg_codes[0]));
                        } else if method == "matmul" {
                            return Ok(format!("tensor_matmul({}, {})", obj_code, arg_codes[0]));
                        } else {
                            return Err(format!("Compilation Error: Native Tensor does not have method '{}'", method));
                        }
                    } else if class_name == "Tensor32" {
                        let obj_code = self.transpile_expression(object)?;
                        let mut arg_codes = Vec::new();
                        for arg in args {
                            arg_codes.push(self.transpile_expression(arg)?);
                        }
                        if method == "get" {
                            return Ok(format!("tensor32_get({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "set" {
                            return Ok(format!("tensor32_set({}, {})", obj_code, arg_codes.join(", ")));
                        } else if method == "shape" || method == "strides" {
                            return Ok(format!("tensor32_{}({})", method, obj_code));
                        } else if method == "sigmoid" || method == "relu" || method == "softmax" || method == "exp" || method == "log" {
                            return Ok(format!("tensor32_{}({})", method, obj_code));
                        } else if method == "reshape" {
                            return Ok(format!("tensor32_reshape({}, to_py({}))", obj_code, arg_codes[0]));
                        } else if method == "transpose" {
                            return Ok(format!("tensor32_transpose({}, {}, {})", obj_code, arg_codes[0], arg_codes[1]));
                        } else if method == "squeeze" {
                            return Ok(format!("tensor32_squeeze({}, {})", obj_code, arg_codes[0]));
                        } else if method == "matmul" {
                            return Ok(format!("tensor32_matmul({}, {})", obj_code, arg_codes[0]));
                        } else {
                            return Err(format!("Compilation Error: Native Tensor32 does not have method '{}'", method));
                        }
                    }
                    if let Some((def_class, cast)) = self.resolve_method(&class_name, method) {
                        let obj_code = self.transpile_expression(object)?;
                        let mut arg_codes = Vec::new();
                        
                        if cast.is_empty() {
                            arg_codes.push(obj_code);
                        } else {
                            arg_codes.push(format!("({}*){}", def_class, obj_code));
                        }

                        for arg in args {
                            arg_codes.push(self.transpile_expression(arg)?);
                        }

                        Ok(format!("{}_{}({})", def_class, method, arg_codes.join(", ")))
                    } else {
                        Err(format!("Compilation Error: Method '{}' not found in class '{}'", method, class_name))
                    }
                } else if obj_t == Type::PyObject {
                    let obj_code = self.transpile_expression(object)?;
                    let mut arg_codes = Vec::new();
                    for arg in args {
                        let arg_c = self.transpile_expression(arg)?;
                        arg_codes.push(format!("to_py({})", arg_c));
                    }
                    let args_str = if arg_codes.is_empty() { "".to_string() } else { format!(", {}", arg_codes.join(", ")) };
                    let call_code = format!("py_call({}, \"{}\", {}{})", obj_code, method, args.len(), args_str);
                    
                    let method_name = method.as_str();
                    match method_name {
                        "length" => Ok(format!("mps_to_int({})", call_code)),
                        "contains" => Ok(format!("mps_to_bool({})", call_code)),
                        _ => Ok(call_code),
                    }
                } else {
                    Err(format!("Compilation Error: Method call on non-class instance of type '{:?}'", obj_t))
                }
            }
            Expr::Subscript { object, index } => {
                let obj_t = self.infer_type(object)?;
                let obj_code = self.transpile_expression(object)?;
                if obj_t == Type::PyObject {
                    let index_code = self.transpile_expression(index)?;
                    Ok(format!("PyObject_GetItem({}, to_py({}))", obj_code, index_code))
                } else if let Type::Custom(ref class_name) = obj_t {
                    if class_name == "Tensor" || class_name.starts_with("Tensor<") || class_name == "Tensor32" {
                        let is_f32 = class_name == "Tensor32" || class_name.contains("float32");
                        if let Some(elts) = self.get_direct_indices(index) {
                            let mut idx_codes = Vec::new();
                            for elt in &elts {
                                idx_codes.push(self.transpile_expression(elt)?);
                            }
                            let idx_count = idx_codes.len();
                            let array_literal = format!("(const int[]){{{}}}", idx_codes.join(", "));
                            let getter_fn = if is_f32 { "tensor32_get_direct" } else { "tensor_get_direct" };
                            Ok(format!("{}({}, {}, {})", getter_fn, obj_code, idx_count, array_literal))
                        } else {
                            let index_code = self.transpile_expression(index)?;
                            if is_f32 {
                                Ok(format!("tensor32_get({}, to_py({}))", obj_code, index_code))
                            } else {
                                Ok(format!("tensor_get({}, to_py({}))", obj_code, index_code))
                            }
                        }
                    } else if class_name == "Matrix" || class_name.starts_with("Matrix<") || class_name == "Matrix32" {
                        let index_code = self.transpile_expression(index)?;
                        Ok(format!("{}->data[{}]", obj_code, index_code))
                    } else {
                        let index_code = self.transpile_expression(index)?;
                        Ok(format!("{}[{}]", obj_code, index_code))
                    }
                } else if obj_t == Type::String {
                    let index_code = self.transpile_expression(index)?;
                    Ok(format!("mps_str_get_char({}, {})", obj_code, index_code))
                } else {
                    let index_code = self.transpile_expression(index)?;
                    Ok(format!("{}[{}]", obj_code, index_code))
                }
            }
            Expr::ListLiteral(_) | Expr::DictLiteral(_) | Expr::TupleLiteral(_) | Expr::SuperCall { .. } | Expr::Lambda { .. } | Expr::AwaitExpr(_) | Expr::OptionalMemberAccess { .. } | Expr::OptionalMemberCall { .. } | Expr::FString { .. } | Expr::Slice { .. } | Expr::ListComprehension { .. } => {
                Err("Internal Error: Flattened expression variants cannot be transpiled directly.".into())
            }
            Expr::Super => Ok("((PyObject*)self)".to_string()),
        }
    }
}
