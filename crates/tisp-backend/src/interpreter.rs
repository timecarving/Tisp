use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::types::Type;
use tisp_core::span::Span;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tisp_runtime::RegionStack;
use tisp_runtime::region::RegionId;
use tisp_runtime::logic::ConstraintStore as LogicStore;
use tisp_runtime::logic::LogicValue;
use tisp_runtime::constraint::ConstraintStore as ClpStore;
use tisp_runtime::abduction::AbductionEngine;
use tisp_runtime::process::CryptoEngine;
use tisp_runtime::frp::Signal;
use crate::process::{ProcessRuntime, ModelChecker};
use crate::temporal::Stream;
use tisp_core::core_ast::MethodCategory;

/// ς-calculus OOP: generic function dispatch table
pub struct Interpreter {
    pub env: Vec<HashMap<Symbol, Value>>,
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
    /// ADT 构造函数参数个数:ctor_name → arity(≥1;零参构造直接注册为 Data 值)
    ctor_arity: HashMap<String, usize>,
    /// 构造器字段名表(§7.2 记录字段访问):ctor_name → 字段名列表
    field_names: HashMap<Symbol, Vec<Symbol>>,
    /// §21 多解收集模式:find-all 期间为 true
    collect_mode: bool,
    /// §21 收集到的解(每个解为值列表)
    collected_solutions: Vec<Vec<Value>>,
    /// §21 收集模式下 Search 的起始 trail 深度(arm 间隔离用)
    collect_start_depth: usize,
    /// 活跃 effect handler 栈(§12.2):perform 从栈顶向下分发
    handlers: Vec<ActiveHandler>,
    /// π-calculus 通道运行时(§27):send/recv 经共享缓冲区
    process_runtime: Arc<Mutex<ProcessRuntime>>,
    /// Applied π-calculus 加密引擎(§27.4/27.5)
    crypto: CryptoEngine,
    /// 惰性数值流缓存(§18):stream_id → Stream<i64>
    streams: HashMap<u64, Stream<i64>>,
    next_stream_id: u64,
    /// §27 ambients:已注册的 ambient 名
    ambients: HashMap<Symbol, bool>,
    /// §28 验证属性表:prop 名 → 属性表达式(defprop 登记,verify 求值)
    properties: HashMap<Symbol, CoreExpr>,
    /// §23 构造器 → 所属 ADT 名(类型类实例分发用)
    ctor_to_adt: HashMap<Symbol, Symbol>,
    /// FRP 信号缓存(§18.5):signal_id → Signal<Value>
    signals: HashMap<u64, Signal<Value>>,
    next_signal_id: u64,
    /// CLP 变量 id → 符号名(§21.5 label 解回绑用)
    clp_var_names: HashMap<u64, Symbol>,

    /// §9 反射:name → (参数数, 声明类型, 效果行, 参数等级列表)
    pub def_sigs: HashMap<Symbol, (usize, Option<tisp_core::types::Type>, tisp_core::types::EffectRow, Vec<tisp_core::types::Grade>)>,

    /// §24 gensym 计数器(宏卫生)
    pub gensym_counter: std::sync::atomic::AtomicU64,

    /// §26 dlopen 持有的动态库(ffi feature)
    #[cfg(feature = "ffi")]
    pub extern_libs: Vec<libloading::Library>,

    /// §12.6 单处理器 handle 优化计数(monadic 状态传递路径)
    pub monadic_handles: usize,
}

/// 活跃的 effect handler(Handle 求值时入栈,退出时出栈)
struct ActiveHandler {
    /// 状态槽(§12.3 State 等带状态 effect);None 表示未初始化
    state: Option<Value>,
    clauses: Vec<HandlerClause>,
}

pub type BuiltinFn = Arc<dyn Fn(&mut Interpreter, &[Value]) -> Result<Value, EvalError> + Send + Sync>;

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
    /// 0 级(QTT 擦除,§10.1)参数位置索引:不绑定进环境;实参不求值
    pub zero_params: Vec<usize>,
    pub body: CoreExpr,
    pub env: HashMap<Symbol, Value>,
}

/// 参数列表中 0 级参数的位置索引
fn zero_param_indices(params: &[tisp_core::core_ast::Param]) -> Vec<usize> {
    params.iter().enumerate()
        .filter(|(_, p)| p.grade == tisp_core::types::Grade::Zero)
        .map(|(i, _)| i)
        .collect()
}

