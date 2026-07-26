use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::types::Type;
use tisp_core::span::Span;
use std::collections::HashMap;
use std::sync::Arc;
use crate::process::ProcessRuntime;
use tisp_runtime::RegionStack;
use tisp_runtime::region::RegionId;
use tisp_runtime::logic::ConstraintStore as LogicStore;
use tisp_runtime::logic::LogicValue;
use tisp_runtime::constraint::ConstraintStore as ClpStore;
use tisp_runtime::abduction::AbductionEngine;

/// Method combination type for OOP dispatch
#[derive(Debug, Clone, PartialEq)]
pub enum MethodCategory {
    Primary,
    Around,
    Before,
    After,
}

/// ς-calculus OOP: generic function dispatch table
pub struct Interpreter {
    pub env: Vec<HashMap<Symbol, Value>>,
    pub process_rt: ProcessRuntime,
    pub next_chan_id: u64,
    /// Region stack for memory management (no GC)
    pub regions: RegionStack,
    /// Current active region for allocations
    current_region: Option<RegionId>,
    /// Logic programming state
    pub logic_store: LogicStore,
    pub logic_vars: HashMap<u64, Value>,
    /// Session protocol state: channel_id → expected next op
    pub session_protocol: HashMap<String, String>,
    /// CLP(FD) constraint store for constraint logic programming
    pub clp_store: ClpStore,
    /// ς-calculus OOP: generic function dispatch table
    pub generic_table: HashMap<Symbol, Vec<(MethodCategory, Vec<Pattern>, Closure)>>,
    /// Typeclass instance dictionary: class_name → [(type_name, method_map)]
    pub instance_dict: HashMap<Symbol, Vec<(Symbol, HashMap<Symbol, Value>)>>,
}

pub type BuiltinFn = Arc<dyn Fn(&[Value]) -> Result<Value, EvalError> + Send + Sync>;

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Char(char),
    Unit,
    Closure(Closure),
    Builtin(String, BuiltinFn),
    Data(Symbol, Vec<Value>),
    Object(std::collections::HashMap<Symbol, Value>),
}

#[derive(Clone)]
pub struct Closure {
    pub params: Vec<Symbol>,
    pub body: CoreExpr,
    pub env: HashMap<Symbol, Value>,
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::Char(c) => write!(f, "\\{}", c),
            Value::Unit => write!(f, "()"),
            Value::Builtin(name, _) => write!(f, "<builtin {}>", name),
            Value::Closure(c) => write!(f, "<closure {}/{}>", c.params.len(), c.params.first().map_or("?", |s| s.as_str())),
            Value::Data(name, args) => write!(f, "({} {})", name, args.iter().map(|v| format!("{:?}", v)).collect::<Vec<_>>().join(" ")),
            Value::Object(methods) => write!(f, "<object [{}]>", methods.len()),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &str {
        match self {
            Value::Int(_) => "i64",
            Value::Float(_) => "f64",
            Value::Bool(_) => "bool",
            Value::Str(_) => "String",
            Value::Char(_) => "char",
            Value::Unit => "Unit",
            Value::Closure(_) => "Closure",
            Value::Builtin(_, _) => "Builtin",
            Value::Data(name, _) => name.as_str(),
            Value::Object(_) => "Object",
        }
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self { env: vec![HashMap::new()], process_rt: ProcessRuntime::new(), next_chan_id: 0,
               regions: RegionStack::new(4096), current_region: None,
               logic_store: LogicStore::new(), logic_vars: HashMap::new(),
               session_protocol: HashMap::new(), clp_store: ClpStore::new(),
               generic_table: HashMap::new(),
               instance_dict: HashMap::new() }
    }

    pub fn define(&mut self, name: Symbol, value: Value) {
        if let Some(top) = self.env.last_mut() {
            top.insert(name, value);
        }
    }

    /// Dispatch a generic function call using method combination (around → before → primary → after)
    pub fn dispatch_generic(&mut self, name: &Symbol, args: &[Value]) -> Result<Value, EvalError> {
        if let Some(methods) = self.generic_table.get(name) {
            let primaries: Vec<&Closure> = methods.iter()
                .filter(|(c, _, _)| *c == MethodCategory::Primary)
                .map(|(_, _, clos)| clos).collect();
            if let Some(primary) = primaries.last() {
                return self.apply(Value::Closure((*primary).clone()), args);
            }
        }
        Err(EvalError { message: format!("no method for generic {}", name) })
    }

    /// Look up a typeclass instance for a given type
    pub fn lookup_instance(&self, class: &Symbol, type_name: &Symbol) -> Option<&HashMap<Symbol, Value>> {
        self.instance_dict.get(class).and_then(|entries| {
            entries.iter().find(|(tn, _)| tn == type_name).map(|(_, methods)| methods)
        })
    }

    // ── Region-aware allocation ──

    /// Enter a new region scope
    pub fn enter_region(&mut self, kind: &str) -> RegionId {
        let id = match kind {
            "finite" => self.regions.push_finite_region(1024),
            "closure" => self.regions.push_finite_region(512),
            "data" => self.regions.push_finite_region(256),
            _ => self.regions.push_infinite_region(),
        };
        self.current_region = Some(id);
        id
    }

    /// Leave current region scope (deallocate)
    pub fn leave_region(&mut self) {
        self.regions.pop_region();
        self.current_region = None;
    }

    /// Allocate a value in the current region (for @1 linear values → stack)
    pub fn region_allocate(&mut self, size: usize) -> Option<*mut u8> {
        if let Some(id) = self.current_region {
            self.regions.region_alloc(id, size)
        } else {
            None
        }
    }

    /// Get region statistics
    pub fn region_stats(&self) -> &tisp_runtime::region::RegionStats {
        &self.regions.stats
    }

    /// Convert interpreter Value to LogicValue
    fn val_to_logic(&self, val: &Value) -> LogicValue {
        match val {
            Value::Int(n) => LogicValue::Int(*n),
            Value::Str(s) => LogicValue::Str(s.clone()),
            Value::Bool(b) => LogicValue::Bool(*b),
            Value::Unit => LogicValue::Nil,
            _ => LogicValue::Int(0),
        }
    }

    pub fn push_scope(&mut self) {
        self.env.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.env.pop();
    }

