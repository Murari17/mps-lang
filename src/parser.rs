use crate::ast::{BinOp, UnaryOp, Expr, Literal, Param, Program, Stmt, Type, TraitMethodSignature, MatchCase, MatchPattern};
use crate::lexer::{Token, SpannedToken};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        if self.pos < self.tokens.len() {
            Some(&self.tokens[self.pos].token)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    fn peek_ahead(&self, n: usize) -> Option<&Token> {
        if self.pos + n < self.tokens.len() {
            Some(&self.tokens[self.pos + n].token)
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].token.clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn current_span(&self) -> (usize, usize) {
        if self.pos < self.tokens.len() {
            (self.tokens[self.pos].line, self.tokens[self.pos].col)
        } else if !self.tokens.is_empty() {
            let last = &self.tokens[self.tokens.len() - 1];
            (last.line, last.col)
        } else {
            (1, 1)
        }
    }

    fn error<T>(&self, msg: &str) -> Result<T, String> {
        let (line, col) = self.current_span();
        Err(format!("Syntax Error (line {}, col {}): {}", line, col, msg))
    }

    fn consume(&mut self, expected: Token, err_msg: &str) -> Result<Token, String> {
        match self.peek() {
            Some(tok) if *tok == expected => Ok(self.advance().unwrap()),
            Some(tok) => {
                let (line, col) = self.current_span();
                Err(format!(
                    "Syntax Error (line {}, col {}): {} (found token {:?}, expected {:?})",
                    line, col, err_msg, tok, expected
                ))
            }
            None => {
                let (line, col) = self.current_span();
                Err(format!(
                    "Syntax Error (line {}, col {}): {} (found EOF, expected {:?})",
                    line, col, err_msg, expected
                ))
            }
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        while self.peek() != Some(&Token::EOF) && self.peek().is_some() {
            if self.peek() == Some(&Token::Newline) {
                self.advance();
                continue;
            }
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        while self.peek() == Some(&Token::Newline) {
            self.advance();
        }

        let mut decorators = Vec::new();
        while self.peek() == Some(&Token::At) {
            self.advance(); // consume '@'
            let dec_name = match self.advance() {
                Some(Token::Identifier(s)) => s,
                Some(tok) => return self.error(&format!("Expected decorator name, found {:?}", tok)),
                None => return self.error("Expected decorator name, found EOF"),
            };
            decorators.push(dec_name);
            self.consume_statement_end()?;
        }

        if !decorators.is_empty() {
            match self.peek() {
                Some(Token::Fn) => {
                    self.advance(); // consume 'fn'
                    return self.parse_function_decl_body(false, decorators);
                }
                Some(Token::Async) => {
                    self.advance(); // consume 'async'
                    self.consume(Token::Fn, "Expected 'fn' keyword after 'async'")?;
                    return self.parse_function_decl_body(true, decorators);
                }
                _ => return self.error("Expected function declaration after decorator"),
            }
        }

        match self.peek() {
            Some(Token::Fn) => self.parse_function_decl(),
            Some(Token::Async) => {
                self.advance();
                self.parse_async_function_decl()
            }
            Some(Token::Class) => self.parse_class_decl(),
            Some(Token::Trait) => self.parse_trait_decl(),
            Some(Token::PyImport) => self.parse_pyimport(),
            Some(Token::Import) => self.parse_import(),
            Some(Token::From) => self.parse_from_import(),
            Some(Token::Let) => self.parse_variable_decl(false),
            Some(Token::Const) => self.parse_variable_decl(true),
            Some(Token::If) => self.parse_if_stmt(),
            Some(Token::While) => self.parse_while_stmt(),
            Some(Token::For) => self.parse_for_stmt(),
            Some(Token::Try) => self.parse_try_catch_stmt(),
            Some(Token::Raise) => self.parse_raise_stmt(),
            Some(Token::Match) => self.parse_match_stmt(),
            Some(Token::Return) => self.parse_return_stmt(),
            Some(Token::Break) => {
                self.advance();
                self.consume_statement_end()?;
                Ok(Stmt::BreakStmt)
            }
            Some(Token::Continue) => {
                self.advance();
                self.consume_statement_end()?;
                Ok(Stmt::ContinueStmt)
            }
            _ => {
                let expr = self.parse_expression()?;
                let is_compound = match self.peek() {
                    Some(Token::Assign) => Some(None),
                    Some(Token::PlusEq) => Some(Some(BinOp::Add)),
                    Some(Token::MinusEq) => Some(Some(BinOp::Sub)),
                    Some(Token::StarEq) => Some(Some(BinOp::Mul)),
                    _ => None,
                };

                if let Some(op_opt) = is_compound {
                    self.advance(); // consume the operator token
                    let mut value = self.parse_expression()?;
                    self.consume_statement_end()?;

                    // Validate LHS is identifier, member access, or subscript expression
                    match &expr {
                        Expr::Identifier(_) | Expr::MemberAccess { .. } | Expr::Subscript { .. } => {
                            if let Some(op) = op_opt {
                                value = Expr::Binary {
                                    op,
                                    left: Box::new(expr.clone()),
                                    right: Box::new(value),
                                };
                            }
                            Ok(Stmt::AssignStmt { lhs: expr, value })
                        }
                        _ => self.error("Invalid left-hand side of assignment expression")
                    }
                } else {
                    self.consume_statement_end()?;
                    Ok(Stmt::ExprStmt(expr))
                }
            }
        }
    }

    fn parse_class_decl(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Class, "Expected 'class' keyword")?;

        let name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            Some(tok) => return self.error(&format!("Expected class name, found {:?}", tok)),
            None => return self.error("Expected class name, found EOF"),
        };

        let mut base_class = None;
        if self.peek() == Some(&Token::OpenParen) {
            self.advance(); // consume '('
            base_class = match self.advance() {
                Some(Token::Identifier(s)) => Some(s),
                Some(tok) => return self.error(&format!("Expected parent class name, found {:?}", tok)),
                None => return self.error("Expected parent class name, found EOF"),
            };
            self.consume(Token::CloseParen, "Expected ')' after parent class")?;
        }

        self.consume(Token::Colon, "Expected ':' to start class block")?;
        
        while self.peek() == Some(&Token::Newline) {
            self.advance();
        }

        self.consume(Token::Indent, "Expected indented class block")?;
        
        let mut members = Vec::new();
        while self.peek() != Some(&Token::Dedent) && self.peek() != Some(&Token::EOF) {
            if self.peek() == Some(&Token::Newline) {
                self.advance();
                continue;
            }
            members.push(self.parse_statement()?);
        }
        self.consume(Token::Dedent, "Expected end of class block (DEDENT)")?;

        Ok(Stmt::ClassDecl {
            name,
            base_class,
            members,
        })
    }

    fn parse_trait_decl(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Trait, "Expected 'trait' keyword")?;
        let name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            Some(tok) => return self.error(&format!("Expected trait name, found {:?}", tok)),
            None => return self.error("Expected trait name, found EOF"),
        };
        self.consume(Token::Colon, "Expected ':' after trait name")?;
        
        while self.peek() == Some(&Token::Newline) {
            self.advance();
        }
        self.consume(Token::Indent, "Expected indented trait block")?;
        
        let mut methods = Vec::new();
        while self.peek() != Some(&Token::Dedent) && self.peek() != Some(&Token::EOF) {
            if self.peek() == Some(&Token::Newline) {
                self.advance();
                continue;
            }
            self.consume(Token::Fn, "Expected 'fn' inside trait block")?;
            let m_name = match self.advance() {
                Some(Token::Identifier(s)) => s,
                Some(tok) => return self.error(&format!("Expected method name, found {:?}", tok)),
                None => return self.error("Expected method name, found EOF"),
            };
            self.consume(Token::OpenParen, "Expected '(' after method name")?;
            let mut params = Vec::new();
            if self.peek() != Some(&Token::CloseParen) {
                params.push(self.parse_param()?);
                while self.peek() == Some(&Token::Comma) {
                    self.advance();
                    params.push(self.parse_param()?);
                }
            }
            self.consume(Token::CloseParen, "Expected ')' after params")?;
            
            let mut return_type = Type::Void;
            if self.peek() == Some(&Token::Arrow) {
                self.advance();
                return_type = self.parse_type()?;
            }
            self.consume_statement_end()?;
            
            methods.push(TraitMethodSignature {
                name: m_name,
                params,
                return_type,
            });
        }
        self.consume(Token::Dedent, "Expected DEDENT at end of trait block")?;
        
        Ok(Stmt::TraitDecl { name, methods })
    }

    fn parse_pyimport(&mut self) -> Result<Stmt, String> {
        self.consume(Token::PyImport, "Expected 'pyimport' keyword")?;

        let library = match self.advance() {
            Some(Token::Identifier(s)) => s,
            Some(tok) => return self.error(&format!("Expected library name, found {:?}", tok)),
            None => return self.error("Expected library name, found EOF"),
        };

        let mut alias = None;
        if let Some(Token::Identifier(s)) = self.peek() {
            if s == "as" {
                self.advance(); // consume "as"
                alias = match self.advance() {
                    Some(Token::Identifier(a)) => Some(a),
                    Some(tok) => return self.error(&format!("Expected library alias, found {:?}", tok)),
                    None => return self.error("Expected library alias, found EOF"),
                };
            }
        }
        self.consume_statement_end()?;

        Ok(Stmt::PyImport { library, alias })
    }

    fn parse_import(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Import, "Expected 'import' keyword")?;

        let mut path = Vec::new();
        loop {
            match self.advance() {
                Some(Token::Identifier(s)) => path.push(s),
                Some(tok) => return self.error(&format!("Expected identifier in import path, found {:?}", tok)),
                None => return self.error("Expected identifier in import path, found EOF"),
            }
            if let Some(Token::Dot) = self.peek() {
                self.advance();
            } else {
                break;
            }
        }

        let mut alias = None;
        if let Some(Token::Identifier(s)) = self.peek() {
            if s == "as" {
                self.advance(); // consume "as"
                alias = match self.advance() {
                    Some(Token::Identifier(a)) => Some(a),
                    Some(tok) => return self.error(&format!("Expected import alias, found {:?}", tok)),
                    None => return self.error("Expected import alias, found EOF"),
                };
            }
        }
        self.consume_statement_end()?;

        Ok(Stmt::Import { path, alias })
    }

    fn parse_from_import(&mut self) -> Result<Stmt, String> {
        self.consume(Token::From, "Expected 'from' keyword")?;

        let mut path = Vec::new();
        loop {
            match self.advance() {
                Some(Token::Identifier(s)) => path.push(s),
                Some(tok) => return self.error(&format!("Expected identifier in import path, found {:?}", tok)),
                None => return self.error("Expected identifier in import path, found EOF"),
            }
            if let Some(Token::Dot) = self.peek() {
                self.advance();
            } else {
                break;
            }
        }

        match self.advance() {
            Some(Token::Identifier(s)) if s == "import" => {}
            Some(Token::Import) => {}
            Some(tok) => return self.error(&format!("Expected 'import' keyword, found {:?}", tok)),
            None => return self.error("Expected 'import' keyword, found EOF"),
        }

        let mut symbols = Vec::new();
        loop {
            match self.advance() {
                Some(Token::Identifier(s)) => symbols.push(s),
                Some(tok) => return self.error(&format!("Expected symbol identifier, found {:?}", tok)),
                None => return self.error("Expected symbol identifier, found EOF"),
            }
            if let Some(Token::Comma) = self.peek() {
                self.advance();
            } else {
                break;
            }
        }
        self.consume_statement_end()?;

        Ok(Stmt::FromImport { path, symbols })
    }

    fn parse_function_decl(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Fn, "Expected 'fn' keyword")?;
        self.parse_function_decl_body(false, Vec::new())
    }

    fn parse_async_function_decl(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Fn, "Expected 'fn' keyword after 'async'")?;
        self.parse_function_decl_body(true, Vec::new())
    }

    fn parse_function_decl_body(&mut self, is_async: bool, decorators: Vec<String>) -> Result<Stmt, String> {
        let name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            Some(tok) => return self.error(&format!("Expected function name, found {:?}", tok)),
            None => return self.error("Expected function name, found EOF"),
        };

        if name == "__init__" {
            return self.error("MPS uses `fn init()` not `fn __init__()`.\n         Did you mean `fn init(...)`?");
        }

        let mut type_params = Vec::new();
        if self.peek() == Some(&Token::Lt) {
            self.advance(); // consume '<'
            while self.peek() != Some(&Token::Gt) {
                match self.advance() {
                    Some(Token::Identifier(tp)) => {
                        type_params.push(tp);
                    }
                    Some(tok) => return self.error(&format!("Expected type parameter name, found {:?}", tok)),
                    None => return self.error("Expected type parameter name, found EOF"),
                }
                if self.peek() == Some(&Token::Comma) {
                    self.advance(); // consume ','
                }
            }
            self.consume(Token::Gt, "Expected '>' after type parameter list")?;
        }

        self.consume(Token::OpenParen, "Expected '(' after function name")?;
        let mut params = Vec::new();
        if self.peek() != Some(&Token::CloseParen) {
            params.push(self.parse_param()?);
            while self.peek() == Some(&Token::Comma) {
                self.advance(); // consume ','
                params.push(self.parse_param()?);
            }
        }
        self.consume(Token::CloseParen, "Expected ')' after parameter list")?;

        let mut return_type = Type::Void;
        if self.peek() == Some(&Token::Arrow) {
            self.advance(); // consume '->'
            return_type = self.parse_type()?;
        }

        let body = self.parse_block()?;
        Ok(Stmt::FunctionDecl {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            decorators,
        })
    }

    fn parse_param(&mut self) -> Result<Param, String> {
        let name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            Some(tok) => return self.error(&format!("Expected parameter name, found {:?}", tok)),
            None => return self.error("Expected parameter name, found EOF"),
        };

        if name == "self" {
            return Ok(Param {
                name,
                param_type: Type::Void,
            });
        }

        self.consume(Token::Colon, "Expected ':' after parameter name")?;
        let param_type = self.parse_type()?;

        Ok(Param { name, param_type })
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match self.peek() {
            Some(Token::Fn) => {
                self.advance(); // consume 'fn'
                self.consume(Token::OpenParen, "Expected '(' after 'fn' type")?;
                let mut params = Vec::new();
                if self.peek() != Some(&Token::CloseParen) {
                    params.push(self.parse_type()?);
                    while self.peek() == Some(&Token::Comma) {
                        self.advance(); // consume ','
                        params.push(self.parse_type()?);
                    }
                }
                self.consume(Token::CloseParen, "Expected ')' after 'fn' type parameters")?;
                
                self.consume(Token::Arrow, "Expected '->' for function type return value")?;
                let return_type = self.parse_type()?;
                Ok(Type::Function {
                    params,
                    return_type: Box::new(return_type),
                })
            }
            _ => {
                match self.advance() {
                    Some(Token::IntType) => Ok(Type::Int),
                    Some(Token::FloatType) => Ok(Type::Float),
                    Some(Token::Float32Type) => Ok(Type::Float32),
                    Some(Token::StringType) => Ok(Type::String),
                    Some(Token::BoolType) => Ok(Type::Bool),
                    Some(Token::VoidType) => Ok(Type::Void),
                    Some(Token::Identifier(s)) => {
                        if self.peek() == Some(&Token::Lt) {
                            self.advance(); // consume '<'
                            let mut type_args = Vec::new();
                            if self.peek() != Some(&Token::Gt) {
                                type_args.push(self.parse_type()?);
                                while self.peek() == Some(&Token::Comma) {
                                    self.advance(); // consume ','
                                    type_args.push(self.parse_type()?);
                                }
                            }
                            self.consume(Token::Gt, "Expected '>' after generic type arguments")?;
                            let args_strs: Vec<String> = type_args.iter().map(|t| t.to_string()).collect();
                            Ok(Type::Custom(format!("{}<{}>", s, args_strs.join(", "))))
                        } else {
                            Ok(Type::Custom(s))
                        }
                    }
                    Some(tok) => self.error(&format!("Expected type name, found {:?}", tok)),
                    None => self.error("Expected type name, found EOF"),
                }
            }
        }
    }

    fn parse_variable_decl(&mut self, is_const: bool) -> Result<Stmt, String> {
        let expected_kw = if is_const { Token::Const } else { Token::Let };
        self.consume(expected_kw, "Expected variable declaration keyword")?;

        let mut names = Vec::new();
        match self.advance() {
            Some(Token::Identifier(s)) => names.push(s),
            Some(tok) => return self.error(&format!("Expected variable name, found {:?}", tok)),
            None => return self.error("Expected variable name, found EOF"),
        }

        while self.peek() == Some(&Token::Comma) {
            self.advance(); // consume ','
            match self.advance() {
                Some(Token::Identifier(s)) => names.push(s),
                Some(tok) => return self.error(&format!("Expected variable name after comma, found {:?}", tok)),
                None => return self.error("Expected variable name after comma, found EOF"),
            }
        }

        if names.len() > 1 {
            if is_const {
                return self.error("Tuple unpacking cannot be used with 'const'. Use 'let' instead.");
            }
            self.consume(Token::Assign, "Expected '=' in tuple unpacking")?;
            let init = self.parse_expression()?;
            self.consume_statement_end()?;
            return Ok(Stmt::TupleUnpack { vars: names, init });
        }

        let name = names.remove(0);
        let mut var_type = None;
        if self.peek() == Some(&Token::Colon) {
            self.advance(); // consume ':'
            var_type = Some(self.parse_type()?);
        }

        let mut init = None;
        if self.peek() == Some(&Token::Assign) {
            self.advance(); // consume '='
            init = Some(self.parse_expression()?);
        } else {
            if var_type.is_none() {
                return self.error("Variables must either have an explicit type or an initializer expression.");
            }
        }
        self.consume_statement_end()?;

        Ok(Stmt::VariableDecl {
            name,
            is_const,
            var_type,
            init,
        })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(Token::If, "Expected 'if'")?;
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;

        let mut else_branch = None;
        let saved_pos = self.pos;
        while self.peek() == Some(&Token::Newline) {
            self.advance();
        }
        if self.peek() == Some(&Token::Else) {
            self.advance(); // consume 'else'
            else_branch = Some(self.parse_block()?);
        } else if self.peek() == Some(&Token::Elif) {
            let elif_stmt = self.parse_elif_stmt()?;
            else_branch = Some(vec![elif_stmt]);
        } else {
            self.pos = saved_pos;
        }

        Ok(Stmt::IfStmt {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_elif_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Elif, "Expected 'elif'")?;
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;

        let mut else_branch = None;
        let saved_pos = self.pos;
        while self.peek() == Some(&Token::Newline) {
            self.advance();
        }
        if self.peek() == Some(&Token::Else) {
            self.advance(); // consume 'else'
            else_branch = Some(self.parse_block()?);
        } else if self.peek() == Some(&Token::Elif) {
            let elif_stmt = self.parse_elif_stmt()?;
            else_branch = Some(vec![elif_stmt]);
        } else {
            self.pos = saved_pos;
        }

        Ok(Stmt::IfStmt {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(Token::While, "Expected 'while'")?;
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Stmt::WhileStmt { condition, body })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(Token::For, "Expected 'for' keyword")?;

        let var_name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            Some(tok) => return self.error(&format!("Expected loop variable name, found {:?}", tok)),
            None => return self.error("Expected loop variable name, found EOF"),
        };

        self.consume(Token::In, "Expected 'in' keyword")?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Stmt::ForStmt {
            var_name,
            iterable,
            body,
        })
    }

    fn parse_try_catch_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Try, "Expected 'try'")?;
        let try_branch = self.parse_block()?;
        
        self.consume(Token::Catch, "Expected 'catch' block after try")?;
        let catch_var = match self.advance() {
            Some(Token::Identifier(s)) => s,
            Some(tok) => return self.error(&format!("Expected catch variable, found {:?}", tok)),
            None => return self.error("Expected catch variable, found EOF"),
        };
        let catch_branch = self.parse_block()?;
        
        let mut finally_branch = None;
        let saved_pos = self.pos;
        while self.peek() == Some(&Token::Newline) {
            self.advance();
        }
        if self.peek() == Some(&Token::Finally) {
            self.advance(); // consume 'finally'
            finally_branch = Some(self.parse_block()?);
        } else {
            self.pos = saved_pos;
        }
        
        Ok(Stmt::TryCatchStmt {
            try_branch,
            catch_var,
            catch_branch,
            finally_branch,
        })
    }

    fn parse_raise_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Raise, "Expected 'raise'")?;
        let expr = self.parse_expression()?;
        self.consume_statement_end()?;
        Ok(Stmt::RaiseStmt(expr))
    }

    fn parse_match_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Match, "Expected 'match'")?;
        let value = self.parse_expression()?;
        self.consume(Token::Colon, "Expected ':' after match value")?;
        
        while self.peek() == Some(&Token::Newline) {
            self.advance();
        }
        self.consume(Token::Indent, "Expected indented match block")?;
        
        let mut cases = Vec::new();
        while self.peek() != Some(&Token::Dedent) && self.peek() != Some(&Token::EOF) {
            if self.peek() == Some(&Token::Newline) {
                self.advance();
                continue;
            }
            self.consume(Token::Case, "Expected 'case' inside match block")?;
            let pattern = match self.peek() {
                Some(Token::IntLiteral(n)) => {
                    let v = *n;
                    self.advance();
                    MatchPattern::Literal(Literal::Int(v))
                }
                Some(Token::FloatLiteral(f)) => {
                    let v = *f;
                    self.advance();
                    MatchPattern::Literal(Literal::Float(v))
                }
                Some(Token::StringLiteral(s)) => {
                    let v = s.clone();
                    self.advance();
                    MatchPattern::Literal(Literal::String(v))
                }
                Some(Token::True) => {
                    self.advance();
                    MatchPattern::Literal(Literal::Bool(true))
                }
                Some(Token::False) => {
                    self.advance();
                    MatchPattern::Literal(Literal::Bool(false))
                }
                Some(Token::Underscore) => {
                    self.advance();
                    MatchPattern::Wildcard
                }
                Some(tok) => return self.error(&format!("Expected pattern literal or '_', found {:?}", tok)),
                None => return self.error("Expected pattern, found EOF"),
            };
            let body = self.parse_block()?;
            cases.push(MatchCase { pattern, body });
        }
        self.consume(Token::Dedent, "Expected DEDENT at end of match block")?;
        
        Ok(Stmt::MatchStmt { value, cases })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, String> {
        self.consume(Token::Return, "Expected 'return'")?;
        let mut expr = None;
        if self.peek() != Some(&Token::Newline) && self.peek() != Some(&Token::Dedent) && self.peek() != Some(&Token::EOF) {
            expr = Some(self.parse_expression()?);
        }
        self.consume_statement_end()?;
        Ok(Stmt::ReturnStmt(expr))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.consume(Token::Colon, "Expected ':' to start block")?;
        
        while self.peek() == Some(&Token::Newline) {
            self.advance();
        }

        self.consume(Token::Indent, "Expected indented block (INDENT)")?;
        let mut body = Vec::new();
        while self.peek() != Some(&Token::Dedent) && self.peek() != Some(&Token::EOF) {
            if self.peek() == Some(&Token::Newline) {
                self.advance();
                continue;
            }
            body.push(self.parse_statement()?);
        }
        self.consume(Token::Dedent, "Expected end of indented block (DEDENT)")?;
        Ok(body)
    }

    fn consume_statement_end(&mut self) -> Result<(), String> {
        match self.peek() {
            Some(Token::Newline) => {
                self.advance();
                Ok(())
            }
            Some(Token::Dedent) | Some(Token::EOF) => Ok(()),
            Some(tok) => self.error(&format!(
                "Expected end of statement or newline, found token {:?}",
                tok
            )),
            None => Ok(()),
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_logical_and()?;
        while self.peek() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_pipe()?;
        while self.peek() == Some(&Token::And) {
            self.advance();
            let right = self.parse_pipe()?;
            left = Expr::Binary {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_pipe(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        while self.peek() == Some(&Token::Pipe) {
            self.advance(); // consume '|>'
            let right = self.parse_comparison()?;
            
            // Rewrite left |> right into nested call nodes!
            left = match right {
                Expr::Call { name, type_args, mut args } => {
                    args.insert(0, left);
                    Expr::Call { name, type_args, args }
                }
                Expr::MemberCall { object, method, mut args } => {
                    args.insert(0, left);
                    Expr::MemberCall { object, method, args }
                }
                Expr::Identifier(name) => {
                    Expr::Call { name, type_args: Vec::new(), args: vec![left] }
                }
                other => {
                    Expr::Call {
                        name: "call_lambda".to_string(), // type checker will lower correctly
                        type_args: Vec::new(),
                        args: vec![other, left],
                    }
                }
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        while let Some(op) = self.peek_binop_comparison() {
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn peek_binop_comparison(&self) -> Option<BinOp> {
        match self.peek() {
            Some(Token::Eq) => Some(BinOp::Eq),
            Some(Token::Ne) => Some(BinOp::Ne),
            Some(Token::Lt) => Some(BinOp::Lt),
            Some(Token::Le) => Some(BinOp::Le),
            Some(Token::Gt) => Some(BinOp::Gt),
            Some(Token::Ge) => Some(BinOp::Ge),
            _ => None,
        }
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        while let Some(op) = self.peek_binop_additive() {
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn peek_binop_additive(&self) -> Option<BinOp> {
        match self.peek() {
            Some(Token::Plus) => Some(BinOp::Add),
            Some(Token::Minus) => Some(BinOp::Sub),
            _ => None,
        }
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_power()?;
        while let Some(op) = self.peek_binop_multiplicative() {
            self.advance();
            let right = self.parse_power()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let left = self.parse_unary()?;
        if self.peek() == Some(&Token::Pow) {
            self.advance();
            let right = self.parse_unary()?;
            Ok(Expr::Binary {
                op: BinOp::Pow,
                left: Box::new(left),
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::Minus) => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                })
            }
            Some(Token::Not) | Some(Token::Bang) => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_call_or_primary(),
        }
    }

    fn peek_binop_multiplicative(&self) -> Option<BinOp> {
        match self.peek() {
            Some(Token::Star) => Some(BinOp::Mul),
            Some(Token::Slash) => Some(BinOp::Div),
            Some(Token::Percent) => Some(BinOp::Percent),
            _ => None,
        }
    }

    fn is_generic_call(&self) -> bool {
        if self.peek() != Some(&Token::Lt) {
            return false;
        }
        let mut n = 1;
        let mut depth = 1;
        while let Some(tok) = self.peek_ahead(n) {
            match tok {
                Token::Lt => depth += 1,
                Token::Gt => {
                    depth -= 1;
                    if depth == 0 {
                        // Look at the token after '>'
                        if self.peek_ahead(n + 1) == Some(&Token::OpenParen) {
                            return true;
                        }
                        return false;
                    }
                }
                _ => {}
            }
            n += 1;
        }
        false
    }

    fn parse_call_or_primary(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.is_generic_call() {
                self.consume(Token::Lt, "Expected '<'")?;
                let mut type_args = Vec::new();
                if self.peek() != Some(&Token::Gt) {
                    type_args.push(self.parse_type()?);
                    while self.peek() == Some(&Token::Comma) {
                        self.advance(); // consume ','
                        type_args.push(self.parse_type()?);
                    }
                }
                self.consume(Token::Gt, "Expected '>' after generic type arguments")?;
                
                self.consume(Token::OpenParen, "Expected '(' after generic type arguments")?;
                let mut args = Vec::new();
                if self.peek() != Some(&Token::CloseParen) {
                    args.push(self.parse_expression()?);
                    while self.peek() == Some(&Token::Comma) {
                        self.advance(); // consume ','
                        args.push(self.parse_expression()?);
                    }
                }
                self.consume(Token::CloseParen, "Expected ')' after arguments")?;
                
                match expr {
                    Expr::Identifier(name) => {
                        expr = Expr::Call { name, type_args, args };
                    }
                    _ => return self.error("Expected identifier for generic call"),
                }
                continue;
            }

            match self.peek() {
                Some(Token::OpenParen) => {
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    if self.peek() != Some(&Token::CloseParen) {
                        args.push(self.parse_expression()?);
                        while self.peek() == Some(&Token::Comma) {
                            self.advance(); // consume ','
                            args.push(self.parse_expression()?);
                        }
                    }
                    self.consume(Token::CloseParen, "Expected ')' after arguments")?;

                    match expr {
                        Expr::Identifier(name) => {
                            expr = Expr::Call { name, type_args: Vec::new(), args };
                        }
                        Expr::MemberAccess { object, member } => {
                            if let Expr::Super = *object {
                                expr = Expr::SuperCall {
                                    method: member,
                                    args,
                                };
                            } else {
                                expr = Expr::MemberCall {
                                    object,
                                    method: member,
                                    args,
                                };
                            }
                        }
                        Expr::OptionalMemberAccess { object, member } => {
                            expr = Expr::OptionalMemberCall {
                                object,
                                method: member,
                                args,
                            };
                        }
                        _ => {
                            return self.error("Expected function or method identifier for call");
                        }
                    }
                }
                Some(Token::Dot) => {
                    self.advance(); // consume '.'
                    let member = match self.advance() {
                        Some(Token::Identifier(s)) => s,
                        Some(tok) => return self.error(&format!("Expected member name, found {:?}", tok)),
                        None => return self.error("Expected member name, found EOF"),
                    };
                    expr = Expr::MemberAccess {
                        object: Box::new(expr),
                        member,
                    };
                }
                Some(Token::QuestionDot) => {
                    self.advance(); // consume '?.'
                    let member = match self.advance() {
                        Some(Token::Identifier(s)) => s,
                        Some(tok) => return self.error(&format!("Expected member name, found {:?}", tok)),
                        None => return self.error("Expected member name, found EOF"),
                    };
                    expr = Expr::OptionalMemberAccess {
                        object: Box::new(expr),
                        member,
                    };
                }
                Some(Token::OpenBracket) => {
                    self.advance(); // consume '['
                    if self.peek() == Some(&Token::Colon) {
                        self.advance(); // consume ':'
                        let end = if self.peek() != Some(&Token::CloseBracket) {
                            Some(Box::new(self.parse_expression()?))
                        } else {
                            None
                        };
                        self.consume(Token::CloseBracket, "Expected ']' after slice")?;
                        expr = Expr::Slice {
                            object: Box::new(expr),
                            start: None,
                            end,
                        };
                    } else {
                        let first = self.parse_expression()?;
                        if self.peek() == Some(&Token::Colon) {
                            self.advance(); // consume ':'
                            let end = if self.peek() != Some(&Token::CloseBracket) {
                                Some(Box::new(self.parse_expression()?))
                            } else {
                                None
                            };
                            self.consume(Token::CloseBracket, "Expected ']' after slice")?;
                            expr = Expr::Slice {
                                object: Box::new(expr),
                                start: Some(Box::new(first)),
                                end,
                            };
                        } else {
                            self.consume(Token::CloseBracket, "Expected ']' after subscript index")?;
                            expr = Expr::Subscript {
                                object: Box::new(expr),
                                index: Box::new(first),
                            };
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::IntLiteral(n)) => {
                let val = *n;
                self.advance();
                Ok(Expr::Literal(Literal::Int(val)))
            }
            Some(Token::FloatLiteral(f)) => {
                let val = *f;
                self.advance();
                Ok(Expr::Literal(Literal::Float(val)))
            }
            Some(Token::StringLiteral(s)) => {
                let val = s.clone();
                self.advance();
                Ok(Expr::Literal(Literal::String(val)))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            Some(Token::Identifier(s)) => {
                let name = s.clone();
                self.advance();
                Ok(Expr::Identifier(name))
            }
            Some(Token::Super) => {
                self.advance();
                Ok(Expr::Super)
            }
            Some(Token::Await) => {
                self.advance();
                let expr = self.parse_expression()?;
                Ok(Expr::AwaitExpr(Box::new(expr)))
            }
            Some(Token::Fn) => {
                self.advance(); // consume 'fn'
                self.consume(Token::OpenParen, "Expected '(' after fn")?;
                let mut params = Vec::new();
                if self.peek() != Some(&Token::CloseParen) {
                    params.push(self.parse_param()?);
                    while self.peek() == Some(&Token::Comma) {
                        self.advance();
                        params.push(self.parse_param()?);
                    }
                }
                self.consume(Token::CloseParen, "Expected ')' after parameters list")?;
                
                let mut return_type = Type::Void;
                if self.peek() == Some(&Token::Arrow) {
                    self.advance();
                    return_type = self.parse_type()?;
                }
                
                self.consume(Token::Colon, "Expected ':' before lambda body")?;
                let body = self.parse_expression()?;
                Ok(Expr::Lambda {
                    params,
                    return_type,
                    body: Box::new(body),
                })
            }
            Some(Token::OpenBracket) => {
                self.advance(); // consume '['
                if self.peek() == Some(&Token::CloseBracket) {
                    self.advance();
                    return Ok(Expr::ListLiteral(Vec::new()));
                }
                let first = self.parse_expression()?;
                if self.peek() == Some(&Token::For) {
                    self.advance(); // consume 'for'
                    let var_name = match self.advance() {
                        Some(Token::Identifier(s)) => s,
                        Some(tok) => return self.error(&format!("Expected loop variable name in list comprehension, found {:?}", tok)),
                        None => return self.error("Expected loop variable name in list comprehension, found EOF"),
                    };
                    self.consume(Token::In, "Expected 'in' keyword in list comprehension")?;
                    let iterable = self.parse_expression()?;
                    self.consume(Token::CloseBracket, "Expected ']' at end of list comprehension")?;
                    Ok(Expr::ListComprehension {
                        element: Box::new(first),
                        var_name,
                        iterable: Box::new(iterable),
                    })
                } else {
                    let mut elements = vec![first];
                    while self.peek() == Some(&Token::Comma) {
                        self.advance(); // consume ','
                        if self.peek() == Some(&Token::CloseBracket) {
                            break;
                        }
                        elements.push(self.parse_expression()?);
                    }
                    self.consume(Token::CloseBracket, "Expected ']' at end of list literal")?;
                    Ok(Expr::ListLiteral(elements))
                }
            }
            Some(Token::OpenBrace) => {
                self.advance(); // consume '{'
                let mut pairs = Vec::new();
                if self.peek() != Some(&Token::CloseBrace) {
                    let key = self.parse_expression()?;
                    self.consume(Token::Colon, "Expected ':' after dict key")?;
                    let val = self.parse_expression()?;
                    pairs.push((key, val));
                    while self.peek() == Some(&Token::Comma) {
                        self.advance(); // consume ','
                        if self.peek() == Some(&Token::CloseBrace) {
                            break;
                        }
                        let key = self.parse_expression()?;
                        self.consume(Token::Colon, "Expected ':' after dict key")?;
                        let val = self.parse_expression()?;
                        pairs.push((key, val));
                    }
                }
                self.consume(Token::CloseBrace, "Expected '}' at end of dict literal")?;
                Ok(Expr::DictLiteral(pairs))
            }
            Some(Token::OpenParen) => {
                self.advance(); // consume '('
                if self.peek() == Some(&Token::CloseParen) {
                    self.advance();
                    return Ok(Expr::TupleLiteral(Vec::new()));
                }
                let first = self.parse_expression()?;
                if self.peek() == Some(&Token::Comma) {
                    let mut elements = vec![first];
                    while self.peek() == Some(&Token::Comma) {
                        self.advance(); // consume ','
                        if self.peek() == Some(&Token::CloseParen) {
                            break;
                        }
                        elements.push(self.parse_expression()?);
                    }
                    self.consume(Token::CloseParen, "Expected ')' after tuple elements")?;
                    Ok(Expr::TupleLiteral(elements))
                } else {
                    self.consume(Token::CloseParen, "Expected ')' after expression")?;
                    Ok(first)
                }
            }
            Some(tok) => self.error(&format!(
                "Expected expression, found token {:?}",
                tok
            )),
            None => self.error("Expected expression, found EOF"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_simple_program() {
        let mut lex = Lexer::new("let x = 10\nfn hello(a: int) -> int:\n    return a + 1");
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 2);
        assert!(matches!(prog.statements[0], Stmt::VariableDecl { .. }));
        assert!(matches!(prog.statements[1], Stmt::FunctionDecl { .. }));
    }

    #[test]
    fn test_parse_if_while_comparison() {
        let mut lex = Lexer::new("if x < 20:\n    print(1)\nelse:\n    print(0)");
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        assert!(matches!(prog.statements[0], Stmt::IfStmt { .. }));
    }

    #[test]
    fn test_parse_for_loop() {
        let mut lex = Lexer::new("for i in range(0, 100):\n    print(i)");
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        assert!(matches!(prog.statements[0], Stmt::ForStmt { .. }));
    }

    #[test]
    fn test_parse_class() {
        let mut lex = Lexer::new("class Dog(Animal):\n    let age: int\n    fn bark(self):\n        print(self.age)");
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        assert!(matches!(prog.statements[0], Stmt::ClassDecl { .. }));
        if let Stmt::ClassDecl { name, base_class, members } = &prog.statements[0] {
            assert_eq!(name, "Dog");
            assert_eq!(base_class.as_deref(), Some("Animal"));
            assert_eq!(members.len(), 2);
            assert!(matches!(members[0], Stmt::VariableDecl { .. }));
            assert!(matches!(members[1], Stmt::FunctionDecl { .. }));
        }
    }

    #[test]
    fn test_parse_pyimport() {
        let mut lex = Lexer::new("pyimport math as pym\npyimport numpy");
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 2);
        if let Stmt::PyImport { library, alias } = &prog.statements[0] {
            assert_eq!(library, "math");
            assert_eq!(alias.as_deref(), Some("pym"));
        } else {
            panic!("Expected PyImport statement");
        }

        if let Stmt::PyImport { library, alias } = &prog.statements[1] {
            assert_eq!(library, "numpy");
            assert_eq!(alias, &None);
        } else {
            panic!("Expected PyImport statement");
        }
    }

    #[test]
    fn test_parse_subscript() {
        let mut lex = Lexer::new("let val = arr[0]\narr[1] = 99\nlet nested = grid[i][j]");
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 3);
        
        // let val = arr[0]
        if let Stmt::VariableDecl { name, init, .. } = &prog.statements[0] {
            assert_eq!(name, "val");
            assert!(matches!(init.as_ref().unwrap(), Expr::Subscript { .. }));
        } else {
            panic!("Expected VariableDecl statement");
        }

        // arr[1] = 99
        if let Stmt::AssignStmt { lhs, value } = &prog.statements[1] {
            assert!(matches!(lhs, Expr::Subscript { .. }));
            assert!(matches!(value, Expr::Literal(Literal::Int(99))));
        } else {
            panic!("Expected AssignStmt statement");
        }

        // let nested = grid[i][j]
        if let Stmt::VariableDecl { name, init, .. } = &prog.statements[2] {
            assert_eq!(name, "nested");
            if let Expr::Subscript { object, index } = init.as_ref().unwrap() {
                assert!(matches!(**object, Expr::Subscript { .. }));
                assert!(matches!(**index, Expr::Identifier(ref n) if n == "j"));
            } else {
                panic!("Expected nested Subscript expression");
            }
        } else {
            panic!("Expected VariableDecl statement");
        }
    }

    #[test]
    fn test_parse_logical_and_power() {
        let mut lex = Lexer::new("let ok = a > 0 and b ** 2 > 4 or c\n");
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::VariableDecl { init, .. } = &prog.statements[0] {
            if let Expr::Binary { op: BinOp::Or, left, right } = init.as_ref().unwrap() {
                assert!(matches!(**right, Expr::Identifier(ref name) if name == "c"));
                if let Expr::Binary { op: BinOp::And, .. } = &**left {
                    // ok
                } else {
                    panic!("Expected nested And expression");
                }
            } else {
                panic!("Expected top-level Or expression");
            }
        } else {
            panic!("Expected VariableDecl statement");
        }
    }

    #[test]
    fn test_parse_try_catch() {
        let code = "try:\n    let x = 10\ncatch err:\n    print(err)";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::TryCatchStmt { catch_var, finally_branch, .. } = &prog.statements[0] {
            assert_eq!(catch_var, "err");
            assert!(finally_branch.is_none());
        } else {
            panic!("Expected TryCatchStmt");
        }
    }

    #[test]
    fn test_parse_try_catch_finally() {
        let code = "try:\n    let x = 10\ncatch e:\n    print(e)\nfinally:\n    print(0)";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::TryCatchStmt { catch_var, finally_branch, .. } = &prog.statements[0] {
            assert_eq!(catch_var, "e");
            assert!(finally_branch.is_some());
        } else {
            panic!("Expected TryCatchStmt with finally");
        }
    }

    #[test]
    fn test_parse_match() {
        let code = "match x:\n    case 1:\n        print(1)\n    case 2:\n        print(2)\n    case _:\n        print(0)";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::MatchStmt { cases, .. } = &prog.statements[0] {
            assert_eq!(cases.len(), 3);
            assert!(matches!(cases[0].pattern, MatchPattern::Literal(Literal::Int(1))));
            assert!(matches!(cases[1].pattern, MatchPattern::Literal(Literal::Int(2))));
            assert!(matches!(cases[2].pattern, MatchPattern::Wildcard));
        } else {
            panic!("Expected MatchStmt");
        }
    }

    #[test]
    fn test_parse_elif() {
        let code = "if x == 1:\n    print(1)\nelif x == 2:\n    print(2)\nelse:\n    print(3)";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        assert!(matches!(prog.statements[0], Stmt::IfStmt { .. }));
        // elif is parsed as nested IfStmt inside else_branch
        if let Stmt::IfStmt { else_branch, .. } = &prog.statements[0] {
            assert!(else_branch.is_some());
            let else_stmts = else_branch.as_ref().unwrap();
            assert_eq!(else_stmts.len(), 1);
            assert!(matches!(else_stmts[0], Stmt::IfStmt { .. }));
        }
    }

    #[test]
    fn test_parse_trait() {
        let code = "trait Drawable:\n    fn draw(self) -> void\n    fn area(self) -> float";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::TraitDecl { name, methods } = &prog.statements[0] {
            assert_eq!(name, "Drawable");
            assert_eq!(methods.len(), 2);
            assert_eq!(methods[0].name, "draw");
            assert_eq!(methods[1].name, "area");
        } else {
            panic!("Expected TraitDecl");
        }
    }

    #[test]
    fn test_parse_list_literal() {
        let code = "let nums = [1, 2, 3]";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::VariableDecl { init, .. } = &prog.statements[0] {
            if let Expr::ListLiteral(elems) = init.as_ref().unwrap() {
                assert_eq!(elems.len(), 3);
            } else {
                panic!("Expected ListLiteral");
            }
        } else {
            panic!("Expected VariableDecl");
        }
    }

    #[test]
    fn test_parse_dict_literal() {
        let code = "let d = {\"a\": 1, \"b\": 2}";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::VariableDecl { init, .. } = &prog.statements[0] {
            if let Expr::DictLiteral(pairs) = init.as_ref().unwrap() {
                assert_eq!(pairs.len(), 2);
            } else {
                panic!("Expected DictLiteral");
            }
        } else {
            panic!("Expected VariableDecl");
        }
    }

    #[test]
    fn test_parse_pipe_operator() {
        // x |> double should parse as double(x)
        let code = "let result = x |> double";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::VariableDecl { init, .. } = &prog.statements[0] {
            if let Expr::Call { name, args, .. } = init.as_ref().unwrap() {
                assert_eq!(name, "double");
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0], Expr::Identifier(ref n) if n == "x"));
            } else {
                panic!("Expected Call from pipe rewrite");
            }
        } else {
            panic!("Expected VariableDecl");
        }
    }

    #[test]
    fn test_parse_raise() {
        let code = "raise \"something went wrong\"";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        assert!(matches!(prog.statements[0], Stmt::RaiseStmt(_)));
    }

    #[test]
    fn test_parse_tuple_literal() {
        let code = "let t = (1, 2, 3)";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::VariableDecl { init, .. } = &prog.statements[0] {
            if let Expr::TupleLiteral(elems) = init.as_ref().unwrap() {
                assert_eq!(elems.len(), 3);
            } else {
                panic!("Expected TupleLiteral");
            }
        } else {
            panic!("Expected VariableDecl");
        }
    }

    #[test]
    fn test_parse_lambda() {
        let code = "let f = fn(x: int) -> int: x + 1";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::VariableDecl { init, .. } = &prog.statements[0] {
            assert!(matches!(init.as_ref().unwrap(), Expr::Lambda { .. }));
        } else {
            panic!("Expected VariableDecl with Lambda");
        }
    }

    #[test]
    fn test_parse_async() {
        let code = "async fn fetch() -> string:\n    let res = await get()\n    return res";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::FunctionDecl { is_async, .. } = &prog.statements[0] {
            assert!(*is_async);
        } else {
            panic!("Expected async FunctionDecl");
        }
    }

    #[test]
    fn test_parse_super() {
        let code = "super.describe()";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::ExprStmt(Expr::SuperCall { method, .. }) = &prog.statements[0] {
            assert_eq!(method, "describe");
        } else {
            panic!("Expected SuperCall");
        }
    }

    #[test]
    fn test_parse_operator_overload() {
        let code = "class Point:\n    fn __add__(self, other: Point) -> Point:\n        return Point()";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::ClassDecl { members, .. } = &prog.statements[0] {
            if let Stmt::FunctionDecl { name, .. } = &members[0] {
                assert_eq!(name, "__add__");
            } else {
                panic!("Expected method __add__");
            }
        } else {
            panic!("Expected ClassDecl");
        }
    }

    #[test]
    fn test_len_and_collections_pipeline() {
        let code = "let x = [1, 2, 3]\nlet l = len(x)\nx.pop()\nx.clear()";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 4);

        use crate::typechecker::TypeChecker;
        let mut tc = TypeChecker::new(code.to_string(), "test.mps".to_string());
        let tc_res = tc.typecheck_program(&prog);
        assert!(tc_res.is_ok(), "Typecheck failed: {:?}", tc_res.err());

        use crate::codegen::Codegen;
        let mut cg = Codegen::new();
        let cg_res = cg.transpile_program(&prog);
        assert!(cg_res.is_ok(), "Codegen failed: {:?}", cg_res.err());
        let c_code = cg_res.unwrap();
        
        assert!(c_code.contains("PyObject_Size"), "Should transpile len() to PyObject_Size. Code: {}", c_code);
        assert!(c_code.contains("py_call"), "Should transpile member calls like pop/clear to py_call. Code: {}", c_code);
    }

    #[test]
    fn test_string_operations_pipeline() {
        let code = "let s = \"hello\"\nlet c = s[0]\nlet u = s.upper()\nlet spl = s.split(\",\")";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 4);

        use crate::typechecker::TypeChecker;
        let mut tc = TypeChecker::new(code.to_string(), "test.mps".to_string());
        let tc_res = tc.typecheck_program(&prog);
        assert!(tc_res.is_ok(), "Typecheck failed: {:?}", tc_res.err());

        use crate::codegen::Codegen;
        let mut cg = Codegen::new();
        let cg_res = cg.transpile_program(&prog);
        assert!(cg_res.is_ok(), "Codegen failed: {:?}", cg_res.err());
        let c_code = cg_res.unwrap();
        
        assert!(c_code.contains("mps_str_get_char"), "Should transpile subscript to mps_str_get_char. Code: {}", c_code);
        assert!(c_code.contains("mps_str_upper"), "Should transpile upper() to mps_str_upper. Code: {}", c_code);
        assert!(c_code.contains("mps_str_split"), "Should transpile split() to mps_str_split. Code: {}", c_code);
    }

    #[test]
    fn test_collection_operations_pipeline() {
        let code = "let l = [1, 2]\nl.append(3)\nlet sz = l.length()\nlet d = {\"a\": 1}\nlet has_a = d.contains(\"a\")";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 5);

        use crate::typechecker::TypeChecker;
        let mut tc = TypeChecker::new(code.to_string(), "test.mps".to_string());
        let tc_res = tc.typecheck_program(&prog);
        assert!(tc_res.is_ok(), "Typecheck failed: {:?}", tc_res.err());

        use crate::codegen::Codegen;
        let mut cg = Codegen::new();
        let cg_res = cg.transpile_program(&prog);
        assert!(cg_res.is_ok(), "Codegen failed: {:?}", cg_res.err());
        let c_code = cg_res.unwrap();
        
        assert!(c_code.contains("py_call"), "Should transpile collection methods to py_call. Code: {}", c_code);
        assert!(c_code.contains("mps_to_int"), "Should transpile length() with mps_to_int wrapper. Code: {}", c_code);
        assert!(c_code.contains("mps_to_bool"), "Should transpile contains() with mps_to_bool wrapper. Code: {}", c_code);
    }

    #[test]
    fn test_extensions_parser() {
        // Test compound assignment, slicing, list comprehensions, tuple unpacking, decorators, float32
        let code = "@jit\nfn f(x: float32) -> float32:\n    let a, b = g()\n    a += 1\n    let slice = data[1:10]\n    let comp = [y * 2 for y in range(0, 10)]\n    return x";
        let mut lex = Lexer::new(code);
        let tokens = lex.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();

        assert_eq!(prog.statements.len(), 1);
        if let Stmt::FunctionDecl { decorators, params, return_type, body, .. } = &prog.statements[0] {
            assert_eq!(decorators.len(), 1);
            assert_eq!(decorators[0], "jit");
            assert_eq!(params[0].param_type, Type::Float32);
            assert_eq!(*return_type, Type::Float32);
            
            // let a, b = g()
            assert!(matches!(body[0], Stmt::TupleUnpack { .. }));
            
            // a += 1 (should be lowered to standard AssignStmt with Binary Add)
            if let Stmt::AssignStmt { value, .. } = &body[1] {
                assert!(matches!(value, Expr::Binary { op: BinOp::Add, .. }));
            } else {
                panic!("Expected AssignStmt");
            }
            
            // let slice = data[1:10]
            if let Stmt::VariableDecl { init, .. } = &body[2] {
                assert!(matches!(init.as_ref().unwrap(), Expr::Slice { .. }));
            } else {
                panic!("Expected VariableDecl with Slice");
            }

            // let comp = [y * 2 for y in range(0, 10)]
            if let Stmt::VariableDecl { init, .. } = &body[3] {
                assert!(matches!(init.as_ref().unwrap(), Expr::ListComprehension { .. }));
            } else {
                panic!("Expected VariableDecl with ListComprehension");
            }
        } else {
            panic!("Expected FunctionDecl");
        }
    }
}