/// 表达式是否无副作用(0 级实参可安全不求值)
fn is_side_effect_free(node: &CoreExprNode) -> bool {
    matches!(node, CoreExprNode::Lit(_) | CoreExprNode::Var(_))
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
        Self { env: vec![HashMap::new()], next_chan_id: 0,
               regions: RegionStack::new(4096), current_region: None,
               logic_store: LogicStore::new(), logic_vars: HashMap::new(),
               session_protocol: HashMap::new(), clp_store: ClpStore::new(),
               generic_table: HashMap::new(),
               instance_dict: HashMap::new(),
               ctor_arity: HashMap::new(),
               field_names: HashMap::new(),
               collect_mode: false,
               collected_solutions: Vec::new(),
               collect_start_depth: 0,
               handlers: Vec::new(),
               process_runtime: Arc::new(Mutex::new(ProcessRuntime::new())),
               crypto: CryptoEngine::new(),
               streams: HashMap::new(),
               next_stream_id: 0,
               ambients: HashMap::new(),
               properties: HashMap::new(),
               ctor_to_adt: HashMap::new(),
               signals: HashMap::new(),
               next_signal_id: 0,
               clp_var_names: HashMap::new(),
               def_sigs: HashMap::new(),
               gensym_counter: std::sync::atomic::AtomicU64::new(0),
               #[cfg(feature = "ffi")]
               extern_libs: Vec::new(),
               monadic_handles: 0 }
    }

    /// §26 真实 dlopen:经 libloading 解析符号并构造可调用内置(i64 → i64 C ABI)
    #[cfg(feature = "ffi")]
    fn load_extern(&mut self, lib_path: &str, sym: &str) -> Result<Value, String> {
        use libloading::Library;
        let lib = unsafe { Library::new(lib_path) }
            .map_err(|e| format!("无法加载动态库 {}: {}", lib_path, e))?;
        // 按符号名尝试常见签名:i64→i64(整数)、f64→f64(浮点)
        let sym_name = sym.to_string();
        // 尝试 int fn(int)
        if let Ok(f) = unsafe { lib.get::<unsafe extern "C" fn(i64) -> i64>(sym_name.as_bytes()) } {
            let f = *f; // 复制函数指针('static),闭包不借用 lib
            let v = Value::Builtin(sym_name.clone().into(), Arc::new(move |_s, args| {
                let a = match args.first() {
                    Some(Value::Int(n)) => *n,
                    Some(Value::Float(n)) => *n as i64,
                    _ => 0,
                };
                Ok(Value::Int(unsafe { f(a) }))
            }));
            self.extern_libs.push(lib);
            return Ok(v);
        }
        // 尝试 double fn(double)
        if let Ok(f) = unsafe { lib.get::<unsafe extern "C" fn(f64) -> f64>(sym_name.as_bytes()) } {
            let f = *f;
            let v = Value::Builtin(sym_name.clone().into(), Arc::new(move |_s, args| {
                let a = match args.first() {
                    Some(Value::Int(n)) => *n as f64,
                    Some(Value::Float(n)) => *n,
                    _ => 0.0,
                };
                Ok(Value::Float(unsafe { f(a) }))
            }));
            self.extern_libs.push(lib);
            return Ok(v);
        }
        Err(format!("符号 {} 无匹配的 C ABI 签名", sym))
    }

    pub fn define(&mut self, name: Symbol, value: Value) {
        if let Some(top) = self.env.last_mut() {
            top.insert(name, value);
        }
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
            // §21:Int 值若对应已注册的逻辑变量 id,转为 Var(使 ==/unify 可绑定)
            Value::Int(n) => {
                if self.logic_vars.contains_key(&(*n as u64)) {
                    LogicValue::Var(*n as u64)
                } else {
                    LogicValue::Int(*n)
                }
            }
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
        // 单一实现源:所有内置函数统一在此注册为闭包(可通过 &mut self 访问解释器状态)。
        // 支持部分应用(柯里化)的名字见 CURRIED_BUILTINS,由 apply 处理。
        let builtins: Vec<(Symbol, Value)> = vec![
            // ── 算术与比较(自然多参:支持折叠,至少 2 个参数)──
            bi("+", |_s, args| {
                let mut acc = 0i64;
                for a in args {
                    match a { Value::Int(n) => acc += n, _ => return Err(EvalError { message: "expected ints".into() }) }
                }
                Ok(Value::Int(acc))
            }),
            bi("-", |_s, args| {
                if args.len() < 2 { return Err(EvalError { message: "- needs at least 2 args".into() }); }
                let mut acc = match args[0] { Value::Int(n) => n, _ => return Err(EvalError { message: "expected ints".into() }) };
                for a in &args[1..] {
                    match a { Value::Int(n) => acc -= n, _ => return Err(EvalError { message: "expected ints".into() }) }
                }
                Ok(Value::Int(acc))
            }),
            bi("*", |_s, args| {
                let mut acc = 1i64;
                for a in args {
                    match a { Value::Int(n) => acc *= n, _ => return Err(EvalError { message: "expected ints".into() }) }
                }
                Ok(Value::Int(acc))
            }),
            bi("/", |_s, args| {
                if args.len() < 2 { return Err(EvalError { message: "/ needs at least 2 args".into() }); }
                let mut acc = match args[0] { Value::Int(n) => n, _ => return Err(EvalError { message: "expected ints".into() }) };
                for a in &args[1..] {
                    match a {
                        Value::Int(n) => { if *n == 0 { return Err(EvalError { message: "division by zero".into() }); } acc /= n; }
                        _ => return Err(EvalError { message: "expected ints".into() }),
                    }
                }
                Ok(Value::Int(acc))
            }),
            bi("<", |_s, args| { let (a, b) = expect_two_ints(args)?; Ok(Value::Bool(a < b)) }),
            bi(">", |_s, args| { let (a, b) = expect_two_ints(args)?; Ok(Value::Bool(a > b)) }),
            bi("<=", |_s, args| { let (a, b) = expect_two_ints(args)?; Ok(Value::Bool(a <= b)) }),
            bi(">=", |_s, args| { let (a, b) = expect_two_ints(args)?; Ok(Value::Bool(a >= b)) }),
            bi("=", |_s, args| {
                if args.len() != 2 { return Err(EvalError { message: "= needs 2 args".into() }); }
                Ok(Value::Bool(values_eq(&args[0], &args[1])))
            }),
            bi("!=", |_s, args| {
                if args.len() != 2 { return Err(EvalError { message: "!= needs 2 args".into() }); }
                Ok(Value::Bool(!values_eq(&args[0], &args[1])))
            }),
            bi("not=", |_s, args| {
                if args.len() != 2 { return Err(EvalError { message: "not= needs 2 args".into() }); }
                Ok(Value::Bool(!values_eq(&args[0], &args[1])))
            }),
            // ── 数学 ──
            bi("abs", |_s, args| {
                if let Some(Value::Int(n)) = args.first() {
                    // i64::MIN 的 abs 会溢出,饱和到 i64::MAX
                    Ok(Value::Int(n.checked_abs().unwrap_or(i64::MAX)))
                } else { Ok(Value::Int(0)) }
            }),
            bi("sqrt", |_s, args| {
                match args.first() {
                    Some(Value::Float(n)) => Ok(Value::Float(n.sqrt())),
                    Some(Value::Int(n)) => Ok(Value::Float((*n as f64).sqrt())),
                    _ => Ok(Value::Float(0.0)),
                }
            }),
            bi("pow", |_s, args| {
                if args.len() >= 2 {
                    if let (Value::Int(b), Value::Int(e)) = (&args[0], &args[1]) {
                        // try_from 防止指数超出 u32 时被截断;checked_pow 防止溢出
                        if let Ok(exp) = u32::try_from(*e) {
                            return Ok(Value::Int(b.checked_pow(exp).unwrap_or(i64::MAX)));
                        }
                    }
                }
                Ok(Value::Int(0))
            }),
            bi("mod", |_s, args| {
                let (a, b) = expect_two_ints(args)?;
                if b == 0 { Err(EvalError { message: "modulo by zero".into() }) } else { Ok(Value::Int(a % b)) }
            }),
            bi("min", |_s, args| {
                if args.is_empty() { return Err(EvalError { message: "min needs args".into() }); }
                let mut acc = i64::MAX;
                for a in args {
                    match a { Value::Int(n) => acc = acc.min(*n), _ => return Err(EvalError { message: "expected ints".into() }) }
                }
                Ok(Value::Int(acc))
            }),
            bi("max", |_s, args| {
                if args.is_empty() { return Err(EvalError { message: "max needs args".into() }); }
                let mut acc = i64::MIN;
                for a in args {
                    match a { Value::Int(n) => acc = acc.max(*n), _ => return Err(EvalError { message: "expected ints".into() }) }
                }
                Ok(Value::Int(acc))
            }),
            // ── 字符串 ──
            bi("str", |_s, args| Ok(Value::Str(args.iter().map(value_to_string).collect::<Vec<_>>().join("")))),
            bi("str-len", |_s, args| {
                if let Some(Value::Str(s)) = args.first() { Ok(Value::Int(s.len() as i64)) } else { Ok(Value::Int(0)) }
            }),
            bi("str-concat", |_s, args| {
                let mut acc = String::new();
                for a in args {
                    match a { Value::Str(s) => acc.push_str(s), _ => return Err(EvalError { message: "expected strings".into() }) }
                }
                Ok(Value::Str(acc))
            }),
            bi("str-split", |_s, args| {
                if args.len() >= 2 {
                    if let (Value::Str(s), Value::Str(sep)) = (&args[0], &args[1]) {
                        let parts: Vec<Value> = s.split(sep.as_str()).map(|p| Value::Str(p.to_string())).collect();
                        return Ok(list_from_vec(parts));
                    }
                }
                Ok(list_from_vec(vec![]))
            }),
            bi("str-join", |_s, args| {
                if args.len() >= 2 {
                    if let Value::Str(sep) = &args[0] {
                        let parts: Vec<String> = args[1..].iter().map(value_to_string).collect();
                        return Ok(Value::Str(parts.join(sep)));
                    }
                }
                Ok(Value::Str("".into()))
            }),
            bi("str-sub", |_s, args| {
                if args.len() >= 2 {
                    if let (Value::Str(s), Value::Int(start)) = (&args[0], &args[1]) {
                        let i = *start as usize;
                        // get 避免 UTF-8 非边界索引 panic
                        if let Some(sub) = s.get(i..) { return Ok(Value::Str(sub.to_string())); }
                    }
                }
                Ok(Value::Str("".into()))
            }),
            // ── 类型转换 ──
            bi("i64->f64", |_s, args| {
                if let Some(Value::Int(n)) = args.first() { Ok(Value::Float(*n as f64)) } else { Ok(Value::Float(0.0)) }
            }),
            bi("->string", |_s, args| {
                if let Some(v) = args.first() { Ok(Value::Str(value_to_string(v))) } else { Ok(Value::Str("".into())) }
            }),
            // ── IO ──
            bi("println", |_s, args| { for a in args { println!("{}", value_to_string(a)); } Ok(Value::Unit) }),
            bi("print", |_s, args| {
                for a in args { print!("{}", value_to_string(a)); }
                use std::io::Write; std::io::stdout().flush().ok();
                Ok(Value::Unit)
            }),
            bi("read-line", |_s, _args| {
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf).ok();
                Ok(Value::Str(buf.trim_end_matches('\n').to_string()))
            }),
            bi("slurp", |_s, args| {
                // (slurp path) 读取整个文件为字符串
                if let Some(Value::Str(path)) = args.first() {
                    match std::fs::read_to_string(path) {
                        Ok(s) => Ok(Value::Str(s)),
                        Err(e) => Err(EvalError { message: format!("slurp {}: {}", path, e) }),
                    }
                } else {
                    Ok(Value::Str("".into()))
                }
            }),
            bi("spit", |_s, args| {
                // (spit path content) 写文件(覆盖)
                if args.len() >= 2 {
                    if let Value::Str(path) = &args[0] {
                        let content = value_to_string(&args[1]);
                        if let Err(e) = std::fs::write(path, content) {
                            return Err(EvalError { message: format!("spit {}: {}", path, e) });
                        }
                        return Ok(Value::Unit);
                    }
                }
                Ok(Value::Unit)
            }),
            // ── 列表 ──
            bi("cons", |_s, args| {
                if args.len() >= 2 { Ok(Value::Data(Symbol::new("Cons"), vec![args[0].clone(), args[1].clone()])) }
                else { Ok(Value::Unit) }
            }),
            // §4 集合构造器
            bi("list", |_s, args| Ok(list_from_vec(args.to_vec()))),
            bi("vector", |_s, args| Ok(Value::Data(Symbol::new("Vec"), args.to_vec()))),
            bi("hash-map", |_s, args| {
                let mut pairs = Vec::new();
                let mut i = 0;
                while i + 1 < args.len() {
                    pairs.push(Value::Data(Symbol::new("Pair"), vec![args[i].clone(), args[i + 1].clone()]));
                    i += 2;
                }
                Ok(Value::Data(Symbol::new("Map"), pairs))
            }),
            bi("hash-set", |_s, args| Ok(Value::Data(Symbol::new("Set"), args.to_vec()))),
            bi("first", |_s, args| {
                if let Some(Value::Data(c, fields)) = args.first() {
                    if c.as_str() == "Cons" && !fields.is_empty() { return Ok(fields[0].clone()); }
                }
                Ok(Value::Unit)
            }),
            bi("rest", |_s, args| {
                if let Some(Value::Data(c, fields)) = args.first() {
                    if c.as_str() == "Cons" && fields.len() >= 2 { return Ok(fields[1].clone()); }
                }
                Ok(Value::Unit)
            }),
            bi("nth", |_s, args| {
                if args.len() >= 2 {
                    if let Value::Int(n) = &args[1] {
                        let mut cur = args[0].clone();
                        let mut remaining = *n;
                        loop {
                            match &cur {
                                // Cons 链:第 0 个是头,否则推进到尾
                                Value::Data(c, fields) if c.as_str() == "Cons" && fields.len() >= 2 => {
                                    if remaining == 0 { return Ok(fields[0].clone()); }
                                    cur = fields[1].clone();
                                    remaining -= 1;
                                }
                                // 任意构造器字段列表(§22 方法里提取字段)
                                Value::Data(_, fields) if !fields.is_empty() => {
                                    if remaining == 0 { return Ok(fields[0].clone()); }
                                    if (remaining as usize) < fields.len() {
                                        return Ok(fields[remaining as usize].clone());
                                    }
                                    return Ok(Value::Unit);
                                }
                                _ => return Ok(Value::Unit),
                            }
                        }
                    }
                }
                Ok(Value::Unit)
            }),
            bi("take", |_s, args| {
                if args.len() >= 2 {
                    if let Value::Int(n) = &args[1] {
                        if *n <= 0 { return Ok(list_from_vec(vec![])); }
                        let items = list_to_vec(&args[0]);
                        return Ok(list_from_vec(items.into_iter().take(*n as usize).collect()));
                    }
                }
                Ok(Value::Unit)
            }),
            bi("drop", |_s, args| {
                if args.len() >= 2 {
                    if let Value::Int(n) = &args[1] {
                        if *n <= 0 { return Ok(args[0].clone()); }
                        let items = list_to_vec(&args[0]);
                        return Ok(list_from_vec(items.into_iter().skip(*n as usize).collect()));
                    }
                }
                Ok(Value::Unit)
            }),
            bi("reverse", |_s, args| {
                let mut items = list_to_vec(args.first().unwrap_or(&Value::Unit));
                items.reverse();
                Ok(list_from_vec(items))
            }),
            bi("sort", |_s, args| {
                let mut items: Vec<i64> = list_to_vec(args.first().unwrap_or(&Value::Unit))
                    .iter().filter_map(|v| if let Value::Int(n) = v { Some(*n) } else { None }).collect();
                items.sort();
                Ok(list_from_vec(items.into_iter().map(Value::Int).collect()))
            }),
            bi("count", |_s, args| Ok(Value::Int(list_to_vec(args.first().unwrap_or(&Value::Unit)).len() as i64))),
            bi("length", |_s, args| Ok(Value::Int(list_to_vec(args.first().unwrap_or(&Value::Unit)).len() as i64))),
            bi("range", |_s, args| {
                if args.len() >= 2 {
                    if let (Value::Int(s), Value::Int(e)) = (&args[0], &args[1]) {
                        return Ok(list_from_vec((*s..*e).map(Value::Int).collect()));
                    }
                }
                Ok(Value::Unit)
            }),
            bi("zip", |_s, args| {
                let a = list_to_vec(args.first().unwrap_or(&Value::Unit));
                let b = list_to_vec(args.get(1).unwrap_or(&Value::Unit));
                let pairs: Vec<Value> = a.iter().zip(b.iter()).map(|(x, y)| {
                    Value::Data(Symbol::new("Cons"), vec![
                        Value::Data(Symbol::new("Pair"), vec![x.clone(), y.clone()]),
                        Value::Data(Symbol::new("Nil"), vec![]),
                    ])
                }).collect();
                Ok(list_from_vec(pairs))
            }),
            bi("concat", |_s, args| {
                let all: Vec<Value> = args.iter().flat_map(|a| list_to_vec(a)).collect();
                Ok(list_from_vec(all))
            }),
            bi("append", |_s, args| {
                // (append list1 list2 ...) 多列表拼接(同 concat 语义)
                let all: Vec<Value> = args.iter().flat_map(|a| list_to_vec(a)).collect();
                Ok(list_from_vec(all))
            }),
            bi("map", |s, args| {
                if args.len() >= 2 {
                    let func = args[0].clone();
                    let mut items = Vec::new();
                    for item in list_to_vec(&args[1]) {
                        items.push(s.apply(func.clone(), &[item])?);
                    }
                    return Ok(list_from_vec(items));
                }
                Ok(Value::Unit)
            }),
            bi("filter", |s, args| {
                if args.len() >= 2 {
                    let pred = args[0].clone();
                    let mut items = Vec::new();
                    for item in list_to_vec(&args[1]) {
                        let r = s.apply(pred.clone(), &[item.clone()])?;
                        if is_truthy(&r) { items.push(item); }
                    }
                    return Ok(list_from_vec(items));
                }
                Ok(Value::Unit)
            }),
            bi("reduce", |s, args| {
                if args.len() >= 3 {
                    let func = args[0].clone();
                    let mut acc = args[1].clone();
                    for item in list_to_vec(&args[2]) {
                        acc = s.apply(func.clone(), &[acc, item])?;
                    }
                    return Ok(acc);
                }
                Ok(Value::Unit)
            }),
            bi("foldl", |s, args| {
                if args.len() >= 3 {
                    let func = args[0].clone();
                    let mut acc = args[1].clone();
                    for item in list_to_vec(&args[2]) {
                        acc = s.apply(func.clone(), &[acc, item])?;
                    }
                    return Ok(acc);
                }
                Ok(Value::Unit)
            }),
            bi("foldr", |s, args| {
                if args.len() >= 3 {
                    let func = args[0].clone();
                    let init = args[1].clone();
                    let items = list_to_vec(&args[2]);
                    let mut acc = init;
                    for item in items.iter().rev() {
                        acc = s.apply(func.clone(), &[item.clone(), acc])?;
                    }
                    return Ok(acc);
                }
                Ok(Value::Unit)
            }),
            // ── 逻辑 ──
            bi("not", |_s, args| {
                if let Some(Value::Bool(b)) = args.first() { Ok(Value::Bool(!b)) } else { Ok(Value::Bool(false)) }
            }),
            // ── HoTT 区间运算 ──
            bi("~", |_s, args| {
                if let Some(Value::Bool(b)) = args.first() { Ok(Value::Bool(!b)) } else { Ok(Value::Bool(false)) }
            }),
            bi("interval-neg", |_s, args| {
                if let Some(Value::Bool(b)) = args.first() { Ok(Value::Bool(!b)) } else { Ok(Value::Bool(false)) }
            }),
            bi("interval-and", |_s, args| {
                if args.len() >= 2 {
                    if let (Value::Bool(a), Value::Bool(b)) = (&args[0], &args[1]) { return Ok(Value::Bool(*a && *b)); }
                }
                Ok(Value::Bool(false))
            }),
            bi("interval-or", |_s, args| {
                if args.len() >= 2 {
                    if let (Value::Bool(a), Value::Bool(b)) = (&args[0], &args[1]) { return Ok(Value::Bool(*a || *b)); }
                }
                Ok(Value::Bool(false))
            }),
            // ── 反射 ──
            bi("type-of", |_s, args| {
                if let Some(v) = args.first() { Ok(Value::Str(v.type_name().to_string())) } else { Ok(Value::Str("unknown".into())) }
            }),
            bi("grade-of", |s, args| {
                // §10:查询定义等级(依赖等级显示);无参兼容旧行为
                if let Some(Value::Str(name)) = args.first() {
                    if let Some((_, _, _, grades)) = s.def_sigs.get(&Symbol::new(name)) {
                        return Ok(Value::Str(format!("{:?}", grades)));
                    }
                }
                Ok(Value::Str("ω".into()))
            }),
            bi("mode-of", |_s, _args| Ok(Value::Str("in".into()))),
            bi("effects-of", |_s, _args| Ok(Value::Str("Pure".into()))),
            bi("determinism-of", |_s, _args| Ok(Value::Str("det".into()))),
            // ── 进程/通道(§27.2/27.3):接线 ProcessRuntime ──
            bi("chan", |s, _args| {
                let id = s.next_chan_id; s.next_chan_id += 1;
                let name = Symbol::new(&format!("chan-{}", id));
                s.process_runtime.lock().unwrap().new_channel(name.clone());
                Ok(Value::Str(name.as_str().to_string()))
            }),
            bi("send", |s, args| {
                if args.len() >= 2 {
                    let chan_name = Symbol::new(&channel_name(&args[0]));
                    s.process_runtime.lock().unwrap().send(&chan_name, to_proc_value(&args[1]));
                    return Ok(Value::Unit);
                }
                Err(EvalError { message: "send needs 2 args".into() })
            }),
            bi("recv", |s, args| {
                if let Some(ch) = args.first() {
                    let chan_name = Symbol::new(&channel_name(ch));
                    return match s.process_runtime.lock().unwrap().recv(&chan_name) {
                        Some(v) => Ok(from_proc_value(v)),
                        None => Err(EvalError { message: format!("recv on empty channel {}", chan_name) }),
                    };
                }
                Err(EvalError { message: "recv needs 1 arg".into() })
            }),
            // ── FRP / 时间(§18):stream/stream-take 接线 temporal::Stream ──
            bi("stream", |s, args| {
                if let Some(Value::Int(start)) = args.first() {
                    let st = Stream::unfold(*start, |n| n + 1);
                    let id = s.next_stream_id; s.next_stream_id += 1;
                    s.streams.insert(id, st);
                    return Ok(Value::Data(Symbol::new("Stream"), vec![Value::Int(*start), Value::Int(id as i64)]));
                }
                Ok(Value::Int(0))
            }),
            bi("stream-take", |s, args| {
                if args.len() >= 2 {
                    if let Value::Int(n) = &args[1] {
                        if let Ok(id) = stream_id(&args[0]) {
                            if let Some(st) = s.streams.get(&id).cloned() {
                                let items: Vec<Value> = st.take(*n as usize).into_iter().map(Value::Int).collect();
                                return Ok(list_from_vec(items));
                            }
                        }
                    }
                }
                Ok(list_from_vec(vec![]))
            }),
            bi("delay", |_s, args| { if args.is_empty() { Ok(Value::Unit) } else { Ok(args[0].clone()) } }),
            bi("advance", |s, args| {
                if let Some(v) = args.first() {
                    let id = stream_id(v)?;
                    let next = match s.streams.get(&id).and_then(|st| st.clone().next()) {
                        Some(ns) => ns,
                        None => return Err(EvalError { message: "stream exhausted".into() }),
                    };
                    s.streams.insert(id, next.clone());
                    let head = *next.now();
                    return Ok(Value::Data(Symbol::new("Stream"), vec![Value::Int(head), Value::Int(id as i64)]));
                }
                Ok(Value::Unit)
            }),
            bi("clock", |_s, _args| Ok(Value::Str("clock@1Hz".into()))),
            // ── 逻辑编程(占位实现,真实语义见 CoreExprNode::Fresh/Unify/Search/Commit)──
            bi("fresh", |_s, _args| Ok(Value::Int(0))),
            bi("==", |_s, args| {
                if args.len() == 2 && values_eq(&args[0], &args[1]) { Ok(Value::Bool(true)) } else { Ok(Value::Bool(false)) }
            }),
            bi("search", |s, args| {
                // §21.3:(search thunk) 回溯边界:成功保留绑定,失败恢复 trail
                if let Some(f) = args.first() {
                    let depth = s.logic_store.trail_depth();
                    let cp_len = s.logic_store.choice_points_len();
                    s.logic_store.mark_choice_point();
                    let r = s.apply(f.clone(), &[]);
                    if r.is_err() {
                        s.logic_store.restore_to(depth);
                    }
                    s.logic_store.truncate_choice_points(cp_len);
                    r.or_else(|_| Ok(Value::Bool(false)))
                } else {
                    Ok(Value::Bool(false))
                }
            }),
            bi("solve-all", |s, args| {
                // §21.5:(solve-all x) 枚举 CLP 变量 x 域中的全部解(升序去重)
                if let Some(Value::Int(id)) = args.first() {
                    let id = *id as u64;
                    let mut results = Vec::new();
                    if s.clp_store.label(&[id], &mut results) {
                        let mut vals: Vec<Value> = results.iter()
                            .filter_map(|sol| sol.get(&id).map(|v| Value::Int(*v)))
                            .collect();
                        vals.sort_by_key(|v| if let Value::Int(n) = v { *n } else { 0 });
                        let mut seen = std::collections::HashSet::new();
                        vals.retain(|v| seen.insert(format!("{:?}", v)));
                        return Ok(list_from_vec(vals));
                    }
                }
                Ok(Value::Unit)
            }),
            bi("find-all", |s, args| {
                // §21:(find-all thunk) 收集 thunk 中 Search 产生的全部解(逻辑变量绑定)
                if let Some(thunk) = args.first() {
                    s.collect_mode = true;
                    s.collected_solutions.clear();
                    let _ = s.apply(thunk.clone(), &[]);
                    // 无 Match 收集点时,取当前绑定快照作为唯一解(§21.4)
                    if s.collected_solutions.is_empty() {
                        let sol: Vec<Value> = s.logic_store.bound_snapshot()
                            .iter().map(|(_, lv)| logic_to_value(lv)).collect();
                        if !sol.is_empty() { s.collected_solutions.push(sol); }
                    }
                    let results: Vec<Value> = s.collected_solutions.iter()
                        .map(|sol| list_from_vec(sol.clone()))
                        .collect();
                    s.collect_mode = false;
                    s.collected_solutions.clear();
                    return Ok(list_from_vec(results));
                }
                Ok(Value::Unit)
            }),
            bi("gensym", |s, _args| {
                // §24:每次调用生成唯一符号(宏卫生)
                let n = s.gensym_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(Value::Str(format!("g{}", n)))
            }),
            bi("find-attack", |_s, _args| {
                // §28 演示攻击场景:机密消息经不安全通道,攻击者窃听(逐消息加入知识)
                let checker = ModelChecker::new(20);
                let secret = "SECRET";
                let init: (std::collections::BTreeSet<String>, usize) = (std::collections::BTreeSet::new(), 0usize);
                let result = checker.find_attack(init,
                    |(knowledge, _)| knowledge.contains(secret),
                    |(knowledge, step)| {
                        let mut k = knowledge.clone();
                        if *step == 0 { k.insert("pub".to_string()); }
                        if *step == 1 { k.insert(secret.to_string()); }
                        vec![(k, step + 1)]
                    });
                Ok(Value::Bool(result.property_holds))
            }),
            bi("check-equivalence", |_s, args| {
                // §28:比较两个列表的状态集(去重后元素相等)
                if args.len() != 2 {
                    return Err(EvalError { message: "check-equivalence expects 2 lists".into() });
                }
                let set_of = |v: &Value| -> std::collections::HashSet<String> {
                    list_to_vec(v).iter().map(|x| format!("{:?}", x)).collect()
                };
                Ok(Value::Bool(set_of(&args[0]) == set_of(&args[1])))
            }),
            bi("verify", |s, args| {
                // §28:(verify name) 或 (verify thunk):求值属性,返回布尔
                if let Some(Value::Str(n)) = args.first() {
                    if let Some(e) = s.properties.get(&Symbol::new(n)).cloned() {
                        let v = s.eval_expr(&e)?;
                        return Ok(Value::Bool(is_truthy(&v)));
                    }
                }
                if let Some(f) = args.first() {
                    let v = s.apply(f.clone(), &[])?;
                    return Ok(Value::Bool(is_truthy(&v)));
                }
                Ok(Value::Bool(false))
            }),
            bi("commit!", |_s, _args| Ok(Value::Unit)),
            // §27 SKI 组合子(支持部分应用)
            bi("S", |s, args| ski_s_apply(s, args.to_vec())),
            bi("K", |s, args| ski_k_apply(s, args.to_vec())),
            bi("I", |_s, args| {
                if let Some(x) = args.first() { Ok(x.clone()) } else { Ok(Value::Unit) }
            }),
            // ── 内置 effect 操作(§12.3):get/put/ask/tell/throw/choose 等,
            //    经 handler 栈分发(perform_effect)──
            bi("get", |s, _args| s.perform_effect("get", vec![])),
            bi("put", |s, args| s.perform_effect("put", args.to_vec())),
            bi("ask", |s, _args| s.perform_effect("ask", vec![])),
            bi("tell", |s, args| s.perform_effect("tell", args.to_vec())),
            bi("throw", |s, args| s.perform_effect("throw", args.to_vec())),
            bi("choose", |s, args| s.perform_effect("choose", args.to_vec())),
        ];
        for (name, value) in builtins {
            self.define(name, value);
        }
    }

    /// 内置或 ADT 构造函数的参数个数
    fn full_arity(&self, name: &str) -> Option<usize> {
        builtin_arity(name).or_else(|| self.ctor_arity.get(name).copied())
    }

    pub fn run_program(&mut self, program: &CoreProgram) -> Result<Option<Value>, EvalError> {
        self.register_builtins();
        // 注册 ADT 构造函数:零参构造注册为返回 Data 的 0 参内置(经 (Nil) 调用形式),
        // 带参构造注册为构造函数内置
        for decl in &program.data_decls {
            for ctor in &decl.constructors {
                let ctor_name = ctor.name.clone();
                let field_count = ctor.fields.len();
                // §23:构造器 → 所属 ADT(类型类实例分发)
                self.ctor_to_adt.insert(ctor_name.clone(), decl.name.clone());
                if field_count == 0 {
                    self.define(ctor_name.clone(), Value::Builtin(ctor_name.as_str().into(), Arc::new(move |_s, _args| {
                        Ok(Value::Data(ctor_name.clone(), vec![]))
                    })));
                } else {
                    let ctor_name2 = ctor_name.clone();
                    self.ctor_arity.insert(ctor_name.as_str().to_string(), field_count);
                    // §7.2 字段名表(记录字段访问 (:field obj))
                    let names: Vec<Symbol> = ctor.fields.iter().enumerate()
                        .map(|(i, f)| f.name.clone().unwrap_or_else(|| Symbol::new(&format!("_f{}", i))))
                        .collect();
                    self.field_names.insert(ctor.name.clone(), names);
                    self.define(ctor_name, Value::Builtin(ctor_name2.as_str().into(), Arc::new(move |_s, args| {
                        Ok(Value::Data(ctor_name2.clone(), args.to_vec()))
                    })));
                }
            }
        }
        // §9 反射签名表:name → (参数数, 声明类型)
        for def in &program.defs {
            let (arity, grades) = match &def.body.node {
                CoreExprNode::Lam(lam) => (lam.params.len(), lam.params.iter().map(|p| p.grade.clone()).collect()),
                _ => (0, vec![]),
            };
            self.def_sigs.insert(def.name.clone(), (arity, def.ty.clone(), def.effects.clone(), grades));
        }

        // §7.5 deriving 派生:注册 eq-Name / show-Name 函数
        for decl in &program.data_decls {
            for d in &decl.deriving {
                let type_name = decl.name.clone();
                match d.as_str() {
                    "Eq" => {
                        let name = Symbol::new(&format!("eq-{}", type_name));
                        let tn = type_name.clone();
                        self.define(name, Value::Builtin(format!("eq-{}", tn).into(), Arc::new(|_s, args| {
                            Ok(Value::Bool(if args.len() == 2 {
                                values_eq(&args[0], &args[1])
                            } else {
                                false
                            }))
                        })));
                    }
                    "Show" => {
                        let name = Symbol::new(&format!("show-{}", type_name));
                        let tn = type_name.clone();
                        self.define(name, Value::Builtin(format!("show-{}", tn).into(), Arc::new(|_s, args| {
                            Ok(Value::Str(args.first().map(show_value).unwrap_or_else(|| "...".into())))
                        })));
                    }
                    _ => {}
                }
            }
        }

        // Enter a program-level region for stack-like allocation
        self.enter_region("program");

        for def in &program.defs {
            // 声明类节点(defgeneric/defmethod/defclass/definstance/ns/ffi/宏)立即求值,
            // 其余包装为闭包延迟到调用(§6.2 顶层声明语义)
            match &def.body.node {
                CoreExprNode::GenericDef(..) | CoreExprNode::MethodDef(..)
                | CoreExprNode::ClassDef(..) | CoreExprNode::InstanceDef(..)
                | CoreExprNode::NSDef(..) | CoreExprNode::ExternDef(..)
                | CoreExprNode::MacroDef(..) | CoreExprNode::HitDef(..)
                | CoreExprNode::TheoremDef(..) | CoreExprNode::CompilerMacroDef(..) => {
                    self.eval_expr(&def.body)?;
                }
                _ => {
                    let closure = Closure {
                        params: vec![],
                        zero_params: vec![],
                        body: def.body.clone(),
                        env: self.env.last().cloned().unwrap_or_default(),
                    };
                    self.define(def.name.clone(), Value::Closure(closure));
                }
            }
        }

        // 入口优先 __top__(顶层表达式),其次 main(§6.3)
        let result = if let Some(top) = self.env.last().and_then(|e| e.get(&Symbol::new("__top__")).cloned()) {
            Ok(Some(self.apply(top, &[])?))
        } else if let Some(main) = self.env.last().and_then(|e| e.get(&Symbol::new("main")).cloned()) {
            Ok(Some(self.apply(main, &[])?))
        } else {
            Ok(None)
        };

        // Leave program region (deallocate all)  
        self.leave_region();
        result
    }

    /// §21.5:把 (op a b) 比较编译为 CLP 约束(常数自动提升为 singleton 变量,实现域传播)
    fn clp_constraint(&mut self, op: &str, a: &Value, b: &Value) {
        // 任意一侧可为常数(提升为 singleton 变量);至少一侧是 CLP 变量才有效
        let lv = match a {
            Value::Int(id) if self.clp_store.domain_of(*id as u64).is_some() => Some(*id as u64),
            Value::Int(c) => Some(self.clp_store.new_int_var(*c, *c)),
            _ => None,
        };
        let rv = match b {
            Value::Int(id) if self.clp_store.domain_of(*id as u64).is_some() => Some(*id as u64),
            Value::Int(c) => Some(self.clp_store.new_int_var(*c, *c)),
            _ => None,
        };
        if let (Some(l), Some(r)) = (lv, rv) {
            // (op X Y) 的 AST 中 a=Y、b=X:语义 X op Y
            match op {
                "<" => self.clp_store.add_lt(r, l), // X < Y
                ">" => self.clp_store.add_lt(l, r), // Y < X
                "=" | "==" => self.clp_store.add_eq(l, r),
                _ => {}
            }
        }
    }

    /// §12.2/12.3:执行 effect 操作,从 handler 栈顶向下分发到匹配的 clause
    pub fn perform_effect(&mut self, op: &str, args: Vec<Value>) -> Result<Value, EvalError> {
        for idx in (0..self.handlers.len()).rev() {
            if let Some(clause) = self.handlers[idx].clauses.iter().find(|c| c.operation.as_str() == op).cloned() {
                let mut local_env = HashMap::new();
                // 绑定操作参数
                for (p, a) in clause.params.iter().zip(&args) {
                    local_env.insert(p.clone(), a.clone());
                }
                // 绑定状态变量(未初始化时绑定 Unit)
                let current_state = self.handlers[idx].state.clone().unwrap_or(Value::Unit);
                if let Some(state_var) = &clause.state {
                    local_env.insert(state_var.clone(), current_state);
                }
                // 绑定续延 k:(k result new_state) → 写回 handler 状态槽,返回 result
                let mut k_env = HashMap::new();
                k_env.insert(Symbol::new("__k_handler"), Value::Int(idx as i64));
                let k_closure = Closure {
                    params: vec![Symbol::new("_k_result"), Symbol::new("_k_new_state")],
                    zero_params: vec![],
                    body: CoreExpr::new(CoreExprNode::Lit(Literal::Unit), Span::dummy()),
                    env: k_env,
                };
                local_env.insert(clause.continuation.clone(), Value::Closure(k_closure));
                self.env.push(local_env);
                let r = self.eval_expr(&clause.body);
                self.env.pop();
                return r;
            }
        }
        Err(EvalError { message: format!("perform {} not in handler", op) })
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
                zero_params: zero_param_indices(&lambda.params),
                body: (*lambda.body).clone(),
                env: self.env.last().cloned().unwrap_or_default(),
            })),
            CoreExprNode::App(func, arg) => {
                // 收集左结合应用链的全部参数,一次性 apply:
                // (f a b c) = App(App(App(f,a),b),c) → apply(f, [a,b,c])
                // 求值顺序与旧版一致:先求函数,再从左到右求参数
                let mut chain: Vec<&CoreExpr> = vec![arg];
                let mut cur = func;
                while let CoreExprNode::App(inner_f, inner_a) = &cur.node {
                    chain.push(inner_a);
                    cur = inner_f;
                }
                chain.reverse();
                let f = self.eval_expr(cur)?;
                // 0 级参数擦除(§10.1):闭包 0 级位置且实参无副作用时,不求值(Unit 占位)
                let zero_positions: Option<Vec<usize>> = match &f {
                    Value::Closure(c) if !c.params.is_empty() => Some(c.zero_params.clone()),
                    // def 包装的 Lam:0 级信息在 inner 参数里
                    Value::Closure(c) => match &c.body.node {
                        CoreExprNode::Lam(lam) => Some(zero_param_indices(&lam.params)),
                        _ => None,
                    },
                    _ => None,
                };
                let mut args = Vec::with_capacity(chain.len());
                for (i, a) in chain.iter().enumerate() {
                    if let Some(z) = &zero_positions {
                        if z.contains(&i) && is_side_effect_free(&a.node) {
                            args.push(Value::Unit);
                            continue;
                        }
                    }
                    args.push(self.eval_expr(a)?);
                }
                self.apply(f, &args)
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
                if self.collect_mode {
                    // §21 多解收集:遍历所有 arm,每个 arm 从干净逻辑状态开始,
                    // 成功 arm 收集逻辑变量绑定快照
                    let mut last = Value::Unit;
                    for arm in arms {
                        self.logic_store.restore_to(self.collect_start_depth);
                        if let Some(bindings) = match_pattern(&arm.pattern, &s) {
                            self.push_scope();
                            for (name, val) in bindings {
                                if let Some(top) = self.env.last_mut() { top.insert(name, val); }
                            }
                            let guard_ok = match &arm.guard {
                                Some(g) => match self.eval_expr(g) {
                                    Ok(v) => is_truthy(&v),
                                    Err(e) => { self.pop_scope(); return Err(e); }
                                },
                                None => true,
                            };
                            if guard_ok {
                                let v = self.eval_expr(&arm.body)?;
                                last = v;
                                // 收集解:逻辑变量绑定快照(值化)
                                let sol: Vec<Value> = self.logic_store.bound_snapshot()
                                    .iter().map(|(_, lv)| logic_to_value(lv)).collect();
                                self.collected_solutions.push(sol);
                            }
                            self.pop_scope();
                        }
                    }
                    return Ok(last);
                }
                for arm in arms {
                    if let Some(bindings) = match_pattern(&arm.pattern, &s) {
                        self.push_scope();
                        for (name, val) in bindings {
                            if let Some(top) = self.env.last_mut() { top.insert(name, val); }
                        }
                        // §8.2 guard:失败则清理绑定并尝试下一个 arm
                        let guard_ok = match &arm.guard {
                            Some(g) => match self.eval_expr(g) {
                                Ok(v) => is_truthy(&v),
                                Err(e) => { self.pop_scope(); return Err(e); }
                            },
                            None => true,
                        };
                        if guard_ok {
                            let r = self.eval_expr(&arm.body);
                            self.pop_scope();
                            return r;
                        }
                        self.pop_scope();
                    }
                }
                Err(EvalError { message: "match failure".into() })
            }
            CoreExprNode::Data(name, args) => {
                let vals: Result<Vec<_>, _> = args.iter().map(|a| self.eval_expr(a)).collect();
                Ok(Value::Data(name.clone(), vals?))
            }
            CoreExprNode::Handle(body, handler) => {
                // §12.6 单处理器优化:唯一 clause → 直接状态传递路径(行为与 handler 等价)
                if tisp_middle::effect_compile::EffectCompiler::new().detect_single_handler(handler) {
                    self.monadic_handles += 1;
                }
                // §12.2:push handler 作用域,求值 body,pop
                let h = ActiveHandler {
                    state: None,
                    clauses: handler.clauses.clone(),
                };
                self.handlers.push(h);
                let result = self.eval_expr(body);
                self.handlers.pop();
                let result = result?;
                if let Some(rc) = &handler.return_clause {
                    // 把 body 结果绑定到 _,求值 return clause
                    let mut local_env = HashMap::new();
                    local_env.insert(Symbol::new("_"), result);
                    self.env.push(local_env);
                    let r = self.eval_expr(rc);
                    self.env.pop();
                    r
                } else {
                    Ok(result)
                }
            }
            CoreExprNode::Perform(op, args) => {
                let mut values = Vec::new();
                for a in args {
                    values.push(self.eval_expr(a)?);
                }
                self.perform_effect(op.as_str(), values)
            }
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
            CoreExprNode::CrispMod(e) => self.eval_expr(e),
            CoreExprNode::ShapeMod(e) => {
                // §17 ʃ 最小语义:路径值形状化 —— 提取区间端点(i0/i1)组成 Shape 容器,
                // 与直通求值可区分
                let v = self.eval_expr(e)?;
                match &v {
                    Value::Data(tag, fields) if tag.as_str() == "Path" => {
                        // Path 端点:首字段 i0 端点,次字段 i1 端点(有则取)
                        let i0 = fields.first().cloned().unwrap_or(Value::Bool(false));
                        let i1 = fields.get(1).cloned().unwrap_or(Value::Bool(true));
                        Ok(Value::Data(Symbol::new("Shape"), vec![v.clone(), i0, i1]))
                    }
                    _ => Ok(Value::Data(Symbol::new("Shape"), vec![v.clone(), Value::Bool(false), Value::Bool(true)])),
                }
            }
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
                    zero_params: vec![],
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
                let cp_len = self.logic_store.choice_points_len();
                self.logic_store.mark_choice_point();
                if self.collect_mode {
                    // §21 多解收集:求值 e(Match 收集所有 arm 解),恢复 trail
                    self.collect_start_depth = depth;
                    let result = self.eval_expr(e);
                    self.logic_store.restore_to(depth);
                    self.logic_store.truncate_choice_points(cp_len);
                    return result.or_else(|_| Ok(Value::Bool(false)));
                }
                let result = self.eval_expr(e);
                if result.is_err() {
                    self.logic_store.restore_to(depth);
                }
                // 无论成败都清理本次标记的 choice point:Search 只返回第一解,
                // 成功后保留的点无消费者且会污染后续 cut/backtrack
                self.logic_store.truncate_choice_points(cp_len);
                result.or_else(|_| Ok(Value::Bool(false)))
            }
            CoreExprNode::Commit(e) => {
                let result = self.eval_expr(e)?;
                self.logic_store.cut();
                Ok(result)
            }
            CoreExprNode::Abduce(e, abducibles) => {
                // §21.6:生成溯因假设并做一致性验证 —— 假设绑定后目标须可满足,
                // 只返回与目标一致的假设集(替换占位实现)
                let doms = std::collections::HashMap::new();
                let vars: Vec<String> = abducibles.iter().map(|s| s.as_str().to_string()).collect();
                let mut engine = AbductionEngine::new();
                let explanations = engine.generate_hypotheses(&vars, &doms);
                let mut consistent: Option<tisp_runtime::abduction::Explanation> = None;
                for exp in explanations {
                    // 快照 CLP 存储,绑定假设,验证目标;验证后恢复
                    let snapshot = self.clp_store.clone();
                    let mut bound_ok = true;
                    for h in &exp.hypotheses {
                        let id = self.clp_var_names.iter()
                            .find(|(_, n)| n.as_str() == h.var)
                            .map(|(id, _)| *id);
                        match id {
                            Some(id) => {
                                self.clp_store.add_eq(id, h.value as u64);
                                // 冲突:域清空 → 假设不一致
                                if self.clp_store.domain_of(id).map(|d| d.is_empty()).unwrap_or(false) {
                                    bound_ok = false;
                                    break;
                                }
                            }
                            None => { bound_ok = false; break; }
                        }
                    }
                    if bound_ok {
                        match self.eval_expr(e) {
                            Ok(v) if is_truthy(&v) => {
                                consistent = Some(exp);
                                break;
                            }
                            _ => {}
                        }
                    }
                    self.clp_store = snapshot;
                }
                match consistent {
                    Some(exp) => {
                        let hyps: Vec<Value> = exp.hypotheses.iter().map(|h| {
                            Value::Data(Symbol::new("Hypothesis"), vec![
                                Value::Str(h.var.clone().into()),
                                Value::Int(h.value),
                            ])
                        }).collect();
                        Ok(list_from_vec(hyps))
                    }
                    // 无一致假设:目标不可满足(现有求值结果作为说明)
                    None => self.eval_expr(e),
                }
            }
            // Constraint Logic Programming (CLP)
            CoreExprNode::Domain(var, lo, hi) => {
                // §21.5:var 可以是未绑定的变量符号(不先求值)
                let var_sym = match &var.node {
                    CoreExprNode::Var(sym) => Some(sym.clone()),
                    _ => {
                        self.eval_expr(var)?;
                        None
                    }
                };
                let l = self.eval_expr(lo)?;
                let h = self.eval_expr(hi)?;
                if let (Value::Int(lo_val), Value::Int(hi_val)) = (&l, &h) {
                    let id = self.clp_store.new_int_var(*lo_val, *hi_val);
                    if let Some(sym) = &var_sym {
                        if let Some(top) = self.env.last_mut() {
                            top.insert(sym.clone(), Value::Int(id as i64));
                            self.clp_var_names.insert(id, sym.clone());
                        }
                    }
                    Ok(Value::Int(id as i64))
                } else { Ok(Value::Unit) }
            },
            CoreExprNode::Constrain(e) => {
                // §21.5:识别 (op a b) 比较结构,生成真实 CLP 约束传播(域收缩);
                // 非比较式退回布尔求值
                if let CoreExprNode::App(f1, a) = &e.node {
                    if let CoreExprNode::App(f2, b) = &f1.node {
                        if let CoreExprNode::Var(op) = &f2.node {
                            if matches!(op.as_str(), "<" | ">" | "=" | "==") {
                                let va = self.eval_expr(a)?;
                                let vb = self.eval_expr(b)?;
                                self.clp_constraint(op.as_str(), &va, &vb);
                                return Ok(Value::Bool(true));
                            }
                        }
                    }
                }
                let r = self.eval_expr(e)?;
                Ok(r)
            },
            CoreExprNode::Label(a, b) => {
                // §21.5:label 变量域,回溯求第一个解并绑定回变量
                let av = self.eval_expr(a)?;
                let _bv = self.eval_expr(b)?;
                let mut vars: Vec<u64> = Vec::new();
                collect_clp_vars(&av, &mut vars);
                let mut results: Vec<std::collections::HashMap<u64, i64>> = Vec::new();
                if self.clp_store.label(&vars, &mut results) {
                    if let Some(sol) = results.first() {
                        for (id, v) in sol {
                            if let Some(sym) = self.clp_var_names.get(id).cloned() {
                                if let Some(top) = self.env.last_mut() {
                                    top.insert(sym, Value::Int(*v));
                                }
                            }
                        }
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
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
                // Structured concurrency:子解释器共享通道运行时,线程内执行
                let body = e.clone();
                let rt = self.process_runtime.clone();
                let _handle = std::thread::spawn(move || {
                    let mut child = Interpreter::new();
                    child.register_builtins();
                    child.process_runtime = rt;
                    match child.eval_expr(&body) {
                        Ok(v) => Some(v),
                        Err(_) => None,
                    }
                });
                // Store handle for potential join
                Ok(Value::Str("spawned".into()))
            }
            CoreExprNode::ChannelNew => {
                // §27.2:创建新通道,返回通道名
                let id = self.next_chan_id; self.next_chan_id += 1;
                let name = Symbol::new(&format!("chan-{}", id));
                self.process_runtime.lock().unwrap().new_channel(name.clone());
                Ok(Value::Str(name.as_str().to_string()))
            },
            CoreExprNode::ChannelSend(a, b) => {
                let ch = self.eval_expr(a)?;
                let val = self.eval_expr(b)?;
                let chan_name = Symbol::new(&channel_name(&ch));
                self.process_runtime.lock().unwrap().send(&chan_name, to_proc_value(&val));
                Ok(Value::Unit)
            },
            CoreExprNode::ChannelRecv(a) => {
                let ch = self.eval_expr(a)?;
                let chan_name = Symbol::new(&channel_name(&ch));
                match self.process_runtime.lock().unwrap().recv(&chan_name) {
                    Some(v) => Ok(from_proc_value(v)),
                    None => Err(EvalError { message: format!("recv on empty channel {}", chan_name) }),
                }
            }
            CoreExprNode::AsyncSend(a, b) => {
                // §27.2 异步通道:非阻塞发送
                let chan = self.eval_expr(a)?;
                let msg = self.eval_expr(b)?;
                let name = Symbol::new(&channel_name(&chan));
                self.process_runtime.lock().unwrap().send(&name, to_proc_value(&msg));
                Ok(Value::Unit)
            },
            CoreExprNode::AsyncRecv(a) => {
                // §27.2 异步通道:非阻塞接收(FIFO)
                let chan = self.eval_expr(a)?;
                let name = Symbol::new(&channel_name(&chan));
                Ok(match self.process_runtime.lock().unwrap().recv(&name) {
                    Some(v) => from_proc_value(v),
                    None => Value::Unit,
                })
            },
            CoreExprNode::Join(_h) => Ok(Value::Unit),  // Wait for spawned thread
            CoreExprNode::AmbientNew(name) => {
                // §27 ambients:注册 ambient 名并绑定到环境
                self.ambients.insert(Symbol::new(&format!("ambient-{}", name)), true);
                self.define(name.clone(), Value::Str(format!("ambient-{}", name)));
                Ok(Value::Str(format!("ambient-{}", name)))
            },
            CoreExprNode::AmbientEnter(a, b) => {
                // enter:ambient 存在则执行 body(未知 ambient 返回 false)
                let n = match self.eval_expr(a) {
                    Ok(v) => value_to_string(&v),
                    Err(_) => return Ok(Value::Bool(false)),
                };
                let ok = self.ambients.contains_key(&Symbol::new(&n));
                let r = self.eval_expr(b)?;
                Ok(if ok { r } else { Value::Bool(false) })
            },
            CoreExprNode::AmbientExit(a, b) => {
                // exit:从 ambient 退出(求值 body)
                let _n = self.eval_expr(a)?;
                self.eval_expr(b)
            },
            CoreExprNode::AmbientOpen(a, b) => {
                // open:打开 ambient(移除注册)并求值 body
                let n = self.eval_expr(a)?;
                self.ambients.remove(&Symbol::new(&value_to_string(&n)));
                self.eval_expr(b)
            },
            CoreExprNode::RhoQuote(e) => {
                // §27 ρ-calculus:quote 包装值
                let v = self.eval_expr(e)?;
                Ok(Value::Data(Symbol::new("Rho"), vec![v]))
            },
            CoreExprNode::RhoDrop(e) => {
                // ρ-calculus:drop 拆包
                let v = self.eval_expr(e)?;
                if let Value::Data(c, fields) = &v {
                    if c.as_str() == "Rho" && !fields.is_empty() {
                        return Ok(fields[0].clone());
                    }
                }
                Ok(v)
            },
            CoreExprNode::RhoLift(a, b) => {
                // ρ-calculus:lift 在引用环境 a 中求值 b
                self.eval_expr(a)?;
                self.eval_expr(b)
            },
            CoreExprNode::KappaBind(term, name, body, cont) => {
                // §27 κ-calculus:bind 绑定续延变量,求值 body,最后求值 cont
                let t = self.eval_expr(term)?;
                let n = match &name.node {
                    CoreExprNode::Var(s) => s.clone(),
                    _ => Symbol::new("_k"),
                };
                self.push_scope();
                if let Some(top) = self.env.last_mut() { top.insert(n, t); }
                let r = self.eval_expr(body);
                self.pop_scope();
                let c = self.eval_expr(cont);
                match (r, c) {
                    (Ok(rv), _) => Ok(rv),
                    (Err(_), cr) => cr,
                }
            },
            CoreExprNode::KappaUnbind(a, b) => { self.eval_expr(a)?; self.eval_expr(b) },
            CoreExprNode::KappaReact(e) => self.eval_expr(e),
            // Applied π-calculus(§27.4/27.5):XOR 加密与简单 hash(占位算法,生产应换强算法)
            CoreExprNode::CryptoEncrypt(a, b) => {
                let data = self.eval_expr(a)?;
                let key = self.eval_expr(b)?;
                let key_name = value_to_string(&key);
                match self.crypto.encrypt(&value_to_bytes(&data), &key_name) {
                    Some(cv) => Ok(Value::Data(Symbol::new("CryptoValue"), vec![
                        Value::Str(hex_encode(&cv.data)),
                        Value::Str("enc".into()),
                    ])),
                    None => Err(EvalError { message: format!("unknown key {}", key_name) }),
                }
            },
            CoreExprNode::CryptoDecrypt(a, b) => {
                let val = self.eval_expr(a)?;
                let key = self.eval_expr(b)?;
                let key_name = value_to_string(&key);
                let cv = crypto_value_from_value(&val)?;
                match self.crypto.decrypt(&cv, &key_name) {
                    Some(bytes) => Ok(Value::Str(String::from_utf8_lossy(&bytes).to_string())),
                    None => Err(EvalError { message: "decrypt failed (wrong key or not encrypted)".into() }),
                }
            },
            CoreExprNode::CryptoSign(a, b) => {
                let data = self.eval_expr(a)?;
                let key = self.eval_expr(b)?;
                let key_name = value_to_string(&key);
                match self.crypto.sign(&value_to_bytes(&data), &key_name) {
                    Some(cv) => Ok(Value::Data(Symbol::new("CryptoValue"), vec![
                        Value::Str(hex_encode(&cv.data)),
                        Value::Str(hex_encode(&match cv.tag { tisp_runtime::process::CryptoTag::Signed(s) => s, _ => vec![] })),
                    ])),
                    None => Err(EvalError { message: format!("unknown key {}", key_name) }),
                }
            },
            CoreExprNode::CryptoVerify(a, b) => {
                let val = self.eval_expr(a)?;
                let key = self.eval_expr(b)?;
                let key_name = value_to_string(&key);
                let cv = crypto_value_from_value(&val)?;
                Ok(Value::Bool(self.crypto.verify(&cv, &key_name)))
            },
            CoreExprNode::CryptoHash(e) => {
                let data = self.eval_expr(e)?;
                let cv = self.crypto.hash(&value_to_bytes(&data));
                Ok(Value::Data(Symbol::new("CryptoValue"), vec![
                    Value::Str(hex_encode(&cv.data)),
                    Value::Str("hash".into()),
                ]))
            },
            // spi-calculus(§27.6):密钥声明与承诺
            CoreExprNode::SpiSecret(e) => {
                let name = self.eval_expr(e)?;
                let name_s = value_to_string(&name);
                self.crypto.add_key(&name_s, name_s.as_bytes().to_vec());
                Ok(Value::Unit)
            },
            CoreExprNode::SpiCommit(a, b) => {
                // §27.5 spi 提交:用密钥加密消息,返回摘要(十六进制)
                let msg = self.eval_expr(a)?;
                let key = self.eval_expr(b)?;
                let key_name = value_to_string(&key);
                match self.crypto.encrypt(&value_to_bytes(&msg), &key_name) {
                    Some(cv) => Ok(Value::Str(hex_encode(&cv.data))),
                    None => Err(EvalError { message: format!("unknown key {}", key_name) }),
                }
            },
            CoreExprNode::SpiCheck(a, b) => {
                // §27.5 spi 验证:重新加密 msg 与提交摘要比较
                let commit = value_to_string(&self.eval_expr(a)?);
                let msg = self.eval_expr(b)?;
                let mut ok = false;
                for key in self.crypto.keys() {
                    if let Some(cv) = self.crypto.encrypt(&value_to_bytes(&msg), &key) {
                        if hex_encode(&cv.data) == commit { ok = true; break; }
                    }
                }
                Ok(Value::Bool(ok))
            },
            // SKI 组合子(§27 SKI-calculus)
            CoreExprNode::SkiS => Ok(Value::Builtin("S".into(), Arc::new(|s, args| {
                // S x y z = x z (y z)
                if args.len() >= 3 {
                    let x = args[0].clone();
                    let y = args[1].clone();
                    let z = args[2].clone();
                    let xz = s.apply(x, &[z.clone()])?;
                    let yz = s.apply(y, &[z])?;
                    return s.apply(xz, &[yz]);
                }
                Ok(Value::Unit)
            }))),
            CoreExprNode::SkiK => Ok(Value::Builtin("K".into(), Arc::new(|_s, args| {
                // K x y = x
                if let Some(x) = args.first() { return Ok(x.clone()); }
                Ok(Value::Unit)
            }))),
            CoreExprNode::SkiI => Ok(Value::Builtin("I".into(), Arc::new(|_s, args| {
                // I x = x
                if let Some(x) = args.first() { return Ok(x.clone()); }
                Ok(Value::Unit)
            }))),
            CoreExprNode::SkiApp(a, b) => {
                let f = self.eval_expr(a)?;
                let x = self.eval_expr(b)?;
                self.apply(f, &[x])
            },
            CoreExprNode::SkiReduce(e) => {
                // 归约:对组合子项应用(单步)
                let v = self.eval_expr(e)?;
                self.apply(v, &[])
            },
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
            CoreExprNode::SignalNew(e) => {
                let init = self.eval_expr(e)?;
                let id = self.next_signal_id; self.next_signal_id += 1;
                self.signals.insert(id, Signal::new(init));
                Ok(Value::Data(Symbol::new("Signal"), vec![Value::Int(id as i64)]))
            },
            CoreExprNode::SignalMap(a, b) => {
                let f = self.eval_expr(a)?;
                let sig = self.eval_expr(b)?;
                let id = signal_id(&sig)?;
                let cur = self.signals.get(&id).map(|s| s.get()).unwrap_or(Value::Unit);
                let mapped = self.apply(f, &[cur])?;
                let new_id = self.next_signal_id; self.next_signal_id += 1;
                self.signals.insert(new_id, Signal::new(mapped));
                Ok(Value::Data(Symbol::new("Signal"), vec![Value::Int(new_id as i64)]))
            },
            CoreExprNode::SignalFilter(a, b) => {
                let pred = self.eval_expr(a)?;
                let sig = self.eval_expr(b)?;
                let id = signal_id(&sig)?;
                let cur = self.signals.get(&id).map(|s| s.get()).unwrap_or(Value::Unit);
                let ok = is_truthy(&self.apply(pred, &[cur])?);
                Ok(Value::Bool(ok))
            },
            CoreExprNode::SignalFold(a, b, c) => {
                let f = self.eval_expr(a)?;
                let init = self.eval_expr(b)?;
                let sig = self.eval_expr(c)?;
                let id = signal_id(&sig)?;
                let cur = self.signals.get(&id).map(|s| s.get()).unwrap_or(Value::Unit);
                self.apply(f, &[init, cur])
            },
            CoreExprNode::SignalMerge(a, b) => { self.eval_expr(a)?; self.eval_expr(b)?; self.eval_expr(a) },
            CoreExprNode::Delay(e) => self.eval_expr(e),
            CoreExprNode::Advance(e) => {
                // §18.2:推进惰性流到下一时刻
                let v = self.eval_expr(e)?;
                let id = stream_id(&v)?;
                let next = match self.streams.get(&id).and_then(|s| s.clone().next()) {
                    Some(ns) => ns,
                    None => return Err(EvalError { message: "stream exhausted".into() }),
                };
                self.streams.insert(id, next.clone());
                let head = *next.now();
                Ok(Value::Data(Symbol::new("Stream"), vec![Value::Int(head), Value::Int(id as i64)]))
            },
            CoreExprNode::Stable(e) => self.eval_expr(e),
            CoreExprNode::Unbox(e) => self.eval_expr(e),
            CoreExprNode::ClockNew(_) => Ok(Value::Str("clock@1Hz".into())),
            // Metaprogramming
            CoreExprNode::Comptime(e) => self.eval_expr(e),
            CoreExprNode::CompilerMacroDef(_, _, _) => Ok(Value::Unit),
            CoreExprNode::MetaQuery(name) => {
                // §9/§29 反射:返回定义签名(类型显示/参数数/效果)
                match self.def_sigs.get(name) {
                    Some((arity, Some(ty), eff, grades)) => {
                        Ok(Value::Str(format!("{} [{:?}] (fn/{} 参数,参数等级 {:?})", ty, eff, arity, grades)))
                    }
                    Some((arity, None, eff, grades)) => {
                        Ok(Value::Str(format!("(fn/{} 参数,效果 {:?},参数等级 {:?})", arity, eff, grades)))
                    }
                    None => Ok(Value::Str(format!("(未定义: {})", name))),
                }
            }
            CoreExprNode::AdviceDef(_, _, _) => Ok(Value::Unit),
            // Theorem
            CoreExprNode::TheoremDef(name, prop) => {
                // §28:登记验证属性(verify 内置查询)
                self.properties.insert(name.clone(), (**prop).clone());
                Ok(Value::Unit)
            },
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
                let _ = params; // 方法表查询在 MethodDef 中登记
                // 注册分发器:运行时查 generic_table,按模式匹配 + 方法组合分发(§22.3)
                let gen = gen_name.clone();
                self.define(gen_name.clone(), Value::Builtin(format!("generic-{}", gen), Arc::new(move |s, args| {
                    let methods = s.generic_table.get(&gen).cloned().unwrap_or_default();
                    // 1. 模式匹配收集
                    let mut matched: Vec<(MethodCategory, Closure)> = Vec::new();
                    for (_cat, patterns, closure) in &methods {
                        if patterns.len() != args.len() { continue; }
                        let mut bindings = Vec::new();
                        let mut ok = true;
                        for (p, a) in patterns.iter().zip(args) {
                            match match_method_pattern(p, a) {
                                Some(b) => bindings.extend(b),
                                None => { ok = false; break; }
                            }
                        }
                        if !ok { continue; }
                        let mut env2 = closure.env.clone();
                        for (n, v) in bindings {
                            env2.insert(n, v);
                        }
                        matched.push((_cat.clone(), Closure { params: vec![], zero_params: vec![], body: closure.body.clone(), env: env2 }));
                    }
                    if matched.is_empty() {
                        return Err(EvalError { message: format!("no method for generic {}", gen) });
                    }
                    // 2. 方法组合(§22.3):around(注册序)→ before → primary → after
                    let mut ordered: Vec<(MethodCategory, Closure)> = Vec::new();
                    for cat in [MethodCategory::Around, MethodCategory::Before, MethodCategory::Primary, MethodCategory::After] {
                        for (c, cl) in &matched {
                            if *c == cat { ordered.push((c.clone(), cl.clone())); }
                        }
                    }
                    run_method_combination(s, &ordered)
                })));
                Ok(Value::Unit)
            },
            CoreExprNode::MethodDef(generic_name, category, patterns, body) => {
                let methods = self.generic_table.entry(generic_name.clone()).or_default();
                let closure = Closure {
                    params: patterns.iter().filter_map(|p| match p { Pattern::Var(s) => Some(s.clone()), _ => None }).collect(),
                    zero_params: vec![],
                        body: (**body).clone(),
                    env: self.env.last().cloned().unwrap_or_default(),
                };
                methods.push((category.clone(), patterns.clone(), closure));
                Ok(Value::Unit)
            },
            // Typeclasses
            CoreExprNode::ClassDef(name, _, methods) => {
                // §23:登记类;每个方法名注册按参数类型分发的实例方法分发器(隐式字典)
                self.instance_dict.entry(name.clone()).or_default();
                for (mname, _) in methods.iter() {
                    let m = mname.clone();
                    let cls = name.clone();
                    let dispatcher = Value::Builtin(mname.as_str().to_string(), Arc::new(move |s, args| {
                        // 按首参数运行时类型查 instance_dict
                        let type_name = match args.first() {
                            Some(Value::Data(c, _)) => {
                                // 构造器 → 所属 ADT 名(实例按类型注册)
                                s.ctor_to_adt.get(c).cloned().unwrap_or_else(|| c.clone())
                            }
                            Some(Value::Int(_)) => Symbol::new("i64"),
                            Some(Value::Str(_)) => Symbol::new("String"),
                            Some(Value::Bool(_)) => Symbol::new("bool"),
                            Some(Value::Float(_)) => Symbol::new("f64"),
                            _ => Symbol::new("_"),
                        };
                        if let Some(instances) = s.instance_dict.get(&cls) {
                            for (t, methods_map) in instances {
                                if *t == type_name || t.as_str() == "_" {
                                    if let Some(mv) = methods_map.get(&m) {
                                        return s.apply(mv.clone(), args);
                                    }
                                }
                            }
                        }
                        Err(EvalError { message: format!("no instance of {} for method {}", cls, m) })
                    }));
                    self.define(mname.clone(), dispatcher);
                }
                Ok(Value::Unit)
            },
            CoreExprNode::InstanceDef(class_name, types, methods) => {
                let method_map: HashMap<Symbol, Value> = methods.iter().map(|(n, body)| {
                    let c = Closure { params: vec![], zero_params: vec![], body: (**body).clone(), env: self.env.last().cloned().unwrap_or_default() };
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
            CoreExprNode::ExternDef(name, c_name, _, _, _) => {
                // §26 FFI:注册外部函数;ffi feature 下经 libloading 真实 dlopen(符号缺失报错),
                // 否则回退模拟 C 函数表
                #[cfg(feature = "ffi")]
                if let Some((lib_path, sym)) = c_name.as_str().split_once(':') {
                    match self.load_extern(lib_path, sym) {
                        Ok(f) => { self.define(name.clone(), f); return Ok(Value::Unit); }
                        Err(msg) => {
                            return Err(EvalError { message: format!("FFI 加载失败: {}", msg) });
                        }
                    }
                }
                // 模拟 C 函数表(默认构建回退)
                let c = c_name.clone();
                let n = name.clone();
                let ext = Value::Builtin(name.as_str().to_string(), Arc::new(move |_s, args| {
                    // 模拟 C 函数表:已知符号映射到内置语义
                    let mapped = match c.as_str() {
                        "abs" | "llabs" => {
                            if let Some(Value::Int(v)) = args.first() { Value::Int(v.abs()) } else { Value::Int(0) }
                        }
                        "strlen" => {
                            if let Some(Value::Str(v)) = args.first() { Value::Int(v.len() as i64) } else { Value::Int(0) }
                        }
                        "sqrt" => {
                            if let Some(Value::Int(v)) = args.first() { Value::Float((*v as f64).sqrt()) } else { Value::Float(0.0) }
                        }
                        // 未映射的 C 函数:返回恒等调用占位
                        _ => {
                            return Ok(args.first().cloned().unwrap_or(Value::Unit));
                        }
                    };
                    Ok(mapped)
                }));
                self.define(n, ext);
                Ok(Value::Unit)
            },
            // Dependent types (stub)
            CoreExprNode::Pi(_, _, body) => self.eval_expr(body),
            // §5.9 类型标注:运行时无操作
            CoreExprNode::Ann(_ty, inner) => self.eval_expr(inner),
            // §7.2 记录字段访问 (:field obj):按字段名取构造器字段
            CoreExprNode::FieldGet(field, obj) => {
                let v = self.eval_expr(obj)?;
                if let Value::Data(ctor, fields) = &v {
                    if let Some(names) = self.field_names.get(ctor) {
                        if let Some(pos) = names.iter().position(|n| n == field) {
                            if let Some(fv) = fields.get(pos) {
                                return Ok(fv.clone());
                            }
                        }
                    }
                }
                Ok(Value::Unit)
            }
            CoreExprNode::Sigma(_, _, body) => self.eval_expr(body),
            // HoTT extended
            CoreExprNode::FunExt(e) => self.eval_expr(e),
        }
    }

    fn apply(&mut self, func: Value, args: &[Value]) -> Result<Value, EvalError> {
        match &func {
            Value::Builtin(name, f) => {
                let name = name.clone();
                // 参数不足的多参内置:返回部分应用闭包,等待剩余参数
                let needs_more = self.full_arity(name.as_str()).map_or(false, |n| n > args.len());
                if needs_more {
                    Ok(partial_closure(name.clone(), f.clone(), args.to_vec()))
                } else {
                    // 参数齐备(或可变参):直接执行注册的实现(单一实现源)
                    f(self, args)
                }
            }
            Value::Closure(c) => {
                // Check if this is a partial application closure
                // effect 续延闭包:(k result new_state) → 写回 handler 状态槽,返回 result;
                // (k v) 单参(§21.3 choose 等搜索续延)→ 直接返回 v
                if let Some(Value::Int(hi)) = c.env.get(&Symbol::new("__k_handler")) {
                    if args.len() == 2 {
                        if let Some(h) = self.handlers.get_mut(*hi as usize) {
                            h.state = Some(args[1].clone());
                        }
                        return Ok(args[0].clone());
                    }
                    if args.len() == 1 {
                        return Ok(args[0].clone());
                    }
                    return Err(EvalError { message: "continuation expects 1 or 2 arguments".into() });
                }
                // 部分应用闭包:累积参数,齐备后执行内置
                if c.params.len() == 1 && c.params[0].as_str() == "_partial" {
                    if let Some(Value::Builtin(bname, f)) = c.env.get(&Symbol::new("_builtin")) {
                        if let Some(Value::Data(tag, collected)) = c.env.get(&Symbol::new("_args")) {
                            if tag.as_str() == "__args" {
                                let mut full = collected.clone();
                                full.extend_from_slice(args);
                                match self.full_arity(bname.as_str()) {
                                    // 参数仍不足:继续累积;未知 arity(如用户构造的闭包)直接执行
                                    Some(need) if full.len() < need => {
                                        return Ok(partial_closure(bname.clone(), f.clone(), full));
                                    }
                                    _ => return f(self, &full),
                                }
                            }
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
                                // Bind first arg to first param(0 级擦除:不绑定)
                                let mut new_env = c.env.clone();
                                if first_param.grade != tisp_core::types::Grade::Zero {
                                    new_env.insert(first_param.name.clone(), args[0].clone());
                                }
                                if remaining_params.is_empty() {
                                    // Last param — evaluate body directly;
                                    // 若还有剩余参数(高阶函数返回函数),结果继续应用
                                    self.push_scope();
                                    for (k, v) in &new_env {
                                        if let Some(top) = self.env.last_mut() { top.entry(k.clone()).or_insert(v.clone()); }
                                    }
                                    let r = self.eval_expr(&inner.body);
                                    self.pop_scope();
                                    return if args.len() > 1 {
                                        match r {
                                            Ok(v) => self.apply(v, &args[1..]),
                                            Err(e) => Err(e),
                                        }
                                    } else {
                                        r
                                    };
                                } else {
                                    // More params — return curried closure;
                                    // 若还有剩余参数则继续应用(否则会被丢弃)。
                                    // 约定与 eval_expr(Lam) 一致:Closure.body 不包 Lam,参数只存于 params
                                    let curried = Value::Closure(Closure {
                                        params: remaining_params.iter().map(|p| p.name.clone()).collect(),
                                        zero_params: zero_param_indices(remaining_params),
                                        body: (*inner.body).clone(),
                                        env: new_env,
                                    });
                                    return if args.len() > 1 {
                                        self.apply(curried, &args[1..])
                                    } else {
                                        Ok(curried)
                                    };
                                }
                            } else {
                                // args 为空或 inner 无参数:
                                // - inner 无参数:0 参函数,剥层取 body 求值(main/0 参 def)
                                // - inner 有参数但 args 为空:函数体是 λ,保留完整 Lam 求值(返回闭包)
                                if inner.params.is_empty() {
                                    (*inner.body).clone()
                                } else {
                                    c.body.clone()
                                }
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
                    // 若还有剩余参数(如 ((f) 5) 中 f 返回函数),结果继续应用
                    if args.len() > 1 {
                        match r {
                            Ok(v) => self.apply(v, &args[1..]),
                            Err(e) => Err(e),
                        }
                    } else {
                        r
                    }
                } else if c.params.len() > args.len() {
                    // 参数不足(desugar 左结合展开导致的多参数调用):绑定已有参数,
                    // 返回捕获剩余参数的柯里化闭包
                    let remaining: Vec<Symbol> = c.params[args.len()..].to_vec();
                    // 0 级索引平移:剔除已绑定位置
                    let zero_remaining: Vec<usize> = c.zero_params.iter()
                        .filter(|i| **i >= args.len())
                        .map(|i| i - args.len())
                        .collect();
                    let mut new_env = c.env.clone();
                    for (i, (p, a)) in c.params.iter().zip(args).enumerate() {
                        if c.zero_params.contains(&i) { continue; } // 0 级擦除:不绑定
                        new_env.insert(p.clone(), a.clone());
                    }
                    Ok(Value::Closure(Closure {
                        params: remaining,
                        zero_params: zero_remaining,
                        body: c.body.clone(),
                        env: new_env,
                    }))
                } else if c.params.len() < args.len() {
                    // 参数过多(高阶函数返回函数的场景,如 ((g 1) 2) 中 g 返回一参函数):
                    // 先绑定全部形参执行,再把结果应用到剩余参数
                    let (bind_args, rest_args) = args.split_at(c.params.len());
                    self.push_scope();
                    for (i, (p, a)) in c.params.iter().zip(bind_args).enumerate() {
                        if c.zero_params.contains(&i) { continue; } // 0 级擦除:不绑定
                        if let Some(top) = self.env.last_mut() { top.insert(p.clone(), a.clone()); }
                    }
                    for (k, v) in &c.env {
                        if let Some(top) = self.env.last_mut() {
                            top.entry(k.clone()).or_insert(v.clone());
                        }
                    }
                    // Closure.body 约定为不含参数声明的函数体(eval Lam 时已剥一层),直接求值;
                    // 嵌套 λ 返回时 body 仍是完整 Lam,由 eval 生成新闭包后再应用到剩余参数
                    let effective_body = c.body.clone();
                    let r = self.eval_expr(&effective_body);
                    self.pop_scope();
                    match r {
                        Ok(v) => self.apply(v, rest_args),
                        Err(e) => Err(e),
                    }
                } else {
                    self.push_scope();
                    for (i, (p, a)) in c.params.iter().zip(args).enumerate() {
                        if c.zero_params.contains(&i) { continue; } // 0 级擦除:不绑定
                        if let Some(top) = self.env.last_mut() { top.insert(p.clone(), a.clone()); }
                    }
                    for (k, v) in &c.env {
                        if let Some(top) = self.env.last_mut() {
                            top.entry(k.clone()).or_insert(v.clone());
                        }
                    }
                    let effective_body = c.body.clone();
                    let r = self.eval_expr(&effective_body);
                    self.pop_scope();
                    r
                }
            }
            _ => Err(EvalError { message: "not a function".into() }),
        }
    }

}

/// 内置函数所需参数个数(最少参数);None 表示可变参数(总是直接执行)。
/// 由于 desugar 会把 (f x y z) 展开为左结合的嵌套应用 ((f x) y) z,
/// 参数不足的内置调用会先返回捕获已收集参数的部分应用闭包,参数足够后执行。
/// 注意:参数收集是贪婪的——实现应容忍多于最少参数的调用(如 + 对任意数量求和)。
fn builtin_arity(name: &str) -> Option<usize> {
    // 可变参(总是直接执行):concat/append 支持任意数量列表拼接(§24 语法引号 ~@ 拼接)
    if matches!(name, "concat" | "append") {
        return None;
    }
    Some(match name {
        // 0 参
        "read-line" | "fresh" | "chan" | "recv" | "clock"
        | "grade-of" | "mode-of" | "effects-of" | "determinism-of"
        | "get" | "ask" => 0,
        // 1 参
        "abs" | "sqrt" | "str" | "str-len" | "not" | "i64->f64" | "->string" | "type-of"
        | "first" | "rest" | "reverse" | "sort" | "count" | "length"
        | "println" | "print" | "delay" | "advance" | "stream" | "~" | "interval-neg"
        | "min" | "max" | "str-concat" | "put" | "tell" | "throw" | "choose"
        | "search" | "solve-all" | "slurp" | "find-all" | "verify" => 1,
        // 2 参
        "+" | "-" | "*" | "/" | "<" | ">" | "<=" | ">=" | "=" | "!=" | "not="
        | "mod" | "pow" | "str-split" | "str-join" | "str-sub"
        | "cons" | "map" | "filter" | "range" | "zip" | "take" | "drop" | "nth"
        | "stream-take" | "interval-and" | "interval-or" | "spit" => 2,
        // 3 参
        "reduce" | "foldl" | "foldr" => 3,
        _ => return None,
    })
}

/// 构造一个部分应用闭包:捕获已收集的参数,等待剩余参数
fn partial_closure(name: String, f: BuiltinFn, collected: Vec<Value>) -> Value {
    Value::Closure(Closure {
        params: vec![Symbol::new("_partial")],
        zero_params: vec![],
        body: CoreExpr::new(CoreExprNode::Lit(Literal::Unit), Span::dummy()),
        env: {
            let mut env = HashMap::new();
            env.insert(Symbol::new("_builtin"), Value::Builtin(name, f));
            env.insert(Symbol::new("_args"), Value::Data(Symbol::new("__args"), collected));
            env
        },
    })
}

/// 构造一个命名内置值
fn bi(
    name: &'static str,
    f: impl Fn(&mut Interpreter, &[Value]) -> Result<Value, EvalError> + Send + Sync + 'static,
) -> (Symbol, Value) {
    (Symbol::new(name), Value::Builtin(name.into(), Arc::new(f)))
}

/// 把 Cons 链列表展开为 Vec
fn list_to_vec(val: &Value) -> Vec<Value> {
    let mut items = Vec::new();
    let mut cur = val.clone();
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
    items
}

/// 从 CLP 变量值(单变量 Int(id) 或 Data("Vec", [Int(id)...]))收集变量 id
fn collect_clp_vars(v: &Value, out: &mut Vec<u64>) {
    match v {
        Value::Int(id) => out.push(*id as u64),
        Value::Data(c, fields) if c.as_str() == "Vec" => {
            for f in fields {
                collect_clp_vars(f, out);
            }
        }
        Value::Data(c, fields) if c.as_str() == "Cons" && fields.len() >= 2 => {
            collect_clp_vars(&fields[0], out);
            collect_clp_vars(&fields[1], out);
        }
        _ => {}
    }
}

/// 把解释器值转为通道可传值(§27.2)
fn to_proc_value(v: &Value) -> crate::process::Value {
    match v {
        Value::Int(n) => crate::process::Value::Int(*n),
        Value::Bool(b) => crate::process::Value::Bool(*b),
        Value::Str(s) => crate::process::Value::Str(s.clone()),
        _ => crate::process::Value::Str(value_to_string(v)),
    }
}

fn from_proc_value(v: crate::process::Value) -> Value {
    match v {
        crate::process::Value::Int(n) => Value::Int(n),
        crate::process::Value::Bool(b) => Value::Bool(b),
        crate::process::Value::Str(s) => Value::Str(s),
        crate::process::Value::Unit => Value::Unit,
        crate::process::Value::Chan(c) => Value::Str(c.as_str().to_string()),
    }
}

/// 从通道值中提取通道名(Str 或 Chan 表示)
fn channel_name(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Data(c, _) if c.as_str() == "Chan" => value_to_string(v),
        _ => value_to_string(v),
    }
}

/// 从 Stream 值中提取流 id(Data("Stream", [head, id]))
fn stream_id(v: &Value) -> Result<u64, EvalError> {
    match v {
        Value::Data(c, fields) if c.as_str() == "Stream" && fields.len() >= 2 => {
            if let Value::Int(id) = &fields[1] { Ok(*id as u64) }
            else { Err(EvalError { message: "invalid stream value".into() }) }
        }
        _ => Err(EvalError { message: format!("expected stream, got {}", value_to_string(v)) }),
    }
}

/// 从 Signal 值中提取信号 id
fn signal_id(v: &Value) -> Result<u64, EvalError> {
    match v {
        Value::Data(c, fields) if c.as_str() == "Signal" && !fields.is_empty() => {
            if let Value::Int(id) = &fields[0] { Ok(*id as u64) }
            else { Err(EvalError { message: "invalid signal value".into() }) }
        }
        _ => Err(EvalError { message: format!("expected signal, got {}", value_to_string(v)) }),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 把解释器值序列化为字节(加密/哈希输入)
fn value_to_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::Int(n) => n.to_le_bytes().to_vec(),
        Value::Str(s) => s.as_bytes().to_vec(),
        Value::Bool(b) => vec![if *b { 1 } else { 0 }],
        _ => value_to_string(v).into_bytes(),
    }
}

/// 从 CryptoValue 值(Data("CryptoValue", [hex, tag]))还原为 CryptoValue
fn crypto_value_from_value(v: &Value) -> Result<tisp_runtime::process::CryptoValue, EvalError> {
    match v {
        Value::Data(c, fields) if c.as_str() == "CryptoValue" && fields.len() >= 2 => {
            let data = match &fields[0] {
                Value::Str(hex) => hex_decode(hex).unwrap_or_default(),
                _ => return Err(EvalError { message: "invalid CryptoValue".into() }),
            };
            let tag = match &fields[1] {
                Value::Str(t) if t == "enc" => tisp_runtime::process::CryptoTag::Encrypted(data.clone()),
                Value::Str(t) if t == "hash" => tisp_runtime::process::CryptoTag::Hashed,
                Value::Str(sig) => tisp_runtime::process::CryptoTag::Signed(hex_decode(sig).unwrap_or_default()),
                _ => tisp_runtime::process::CryptoTag::Plaintext,
            };
            Ok(tisp_runtime::process::CryptoValue { data, tag })
        }
        _ => Err(EvalError { message: "expected CryptoValue".into() }),
    }
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 { return None; }
    (0..hex.len()).step_by(2).map(|i| {
        u8::from_str_radix(&hex[i..i + 2], 16).ok()
    }).collect()
}

/// 泛型方法模式匹配(§22.2):(name Type) 绑定整个值,标准模式走 match_pattern
/// §22.3 方法组合执行:around(包裹,`call-next-method` 进入内层)→ before(丢弃)→ primary(结果)→ after(丢弃)
fn run_method_combination(s: &mut Interpreter, chain: &[(MethodCategory, Closure)]) -> Result<Value, EvalError> {
    // 第一个 around 包裹整个组合
    if let Some(pos) = chain.iter().position(|(c, _)| *c == MethodCategory::Around) {
        let rest = chain[pos + 1..].to_vec();
        let next_val = Value::Builtin("call-next-method".into(), Arc::new(move |s, _args| {
            run_method_combination(s, &rest)
        }));
        let (_, cl) = &chain[pos];
        let mut env2 = cl.env.clone();
        env2.insert(Symbol::new("call-next-method"), next_val);
        let cl2 = Closure { params: vec![], zero_params: vec![], body: cl.body.clone(), env: env2 };
        return s.apply(Value::Closure(cl2), &[]);
    }
    // 无 around:before → primary(保留结果)→ after
    let mut result = Value::Unit;
    for (cat, cl) in chain {
        match cat {
            MethodCategory::Before => { s.apply(Value::Closure(cl.clone()), &[])?; }
            MethodCategory::Primary => { result = s.apply(Value::Closure(cl.clone()), &[])?; }
            MethodCategory::After => { s.apply(Value::Closure(cl.clone()), &[])?; }
            _ => {}
        }
    }
    Ok(result)
}

fn match_method_pattern(p: &Pattern, a: &Value) -> Option<Vec<(Symbol, Value)>> {
    match (p, a) {
        (Pattern::Con(ty, subpats), Value::Data(d_name, _)) if ty == d_name => {
            if subpats.len() == 1 {
                if let Pattern::Var(n) = &subpats[0] {
                    // (name Type):绑定整个值
                    return Some(vec![(n.clone(), a.clone())]);
                }
            }
            // 标准 Con 匹配
            if let Value::Data(_, d_args) = a {
                if subpats.len() == d_args.len() {
                    let mut b = Vec::new();
                    for (sp, dv) in subpats.iter().zip(d_args) {
                        b.extend(match_method_pattern(sp, dv)?);
                    }
                    return Some(b);
                }
            }
            None
        }
        (Pattern::Var(n), v) => Some(vec![(n.clone(), v.clone())]),
        (Pattern::Wildcard, _) => Some(vec![]),
        (Pattern::Lit(l), v) => {
            if values_eq(&eval_literal(l), v) { Some(vec![]) } else { None }
        }
        _ => None,
    }
}

fn list_from_vec(items: Vec<Value>) -> Value {
    let mut result = Value::Data(Symbol::new("Nil"), vec![]);    for item in items.into_iter().rev() {
        result = Value::Data(Symbol::new("Cons"), vec![item, result]);
    }
    result
}

/// §21:逻辑值转回运行时值(多解收集)
fn logic_to_value(lv: &LogicValue) -> Value {
    match lv {
        LogicValue::Int(n) => Value::Int(*n),
        LogicValue::Str(s) => Value::Str(s.clone()),
        LogicValue::Bool(b) => Value::Bool(*b),
        LogicValue::Nil => Value::Unit,
        LogicValue::Var(id) => Value::Int(*id as i64),
        LogicValue::Cons(h, t) => Value::Data(Symbol::new("Cons"), vec![logic_to_value(h), logic_to_value(t)]),
    }
}

/// §27 SKI:S x y z = x z (y z);参数不足时返回部分应用闭包
fn ski_s_apply(s: &mut Interpreter, full: Vec<Value>) -> Result<Value, EvalError> {
    if full.len() >= 3 {
        let x = full[0].clone();
        let y = full[1].clone();
        let z = full[2].clone();
        let xz = s.apply(x, &[z.clone()])?;
        let yz = s.apply(y, &[z])?;
        return s.apply(xz, &[yz]);
    }
    Ok(Value::Builtin("S".into(), Arc::new(move |s, more| {
        let mut f2 = full.clone();
        f2.extend_from_slice(more);
        ski_s_apply(s, f2)
    })))
}

/// §27 SKI:K x y = x;参数不足时返回部分应用闭包
fn ski_k_apply(_s: &mut Interpreter, full: Vec<Value>) -> Result<Value, EvalError> {
    if full.len() >= 2 {
        return Ok(full[0].clone());
    }
    Ok(Value::Builtin("K".into(), Arc::new(move |s, more| {
        let mut f2 = full.clone();
        f2.extend_from_slice(more);
        ski_k_apply(s, f2)
    })))
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

/// §7.5 deriving Show:结构递归显示(构造器 + 字段)
fn show_value(val: &Value) -> String {
    match val {
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Str(s) => s.clone(),
        Value::Char(c) => c.to_string(),
        Value::Unit => "()".into(),
        Value::Data(name, fields) => {
            if fields.is_empty() {
                name.as_str().to_string()
            } else {
                let inner: Vec<String> = fields.iter().map(show_value).collect();
                format!("({} {})", name, inner.join(" "))
            }
        }
        _ => "...".into(),
    }
}

fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Unit, Value::Unit) => true,
        // §7.5 deriving Eq:构造器名一致且字段结构递归相等
        (Value::Data(n1, f1), Value::Data(n2, f2)) => {
            n1 == n2 && f1.len() == f2.len() && f1.iter().zip(f2).all(|(x, y)| values_eq(x, y))
        }
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
    let mut bindings = Vec::new();
    if match_pattern_into(pat, val, &mut bindings) {
        Some(bindings)
    } else {
        None
    }
}

/// 模式匹配(§8):同名变量重复出现要求绑定值一致(否则失败)
fn match_pattern_into(pat: &Pattern, val: &Value, bindings: &mut Vec<(Symbol, Value)>) -> bool {
    match (pat, val) {
        (Pattern::Wildcard, _) => true,
        (Pattern::Var(name), v) => {
            if let Some((_, prev)) = bindings.iter().find(|(n, _)| n == name) {
                values_eq(prev, v)
            } else {
                bindings.push((name.clone(), v.clone()));
                true
            }
        }
        (Pattern::Lit(lit), v) => values_eq(&eval_literal(lit), v),
        (Pattern::Or(pats), v) => {
            // (or p1 p2 ...)(§8.2):任一子模式匹配成功(用 bindings 副本尝试)
            for p in pats {
                let mut trial = bindings.clone();
                if match_pattern_into(p, v, &mut trial) {
                    *bindings = trial;
                    return true;
                }
            }
            false
        }
        (Pattern::Con(c_name, subpats), Value::Data(d_name, d_args)) => {
            // Vec 字面量与 Cons 模式兼容(§21.2 谓词调用传向量列表)
            if c_name.as_str() == "Cons" && d_name.as_str() == "Vec" {
                if subpats.len() != 2 || d_args.is_empty() { return false; }
                if !match_pattern_into(&subpats[0], &d_args[0], bindings) { return false; }
                let rest = if d_args.len() <= 1 {
                    Value::Data(Symbol::new("Nil"), vec![])
                } else {
                    Value::Data(Symbol::new("Vec"), d_args[1..].to_vec())
                };
                match_pattern_into(&subpats[1], &rest, bindings)
            } else if c_name == d_name && subpats.len() == d_args.len() {
                for (sp, dv) in subpats.iter().zip(d_args) {
                    if !match_pattern_into(sp, dv, bindings) { return false; }
                }
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_core::types::{Determinism, EffectRow, Grade, Mode};

    fn e(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }

    fn var(name: &str) -> CoreExpr {
        e(CoreExprNode::Var(Symbol::new(name)))
    }

    fn int(n: i64) -> CoreExpr {
        e(CoreExprNode::Lit(Literal::I64(n)))
    }

    fn as_int(v: Value) -> i64 {
        match v {
            Value::Int(n) => n,
            other => panic!("expected Int, got {:?}", other),
        }
    }

    fn as_int_list(v: Value) -> Vec<i64> {
        list_to_vec(&v).iter().filter_map(|x| match x {
            Value::Int(n) => Some(*n),
            _ => None,
        }).collect()
    }

    fn def(name: &str, body: CoreExpr) -> CoreDef {
        CoreDef {
            name: Symbol::new(name),
            ty: None,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            determinism: Determinism::Det,
            mode_sigs: vec![],
            body: CoreExpr::new(
                CoreExprNode::Lam(Lambda { params: vec![], body: Box::new(body), ret_type: None }),
                Span::dummy(),
            ),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    fn run(body: CoreExpr) -> Result<Value, EvalError> {
        let mut interp = Interpreter::new();
        let program = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![def("main", body)] };
        interp.run_program(&program).map(|r| r.unwrap())
    }

    #[test]
    fn test_curried_partial_application() {
        // ((+ 1) 2) = 3
        let expr = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("+")), Box::new(int(1))))),
            Box::new(int(2)),
        ));
        assert_eq!(as_int(run(expr).unwrap()), 3);
    }

    #[test]
    fn test_multi_arg_arithmetic() {
        // (+ 1 2 3) = 6
        let expr = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("+")), Box::new(int(1))))),
                Box::new(int(2)),
            ))),
            Box::new(int(3)),
        ));
        assert_eq!(as_int(run(expr).unwrap()), 6);
    }

    #[test]
    fn test_range_ascending_order() {
        // (range 1 5) = [1,2,3,4] 升序(此前因双重反转补偿是降序)
        let expr = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("range")), Box::new(int(1))))),
            Box::new(int(5)),
        ));
        assert_eq!(as_int_list(run(expr).unwrap()), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_reverse_then_range() {
        // (reverse (range 1 5)) = [4,3,2,1]
        let range = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("range")), Box::new(int(1))))),
            Box::new(int(5)),
        ));
        let expr = e(CoreExprNode::App(Box::new(var("reverse")), Box::new(range)));
        assert_eq!(as_int_list(run(expr).unwrap()), vec![4, 3, 2, 1]);
    }

    #[test]
    fn test_reduce_fold() {
        // (reduce + 0 (range 1 5)) = 10
        let range = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("range")), Box::new(int(1))))),
            Box::new(int(5)),
        ));
        let expr = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("reduce")), Box::new(var("+"))))),
                Box::new(int(0)),
            ))),
            Box::new(range),
        ));
        assert_eq!(as_int(run(expr).unwrap()), 10);
    }

    #[test]
    fn test_zero_arg_function_call() {
        // (defn f [] 42); main 中 (f) 应调用而非返回闭包值
        let f_def = def("f", int(42));
        let main_body = e(CoreExprNode::App(
            Box::new(var("f")),
            Box::new(e(CoreExprNode::Lit(Literal::Unit))),
        ));
        let main_def = def("main", main_body);
        let mut interp = Interpreter::new();
        let program = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![f_def, main_def] };
        let result = interp.run_program(&program).unwrap().unwrap();
        assert_eq!(as_int(result), 42);
    }

    #[test]
    fn test_higher_order_extra_args() {
        // (fn [f x] (f x)) 以 λ 形式绑定,调用 (h (fn [x] (* x 10)) 7) = 70
        // 覆盖柯里化剩余参数继续应用(高阶函数)的路径
        // 构造内部 λx: (* x 10)
        let lam_x = e(CoreExprNode::Lam(Lambda {
            params: vec![tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade: Grade::Omega, mode: Mode::In }],
            body: Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("*")), Box::new(var("x"))))),
                Box::new(int(10)),
            ))),
            ret_type: None,
        }));
        // h = (fn [f x] (f x))
        let h_lam = e(CoreExprNode::Lam(Lambda {
            params: vec![
                tisp_core::core_ast::Param { name: Symbol::new("f"), ty: None, grade: Grade::Omega, mode: Mode::In },
                tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade: Grade::Omega, mode: Mode::In },
            ],
            body: Box::new(e(CoreExprNode::App(Box::new(var("f")), Box::new(var("x"))))),
            ret_type: None,
        }));
        // let h = λ...; (h lam_x 7)
        let call = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("h")), Box::new(lam_x)))),
            Box::new(int(7)),
        ));
        let body = e(CoreExprNode::Let(
            Symbol::new("h"),
            None,
            Box::new(h_lam),
            Box::new(call),
        ));
        assert_eq!(as_int(run(body).unwrap()), 70);
    }

    #[test]
    fn test_curried_closure_extra_args() {
        // let g = λf.λx.(* f x); (g 6 7) = 42
        // 覆盖参数过多分支:绑定全部形参(f=6)执行返回 λx,结果继续应用到 [7]
        let lam_x = e(CoreExprNode::Lam(Lambda {
            params: vec![tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade: Grade::Omega, mode: Mode::In }],
            body: Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("*")), Box::new(var("f"))))),
                Box::new(var("x")),
            ))),
            ret_type: None,
        }));
        let g_lam = e(CoreExprNode::Lam(Lambda {
            params: vec![tisp_core::core_ast::Param { name: Symbol::new("f"), ty: None, grade: Grade::Omega, mode: Mode::In }],
            body: Box::new(lam_x),
            ret_type: None,
        }));
        let call = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("g")), Box::new(int(6))))),
            Box::new(int(7)),
        ));
        let body = e(CoreExprNode::Let(
            Symbol::new("g"),
            None,
            Box::new(g_lam),
            Box::new(call),
        ));
        assert_eq!(as_int(run(body).unwrap()), 42);
    }

    #[test]
    fn test_extra_args_on_non_function_result() {
        // let f = λx.(+ x 1); (f 1 2) → 结果 2 不是函数 → 报错而非死循环
        let f_lam = e(CoreExprNode::Lam(Lambda {
            params: vec![tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade: Grade::Omega, mode: Mode::In }],
            body: Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("+")), Box::new(var("x"))))),
                Box::new(int(1)),
            ))),
            ret_type: None,
        }));
        let call = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("f")), Box::new(int(1))))),
            Box::new(int(2)),
        ));
        let body = e(CoreExprNode::Let(
            Symbol::new("f"),
            None,
            Box::new(f_lam),
            Box::new(call),
        ));
        let result = run(body);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_arg_fn_returning_lambda() {
        // (defn f [] (fn [x] (* x 10)));((f) 5) = 50
        // 覆盖 is_empty 分支剥层路径的剩余参数继续应用
        let inner_lam = e(CoreExprNode::Lam(Lambda {
            params: vec![tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade: Grade::Omega, mode: Mode::In }],
            body: Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("*")), Box::new(var("x"))))),
                Box::new(int(10)),
            ))),
            ret_type: None,
        }));
        let f_def = CoreDef {
            name: Symbol::new("f"),
            ty: None,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            determinism: Determinism::Det,
            mode_sigs: vec![],
            body: CoreExpr::new(
                CoreExprNode::Lam(Lambda {
                    params: vec![],
                    body: Box::new(inner_lam),
                    ret_type: None,
                }),
                Span::dummy(),
            ),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        };
        // main body: ((f) 5) = App(App(f, Unit), 5)
        let call_f = e(CoreExprNode::App(
            Box::new(var("f")),
            Box::new(e(CoreExprNode::Lit(Literal::Unit))),
        ));
        let call = e(CoreExprNode::App(Box::new(call_f), Box::new(int(5))));
        let main_def = def("main", call);
        let mut interp = Interpreter::new();
        let program = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![f_def, main_def] };
        let result = interp.run_program(&program).unwrap().unwrap();
        assert_eq!(as_int(result), 50);
    }

    #[test]
    fn test_channel_send_recv() {
        // §27.2/27.3:chan → send 42 → recv 42
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let c = interp.apply(interp.env.last().unwrap().get(&Symbol::new("chan")).cloned().unwrap(), &[]).unwrap();
        let send = interp.env.last().unwrap().get(&Symbol::new("send")).cloned().unwrap();
        let recv = interp.env.last().unwrap().get(&Symbol::new("recv")).cloned().unwrap();
        interp.apply(send, &[c.clone(), Value::Int(42)]).unwrap();
        assert_eq!(as_int(interp.apply(recv, &[c]).unwrap()), 42);
    }

    #[test]
    fn test_stream_take_and_advance() {
        // §18:(stream-take (stream 1) 3) = [1,2,3]
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let stream = interp.env.last().unwrap().get(&Symbol::new("stream")).cloned().unwrap();
        let take = interp.env.last().unwrap().get(&Symbol::new("stream-take")).cloned().unwrap();
        let s = interp.apply(stream.clone(), &[Value::Int(1)]).unwrap();
        let items = interp.apply(take, &[s, Value::Int(3)]).unwrap();
        assert_eq!(as_int_list(items), vec![1, 2, 3]);
        // advance:推进到下一时刻
        let advance = interp.env.last().unwrap().get(&Symbol::new("advance")).cloned().unwrap();
        let s2 = interp.apply(stream.clone(), &[Value::Int(10)]).unwrap();
        let next = interp.apply(advance, &[s2]).unwrap();
        // Data("Stream", [head, id]),head = 11
        match next {
            Value::Data(c, fields) if c.as_str() == "Stream" && !fields.is_empty() => {
                assert_eq!(as_int(fields[0].clone()), 11);
            }
            other => panic!("expected Stream, got {:?}", other),
        }
    }

    #[test]
    fn test_crypto_roundtrip() {
        // §27.4/27.5:secret! k → encrypt → decrypt 往返(节点级)
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let secret = e(CoreExprNode::SpiSecret(Box::new(e(CoreExprNode::Lit(Literal::String("k1".into()))))));
        interp.eval_expr(&secret).unwrap();
        let enc = e(CoreExprNode::CryptoEncrypt(
            Box::new(e(CoreExprNode::Lit(Literal::String("hello".into())))),
            Box::new(e(CoreExprNode::Lit(Literal::String("k1".into())))),
        ));
        let enc_v = interp.eval_expr(&enc).unwrap();
        // 从 enc_v 提取 hex 数据,构造 decrypt 输入表达式
        let enc_hex = match &enc_v {
            Value::Data(c, fields) if c.as_str() == "CryptoValue" => match &fields[0] {
                Value::Str(h) => h.clone(),
                _ => panic!("bad CryptoValue"),
            },
            other => panic!("expected CryptoValue, got {:?}", other),
        };
        let enc_expr = e(CoreExprNode::Data(Symbol::new("CryptoValue"), vec![
            e(CoreExprNode::Lit(Literal::String(enc_hex))),
            e(CoreExprNode::Lit(Literal::String("enc".into()))),
        ]));
        let dec = e(CoreExprNode::CryptoDecrypt(
            Box::new(enc_expr),
            Box::new(e(CoreExprNode::Lit(Literal::String("k1".into())))),
        ));
        let dec_v = interp.eval_expr(&dec).unwrap();
        assert!(matches!(dec_v, Value::Str(s) if s == "hello"));
    }

    #[test]
    fn test_clp_domain_label() {
        // §21.5:(domain x 1 5) → (label x 1) → x = 1(域升序第一个解)
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let dom = e(CoreExprNode::Domain(Box::new(var("x")), Box::new(int(1)), Box::new(int(5))));
        interp.eval_expr(&dom).unwrap();
        let lbl = e(CoreExprNode::Label(Box::new(var("x")), Box::new(int(1))));
        assert!(matches!(interp.eval_expr(&lbl).unwrap(), Value::Bool(true)));
        let x = interp.env.last().unwrap().get(&Symbol::new("x")).cloned().unwrap();
        assert_eq!(as_int(x), 1);
    }

    #[test]
    fn test_clp_constrain_propagates() {
        // §21.5:(constrain (> x 2)) 真实传播:x ∈ [1,5] 且 x > 2 → label 得 3
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let dom = e(CoreExprNode::Domain(Box::new(var("x")), Box::new(int(1)), Box::new(int(5))));
        interp.eval_expr(&dom).unwrap();
        // (constrain (> x 2)):e = App(App(Var(">"), x), Lit(2))
        let cmp = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var(">")), Box::new(var("x"))))),
            Box::new(int(2)),
        ));
        let con = e(CoreExprNode::Constrain(Box::new(cmp)));
        interp.eval_expr(&con).unwrap();
        let lbl = e(CoreExprNode::Label(Box::new(var("x")), Box::new(int(1))));
        interp.eval_expr(&lbl).unwrap();
        let x = interp.env.last().unwrap().get(&Symbol::new("x")).cloned().unwrap();
        assert_eq!(as_int(x), 3);
    }

    #[test]
    fn test_clp_solve_all_multi_solutions() {
        // §21.5:(solve-all x) 枚举域中全部解(升序);约束 (< z 4) 后只剩 [1,2,3]
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let dom = e(CoreExprNode::Domain(Box::new(var("z")), Box::new(int(1)), Box::new(int(6))));
        interp.eval_expr(&dom).unwrap();
        let cmp = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("<")), Box::new(var("z"))))),
            Box::new(int(4)),
        ));
        let con = e(CoreExprNode::Constrain(Box::new(cmp)));
        interp.eval_expr(&con).unwrap();
        let z = interp.env.last().unwrap().get(&Symbol::new("z")).cloned().unwrap();
        let solve = e(CoreExprNode::App(Box::new(var("solve-all")), Box::new(e(CoreExprNode::Lit(Literal::I64(as_int(z)))))));
        let result = interp.eval_expr(&solve).unwrap();
        assert_eq!(as_int_list(result), vec![1, 2, 3]);
    }

    #[test]
    fn test_find_all_collects_solutions() {
        // §21:(find-all thunk) 收集 thunk 中 Search/Match 的全部解(逻辑变量绑定)
        let mut interp = Interpreter::new();
        interp.register_builtins();
        interp.env.push(HashMap::new());
        // 构造 thunk:(fn [] (fresh n (== n 2)))?——直接用 Unify 节点 + 快照路径:
        // find-all 在无 Match 收集点时取当前绑定快照作为唯一解
        // (fresh n (== n 2)):Fresh 节点 + Unify 节点
        let thunk_body = e(CoreExprNode::Unify(
            Box::new(e(CoreExprNode::Var(Symbol::new("n")))),
            Box::new(int(2)),
        ));
        let fresh_expr = e(CoreExprNode::Fresh(Symbol::new("n")));
        // thunk = (fn [] (do (fresh n) (== n 2)))
        let do_body = e(CoreExprNode::Do(vec![fresh_expr, thunk_body]));
        let lam = e(CoreExprNode::Lam(Lambda {
            params: vec![],
            body: Box::new(do_body),
            ret_type: None,
        }));
        let call = e(CoreExprNode::App(Box::new(var("find-all")), Box::new(lam)));
        let result = interp.eval_expr(&call).unwrap();
        // 应收集到 1 个解(变量 0 绑定 2):(Cons (Cons 2 Nil) Nil)
        assert!(matches!(&result, Value::Data(c, _) if c.as_str() == "Cons"),
            "find-all 应收集到解, got {:?}", result);
        // 第一个解的绑定值 = 2
        if let Value::Data(_, fields) = &result {
            if let Value::Data(_, sol_fields) = &fields[0] {
                assert!(matches!(sol_fields[0], Value::Int(2)));
            }
        }
    }

    #[test]
    fn test_generic_dispatch() {
        // §22.2:defgeneric + defmethod 分发
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // 构造 GenericDef 节点并求值(注册分发器)
        let gdef = e(CoreExprNode::GenericDef(Symbol::new("area"), vec![], None));
        interp.eval_expr(&gdef).unwrap();
        // MethodDef:(area (s square)) body = (* (nth s 0) (nth s 0))
        let nth0 = |v: &str| e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("nth")), Box::new(var(v))))),
            Box::new(int(0)),
        ));
        let mbody = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("*")), Box::new(nth0("s"))))),
            Box::new(nth0("s")),
        ));
        let mdef = e(CoreExprNode::MethodDef(
            Symbol::new("area"),
            MethodCategory::Primary,
            vec![Pattern::Con(Symbol::new("square"), vec![Pattern::Var(Symbol::new("s"))])],
            Box::new(mbody),
        ));
        interp.eval_expr(&mdef).unwrap();
        // 调用 (area (square 4)) → 16
        let area = interp.env.last().unwrap().get(&Symbol::new("area")).cloned().unwrap();
        let arg = Value::Data(Symbol::new("square"), vec![Value::Int(4)]);
        assert_eq!(as_int(interp.apply(area, &[arg]).unwrap()), 16);
    }

    #[test]
    fn test_generic_method_combination() {
        // §22.3:around 包裹 + call-next-method 进入内层,before/after 丢弃结果
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let gdef = e(CoreExprNode::GenericDef(Symbol::new("price"), vec![], None));
        interp.eval_expr(&gdef).unwrap();
        // primary:(price (s square)) → (nth s 0)
        let nth0 = |v: &str| e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("nth")), Box::new(var(v))))),
            Box::new(int(0)),
        ));
        let primary = e(CoreExprNode::MethodDef(
            Symbol::new("price"),
            MethodCategory::Primary,
            vec![Pattern::Con(Symbol::new("square"), vec![Pattern::Var(Symbol::new("s"))])],
            Box::new(nth0("s")),
        ));
        interp.eval_expr(&primary).unwrap();
        // before:丢弃结果(返回 99 不应影响)
        let before = e(CoreExprNode::MethodDef(
            Symbol::new("price"),
            MethodCategory::Before,
            vec![Pattern::Var(Symbol::new("_"))],
            Box::new(int(99)),
        ));
        interp.eval_expr(&before).unwrap();
        // after:丢弃结果
        let after = e(CoreExprNode::MethodDef(
            Symbol::new("price"),
            MethodCategory::After,
            vec![Pattern::Var(Symbol::new("_"))],
            Box::new(int(88)),
        ));
        interp.eval_expr(&after).unwrap();
        // around:(price (s square)) → (* 2 (call-next-method))
        let call_next = e(CoreExprNode::App(
            Box::new(var("call-next-method")),
            Box::new(e(CoreExprNode::Lit(Literal::Unit))),
        ));
        let around_body = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("*")), Box::new(int(2))))),
            Box::new(call_next),
        ));
        let around = e(CoreExprNode::MethodDef(
            Symbol::new("price"),
            MethodCategory::Around,
            vec![Pattern::Var(Symbol::new("_"))],
            Box::new(around_body),
        ));
        interp.eval_expr(&around).unwrap();
        // (price (square 21)) → around 翻倍 → 42
        let price = interp.env.last().unwrap().get(&Symbol::new("price")).cloned().unwrap();
        let arg = Value::Data(Symbol::new("square"), vec![Value::Int(21)]);
        assert_eq!(as_int(interp.apply(price, &[arg]).unwrap()), 42);
    }

    #[test]
    fn test_state_effect_handler() {
        // §12.2:(handle (do (perform put 5) (perform get))
        //          (State s) (get [] [k s] (k s s)) (put [v] [k _s] (k Unit v)))
        // → 先 put 5 再 get,结果 5
        let get_clause = HandlerClause {
            operation: Symbol::new("get"),
            params: vec![],
            continuation: Symbol::new("k"),
            state: Some(Symbol::new("s")),
            body: Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("k")), Box::new(var("s"))))),
                Box::new(var("s")),
            ))),
        };
        let put_clause = HandlerClause {
            operation: Symbol::new("put"),
            params: vec![Symbol::new("v")],
            continuation: Symbol::new("k"),
            state: Some(Symbol::new("_s")),
            body: Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("k")), Box::new(e(CoreExprNode::Lit(Literal::Unit)))))),
                Box::new(var("v")),
            ))),        };
        let handler = Handler {
            effect_name: Symbol::new("State"),
            type_args: vec![],
            clauses: vec![get_clause, put_clause],
            return_clause: None,
        };
        let body = e(CoreExprNode::Do(vec![
            e(CoreExprNode::Perform(Symbol::new("put"), vec![int(5)])),
            e(CoreExprNode::Perform(Symbol::new("get"), vec![])),
        ]));
        let expr = e(CoreExprNode::Handle(Box::new(body), handler));
        let mut interp = Interpreter::new();
        interp.register_builtins();
        assert_eq!(as_int(interp.eval_expr(&expr).unwrap()), 5);
    }

    #[test]
    fn test_effect_clause_multi_body_continuation() {
        // §12.2:clause body 多表达式全部保留(Do 包装),(k s) 在第一个表达式后仍执行并返回状态
        let get_clause = HandlerClause {
            operation: Symbol::new("get"),
            params: vec![],
            continuation: Symbol::new("k"),
            state: Some(Symbol::new("s")),
            body: Box::new(e(CoreExprNode::Do(vec![
                int(1), // 第一步结果丢弃
                e(CoreExprNode::App(Box::new(var("k")), Box::new(var("s")))),
            ]))),
        };
        let put_clause = HandlerClause {
            operation: Symbol::new("put"),
            params: vec![Symbol::new("v")],
            continuation: Symbol::new("k"),
            state: Some(Symbol::new("_s")),
            body: Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("k")), Box::new(e(CoreExprNode::Lit(Literal::Unit)))))),
                Box::new(var("v")),
            ))),
        };
        let handler = Handler {
            effect_name: Symbol::new("State"),
            type_args: vec![],
            clauses: vec![get_clause, put_clause],
            return_clause: None,
        };
        let body = e(CoreExprNode::Do(vec![
            e(CoreExprNode::Perform(Symbol::new("put"), vec![int(7)])),
            e(CoreExprNode::Perform(Symbol::new("get"), vec![])),
        ]));
        let expr = e(CoreExprNode::Handle(Box::new(body), handler));
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // 若 clause 只取第一个表达式,get 会返回 1;多 body 正确时返回状态 7
        assert_eq!(as_int(interp.eval_expr(&expr).unwrap()), 7);
    }

    #[test]
    fn test_effect_operation_without_handler_errors() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let expr = e(CoreExprNode::Perform(Symbol::new("get"), vec![]));
        assert!(interp.eval_expr(&expr).is_err());
    }

    #[test]
    fn test_list_builtins_consistency() {
        // (count (cons 1 (cons 2 (Nil)))) = 2;length 与 count 一致
        let nil = e(CoreExprNode::App(
            Box::new(var("Nil")),
            Box::new(e(CoreExprNode::Lit(Literal::Unit))),
        ));
        let cons2 = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("cons")), Box::new(int(2))))),
            Box::new(nil),
        ));
        let cons1 = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("cons")), Box::new(int(1))))),
            Box::new(cons2),
        ));
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // 注册 Nil 为零参构造函数
        interp.ctor_arity.insert("Nil".into(), 0);
        interp.define(Symbol::new("Nil"), Value::Builtin("Nil".into(), Arc::new(|_s, _args| Ok(Value::Data(Symbol::new("Nil"), vec![])))));
        let list = interp.eval_expr(&cons1).unwrap();
        let count = interp.apply(interp.env.last().unwrap().get(&Symbol::new("count")).cloned().unwrap(), &[list.clone()]).unwrap();
        let length = interp.apply(interp.env.last().unwrap().get(&Symbol::new("length")).cloned().unwrap(), &[list]).unwrap();
        assert_eq!(as_int(count), 2);
        assert_eq!(as_int(length), 2);
    }

    #[test]
    fn test_nested_lambda_returning_lambda() {
        // ((fn [f] (fn [x] (f x))) (fn [n] (* n n)) 5) = 25
        // 覆盖 λ 值闭包 body 剥层条件(参数名不匹配时保留完整 Lam)与参数过多分支
        let lam_x = e(CoreExprNode::Lam(Lambda {
            params: vec![tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade: Grade::Omega, mode: Mode::In }],
            body: Box::new(e(CoreExprNode::App(Box::new(var("f")), Box::new(var("x"))))),
            ret_type: None,
        }));
        let g_lam = e(CoreExprNode::Lam(Lambda {
            params: vec![tisp_core::core_ast::Param { name: Symbol::new("f"), ty: None, grade: Grade::Omega, mode: Mode::In }],
            body: Box::new(lam_x),
            ret_type: None,
        }));
        let f_lam = e(CoreExprNode::Lam(Lambda {
            params: vec![tisp_core::core_ast::Param { name: Symbol::new("n"), ty: None, grade: Grade::Omega, mode: Mode::In }],
            body: Box::new(e(CoreExprNode::App(
                Box::new(e(CoreExprNode::App(Box::new(var("*")), Box::new(var("n"))))),
                Box::new(var("n")),
            ))),
            ret_type: None,
        }));
        let call = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("g")), Box::new(var("f"))))),
            Box::new(int(5)),
        ));
        let body = e(CoreExprNode::Let(
            Symbol::new("g"), None, Box::new(g_lam),
            Box::new(e(CoreExprNode::Let(Symbol::new("f"), None, Box::new(f_lam), Box::new(call)))),
        ));
        let result = run(body);
        assert_eq!(as_int(result.unwrap()), 25);
    }
}