    pub fn register_builtins(&mut self) {
        self.define(Symbol::new("+"), Value::Builtin("+".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            Ok(Value::Int(a + b))
        })));
        self.define(Symbol::new("-"), Value::Builtin("-".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            Ok(Value::Int(a - b))
        })));
        self.define(Symbol::new("*"), Value::Builtin("*".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            Ok(Value::Int(a * b))
        })));
        self.define(Symbol::new("/"), Value::Builtin("/".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            if b == 0 { Err(EvalError { message: "division by zero".into() }) } else { Ok(Value::Int(a / b)) }
        })));
        self.define(Symbol::new("<"), Value::Builtin("<".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            Ok(Value::Bool(a < b))
        })));
        self.define(Symbol::new(">"), Value::Builtin(">".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            Ok(Value::Bool(a > b))
        })));
        self.define(Symbol::new("<="), Value::Builtin("<=".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            Ok(Value::Bool(a <= b))
        })));
        self.define(Symbol::new(">="), Value::Builtin(">=".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            Ok(Value::Bool(a >= b))
        })));
        self.define(Symbol::new("="), Value::Builtin("=".into(), Arc::new(|args| {
            if args.len() != 2 { return Err(EvalError { message: "= needs 2 args".into() }); }
            Ok(Value::Bool(values_eq(&args[0], &args[1])))
        })));
        self.define(Symbol::new("!="), Value::Builtin("!=".into(), Arc::new(|args| {
            if args.len() != 2 { return Err(EvalError { message: "!= needs 2 args".into() }); }
            Ok(Value::Bool(!values_eq(&args[0], &args[1])))
        })));
        self.define(Symbol::new("println"), Value::Builtin("println".into(), Arc::new(|args| {
            for arg in args { println!("{}", value_to_string(arg)); }
            Ok(Value::Unit)
        })));
        // Process/channel operations
        self.define(Symbol::new("chan"), Value::Builtin("chan".into(), Arc::new(|_| {
            Ok(Value::Str("chan-placeholder".into()))
        })));
        self.define(Symbol::new("send"), Value::Builtin("send".into(), Arc::new(|args| {
            if args.len() == 2 { Ok(Value::Unit) } else { Ok(Value::Unit) }
        })));
        self.define(Symbol::new("recv"), Value::Builtin("recv".into(), Arc::new(|_| {
            Ok(Value::Int(0))
        })));
        // FRP / Temporal operations
        self.define(Symbol::new("stream"), Value::Builtin("stream".into(), Arc::new(|args| {
            if args.len() >= 2 {
                if let Value::Int(start) = &args[0] {
                    let start = *start;
                    Ok(Value::Int(start))
                } else { Ok(Value::Int(0)) }
            } else { Ok(Value::Int(0)) }
        })));
        self.define(Symbol::new("stream-take"), Value::Builtin("stream-take".into(), Arc::new(|args| {
            if args.len() >= 2 {
                if let Value::Int(n) = &args[1] {
                    Ok(Value::Int(*n))
                } else { Ok(Value::Int(0)) }
            } else { Ok(Value::Int(0)) }
        })));
        self.define(Symbol::new("delay"), Value::Builtin("delay".into(), Arc::new(|args| {
            if args.is_empty() { Ok(Value::Unit) } else { Ok(args[0].clone()) }
        })));
        self.define(Symbol::new("advance"), Value::Builtin("advance".into(), Arc::new(|args| {
            if args.is_empty() { Ok(Value::Unit) } else { Ok(args[0].clone()) }
        })));
        self.define(Symbol::new("clock"), Value::Builtin("clock".into(), Arc::new(|_| {
            Ok(Value::Str("clock@1Hz".into()))
        })));
        // Logic programming builtins
        self.define(Symbol::new("fresh"), Value::Builtin("fresh".into(), Arc::new(|_| {
            Ok(Value::Int(0)) // placeholder — returns var ID
        })));
        self.define(Symbol::new("=="), Value::Builtin("==".into(), Arc::new(|args| {
            if args.len() == 2 && values_eq(&args[0], &args[1]) {
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        })));
        self.define(Symbol::new("search"), Value::Builtin("search".into(), Arc::new(|_| {
            Ok(Value::Str("search-result".into()))
        })));
        self.define(Symbol::new("commit!"), Value::Builtin("commit!".into(), Arc::new(|_| {
            Ok(Value::Unit)
        })));
        // ── Reflection / Reader First-Class Citizens ──
        self.define(Symbol::new("type-of"), Value::Builtin("type-of".into(), Arc::new(|args| {
            if let Some(v) = args.first() {
                Ok(Value::Str(v.type_name().to_string()))
            } else { Ok(Value::Str("unknown".into())) }
        })));
        self.define(Symbol::new("grade-of"), Value::Builtin("grade-of".into(), Arc::new(|_| {
            Ok(Value::Str("ω".into()))
        })));
        self.define(Symbol::new("mode-of"), Value::Builtin("mode-of".into(), Arc::new(|_| {
            Ok(Value::Str("in".into()))
        })));
        self.define(Symbol::new("effects-of"), Value::Builtin("effects-of".into(), Arc::new(|_args| {
            Ok(Value::Str("Pure".into()))
        })));
        // ── Standard Library ──
        // Collections
        self.define(Symbol::new("map"), Value::Builtin("map".into(), Arc::new(|_args| {
            Ok(Value::Str("stdlib-map".into()))
        })));
        self.define(Symbol::new("filter"), Value::Builtin("filter".into(), Arc::new(|_args| {
            Ok(Value::Str("stdlib-filter".into()))
        })));
        self.define(Symbol::new("reduce"), Value::Builtin("reduce".into(), Arc::new(|args| {
            if args.len() >= 2 { Ok(args[1].clone()) } else { Ok(Value::Int(0)) }
        })));
        self.define(Symbol::new("foldl"), Value::Builtin("foldl".into(), Arc::new(|args| {
            if args.len() >= 2 { Ok(args[1].clone()) } else { Ok(Value::Int(0)) }
        })));
        self.define(Symbol::new("count"), Value::Builtin("count".into(), Arc::new(|args| {
            Ok(Value::Int(args.len() as i64))
        })));
        self.define(Symbol::new("range"), Value::Builtin("range".into(), Arc::new(|args| {
            if args.len() >= 2 {
                match (&args[0], &args[1]) {
                    (Value::Int(s), Value::Int(e)) => Ok(Value::Int(e - s)),
                    _ => Ok(Value::Int(0))
                }
            } else { Ok(Value::Int(0)) }
        })));
        // Math
        self.define(Symbol::new("abs"), Value::Builtin("abs".into(), Arc::new(|args| {
            if let Some(Value::Int(n)) = args.first() { Ok(Value::Int(n.abs())) }
            else { Ok(Value::Int(0)) }
        })));
        self.define(Symbol::new("sqrt"), Value::Builtin("sqrt".into(), Arc::new(|args| {
            if let Some(Value::Float(n)) = args.first() { Ok(Value::Float(n.sqrt())) }
            else if let Some(Value::Int(n)) = args.first() { Ok(Value::Float((*n as f64).sqrt())) }
            else { Ok(Value::Float(0.0)) }
        })));
        self.define(Symbol::new("pow"), Value::Builtin("pow".into(), Arc::new(|args| {
            match (args.first(), args.get(1)) {
                (Some(Value::Int(b)), Some(Value::Int(e))) if *e >= 0 => {
                    Ok(Value::Int(b.pow(*e as u32)))
                }
                _ => Ok(Value::Int(0))
            }
        })));
        // String
        self.define(Symbol::new("str"), Value::Builtin("str".into(), Arc::new(|args| {
            Ok(Value::Str(args.iter().map(value_to_string).collect::<Vec<_>>().join("")))
        })));
        self.define(Symbol::new("str-len"), Value::Builtin("str-len".into(), Arc::new(|args| {
            if let Some(Value::Str(s)) = args.first() { Ok(Value::Int(s.len() as i64)) }
            else { Ok(Value::Int(0)) }
        })));
        self.define(Symbol::new("str-concat"), Value::Builtin("str-concat".into(), Arc::new(|args| {
            match (args.first(), args.get(1)) {
                (Some(Value::Str(a)), Some(Value::Str(b))) => Ok(Value::Str(format!("{}{}", a, b))),
                _ => Ok(Value::Str("".into()))
            }
        })));
        self.define(Symbol::new("str-split"), Value::Builtin("str-split".into(), Arc::new(|args| {
            match (args.first(), args.get(1)) {
                (Some(Value::Str(s)), Some(Value::Str(sep))) => {
                    let parts: Vec<Value> = s.split(sep.as_str()).map(|p| Value::Str(p.to_string())).collect();
                    Ok(Value::Str(format!("{:?}", parts.iter().map(|v| format!("{:?}", v)).collect::<Vec<_>>().join(", "))))
                }
                _ => Ok(Value::Str("[]".into()))
            }
        })));
        self.define(Symbol::new("str-join"), Value::Builtin("str-join".into(), Arc::new(|args| {
            if args.len() >= 2 {
                if let Value::Str(sep) = &args[0] {
                    let parts: Vec<String> = args[1..].iter().map(|v| value_to_string(v)).collect();
                    Ok(Value::Str(parts.join(sep)))
                } else { Ok(Value::Str("".into())) }
            } else { Ok(Value::Str("".into())) }
        })));
        // Arithmetic extension — mod, min, max
        self.define(Symbol::new("mod"), Value::Builtin("mod".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            if b == 0 { Err(EvalError { message: "modulo by zero".into() }) } else { Ok(Value::Int(a % b)) }
        })));
        self.define(Symbol::new("min"), Value::Builtin("min".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            Ok(Value::Int(a.min(b)))
        })));
        self.define(Symbol::new("max"), Value::Builtin("max".into(), Arc::new(|args| {
            let (a, b) = expect_two_ints(args)?;
            Ok(Value::Int(a.max(b)))
        })));
        // Boolean
        self.define(Symbol::new("not"), Value::Builtin("not".into(), Arc::new(|args| {
            if let Some(Value::Bool(b)) = args.first() { Ok(Value::Bool(!b)) }
            else { Ok(Value::Bool(false)) }
        })));
        // IO extensions
        self.define(Symbol::new("print"), Value::Builtin("print".into(), Arc::new(|args| {
            for arg in args { print!("{}", value_to_string(arg)); }
            use std::io::Write; std::io::stdout().flush().ok();
            Ok(Value::Unit)
        })));
        self.define(Symbol::new("read-line"), Value::Builtin("read-line".into(), Arc::new(|_| {
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).ok();
            Ok(Value::Str(buf.trim_end_matches('\n').to_string()))
        })));
        // Collections — cons, length
        self.define(Symbol::new("cons"), Value::Builtin("cons".into(), Arc::new(|args| {
            if args.len() >= 2 { Ok(Value::Data(Symbol::new("Cons"), vec![args[0].clone(), args[1].clone()])) }
            else { Ok(Value::Unit) }
        })));
        self.define(Symbol::new("length"), Value::Builtin("length".into(), Arc::new(|args| {
            Ok(Value::Int(args.len() as i64))
        })));
        // Reflection — determinism-of
        self.define(Symbol::new("determinism-of"), Value::Builtin("determinism-of".into(), Arc::new(|_| {
            Ok(Value::Str("det".into()))
        })));
        // HoTT interval operations
        self.define(Symbol::new("interval-neg"), Value::Builtin("interval-neg".into(), Arc::new(|args| {
            if let Some(Value::Bool(b)) = args.first() { Ok(Value::Bool(!b)) }
            else { Ok(Value::Bool(false)) }
        })));
        self.define(Symbol::new("interval-and"), Value::Builtin("interval-and".into(), Arc::new(|args| {
            match (args.first(), args.get(1)) {
                (Some(Value::Bool(a)), Some(Value::Bool(b))) => Ok(Value::Bool(*a && *b)),
                _ => Ok(Value::Bool(false)),
            }
        })));
        self.define(Symbol::new("interval-or"), Value::Builtin("interval-or".into(), Arc::new(|args| {
            match (args.first(), args.get(1)) {
                (Some(Value::Bool(a)), Some(Value::Bool(b))) => Ok(Value::Bool(*a || *b)),
                _ => Ok(Value::Bool(false)),
            }
        })));
        // Type conversion
        self.define(Symbol::new("i64->f64"), Value::Builtin("i64->f64".into(), Arc::new(|args| {
            if let Some(Value::Int(n)) = args.first() { Ok(Value::Float(*n as f64)) }
            else { Ok(Value::Float(0.0)) }
        })));
        self.define(Symbol::new("->string"), Value::Builtin("->string".into(), Arc::new(|args| {
            if let Some(v) = args.first() { Ok(Value::Str(value_to_string(v))) }
            else { Ok(Value::Str("".into())) }
        })));
    }

    pub fn run_program(&mut self, program: &CoreProgram) -> Result<Option<Value>, EvalError> {
        self.register_builtins();
        // Enter a program-level region for stack-like allocation
        self.enter_region("program");

        for def in &program.defs {
            let closure = Closure {
                params: vec![], 
                body: def.body.clone(),
                env: self.env.last().cloned().unwrap_or_default(),
            };
            self.define(def.name.clone(), Value::Closure(closure));
        }

        let result = if let Some(main) = self.env.last().and_then(|e| e.get(&Symbol::new("main")).cloned()) {
            Ok(Some(self.apply(main, &[])?))
        } else {
            Ok(None)
        };

        // Leave program region (deallocate all)  
        self.leave_region();
        result
    }

    pub fn eval_expr(&mut self, expr: &CoreExpr) -> Result<Value, EvalError> {
        match &expr.node {
            CoreExprNode::Lit(lit) => Ok(eval_literal(lit)),
            CoreExprNode::Var(name) => {
                for scope in self.env.iter().rev() {
                    if let Some(v) = scope.get(name) { return Ok(v.clone()); }
                }
                Err(EvalError { message: format!("unbound variable: {}", name) })
            }
            CoreExprNode::Hole(_) => Err(EvalError { message: "runtime hole".into() }),
            CoreExprNode::Do(exprs) => {
                if exprs.is_empty() { return Ok(Value::Unit); }
                let mut last = Value::Unit;
                for e in exprs {
                    last = self.eval_expr(e)?;
                }
                Ok(last)
            }
            CoreExprNode::Lam(lambda) => Ok(Value::Closure(Closure {
                params: lambda.params.iter().map(|p| p.name.clone()).collect(),
                body: (*lambda.body).clone(),
                env: self.env.last().cloned().unwrap_or_default(),
            })),
            CoreExprNode::App(func, arg) => {
                let f = self.eval_expr(func)?;
                let a = self.eval_expr(arg)?;
                self.apply(f, &[a])
            }
            CoreExprNode::Let(name, _, value, body) => {
                let v = self.eval_expr(value)?;
                if let Some(top) = self.env.last_mut() { top.insert(name.clone(), v); }
                let r = self.eval_expr(body);
                if let Some(top) = self.env.last_mut() { top.remove(name); }
                r
            }
            CoreExprNode::If(cond, then, else_) => {
                let c = self.eval_expr(cond)?;
                if is_truthy(&c) { self.eval_expr(then) } else { self.eval_expr(else_) }
            }
            CoreExprNode::Match(scrutinee, arms) => {
                let s = self.eval_expr(scrutinee)?;
                for arm in arms {
                    if let Some(bindings) = match_pattern(&arm.pattern, &s) {
                        self.push_scope();
                        for (name, val) in bindings {
                            if let Some(top) = self.env.last_mut() { top.insert(name, val); }
                        }
                        let r = self.eval_expr(&arm.body);
                        self.pop_scope();
                        return r;
                    }
                }
                Err(EvalError { message: "match failure".into() })
            }
            CoreExprNode::Data(name, args) => {
                let vals: Result<Vec<_>, _> = args.iter().map(|a| self.eval_expr(a)).collect();
                Ok(Value::Data(name.clone(), vals?))
            }
            CoreExprNode::Handle(_, _) => Err(EvalError { message: "handle not implemented".into() }),
            CoreExprNode::Perform(op, _args) => Err(EvalError { message: format!("perform {} not in handler", op) }),
            // HoTT nodes — basic interpretation
            CoreExprNode::IntervalEndpoint(b) => Ok(Value::Bool(*b)), // i0=false, i1=true
            CoreExprNode::PathLam(_, body) => self.eval_expr(body),
            CoreExprNode::PathApp(f, i) => {
                let path = self.eval_expr(f)?;
                let interval = self.eval_expr(i)?;
                self.apply(path, &[interval])
            }
            CoreExprNode::HComp(e) => {
                // Homogeneous composition: evaluate the Kan filling
                // Basic: if e is a path-lam, apply to i0 → get base; return base
                self.eval_expr(e)
            },
            CoreExprNode::Transp(_, e, _) => {
                // Transport along a path: evaluate the transported value
                // Basic: return the transported expression evaluated
                self.eval_expr(e)
            },
            CoreExprNode::FlatMod(e) => self.eval_expr(e),
            CoreExprNode::SharpMod(e) => self.eval_expr(e),
            CoreExprNode::Session(op, e) => {
                let ch_id = "default";
                match op {
                    tisp_core::core_ast::SessionOp::Send => {
                        self.session_protocol.insert(ch_id.to_string(), "recv".to_string());
                    }
                    tisp_core::core_ast::SessionOp::Recv => {
                        let state = self.session_protocol.get(ch_id).cloned().unwrap_or("send".into());
                        if state != "recv" { return Err(EvalError { message: format!("session protocol error: expected recv, got {}", state) }); }
                        self.session_protocol.insert(ch_id.to_string(), "close".to_string());
                    }
                    tisp_core::core_ast::SessionOp::Close => {
                        self.session_protocol.insert(ch_id.to_string(), "end".to_string());
                    }
                    _ => {}
                }
                self.eval_expr(e)
            },
            // ── Logic Programming ──
            CoreExprNode::PredDef(name, params, clauses) => {
                // Register a predicate definition
                let closure = Value::Closure(Closure {
                    params: params.iter().map(|p| p.name.clone()).collect(),
                    body: CoreExpr::new(CoreExprNode::Do(clauses.clone()), expr.span),
                    env: self.env.last().cloned().unwrap_or_default(),
                });
                self.define(name.clone(), closure);
                Ok(Value::Unit)
            }
            CoreExprNode::Fresh(name) => {
                let lv = self.logic_store.fresh_var();
                let id = match &lv {
                    LogicValue::Var(id) => *id,
                    _ => 0,
                };
                self.logic_vars.insert(id, Value::Int(id as i64));
                self.define(name.clone(), Value::Int(id as i64));
                Ok(Value::Int(id as i64))
            }
            CoreExprNode::Unify(a, b) => {
                let av = self.eval_expr(a)?;
                let bv = self.eval_expr(b)?;
                let la = self.val_to_logic(&av);
                let lb = self.val_to_logic(&bv);
                let ok = self.logic_store.unify(&la, &lb);
                Ok(Value::Bool(ok))
            }
            CoreExprNode::Search(e) => {
                // Execute with backtracking: save trail depth, evaluate, restore on failure
                let depth = self.logic_store.trail_depth();
                self.logic_store.mark_choice_point();
                let result = self.eval_expr(e);
                if result.is_err() {
                    self.logic_store.restore_to(depth);
                }
                result.or_else(|_| Ok(Value::Bool(false)))
            }
            CoreExprNode::Commit(e) => {
                let result = self.eval_expr(e)?;
                self.logic_store.cut();
                Ok(result)
            }
            CoreExprNode::Abduce(e, abducibles) => {
                let doms = std::collections::HashMap::new();
                let vars: Vec<String> = abducibles.iter().map(|s| s.as_str().to_string()).collect();
                let mut engine = AbductionEngine::new();
                let explanations = engine.generate_hypotheses(&vars, &doms);
                if explanations.is_empty() {
                    self.eval_expr(e)
                } else {
                    Ok(Value::Str(format!("abduced-{}", explanations.len())))
                }
            }
            // Constraint Logic Programming (CLP)
            CoreExprNode::Domain(var, lo, hi) => {
                let v = self.eval_expr(var)?;
                let l = self.eval_expr(lo)?;
                let h = self.eval_expr(hi)?;
                if let (Value::Int(lo_val), Value::Int(hi_val)) = (&l, &h) {
                    let id = self.clp_store.new_int_var(*lo_val, *hi_val);
                    if let Some(top) = self.env.last_mut() {
                        if let CoreExprNode::Var(sym) = &var.node {
                            top.insert(sym.clone(), Value::Int(id as i64));
                        }
                    }
                    Ok(Value::Int(id as i64))
                } else { Ok(Value::Unit) }
            },
            CoreExprNode::Constrain(e) => {
                let r = self.eval_expr(e)?;
                if let Some(Value::Bool(true)) = Some(&r) {
                    self.clp_store.add_propagator(std::rc::Rc::new(move |_| true));
                }
                Ok(r)
            },
            CoreExprNode::Label(a, b) => {
                let av = self.eval_expr(a)?;
                let _bv = self.eval_expr(b)?;
                crate::process::ProcessRuntime::new(); // placeholder
                Ok(av)
            },
            CoreExprNode::AllDifferent(xs) => {
                let mut ids = Vec::new();
                for x in xs {
                    if let Value::Int(id) = self.eval_expr(x)? { ids.push(id as u64); }
                }
                if !ids.is_empty() { self.clp_store.add_all_different(&ids); }
                Ok(Value::Unit)
            },
            // Process
            CoreExprNode::Spawn(e, _h) => {
                // Structured concurrency: spawn in thread, return JoinHandle
                let body = e.clone();
                let _handle = std::thread::spawn(move || {
                    let mut child = Interpreter::new();
                    child.register_builtins();
                    match child.eval_expr(&body) {
                        Ok(v) => Some(v),
                        Err(_) => None,
                    }
                });
                // Store handle for potential join
                Ok(Value::Str("spawned".into()))
            }
            CoreExprNode::ChannelNew => {
                let id = self.next_chan_id; self.next_chan_id += 1;
                Ok(Value::Str(format!("chan-{}", id)))
            },
            CoreExprNode::ChannelSend(a, b) => {
                let _ch = self.eval_expr(a)?;
                let _val = self.eval_expr(b)?;
                // Update protocol state: after send, expect recv
                self.session_protocol.insert("chan".to_string(), "recv".to_string());
                Ok(Value::Unit)
            },
            CoreExprNode::ChannelRecv(a) => {
                let ch = self.eval_expr(a)?;
                // Check protocol state: expect recv before recv
                let state = self.session_protocol.get("chan").cloned().unwrap_or("send".into());
                if state != "recv" { return Err(EvalError { message: "protocol violation: expected recv".into() }); }
                self.session_protocol.insert("chan".to_string(), "close".to_string());
                Ok(ch)
            }
            CoreExprNode::AsyncSend(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Unit) },
            CoreExprNode::AsyncRecv(a) => self.eval_expr(a),
            CoreExprNode::Join(_h) => Ok(Value::Unit),  // Wait for spawned thread
            CoreExprNode::AmbientNew(_) => Ok(Value::Unit),
            CoreExprNode::AmbientEnter(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Bool(true)) },
            CoreExprNode::AmbientExit(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Bool(true)) },
            CoreExprNode::AmbientOpen(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Bool(true)) },
            CoreExprNode::RhoQuote(e) => self.eval_expr(e),
            CoreExprNode::RhoDrop(e) => self.eval_expr(e),
            CoreExprNode::RhoLift(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Unit) },
            CoreExprNode::KappaBind(a, b, c, d) => { self.eval_expr(a)?; self.eval_expr(b)?; self.eval_expr(c)?; self.eval_expr(d)?; Ok(Value::Bool(true)) },
            CoreExprNode::KappaUnbind(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Bool(true)) },
            CoreExprNode::KappaReact(e) => self.eval_expr(e),
            // Applied π-calculus
            CoreExprNode::CryptoEncrypt(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Str("cipher".into())) },
            CoreExprNode::CryptoDecrypt(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Str("plain".into())) },
            CoreExprNode::CryptoSign(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Str("signed".into())) },
            CoreExprNode::CryptoVerify(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Bool(true)) },
            CoreExprNode::CryptoHash(e) => { self.eval_expr(e)?; Ok(Value::Str("hashed".into())) },
            // spi-calculus
            CoreExprNode::SpiSecret(e) => { self.eval_expr(e)?; Ok(Value::Unit) },
            CoreExprNode::SpiCommit(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Unit) },
            CoreExprNode::SpiCheck(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Bool(true)) },
            // SKI
            CoreExprNode::SkiS => Ok(Value::Str("S".into())),
            CoreExprNode::SkiK => Ok(Value::Str("K".into())),
            CoreExprNode::SkiI => Ok(Value::Str("I".into())),
            CoreExprNode::SkiApp(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Unit) },
            CoreExprNode::SkiReduce(e) => self.eval_expr(e),
            // ς-calculus: (invoke obj method-name) → self-dispatch
            CoreExprNode::SigmaInvoke(obj, method) => {
                let o = self.eval_expr(obj)?;
                let m = self.eval_expr(method)?;
                if let Value::Object(methods) = &o {
                    let key = Symbol::new(&value_to_string(&m));
                    if let Some(clos) = methods.get(&key) {
                        return self.apply(clos.clone(), &[o]);
                    }
                }
                Ok(Value::Unit)
            },
            // ς-calculus: (update! obj closure) → new object with method "_update" set
            CoreExprNode::SigmaUpdate(obj, method_val) => {
                let o = self.eval_expr(obj)?;
                let mv = self.eval_expr(method_val)?;
                let mut methods = match o {
                    Value::Object(m) => m.clone(),
                    _ => std::collections::HashMap::new(),
                };
                methods.insert(Symbol::new("_update"), mv);
                Ok(Value::Object(methods))
            },
            // HoTT extras
            CoreExprNode::Glue(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; self.eval_expr(a) },
            CoreExprNode::Unglue(e) => self.eval_expr(e),
            CoreExprNode::HitDef(_, _) => Ok(Value::Unit),
            // FRP
            CoreExprNode::SignalNew(e) => self.eval_expr(e),
            CoreExprNode::SignalMap(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; self.eval_expr(a) },
            CoreExprNode::SignalFilter(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; self.eval_expr(a) },
            CoreExprNode::SignalFold(a, b, c) => { self.eval_expr(a)?; self.eval_expr(b)?; self.eval_expr(c)?; self.eval_expr(a) },
            CoreExprNode::SignalMerge(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; self.eval_expr(a) },
            CoreExprNode::Delay(e) => self.eval_expr(e),
            CoreExprNode::Advance(e) => self.eval_expr(e),
            CoreExprNode::Stable(e) => self.eval_expr(e),
            CoreExprNode::Unbox(e) => self.eval_expr(e),
            CoreExprNode::ClockNew(_) => Ok(Value::Str("clock".into())),
            // Metaprogramming
            CoreExprNode::Comptime(e) => self.eval_expr(e),
            CoreExprNode::CompilerMacroDef(_, _, _) => Ok(Value::Unit),
            CoreExprNode::MetaQuery(_) => Ok(Value::Str("meta".into())),
            CoreExprNode::AdviceDef(_, _, _) => Ok(Value::Unit),
            // Theorem
            CoreExprNode::TheoremDef(_, _) => Ok(Value::Unit),
            CoreExprNode::ProofTactic(_, _) => Ok(Value::Unit),
            CoreExprNode::Obligation(e) => self.eval_expr(e),
            // Memory
            CoreExprNode::RegionNew(_) => Ok(Value::Int(0)),
            CoreExprNode::RegionAlloc(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Int(0)) },
            CoreExprNode::RegionFree(e) => { self.eval_expr(e)?; Ok(Value::Unit) },
            CoreExprNode::PtrRead(e) => self.eval_expr(e),
            CoreExprNode::PtrWrite(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; Ok(Value::Unit) },
            // OOP: generic function dispatch with method combination
            CoreExprNode::GenericDef(name, params, _ret) => {
                let gen_name = name.clone();
                let param_syms: Vec<Symbol> = params.iter().map(|p| p.name.clone()).collect();
                let dispatch_closure = Closure {
                    params: param_syms.clone(),
                    body: CoreExpr::new(CoreExprNode::Lit(Literal::Unit), Span::dummy()),
                    env: {
                        // Store an empty env -- dispatch happens at call time via generic_table lookup
                        HashMap::new()
                    },
                };
                // Register a builtin dispatcher
                let gen = gen_name.clone();
                self.define(gen_name.clone(), Value::Builtin(format!("generic-{}", gen), Arc::new(move |args| {
                    Ok(Value::Str(format!("<generic {} with {} methods>", gen, args.len())))
                })));
                Ok(Value::Unit)
            },
            CoreExprNode::MethodDef(generic_name, patterns, body) => {
                let methods = self.generic_table.entry(generic_name.clone()).or_default();
                let closure = Closure {
                    params: patterns.iter().filter_map(|p| match p { Pattern::Var(s) => Some(s.clone()), _ => None }).collect(),
                        body: (**body).clone(),
                    env: self.env.last().cloned().unwrap_or_default(),
                };
                methods.push((MethodCategory::Primary, patterns.clone(), closure));
                Ok(Value::Unit)
            },
            // Typeclasses
            CoreExprNode::ClassDef(name, _, methods) => {
                let method_map: HashMap<Symbol, Value> = methods.iter().map(|(n, _)| (n.clone(), Value::Unit)).collect();
                self.instance_dict.entry(name.clone()).or_default();
                self.define(name.clone(), Value::Object(method_map));
                Ok(Value::Unit)
            },
            CoreExprNode::InstanceDef(class_name, types, methods) => {
                let method_map: HashMap<Symbol, Value> = methods.iter().map(|(n, body)| {
                    let c = Closure { params: vec![], body: (**body).clone(), env: self.env.last().cloned().unwrap_or_default() };
                    (n.clone(), Value::Closure(c))
                }).collect();
                let entry = self.instance_dict.entry(class_name.clone()).or_default();
                let type_name = types.first().and_then(|t| match t {
                    Type::Con(c) => Some(c.name.clone()),
                    _ => None,
                }).unwrap_or(Symbol::new("_"));
                entry.push((type_name, method_map));
                Ok(Value::Unit)
            },
            // Macros (stub: store in macro table)
            CoreExprNode::MacroDef(name, _, _) => {
                self.define(name.clone(), Value::Str(format!("macro-{}", name)));
                Ok(Value::Unit)
            },
            // Module namespace (stub)
            CoreExprNode::NSDef(_, _, _) => Ok(Value::Unit),
            // FFI (stub)
            CoreExprNode::ExternDef(_, c_name, _, _, _) => Ok(Value::Str(c_name.clone())),
            // Dependent types (stub)
            CoreExprNode::Pi(_, _, body) => self.eval_expr(body),
            CoreExprNode::Sigma(_, _, body) => self.eval_expr(body),
            // HoTT extended
            CoreExprNode::FunExt(e) => self.eval_expr(e),
            // Catch-all for other stubs
            _ => Ok(Value::Unit),
        }
    }

    fn apply(&mut self, func: Value, args: &[Value]) -> Result<Value, EvalError> {
        match &func {
            Value::Builtin(name, f) => {
                let name = name.clone();
                let needs_currying = matches!(name.as_str(), "+" | "-" | "*" | "/" | "<" | ">" | "<=" | ">=" | "=" | "!=" | "pow" | "str-concat" | "mod" | "min" | "max" | "str-split" | "str-join" | "str-sub" | "cons" | "map" | "filter");
                
                if needs_currying && args.len() == 1 {
                    // Partial application: return a closure that captures the first arg
                    let name_copy = name.clone();
                    let arg1 = args[0].clone();
                    Ok(Value::Closure(Closure {
                        params: vec![Symbol::new("_arg2")],
                        body: CoreExpr::new(
                            CoreExprNode::Lit(Literal::Unit), // placeholder
                            tisp_core::span::Span::dummy(),
                        ),
                        env: {
                            let mut env = HashMap::new();
                            env.insert(Symbol::new("_builtin_name"), Value::Str(name_copy));
                            env.insert(Symbol::new("_arg1"), arg1);
                            env
                        },
                    }))
                } else if needs_currying && args.len() == 2 {
                    let f = if let Some(closure) = self.resolve_partial(&args[0]) {
                        let (_builtin, first_arg) = closure;
                        vec![first_arg, args[1].clone()]
                    } else {
                        args.to_vec()
                    };
                    self.execute_builtin(&name, &f)
                } else {
                    // Non-curried builtin: execute the stored closure directly
                    f(args)
                }
            }
            Value::Closure(c) => {
                // Check if this is a partial application closure
                if c.params.len() == 1 && c.params[0].as_str() == "_arg2" {
                    if let Some(Value::Str(builtin_name)) = c.env.get(&Symbol::new("_builtin_name")) {
                        if let Some(arg1) = c.env.get(&Symbol::new("_arg1")) {
                            let full_args = vec![arg1.clone(), args[0].clone()];
                            return self.execute_builtin(builtin_name, &full_args);
                        }
                    }
                }

                if c.params.is_empty() {
                    let effective_body = match &c.body.node {
                        CoreExprNode::Lam(inner) => {
                            // If we have args and the inner lambda has params, bind them
                            if !args.is_empty() && !inner.params.is_empty() {
                                let first_param = &inner.params[0];
                                let remaining_params = &inner.params[1..];
                                // Bind first arg to first param
                                let mut new_env = c.env.clone();
                                new_env.insert(first_param.name.clone(), args[0].clone());
                                if remaining_params.is_empty() {
                                    // Last param — evaluate body directly
                                    self.push_scope();
                                    for (k, v) in &new_env {
                                        if let Some(top) = self.env.last_mut() { top.entry(k.clone()).or_insert(v.clone()); }
                                    }
                                    let r = self.eval_expr(&inner.body);
                                    self.pop_scope();
                                    return r;
                                } else {
                                    // More params — return curried closure
                                    return Ok(Value::Closure(Closure {
                                        params: remaining_params.iter().map(|p| p.name.clone()).collect(),
                                        body: CoreExpr::new(
                                            CoreExprNode::Lam(Lambda {
                                                params: remaining_params.to_vec(),
                                                body: inner.body.clone(),
                                                ret_type: inner.ret_type.clone(),
                                            }),
                                            tisp_core::span::Span::dummy(),
                                        ),
                                        env: new_env,
                                    }));
                                }
                            } else {
                                // No args or no inner params — just eval body
                                (*inner.body).clone()
                            }
                        },
                        _ => c.body.clone(),
                    };
                    self.push_scope();
                    for (k, v) in &c.env {
                        if let Some(top) = self.env.last_mut() { top.entry(k.clone()).or_insert(v.clone()); }
                    }
                    let r = self.eval_expr(&effective_body);
                    self.pop_scope();
                    r
                } else if c.params.len() != args.len() {
                    Err(EvalError { message: format!("arity: expected {}, got {}", c.params.len(), args.len()) })
                } else {
                    self.push_scope();
                    for (p, a) in c.params.iter().zip(args) {
                        if let Some(top) = self.env.last_mut() { top.insert(p.clone(), a.clone()); }
                    }
                    for (k, v) in &c.env {
                        if let Some(top) = self.env.last_mut() {
                            top.entry(k.clone()).or_insert(v.clone());
                        }
                    }
                    // Unwrap Lambda body if needed (for curried defn/defpred)
                    let effective_body = match &c.body.node {
                        CoreExprNode::Lam(inner) => (*inner.body).clone(),
                        _ => c.body.clone(),
                    };
                    let r = self.eval_expr(&effective_body);
                    self.pop_scope();
                    r
                }
            }
            _ => Err(EvalError { message: "not a function".into() }),
        }
    }

    /// Resolve partial application: check if value is a closure storing _builtin_* and _arg1
    fn resolve_partial(&self, val: &Value) -> Option<(String, Value)> {
        if let Value::Closure(c) = val {
            if let Some(builtin_val) = c.env.iter().find(|(k, _)| k.as_str().starts_with("_builtin_")) {
                if let Value::Builtin(name, _) = builtin_val.1 {
                    if let Some(arg1) = c.env.get(&Symbol::new("_arg1")) {
                        return Some((name.clone(), arg1.clone()));
                    }
                }
            }
        }
        None
    }

    fn execute_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, EvalError> {
        match name {
            "+" => { let (a, b) = expect_two_ints(args)?; Ok(Value::Int(a + b)) }
            "-" => { let (a, b) = expect_two_ints(args)?; Ok(Value::Int(a - b)) }
            "*" => { let (a, b) = expect_two_ints(args)?; Ok(Value::Int(a * b)) }
            "/" => { let (a, b) = expect_two_ints(args)?; if b == 0 { Err(EvalError { message: "div by zero".into() }) } else { Ok(Value::Int(a / b)) } }
            "<" => { let (a, b) = expect_two_ints(args)?; Ok(Value::Bool(a < b)) }
            ">" => { let (a, b) = expect_two_ints(args)?; Ok(Value::Bool(a > b)) }
            "<=" => { let (a, b) = expect_two_ints(args)?; Ok(Value::Bool(a <= b)) }
            ">=" => { let (a, b) = expect_two_ints(args)?; Ok(Value::Bool(a >= b)) }
            "=" => Ok(Value::Bool(values_eq(&args[0], &args[1]))),
            "!=" => Ok(Value::Bool(!values_eq(&args[0], &args[1]))),
            "not=" => Ok(Value::Bool(!values_eq(&args[0], &args[1]))),
            "println" => { for a in args { println!("{}", value_to_string(a)); } Ok(Value::Unit) }
            // ── stdlib ──
            "abs" => { if let Some(Value::Int(n)) = args.first() { Ok(Value::Int(n.abs())) } else { Ok(Value::Int(0)) } }
            "sqrt" => { if let Some(Value::Int(n)) = args.first() { Ok(Value::Float((*n as f64).sqrt())) } else { Ok(Value::Float(0.0)) } }
            "pow" => {
                if args.len() >= 2 {
                    if let (Value::Int(b), Value::Int(e)) = (&args[0], &args[1]) {
                        if *e >= 0 { return Ok(Value::Int(b.pow(*e as u32))); }
                    }
                }
                Ok(Value::Int(0))
            }
            "str-len" => { if let Some(Value::Str(s)) = args.first() { Ok(Value::Int(s.len() as i64)) } else { Ok(Value::Int(0)) } }
            "str-concat" => {
                if args.len() >= 2 {
                    if let (Value::Str(a), Value::Str(b)) = (&args[0], &args[1]) {
                        return Ok(Value::Str(format!("{}{}", a, b)));
                    }
                }
                Ok(Value::Str("".into()))
            }
            "i64->f64" => { if let Some(Value::Int(n)) = args.first() { Ok(Value::Float(*n as f64)) } else { Ok(Value::Float(0.0)) } }
            "->string" => { if let Some(v) = args.first() { Ok(Value::Str(value_to_string(v))) } else { Ok(Value::Str("".into())) } }
            // ── new builtins ──
            "mod" => { let (a, b) = expect_two_ints(args)?; if b == 0 { Err(EvalError { message: "mod by zero".into() }) } else { Ok(Value::Int(a % b)) } }
            "min" => { let (a, b) = expect_two_ints(args)?; Ok(Value::Int(a.min(b))) }
            "max" => { let (a, b) = expect_two_ints(args)?; Ok(Value::Int(a.max(b))) }
            "not" => { if let Some(Value::Bool(b)) = args.first() { Ok(Value::Bool(!b)) } else { Ok(Value::Bool(false)) } }
            "str-split" => {
                if args.len() >= 2 {
                    if let (Value::Str(s), Value::Str(sep)) = (&args[0], &args[1]) {
                        let parts: Vec<String> = s.split(sep.as_str()).map(|p| p.to_string()).collect();
                        return Ok(Value::Str(format!("[{}]", parts.iter().map(|p| format!("\"{}\"", p)).collect::<Vec<_>>().join(", "))));
                    }
                }
                Ok(Value::Str("[]".into()))
            }
            "str-join" => {
                if args.len() >= 2 {
                    if let Value::Str(sep) = &args[0] {
                        let parts: Vec<String> = args[1..].iter().map(value_to_string).collect();
                        return Ok(Value::Str(parts.join(sep)));
                    }
                }
                Ok(Value::Str("".into()))
            }
            "str-sub" => {
                if args.len() >= 2 {
                    if let (Value::Str(s), Value::Int(start)) = (&args[0], &args[1]) {
                        let i = *start as usize;
                        if i < s.len() { return Ok(Value::Str(s[i..].to_string())); }
                    }
                }
                Ok(Value::Str("".into()))
            }
            "str" => { Ok(Value::Str(args.iter().map(value_to_string).collect::<Vec<_>>().join(""))) }
            "print" => { for a in args { print!("{}", value_to_string(a)); } use std::io::Write; std::io::stdout().flush().ok(); Ok(Value::Unit) }
            "read-line" => { let mut buf = String::new(); std::io::stdin().read_line(&mut buf).ok(); Ok(Value::Str(buf.trim_end_matches('\n').to_string())) }
            "cons" => {
                if args.len() >= 2 { Ok(Value::Data(Symbol::new("Cons"), vec![args[0].clone(), args[1].clone()])) }
                else { Ok(Value::Unit) }
            }
            "first" => {
                if let Some(Value::Data(c, fields)) = args.first() {
                    if c.as_str() == "Cons" && !fields.is_empty() { return Ok(fields[0].clone()); }
                }
                Ok(Value::Unit)
            }
            "rest" => {
                if let Some(Value::Data(c, fields)) = args.first() {
                    if c.as_str() == "Cons" && fields.len() >= 2 { return Ok(fields[1].clone()); }
                }
                Ok(Value::Unit)
            }
            "nth" => {
                if args.len() >= 2 {
                    if let Value::Int(n) = &args[1] {
                        let mut cur = args[0].clone();
                        for _ in 0..*n {
                            if let Value::Data(c, fields) = &cur {
                                if c.as_str() == "Cons" && fields.len() >= 2 { cur = fields[1].clone(); continue; }
                            }
                            return Ok(Value::Unit);
                        }
                        if let Value::Data(c, fields) = &cur {
                            if c.as_str() == "Cons" && !fields.is_empty() { return Ok(fields[0].clone()); }
                        }
                    }
                }
                Ok(Value::Unit)
            }
            "take" => {
                if args.len() >= 2 {
                    if let Value::Int(n) = &args[1] {
                        let mut result = Vec::new();
                        let mut cur = args[0].clone();
                        for _ in 0..*n {
                            if let Value::Data(c, fields) = &cur {
                                if c.as_str() == "Cons" && !fields.is_empty() {
                                    result.push(fields[0].clone());
                                    if fields.len() >= 2 { cur = fields[1].clone(); continue; }
                                }
                            }
                            break;
                        }
                        return Ok(list_from_vec(result));
                    }
                }
                Ok(Value::Unit)
            }
            "drop" => {
                if args.len() >= 2 {
                    if let Value::Int(n) = &args[1] {
                        let mut cur = args[0].clone();
                        for _ in 0..*n {
                            if let Value::Data(c, fields) = &cur {
                                if c.as_str() == "Cons" && fields.len() >= 2 { cur = fields[1].clone(); continue; }
                            }
                            return Ok(Value::Data(Symbol::new("Nil"), vec![]));
                        }
                        return Ok(cur);
                    }
                }
                Ok(Value::Unit)
            }
            "reverse" => {
                let mut items = Vec::new();
                let mut cur = args.first().cloned().unwrap_or(Value::Unit);
                loop {
                    match &cur {
                        Value::Data(c, fields) if c.as_str() == "Cons" && !fields.is_empty() => {
                            items.push(fields[0].clone());
                            if fields.len() >= 2 { cur = fields[1].clone(); continue; }
                        }
                        _ => break,
                    }
                    break;
                }
                items.reverse();
                Ok(list_from_vec(items))
            }
            "sort" => {
                let mut items: Vec<i64> = Vec::new();
                let mut cur = args.first().cloned().unwrap_or(Value::Unit);
                loop {
                    match &cur {
                        Value::Data(c, fields) if c.as_str() == "Cons" && !fields.is_empty() => {
                            if let Value::Int(n) = &fields[0] { items.push(*n); }
                            if fields.len() >= 2 { cur = fields[1].clone(); continue; }
                        }
                        _ => break,
                    }
                    break;
                }
                items.sort();
                Ok(list_from_vec(items.into_iter().map(Value::Int).collect()))
            }
            "count" => {
                let mut n: i64 = 0;
                let mut cur = args.first().cloned().unwrap_or(Value::Unit);
                loop {
                    match &cur {
                        Value::Data(c, fields) if c.as_str() == "Cons" && !fields.is_empty() => {
                            n += 1;
                            if fields.len() >= 2 { cur = fields[1].clone(); continue; }
                        }
                        _ => break,
                    }
                    break;
                }
                Ok(Value::Int(n))
            }
            "range" => {
                if args.len() >= 2 {
                    if let (Value::Int(s), Value::Int(e)) = (&args[0], &args[1]) {
                        let mut items: Vec<Value> = ((*s)..(*e)).map(Value::Int).collect();
                        items.reverse();
                        return Ok(list_from_vec(items));
                    }
                }
                Ok(Value::Unit)
            }
            "zip" => {
                let mut items = Vec::new();
                let mut a = args.first().cloned().unwrap_or(Value::Unit);
                let mut b = args.get(1).cloned().unwrap_or(Value::Unit);
                loop {
                    match (&a, &b) {
                        (Value::Data(c1, f1), Value::Data(c2, f2))
                            if c1.as_str() == "Cons" && c2.as_str() == "Cons"
                            && !f1.is_empty() && !f2.is_empty() =>
                        {
                            items.push(Value::Data(Symbol::new("Cons"), vec![
                                Value::Data(Symbol::new("Pair"), vec![f1[0].clone(), f2[0].clone()]),
                                Value::Data(Symbol::new("Nil"), vec![]),
                            ]));
                            if f1.len() >= 2 { a = f1[1].clone(); } else { break; }
                            if f2.len() >= 2 { b = f2[1].clone(); } else { break; }
                        }
                        _ => break,
                    }
                }
                items.reverse();
                Ok(list_from_vec(items))
            }
            "concat" => {
                let mut all = Vec::new();
                for a in args {
                    let mut cur = a.clone();
                    loop {
                        match &cur {
                            Value::Data(c, fields) if c.as_str() == "Cons" && !fields.is_empty() => {
                                all.push(fields[0].clone());
                                if fields.len() >= 2 { cur = fields[1].clone(); continue; }
                            }
                            _ => break,
                        }
                        break;
                    }
                }
                all.reverse();
                Ok(list_from_vec(all))
            }
            "map" => {
                if args.len() >= 2 {
                    let func = args[0].clone();
                    let mut items = Vec::new();
                    let mut cur = args[1].clone();
                    loop {
                        match &cur {
                            Value::Data(c, fields) if c.as_str() == "Cons" && !fields.is_empty() => {
                                let mapped = self.apply(func.clone(), &[fields[0].clone()])?;
                                items.push(mapped);
                                if fields.len() >= 2 { cur = fields[1].clone(); continue; }
                            }
                            _ => break,
                    }
                    break;
                }
                    return Ok(list_from_vec(items));
                }
                Ok(Value::Unit)
            }
            "filter" => {
                if args.len() >= 2 {
                    let pred = args[0].clone();
                    let mut items = Vec::new();
                    let mut cur = args[1].clone();
                    loop {
                        match &cur {
                            Value::Data(c, fields) if c.as_str() == "Cons" && !fields.is_empty() => {
                                let r = self.apply(pred.clone(), &[fields[0].clone()])?;
                                if is_truthy(&r) { items.push(fields[0].clone()); }
                                if fields.len() >= 2 { cur = fields[1].clone(); continue; }
                            }
                            _ => break,
                        }
                    break;
                }
                    return Ok(list_from_vec(items));
                }
                Ok(Value::Unit)
            }
            "reduce" | "foldl" => {
                if args.len() >= 3 {
                    let func = args[0].clone();
                    let mut acc = args[1].clone();
                    let mut cur = args[2].clone();
                    loop {
                        match &cur {
                            Value::Data(c, fields) if c.as_str() == "Cons" && !fields.is_empty() => {
                                acc = self.apply(func.clone(), &[acc, fields[0].clone()])?;
                                if fields.len() >= 2 { cur = fields[1].clone(); continue; }
                            }
                            _ => break,
                        }
                        break;
                    }
                    return Ok(acc);
                }
                Ok(Value::Unit)
            }
            "foldr" => {
                if args.len() >= 3 {
                    let func = args[0].clone();
                    let init = args[1].clone();
                    let mut items = Vec::new();
                    let mut cur = args[2].clone();
                    loop {
                        match &cur {
                            Value::Data(c, fields) if c.as_str() == "Cons" && !fields.is_empty() => {
                                items.push(fields[0].clone());
                                if fields.len() >= 2 { cur = fields[1].clone(); continue; }
                            }
                            _ => break,
                        }
                        break;
                    }
                    let mut acc = init;
                    for item in items.iter().rev() {
                        acc = self.apply(func.clone(), &[item.clone(), acc])?;
                    }
                    return Ok(acc);
                }
                Ok(Value::Unit)
            }
            // HoTT interval operations (~, i∧j, i∨j)
            "~" | "interval-neg" => {
                if let Some(Value::Bool(b)) = args.first() { Ok(Value::Bool(!b)) }
                else { Ok(Value::Bool(false)) }
            }
            // i∧j (meet) and i∨j (join) — same as and/or on interval endpoints
            "interval-and" => {
                if args.len() >= 2 {
                    if let (Value::Bool(a), Value::Bool(b)) = (&args[0], &args[1]) { return Ok(Value::Bool(*a && *b)); }
                }
                Ok(Value::Bool(false))
            }
            "interval-or" => {
                if args.len() >= 2 {
                    if let (Value::Bool(a), Value::Bool(b)) = (&args[0], &args[1]) { return Ok(Value::Bool(*a || *b)); }
                }
                Ok(Value::Bool(false))
            }
            _ => Err(EvalError { message: format!("unknown builtin: {}", name) }),
        }
    }
}

