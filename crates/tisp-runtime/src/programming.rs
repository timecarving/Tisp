//! 8 类编程范式(组合优先,纯声明式副作用管理)
//!
//! 数组 / 栈 / 连接式 / 符号 / 自动机 / 状态机 / 数据驱动 / 基于流。
//! 副作用经代数效应/单子管理;全部为纯函数 + ADT 数据。
use std::collections::HashMap;

// ── 数组编程 ──

/// 多维数组(行主序扁平存储)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array<T> {
    pub shape: Vec<usize>,
    pub data: Vec<T>,
}

impl<T: Clone> Array<T> {
    /// 以行主序数据构造(元素总数须等于 shape 乘积)
    pub fn new(shape: Vec<usize>, data: Vec<T>) -> Self {
        let total: usize = shape.iter().product();
        assert_eq!(total, data.len(), "shape 与 data 长度不一致");
        Array { shape, data }
    }

    /// 校验构造:shape/data 不匹配返回可读错误(§1.1 显式错误)
    pub fn new_checked(shape: Vec<usize>, data: Vec<T>) -> Result<Self, String> {
        let total: usize = shape.iter().product();
        if total != data.len() {
            return Err(format!("数组形状 {:?} 需要 {} 个元素,实际 {}", shape, total, data.len()));
        }
        Ok(Array { shape, data })
    }

    /// 查询形状(维度/长度)
    pub fn dims(&self) -> &[usize] {
        &self.shape
    }

    /// 多维系索引
    pub fn index(&self, idx: &[usize]) -> Option<&T> {
        self.index_checked(idx).ok()
    }

    /// 多维系索引:维度数不匹配/越界返回可读错误(§1.1 显式错误)
    pub fn index_checked(&self, idx: &[usize]) -> Result<&T, String> {
        if idx.len() != self.shape.len() {
            return Err(format!("索引维数 {} 与数组维数 {} 不一致", idx.len(), self.shape.len()));
        }
        let mut flat = 0usize;
        let mut stride = 1usize;
        for (coord, &dim) in idx.iter().rev().zip(self.shape.iter().rev()) {
            if *coord >= dim {
                return Err(format!("索引 {:?} 越界:维 {} 长度 {}", idx, coord, dim));
            }
            flat += *coord * stride;
            stride *= dim;
        }
        self.data.get(flat).ok_or_else(|| format!("索引 {:?} 越界", idx))
    }

    /// 一维切片:[lo, hi)(闭区间右开);返回新数组(纯函数)
    pub fn slice(&self, lo: usize, hi: usize) -> Result<Array<T>, String> {
        if self.shape.len() != 1 {
            return Err("切片当前仅支持一维数组".into());
        }
        let n = self.shape[0];
        if lo > hi || hi > n {
            return Err(format!("切片区间 [{}, {}) 越界:长度 {}", lo, hi, n));
        }
        Ok(Array { shape: vec![hi - lo], data: self.data[lo..hi].to_vec() })
    }

    /// 逐元素映射(纯函数,原数组不变)
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> Array<U> {
        Array { shape: self.shape.clone(), data: self.data.iter().map(f).collect() }
    }

    /// 归约(折叠全部元素)
    pub fn reduce<U>(&self, init: U, f: impl Fn(U, &T) -> U) -> U {
        self.data.iter().fold(init, f)
    }
}

impl Array<i64> {
    /// 二维数组沿轴 0 求和(列和)
    pub fn sum_axis0(&self) -> Vec<i64> {
        if self.shape.len() != 2 {
            return self.data.iter().copied().collect();
        }
        let cols = self.shape[1];
        let rows = self.shape[0];
        let mut out = vec![0i64; cols];
        for r in 0..rows {
            for c in 0..cols {
                if let Some(v) = self.index(&[r, c]) {
                    out[c] += v;
                }
            }
        }
        out
    }
}

// ── 栈编程 ──