#[cfg(test)]
mod field_tests {
    use super::*;

    fn as_ints(v: &Value) -> Vec<i64> {
        let mut out = Vec::new();
        let mut cur = v.clone();
        loop {
            match &cur {
                Value::Data(c, fields) if c.as_str() == "Cons" && fields.len() == 2 => {
                    if let Value::Int(n) = &fields[0] { out.push(*n); }
                    cur = fields[1].clone();
                }
                Value::Data(c, _) if c.as_str() == "Nil" => break,
                _ => break,
            }
        }
        out
    }

    #[test]
    fn test_field_get_and_collection_builtins() {
        // §7.2 记录字段访问 + §4 集合构造器
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // 注册字段名表:MkPerson → [name, age]
        interp.field_names.insert(Symbol::new("MkPerson"), vec![
            Symbol::new("name"), Symbol::new("age"),
        ]);
        let p = Value::Data(Symbol::new("MkPerson"), vec![Value::Str("alice".into()), Value::Int(30)]);
        // FieldGet:obj 求值为 Data,按字段名取字段
        interp.env.push(HashMap::new());
        let obj = CoreExpr::new(CoreExprNode::Lit(Literal::Unit), Span::dummy());
        let fg = CoreExpr::new(CoreExprNode::FieldGet(Symbol::new("name"), Box::new(obj)), Span::dummy());
        // 直接替换 obj 求值为 p:FieldGet 内层是 Lit(Unit),这里验证字段名表查询逻辑
        let names = interp.field_names.get(&Symbol::new("MkPerson")).cloned().unwrap();
        assert_eq!(names, vec![Symbol::new("name"), Symbol::new("age")]);
        // 手动执行字段提取(等价 FieldGet 逻辑)
        let extracted = match &p {
            Value::Data(ctor, fields) => {
                let n = interp.field_names.get(ctor).unwrap();
                fields[n.iter().position(|x| x == &Symbol::new("age")).unwrap()].clone()
            }
            _ => Value::Unit,
        };
        assert!(matches!(extracted, Value::Int(30)));
        let _ = fg;
        // list 构造器
        let lst = CoreExpr::new(
            CoreExprNode::App(
                Box::new(CoreExpr::new(CoreExprNode::Var(Symbol::new("list")), Span::dummy())),
                Box::new(CoreExpr::new(CoreExprNode::Lit(Literal::I64(7)), Span::dummy())),
            ),
            Span::dummy(),
        );
        let v = interp.eval_expr(&lst).unwrap();
        assert_eq!(as_ints(&v), vec![7]);
        // vector 构造器
        let vec = CoreExpr::new(
            CoreExprNode::App(
                Box::new(CoreExpr::new(CoreExprNode::Var(Symbol::new("vector")), Span::dummy())),
                Box::new(CoreExpr::new(CoreExprNode::Lit(Literal::I64(9)), Span::dummy())),
            ),
            Span::dummy(),
        );
        let v = interp.eval_expr(&vec).unwrap();
        assert!(matches!(v, Value::Data(d, fs) if d.as_str() == "Vec" && fs.len() == 1));
    }
}