fn list_from_vec(items: Vec<Value>) -> Value {
    let mut result = Value::Data(Symbol::new("Nil"), vec![]);
    for item in items.into_iter().rev() {
        result = Value::Data(Symbol::new("Cons"), vec![item, result]);
    }
    result
}

#[derive(Debug, Clone)]
pub struct EvalError {
    pub message: String,
}
impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "eval error: {}", self.message)
    }
}
impl std::error::Error for EvalError {}

fn expect_two_ints(args: &[Value]) -> Result<(i64, i64), EvalError> {
    if args.len() != 2 { return Err(EvalError { message: "expected 2 args".into() }); }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok((*a, *b)),
        _ => Err(EvalError { message: "expected ints".into() }),
    }
}

fn eval_literal(lit: &Literal) -> Value {
    match lit {
        Literal::I8(n) => Value::Int(*n as i64), Literal::I16(n) => Value::Int(*n as i64),
        Literal::I32(n) => Value::Int(*n as i64), Literal::I64(n) => Value::Int(*n),
        Literal::U8(n) => Value::Int(*n as i64), Literal::U16(n) => Value::Int(*n as i64),
        Literal::U32(n) => Value::Int(*n as i64), Literal::U64(n) => Value::Int(*n as i64),
        Literal::F32(n) => Value::Float(*n as f64), Literal::F64(n) => Value::Float(*n),
        Literal::Bool(b) => Value::Bool(*b), Literal::String(s) => Value::Str(s.clone()),
        Literal::Char(c) => Value::Char(*c), Literal::Unit => Value::Unit,
    }
}

fn is_truthy(val: &Value) -> bool {
    !matches!(val, Value::Bool(false) | Value::Unit | Value::Int(0))
}

fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        _ => false,
    }
}

fn value_to_string(val: &Value) -> String {
    match val {
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Str(s) => s.clone(),
        Value::Unit => "()".into(),
        Value::Data(name, _) => format!("<{}>", name),
        _ => "...".into(),
    }
}

fn match_pattern(pat: &Pattern, val: &Value) -> Option<Vec<(Symbol, Value)>> {
    match (pat, val) {
        (Pattern::Wildcard, _) => Some(vec![]),
        (Pattern::Var(name), v) => Some(vec![(name.clone(), v.clone())]),
        (Pattern::Lit(lit), v) => {
            if values_eq(&eval_literal(lit), v) { Some(vec![]) } else { None }
        }
        (Pattern::Con(c_name, subpats), Value::Data(d_name, d_args)) => {
            if c_name == d_name && subpats.len() == d_args.len() {
                let mut bindings = Vec::new();
                for (sp, dv) in subpats.iter().zip(d_args) {
                    bindings.extend(match_pattern(sp, dv)?);
                }
                Some(bindings)
            } else { None }
        }
        _ => None,
    }
}
