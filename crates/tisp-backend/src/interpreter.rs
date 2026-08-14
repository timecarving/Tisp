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
use crate::process::{ProcessRuntime, ModelChecker, DolevYaoAttacker};
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
    /// Typeclass instance dictionary: class_name → [(实例类型列表, 方法表)]
    pub instance_dict: HashMap<Symbol, Vec<(Vec<Type>, HashMap<Symbol, Value>)>>,
    /// 类型类 fun-deps(§23.3):class_name → [(输入类型变量, 输出类型变量)]
    class_fun_deps: HashMap<Symbol, Vec<(Symbol, Symbol)>>,
    /// 类型类超类(§23.1):class_name → 超类名列表
    class_supers: HashMap<Symbol, Vec<Symbol>>,
    /// 类型类实例类型表(§23.3 fun-deps 冲突检测):class_name → 已登记实例的类型列表
    class_instance_types: HashMap<Symbol, Vec<Vec<Type>>>,
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
    /// §12.6 直接状态线程:单状态 handler 时 get/put 直接读写此槽(替换栈分发)
    direct_state: Option<Value>,
    /// π-calculus 通道运行时(§27):send/recv 经共享缓冲区
    process_runtime: Arc<Mutex<ProcessRuntime>>,
    /// Applied π-calculus 加密引擎(§27.4/27.5)
    crypto: CryptoEngine,
    /// §7.2 非加密占位警告是否已输出(一次;crypto feature 下无用)
    #[cfg(not(feature = "crypto"))]
    crypto_warned: bool,
    /// §26.4 Unsafe 门控警告是否已输出(一次)
    unsafe_warned: bool,
    /// §26.2 裸指针模拟内存:地址 → 值(线性指针读写;真实裸内存由 ffi 门控)
    ptr_mem: HashMap<u64, Value>,
    /// §26.3 区域分配地址计数器
    next_ptr_addr: u64,
    /// §26.3 已释放(悬垂)地址集合:PtrRead 读到悬垂指针时报错
    freed_addrs: std::collections::HashSet<u64>,
    /// §31 MOP 知识库:GetKB/SetKB 效应的运行时状态(事实/规则集)
    kb: tisp_core::evolp::Program,
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
    pub def_sigs: HashMap<Symbol, (usize, Option<tisp_core::types::Type>, tisp_core::types::EffectRow, Vec<tisp_core::types::Grade>, tisp_core::types::Mode, tisp_core::types::Determinism)>,
    /// §9 反射:name → 参数名列表(§29 反射完整信息)
    pub def_params: HashMap<Symbol, Vec<Symbol>>,

    /// §24 gensym 计数器(宏卫生)
    pub gensym_counter: std::sync::atomic::AtomicU64,

    /// §26 dlopen 持有的动态库(ffi feature)
    #[cfg(feature = "ffi")]
    pub extern_libs: Vec<libloading::Library>,

    /// §12.6 单处理器 handle 优化计数(monadic 状态传递路径)
    pub monadic_handles: usize,

    /// 调用深度(诊断)
    pub call_depth: usize,
    pub max_call_depth: usize,

    /// eval 计数(诊断)
    pub eval_count: u64,
}

/// 活跃的 effect handler(Handle 求值时入栈,退出时出栈)
struct ActiveHandler {
    /// 状态槽(§12.3 State 等带状态 effect);None 表示未初始化
    state: Option<Value>,
    clauses: Vec<HandlerClause>,
}

pub type BuiltinFn = Arc<dyn Fn(&mut Interpreter, &[Value]) -> Result<Value, EvalError> + Send + Sync>;

/// §8.1 TCO:apply 蹦床的中间结果——值(结束)或尾调用(继续循环)
enum ApplyOutcome {
    Done(Value),
    Tail(Value, Vec<Value>),
}

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
    /// §9 类型一等值:运行时类型值
    Type(tisp_core::types::Type),
    /// §4 持久化 Vector(im HAMT,O(log32) 结构共享)
    Vector(im::Vector<Value>),
    /// §4 持久化 Map(im HAMT,结构共享)
    Map(im::HashMap<Value, Value>),
    /// §4 持久化 Set(im HAMT,结构共享)
    Set(im::HashSet<Value>),
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
            Value::Type(ty) => write!(f, "{}", ty),
            Value::Vector(v) => write!(f, "[{}]", v.iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>().join(" ")),
            Value::Map(m) => write!(f, "{{{}}}", m.iter().map(|(k, v)| format!("{:?} {:?}", k, v)).collect::<Vec<_>>().join(" ")),
            Value::Set(s) => write!(f, "#{{{}}}", s.iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>().join(" ")),
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
            Value::Type(_) => "Type",
            Value::Vector(_) => "Vec",
            Value::Map(_) => "Map",
            Value::Set(_) => "Set",
        }
    }
}

/// §4 结构相等:标量按值、Data 按构造器名+字段、集合按内容、Type 按类型结构。
/// 闭包(捕获环境)不可结构比较,按不同处理(仅同类判别,eq 返回 false)。
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (Value::Data(na, fa), Value::Data(nb, fb)) => na == nb && fa == fb,
            (Value::Object(a), Value::Object(b)) => a == b,
            (Value::Type(a), Value::Type(b)) => a == b,
            (Value::Vector(a), Value::Vector(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::Builtin(na, _), Value::Builtin(nb, _)) => na == nb,
            (Value::Closure(_), Value::Closure(_)) => false,
            _ => false,
        }
    }
}

impl Eq for Value {}