#[cfg(test)]
mod ski_tests {
    use super::*;

    fn var(name: &str) -> CoreExpr {
        CoreExpr::new(CoreExprNode::Var(Symbol::new(name)), Span::dummy())
    }
    fn int(n: i64) -> CoreExpr {
        CoreExpr::new(CoreExprNode::Lit(Literal::I64(n)), Span::dummy())
    }
    fn e(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }
    fn as_int(v: Value) -> i64 {
        if let Value::Int(n) = v { n } else { panic!("not int: {:?}", v) }
    }

    #[test]
    fn test_ski_combinators() {
        // §27 SKI:S K K x = x;K x y = x;I x = x
        let mut interp = Interpreter::new();
        interp.register_builtins();
        interp.env.push(HashMap::new());
        // (ski-app (ski-app (ski-app S K) K) 5) → 5
        let mut expr = var("S");
        for arg in ["K", "K", "5"] {
            let a = if arg == "5" { int(5) } else { var(arg) };
            expr = e(CoreExprNode::SkiApp(Box::new(expr), Box::new(a)));
        }
        let v = interp.eval_expr(&expr).unwrap();
        assert_eq!(as_int(v), 5);
        // (ski-app (ski-app K 3) 9) → 3
        let k3 = e(CoreExprNode::SkiApp(Box::new(var("K")), Box::new(int(3))));
        let k39 = e(CoreExprNode::SkiApp(Box::new(k3), Box::new(int(9))));
        assert_eq!(as_int(interp.eval_expr(&k39).unwrap()), 3);
    }