/// 数据栈(纯函数:操作消费并返回新栈)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stack<T>(Vec<T>);

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack(Vec::new())
    }

    pub fn push(mut self, v: T) -> Self {
        self.0.push(v);
        self
    }

    pub fn pop(mut self) -> (Self, Option<T>) {
        let v = self.0.pop();
        (self, v)
    }

    /// pop 显式错误(空栈不静默返回 None)
    pub fn pop_checked(mut self) -> Result<(Self, T), String> {
        match self.0.pop() {
            Some(v) => Ok((self, v)),
            None => Err("pop on empty stack".into()),
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.0.last()
    }

    /// peek 显式错误(空栈不静默返回 None)
    pub fn peek_checked(&self) -> Result<&T, String> {
        self.0.last().ok_or_else(|| "peek on empty stack".into())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 旋转:把栈顶第 n 项移到栈顶(纯函数)
    pub fn rotate(mut self, n: usize) -> Result<Self, String> {
        let len = self.0.len();
        if len == 0 {
            return Err("rotate on empty stack".into());
        }
        let n = n % len;
        if n == 0 {
            return Ok(self);
        }
        let v = self.0.remove(len - 1 - n);
        self.0.push(v);
        Ok(self)
    }
}

impl<T: Clone> Stack<T> {
    /// 复制栈顶
    pub fn dup(mut self) -> Self {
        if let Some(t) = self.0.last().cloned() {
            self.0.push(t);
        }
        self
    }

    /// 交换栈顶两项
    pub fn swap(mut self) -> Self {
        let n = self.0.len();
        if n >= 2 {
            self.0.swap(n - 1, n - 2);
        }
        self
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ── 连接式编程 ──

/// 点自由组合:compose(f, g) = g ∘ f
pub fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |a| g(f(a))
}

/// 点自由应用
pub fn apply<A, B>(f: impl Fn(A) -> B, a: A) -> B {
    f(a)
}

/// 分支组合子(点自由 if)
pub fn branch<A, B: Clone>(cond: bool, then: B, otherwise: B) -> B {
    if cond { then } else { otherwise }
}

// ── 符号编程 ──

/// 符号表达式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymExpr {
    Num(i64),
    Var(String),
    Add(Box<SymExpr>, Box<SymExpr>),
    Mul(Box<SymExpr>, Box<SymExpr>),
}

impl SymExpr {
    /// 变量代换
    pub fn substitute(&self, var: &str, val: i64) -> SymExpr {
        match self {
            SymExpr::Var(v) if v == var => SymExpr::Num(val),
            SymExpr::Add(a, b) => SymExpr::Add(Box::new(a.substitute(var, val)), Box::new(b.substitute(var, val))),
            SymExpr::Mul(a, b) => SymExpr::Mul(Box::new(a.substitute(var, val)), Box::new(b.substitute(var, val))),
            other => other.clone(),
        }
    }

    /// 化简(常量折叠)
    pub fn simplify(&self) -> SymExpr {
        match self {
            SymExpr::Add(a, b) => {
                let (a, b) = (a.simplify(), b.simplify());
                match (a, b) {
                    (SymExpr::Num(x), SymExpr::Num(y)) => SymExpr::Num(x + y),
                    (SymExpr::Num(0), e) | (e, SymExpr::Num(0)) => e,
                    (a, b) => SymExpr::Add(Box::new(a), Box::new(b)),
                }
            }
            SymExpr::Mul(a, b) => {
                let (a, b) = (a.simplify(), b.simplify());
                match (a, b) {
                    (SymExpr::Num(x), SymExpr::Num(y)) => SymExpr::Num(x * y),
                    (SymExpr::Num(0), _) | (_, SymExpr::Num(0)) => SymExpr::Num(0),
                    (SymExpr::Num(1), e) | (e, SymExpr::Num(1)) => e,
                    (a, b) => SymExpr::Mul(Box::new(a), Box::new(b)),
                }
            }
            other => other.clone(),
        }
    }

    /// 求值(假设无自由变量)
    pub fn eval(&self) -> Option<i64> {
        match self {
            SymExpr::Num(n) => Some(*n),
            SymExpr::Var(_) => None,
            SymExpr::Add(a, b) => Some(a.eval()? + b.eval()?),
            SymExpr::Mul(a, b) => Some(a.eval()? * b.eval()?),
        }
    }

    /// 求值:含自由变量返回可读错误(§1.4 显式错误)
    pub fn eval_checked(&self) -> Result<i64, String> {
        match self {
            SymExpr::Num(n) => Ok(*n),
            SymExpr::Var(v) => Err(format!("符号表达式含自由变量 {}", v)),
            SymExpr::Add(a, b) => Ok(a.eval_checked()? + b.eval_checked()?),
            SymExpr::Mul(a, b) => Ok(a.eval_checked()? * b.eval_checked()?),
        }
    }
}

// ── 自动机编程 ──

/// 确定性有限自动机(DFA)
#[derive(Debug, Clone)]
pub struct Dfa {
    pub start: String,
    pub accept: im::HashSet<String>,
    /// (from, symbol, to)
    pub transitions: Vec<(String, char, String)>,
}

impl Dfa {
    /// 识别输入串(接受/拒绝)
    pub fn accepts(&self, input: &str) -> bool {
        let mut state = self.start.clone();
        for c in input.chars() {
            match self.transitions.iter().find(|(from, sym, _)| *from == state && *sym == c) {
                Some((_, _, to)) => state = to.clone(),
                None => return false,
            }
        }
        self.accept.contains(&state)
    }

    /// 识别输入串:未声明符号/未声明状态转移 SHALL 显式报错;
    /// 支持组合自动机的 ε 转移('\0')。
    pub fn accepts_checked(&self, input: &str) -> Result<bool, String> {
        let mut states: im::HashSet<String> = im::HashSet::new();
        states.insert(self.start.clone());
        self.epsilon_close(&mut states);
        for c in input.chars() {
            let mut next: im::HashSet<String> = im::HashSet::new();
            let mut declared_any = false;
            for from in &states {
                for (f, sym, to) in &self.transitions {
                    if f == from && *sym == c {
                        next.insert(to.clone());
                        declared_any = true;
                    }
                }
            }
            if next.is_empty() {
                return Err(format!("DFA 非法输入:当前状态集 {:?} 对符号 '{}' 无转移", states, c));
            }
            let _ = declared_any;
            self.epsilon_close(&mut next);
            states = next;
        }
        Ok(states.iter().any(|s| self.accept.contains(s)))
    }

    fn epsilon_close(&self, states: &mut im::HashSet<String>) {
        let mut changed = true;
        while changed {
            changed = false;
            let current: Vec<String> = states.iter().cloned().collect();
            for from in current {
                for (f, sym, to) in &self.transitions {
                    if f == &from && *sym == '\0' && states.insert(to.clone()).is_none() {
                        changed = true;
                    }
                }
            }
        }
    }

    /// 自动机并:接受两个自动机语言的并集(状态名加前缀避免冲突)
    pub fn union(&self, other: &Dfa) -> Dfa {
        let prefix_a = |s: &str| format!("a:{}", s);
        let prefix_b = |s: &str| format!("b:{}", s);
        let mut dfa = Dfa {
            start: "__union__".to_string(),
            accept: im::HashSet::new(),
            transitions: Vec::new(),
        };
        for (from, sym, to) in &self.transitions {
            dfa.transitions.push((prefix_a(from), *sym, prefix_a(to)));
        }
        for (from, sym, to) in &other.transitions {
            dfa.transitions.push((prefix_b(from), *sym, prefix_b(to)));
        }
        for s in &self.accept { dfa.accept.insert(prefix_a(s)); }
        for s in &other.accept { dfa.accept.insert(prefix_b(s)); }
        dfa.transitions.push(("__union__".into(), '\0', prefix_a(&self.start)));
        dfa.transitions.push(("__union__".into(), '\0', prefix_b(&other.start)));
        dfa
    }

    /// 自动机串联:先识别 self 再识别 other(接受 self 后进入 other 的启动)
    pub fn concat(&self, other: &Dfa) -> Dfa {
        let mut dfa = Dfa {
            start: self.start.clone(),
            accept: other.accept.clone(),
            transitions: self.transitions.clone(),
        };
        for (from, sym, to) in &other.transitions {
            dfa.transitions.push((from.clone(), *sym, to.clone()));
        }
        // self 的接受态以 ε 转移到 other.start;DFA 表示中用 '\0' 占位并在识别时跳过
        for acc in &self.accept {
            dfa.transitions.push((acc.clone(), '\0', other.start.clone()));
        }
        dfa
    }
}

// ── 状态机编程 ──

/// 显式状态机:状态/事件/转移/动作
#[derive(Debug, Clone)]
pub struct StateMachine {
    pub current: String,
    /// (from, event, to)
    pub transitions: Vec<(String, String, String)>,
    /// (state, event, action)——转移动作,成功转移时追加进 trace
    pub actions: Vec<(String, String, String)>,
    /// 转移动作日志(每次转移追加)
    pub trace: Vec<String>,
}

impl StateMachine {
    pub fn new(initial: &str) -> Self {
        StateMachine { current: initial.to_string(), transitions: Vec::new(), actions: Vec::new(), trace: Vec::new() }
    }

    /// 事件驱动转移:合法则转移并记录动作,非法则报错且状态不变
    pub fn drive(&mut self, event: &str) -> Result<(), String> {
        match self.transitions.iter().find(|(from, ev, _)| *from == self.current && ev == event) {
            Some((_, _, to)) => {
                let to = to.clone();
                let from = self.current.clone();
                for (st, ev, action) in &self.actions {
                    if *st == from && ev == event {
                        self.trace.push(action.clone());
                    }
                }
                self.trace.push(format!("{} --{}--> {}", from, event, to));
                self.current = to;
                Ok(())
            }
            None => Err(format!("非法转移:{} 在状态 {} 无事件 {}", self.current, event, event)),
        }
    }
}

// ── 数据驱动编程 ──

/// 数据驱动的分发表(行为由数据决定,非硬编码)
#[derive(Debug, Clone)]
pub struct DispatchTable {
    pub table: HashMap<String, fn(&str) -> String>,
}

impl DispatchTable {
    pub fn dispatch(&self, key: &str, arg: &str) -> Option<String> {
        self.table.get(key).map(|handler| handler(arg))
    }

    /// 查表分发:缺失键显式报错(§1.7 错误语义)
    pub fn dispatch_checked(&self, key: &str, arg: &str) -> Result<String, String> {
        match self.table.get(key) {
            Some(handler) => Ok(handler(arg)),
            None => Err(format!("分发表缺失键: {}", key)),
        }
    }
}

// ── 基于流编程 ──

/// 数据流节点:源 → 变换 → 汇,惰性迭代
pub fn source<T>(items: Vec<T>) -> impl Iterator<Item = T> {
    items.into_iter()
}

pub fn map_node<I, T, U>(iter: I, f: impl Fn(T) -> U) -> impl Iterator<Item = U>
where
    I: Iterator<Item = T>,
{
    iter.map(f)
}

pub fn filter_node<I, T>(iter: I, p: impl Fn(&T) -> bool) -> impl Iterator<Item = T>
where
    I: Iterator<Item = T>,
{
    iter.filter(p)
}

pub fn take_node<I>(iter: I, n: usize) -> impl Iterator<Item = I::Item>
where
    I: Iterator,
{
    iter.take(n)
}

pub fn sink<I, T>(iter: I) -> Vec<T>
where
    I: Iterator<Item = T>,
{
    iter.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_map_reduce() {
        let a = Array::new(vec![2, 2], vec![1, 2, 3, 4]);
        let b = a.map(|x| x + 1);
        assert_eq!(b.data, vec![2, 3, 4, 5]);
        assert_eq!(a.data, vec![1, 2, 3, 4]); // 原数组不变
        assert_eq!(a.reduce(0, |acc, x| acc + x), 10);
    }

    #[test]
    fn test_array_sum_axis0() {
        let a = Array::new(vec![2, 2], vec![1, 2, 3, 4]);
        assert_eq!(a.sum_axis0(), vec![4, 6]);
    }

    #[test]
    fn test_array_slice_index() {
        let a = Array::new(vec![2, 2], vec![1, 2, 3, 4]);
        assert_eq!(a.index(&[0, 1]), Some(&2));
        assert_eq!(a.index(&[2, 0]), None); // 越界
    }

    #[test]
    fn test_stack_ops() {
        let s = Stack::new().push(1).push(2).dup();
        assert_eq!(s.peek(), Some(&2));
        let s = s.swap();
        assert_eq!(s.peek(), Some(&2));
        let (s, top) = s.pop();
        assert_eq!(top, Some(2));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_concatenative() {
        let add1 = |x: i64| x + 1;
        let double = |x: i64| x * 2;
        let f = compose(add1, double);
        assert_eq!(apply(&f, 3), 8); // (3+1)*2
    }

    #[test]
    fn test_symbolic_substitute_simplify() {
        // (+ x 1) 代换 x=2 → 3
        let e = SymExpr::Add(Box::new(SymExpr::Var("x".into())), Box::new(SymExpr::Num(1)));
        assert_eq!(e.substitute("x", 2).eval(), Some(3));
        // (+ 0 5) 化简 → 5
        let e2 = SymExpr::Add(Box::new(SymExpr::Num(0)), Box::new(SymExpr::Num(5)));
        assert_eq!(e2.simplify(), SymExpr::Num(5));
    }

    #[test]
    fn test_dfa_accepts() {
        // 接受偶数个 'a':s0 -a-> s1 -a-> s0,s0 为接受态
        let dfa = Dfa {
            start: "s0".into(),
            accept: ["s0".to_string()].into_iter().collect(),
            transitions: vec![
                ("s0".into(), 'a', "s1".into()),
                ("s1".into(), 'a', "s0".into()),
            ],
        };
        assert!(dfa.accepts("aa"));
        assert!(!dfa.accepts("a"));
    }

    #[test]
    fn test_state_machine() {
        let mut sm = StateMachine::new("idle");
        sm.transitions = vec![("idle".into(), "start".into(), "running".into())];
        sm.drive("start").unwrap();
        assert_eq!(sm.current, "running");
        assert!(sm.drive("bogus").is_err());
        assert_eq!(sm.current, "running"); // 非法转移不改变状态
    }

    #[test]
    fn test_data_driven() {
        let dt = DispatchTable {
            table: [
                ("greet".to_string(), (|n: &str| format!("Hello, {}!", n)) as fn(&str) -> String),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(dt.dispatch("greet", "Tisp"), Some("Hello, Tisp!".into()));
        assert_eq!(dt.dispatch("nope", "x"), None);
    }

    #[test]
    fn test_stream_lazy() {
        // 无限流:0,1,2,... 惰性取前 3 项并映射(不卡死)
        let out = sink(map_node(take_node(0i64.., 3), |x| x * 2));
        assert_eq!(out, vec![0, 2, 4]);
    }

    #[test]
    fn test_array_checked_errors() {
        assert!(Array::new_checked(vec![2, 2], vec![1, 2, 3]).is_err(), "shape/data 不匹配应报错");
        let a = Array::new_checked(vec![2, 2], vec![1, 2, 3, 4]).unwrap();
        assert_eq!(a.index_checked(&[1, 1]).unwrap(), &4);
        assert!(a.index_checked(&[2, 0]).is_err(), "越界索引应报错");
        assert!(a.index_checked(&[1]).is_err(), "维数不匹配应报错");
        let one = Array::new_checked(vec![4], vec![1, 2, 3, 4]).unwrap();
        assert_eq!(one.slice(1, 3).unwrap().data, vec![2, 3]);
    }

    #[test]
    fn test_stack_checked_errors() {
        let s0 = Stack::<i64>::new();
        assert!(s0.peek_checked().is_err(), "空栈 peek 应报错");
        assert!(s0.pop_checked().is_err(), "空栈 pop 应报错");
        let s = Stack::<i64>::new().push(1).push(2).push(3);
        let s = s.rotate(1).unwrap();
        assert_eq!(s.peek_checked().unwrap(), &2, "rotate 1 应把栈顶下第 1 项移到栈顶");
        let (_, top) = s.pop_checked().unwrap();
        assert_eq!(top, 2);
    }

    #[test]
    fn test_sym_eval_checked() {
        let e = SymExpr::Add(Box::new(SymExpr::Var("x".into())), Box::new(SymExpr::Num(1)));
        assert!(e.eval_checked().is_err(), "自由变量应显式报错");
        assert_eq!(e.substitute("x", 2).eval_checked().unwrap(), 3);
    }

    #[test]
    fn test_dfa_union_concat_epsilon() {
        let dfa = Dfa {
            start: "s0".into(),
            accept: ["s0".to_string()].into_iter().collect(),
            transitions: vec![("s0".into(), 'a', "s1".into()), ("s1".into(), 'a', "s0".into())],
        };
        let u = dfa.union(&dfa);
        assert!(u.accepts_checked("aa").unwrap(), "并集应接受 aa");
        let c = dfa.concat(&dfa);
        assert!(c.accepts_checked("aaaa").unwrap(), "串联应接受 aaaa");
        assert!(c.accepts_checked("b").is_err(), "未声明符号应报错");
    }

    #[test]
    fn test_state_machine_actions() {
        let mut sm = StateMachine::new("idle");
        sm.transitions = vec![("idle".into(), "start".into(), "running".into())];
        sm.actions = vec![("idle".into(), "start".into(), "entry-running".into())];
        sm.drive("start").unwrap();
        assert_eq!(sm.current, "running");
        assert_eq!(sm.trace.last().unwrap(), "idle --start--> running");
        assert!(sm.trace.iter().any(|a| a == "entry-running"), "entry/exit 动作应记录");
        assert!(sm.drive("bogus").is_err());
        assert_eq!(sm.current, "running", "非法转移后状态不变");
    }

    #[test]
    fn test_dispatch_checked() {
        let dt = DispatchTable {
            table: [("greet".to_string(), (|n: &str| format!("Hello, {}!", n)) as fn(&str) -> String)]
                .into_iter().collect(),
        };
        assert_eq!(dt.dispatch_checked("greet", "Tisp").unwrap(), "Hello, Tisp!");
        assert!(dt.dispatch_checked("nope", "x").is_err(), "缺失键应显式报错");
    }
}