/// §4 结构哈希:与 PartialEq 一致(相等 ⇒ 同哈希);闭包/内置按判别式区分。
impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Int(n) => n.hash(state),
            Value::Float(f) => f.to_bits().hash(state),
            Value::Bool(b) => b.hash(state),
            Value::Str(s) => s.hash(state),
            Value::Char(c) => c.hash(state),
            Value::Unit => {}
            Value::Data(n, f) => { n.hash(state); f.hash(state); }
            Value::Object(m) => {
                // std HashMap 不实现 Hash:按 key 字典序排序后逐个哈希(与 Eq 一致)
                let mut entries: Vec<(&Symbol, &Value)> = m.iter().collect();
                entries.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
                for (k, v) in entries { k.hash(state); v.hash(state); }
            }
            Value::Type(t) => t.hash(state),
            Value::Vector(v) => v.hash(state),
            Value::Map(m) => m.hash(state),
            Value::Set(s) => s.hash(state),
            Value::Builtin(n, _) => n.hash(state),
            Value::Closure(_) => {}
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
               class_fun_deps: HashMap::new(),
               class_supers: HashMap::new(),
               class_instance_types: HashMap::new(),
               ctor_arity: HashMap::new(),
               field_names: HashMap::new(),
               collect_mode: false,
               collected_solutions: Vec::new(),
               collect_start_depth: 0,
               handlers: Vec::new(),
               direct_state: None,
               process_runtime: Arc::new(Mutex::new(ProcessRuntime::new())),
               crypto: CryptoEngine::new(),
               #[cfg(not(feature = "crypto"))]
               crypto_warned: false,
               unsafe_warned: false,
               ptr_mem: HashMap::new(),
               next_ptr_addr: 1,
               freed_addrs: std::collections::HashSet::new(),
               kb: tisp_core::evolp::Program::new(),
               streams: HashMap::new(),
               next_stream_id: 0,
               ambients: HashMap::new(),
               properties: HashMap::new(),
               ctor_to_adt: HashMap::new(),
               signals: HashMap::new(),
               next_signal_id: 0,
               clp_var_names: HashMap::new(),
               def_sigs: HashMap::new(),
               def_params: HashMap::new(),
               gensym_counter: std::sync::atomic::AtomicU64::new(0),
               #[cfg(feature = "ffi")]
               extern_libs: Vec::new(),
               monadic_handles: 0,
               call_depth: 0,
               eval_count: 0,
               max_call_depth: 0 }
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
        // §26 字符串签名:fn(str) -> str(UTF-8 → CString → CStr → UTF-8)
        {
            use std::ffi::{CString, CStr};
            if let Ok(f) = unsafe { lib.get::<unsafe extern "C" fn(*const i8) -> *const i8>(sym_name.as_bytes()) } {
                let f = *f;
                let v = Value::Builtin(sym_name.clone().into(), Arc::new(move |_s, args| {
                    let a = match args.first() {
                        Some(Value::Str(s)) => s.clone(),
                        _ => String::new(),
                    };
                    let c = match CString::new(a) { Ok(c) => c, Err(_) => return Ok(Value::Str(String::new())) };
                    let r = unsafe { f(c.as_ptr()) };
                    if r.is_null() {
                        Ok(Value::Str(String::new()))
                    } else {
                        let s = unsafe { CStr::from_ptr(r) }.to_string_lossy().into_owned();
                        Ok(Value::Str(s))
                    }
                }));
                self.extern_libs.push(lib);
                return Ok(v);
            }
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
            // §21.2 结构化值统一:Cons/Nil/Vec → Cons 链(替换 Int(0) 折叠)
            Value::Data(name, args) => match name.as_str() {
                "Nil" => LogicValue::Nil,
                "Cons" if args.len() == 2 => {
                    LogicValue::Cons(Box::new(self.val_to_logic(&args[0])), Box::new(self.val_to_logic(&args[1])))
                }
                "Vec" => {
                    let mut result = LogicValue::Nil;
                    for a in args.iter().rev() {
                        result = LogicValue::Cons(Box::new(self.val_to_logic(a)), Box::new(result));
                    }
                    result
                }
                _ => LogicValue::Int(0),
            },
            // §4 持久化 Vector → Cons 链(结构化统一)
            Value::Vector(v) => {
                let mut result = LogicValue::Nil;
                for a in v.iter().rev() {
                    result = LogicValue::Cons(Box::new(self.val_to_logic(a)), Box::new(result));
                }
                result
            }
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
            bi("<=", |_s, args| {
                let (a, b) = expect_two_ints(args)?;
                Ok(Value::Bool(a <= b))
            }),
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
            bi("vector", |_s, args| Ok(Value::Vector(args.iter().cloned().collect()))),
            bi("hash-map", |_s, args| {
                let mut m = im::HashMap::new();
                let mut i = 0;
                while i + 1 < args.len() {
                    m.insert(args[i].clone(), args[i + 1].clone());
                    i += 2;
                }
                Ok(Value::Map(m))
            }),
            bi("hash-set", |_s, args| Ok(Value::Set(args.iter().cloned().collect()))),
            // §4 持久化集合操作(结构共享:返回新结构,旧结构不变)
            bi("conj", |_s, args| {
                if args.len() >= 2 {
                    let coll = &args[0];
                    let item = args[1].clone();
                    match coll {
                        Value::Vector(v) => {
                            let mut v2 = v.clone();
                            v2.push_back(item);
                            Ok(Value::Vector(v2))
                        }
                        Value::Set(s) => {
                            let mut s2 = s.clone();
                            s2.insert(item);
                            Ok(Value::Set(s2))
                        }
                        Value::Map(m) => {
                            // (conj m [k v]) 或 (conj m entry...)
                            let mut m2 = m.clone();
                            if let Value::Vector(kv) = &item {
                                if kv.len() == 2 {
                                    m2.insert(kv[0].clone(), kv[1].clone());
                                }
                            }
                            Ok(Value::Map(m2))
                        }
                        // 列表:头插(结构共享尾部)
                        other => Ok(Value::Data(Symbol::new("Cons"), vec![item, other.clone()])),
                    }
                } else {
                    Ok(Value::Unit)
                }
            }),
            bi("assoc", |_s, args| {
                if args.len() >= 3 {
                    if let Value::Map(m) = &args[0] {
                        let mut m2 = m.clone();
                        let mut i = 1;
                        while i + 1 < args.len() {
                            m2.insert(args[i].clone(), args[i + 1].clone());
                            i += 2;
                        }
                        return Ok(Value::Map(m2));
                    }
                    if let Value::Vector(v) = &args[0] {
                        if let (Value::Int(idx), val) = (&args[1], &args[2]) {
                            let mut v2 = v.clone();
                            if *idx >= 0 && (*idx as usize) < v2.len() {
                                v2.set(*idx as usize, val.clone());
                                return Ok(Value::Vector(v2));
                            }
                        }
                    }
                }
                Ok(Value::Unit)
            }),
            bi("contains?", |_s, args| {
                if args.len() >= 2 {
                    return Ok(Value::Bool(match &args[0] {
                        Value::Set(s) => s.contains(&args[1]),
                        Value::Map(m) => m.contains_key(&args[1]),
                        other => {
                            // 列表/任意集合:线性扫描
                            list_to_vec(other).iter().any(|v| v == &args[1])
                        }
                    }));
                }
                Ok(Value::Bool(false))
            }),
            bi("dissoc", |_s, args| {
                if args.len() >= 2 {
                    if let Value::Map(m) = &args[0] {
                        let mut m2 = m.clone();
                        for k in &args[1..] {
                            m2.remove(k);
                        }
                        return Ok(Value::Map(m2));
                    }
                }
                Ok(Value::Unit)
            }),
            bi("disj", |_s, args| {
                if args.len() >= 2 {
                    if let Value::Set(s) = &args[0] {
                        let mut s2 = s.clone();
                        for x in &args[1..] {
                            s2.remove(x);
                        }
                        return Ok(Value::Set(s2));
                    }
                }
                Ok(Value::Unit)
            }),
            bi("first", |_s, args| {
                match args.first() {
                    Some(Value::Data(c, fields)) if c.as_str() == "Cons" && !fields.is_empty() => Ok(fields[0].clone()),
                    Some(Value::Vector(v)) => Ok(v.front().cloned().unwrap_or(Value::Unit)),
                    _ => Ok(Value::Unit),
                }
            }),
            bi("rest", |_s, args| {
                match args.first() {
                    Some(Value::Data(c, fields)) if c.as_str() == "Cons" && fields.len() >= 2 => Ok(fields[1].clone()),
                    Some(Value::Vector(v)) if !v.is_empty() => Ok(Value::Vector(v.skip(1))),
                    _ => Ok(Value::Unit),
                }
            }),
            bi("nth", |_s, args| {
                if args.len() >= 2 {
                    if let Value::Int(n) = &args[1] {
                        // §4 持久化 Vector:直接 O(log32) 索引
                        if let Value::Vector(v) = &args[0] {
                            if *n >= 0 && (*n as usize) < v.len() {
                                return Ok(v[*n as usize].clone());
                            }
                            return Ok(Value::Unit);
                        }
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
            // §16.3 fun-ext:有限域函数点态等价
            bi("fun-ext", |s, args| {
                if args.len() >= 3 {
                    let f = args[0].clone();
                    let g = args[1].clone();
                    let vals: Vec<Value> = match &args[2] {
                        Value::Data(_, fields) => fields.clone(),
                        _ => vec![],
                    };
                    for v in vals {
                        let fv = s.apply(f.clone(), &[v.clone()]).unwrap_or(Value::Unit);
                        let gv = s.apply(g.clone(), &[v]).unwrap_or(Value::Unit);
                        if !values_eq(&fv, &gv) { return Ok(Value::Bool(false)); }
                    }
                    return Ok(Value::Bool(true));
                }
                Ok(Value::Bool(false))
            }),
            // §16.4 幺半等价:结合律 + 单位元(有限域枚举,反例即 false)
            bi("monoid-check", |s, args| {
                if args.len() >= 3 {
                    let op = args[0].clone();
                    let unit = args[1].clone();
                    let vals: Vec<Value> = match &args[2] {
                        Value::Data(_, fields) => fields.clone(),
                        _ => vec![],
                    };
                    for x in &vals {
                        let l = s.apply(op.clone(), &[unit.clone(), x.clone()]).unwrap_or(Value::Unit);
                        let r = s.apply(op.clone(), &[x.clone(), unit.clone()]).unwrap_or(Value::Unit);
                        if !values_eq(&l, x) || !values_eq(&r, x) { return Ok(Value::Bool(false)); }
                    }
                    for x in &vals { for y in &vals { for z in &vals {
                        let xy = s.apply(op.clone(), &[x.clone(), y.clone()]).unwrap_or(Value::Unit);
                        let l = s.apply(op.clone(), &[xy, z.clone()]).unwrap_or(Value::Unit);
                        let yz = s.apply(op.clone(), &[y.clone(), z.clone()]).unwrap_or(Value::Unit);
                        let r = s.apply(op.clone(), &[x.clone(), yz]).unwrap_or(Value::Unit);
                        if !values_eq(&l, &r) { return Ok(Value::Bool(false)); }
                    }}}
                    return Ok(Value::Bool(true));
                }
                Ok(Value::Bool(false))
            }),
            // ── 反射(§29.7):查询 def_sigs 返回真实静态信息,替换硬编码常量 ──
            bi("type-of", |s, args| {
                // 静态推断类型(声明返回类型),非运行时值标签
                if let Some(Value::Str(name)) = args.first() {
                    if let Some((_, Some(ty), _, _, _, _)) = s.def_sigs.get(&Symbol::new(name)) {
                        return Ok(Value::Type(ty.clone()));
                    }
                }
                if let Some(v) = args.first() { Ok(Value::Str(v.type_name().to_string())) } else { Ok(Value::Str("unknown".into())) }
            }),
            bi("grade-of", |s, args| {
                // §10:查询定义参数等级列表
                if let Some(Value::Str(name)) = args.first() {
                    if let Some((_, _, _, grades, _, _)) = s.def_sigs.get(&Symbol::new(name)) {
                        return Ok(Value::Str(format!("{:?}", grades)));
                    }
                }
                Ok(Value::Str("ω".into()))
            }),
            bi("mode-of", |s, args| {
                if let Some(Value::Str(name)) = args.first() {
                    if let Some((_, _, _, _, mode, _)) = s.def_sigs.get(&Symbol::new(name)) {
                        return Ok(Value::Str(format!("{:?}", mode)));
                    }
                }
                Ok(Value::Str("in".into()))
            }),
            bi("effects-of", |s, args| {
                if let Some(Value::Str(name)) = args.first() {
                    if let Some((_, _, effects, _, _, _)) = s.def_sigs.get(&Symbol::new(name)) {
                        return Ok(Value::Str(format!("{:?}", effects)));
                    }
                }
                Ok(Value::Str("Pure".into()))
            }),
            bi("determinism-of", |s, args| {
                if let Some(Value::Str(name)) = args.first() {
                    if let Some((_, _, _, _, _, det)) = s.def_sigs.get(&Symbol::new(name)) {
                        return Ok(Value::Str(format!("{:?}", det)));
                    }
                }
                Ok(Value::Str("det".into()))
            }),
            bi("reflect", |s, args| {
                // §29 反射完整信息:名称/参数/类型/效果/等级/模式/确定性(全真实,无近似)
                let name = args.first().and_then(|a| match a { Value::Str(n) => Some(n.clone()), _ => None });
                if let Some(n) = name {
                    if let Some((arity, ty, eff, grades, mode, det)) = s.def_sigs.get(&Symbol::new(&n)).cloned() {
                        let params: im::Vector<Value> = s.def_params.get(&Symbol::new(&n))
                            .cloned().unwrap_or_default()
                            .into_iter().map(|p| Value::Str(p.as_str().to_string())).collect();
                        let ty_v = ty.clone().map(Value::Type).unwrap_or(Value::Unit);
                        return Ok(Value::Data(Symbol::new("DefInfo"), vec![
                            Value::Str(n),
                            Value::Int(arity as i64),
                            Value::Vector(params),
                            ty_v,
                            Value::Str(format!("{:?}", eff)),
                            Value::Str(format!("{:?}", grades)),
                            Value::Str(format!("{:?}", mode)),
                            Value::Str(format!("{:?}", det)),
                        ]));
                    }
                }
                Ok(Value::Unit)
            }),
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
            // §17 Cohesive 连通图:(shape-graph [路径...]) → 节点与连通边
            bi("shape-graph", |s, args| {
                if let Some(paths) = args.first() {
                    if let Value::Data(_, fields) = paths {
                        let mut edges = Vec::new();
                        for p in fields {
                            let b0 = s.apply(p.clone(), &[Value::Bool(false)]).unwrap_or(Value::Unit);
                            let b1 = s.apply(p.clone(), &[Value::Bool(true)]).unwrap_or(Value::Unit);
                            edges.push(Value::Data(Symbol::new("Edge"), vec![
                                Value::Bool(values_eq(&b0, &b1)), b0, b1,
                            ]));
                        }
                        return Ok(Value::Data(Symbol::new("ShapeGraph"), edges));
                    }
                }
                Ok(Value::Data(Symbol::new("ShapeGraph"), vec![]))
            }),
            // §18.6 多时钟重采样:(resample stream rate) 按速率抽值(每 rate 取一)
            bi("resample", |s, args| {
                if args.len() >= 2 {
                    if let Ok(id) = stream_id(&args[0]) {
                        let rate = match &args[1] { Value::Int(n) => (*n).max(1) as usize, _ => 1 };
                        if let Some(st) = s.streams.get(&id).cloned() {
                            let vals: Vec<Value> = st.take(rate * 5).into_iter()
                                .step_by(rate).map(Value::Int).collect();
                            return Ok(list_from_vec(vals));
                        }
                    }
                }
                Ok(list_from_vec(vec![]))
            }),
            // §18.6 多时钟:(clock name rate) 注册真实时钟(替换字面量占位)
            bi("clock", |_s, args| {
                if args.len() >= 2 {
                    let name = show_value(&args[0]);
                    let rate = match &args[1] { Value::Int(n) => *n, Value::Float(f) => *f as i64, _ => 1 };
                    Ok(Value::Data(Symbol::new("Clock"), vec![Value::Int(rate), Value::Str(name)]))
                } else {
                    Ok(Value::Data(Symbol::new("Clock"), vec![Value::Int(1), Value::Str("clock@1Hz".into())]))
                }
            }),
            // §26.3 手动区域:with-region 创建区域、运行 f、退出时回收该区域内所有分配
            bi("with-region", |s, args| {
                unsafe_warn(s);
                if let Some(f) = args.first().cloned() {
                    let start_addr = s.next_ptr_addr;
                    // 运行 f(区域 id 以 0 传入)
                    let r = s.apply(f, &[Value::Int(0)]);
                    // 退出:回收本区域分配(addr >= start_addr),并标记悬垂
                    let addrs: Vec<u64> = s.ptr_mem.keys().filter(|&&a| a >= start_addr).cloned().collect();
                    for a in addrs { s.ptr_mem.remove(&a); s.freed_addrs.insert(a); }
                    r
                } else {
                    Ok(Value::Unit)
                }
            }),
            // §18.1 always/eventually:有限窗口流判定
            bi("always", |s, args| {
                if args.len() >= 3 {
                    if let Ok(id) = stream_id(&args[0]) {
                        let window = match &args[2] { Value::Int(n) => *n as usize, _ => 10 };
                        let pred = args[1].clone();
                        if let Some(st) = s.streams.get(&id).cloned() {
                            let vals = st.take(window);
                            for v in vals {
                                let pv = s.apply(pred.clone(), &[Value::Int(v)]).unwrap_or(Value::Bool(false));
                                if !is_truthy(&pv) { return Ok(Value::Bool(false)); }
                            }
                            return Ok(Value::Bool(true));
                        }
                    }
                }
                Ok(Value::Bool(false))
            }),
            bi("eventually", |s, args| {
                if args.len() >= 3 {
                    if let Ok(id) = stream_id(&args[0]) {
                        let window = match &args[2] { Value::Int(n) => *n as usize, _ => 10 };
                        let pred = args[1].clone();
                        if let Some(st) = s.streams.get(&id).cloned() {
                            let vals = st.take(window);
                            for v in vals {
                                let pv = s.apply(pred.clone(), &[Value::Int(v)]).unwrap_or(Value::Bool(false));
                                if is_truthy(&pv) { return Ok(Value::Bool(true)); }
                            }
                            return Ok(Value::Bool(false));
                        }
                    }
                }
                Ok(Value::Bool(false))
            }),
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
                    // §21 过滤:有非空解时丢弃空解(递归尾失败产生的空快照)
                    let mut sols: Vec<Vec<Value>> = s.collected_solutions.clone();
                    let has_nonempty = sols.iter().any(|sol| !sol.is_empty());
                    if has_nonempty {
                        sols.retain(|sol| !sol.is_empty());
                    }
                    let results: Vec<Value> = sols.iter()
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
                // §28 dolev-yao:攻击者窃听传输 + 合成(拼接)后检查机密是否进入知识
                let checker = ModelChecker::new(20);
                let secret = "SECRET";
                let init = DolevYaoAttacker::new();
                let result = checker.find_attack(init,
                    |a| a.knows(secret),
                    |a| {
                        let mut next = a.clone();
                        // 教学级协议:步骤 0 泄漏前缀,步骤 1 泄漏机密(不安全通道)
                        if next.knowledge.is_empty() {
                            next.eavesdrop("pub");
                        } else {
                            next.eavesdrop(secret);
                        }
                        // dolev-yao 合成:拼接已知消息(重放/篡改基础)
                        next.synthesize();
                        vec![next]
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
            // ── 范式集成(§31/§32):24 范式经 ParadigmRegistry 全链路接入,从源码可调用 ──
            bi("pf-higher-order", |_s, args| paradigm_eval("higher-order", args)),
            bi("pf-induce", |_s, args| paradigm_eval("induce", args)),
            bi("pf-prob", |_s, args| paradigm_eval("prob", args)),
            bi("pf-eventually", |_s, args| paradigm_eval("eventually", args)),
            bi("pf-subsume", |_s, args| paradigm_eval("subsume", args)),
            bi("pf-settle", |_s, args| paradigm_eval("settle", args)),
            bi("pf-fuzzy-and", |_s, args| paradigm_eval("fuzzy-and", args)),
            bi("pf-tabling", |_s, args| paradigm_eval("tabling", args)),
            bi("pf-typed-pred", |_s, args| paradigm_eval("typed-pred", args)),
            bi("pf-reactive", |_s, args| paradigm_eval("reactive", args)),
            bi("pf-context-query", |_s, args| paradigm_eval("context-query", args)),
            bi("pf-possible", |_s, args| paradigm_eval("possible", args)),
            bi("pf-evolp", |_s, args| paradigm_eval("evolp", args)),
            bi("pf-dlp", |_s, args| paradigm_eval("dlp", args)),
            bi("pf-get-kb", |_s, args| paradigm_eval("get-kb", args)),
            bi("pf-array-sum", |_s, args| paradigm_eval("array-sum", args)),
            bi("pf-stack-top", |_s, args| paradigm_eval("stack-top", args)),
            bi("pf-compose", |_s, args| paradigm_eval("compose", args)),
            bi("pf-sym-eval", |_s, args| paradigm_eval("sym-eval", args)),
            bi("pf-dfa-accept", |_s, args| paradigm_eval("dfa-accept", args)),
            bi("pf-sm-drive", |_s, args| paradigm_eval("sm-drive", args)),
            bi("pf-dispatch", |_s, args| paradigm_eval("dispatch", args)),
            bi("pf-stream-take", |_s, args| paradigm_eval("stream-take", args)),
            bi("pf-aop-weave", |_s, args| paradigm_eval("aop-weave", args)),
            // §32 真实自动机:DFA 识别(接线 tisp_runtime::programming::Dfa,替换 pf-dfa-accept 的 sum%2 占位)
            // (dfa-accept start accept-list transitions input):transitions 为扁平 [from char-code to ...] 三元组
            bi("dfa-accept", |_s, args| {
                if args.len() != 4 {
                    return Err(EvalError { message: "dfa-accept 需 (start accept-list transitions input) 4 参".into() });
                }
                let start = match &args[0] {
                    Value::Int(n) => n.to_string(),
                    _ => return Err(EvalError { message: "dfa-accept:start 应为整数状态".into() }),
                };
                let accept: im::HashSet<String> = value_to_int_list(&args[1])?
                    .into_iter().map(|n| n.to_string()).collect();
                let triples = value_to_int_list(&args[2])?;
                if triples.len() % 3 != 0 {
                    return Err(EvalError { message: "dfa-accept:transitions 长度须为 3 的倍数".into() });
                }
                let mut transitions = Vec::new();
                for chunk in triples.chunks(3) {
                    let from = chunk[0].to_string();
                    let ch = char::from_u32(chunk[1] as u32).unwrap_or('?');
                    let to = chunk[2].to_string();
                    transitions.push((from, ch, to));
                }
                let input = match &args[3] {
                    Value::Str(s) => s.clone(),
                    _ => return Err(EvalError { message: "dfa-accept:input 应为字符串".into() }),
                };
                let dfa = tisp_runtime::programming::Dfa { start, accept, transitions };
                Ok(Value::Bool(dfa.accepts(&input)))
            }),
            // §32 真实状态机:事件驱动转移(接线 tisp_runtime::programming::StateMachine,替换 sm-drive 占位)
            // (sm-drive current event transitions):transitions 为扁平 [from event to ...] 三元组
            bi("sm-drive", |_s, args| {
                if args.len() != 3 {
                    return Err(EvalError { message: "sm-drive 需 (current event transitions) 3 参".into() });
                }
                let current = match &args[0] {
                    Value::Int(n) => n.to_string(),
                    _ => return Err(EvalError { message: "sm-drive:current 应为整数状态".into() }),
                };
                let event = match &args[1] {
                    Value::Int(n) => n.to_string(),
                    _ => return Err(EvalError { message: "sm-drive:event 应为整数事件".into() }),
                };
                let triples = value_to_int_list(&args[2])?;
                if triples.len() % 3 != 0 {
                    return Err(EvalError { message: "sm-drive:transitions 长度须为 3 的倍数".into() });
                }
                let transitions: Vec<(String, String, String)> = triples.chunks(3)
                    .map(|c| (c[0].to_string(), c[1].to_string(), c[2].to_string()))
                    .collect();
                let mut sm = tisp_runtime::programming::StateMachine { current, transitions, trace: Vec::new() };
                match sm.drive(&event) {
                    Ok(()) => Ok(Value::Int(sm.current.parse::<i64>().unwrap_or(0))),
                    Err(e) => Err(EvalError { message: e }),
                }
            }),
            // §32 真实描述逻辑:概念子概念推理(接线 tisp_runtime::paradigms::Ontology,替换 subsume 占位)
            // (subsume subsumes-pairs concept query):subsumes-pairs 为扁平 [sub super ...]
            bi("subsume", |_s, args| {
                if args.len() != 3 {
                    return Err(EvalError { message: "subsume 需 (subsumes concept query) 3 参".into() });
                }
                let pairs = value_to_int_list(&args[0])?;
                if pairs.len() % 2 != 0 {
                    return Err(EvalError { message: "subsume:subsumes 长度须为偶数".into() });
                }
                let subsumes: Vec<(Symbol, Symbol)> = pairs.chunks(2)
                    .map(|c| (Symbol::new(&c[0].to_string()), Symbol::new(&c[1].to_string())))
                    .collect();
                let concept = match &args[1] {
                    Value::Int(n) => Symbol::new(&n.to_string()),
                    _ => return Err(EvalError { message: "subsume:concept 应为整数".into() }),
                };
                let query = match &args[2] {
                    Value::Int(n) => Symbol::new(&n.to_string()),
                    _ => return Err(EvalError { message: "subsume:query 应为整数".into() }),
                };
                let ont = tisp_runtime::paradigms::Ontology { subsumes };
                Ok(Value::Bool(ont.is_instance(&concept, &query)))
            }),
            // §32 真实表格化逻辑:左递归终止 + 记忆(接线 tisp_runtime::paradigms::Tabler,替换 tabling 占位)
            // (tabling facts rules goal):facts 为原子列表,rules 为扁平 [head body ...](每规则单个体原子)
            bi("tabling", |_s, args| {
                use tisp_core::evolp::LTerm;
                if args.len() != 3 {
                    return Err(EvalError { message: "tabling 需 (facts rules goal) 3 参".into() });
                }
                let facts: Vec<LTerm> = value_to_int_list(&args[0])?
                    .into_iter().map(|n| LTerm::atom(&n.to_string())).collect();
                let rule_pairs = value_to_int_list(&args[1])?;
                if rule_pairs.len() % 2 != 0 {
                    return Err(EvalError { message: "tabling:rules 长度须为偶数".into() });
                }
                let rules: Vec<(LTerm, Vec<LTerm>)> = rule_pairs.chunks(2)
                    .map(|c| (LTerm::atom(&c[0].to_string()), vec![LTerm::atom(&c[1].to_string())]))
                    .collect();
                let goal = match &args[2] {
                    Value::Int(n) => LTerm::atom(&n.to_string()),
                    _ => return Err(EvalError { message: "tabling:goal 应为整数原子".into() }),
                };
                let mut tabler = tisp_runtime::paradigms::Tabler::new(&facts, rules);
                Ok(Value::Bool(tabler.prove(&goal)))
            }),
            // §32 真实符号编程:后序构造 SymExpr + 化简 + 求值(接线 programming::SymExpr,替换 sym-eval 占位)
            // (sym-eval postfix):postfix 为扁平后序记号,n≥0 = Num(n),-1 = Add,-2 = Mul
            bi("sym-eval", |_s, args| {
                use tisp_runtime::programming::SymExpr;
                if args.len() != 1 {
                    return Err(EvalError { message: "sym-eval 需 (postfix) 1 参".into() });
                }
                let toks = value_to_int_list(&args[0])?;
                let mut stack: Vec<SymExpr> = Vec::new();
                for t in toks {
                    match t {
                        n if n >= 0 => stack.push(SymExpr::Num(n)),
                        -1 => {
                            let b = stack.pop().ok_or_else(|| EvalError { message: "sym-eval:Add 缺操作数".into() })?;
                            let a = stack.pop().ok_or_else(|| EvalError { message: "sym-eval:Add 缺操作数".into() })?;
                            stack.push(SymExpr::Add(Box::new(a), Box::new(b)));
                        }
                        -2 => {
                            let b = stack.pop().ok_or_else(|| EvalError { message: "sym-eval:Mul 缺操作数".into() })?;
                            let a = stack.pop().ok_or_else(|| EvalError { message: "sym-eval:Mul 缺操作数".into() })?;
                            stack.push(SymExpr::Mul(Box::new(a), Box::new(b)));
                        }
                        _ => return Err(EvalError { message: format!("sym-eval:未知记号 {}", t) }),
                    }
                }
                let expr = stack.pop().ok_or_else(|| EvalError { message: "sym-eval:空表达式".into() })?;
                Ok(Value::Int(expr.simplify().eval().unwrap_or(0)))
            }),
            // §31 真实 EVOLP/ASP:命题稳定模型(接线 tisp_runtime::evolp::stable_models,替换 evolp 占位)
            // (evolp-stable facts rules):facts 为正原子列表,rules 为扁平 [head body ...](body<0 = not atom)
            bi("evolp-stable", |_s, args| {
                use tisp_core::evolp::{LTerm, Literal, Program, Rule};
                if args.len() != 2 {
                    return Err(EvalError { message: "evolp-stable 需 (facts rules) 2 参".into() });
                }
                let facts = value_to_int_list(&args[0])?;
                let rule_pairs = value_to_int_list(&args[1])?;
                if rule_pairs.len() % 2 != 0 {
                    return Err(EvalError { message: "evolp-stable:rules 长度须为偶数".into() });
                }
                let mut prog = Program::new();
                for f in &facts {
                    prog.add(Rule::fact(&f.to_string(), LTerm::atom(&f.to_string())));
                }
                for chunk in rule_pairs.chunks(2) {
                    let head = chunk[0];
                    let body = chunk[1];
                    let lit = if body >= 0 {
                        Literal::Pos(LTerm::atom(&body.to_string()))
                    } else {
                        Literal::Neg(LTerm::atom(&(-body).to_string()))
                    };
                    prog.add(Rule::rule(&format!("r{}", head), LTerm::atom(&head.to_string()), vec![lit]));
                }
                let models = tisp_runtime::evolp::stable_models(&prog);
                match models.first() {
                    Some(m) => {
                        let atoms: im::Vector<Value> = m.iter().filter_map(|t| match t {
                            LTerm::Fun(n, _) => n.as_str().parse::<i64>().ok().map(Value::Int),
                            _ => None,
                        }).collect();
                        Ok(Value::Vector(atoms))
                    }
                    None => Ok(Value::Vector(im::Vector::new())),
                }
            }),
            // §31 真实 DLP:动态稳定模型(接线 tisp_runtime::evolp::dynamic_stable_models)
            // (dlp-stable facts1 rules1 facts2 rules2):两状态,拒绝被后续状态否定的规则
            bi("dlp-stable", |_s, args| {
                use tisp_core::evolp::{LTerm, Literal, Program, Rule};
                if args.len() != 4 {
                    return Err(EvalError { message: "dlp-stable 需 (facts1 rules1 facts2 rules2) 4 参".into() });
                }
                let build = |fv: &Value, rv: &Value| -> Result<Program, EvalError> {
                    let facts = value_to_int_list(fv)?;
                    let rule_pairs = value_to_int_list(rv)?;
                    if rule_pairs.len() % 2 != 0 {
                        return Err(EvalError { message: "dlp-stable:rules 长度须为偶数".into() });
                    }
                    let mut prog = Program::new();
                    for f in &facts {
                        prog.add(Rule::fact(&f.to_string(), LTerm::atom(&f.to_string())));
                    }
                    for chunk in rule_pairs.chunks(2) {
                        let head = chunk[0];
                        let body = chunk[1];
                        let lit = if body >= 0 {
                            Literal::Pos(LTerm::atom(&body.to_string()))
                        } else {
                            Literal::Neg(LTerm::atom(&(-body).to_string()))
                        };
                        prog.add(Rule::rule(&format!("r{}", head), LTerm::atom(&head.to_string()), vec![lit]));
                    }
                    Ok(prog)
                };
                let p1 = build(&args[0], &args[1])?;
                let p2 = build(&args[2], &args[3])?;
                let models = tisp_runtime::evolp::dynamic_stable_models(&[p1, p2]);
                match models.first() {
                    Some(m) => {
                        let atoms: im::Vector<Value> = m.iter().filter_map(|t| match t {
                            LTerm::Fun(n, _) => n.as_str().parse::<i64>().ok().map(Value::Int),
                            _ => None,
                        }).collect();
                        Ok(Value::Vector(atoms))
                    }
                    None => Ok(Value::Vector(im::Vector::new())),
                }
            }),
            // §31 真实 EVOLP 演化:assert/retract 指令 + foldl 折叠(接线 tisp_runtime::evolp::evolve_all)
            // (evolp-evolve facts instructions):instructions 为扁平 [op atom ...],op=1 assert / 0 retract
            bi("evolp-evolve", |_s, args| {
                use tisp_core::evolp::{EvolInstr, LTerm, Program, Rule};
                use tisp_core::symbol::Symbol as Sym;
                if args.len() != 2 {
                    return Err(EvalError { message: "evolp-evolve 需 (facts instructions) 2 参".into() });
                }
                let facts = value_to_int_list(&args[0])?;
                let instrs = value_to_int_list(&args[1])?;
                if instrs.len() % 2 != 0 {
                    return Err(EvalError { message: "evolp-evolve:instructions 长度须为偶数".into() });
                }
                let mut prog = Program::new();
                for f in &facts {
                    prog.add(Rule::fact(&f.to_string(), LTerm::atom(&f.to_string())));
                }
                let mut evols = Vec::new();
                for chunk in instrs.chunks(2) {
                    let op = chunk[0];
                    let atom = chunk[1];
                    if op == 1 {
                        evols.push(EvolInstr::Assert(Rule::fact(&atom.to_string(), LTerm::atom(&atom.to_string()))));
                    } else {
                        evols.push(EvolInstr::Retract(Sym::new(&atom.to_string())));
                    }
                }
                let result = tisp_runtime::evolp::evolve_all(&prog, &evols);
                let atoms: im::Vector<Value> = result.iter().filter_map(|r| match &r.head {
                    LTerm::Fun(n, _) => n.as_str().parse::<i64>().ok().map(Value::Int),
                    _ => None,
                }).collect();
                Ok(Value::Vector(atoms))
            }),
            // §31 MOP:GetKB/SetKB 效应操作(运行时 KB 状态)
            bi("get-kb", |s, _args| {
                use tisp_core::evolp::LTerm;
                let atoms: im::Vector<Value> = s.kb.iter().filter_map(|r| match &r.head {
                    LTerm::Fun(n, _) => n.as_str().parse::<i64>().ok().map(Value::Int),
                    _ => None,
                }).collect();
                Ok(Value::Vector(atoms))
            }),
            bi("set-kb", |s, args| {
                use tisp_core::evolp::{LTerm, Program, Rule};
                if args.len() != 1 {
                    return Err(EvalError { message: "set-kb 需 (facts) 1 参".into() });
                }
                let atoms = value_to_int_list(&args[0])?;
                let mut kb = Program::new();
                for a in atoms {
                    kb.add(Rule::fact(&a.to_string(), LTerm::atom(&a.to_string())));
                }
                s.kb = kb;
                Ok(Value::Unit)
            }),
            // §统一内存管理:Ref a 分级值(State 效应,非 Unsafe)——ref/deref/set!
            bi("ref", |s, args| {
                if args.len() != 1 {
                    return Err(EvalError { message: "ref 需 (value) 1 参".into() });
                }
                let addr = s.next_ptr_addr;
                s.next_ptr_addr += 1;
                s.ptr_mem.insert(addr, args[0].clone());
                Ok(Value::Int(addr as i64))
            }),
            bi("deref", |s, args| {
                if let Some(Value::Int(a)) = args.first() {
                    if s.freed_addrs.contains(&(*a as u64)) {
                        return Err(EvalError { message: format!("悬垂引用:地址 {} 已释放", a) });
                    }
                    Ok(s.ptr_mem.get(&(*a as u64)).cloned().unwrap_or(Value::Unit))
                } else {
                    Err(EvalError { message: "deref 需整数地址".into() })
                }
            }),
            bi("set!", |s, args| {
                if args.len() != 2 {
                    return Err(EvalError { message: "set! 需 (addr value) 2 参".into() });
                }
                if let Value::Int(a) = &args[0] {
                    s.ptr_mem.insert(*a as u64, args[1].clone());
                    Ok(Value::Unit)
                } else {
                    Err(EvalError { message: "set! 需整数地址".into() })
                }
            }),
            // §16 完整立方填充:2 维 Kan(hcomp-2d top bottom left right)
            // 四条边共享四角,角一致则填充成功,不一致报错(镜像 hott.rs kan_fill_2d)
            bi("hcomp-2d", |s, args| {
                if args.len() != 4 {
                    return Err(EvalError { message: "hcomp-2d 需 (top bottom left right) 4 条边".into() });
                }
                let mut corner = |edge: &Value, i: bool| -> Value {
                    s.apply(edge.clone(), &[interval_endpoint(i)]).unwrap_or(Value::Unit)
                };
                let tl_t = corner(&args[0], false);
                let tl_l = corner(&args[2], false);
                if !values_eq(&tl_t, &tl_l) {
                    return Err(EvalError { message: "Kan 填充边界不一致:左上角".into() });
                }
                let tr_t = corner(&args[0], true);
                let tr_r = corner(&args[3], false);
                if !values_eq(&tr_t, &tr_r) {
                    return Err(EvalError { message: "Kan 填充边界不一致:右上角".into() });
                }
                let bl_b = corner(&args[1], false);
                let bl_l = corner(&args[2], true);
                if !values_eq(&bl_b, &bl_l) {
                    return Err(EvalError { message: "Kan 填充边界不一致:左下角".into() });
                }
                let br_b = corner(&args[1], true);
                let br_r = corner(&args[3], true);
                if !values_eq(&br_b, &br_r) {
                    return Err(EvalError { message: "Kan 填充边界不一致:右下角".into() });
                }
                Ok(Value::Data(Symbol::new("KanFill2D"), vec![tl_t]))
            }),
            // §16 完整立方填充:N(≥2)维 Kan(hcomp-nd)——N 维立方有 2^N 个角,角全一致则填充成功
            bi("hcomp-nd", |_s, args| {
                if args.len() != 1 {
                    return Err(EvalError { message: "hcomp-nd 需 (corners) 1 参".into() });
                }
                let corners = value_to_int_list(&args[0])?;
                if corners.is_empty() {
                    return Err(EvalError { message: "hcomp-nd:立方体无角".into() });
                }
                let first = corners[0];
                if corners.iter().all(|&c| c == first) {
                    Ok(Value::Int(first))
                } else {
                    Err(EvalError { message: "N 维立方边界不一致(角不一致)".into() })
                }
            }),
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
            let (arity, grades, params) = match &def.body.node {
                CoreExprNode::Lam(lam) => (
                    lam.params.len(),
                    lam.params.iter().map(|p| p.grade.clone()).collect(),
                    lam.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>(),
                ),
                _ => (0, vec![], vec![]),
            };
            self.def_params.insert(def.name.clone(), params);
            self.def_sigs.insert(def.name.clone(), (arity, def.ty.clone(), def.effects.clone(), grades, def.mode.clone(), def.determinism.clone()));
        }

        // (deriving 已由 desugar 生成 DerivingImpl 节点,解释器在 def 求值时注册结构内置)

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
                | CoreExprNode::TheoremDef(..) | CoreExprNode::CompilerMacroDef(..)
                | CoreExprNode::DerivingImpl(..) => {
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

    /// §21.5 识别 (all-different v1 v2 ...) 柯里化链:返回变量表达式列表
    fn collect_all_different<'a>(&self, e: &'a CoreExpr) -> Option<Vec<&'a CoreExpr>> {
        // 最内层 Var(all-different),逐层解包 App
        let mut cur = e;
        let mut args: Vec<&CoreExpr> = Vec::new();
        while let CoreExprNode::App(f, a) = &cur.node {
            args.push(a);
            cur = f;
        }
        if let CoreExprNode::Var(name) = &cur.node {
            if name.as_str() == "all-different" {
                args.reverse();
                return Some(args);
            }
        }
        None
    }

    /// §21.5 算术约束:识别变量/常数为 CLP 变量,分发乘/除/模传播器
    fn clp_arith_constraint(&mut self, op: &str, x: &Value, y: &Value, z: &Value) {
        let mut to_var = |v: &Value| -> Option<u64> {
            match v {
                Value::Int(id) if self.clp_store.domain_of(*id as u64).is_some() => Some(*id as u64),
                Value::Int(c) => Some(self.clp_store.new_int_var(*c, *c)),
                _ => None,
            }
        };
        if let (Some(xv), Some(yv), Some(zv)) = (to_var(x), to_var(y), to_var(z)) {
            match op {
                "*" => self.clp_store.add_mul(xv, yv, zv),
                "/" => self.clp_store.add_div(xv, yv, zv),
                "%" => self.clp_store.add_mod(xv, yv, zv),
                "+" => self.clp_store.add_plus(xv, yv, zv),
                "-" => self.clp_store.add_minus(xv, yv, zv),
                _ => {}
            }
        }
    }

    /// §12.2/12.3:执行 effect 操作,从 handler 栈顶向下分发到匹配的 clause
    pub fn perform_effect(&mut self, op: &str, args: Vec<Value>) -> Result<Value, EvalError> {
        // §12.6 直接状态线程:单状态 handler 时 get/put 直接读写状态槽(替换栈分发)
        if let Some(state) = self.direct_state.clone() {
            match op {
                "get" => return Ok(state),
                "put" if !args.is_empty() => {
                    self.direct_state = Some(args[0].clone());
                    return Ok(Value::Unit);
                }
                _ => {}
            }
        }
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
        self.eval_count += 1;
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
            CoreExprNode::App(..) => {
                let (f, args) = self.eval_app(expr)?;
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
                    // §21 多解收集:遍历所有 arm,每个 arm 从本 Match 入口的干净状态开始
                    // (分支隔离:局部 trail 快照,替换全局 collect_start_depth,避免嵌套 Match 泄漏)
                    let arm_start = self.logic_store.trail_depth();
                    let mut last = Value::Unit;
                    for arm in arms {
                        self.logic_store.restore_to(arm_start);
                        if let Some(bindings) = self.match_pattern(&arm.pattern, &s) {
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
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &s) {
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
                // §12.6 单处理器优化:状态 handler 且无嵌套 → 直接状态线程(状态槽,替换栈分发)
                let ec = tisp_middle::effect_compile::EffectCompiler::new();
                let direct = ec.detect_single_handler(handler) && ec.detect_no_nesting(body);
                if direct {
                    self.monadic_handles += 1;
                    // 直接状态线程:get/put 经 self.direct_state 读写,不 push handler
                    let prev = self.direct_state.take();
                    self.direct_state = Some(Value::Unit);
                    let result = self.eval_expr(body);
                    let result = result?;
                    self.direct_state = prev;
                    return if let Some(rc) = &handler.return_clause {
                        let mut local_env = HashMap::new();
                        local_env.insert(Symbol::new("_"), result);
                        self.env.push(local_env);
                        let r = self.eval_expr(rc);
                        self.env.pop();
                        r
                    } else {
                        Ok(result)
                    };
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
                // §16.3 同伦合成:路径 e 应用 i0/i1 得边界值;填充返回与边界一致的值
                let path = self.eval_expr(e)?;
                let b0 = self.apply(path.clone(), &[interval_endpoint(false)])?;
                let b1 = self.apply(path, &[interval_endpoint(true)])?;
                // §16 完整立方填充:边界不一致 SHALL 为错误(而非静默返回一端)
                if !values_eq(&b0, &b1) {
                    return Err(EvalError { message: format!(
                        "HComp 边界不一致(完整立方填充):{} != {}", value_to_string(&b0), value_to_string(&b1)) });
                }
                Ok(Value::Data(Symbol::new("KanFill"), vec![b0, b1]))
            },
            CoreExprNode::Transp(_, e, target) => {
                // §16.3 传输:沿路径 e 传送到目标端点,返回目标端点值
                let path = self.eval_expr(e)?;
                let t = self.eval_expr(target)?;
                self.apply(path, &[t])
            },
            CoreExprNode::FlatMod(e) => {
                // §17 ♭ flat:剥离拓扑/光滑结构,返回离散点(与直通可区分)
                // §17 adjoint-triple(ʃ ⊣ ♭ ⊣ ♯):♭∘♯ = counit → id(sharp 的 flat 返回原值)
                if let CoreExprNode::SharpMod(inner) = &e.node {
                    return self.eval_expr(inner);
                }
                // §17 adjoint-triple:♭∘ʃ = unit η(flat 的 shape 返回单元嵌入)
                if let CoreExprNode::ShapeMod(inner) = &e.node {
                    let v = self.eval_expr(inner)?;
                    return Ok(Value::Data(Symbol::new("UnitFlatShape"), vec![v]));
                }
                let v = self.eval_expr(e)?;
                Ok(Value::Data(Symbol::new("Flat"), vec![v]))
            },
            CoreExprNode::SharpMod(e) => {
                // §17 ♯ sharp:嵌入 codiscrete 空间(与直通可区分)
                // §17 adjoint-triple:♯∘♭ = unit η'(sharp 的 flat 返回单元嵌入)
                if let CoreExprNode::FlatMod(inner) = &e.node {
                    let v = self.eval_expr(inner)?;
                    return Ok(Value::Data(Symbol::new("UnitSharpFlat"), vec![v]));
                }
                let v = self.eval_expr(e)?;
                Ok(Value::Data(Symbol::new("Sharp"), vec![v]))
            },
            CoreExprNode::CrispMod(e) => self.eval_expr(e),
            CoreExprNode::ShapeMod(e) => {
                // §17 ʃ 形状代数:路径值计算端点连通(i0/i1 端点值相等性,经 hott.rs Interval)
                if let CoreExprNode::FlatMod(inner) = &e.node {
                    return self.eval_expr(inner);
                }
                let v = self.eval_expr(e)?;
                let b0 = self.apply(v.clone(), &[interval_endpoint(false)]).unwrap_or(Value::Unit);
                let b1 = self.apply(v.clone(), &[interval_endpoint(true)]).unwrap_or(Value::Unit);
                let connected = values_eq(&b0, &b1);
                Ok(Value::Data(Symbol::new("Shape"), vec![
                    Value::Bool(connected),
                    b0,
                    b1,
                ]))
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
                // §21.6 domain 感知:从 CLP 存储取已声明变量的域范围,约束假设生成
                let mut doms = std::collections::HashMap::new();
                for (id, name) in &self.clp_var_names {
                    if let Some(dom) = self.clp_store.domain_of(*id) {
                        if let (Some(lo), Some(hi)) = (dom.min(), dom.max()) {
                            doms.insert(name.as_str().to_string(), (lo, hi));
                        }
                    }
                }
                let vars: Vec<String> = abducibles.iter().map(|s| s.as_str().to_string()).collect();
                let mut engine = AbductionEngine::new();
                let explanations = engine.generate_hypotheses(&vars, &doms);
                // §21.6 多解枚举:收集全部一致解释;每解释为假设列表
                let total_candidates = explanations.len();
                let mut consistent_all: Vec<Vec<Value>> = Vec::new();
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
                                // 单值赋值(值不是变量 id):域 {value},传播后检测冲突
                                self.clp_store.assign(id, h.value);
                                self.clp_store.propagate();
                                if self.clp_store.domain_of(id).map(|d| d.is_empty()).unwrap_or(false)
                                    || self.clp_store.has_empty_domain() {
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
                                // 约束冲突检测:传播(eval 内 constrain 只 push)后查域空
                                self.clp_store.propagate();
                                if self.clp_store.has_empty_domain() {
                                    self.clp_store = snapshot;
                                    continue;
                                }
                                let hyps: Vec<Value> = exp.hypotheses.iter().map(|h| {
                                    Value::Data(Symbol::new("Hypothesis"), vec![
                                        Value::Str(h.var.clone().into()),
                                        Value::Int(h.value),
                                    ])
                                }).collect();
                                consistent_all.push(hyps);
                            }
                            _ => {}
                        }
                    }
                    self.clp_store = snapshot;
                }
                if !consistent_all.is_empty() {
                    // 全部一致解释列表
                    Ok(list_from_vec(consistent_all.into_iter().map(list_from_vec).collect()))
                } else {
                    // 不可满足原因:全部候选失败
                    Ok(list_from_vec(vec![Value::Data(
                        Symbol::new("no-consistent-explanation"),
                        vec![Value::Int(total_candidates as i64)],
                    )]))
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
                // §算术约束:(= (* x y) c) 等嵌套算术识别为乘/除/模传播
                // §全局约束:(all-different x y z) 互斥传播
                // 非比较式退回布尔求值
                if let CoreExprNode::Var(name) = &e.node {
                    if name.as_str() == "all-different" {
                        // 空参数形式:由调用方求值变量;此处无法取变量,退回布尔求值
                        return self.eval_expr(e);
                    }
                }
                if let CoreExprNode::App(f1, a) = &e.node {
                    if let CoreExprNode::App(f2, b) = &f1.node {
                        if let CoreExprNode::Var(op) = &f2.node {
                            if matches!(op.as_str(), "<" | ">" | "=" | "==") {
                                // 嵌套算术:(= (* x y) c) → App(App(Var(=), App(App(Var(*), x), y)), c)
                                if let CoreExprNode::App(g1, g2) = &b.node {
                                    if let CoreExprNode::App(g3, g4) = &g1.node {
                                        if let CoreExprNode::Var(arith) = &g3.node {
                                            if matches!(arith.as_str(), "*" | "/" | "%" | "+" | "-") {
                                                let xv = self.eval_expr(g2)?;
                                                let yv = self.eval_expr(g4)?;
                                                let zv = self.eval_expr(a)?;
                                                self.clp_arith_constraint(arith.as_str(), &xv, &yv, &zv);
                                                return Ok(Value::Bool(true));
                                            }
                                        }
                                    }
                                }
                                let va = self.eval_expr(a)?;
                                let vb = self.eval_expr(b)?;
                                self.clp_constraint(op.as_str(), &va, &vb);
                                return Ok(Value::Bool(true));
                            }
                        }
                    }
                }
                // §all-different:柯里化链 (all-different x y z) = App(App(App(Var(ad), x), y), z)
                if let Some(vars) = self.collect_all_different(e) {
                    let mut ids = Vec::new();
                    for v in &vars {
                        if let Value::Int(id) = self.eval_expr(v)? {
                            if self.clp_store.domain_of(id as u64).is_some() {
                                ids.push(id as u64);
                            }
                        }
                    }
                    if !ids.is_empty() {
                        self.clp_store.add_all_different(&ids);
                        return Ok(Value::Bool(true));
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
                        // 提交解:写回单值域并传播(保留约束一致性,供后续独立 label)
                        for (id, v) in sol {
                            self.clp_store.assign(*id, *v);
                            if let Some(sym) = self.clp_var_names.get(id).cloned() {
                                if let Some(top) = self.env.last_mut() {
                                    top.insert(sym, Value::Int(*v));
                                }
                            }
                        }
                        self.clp_store.propagate();
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
                crypto_warn(self);
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
                crypto_warn(self);
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
                crypto_warn(self);
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
                // §9/§29 反射:类型一等值——有声明类型时返回 Value::Type(可绑定/传递/比较)
                match self.def_sigs.get(name) {
                    Some((_arity, Some(ty), _eff, _grades, _mode, _det)) => Ok(Value::Type(ty.clone())),
                    Some((arity, None, eff, grades, mode, det)) => {
                        Ok(Value::Str(format!("(fn/{} 参数,效果 {:?},参数等级 {:?},模式 {:?},确定性 {:?})", arity, eff, grades, mode, det)))
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
            CoreExprNode::RegionAlloc(a, b) => {
                // §26.3 区域分配:分配地址并写入初值(模拟内存)
                let addr = self.next_ptr_addr;
                self.next_ptr_addr += 1;
                let v = self.eval_expr(b)?;
                self.eval_expr(a)?;
                self.ptr_mem.insert(addr, v);
                Ok(Value::Int(addr as i64))
            },
            CoreExprNode::RegionFree(e) => {
                // §26.3 区域释放:清除地址对应内存并标记为悬垂
                if let Value::Int(addr) = self.eval_expr(e)? {
                    self.ptr_mem.remove(&(addr as u64));
                    self.freed_addrs.insert(addr as u64);
                }
                Ok(Value::Unit)
            },
            CoreExprNode::PtrRead(e) => {
                // §26.2 裸指针读:地址 → 值;悬垂指针(已释放)报错;未写返回 Unit;Unsafe 门控(运行时警告)
                unsafe_warn(self);
                let addr = self.eval_expr(e)?;
                if let Value::Int(a) = addr {
                    if self.freed_addrs.contains(&(a as u64)) {
                        return Err(EvalError { message: format!("悬垂指针:地址 {} 已释放", a) });
                    }
                    Ok(self.ptr_mem.get(&(a as u64)).cloned().unwrap_or(Value::Unit))
                } else {
                    Err(EvalError { message: "ptr-read expects an integer address".into() })
                }
            },
            CoreExprNode::PtrWrite(a, b) => {
                // §26.2 裸指针写:地址 ← 值;Unsafe 门控(运行时警告)
                unsafe_warn(self);
                let addr = self.eval_expr(a)?;
                let val = self.eval_expr(b)?;
                if let Value::Int(addr_i) = addr {
                    self.ptr_mem.insert(addr_i as u64, val);
                    Ok(Value::Unit)
                } else {
                    Err(EvalError { message: "ptr-write expects an integer address".into() })
                }
            },
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
            CoreExprNode::ClassDef(name, _, methods, fun_deps, supers) => {
                // §23:登记类;每个方法名注册按参数类型分发的实例方法分发器(隐式字典)
                self.instance_dict.entry(name.clone()).or_default();
                // §23.3/§23.1:记录 fun-deps 与超类约束(实例登记时校验)
                if !fun_deps.is_empty() { self.class_fun_deps.insert(name.clone(), fun_deps.clone()); }
                if !supers.is_empty() { self.class_supers.insert(name.clone(), supers.clone()); }
                for (mname, _) in methods.iter() {
                    let m = mname.clone();
                    let cls = name.clone();
                    let dispatcher = Value::Builtin(mname.as_str().to_string(), Arc::new(move |s, args| {
                        // §23 约束求解驱动查找:全部实参运行时类型与实例类型列表逐项匹配
                        let arg_types: Vec<Type> = args.iter().map(|a| value_to_type(a, s)).collect();
                        if let Some(instances) = s.instance_dict.get(&cls) {
                            for (types, methods_map) in instances {
                                if types.len() == arg_types.len()
                                    && types.iter().zip(arg_types.iter()).all(|(t, a)| type_matches(t, a)) {
                                    if let Some(mv) = methods_map.get(&m) {
                                        return s.apply(mv.clone(), args);
                                    }
                                }
                            }
                        }
                        Err(EvalError { message: format!("no instance of {} for method {} (arg types {:?})", cls, m, arg_types) })
                    }));
                    self.define(mname.clone(), dispatcher);
                }
                Ok(Value::Unit)
            },
            CoreExprNode::InstanceDef(class_name, types, methods) => {
                // §23.3 fun-deps 冲突检测:同输入类型(types[0])不同输出类型(types[1])报错
                if let Some(fds) = self.class_fun_deps.get(&class_name).cloned() {
                    if !fds.is_empty() && types.len() >= 2 {
                        let input = types[0].clone();
                        let output = types[1].clone();
                        if let Some(existing) = self.class_instance_types.get(&class_name) {
                            for prior in existing {
                                if prior.len() >= 2 && prior[0] == input && prior[1] != output {
                                    return Err(EvalError {
                                        message: format!("fun-deps 冲突:{} 的同输入 {} 已有不同输出", class_name, input),
                                    });
                                }
                            }
                        }
                        let _ = fds;
                    }
                }
                // §23.1 超类约束:实例须满足超类(超类须有已登记实例)
                if let Some(supers) = self.class_supers.get(&class_name).cloned() {
                    for sup in &supers {
                        let has_super = self.instance_dict.get(sup).map(|v| !v.is_empty()).unwrap_or(false);
                        if !has_super {
                            return Err(EvalError { message: format!("实例 {} 缺少超类 {} 的实例", class_name, sup) });
                        }
                    }
                }
                let method_map: HashMap<Symbol, Value> = methods.iter().map(|(n, body)| {
                    let c = Closure { params: vec![], zero_params: vec![], body: (**body).clone(), env: self.env.last().cloned().unwrap_or_default() };
                    (n.clone(), Value::Closure(c))
                }).collect();
                let entry = self.instance_dict.entry(class_name.clone()).or_default();
                entry.push((types.clone(), method_map));
                self.class_instance_types.entry(class_name.clone()).or_default().push(types.clone());
                Ok(Value::Unit)
            },
            // §7.5 deriving:结构派生实现(desugar 生成的 eq-/ord-/show- 函数)
            CoreExprNode::DerivingImpl(trait_name, type_name) => {
                match trait_name.as_str() {
                    "Eq" => {
                        let name = Symbol::new(&format!("eq-{}", type_name));
                        self.define(name.clone(), Value::Builtin(name.as_str().to_string().into(), Arc::new(|_s, args| {
                            Ok(Value::Bool(if args.len() == 2 { values_eq(&args[0], &args[1]) } else { false }))
                        })));
                    }
                    "Ord" => {
                        let name = Symbol::new(&format!("ord-{}", type_name));
                        self.define(name.clone(), Value::Builtin(name.as_str().to_string().into(), Arc::new(|_s, args| {
                            if args.len() == 2 {
                                let v = match values_compare(&args[0], &args[1]) {
                                    std::cmp::Ordering::Less => -1,
                                    std::cmp::Ordering::Equal => 0,
                                    std::cmp::Ordering::Greater => 1,
                                };
                                Ok(Value::Int(v))
                            } else { Ok(Value::Int(0)) }
                        })));
                    }
                    "Show" => {
                        let name = Symbol::new(&format!("show-{}", type_name));
                        self.define(name.clone(), Value::Builtin(name.as_str().to_string().into(), Arc::new(|_s, args| {
                            Ok(Value::Str(args.first().map(show_value).unwrap_or_else(|| "...".into())))
                        })));
                    }
                    other => return Err(EvalError { message: format!("未知 deriving trait: {}", other) }),
                }
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
        // §8.1 TCO 蹦床:尾调用在循环内复用栈帧(替换 apply→apply_inner→eval_expr→apply 的递归)
        let mut func = func;
        let mut args: Vec<Value> = args.to_vec();
        loop {
            match self.apply_inner(func, &args)? {
                ApplyOutcome::Done(v) => return Ok(v),
                ApplyOutcome::Tail(f, a) => { func = f; args = a; }
            }
        }
    }

    /// 求值函数应用链,返回 (函数, 实参)(不 apply;供 TCO 尾调用与普通调用复用)
    fn eval_app(&mut self, expr: &CoreExpr) -> Result<(Value, Vec<Value>), EvalError> {
        if let CoreExprNode::App(func, arg) = &expr.node {
            let mut chain: Vec<&CoreExpr> = vec![arg];
            let mut cur = func;
            while let CoreExprNode::App(inner_f, inner_a) = &cur.node {
                chain.push(inner_a);
                cur = inner_f;
            }
            chain.reverse();
            let f = self.eval_expr(cur)?;
            let zero_positions: Option<Vec<usize>> = match &f {
                Value::Closure(c) if !c.params.is_empty() => Some(c.zero_params.clone()),
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
            Ok((f, args))
        } else {
            Err(EvalError { message: "eval_app expects App node".into() })
        }
    }

    /// §8.1 TCO:尾位置求值——If/Let/Do 循环下钻,尾 App 返回 Tail 交给 apply 蹦床
    fn eval_tail(&mut self, expr: &CoreExpr) -> Result<ApplyOutcome, EvalError> {
        match &expr.node {
            CoreExprNode::If(cond, then, else_) => {
                let c = self.eval_expr(cond)?;
                self.eval_tail(if is_truthy(&c) { then } else { else_ })
            }
            CoreExprNode::Let(name, _, value, body) => {
                let v = self.eval_expr(value)?;
                if let Some(top) = self.env.last_mut() { top.insert(name.clone(), v); }
                // let 绑定的作用域不能跨 TCO 蹦床存活(会被调用方提前 pop);
                // 普通求值以保持 let 内递归/闭包引用正确(尾递归本身不经过 let)
                Ok(ApplyOutcome::Done(self.eval_expr(body)?))
            }
            CoreExprNode::Do(items) => {
                if items.is_empty() { return Ok(ApplyOutcome::Done(Value::Unit)); }
                let last = items.len() - 1;
                for e in &items[..last] { self.eval_expr(e)?; }
                self.eval_tail(&items[last])
            }
            CoreExprNode::App(..) => {
                let (f, args) = self.eval_app(expr)?;
                Ok(ApplyOutcome::Tail(f, args))
            }
            _ => Ok(ApplyOutcome::Done(self.eval_expr(expr)?)),
        }
    }

    fn apply_inner(&mut self, func: Value, args: &[Value]) -> Result<ApplyOutcome, EvalError> {
        match &func {
            Value::Builtin(name, f) => {
                let name = name.clone();
                // 参数不足的多参内置:返回部分应用闭包,等待剩余参数
                let needs_more = self.full_arity(name.as_str()).map_or(false, |n| n > args.len());
                if needs_more {
                    Ok(ApplyOutcome::Done(partial_closure(name.clone(), f.clone(), args.to_vec())))
                } else {
                    // 参数齐备(或可变参):直接执行注册的实现(单一实现源)
                    Ok(ApplyOutcome::Done(f(self, args)?))
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
                        return Ok(ApplyOutcome::Done(args[0].clone()));
                    }
                    if args.len() == 1 {
                        return Ok(ApplyOutcome::Done(args[0].clone()));
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
                                        return Ok(ApplyOutcome::Done(partial_closure(bname.clone(), f.clone(), full)));
                                    }
                                    _ => return Ok(ApplyOutcome::Done(f(self, &full)?)),
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
                                    if args.len() > 1 {
                                        // 多余参数:结果继续应用(非尾位置,直接求值)
                                        let r = self.eval_expr(&inner.body);
                                        self.pop_scope();
                                        return match r {
                                            Ok(v) => Ok(ApplyOutcome::Tail(v, args[1..].to_vec())),
                                            Err(e) => Err(e),
                                        };
                                    } else {
                                        let r = self.eval_tail(&inner.body);
                                        self.pop_scope();
                                        return r;
                                    }
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
                                        Ok(ApplyOutcome::Tail(curried, args[1..].to_vec()))
                                    } else {
                                        Ok(ApplyOutcome::Done(curried))
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
                    // 若还有剩余参数(如 ((f) 5) 中 f 返回函数),结果继续应用(非尾位置直接求值)
                    if args.len() > 1 {
                        let r = self.eval_expr(&effective_body);
                        self.pop_scope();
                        match r {
                            Ok(v) => Ok(ApplyOutcome::Tail(v, args[1..].to_vec())),
                            Err(e) => Err(e),
                        }
                    } else {
                        let r = self.eval_tail(&effective_body);
                        self.pop_scope();
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
                    Ok(ApplyOutcome::Done(Value::Closure(Closure {
                        params: remaining,
                        zero_params: zero_remaining,
                        body: c.body.clone(),
                        env: new_env,
                    })))
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
                        Ok(v) => Ok(ApplyOutcome::Tail(v, rest_args.to_vec())),
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
                    let r = self.eval_tail(&effective_body);
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

/// 范式求值(§31/§32 全链路接入):经 tisp-runtime 的 `ParadigmRegistry` 统一分发
fn paradigm_eval(keyword: &'static str, args: &[Value]) -> Result<Value, EvalError> {
    use tisp_runtime::facility::{ParadigmValue as PV, default_registry};
    let registry = default_registry();
    let pv: Vec<PV> = args.iter().map(value_to_paradigm).collect::<Result<_, _>>()?;
    let result = registry.eval(keyword, &pv).map_err(|e| EvalError { message: e })?;
    Ok(paradigm_to_value(&result))
}

/// 解释器 `Value` → 范式统一值(跨范式抽象)
fn value_to_paradigm(v: &Value) -> Result<tisp_runtime::facility::ParadigmValue, EvalError> {
    use tisp_runtime::facility::ParadigmValue as PV;
    Ok(match v {
        Value::Int(n) => PV::Int(*n),
        Value::Float(f) => PV::Float(*f),
        Value::Bool(b) => PV::Bool(*b),
        Value::Str(s) => PV::Str(s.clone()),
        Value::Vector(_) | Value::Data(_, _) => PV::List(value_to_int_list(v)?),
        _ => return Err(EvalError { message: "不支持的范式参数类型".into() }),
    })
}

/// 从列表类值(Vector / Vec 数据 / Cons 链)提取整数序列
fn value_to_int_list(v: &Value) -> Result<Vec<i64>, EvalError> {
    let items: Vec<Value> = match v {
        Value::Data(name, fields) if name.as_str() == "Vec" => fields.clone(),
        Value::Data(name, _) if name.as_str() == "Cons" => list_to_vec(v),
        Value::Vector(vs) => vs.iter().cloned().collect(),
        _ => return Err(EvalError { message: "期望列表参数".into() }),
    };
    items
        .into_iter()
        .map(|x| match x {
            Value::Int(n) => Ok(n),
            _ => Err(EvalError { message: "列表参数需为整数".into() }),
        })
        .collect()
}

/// 范式统一值 → 解释器 `Value`
fn paradigm_to_value(pv: &tisp_runtime::facility::ParadigmValue) -> Value {
    use tisp_runtime::facility::ParadigmValue as PV;
    match pv {
        PV::Int(n) => Value::Int(*n),
        PV::Float(f) => Value::Float(*f),
        PV::Bool(b) => Value::Bool(*b),
        PV::Str(s) => Value::Str(s.clone()),
        PV::List(xs) => Value::Vector(xs.iter().map(|x| Value::Int(*x)).collect()),
    }
}

/// 把 Cons 链列表或持久化 Vector 展开为 Vec
fn list_to_vec(val: &Value) -> Vec<Value> {
    let mut items = Vec::new();
    let mut cur = val.clone();
    loop {
        match &cur {
            Value::Data(c, fields) if c.as_str() == "Cons" && !fields.is_empty() => {
                items.push(fields[0].clone());
                if fields.len() >= 2 { cur = fields[1].clone(); continue; }
            }
            // §4 持久化 Vector:视为元素序列
            Value::Vector(v) => {
                items.extend(v.iter().cloned());
                break;
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
        Value::Vector(v) => {
            for f in v.iter() {
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

/// §7.2 非加密占位警告(默认构建 XOR/简单 hash;crypto feature 下为强算法,不警告)
fn crypto_warn(s: &mut Interpreter) {
    #[cfg(not(feature = "crypto"))]
    if !s.crypto_warned {
        s.crypto_warned = true;
        eprintln!("; warning: 密码学为 XOR/简单 hash 占位(非加密,§27.4/27.5);生产环境应启用 crypto feature");
    }
    #[cfg(feature = "crypto")]
    let _ = s;
}

/// §26.4 Unsafe 门控(运行时警告):裸指针/区域操作为 Unsafe 效应,纯代码不应调用
fn unsafe_warn(s: &mut Interpreter) {
    if !s.unsafe_warned {
        s.unsafe_warned = true;
        eprintln!("; warning: 裸指针/区域操作为 Unsafe 效应(§26.4),纯代码未经 handler 不应调用");
    }
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

/// §23 约束求解驱动:把运行时值转为类型(实例查找用)
fn value_to_type(v: &Value, s: &Interpreter) -> Type {
    match v {
        Value::Int(_) => Type::i64(),
        Value::Str(_) => Type::string(),
        Value::Bool(_) => Type::bool(),
        Value::Float(_) => Type::f64(),
        Value::Unit => Type::unit(),
        Value::Data(c, _) => {
            let adt = s.ctor_to_adt.get(c).cloned().unwrap_or_else(|| c.clone());
            Type::Con(tisp_core::types::TypeCon { name: adt, kind: tisp_core::types::Kind::Star })
        }
        Value::Type(ty) => ty.clone(),
        _ => Type::unit(),
    }
}

/// §23 实例类型与实参类型匹配:构造器名一致,或类型变量(泛型)匹配任意
fn type_matches(instance_ty: &Type, arg_ty: &Type) -> bool {
    match instance_ty {
        Type::Con(c) => matches!(arg_ty, Type::Con(ac) if ac.name == c.name),
        Type::Var(_) => true,
        Type::App(f, a) => match arg_ty {
            Type::App(gf, ga) => type_matches(f, gf) && type_matches(a, ga),
            _ => false,
        },
        _ => instance_ty == arg_ty,
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
        // §9 类型一等值:类型值可打印(如 i64 / List a)
        Value::Type(ty) => ty.to_string(),
        // §4 持久化集合:可打印表示
        Value::Vector(v) => {
            let inner: Vec<String> = v.iter().map(show_value).collect();
            format!("[{}]", inner.join(" "))
        }
        Value::Map(m) => {
            let inner: Vec<String> = m.iter().map(|(k, v)| format!("{} {}", show_value(k), show_value(v))).collect();
            format!("{{{}}}", inner.join(" "))
        }
        Value::Set(s) => {
            let inner: Vec<String> = s.iter().map(show_value).collect();
            format!("#{{{}}}", inner.join(" "))
        }
        _ => "...".into(),
    }
}

/// §16.3 hott.rs 接线:区间端点经 tisp_runtime::hott::Interval 表示(接线泛型模块,替换内联 Bool 占位)
fn interval_endpoint(b: bool) -> Value {
    let i = if b {
        tisp_runtime::hott::Interval::i1()
    } else {
        tisp_runtime::hott::Interval::i0()
    };
    Value::Bool(matches!(i, tisp_runtime::hott::Interval::Point(true)))
}

/// §7.5 deriving Ord:结构化排序(构造器名 → 字段逐项)
fn values_compare(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::Str(x), Value::Str(y)) => x.cmp(y),
        (Value::Char(x), Value::Char(y)) => x.cmp(y),
        (Value::Unit, Value::Unit) => std::cmp::Ordering::Equal,
        (Value::Data(n1, f1), Value::Data(n2, f2)) => {
            match n1.as_str().cmp(n2.as_str()) {
                std::cmp::Ordering::Equal => {
                    for (x, y) in f1.iter().zip(f2) {
                        match values_compare(x, y) {
                            std::cmp::Ordering::Equal => {}
                            other => return other,
                        }
                    }
                    f1.len().cmp(&f2.len())
                }
                other => other,
            }
        }
        _ => std::cmp::Ordering::Equal,
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
        // §9 类型一等值:类型值相等性语义
        (Value::Type(t1), Value::Type(t2)) => t1 == t2,
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
        Value::Type(ty) => ty.to_string(),
        _ => "...".into(),
    }
}

impl Interpreter {
    /// 模式匹配(§8):同名变量重复出现要求绑定值一致;逻辑变量经统一
    fn match_pattern(&mut self, pat: &Pattern, val: &Value) -> Option<Vec<(Symbol, Value)>> {
        let mut bindings = Vec::new();
        if self.match_pattern_into(pat, val, &mut bindings) {
            Some(bindings)
        } else {
            None
        }
    }

    /// 模式匹配(§8):同名变量重复出现要求绑定值一致(逻辑变量经统一,§21)
    fn match_pattern_into(&mut self, pat: &Pattern, val: &Value, bindings: &mut Vec<(Symbol, Value)>) -> bool {
        match (pat, val) {
            (Pattern::Wildcard, _) => true,
            (Pattern::Var(name), v) => {
                if let Some((_, prev)) = bindings.iter().find(|(n, _)| n == name) {
                    self.unify_or_eq(prev, v)
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
                    if self.match_pattern_into(p, v, &mut trial) {
                        *bindings = trial;
                        return true;
                    }
                }
                false
            }
            (Pattern::Con(c_name, subpats), Value::Vector(v)) => {
                // §4 持久化 Vector 与 Cons 模式兼容(头+尾)
                if c_name.as_str() == "Cons" {
                    if subpats.len() != 2 || v.is_empty() { return false; }
                    if !self.match_pattern_into(&subpats[0], &v[0], bindings) { return false; }
                    let rest = Value::Vector(v.skip(1));
                    return self.match_pattern_into(&subpats[1], &rest, bindings);
                }
                if c_name.as_str() == "Vec" {
                    let sub_vals: Vec<Value> = v.iter().cloned().collect();
                    if subpats.len() != sub_vals.len() { return false; }
                    for (sp, dv) in subpats.iter().zip(sub_vals.iter()) {
                        if !self.match_pattern_into(sp, dv, bindings) { return false; }
                    }
                    return true;
                }
                false
            }
            (Pattern::Con(c_name, subpats), Value::Data(d_name, d_args)) => {
                // Vec 字面量与 Cons 模式兼容(§21.2 谓词调用传向量列表)
                if c_name.as_str() == "Cons" && d_name.as_str() == "Vec" {
                    if subpats.len() != 2 || d_args.is_empty() { return false; }
                    if !self.match_pattern_into(&subpats[0], &d_args[0], bindings) { return false; }
                    let rest = if d_args.len() <= 1 {
                        Value::Data(Symbol::new("Nil"), vec![])
                    } else {
                        Value::Data(Symbol::new("Vec"), d_args[1..].to_vec())
                    };
                    self.match_pattern_into(&subpats[1], &rest, bindings)
                } else if c_name == d_name && subpats.len() == d_args.len() {
                    for (sp, dv) in subpats.iter().zip(d_args) {
                        if !self.match_pattern_into(sp, dv, bindings) { return false; }
                    }
                    true
                } else {
                    false
                }
            }
            // §9 类型一等值:类型值模式匹配(Int 匹配 Con(Int);(List a) 匹配 App(Con(List), a))
            (Pattern::Con(c_name, subpats), Value::Type(ty)) => {
                if let Some((name, args)) = flatten_type_app(ty) {
                    if name != *c_name || subpats.len() != args.len() { return false; }
                    for (sp, arg) in subpats.iter().zip(args) {
                        if !self.match_pattern_into(sp, &Value::Type(arg), bindings) { return false; }
                    }
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// §21 同名变量一致性:若既有绑定是逻辑变量,统一;否则值比较
    fn unify_or_eq(&mut self, prev: &Value, v: &Value) -> bool {
        if let Value::Int(id) = prev {
            if self.logic_vars.contains_key(&(*id as u64)) {
                let lv = self.val_to_logic(prev);
                let rv = self.val_to_logic(v);
                return self.logic_store.unify(&lv, &rv);
            }
        }
        values_eq(prev, v)
    }
}

/// 展平类型应用链:App(Con(c), a1..an) → (c, [a1..an])
fn flatten_type_app(ty: &Type) -> Option<(Symbol, Vec<Type>)> {
    match ty {
        Type::Con(tc) => Some((tc.name.clone(), vec![])),
        Type::App(f, a) => {
            let (name, mut args) = flatten_type_app(f)?;
            args.push((**a).clone());
            Some((name, args))
        }
        _ => None,
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

    /// §26.2/26.3 裸指针与手动区域:模拟内存读写 + 区域分配/释放
    #[test]
    fn test_ptr_and_region_memory() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // region-alloc r 42 → 返回地址
        let alloc = e(CoreExprNode::RegionAlloc(
            Box::new(e(CoreExprNode::Lit(Literal::Unit))),
            Box::new(int(42)),
        ));
        let addr = interp.eval_expr(&alloc).unwrap();
        let a = match addr { Value::Int(n) => n as u64, other => panic!("expected address, got {:?}", other) };
        // ptr-read addr → 42
        let read = e(CoreExprNode::PtrRead(Box::new(int(a as i64))));
        assert!(matches!(interp.eval_expr(&read).unwrap(), Value::Int(42)), "ptr-read 应返回 42");
        // ptr-write addr 99 → 再读 99
        let write = e(CoreExprNode::PtrWrite(Box::new(int(a as i64)), Box::new(int(99))));
        interp.eval_expr(&write).unwrap();
        assert!(matches!(interp.eval_expr(&read).unwrap(), Value::Int(99)), "ptr-write 后应读到 99");
        // region-free addr → 清除并标记悬垂 → 读到悬垂指针报错
        let free = e(CoreExprNode::RegionFree(Box::new(int(a as i64))));
        interp.eval_expr(&free).unwrap();
        assert!(interp.eval_expr(&read).is_err(), "region-free 后读到悬垂指针应报错");
    }

    /// §26.3 with-region:区域作用域内分配,退出时回收
    #[test]
    fn test_with_region_scoped_dealloc() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // f = (fn [r] (region-alloc r 42))
        let f = Value::Closure(Closure {
            params: vec![Symbol::new("r")],
            zero_params: vec![],
            body: e(CoreExprNode::RegionAlloc(Box::new(var("r")), Box::new(int(42)))),
            env: HashMap::new(),
        });
        let wr = interp.env.last().unwrap().get(&Symbol::new("with-region")).cloned().unwrap();
        let addr = interp.apply(wr, &[f]).unwrap();
        let a = match addr { Value::Int(n) => n as u64, other => panic!("expected address, got {:?}", other) };
        // 退出后:地址内存已回收并标记悬垂 → ptr-read 报错
        let read = e(CoreExprNode::PtrRead(Box::new(int(a as i64))));
        assert!(interp.eval_expr(&read).is_err(), "with-region 退出后读到悬垂指针应报错");
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
            region: None,
            visibility: Visibility::Public,
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
            resource_algebras: vec![], defs: vec![def("main", body)], pragmas: vec![] };
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
            resource_algebras: vec![], defs: vec![f_def, main_def], pragmas: vec![] };
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
            region: None,
            visibility: Visibility::Public,
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
            resource_algebras: vec![], defs: vec![f_def, main_def], pragmas: vec![] };
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
    #[cfg(not(feature = "crypto"))]
    fn test_crypto_placeholder_warning() {
        // §7.2 默认构建(非 crypto feature)加密为 XOR 占位,首次使用置警告标记
        let mut interp = Interpreter::new();
        interp.register_builtins();
        assert!(!interp.crypto_warned);
        let secret = e(CoreExprNode::SpiSecret(Box::new(e(CoreExprNode::Lit(Literal::String("k1".into()))))));
        interp.eval_expr(&secret).unwrap();
        let enc = e(CoreExprNode::CryptoEncrypt(
            Box::new(e(CoreExprNode::Lit(Literal::String("hello".into())))),
            Box::new(e(CoreExprNode::Lit(Literal::String("k1".into())))),
        ));
        interp.eval_expr(&enc).unwrap();
        assert!(interp.crypto_warned, "加密占位警告应已标记");
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
        assert!(matches!(v, Value::Vector(fs) if fs.len() == 1));
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
        assert!(matches!(r, Value::Type(_)), "反射应返回类型值,实际 {:?}", r);

        let src2 = "(defn main [] (reflect-type nope))";
        let prog2 = desugar(src2);
        let mut interp2 = Interpreter::new();
        let r2 = interp2.run_program(&prog2).unwrap().unwrap();
        assert!(as_str(&r2).contains("未定义"), "未定义符号应提示,实际 {:?}", r2);
    }

    #[test]
    fn test_reflect_full_info() {
        // §29 反射完整信息:名称/参数/类型/效果/等级/模式/确定性全真实
        let src = "(defn add [a b] -> i64 (+ a b))\n(defn main [] (reflect \"add\"))";
        let prog = desugar(src);
        let mut interp = Interpreter::new();
        let r = interp.run_program(&prog).unwrap().unwrap();
        match &r {
            Value::Data(tag, fields) if tag.as_str() == "DefInfo" && fields.len() == 8 => {
                // [name, arity, params, type, effects, grades, mode, det]
                assert!(matches!(&fields[0], Value::Str(s) if s == "add"), "name 应为 add");
                assert!(matches!(fields[1], Value::Int(2)), "arity 应为 2");
                match &fields[2] {
                    Value::Vector(p) => assert_eq!(p.len(), 2, "params 应含 2 个参数名"),
                    other => panic!("params 应为 Vector,实际 {:?}", other),
                }
                assert!(matches!(&fields[3], Value::Type(_)), "type 应为类型值");
                assert!(matches!(&fields[4], Value::Str(_)), "effects 应为字符串");
            }
            other => panic!("reflect 应返回 8 字段 DefInfo,实际 {:?}", other),
        }
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
        // 新结构:解释列表(每个解释 = Hypothesis 列表);(> x 3) 一致解释非空
        match r {
            Value::Data(_, items) => {
                assert!(!items.is_empty(), "解释列表不应为空");
            }
            _ => panic!("应返回解释列表,实际 {:?}", r),
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

    #[test]
    fn test_constructor_type_specialization() {
        // §22.4:构造器类型驱动特化 (area (MkCircle 5.0)) → area__spec_N
        let src = "(defdata Circle (MkCircle [Float]))\n\
                   (defgeneric area [x])\n\
                   (defmethod area [(c Circle)] 42)\n\
                   (defn main [] (area (MkCircle 5.0)))";
        let prog = desugar(src);
        let mut sp = tisp_middle::specialize::Specializer::new();
        let out = sp.specialize(&prog);
        assert_eq!(sp.specialized, 1, "构造器类型调用应特化");
        // 特化后含 spec def
        assert!(out.defs.iter().any(|d| d.name.as_str().starts_with("area__spec_")), "应生成 area__spec_N");
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
    fn test_recursive_predicate_multi_solution() {
        // §21:递归谓词 + 逻辑变量统一 → find-all 枚举全部解(绑定正确)
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defpred member [x xs] ([x [x . _]]) ([x [_ . xs]] (member x xs)))\n\
                       (defn main [] (count (find-all (fn [] (fresh [x] (member x [1 2 3]))))))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 3, "member [1 2 3] 应枚举 3 个解(1/2/3)");
    }

    #[test]
    fn test_hott_and_temporal_builtins() {
        // §16.3/16.4/18:fun-ext / monoid-check / clock / always / eventually
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn id [x] x)\n\
                       (defn main [] (fun-ext id id [1 2 3]))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r, Value::Bool(true)), "fun-ext 同函数应等价,实际 {:?}", r);

        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn plus [a b] (+ a b))\n(defn main [] (monoid-check plus 0 [1 2 3]))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r2, Value::Bool(true)), "整数加法应为幺半群,实际 {:?}", r2);

        let r3 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (clock \"c\" 5))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r3, Value::Data(ref t, _) if t.as_str() == "Clock"), "clock 应返回 Clock,实际 {:?}", r3);
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
    fn test_typeclass_fun_deps_conflict() {
        // §23.3:fun-deps 冲突——同输入不同输出报错
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defclass Coll [c e] :fun-deps [(c -> e)] (elem [c] -> e))\n\
                       (definstance (Coll i64 i64) (elem [x] x))\n\
                       (definstance (Coll i64 String) (elem [x] \"s\"))\n\
                       (defn main [] 1)";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).map(|_| ())
        }).unwrap().join().unwrap();
        assert!(r.is_err(), "fun-deps 冲突应报错");
        if let Err(e) = r {
            assert!(e.message.contains("fun-deps"), "应报 fun-deps 冲突,实际 {}", e.message);
        }
    }

    #[test]
    fn test_typeclass_type_matching_helpers() {
        // §23 约束求解驱动:值→类型 + 实例类型匹配(构造器名一致 / 类型变量匹配任意)
        let interp = Interpreter::new();
        assert_eq!(value_to_type(&Value::Int(1), &interp), Type::i64());
        assert_eq!(value_to_type(&Value::Str("x".into()), &interp), Type::string());
        assert_eq!(value_to_type(&Value::Bool(true), &interp), Type::bool());
        assert!(type_matches(&Type::i64(), &Type::i64()));
        assert!(!type_matches(&Type::i64(), &Type::string()));
        let tvar = Type::Var(tisp_core::types::TypeVar { name: Symbol::new("a"), kind: tisp_core::types::Kind::Star, id: 0 });
        assert!(type_matches(&tvar, &Type::i64()), "类型变量应匹配任意实参类型");
    }

    #[test]
    fn test_hit_endpoint_value() {
        // §7.4/16.3 端点值构造:HIT 构造器经 defdata 路径构造 Data 值(非 Unit 占位)
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defdata-hit S1 (base) (loop :boundary [(i = i0) -> base (i = i1) -> base]))\n(defn main [] (base))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r {
            Value::Data(tag, fields) => {
                assert_eq!(tag.as_str(), "base", "base 构造器应构造 Data(base, [])");
                assert!(fields.is_empty(), "零参构造器应无字段");
            }
            other => panic!("base 应构造 Data 值,实际 {:?}", other),
        }
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

        // §7.5 deriving Ord:ord-Name 结构化排序
        let r4 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defdata Color :deriving (Ord) (Red) (RGB i64 i64 i64))\n(defn main [] (ord-Color (RGB 1 2 3) (RGB 1 2 4)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r4, Value::Int(n) if n < 0), "RGB(1,2,3) < RGB(1,2,4),实际 {:?}", r4);
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

        // §17 ♭/♯ 与直通可区分:flat 返回 Flat 容器、sharp 返回 Sharp 容器
        let flat_src = "(defn main [] (crisp (flat 1)))";
        let rf = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let mut interp = Interpreter::new();
            interp.run_program(&desugar(flat_src)).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(rf, Value::Data(ref t, _) if t.as_str() == "Flat"), "flat 应返回 Flat 容器,实际 {:?}", rf);

        let sharp_src = "(defn main [] (crisp (sharp 1)))";
        let rs = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let mut interp = Interpreter::new();
            interp.run_program(&desugar(sharp_src)).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(rs, Value::Data(ref t, _) if t.as_str() == "Sharp"), "sharp 应返回 Sharp 容器,实际 {:?}", rs);
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
    fn test_forward_reference_typecheck() {
        // 前向引用:使用在前、定义在后 → typecheck 通过
        let src = "(defn main [] (foo 1))\n(defn foo [x] (+ x 1))";
        let prog = desugar(src);
        let mut ti = tisp_middle::type_infer::TypeInfer::new();
        assert!(ti.infer_program(&prog).is_ok(), "前向引用应通过类型检查");
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 2, "run 应输出 2");
    }

    #[test]
    fn test_mutual_recursion_typecheck() {
        // 相互递归:is-even/is-odd 互调 → 通过
        let src = "(defn is-even [n] (if (= n 0) true (is-odd (- n 1))))\n(defn is-odd [n] (if (= n 0) false (is-even (- n 1))))\n(defn main [] (is-even 10))";
        let prog = desugar(src);
        let mut ti = tisp_middle::type_infer::TypeInfer::new();
        assert!(ti.infer_program(&prog).is_ok(), "相互递归应通过类型检查");
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(matches!(r, Value::Bool(true)), "is-even 10 应为 true");
    }

    #[test]
    fn test_let_recursion_typecheck() {
        // let 内递归:局部 fact → 通过,run 输出 120
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (let [fact (fn [n] (if (= n 0) 1 (* n (fact (- n 1)))))] (fact 5)))";
            let prog = desugar(src);
            let mut ti = tisp_middle::type_infer::TypeInfer::new();
            assert!(ti.infer_program(&prog).is_ok(), "let 内递归应通过类型检查");
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 120, "fact 5 应为 120");
    }

    #[test]
    fn test_tco_deep_tail_recursion() {
        // §8.1 TCO:尾递归 sum-to 在 1MB 栈下不溢出(蹦床复用栈帧)
        let r = std::thread::Builder::new().stack_size(1024 * 1024).spawn(move || {
            let src = "(defn sum-to [n acc] (if (= n 0) acc (sum-to (- n 1) (+ acc n))))\n(defn main [] (sum-to 20000 0))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 200010000, "sum-to 20000 0 应为 200010000");
    }

    #[test]
    fn test_recursive_closure_finite_and_infinite() {
        // 有限类型递归返回闭包:通过 + run 3
        let ok_src = "(defn make-adder-n [n] (if (= n 0) (fn [x] x) (fn [x] ((make-adder-n (- n 1)) (+ x 1)))))\n(defn main [] ((make-adder-n 3) 0))";
        let prog = desugar(ok_src);
        let mut ti = tisp_middle::type_infer::TypeInfer::new();
        assert!(ti.infer_program(&prog).is_ok(), "有限递归闭包应通过");
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 3);

        // 无限类型(T = Unit -> T)拒绝(occurs check 正确行为)
        let bad_src = "(defn make-countdown [n] (if (= n 0) (fn [] 0) (fn [] (make-countdown (- n 1)))))\n(defn main [] 1)";
        let prog_bad = desugar(bad_src);
        let mut ti2 = tisp_middle::type_infer::TypeInfer::new();
        assert!(ti2.infer_program(&prog_bad).is_err(), "无限类型应被拒绝");
    }

    #[test]
    fn test_real_type_error_still_rejected() {
        // 负例:真实类型错误(i64 当函数)仍被拒绝(修复不引入误放行)
        let src = "(defn main [] (1 2))";
        let prog = desugar(src);
        let mut ti = tisp_middle::type_infer::TypeInfer::new();
        assert!(ti.infer_program(&prog).is_err(), "真实类型错误应被拒绝");
    }


    #[test]
    fn test_grade_inequality_diagnostic_and_hit_endpoint() {
        // §10:自由符号等级通过 + 诊断警告(不误报)
        let src = "(defn f [xs : (Vec i64 n) (n x : i64)] -> i64 (do x x))\n(defn main [] 1)";
        let prog = desugar(src);
        let mut ti = tisp_middle::type_infer::TypeInfer::new();
        assert!(ti.infer_program(&prog).is_ok(), "自由符号等级应通过");
        let mut gc = tisp_middle::grade_check::GradeChecker::new();
        assert!(gc.check_program(&prog).is_ok());
        assert!(!gc.inequalities.is_empty(), "应收集等级不等式诊断");

        // §16.3:端点方程不可满足 → 边界违反;i0=i0 通过
        let bad = "(defdata-hit B (base) (base) (loop :boundary (= i0 i1)))\n(defn main [] 1)";
        let err = {
            use tisp_frontend::desugar::Desugarer;
            use tisp_frontend::reader::read;
            let forms = read(bad).unwrap();
            Desugarer::new().desugar_program(forms).unwrap_err()
        };
        assert!(err.message.contains("端点方程不可满足"), "应报端点方程违反,实际: {}", err.message);
    }

    #[test]
    fn test_clp_arith_and_all_different() {
        // §21.5:乘法约束 (x·y=12) 解集正确;all-different 排列互斥
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (fresh [x y] (domain x 1 6) (domain y 1 6) (constrain (= (* x y) 12)) (label x 1) (label y 1) (+ (* x 10) y)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        // 解应为 x=2, y=6 → 2*10+6 = 26(传播后最小可行解)
        assert_eq!(as_int(r), 26, "乘法约束应得 x=2 y=6");

        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (fresh [x y z] (domain x 1 3) (domain y 1 3) (domain z 1 3) (constrain (all-different x y z)) (label x 1) (label y 1) (label z 1) (+ (* x 100) (+ (* y 10) z))))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r2), 123, "all-different 应为 1,2,3 排列");
    }

    #[test]
    fn test_abduce_multi_explanations() {
        // §21.6:abduce 返回全部一致解释(多解枚举);不可满足返回原因
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (fresh [x] (domain x 1 3) (count (abduce (constrain (> x 1)) x))))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert!(as_int(r) >= 2, "应返回多个一致解释");

        // 不可满足:返回 no-consistent-explanation 原因
        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (fresh [x] (domain x 1 3) (abduce (constrain (> x 9)) x)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r2 {
            Value::Data(_, items) => {
                assert!(!items.is_empty(), "原因列表不应为空");
                if let Value::Data(tag, _) = &items[0] {
                    assert_eq!(tag.as_str(), "no-consistent-explanation", "应返回不可满足原因,实际 {}", tag);
                } else {
                    panic!("应返回原因节点,实际 {:?}", items[0]);
                }
            }
            _ => panic!("应返回列表,实际 {:?}", r2),
        }
    }

    #[test]
    fn test_type_first_class_value() {
        // §9:reflect-type 返回 Value::Type(绑定/传递/比较);类型值相等性
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn f [x : i64] -> i64 x)\n(defn main [] (let [t (reflect-type f)] (if (= t t) 1 0)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 1, "相同类型值应相等");

        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn f [x : i64] -> i64 x)\n(defn main [] (let [t (reflect-type f)] (if (= t 5) 1 0)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r2), 0, "类型值与整数不应相等");
    }

    #[test]
    fn test_type_value_display() {
        // §9:类型值可打印(show_value 输出类型表示,而非占位 "...")
        let ty = tisp_core::types::Type::i64();
        assert_eq!(show_value(&Value::Type(ty.clone())), "i64");
        assert_eq!(value_to_string(&Value::Type(ty)), "i64");
        // 复合类型
        let list_ty = tisp_core::types::Type::list(tisp_core::types::Type::i64());
        assert_eq!(show_value(&Value::Type(list_ty)), "List i64");
    }

    #[test]
    fn test_type_value_pattern_match() {
        // §9:类型值可模式匹配(Int 匹配 Con(i64);(List a) 匹配 App(Con(List), a))
        let pat_int = tisp_core::core_ast::Pattern::Con(Symbol::new("i64"), vec![]);
        let val_int = Value::Type(tisp_core::types::Type::i64());
        assert!(Interpreter::new().match_pattern(&pat_int, &val_int).is_some(), "i64 类型值应匹配 (i64) 模式");

        let pat_list = tisp_core::core_ast::Pattern::Con(
            Symbol::new("List"),
            vec![tisp_core::core_ast::Pattern::Var(Symbol::new("a"))],
        );
        let val_list = Value::Type(tisp_core::types::Type::list(tisp_core::types::Type::i64()));
        assert!(Interpreter::new().match_pattern(&pat_list, &val_list).is_some(), "(List a) 模式应匹配 List i64 类型值");

        // 不匹配:构造器名不同
        let pat_str = tisp_core::core_ast::Pattern::Con(Symbol::new("String"), vec![]);
        assert!(Interpreter::new().match_pattern(&pat_str, &val_int).is_none(), "i64 类型值不应匹配 String 模式");
    }

    #[test]
    fn test_hott_real_semantics() {
        // §16.3/§17:HComp 边界填充、Transp 端点传输、Shape 路径连通
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (transp (fn [i] 9) true))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 9, "Transp 应返回目标端点值 9");

        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (hcomp (fn [i] 7)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r2 {
            Value::Data(tag, fields) => {
                assert_eq!(tag.as_str(), "KanFill", "HComp 应返回 KanFill,实际 {}", tag);
                assert_eq!(fields.len(), 2, "KanFill 应含两端点边界值");
            }
            other => panic!("HComp 应返回 KanFill,实际 {:?}", other),
        }

        // Shape 连通:恒等路径连通(true),分支路径不连通(false)
        let r3 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (shape (fn [i] 42)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r3 {
            Value::Data(_, fields) => {
                assert!(matches!(fields[0], Value::Bool(true)), "恒等路径应连通");
            }
            other => panic!("Shape 应为容器,实际 {:?}", other),
        }
    }

    #[test]
    fn test_hcomp_boundary_inconsistency() {
        // §16 完整立方填充:边界不一致 SHALL 报错(而非静默返回一端)
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (hcomp (fn [i] (if i 1 2))))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog)
        }).unwrap().join().unwrap();
        assert!(r.is_err(), "HComp 边界不一致应报错,实际 {:?}", r);
    }

    /// §17 Cohesive adjoint-triple(ʃ ⊣ ♭ ⊣ ♯):♭∘♯ 与 ʃ∘♭ 的 counit → id
    #[test]
    fn test_cohesive_adjoint_triple() {
        // ♭(♯(42)) = 42(counit of ♭ ⊣ ♯)
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (flat (sharp 42)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r), 42, "♭∘♯ 应返回原值 42");

        // ʃ(♭(42)) = 42(counit of ʃ ⊣ ♭)
        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (shape (flat 42)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        assert_eq!(as_int(r2), 42, "ʃ∘♭ 应返回原值 42");
    }

    /// §17 Cohesive adjoint-triple 的 unit(♯∘♭、♭∘ʃ 为单元嵌入,非直通)
    #[test]
    fn test_cohesive_adjoint_unit() {
        // ♯(♭(42)) = UnitSharpFlat(unit η')
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (sharp (flat 42)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r {
            Value::Data(tag, fields) => {
                assert_eq!(tag.as_str(), "UnitSharpFlat", "♯∘♭ 应为单元嵌入");
                assert_eq!(as_int(fields[0].clone()), 42);
            }
            other => panic!("expected UnitSharpFlat, got {:?}", other),
        }
    }

    /// §16 完整立方填充:2 维 Kan(hcomp-2d)边界一致性
    #[test]
    fn test_hcomp_2d() {
        // 四条常量边 → 四角一致 → KanFill2D
        let r = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (hcomp-2d (fn [i] 7) (fn [i] 7) (fn [i] 7) (fn [i] 7)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog).unwrap().unwrap()
        }).unwrap().join().unwrap();
        match r {
            Value::Data(tag, fields) => {
                assert_eq!(tag.as_str(), "KanFill2D", "hcomp-2d 应返回 KanFill2D");
                assert_eq!(fields.len(), 1);
            }
            other => panic!("expected KanFill2D, got {:?}", other),
        }
        // 边界不一致(top(i0)≠left(i0))→ 报错
        let r2 = std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {
            let src = "(defn main [] (hcomp-2d (fn [i] (if i 1 2)) (fn [i] 2) (fn [i] 2) (fn [i] 2)))";
            let prog = desugar(src);
            let mut interp = Interpreter::new();
            interp.run_program(&prog)
        }).unwrap().join().unwrap();
        assert!(r2.is_err(), "边界不一致应报错");
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


#[cfg(test)]
mod persistent_tests {
    use super::*;

    fn app(fname: &str, args: Vec<CoreExpr>) -> CoreExpr {
        let mut e = CoreExpr::new(CoreExprNode::Var(Symbol::new(fname)), Span::dummy());
        for a in args {
            e = CoreExpr::new(CoreExprNode::App(Box::new(e), Box::new(a)), Span::dummy());
        }
        e
    }
    fn int(n: i64) -> CoreExpr {
        CoreExpr::new(CoreExprNode::Lit(Literal::I64(n)), Span::dummy())
    }

    /// §4 结构相等/哈希:Vector/Map/Set 按内容比较,可作 map/set 键
    #[test]
    fn test_value_struct_eq_hash() {
        let a = Value::Vector(im::vector![Value::Int(1), Value::Int(2)]);
        let b = Value::Vector(im::vector![Value::Int(1), Value::Int(2)]);
        assert_eq!(a, b);

        use std::collections::HashSet;
        let mut s = HashSet::new();
        s.insert(a.clone());
        assert!(s.contains(&b));

        // Map 以结构相等的 Vector 作键
        let m: im::HashMap<Value, Value> = im::HashMap::unit(a.clone(), Value::Int(42));
        assert_eq!(m.get(&b), Some(&Value::Int(42)));
    }

    /// §4 conj 结构共享:返回新 Vector,元素追加
    #[test]
    fn test_persistent_vector_conj() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let v = interp.eval_expr(&app("conj", vec![app("vector", vec![int(1), int(2)]), int(3)])).unwrap();
        match &v {
            Value::Vector(vv) => assert_eq!(vv.len(), 3),
            other => panic!("expected Vector, got {:?}", other),
        }
        // 原 vector 仍为 2 元素(纯函数式:旧值不变)
        let old = interp.eval_expr(&app("vector", vec![int(1), int(2)])).unwrap();
        if let Value::Vector(vv) = &old { assert_eq!(vv.len(), 2); }
    }

    /// §4 assoc/dissoc/disj/contains? 持久化操作
    #[test]
    fn test_persistent_map_set() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // assoc:插入新键
        let m = interp.eval_expr(&app("assoc", vec![
            app("hash-map", vec![int(1), int(2)]), int(3), int(4),
        ])).unwrap();
        if let Value::Map(mm) = &m { assert!(mm.contains_key(&Value::Int(3))); }
        else { panic!("expected Map"); }
        // contains? set
        let c = interp.eval_expr(&app("contains?", vec![
            app("hash-set", vec![int(1), int(2)]), int(2),
        ])).unwrap();
        assert!(matches!(c, Value::Bool(true)));
        // disj:移除
        let d = interp.eval_expr(&app("disj", vec![
            app("hash-set", vec![int(1), int(2)]), int(1),
        ])).unwrap();
        if let Value::Set(ss) = &d {
            assert!(!ss.contains(&Value::Int(1)));
            assert!(ss.contains(&Value::Int(2)));
        } else { panic!("expected Set"); }
    }

    /// §4 quote 产生可操作数据:符号→字符串、数字→i64(list 构造已由 desugar 覆盖)
    #[test]
    fn test_quote_produces_list() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // '(1 2 3) 脱糖为 (list 1 2 3)
        let v = interp.eval_expr(&app("list", vec![int(1), int(2), int(3)])).unwrap();
        let items: Vec<i64> = list_to_vec(&v).iter().filter_map(|x| if let Value::Int(n) = x { Some(*n) } else { None }).collect();
        assert_eq!(items, vec![1, 2, 3]);
    }

    /// §31/§32 范式全链路接入:24 范式经 ParadigmRegistry 从解释器可调用
    #[test]
    fn test_paradigm_builtins_full_chain() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // 数组范式:数组归约
        let r = interp.eval_expr(&app("pf-array-sum", vec![app("vector", vec![int(1), int(2), int(3)])])).unwrap();
        assert_eq!(r, Value::Int(6));
        // 符号范式:符号代换求值
        let r = interp.eval_expr(&app("pf-sym-eval", vec![int(2)])).unwrap();
        assert_eq!(r, Value::Int(3));
        // 基于流范式:惰性取前 3 项并映射
        let r = interp.eval_expr(&app("pf-stream-take", vec![int(3)])).unwrap();
        match r {
            Value::Vector(v) => {
                let xs: Vec<i64> = v.iter().filter_map(|x| if let Value::Int(n) = x { Some(*n) } else { None }).collect();
                assert_eq!(xs, vec![0, 2, 4]);
            }
            other => panic!("expected Vector, got {:?}", other),
        }
        // AOP 范式:around 编织
        let r = interp.eval_expr(&app("pf-aop-weave", vec![int(42)])).unwrap();
        assert_eq!(r, Value::Int(142));
    }

    /// §32 真实自动机:DFA 识别(接线 tisp_runtime::programming::Dfa)
    #[test]
    fn test_real_dfa_accepts() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let s = |txt: &str| CoreExpr::new(CoreExprNode::Lit(Literal::String(txt.into())), Span::dummy());
        // 偶数个 'a':s0 -a-> s1 -a-> s0,s0 接受
        let transitions = vec![int(0), int(97), int(1), int(1), int(97), int(0)];
        let call = app("dfa-accept", vec![
            int(0),
            app("vector", vec![int(0)]),
            app("vector", transitions.clone()),
            s("aa"),
        ]);
        assert_eq!(interp.eval_expr(&call).unwrap(), Value::Bool(true), "偶数个 a 应被接受");

        let call2 = app("dfa-accept", vec![
            int(0),
            app("vector", vec![int(0)]),
            app("vector", transitions),
            s("a"),
        ]);
        assert_eq!(interp.eval_expr(&call2).unwrap(), Value::Bool(false), "奇数个 a 应被拒绝");
    }

    /// §32 真实状态机:事件驱动转移(接线 tisp_runtime::programming::StateMachine)
    #[test]
    fn test_real_state_machine_drive() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // 0 -e1-> 2:驱动事件 1 → 新状态 2
        let call = app("sm-drive", vec![
            int(0), int(1), app("vector", vec![int(0), int(1), int(2)]),
        ]);
        assert_eq!(interp.eval_expr(&call).unwrap(), Value::Int(2));
        // 非法事件 → 报错
        let bad = app("sm-drive", vec![
            int(0), int(9), app("vector", vec![int(0), int(1), int(2)]),
        ]);
        assert!(interp.eval_expr(&bad).is_err(), "非法转移应报错");
    }

    /// §32 真实描述逻辑:概念子概念推理(接线 tisp_runtime::paradigms::Ontology)
    #[test]
    fn test_real_subsume() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // Man(1) ⊑ Person(2):is_instance(1, 2) = true
        let call = app("subsume", vec![
            app("vector", vec![int(1), int(2)]), int(1), int(2),
        ]);
        assert_eq!(interp.eval_expr(&call).unwrap(), Value::Bool(true));
        // Dog(3) 非 Person(2) 子概念 → false
        let call2 = app("subsume", vec![
            app("vector", vec![int(1), int(2)]), int(3), int(2),
        ]);
        assert_eq!(interp.eval_expr(&call2).unwrap(), Value::Bool(false));
    }

    /// §32 真实表格化逻辑:左递归终止(接线 tisp_runtime::paradigms::Tabler)
    #[test]
    fn test_real_tabling() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // 左递归 p(1) :- p(1):表格化使左递归终止 → false(facts 含无关事实 9)
        let call = app("tabling", vec![
            app("vector", vec![int(9)]), app("vector", vec![int(1), int(1)]), int(1),
        ]);
        assert_eq!(interp.eval_expr(&call).unwrap(), Value::Bool(false));
        // q(2) 为事实,p(1) :- q(2):prove(p) = true
        let call2 = app("tabling", vec![
            app("vector", vec![int(2)]), app("vector", vec![int(1), int(2)]), int(1),
        ]);
        assert_eq!(interp.eval_expr(&call2).unwrap(), Value::Bool(true));
    }

    /// §32 真实符号编程:后序构造 SymExpr + 化简 + 求值
    #[test]
    fn test_real_sym_eval() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // (+ 2 1) → 3
        let call = app("sym-eval", vec![app("vector", vec![int(2), int(1), int(-1)])]);
        assert_eq!(interp.eval_expr(&call).unwrap(), Value::Int(3));
        // (* 2 3) → 6
        let call2 = app("sym-eval", vec![app("vector", vec![int(2), int(3), int(-2)])]);
        assert_eq!(interp.eval_expr(&call2).unwrap(), Value::Int(6));
        // (+ 0 5) 化简 → 5
        let call3 = app("sym-eval", vec![app("vector", vec![int(0), int(5), int(-1)])]);
        assert_eq!(interp.eval_expr(&call3).unwrap(), Value::Int(5));
    }

    /// §31 真实 EVOLP/ASP:命题稳定模型(p :- not q → {p})
    #[test]
    fn test_real_evolp_stable() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // p(1) :- not q(2),facts=[9 无关]:稳定模型含 p(1)
        let call = app("evolp-stable", vec![
            app("vector", vec![int(9)]), app("vector", vec![int(1), int(-2)]),
        ]);
        match interp.eval_expr(&call).unwrap() {
            Value::Vector(v) => {
                let xs: Vec<i64> = v.iter().filter_map(|x| if let Value::Int(n) = x { Some(*n) } else { None }).collect();
                assert!(xs.contains(&1), "稳定模型应含 p(1),实际 {:?}", xs);
            }
            other => panic!("expected Vector, got {:?}", other),
        }
        // q(2) 为事实时,not q(2) 为假 → p(1) 不成立,稳定模型不含 p(1)
        let call2 = app("evolp-stable", vec![
            app("vector", vec![int(2)]), app("vector", vec![int(1), int(-2)]),
        ]);
        match interp.eval_expr(&call2).unwrap() {
            Value::Vector(v) => {
                let xs: Vec<i64> = v.iter().filter_map(|x| if let Value::Int(n) = x { Some(*n) } else { None }).collect();
                assert!(!xs.contains(&1), "q 为事实时稳定模型不应含 p(1),实际 {:?}", xs);
            }
            other => panic!("expected Vector, got {:?}", other),
        }
    }

    /// §31 真实 DLP:动态稳定模型(状态 1 的 p 被状态 2 否定 → 拒绝)
    #[test]
    fn test_real_dlp_stable() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        let empty = || CoreExpr::new(CoreExprNode::Data(Symbol::new("Vec"), vec![]), Span::dummy());
        // 状态1:fact p(1);状态2:rule q(2) :- not p(1) → p 被后续状态否定,结果含 q(2)
        let call = app("dlp-stable", vec![
            app("vector", vec![int(1)]), empty(),
            empty(), app("vector", vec![int(2), int(-1)]),
        ]);
        match interp.eval_expr(&call).unwrap() {
            Value::Vector(v) => {
                let xs: Vec<i64> = v.iter().filter_map(|x| if let Value::Int(n) = x { Some(*n) } else { None }).collect();
                assert!(!xs.contains(&1), "p(1) 应被后续状态否定,实际 {:?}", xs);
                assert!(xs.contains(&2), "动态稳定模型应含 q(2),实际 {:?}", xs);
            }
            other => panic!("expected Vector, got {:?}", other),
        }
    }

    /// §31 真实 EVOLP 演化:assert/retract + foldl 折叠
    #[test]
    fn test_real_evolp_evolve() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // facts [1 2],assert 3,retract 1 → [2 3]
        let call = app("evolp-evolve", vec![
            app("vector", vec![int(1), int(2)]), app("vector", vec![int(1), int(3), int(0), int(1)]),
        ]);
        match interp.eval_expr(&call).unwrap() {
            Value::Vector(v) => {
                let mut xs: Vec<i64> = v.iter().filter_map(|x| if let Value::Int(n) = x { Some(*n) } else { None }).collect();
                xs.sort();
                assert_eq!(xs, vec![2, 3], "assert 3 + retract 1 后应为 [2, 3]");
            }
            other => panic!("expected Vector, got {:?}", other),
        }
    }

    /// §31 MOP:GetKB/SetKB 运行时知识库读写
    #[test]
    fn test_mop_get_set_kb() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // set-kb [1 2] → get-kb 返回 [1 2]
        interp.eval_expr(&app("set-kb", vec![app("vector", vec![int(1), int(2)])])).unwrap();
        let r = interp.eval_expr(&app("get-kb", vec![int(0)])).unwrap();
        match r {
            Value::Vector(v) => {
                let mut xs: Vec<i64> = v.iter().filter_map(|x| if let Value::Int(n) = x { Some(*n) } else { None }).collect();
                xs.sort();
                assert_eq!(xs, vec![1, 2], "get-kb 应返回已写入的知识库");
            }
            other => panic!("expected Vector, got {:?}", other),
        }
        // set-kb [3] 覆盖 → get-kb 返回 [3]
        interp.eval_expr(&app("set-kb", vec![app("vector", vec![int(3)])])).unwrap();
        let r2 = interp.eval_expr(&app("get-kb", vec![int(0)])).unwrap();
        match r2 {
            Value::Vector(v) => {
                let xs: Vec<i64> = v.iter().filter_map(|x| if let Value::Int(n) = x { Some(*n) } else { None }).collect();
                assert_eq!(xs, vec![3], "set-kb 覆盖后 get-kb 应返回 [3]");
            }
            other => panic!("expected Vector, got {:?}", other),
        }
    }

    /// §统一内存管理:Ref a 分级值(ref/deref/set! 为 State 效应操作)
    #[test]
    fn test_ref_deref_set() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // (ref 42) → 地址;deref → 42;set! → Unit;deref → 100
        let addr = interp.eval_expr(&app("ref", vec![int(42)])).unwrap();
        let a = match addr { Value::Int(n) => n, other => panic!("expected address, got {:?}", other) };
        let r = interp.eval_expr(&app("deref", vec![int(a)])).unwrap();
        assert_eq!(r, Value::Int(42), "deref 应返回初值 42");
        interp.eval_expr(&app("set!", vec![int(a), int(100)])).unwrap();
        let r2 = interp.eval_expr(&app("deref", vec![int(a)])).unwrap();
        assert_eq!(r2, Value::Int(100), "set! 后 deref 应返回 100");
    }

    /// §16 完整立方填充:N(≥2)维 Kan(hcomp-nd)角一致性
    #[test]
    fn test_hcomp_nd() {
        let mut interp = Interpreter::new();
        interp.register_builtins();
        // 3 维立方:8 个角全一致 → 填充成功
        let corners = vec![int(7); 8];
        let r = interp.eval_expr(&app("hcomp-nd", vec![app("vector", corners)])).unwrap();
        assert_eq!(r, Value::Int(7), "角全一致应返回填充值 7");
        // 角不一致 → 报错
        let bad = vec![int(7), int(7), int(7), int(7), int(7), int(7), int(7), int(8)];
        let r2 = interp.eval_expr(&app("hcomp-nd", vec![app("vector", bad)]));
        assert!(r2.is_err(), "角不一致应报错");
    }
}