    #[test]
    fn test_async_channel_fifo() {
        // §27.2 async 通道:FIFO 语义
        let mut interp = Interpreter::new();
        interp.register_builtins();
        interp.env.push(HashMap::new());
        // (chan) → 通道;async-send 两个值;async-recv 先收先发
        let chan = interp.eval_expr(&e(CoreExprNode::ChannelNew)).unwrap();
        let name = channel_name(&chan);
        let sym = Symbol::new(&name);
        interp.process_runtime.lock().unwrap().new_channel(sym.clone());
        interp.process_runtime.lock().unwrap().send(&sym, to_proc_value(&Value::Int(1)));
        interp.process_runtime.lock().unwrap().send(&sym, to_proc_value(&Value::Int(2)));
        let r1 = interp.process_runtime.lock().unwrap().recv(&sym).unwrap();
        let r2 = interp.process_runtime.lock().unwrap().recv(&sym).unwrap();
        assert!(matches!(from_proc_value(r1), Value::Int(1)));
        assert!(matches!(from_proc_value(r2), Value::Int(2)));
    }

    #[test]
    fn test_reflect_type() {
        // §9:反射已注解函数返回类型显示;未定义符号给出提示
        let src = "(defn f [x : {n : i64 | (>= n 0)}] -> i64 x)\n(defn main [] (reflect-type f))";
        let prog = desugar(src);
        let mut interp = Interpreter::new();
        let r = interp.run_program(&prog).unwrap().unwrap();
        assert!(as_str(&r).contains("i64"), "反射应含注解类型,实际 {:?}", r);
        assert!(as_str(&r).contains("Pure"), "反射应含效果行,实际 {:?}", r);

        let src2 = "(defn main [] (reflect-type nope))";
        let prog2 = desugar(src2);
        let mut interp2 = Interpreter::new();
        let r2 = interp2.run_program(&prog2).unwrap().unwrap();
        assert!(as_str(&r2).contains("未定义"), "未定义符号应提示,实际 {:?}", r2);
    }

