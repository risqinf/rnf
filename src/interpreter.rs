// ─────────────────────────────────────────────────────────────────────────────
//  RNF Interpreter  –  Tree-walking interpreter for `rnf --run`
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::ast::*;

// ── Runtime Values ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    Array(Vec<Value>),
    Ptr(Arc<Mutex<Value>>),
    Chan(Arc<Mutex<std::collections::VecDeque<Value>>>),
    Struct { name: String, fields: HashMap<String, Value> },
    Function { params: Vec<Param>, body: Vec<Stmt>, closure: Env },
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_)      => "int",
            Value::Float(_)    => "float",
            Value::Str(_)      => "str",
            Value::Bool(_)     => "bool",
            Value::Nil         => "nil",
            Value::Array(_)    => "array",
            Value::Ptr(_)      => "ptr",
            Value::Chan(_)     => "chan",
            Value::Struct{..}  => "struct",
            Value::Function{..}=> "fn",
        }
    }

    fn to_bool(&self) -> bool {
        match self {
            Value::Bool(b)  => *b,
            Value::Int(n)   => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s)   => !s.is_empty(),
            Value::Nil      => false,
            _               => true,
        }
    }

    fn display(&self) -> String {
        match self {
            Value::Int(n)    => n.to_string(),
            Value::Float(f)  => {
                if f.fract() == 0.0 { format!("{:.1}", f) } else { f.to_string() }
            }
            Value::Str(s)    => s.clone(),
            Value::Bool(b)   => b.to_string(),
            Value::Nil       => "nil".to_string(),
            Value::Array(v)  => {
                let items: Vec<_> = v.iter().map(|x| x.display()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Ptr(_)    => "<ptr>".to_string(),
            Value::Chan(_)   => "<chan>".to_string(),
            Value::Struct { name, fields } => {
                let fs: Vec<_> = fields.iter().map(|(k,v)| format!("{}: {}", k, v.display())).collect();
                format!("{} {{ {} }}", name, fs.join(", "))
            }
            Value::Function{..} => "<fn>".to_string(),
        }
    }
}

// ── Environment ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Env {
    vars:   HashMap<String, Value>,
    parent: Option<Box<Env>>,
}

impl Env {
    pub fn new() -> Self { Env { vars: HashMap::new(), parent: None } }

    pub fn child(parent: Env) -> Self {
        Env { vars: HashMap::new(), parent: Some(Box::new(parent)) }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.vars.get(name) { return Some(v.clone()); }
        self.parent.as_ref().and_then(|p| p.get(name))
    }

    pub fn set(&mut self, name: &str, val: Value) {
        if self.vars.contains_key(name) {
            self.vars.insert(name.to_string(), val);
        } else if let Some(p) = &mut self.parent {
            if p.has(name) { p.set(name, val.clone()); return; }
            self.vars.insert(name.to_string(), val);
        } else {
            self.vars.insert(name.to_string(), val);
        }
    }

    pub fn define(&mut self, name: &str, val: Value) {
        self.vars.insert(name.to_string(), val);
    }

    pub fn has(&self, name: &str) -> bool {
        if self.vars.contains_key(name) { return true; }
        self.parent.as_ref().map_or(false, |p| p.has(name))
    }
}

// ── Control-flow signals ──────────────────────────────────────────────────────

enum Signal {
    Return(Value),
    Break,
    Continue,
}

// ── Interpreter ───────────────────────────────────────────────────────────────

