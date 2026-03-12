// ─────────────────────────────────────────────────────────────────────────────
//  RNF Parser  –  Recursive-descent parser
// ─────────────────────────────────────────────────────────────────────────────

use crate::ast::*;
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos:    usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn cur(&self) -> &Token { &self.tokens[self.pos.min(self.tokens.len() - 1)] }

    fn peek_kind(&self) -> &TokenKind { &self.cur().kind }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() - 1 { self.pos += 1; }
        t
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<(), String> {
        if std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind) {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?} but got {:?} at line {}",
                kind, self.peek_kind(), self.cur().line
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        if let TokenKind::Ident(name) = self.peek_kind().clone() {
            self.advance();
            Ok(name)
        } else {
            Err(format!("Expected identifier, got {:?} at line {}", self.peek_kind(), self.cur().line))
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) { self.advance(); true } else { false }
    }

    // ── Entry ─────────────────────────────────────────────────────────────────

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::EOF) {
            self.skip_semis();
            if self.check(&TokenKind::EOF) { break; }
            items.push(self.parse_top_level()?);
            self.skip_semis();
        }
        Ok(Program { items })
    }

    fn skip_semis(&mut self) {
        while self.eat(&TokenKind::Semicolon) {}
    }

    // ── Top-level ─────────────────────────────────────────────────────────────

    fn parse_top_level(&mut self) -> Result<TopLevel, String> {
        let is_pub = self.eat(&TokenKind::Pub);

        match self.peek_kind().clone() {
            TokenKind::Fn     => Ok(TopLevel::Function(self.parse_fn(is_pub)?)),
            TokenKind::Struct => Ok(TopLevel::Struct(self.parse_struct(is_pub)?)),
            TokenKind::Impl   => Ok(TopLevel::Impl(self.parse_impl()?)),
            TokenKind::Use    => {
                self.advance();
                let path = self.expect_ident()?;
                Ok(TopLevel::Use(path))
            }
            TokenKind::Let    => {
                self.advance();
                let name = self.expect_ident()?;
                let ty = if self.eat(&TokenKind::Colon) { Some(self.parse_type()?) } else { None };
                self.expect(&TokenKind::Assign)?;
                let value = self.parse_expr()?;
                Ok(TopLevel::GlobalVar { name, ty, value })
            }
            TokenKind::Ident(_) => {
                // Shell-style: name = expr  (global var)
                let name = self.expect_ident()?;
                if self.eat(&TokenKind::Assign) {
                    let value = self.parse_expr()?;
                    Ok(TopLevel::GlobalVar { name, ty: None, value })
                } else {
                    Err(format!("Unexpected identifier '{}' at top level — line {}", name, self.cur().line))
                }
            }
            _ => Err(format!("Unexpected token {:?} at top level (line {})", self.peek_kind(), self.cur().line)),
        }
    }

    // ── Function ──────────────────────────────────────────────────────────────

    fn parse_fn(&mut self, is_pub: bool) -> Result<FunctionDecl, String> {
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;
        let ret_type = if self.eat(&TokenKind::Arrow) { Some(self.parse_type()?) } else { None };
        let body = self.parse_block()?;
        Ok(FunctionDecl { name, params, ret_type, body, is_pub })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) {
            let name = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            params.push(Param { name, ty });
            if !self.eat(&TokenKind::Comma) { break; }
        }
        Ok(params)
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match self.peek_kind().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(match name.as_str() {
                    "int"   => Type::Int,
                    "float" => Type::Float,
                    "str"   => Type::Str,
                    "bool"  => Type::Bool,
                    "void"  => Type::Void,
                    _       => Type::Custom(name),
                })
            }
            TokenKind::Star | TokenKind::Ptr => {
                self.advance();
                Ok(Type::Ptr(Box::new(self.parse_type()?)))
            }
            TokenKind::LBracket => {
                self.advance();
                let inner = self.parse_type()?;
                let sz = if self.eat(&TokenKind::Semicolon) {
                    if let TokenKind::Int(n) = self.peek_kind().clone() {
                        self.advance(); Some(n as usize)
                    } else { None }
                } else { None };
                self.expect(&TokenKind::RBracket)?;
                Ok(Type::Array(Box::new(inner), sz))
            }
            _ => Err(format!("Expected type, got {:?} at line {}", self.peek_kind(), self.cur().line)),
        }
    }

    // ── Block ─────────────────────────────────────────────────────────────────

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        loop {
            self.skip_semis();
            if self.check(&TokenKind::RBrace) || self.check(&TokenKind::EOF) { break; }
            stmts.push(self.parse_stmt()?);
            self.skip_semis();
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(stmts)
    }

    // ── Struct ────────────────────────────────────────────────────────────────

    fn parse_struct(&mut self, is_pub: bool) -> Result<StructDecl, String> {
        self.expect(&TokenKind::Struct)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        loop {
            self.skip_semis();
            if self.check(&TokenKind::RBrace) { break; }
            let fname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ftype = self.parse_type()?;
            fields.push((fname, ftype));
            if !self.eat(&TokenKind::Comma) { self.skip_semis(); }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(StructDecl { name, fields, is_pub })
    }

    fn parse_impl(&mut self) -> Result<ImplDecl, String> {
        self.expect(&TokenKind::Impl)?;
        let target = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut methods = Vec::new();
        loop {
            self.skip_semis();
            if self.check(&TokenKind::RBrace) { break; }
            let is_pub = self.eat(&TokenKind::Pub);
            self.expect(&TokenKind::Fn)?;
            let mname = self.expect_ident()?;
            self.expect(&TokenKind::LParen)?;
            let params = self.parse_params()?;
            self.expect(&TokenKind::RParen)?;
            let ret_type = if self.eat(&TokenKind::Arrow) { Some(self.parse_type()?) } else { None };
            let body = self.parse_block()?;
            methods.push(FunctionDecl { name: mname, params, ret_type, body, is_pub });
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(ImplDecl { target, methods })
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek_kind().clone() {
            // let name[: type] = value
            TokenKind::Let => {
                self.advance();
                let name = self.expect_ident()?;
                let ty = if self.eat(&TokenKind::Colon) { Some(self.parse_type()?) } else { None };
                self.expect(&TokenKind::Assign)?;
                let value = self.parse_expr()?;
                Ok(Stmt::Let { name, ty, value })
            }

            // ptr name = &expr
            TokenKind::Ptr => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&TokenKind::Assign)?;
                let expr = self.parse_expr()?;
                Ok(Stmt::PtrDecl { name, expr })
            }

            // return [expr]
            TokenKind::Return => {
                self.advance();
                let expr = if self.check(&TokenKind::RBrace)
                    || self.check(&TokenKind::Semicolon)
                    || self.check(&TokenKind::EOF)
                {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                Ok(Stmt::Return(expr))
            }

            // if cond { } [else { }]
            TokenKind::If => {
                self.advance();
                let cond = self.parse_expr()?;
                let then_body = self.parse_block()?;
                let else_body = if self.eat(&TokenKind::Else) {
                    if self.check(&TokenKind::If) {
                        Some(vec![self.parse_stmt()?])
                    } else {
                        Some(self.parse_block()?)
                    }
                } else { None };
                Ok(Stmt::If { cond, then_body, else_body })
            }

            // while cond { }
            TokenKind::While => {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::While { cond, body })
            }

            // loop var in range { }
            TokenKind::Loop => {
                self.advance();
                let var = self.expect_ident()?;
                self.expect(&TokenKind::In)?;
                let range = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::Loop { var, range, body })
            }

            // exec "cmd"
            TokenKind::Exec => {
                self.advance();
                let cmd = self.parse_expr()?;
                Ok(Stmt::Exec(cmd))
            }

            // print(args…)  OR  print "string"
            TokenKind::Print => {
                self.advance();
                let args = if self.eat(&TokenKind::LParen) {
                    let mut a = Vec::new();
                    while !self.check(&TokenKind::RParen) {
                        a.push(self.parse_expr()?);
                        if !self.eat(&TokenKind::Comma) { break; }
                    }
                    self.expect(&TokenKind::RParen)?;
                    a
                } else {
                    vec![self.parse_expr()?]
                };
                Ok(Stmt::Print(args))
            }

            // go fn_call
            TokenKind::Go => {
                self.advance();
                let call = self.parse_primary()?;
                Ok(Stmt::Go(call))
            }

            // send(ch, val)
            TokenKind::Send => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let chan  = self.parse_expr()?;
                self.expect(&TokenKind::Comma)?;
                let value = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(Stmt::Send { chan, value })
            }

            // asm { "raw" }
            TokenKind::Asm => {
                self.advance();
                self.expect(&TokenKind::LBrace)?;
                let raw = if let TokenKind::Str(s) = self.peek_kind().clone() {
                    self.advance(); s
                } else { String::new() };
                self.expect(&TokenKind::RBrace)?;
                Ok(Stmt::Asm(raw))
            }

            TokenKind::Break    => { self.advance(); Ok(Stmt::Break) }
            TokenKind::Continue => { self.advance(); Ok(Stmt::Continue) }

            // Shell-style: name = expr  OR  function call
            _ => {
                let expr = self.parse_expr()?;
                // Check for assignment  (target = value)
                if self.eat(&TokenKind::Assign) {
                    let value = self.parse_expr()?;
                    Ok(Stmt::Assign { target: expr, value })
                } else {
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.eat(&TokenKind::Or) {
            let right = self.parse_and()?;
            left = Expr::Binary { op: BinOp::Or, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_eq()?;
        while self.eat(&TokenKind::And) {
            let right = self.parse_eq()?;
            left = Expr::Binary { op: BinOp::And, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_eq(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_cmp()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Eq => BinOp::Eq,
                TokenKind::Ne => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_cmp()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_add()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_add()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus  => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star    => BinOp::Mul,
                TokenKind::Slash   => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek_kind().clone() {
            TokenKind::Not   => { self.advance(); Ok(Expr::Unary { op: UnOp::Not, expr: Box::new(self.parse_unary()?) }) }
            TokenKind::Minus => { self.advance(); Ok(Expr::Unary { op: UnOp::Neg, expr: Box::new(self.parse_unary()?) }) }
            TokenKind::Amp   => { self.advance(); Ok(Expr::Ref(Box::new(self.parse_unary()?))) }
            TokenKind::Star  => { self.advance(); Ok(Expr::Deref(Box::new(self.parse_unary()?))) }
            // <-ch  receive
            TokenKind::ChanRecv => { self.advance(); Ok(Expr::ChanRecv(Box::new(self.parse_primary()?))) }
            _ => self.parse_range(),
        }
    }

    fn parse_range(&mut self) -> Result<Expr, String> {
        let left = self.parse_postfix()?;
        if self.eat(&TokenKind::DotDot) {
            let right = self.parse_postfix()?;
            return Ok(Expr::Range { start: Box::new(left), end: Box::new(right) });
        }
        Ok(left)
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect_ident()?;
                    // method call?
                    if self.eat(&TokenKind::LParen) {
                        let mut args = vec![expr];
                        while !self.check(&TokenKind::RParen) {
                            args.push(self.parse_expr()?);
                            if !self.eat(&TokenKind::Comma) { break; }
                        }
                        self.expect(&TokenKind::RParen)?;
                        expr = Expr::Call { name: field, args };
                    } else {
                        expr = Expr::Field { object: Box::new(expr), field };
                    }
                }
                TokenKind::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(&TokenKind::RBracket)?;
                    expr = Expr::Index { array: Box::new(expr), index: Box::new(idx) };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek_kind().clone() {
            TokenKind::Int(n)   => { self.advance(); Ok(Expr::Int(n)) }
            TokenKind::Float(f) => { self.advance(); Ok(Expr::Float(f)) }
            TokenKind::Str(s)   => { self.advance(); Ok(Expr::Str(s)) }
            TokenKind::Bool(b)  => { self.advance(); Ok(Expr::Bool(b)) }
            TokenKind::Nil      => { self.advance(); Ok(Expr::Nil) }

            // Grouped expression
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(e)
            }

            // Array literal
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !self.check(&TokenKind::RBracket) {
                    items.push(self.parse_expr()?);
                    if !self.eat(&TokenKind::Comma) { break; }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::Array(items))
            }

            // raw_mem(addr)
            TokenKind::RawMem => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let addr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(Expr::RawMem(Box::new(addr)))
            }

            // make_chan()  or  make_chan<type>()
            TokenKind::MakeChan => {
                self.advance();
                let ty = if self.eat(&TokenKind::Lt) {
                    let t = self.parse_type()?;
                    self.expect(&TokenKind::Gt)?;
                    t
                } else { Type::Int };
                if self.eat(&TokenKind::LParen) { self.expect(&TokenKind::RParen)?; }
                Ok(Expr::MakeChan(ty))
            }

            // exec("cmd") as expression — returns captured output
            TokenKind::Exec => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let arg = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(Expr::Call { name: "exec".to_string(), args: vec![arg] })
            }

            // recv(ch) as expression
            TokenKind::Recv => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let ch = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(Expr::ChanRecv(Box::new(ch)))
            }

            // Identifier — may be a call or struct init
            TokenKind::Ident(name) => {
                self.advance();
                if self.eat(&TokenKind::LParen) {
                    // Function call
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::RParen) {
                        args.push(self.parse_expr()?);
                        if !self.eat(&TokenKind::Comma) { break; }
                    }
                    self.expect(&TokenKind::RParen)?;
                    Ok(Expr::Call { name, args })
                } else if name.chars().next().map_or(false, |c| c.is_uppercase())
                    && self.eat(&TokenKind::LBrace)
                {
                    // Struct initialiser: Name { field: val, … }  (uppercase only)
                    let mut fields = Vec::new();
                    loop {
                        self.skip_semis();
                        if self.check(&TokenKind::RBrace) { break; }
                        let fname = self.expect_ident()?;
                        self.expect(&TokenKind::Colon)?;
                        let fval = self.parse_expr()?;
                        fields.push((fname, fval));
                        if !self.eat(&TokenKind::Comma) { self.skip_semis(); }
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Expr::StructInit { name, fields })
                } else {
                    Ok(Expr::Ident(name))
                }
            }

            _ => Err(format!("Unexpected token in expression: {:?} at line {}", self.peek_kind(), self.cur().line)),
        }
    }
}