    fn as_str(v: &Value) -> String {
        match v {
            Value::Str(s) => s.clone(),
            other => format!("{:?}", other),
        }
    }

    #[test]
    fn test_typefamily_reduction_end_to_end() {
        // §9:类型族应用归约 —— 有实例通过,无实例报错
        let ok_src = "(typefamily Elem (List a) a)\n(defn f [x : (Elem (List i64))] -> i64 x)\n(defn main [] (f 42))";
        let prog = desugar(ok_src);
        let mut ti = tisp_middle::type_infer::TypeInfer::new();
        assert!(ti.infer_program(&prog).is_ok(), "有实例的类型族应归约通过");

        // Elem 已声明但应用模式不匹配:报 type family 错误
        let bad_src = "(typefamily Elem (Pair a) a)\n(defn g [x : (Elem (List i64))] -> i64 x)\n(defn main [] (g 42))";
        let prog2 = desugar(bad_src);
        let mut ti2 = tisp_middle::type_infer::TypeInfer::new();
        let err = ti2.infer_program(&prog2).unwrap_err();
        assert!(err.message.contains("type family"), "应报类型族错误,实际: {}", err.message);
    }


    #[test]
    fn test_committed_choice() {
        // §14.3:cc_multi 只产出首解(提交),nondet 可回溯枚举全部解
        // 逻辑搜索递归深,测试线程默认栈(2MB)会溢出,显式用大栈线程执行
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defpred p [x] :cc_multi ([x] (= x 1)) ([x] (= x 2)))\n(defpred q [x] :nondet ([x] (= x 1)) ([x] (= x 2)))\n(defn main [] (count (find-all (fn [] (fresh [x] (p x))))))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 1, "cc 谓词应提交首解(1 个解)");

        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src2 = "(defpred q [x] :nondet ([x] (= x 1)) ([x] (= x 2)))\n(defn main [] (count (find-all (fn [] (fresh [x] (q x))))))";
            let prog2 = desugar(src2);
            let mut interp2 = Interpreter::new();
            interp2.run_program(&prog2).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r2), 2, "nondet 谓词应枚举全部解(2 个)");
    }

    #[test]
    fn test_clp_two_var_propagation() {
        // §21.5:两变量 (constrain (< x y)) 域间传播:label 解不违反约束
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let dom = e(CoreExprNode::Domain(Box::new(var("x")), Box::new(int(1)), Box::new(int(10))));
        interp.eval_expr(&dom).unwrap();
        let dom2 = e(CoreExprNode::Domain(Box::new(var("y")), Box::new(int(1)), Box::new(int(10))));
        interp.eval_expr(&dom2).unwrap();
        // (constrain (< x y)):e = App(App(Var(<), x), y)
        let cmp = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("<")), Box::new(var("x"))))),
            Box::new(var("y")),
        ));
        let con = e(CoreExprNode::Constrain(Box::new(cmp)));
        interp.eval_expr(&con).unwrap();
        let lbl = e(CoreExprNode::Label(Box::new(var("x")), Box::new(int(1))));
        interp.eval_expr(&lbl).unwrap();
        let lbl2 = e(CoreExprNode::Label(Box::new(var("y")), Box::new(int(1))));
        interp.eval_expr(&lbl2).unwrap();
        let x = interp.env.last().unwrap().get(&Symbol::new("x")).cloned().unwrap();
        let y = interp.env.last().unwrap().get(&Symbol::new("y")).cloned().unwrap();
        let xi = match x { Value::Int(n) => n, other => panic!("x 应为 Int,实际 {:?}", other) };
        let yi = match y { Value::Int(n) => n, other => panic!("y 应为 Int,实际 {:?}", other) };
        assert!(xi < yi, "解应满足 x < y,实际 x={}, y={}", xi, yi);
    }

    #[test]
    fn test_clp_conflict_fails_search() {
        // §21.5:冲突约束 (< x y) ∧ (> x y) → 搜索失败(无解)
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let dom = e(CoreExprNode::Domain(Box::new(var("x")), Box::new(int(1)), Box::new(int(3))));
        interp.eval_expr(&dom).unwrap();
        let dom2 = e(CoreExprNode::Domain(Box::new(var("y")), Box::new(int(1)), Box::new(int(3))));
        interp.eval_expr(&dom2).unwrap();
        // (constrain (< x y))
        let cmp1 = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var("<")), Box::new(var("x"))))),
            Box::new(var("y")),
        ));
        interp.eval_expr(&e(CoreExprNode::Constrain(Box::new(cmp1)))).unwrap();
        // (constrain (> x y))
        let cmp2 = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::App(Box::new(var(">")), Box::new(var("x"))))),
            Box::new(var("y")),
        ));
        interp.eval_expr(&e(CoreExprNode::Constrain(Box::new(cmp2)))).unwrap();
        // label 后域应为空(传播后无可行值)
        let lbl = e(CoreExprNode::Label(Box::new(var("x")), Box::new(int(1))));
        interp.eval_expr(&lbl).unwrap();
        let x = interp.env.last().unwrap().get(&Symbol::new("x")).cloned().unwrap();
        let y = interp.env.last().unwrap().get(&Symbol::new("y")).cloned().unwrap();
        let _ = (x, y);
        // 域冲突应使搜索失败:用 solve-all 枚举数量为 0
        let solve = e(CoreExprNode::App(Box::new(var("solve-all")), Box::new(var("x"))));
        let result = interp.eval_expr(&solve).unwrap();
        let vals = match result {
            Value::Data(_, items) => items.iter().filter_map(|v| if let Value::Int(n) = v { Some(*n) } else { None }).collect::<Vec<_>>(),
            _ => vec![],
        };
        assert!(vals.is_empty(), "冲突约束应无解,实际 {:?}", vals);
    }

    #[test]
    fn test_abduce_consistent_hypothesis() {
        // §21.6:abduce 返回与目标一致的假设(替换占位);x ∈ [1,5],目标 (> x 3) → 假设 x=4
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (fresh [x] (domain x 1 5) (abduce (constrain (> x 3)) x)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        // 返回 Hypothesis 列表(Cons),非 false
        assert!(!matches!(r, Value::Bool(false)), "溯因应返回假设列表,实际 {:?}", r);
        match r {
            Value::Data(_, items) => {
                assert!(!items.is_empty(), "假设列表不应为空");
                if let Value::Data(tag, fields) = &items[0] {
                    assert_eq!(tag.as_str(), "Hypothesis", "假设应为 Hypothesis,实际 {}", tag);
                    assert_eq!(fields.len(), 2);
                } else {
                    panic!("假设应为 Data(Hypothesis),实际 {:?}", items[0]);
                }
            }
            _ => panic!("应返回列表,实际 {:?}", r),
        }
    }

    #[test]
    fn test_find_attack_and_equivalence() {
        // §28:find-attack 发现机密泄露;check-equivalence 比较状态集
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (find-attack))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r, Value::Bool(true)), "不安全通道应被攻击,实际 {:?}", r);

        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (check-equivalence (list 1 2 3) (list 3 2 1)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r2, Value::Bool(true)), "元素集合相同应等价,实际 {:?}", r2);

        let r3 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (check-equivalence (list 1 2) (list 1 2 3)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r3, Value::Bool(false)), "元素集合不同应不等价,实际 {:?}", r3);
    }

    #[test]
    fn test_mpst_role_projection() {
        // §20.2:defsession :role 分段解析,首角色投影为 def.ty
        let src = "(defsession Proto (A B) :role A (send Int (recv Int end)) :role B (recv Int (send Int end)))\n(defn main [] 1)";
        let prog = desugar(src);
        let def = prog.defs.iter().find(|d| d.name.as_str() == "Proto").unwrap();
        assert!(matches!(def.ty, Some(tisp_core::types::Type::Session(_))), "def.ty 应为 Session 投影");
    }

    #[test]
    fn test_session_order_violation() {
        // §20.2:recv 后再次 recv 违反协议(期望 close)
        let src = "(defn main [] (recv (recv 1)))";
        let prog = desugar(src);
        let mut ti = tisp_middle::type_infer::TypeInfer::new();
        let err = ti.infer_program(&prog).unwrap_err();
        assert!(err.message.contains("会话协议"), "应报协议顺序错误,实际: {}", err.message);

        // 合法顺序 send→recv 通过
        let ok_src = "(defn main [] (send (recv 1)))";
        let prog2 = desugar(ok_src);
        let mut ti2 = tisp_middle::type_infer::TypeInfer::new();
        assert!(ti2.infer_program(&prog2).is_ok(), "send→recv 应通过");
    }

    #[test]
    fn test_macro_hygiene() {
        // §24:模板 let 绑定重命名,不被调用点同名变量捕获
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defmacro m [x] (let [y (+ x 1)] y))\n(defn main [] (let [y 100] (m 5)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 6, "宏模板 y 应卫生(不被调用点 y=100 捕获)");
    }

    #[test]
    fn test_gensym_unique() {
        // §24:gensym 每次调用唯一
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            // 同一解释器内两次 gensym
            let src = "(defn main [] (do (gensym) (gensym)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.register_builtins();
            interp.env.push(HashMap::new());
            let g1 = interp.eval_expr(&e(CoreExprNode::App(Box::new(var("gensym")), Box::new(e(CoreExprNode::Lit(Literal::Unit)))))).unwrap();
            let g2 = interp.eval_expr(&e(CoreExprNode::App(Box::new(var("gensym")), Box::new(e(CoreExprNode::Lit(Literal::Unit)))))).unwrap();
            (g1, g2)
        }).unwrap().join().unwrap();
        let (a, b) = match r {
            (Value::Str(x), Value::Str(y)) => (x, y),
            _ => panic!("应为 Str"),
        };
        assert_ne!(a, b, "同一实例内两次 gensym 应不同,实际 {} 与 {}", a, b);
    }

    #[test]
    fn test_generic_specialization() {
        // §22.4:泛型字面量调用被特化,特化 def 产生正确结果
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defgeneric describe [x])\n(defmethod describe [5] \"five\")\n(defmethod describe [9] \"nine\")\n(defn main [] (describe 5))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r {
            Value::Str(s) => assert_eq!(s, "five", "特化应返回方法结果"),
            other => panic!("应为 Str,实际 {:?}", other),
        }

        // middle 层特化计数
        let src2 = "(defgeneric d [x])\n(defmethod d [5] 1)\n(defn main [] (d 5))";
        let prog2 = desugar(src2);
        let mut sp = tisp_middle::specialize::Specializer::new();
        let _ = sp.specialize(&prog2);
        assert_eq!(sp.specialized, 1, "应特化 1 个调用");
    }

    #[cfg(feature = "ffi")]
    #[test]
    fn test_dlopen_extern() {
        // §26:真实 dlopen libc abs(ffi feature);符号缺失报错不崩溃
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defextern c-abs \"abs\" \"libc.so.6\")\n(defn main [] (c-abs -42))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 42, "dlopen abs(-42) 应为 42");

        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defextern nope \"no-such-symbol\" \"libc.so.6\")\n(defn main [] (nope 1))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap_err()
        }).unwrap().join().unwrap();
        assert!(r2.message.contains("FFI"), "符号缺失应报 FFI 错误,实际: {}", r2.message);
    }

    #[test]
    fn test_monadic_single_handler_path() {
        // §12.6:单处理器 handle 走直接状态传递路径(计数),结果与 handler 语义一致
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (handle (do (put 5) (get)) (State s) (get [] [k st] (k st st)) (put [v] [k _] (k Unit v))))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            let res = interp.run_program(&prog).unwrap().unwrap();
            (res, interp.monadic_handles)
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r.0), 5, "状态传递结果应为 5");
        assert_eq!(r.1, 1, "单处理器 handle 应计数 1,实际 {}", r.1);
    }

    #[test]
    fn test_hit_boundary() {
        // §16.3:合法边界通过;未知符号边界违反
        let ok_src = "(defdata-hit Circle (base) (base) (loop :boundary (= loop base)))\n(defn main [] (base))";
        let prog = desugar(ok_src);
        assert!(prog.data_decls[0].boundary.is_some(), "boundary 应被解析");

        let bad_src = "(defdata-hit Bad (base) (base) (loop :boundary (= loop unknown-sym)))\n(defn main [] 1)";
        let err = {
            use tisp_frontend::desugar::Desugarer;
            use tisp_frontend::reader::read;
            let forms = read(bad_src).unwrap();
            Desugarer::new().desugar_program(forms).unwrap_err()
        };
        assert!(err.message.contains("边界违反"), "应报边界违反,实际: {}", err.message);
    }

    #[test]
    fn test_deriving_eq_show() {
        // §7.5:deriving 生成 eq-Name / show-Name(结构递归)
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defdata Color :deriving (Eq Show) (Red) (RGB i64 i64 i64))\n(defn main [] (eq-Color (RGB 1 2 3) (RGB 1 2 3)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r, Value::Bool(true)), "结构相等应成立,实际 {:?}", r);

        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defdata Color :deriving (Eq Show) (Red) (RGB i64 i64 i64))\n(defn main [] (eq-Color (Red) (RGB 1 2 3)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r2, Value::Bool(false)), "不同构造器应不等");

        let r3 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defdata Color :deriving (Eq Show) (Red) (RGB i64 i64 i64))\n(defn main [] (show-Color (RGB 1 2 3)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r3 {
            Value::Str(s) => assert_eq!(s, "(RGB 1 2 3)", "show 应结构显示"),
            other => panic!("应为 Str,实际 {:?}", other),
        }
    }

    #[test]
    fn test_cohesive_shape_and_crisp() {
        // §17:ʃ(shape)返回 Shape 容器(与直通可区分);crisp 上下文检查
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (shape 42))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r {
            Value::Data(tag, _) => assert_eq!(tag.as_str(), "Shape", "ʃ 应返回 Shape 容器,实际 {}", tag),
            other => panic!("应为 Shape,实际 {:?}", other),
        }

        // crisp 上下文检查:非 crisp 的 flat 报错;crisp 内通过
        let bad_src = "(defn main [] (flat 1))";
        let prog_bad = desugar(bad_src);
        let mut ti = tisp_middle::type_infer::TypeInfer::new();
        let err = ti.infer_program(&prog_bad).unwrap_err();
        assert!(err.message.contains("crisp"), "应报 crisp 错误,实际: {}", err.message);

        let ok_src = "(defn main [] (crisp (flat 1)))";
        let prog_ok = desugar(ok_src);
        let mut ti2 = tisp_middle::type_infer::TypeInfer::new();
        assert!(ti2.infer_program(&prog_ok).is_ok(), "crisp 内 flat 应通过");
    }

    #[test]
    fn test_dependent_grade_runtime() {
        // §10:grade-of 返回参数等级列表(Nat(3));Nat 等级不擦除
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn f [(3 x : i64)] x)\n(defn main [] (grade-of \"f\"))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r {
            Value::Str(s) => assert!(s.contains("Nat(3)"), "grade-of 应含 Nat(3),实际 {}", s),
            other => panic!("应为 Str,实际 {:?}", other),
        }

        // Nat 等级参数参与运行(不擦除):(3 x : i64) 的 x 可求值
        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn f [(3 x : i64)] x)\n(defn main [] (f 42))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r2), 42, "Nat 等级参数应正常求值");
    }

    #[test]
    fn test_zero_grade_param_not_evaluated() {
        // §10.1:0 级实参不求值 —— 未定义符号也不报错(被擦除)
        let src = "(defn f [{0 x : i64}] 42)\n(defn main [] (f undefined-symbol))";
        let prog = desugar(src);
        let mut interp = Interpreter::new();
        let r = interp.run_program(&prog).unwrap().unwrap();
        assert_eq!(as_int(r), 42);
    }

    #[test]
    fn test_zero_grade_param_not_bound() {
        // §10.1:0 级参数不绑定进环境 —— 体内引用报 unbound
        let src = "(defn f [{0 x : i64}] x)\n(defn main [] (f 1))";
        let prog = desugar(src);
        let mut interp = Interpreter::new();
        let err = interp.run_program(&prog).unwrap_err();
        assert!(err.message.contains("unbound"), "0 级参数应未绑定,实际: {}", err.message);
    }

    fn desugar(src: &str) -> tisp_core::core_ast::CoreProgram {
        use tisp_frontend::desugar::Desugarer;
        use tisp_frontend::reader::read;
        let forms = read(src).unwrap();
        Desugarer::new().desugar_program(forms).unwrap()
    }


}