pub struct Interpreter {
    global: Env,
    functions: HashMap<String, FunctionDecl>,
    structs:   HashMap<String, StructDecl>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            global:    Env::new(),
            functions: HashMap::new(),
            structs:   HashMap::new(),
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), String> {
        // First pass: register all top-level declarations
        for item in &program.items {
            match item {
                TopLevel::Function(f) => { self.functions.insert(f.name.clone(), f.clone()); }
                TopLevel::Struct(s)   => { self.structs.insert(s.name.clone(), s.clone()); }
                TopLevel::Impl(i)     => {
                    for method in &i.methods {
                        let full_name = format!("{}::{}", i.target, method.name);
                        self.functions.insert(full_name, method.clone());
                    }
                }
                TopLevel::GlobalVar { name, value, .. } => {
                    let v = self.eval_expr(value, &mut self.global.clone())?;
                    self.global.define(name, v);
                }
                TopLevel::Use(_) => {}
            }
        }

        // Second pass: execute main()
        if let Some(main_fn) = self.functions.get("main").cloned() {
            let mut env = Env::child(self.global.clone());
            self.exec_block(&main_fn.body, &mut env)?;
        } else {
            // No main → run all top-level statements from GlobalVar exprs
            // (scripts without main)
            for item in &program.items.clone() {
                if let TopLevel::GlobalVar { name, value, .. } = item {
                    let v = self.eval_expr(value, &mut self.global.clone())?;
                    self.global.define(name, v);
                }
            }
        }

        Ok(())
    }

    // ── Statement execution ───────────────────────────────────────────────────

    fn exec_block(&mut self, stmts: &[Stmt], env: &mut Env) -> Result<Option<Signal>, String> {
        for stmt in stmts {
            if let Some(sig) = self.exec_stmt(stmt, env)? {
                return Ok(Some(sig));
            }
        }
        Ok(None)
    }

    fn exec_stmt(&mut self, stmt: &Stmt, env: &mut Env) -> Result<Option<Signal>, String> {
        match stmt {
            Stmt::Let { name, value, .. } => {
                let v = self.eval_expr(value, env)?;
                env.define(name, v);
                Ok(None)
            }

            Stmt::PtrDecl { name, expr } => {
                let v = self.eval_expr(expr, env)?;
                // Strip the & if it's a Ref expr — store value in Arc
                let ptr = Value::Ptr(Arc::new(Mutex::new(v)));
                env.define(name, ptr);
                Ok(None)
            }

            Stmt::Assign { target, value } => {
                let v = self.eval_expr(value, env)?;
                self.assign_target(target, v, env)?;
                Ok(None)
            }

            Stmt::If { cond, then_body, else_body } => {
                let c = self.eval_expr(cond, env)?;
                let mut child = Env::child(env.clone());
                if c.to_bool() {
                    self.exec_block(then_body, &mut child)
                } else if let Some(eb) = else_body {
                    self.exec_block(eb, &mut child)
                } else {
                    Ok(None)
                }
            }

            Stmt::While { cond, body } => {
                loop {
                    let c = self.eval_expr(cond, env)?;
                    if !c.to_bool() { break; }
                    match self.exec_block(body, env)? {
                        Some(Signal::Break)    => break,
                        Some(Signal::Continue) => continue,
                        Some(r @ Signal::Return(_)) => return Ok(Some(r)),
                        None => {}
                    }
                }
                Ok(None)
            }

            Stmt::Loop { var, range, body } => {
                match self.eval_expr(range, env)? {
                    Value::Array(items) => {
                        for item in items {
                            env.define(var, item);
                            match self.exec_block(body, env)? {
                                Some(Signal::Break)    => break,
                                Some(Signal::Continue) => continue,
                                Some(r @ Signal::Return(_)) => return Ok(Some(r)),
                                None => {}
                            }
                        }
                    }
                    _ => {
                        return Err("Loop range must be an array or a..b range".into());
                    }
                }
                Ok(None)
            }

            Stmt::Return(expr) => {
                let v = if let Some(e) = expr {
                    self.eval_expr(e, env)?
                } else { Value::Nil };
                Ok(Some(Signal::Return(v)))
            }

            Stmt::Exec(cmd_expr) => {
                let cmd = self.eval_expr(cmd_expr, env)?;
                let cmd_str = cmd.display();
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd_str)
                    .status()
                    .map_err(|e| format!("exec failed: {}", e))?;
                env.define("__exit__", Value::Int(status.code().unwrap_or(0) as i64));
                Ok(None)
            }

            Stmt::Print(args) => {
                let parts: Vec<String> = args.iter()
                    .map(|a| self.eval_expr(a, env).map(|v| v.display()))
                    .collect::<Result<_, _>>()?;
                println!("{}", parts.join(" "));
                Ok(None)
            }

            Stmt::Go(call) => {
                // Spawn a thread for the goroutine
                if let Expr::Call { name, args } = call {
                    let fn_decl = self.functions.get(name).cloned()
                        .ok_or_else(|| format!("Unknown function '{}' in go statement", name))?;
                    let mut eval_args: Vec<Value> = args.iter()
                        .map(|a| self.eval_expr(a, env))
                        .collect::<Result<_, _>>()?;
                    let global_clone = self.global.clone();
                    let functions    = self.functions.clone();
                    let structs      = self.structs.clone();
                    std::thread::spawn(move || {
                        let mut interp = Interpreter { global: global_clone, functions, structs };
                        let mut child  = Env::child(interp.global.clone());
                        for (param, val) in fn_decl.params.iter().zip(eval_args.drain(..)) {
                            child.define(&param.name, val);
                        }
                        let _ = interp.exec_block(&fn_decl.body, &mut child);
                    });
                }
                Ok(None)
            }

            Stmt::Send { chan, value } => {
                let ch_val  = self.eval_expr(chan,  env)?;
                let msg_val = self.eval_expr(value, env)?;
                if let Value::Chan(q) = ch_val {
                    q.lock().unwrap().push_back(msg_val);
                } else {
                    return Err("send() requires a channel value".into());
                }
                Ok(None)
            }

            Stmt::Asm(raw) => {
                eprintln!("[asm] Inline assembly only available in --release mode: {}", raw);
                Ok(None)
            }

            Stmt::Break    => Ok(Some(Signal::Break)),
            Stmt::Continue => Ok(Some(Signal::Continue)),

            Stmt::Expr(e)  => { self.eval_expr(e, env)?; Ok(None) }
        }
    }

    // ── Assign target ─────────────────────────────────────────────────────────

    fn assign_target(&mut self, target: &Expr, val: Value, env: &mut Env) -> Result<(), String> {
        match target {
            Expr::Ident(name) => { env.set(name, val); Ok(()) }
            Expr::Index { array, index } => {
                if let Expr::Ident(name) = array.as_ref() {
                    let idx = self.eval_expr(index, env)?;
                    if let Some(Value::Array(mut arr)) = env.get(name) {
                        if let Value::Int(i) = idx {
                            let i = i as usize;
                            if i < arr.len() { arr[i] = val; env.set(name, Value::Array(arr)); Ok(()) }
                            else { Err(format!("Index {} out of bounds (len {})", i, arr.len())) }
                        } else { Err("Array index must be int".into()) }
                    } else { Err(format!("'{}' is not an array", name)) }
                } else { Err("Complex index assignment not supported".into()) }
            }
            Expr::Deref(ptr_expr) => {
                let ptr = self.eval_expr(ptr_expr, env)?;
                if let Value::Ptr(arc) = ptr { *arc.lock().unwrap() = val; Ok(()) }
                else { Err("Dereference of non-pointer".into()) }
            }
            Expr::Field { object, field } => {
                if let Expr::Ident(name) = object.as_ref() {
                    if let Some(Value::Struct { name: sname, mut fields }) = env.get(name) {
                        fields.insert(field.clone(), val);
                        env.set(name, Value::Struct { name: sname, fields });
                        Ok(())
                    } else { Err(format!("'{}' is not a struct", name)) }
                } else { Err("Complex field assignment not supported".into()) }
            }
            _ => Err("Invalid assignment target".into()),
        }
    }

    // ── Expression evaluation ─────────────────────────────────────────────────

    fn eval_expr(&mut self, expr: &Expr, env: &mut Env) -> Result<Value, String> {
        match expr {
            Expr::Int(n)    => Ok(Value::Int(*n)),
            Expr::Float(f)  => Ok(Value::Float(*f)),
            Expr::Str(s)    => Ok(Value::Str(s.clone())),
            Expr::Bool(b)   => Ok(Value::Bool(*b)),
            Expr::Nil       => Ok(Value::Nil),

            Expr::Ident(name) => {
                env.get(name)
                   .or_else(|| self.global.get(name))
                   .ok_or_else(|| format!("Undefined variable '{}'", name))
            }

            Expr::Array(items) => {
                let vals: Vec<Value> = items.iter()
                    .map(|e| self.eval_expr(e, env))
                    .collect::<Result<_, _>>()?;
                Ok(Value::Array(vals))
            }

            Expr::Range { start, end } => {
                let s = self.eval_expr(start, env)?;
                let e = self.eval_expr(end, env)?;
                if let (Value::Int(a), Value::Int(b)) = (s, e) {
                    Ok(Value::Array((a..b).map(Value::Int).collect()))
                } else {
                    Err("Range start and end must be integers".into())
                }
            }

            Expr::Ref(inner) => {
                let v = self.eval_expr(inner, env)?;
                Ok(Value::Ptr(Arc::new(Mutex::new(v))))
            }

            Expr::Deref(ptr_expr) => {
                let v = self.eval_expr(ptr_expr, env)?;
                match v {
                    Value::Ptr(arc) => Ok(arc.lock().unwrap().clone()),
                    _ => Err(format!("Cannot dereference non-pointer: {}", v.type_name())),
                }
            }

            Expr::RawMem(addr) => {
                let v = self.eval_expr(addr, env)?;
                // In interpreter mode, just return nil (hardware access needs --release)
                eprintln!("[raw_mem] Direct memory access only in --release mode. addr={}", v.display());
                Ok(Value::Nil)
            }

            Expr::MakeChan(_) => {
                Ok(Value::Chan(Arc::new(Mutex::new(std::collections::VecDeque::new()))))
            }

            Expr::ChanRecv(ch_expr) => {
                let v = self.eval_expr(ch_expr, env)?;
                if let Value::Chan(q) = v {
                    // Spin-wait for a message (simple implementation)
                    loop {
                        if let Some(msg) = q.lock().unwrap().pop_front() {
                            return Ok(msg);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                } else {
                    Err("recv requires a channel".into())
                }
            }

            Expr::Binary { op, left, right } => {
                let l = self.eval_expr(left, env)?;
                let r = self.eval_expr(right, env)?;
                self.eval_binary(op, l, r)
            }

            Expr::Unary { op, expr } => {
                let v = self.eval_expr(expr, env)?;
                match op {
                    UnOp::Neg => match v {
                        Value::Int(n)   => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err(format!("Cannot negate {}", v.type_name())),
                    },
                    UnOp::Not => Ok(Value::Bool(!v.to_bool())),
                }
            }

            Expr::Call { name, args } => {
                self.eval_call(name, args, env)
            }

            Expr::Index { array, index } => {
                let arr = self.eval_expr(array, env)?;
                let idx = self.eval_expr(index, env)?;
                match (arr, idx) {
                    (Value::Array(v), Value::Int(i)) => {
                        v.get(i as usize)
                         .cloned()
                         .ok_or_else(|| format!("Index {} out of bounds", i))
                    }
                    (Value::Str(s), Value::Int(i)) => {
                        s.chars().nth(i as usize)
                         .map(|c| Value::Str(c.to_string()))
                         .ok_or_else(|| format!("String index {} out of bounds", i))
                    }
                    _ => Err("Invalid index operation".into()),
                }
            }

            Expr::Field { object, field } => {
                let obj = self.eval_expr(object, env)?;
                match obj {
                    Value::Struct { fields, .. } => {
                        fields.get(field)
                              .cloned()
                              .ok_or_else(|| format!("No field '{}'", field))
                    }
                    _ => Err(format!("Field access on non-struct: {}", obj.type_name())),
                }
            }

            Expr::StructInit { name, fields } => {
                let mut fmap = HashMap::new();
                for (fname, fexpr) in fields {
                    fmap.insert(fname.clone(), self.eval_expr(fexpr, env)?);
                }
                Ok(Value::Struct { name: name.clone(), fields: fmap })
            }

            Expr::Cast { expr, to } => {
                let v = self.eval_expr(expr, env)?;
                match (v, to) {
                    (Value::Int(n),   Type::Float)  => Ok(Value::Float(n as f64)),
                    (Value::Float(f), Type::Int)    => Ok(Value::Int(f as i64)),
                    (Value::Int(n),   Type::Str)    => Ok(Value::Str(n.to_string())),
                    (Value::Float(f), Type::Str)    => Ok(Value::Str(f.to_string())),
                    (Value::Str(s),   Type::Int)    => Ok(Value::Int(s.parse().unwrap_or(0))),
                    (Value::Bool(b),  Type::Int)    => Ok(Value::Int(if b { 1 } else { 0 })),
                    (v, _) => Ok(v),
                }
            }
        }
    }

    // ── Binary evaluation ─────────────────────────────────────────────────────

    fn eval_binary(&self, op: &BinOp, l: Value, r: Value) -> Result<Value, String> {
        match op {
            BinOp::Add => match (l, r) {
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a + b as f64)),
                (Value::Str(a),   Value::Str(b))   => Ok(Value::Str(a + &b)),
                (Value::Str(a),   b)               => Ok(Value::Str(a + &b.display())),
                (a, b) => Err(format!("Cannot add {} and {}", a.type_name(), b.type_name())),
            },
            BinOp::Sub => match (l, r) {
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
                (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a - b as f64)),
                (a, b) => Err(format!("Cannot subtract {} from {}", b.type_name(), a.type_name())),
            },
            BinOp::Mul => match (l, r) {
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a * b as f64)),
                _ => Err("Cannot multiply these types".into()),
            },
            BinOp::Div => match (l, r) {
                (_, Value::Int(0))     => Err("Division by zero".into()),
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a / b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64 / b)),
                (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a / b as f64)),
                _ => Err("Cannot divide these types".into()),
            },
            BinOp::Mod => match (l, r) {
                (Value::Int(a), Value::Int(b)) if b != 0 => Ok(Value::Int(a % b)),
                (Value::Int(_), Value::Int(0)) => Err("Modulo by zero".into()),
                _ => Err("Modulo requires integers".into()),
            },
            BinOp::Eq  => Ok(Value::Bool(self.values_eq(&l, &r))),
            BinOp::Ne  => Ok(Value::Bool(!self.values_eq(&l, &r))),
            BinOp::Lt  => Ok(Value::Bool(self.cmp_values(&l, &r)? < 0)),
            BinOp::Gt  => Ok(Value::Bool(self.cmp_values(&l, &r)? > 0)),
            BinOp::Le  => Ok(Value::Bool(self.cmp_values(&l, &r)? <= 0)),
            BinOp::Ge  => Ok(Value::Bool(self.cmp_values(&l, &r)? >= 0)),
            BinOp::And => Ok(Value::Bool(l.to_bool() && r.to_bool())),
            BinOp::Or  => Ok(Value::Bool(l.to_bool() || r.to_bool())),
        }
    }

    fn values_eq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x),   Value::Int(y))   => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Str(x),   Value::Str(y))   => x == y,
            (Value::Bool(x),  Value::Bool(y))  => x == y,
            (Value::Nil,      Value::Nil)       => true,
            _ => false,
        }
    }

    fn cmp_values(&self, a: &Value, b: &Value) -> Result<i32, String> {
        match (a, b) {
            (Value::Int(x),   Value::Int(y))   => Ok(x.cmp(y) as i32),
            (Value::Float(x), Value::Float(y)) => Ok(x.partial_cmp(y).map(|o| o as i32).unwrap_or(0)),
            (Value::Str(x),   Value::Str(y))   => Ok(x.cmp(y) as i32),
            _ => Err(format!("Cannot compare {} and {}", a.type_name(), b.type_name())),
        }
    }

    // ── Function calls ────────────────────────────────────────────────────────

    fn eval_call(&mut self, name: &str, args: &[Expr], env: &mut Env) -> Result<Value, String> {
        // Evaluate arguments first
        let mut eval_args: Vec<Value> = args.iter()
            .map(|a| self.eval_expr(a, env))
            .collect::<Result<_, _>>()?;

        // ── Built-in functions ────────────────────────────────────────────────
        match name {
            "print" | "println" => {
                let parts: Vec<String> = eval_args.iter().map(|v| v.display()).collect();
                println!("{}", parts.join(" "));
                return Ok(Value::Nil);
            }
            "print_no_newline" | "print_raw" => {
                let parts: Vec<String> = eval_args.iter().map(|v| v.display()).collect();
                print!("{}", parts.join(" "));
                return Ok(Value::Nil);
            }
            "eprint" => {
                let parts: Vec<String> = eval_args.iter().map(|v| v.display()).collect();
                eprintln!("{}", parts.join(" "));
                return Ok(Value::Nil);
            }
            "exec" => {
                let cmd = eval_args.first().map(|v| v.display()).unwrap_or_default();
                let output = std::process::Command::new("sh").arg("-c").arg(&cmd).output()
                    .map_err(|e| format!("exec failed: {}", e))?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                return Ok(Value::Str(stdout.trim_end().to_string()));
            }
            "exec_silent" => {
                let cmd = eval_args.first().map(|v| v.display()).unwrap_or_default();
                std::process::Command::new("sh").arg("-c").arg(&cmd).status().ok();
                return Ok(Value::Nil);
            }
            "len" => {
                return match eval_args.first() {
                    Some(Value::Array(v))  => Ok(Value::Int(v.len() as i64)),
                    Some(Value::Str(s))    => Ok(Value::Int(s.len() as i64)),
                    _                      => Err("len() requires array or string".into()),
                };
            }
            "push" => {
                if let (Some(Value::Array(mut arr)), Some(item)) =
                    (eval_args.first().cloned(), eval_args.get(1).cloned())
                {
                    arr.push(item);
                    // Update variable in env if it was passed by name
                    if let Some(Expr::Ident(name)) = args.first() {
                        env.set(name, Value::Array(arr.clone()));
                    }
                    return Ok(Value::Array(arr));
                }
                return Err("push(array, value) — wrong args".into());
            }
            "pop" => {
                if let Some(Value::Array(mut arr)) = eval_args.first().cloned() {
                    let last = arr.pop().unwrap_or(Value::Nil);
                    if let Some(Expr::Ident(name)) = args.first() {
                        env.set(name, Value::Array(arr));
                    }
                    return Ok(last);
                }
                return Err("pop(array) — wrong args".into());
            }
            "int"   => return Ok(match eval_args.first() {
                Some(Value::Int(n))   => Value::Int(*n),
                Some(Value::Float(f)) => Value::Int(*f as i64),
                Some(Value::Str(s))   => Value::Int(s.parse().unwrap_or(0)),
                Some(Value::Bool(b))  => Value::Int(if *b { 1 } else { 0 }),
                _ => Value::Int(0),
            }),
            "float" => return Ok(match eval_args.first() {
                Some(Value::Int(n))   => Value::Float(*n as f64),
                Some(Value::Float(f)) => Value::Float(*f),
                Some(Value::Str(s))   => Value::Float(s.parse().unwrap_or(0.0)),
                _ => Value::Float(0.0),
            }),
            "str"   => return Ok(Value::Str(eval_args.first().map(|v| v.display()).unwrap_or_default())),
            "bool"  => return Ok(Value::Bool(eval_args.first().map(|v| v.to_bool()).unwrap_or(false))),
            "exit"  => {
                let code = eval_args.first().and_then(|v| if let Value::Int(n) = v { Some(*n) } else { None }).unwrap_or(0);
                std::process::exit(code as i32);
            }
            "sleep_ms" => {
                if let Some(Value::Int(ms)) = eval_args.first() {
                    std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
                }
                return Ok(Value::Nil);
            }
            "env_get" => {
                let key = eval_args.first().map(|v| v.display()).unwrap_or_default();
                return Ok(Value::Str(std::env::var(&key).unwrap_or_default()));
            }
            "env_set" => {
                let key = eval_args.first().map(|v| v.display()).unwrap_or_default();
                let val = eval_args.get(1).map(|v| v.display()).unwrap_or_default();
                std::env::set_var(&key, &val);
                return Ok(Value::Nil);
            }
            "args" => {
                let all: Vec<Value> = std::env::args().map(|a| Value::Str(a)).collect();
                return Ok(Value::Array(all));
            }
            "read_file" => {
                let path = eval_args.first().map(|v| v.display()).unwrap_or_default();
                return Ok(Value::Str(std::fs::read_to_string(&path).unwrap_or_default()));
            }
            "write_file" => {
                let path = eval_args.first().map(|v| v.display()).unwrap_or_default();
                let data = eval_args.get(1).map(|v| v.display()).unwrap_or_default();
                std::fs::write(&path, &data).ok();
                return Ok(Value::Nil);
            }
            "file_exists" => {
                let path = eval_args.first().map(|v| v.display()).unwrap_or_default();
                return Ok(Value::Bool(std::path::Path::new(&path).exists()));
            }
            "split" => {
                let s   = eval_args.first().map(|v| v.display()).unwrap_or_default();
                let sep = eval_args.get(1).map(|v| v.display()).unwrap_or_else(|| " ".into());
                let parts: Vec<Value> = s.split(sep.as_str()).map(|p| Value::Str(p.to_string())).collect();
                return Ok(Value::Array(parts));
            }
            "join" => {
                if let Some(Value::Array(arr)) = eval_args.first() {
                    let sep = eval_args.get(1).map(|v| v.display()).unwrap_or_default();
                    let s: Vec<String> = arr.iter().map(|v| v.display()).collect();
                    return Ok(Value::Str(s.join(&sep)));
                }
                return Err("join(array, sep)".into());
            }
            "trim"    => return Ok(Value::Str(eval_args.first().map(|v| v.display().trim().to_string()).unwrap_or_default())),
            "to_upper"=> return Ok(Value::Str(eval_args.first().map(|v| v.display().to_uppercase()).unwrap_or_default())),
            "to_lower"=> return Ok(Value::Str(eval_args.first().map(|v| v.display().to_lowercase()).unwrap_or_default())),
            "contains"=> {
                let s = eval_args.first().map(|v| v.display()).unwrap_or_default();
                let p = eval_args.get(1).map(|v| v.display()).unwrap_or_default();
                return Ok(Value::Bool(s.contains(p.as_str())));
            }
            "starts_with" => {
                let s = eval_args.first().map(|v| v.display()).unwrap_or_default();
                let p = eval_args.get(1).map(|v| v.display()).unwrap_or_default();
                return Ok(Value::Bool(s.starts_with(p.as_str())));
            }
            "ends_with" => {
                let s = eval_args.first().map(|v| v.display()).unwrap_or_default();
                let p = eval_args.get(1).map(|v| v.display()).unwrap_or_default();
                return Ok(Value::Bool(s.ends_with(p.as_str())));
            }
            "replace" => {
                let s    = eval_args.first().map(|v| v.display()).unwrap_or_default();
                let from = eval_args.get(1).map(|v| v.display()).unwrap_or_default();
                let to   = eval_args.get(2).map(|v| v.display()).unwrap_or_default();
                return Ok(Value::Str(s.replace(from.as_str(), &to)));
            }
            "format" => {
                // format("hello {}", name) — simple {} replacement
                let mut template = eval_args.first().map(|v| v.display()).unwrap_or_default();
                for i in 1..eval_args.len() {
                    template = template.replacen("{}", &eval_args[i].display(), 1);
                }
                return Ok(Value::Str(template));
            }
            _ => {}
        }

        // ── User-defined functions ────────────────────────────────────────────
        let fn_decl = self.functions.get(name).cloned()
            .ok_or_else(|| format!("Unknown function '{}'", name))?;

        if fn_decl.params.len() != eval_args.len() {
            return Err(format!(
                "Function '{}' expects {} args, got {}",
                name, fn_decl.params.len(), eval_args.len()
            ));
        }

        let mut fn_env = Env::child(self.global.clone());
        for (param, val) in fn_decl.params.iter().zip(eval_args.drain(..)) {
            fn_env.define(&param.name, val);
        }

        match self.exec_block(&fn_decl.body, &mut fn_env)? {
            Some(Signal::Return(v)) => Ok(v),
            _ => Ok(Value::Nil),
        }
    }
}
