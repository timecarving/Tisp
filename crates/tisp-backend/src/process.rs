use tisp_core::symbol::Symbol;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};

/// Process runtime supporting π-calculus channels
pub struct ProcessRuntime {
    channels: HashMap<Symbol, Channel>,
}

struct ChannelInner {
    queue: VecDeque<Value>,
    closed: bool,
}

#[derive(Clone)]
pub(crate) struct Channel {
    inner: Arc<(Mutex<ChannelInner>, Condvar)>,
}

impl Channel {
    pub(crate) fn send_value(&self, val: Value) {
        let (lock, cvar) = &*self.inner;
        let mut inner = lock.lock().unwrap();
        if !inner.closed {
            inner.queue.push_back(val);
            cvar.notify_one();
        }
    }

    /// 阻塞接收:空通道等待 send;close 后唤醒并返回 None
    pub(crate) fn recv_blocking(&self) -> Option<Value> {
        let (lock, cvar) = &*self.inner;
        let mut inner = lock.lock().unwrap();
        while inner.queue.is_empty() && !inner.closed {
            inner = cvar.wait(inner).unwrap();
        }
        inner.queue.pop_front()
    }

    pub(crate) fn try_recv_value(&self) -> Option<Value> {
        let (lock, _cvar) = &*self.inner;
        lock.lock().unwrap().queue.pop_front()
    }

    pub(crate) fn close_channel(&self) {
        let (lock, cvar) = &*self.inner;
        let mut inner = lock.lock().unwrap();
        inner.closed = true;
        // 关闭即释放缓冲队列中的待收负载(§统一内存管理:通道关闭/区域退出均释放)
        inner.queue.clear();
        cvar.notify_all();
    }

    pub(crate) fn is_closed_channel(&self) -> bool {
        self.inner.0.lock().unwrap().closed
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Unit,
    Chan(Symbol),
}

impl ProcessRuntime {
    pub fn new() -> Self {
        Self { channels: HashMap::new() }
    }

    pub fn new_channel(&mut self, name: Symbol) -> Value {
        self.channels.insert(name.clone(), Channel {
            inner: Arc::new((Mutex::new(ChannelInner { queue: VecDeque::new(), closed: false }), Condvar::new())),
        });
        Value::Chan(name)
    }

    /// 释放通道:关闭、清空缓冲队列并从通道表摘除(程序区域 pop 时经 RegionStack 钩子调用)
    pub fn release_channel(&mut self, name: &Symbol) {
        if let Some(ch) = self.channels.remove(name) {
            ch.close_channel();
        }
    }

    /// 取出通道句柄(短暂持锁),供调用方在不持有进程运行时锁的情况下阻塞等待
    pub(crate) fn get_channel(&self, chan_name: &Symbol) -> Option<Channel> {
        self.channels.get(chan_name).cloned()
    }

    pub fn send(&self, chan_name: &Symbol, val: Value) {
        if let Some(ch) = self.get_channel(chan_name) {
            ch.send_value(val);
        }
    }

    /// FIFO 阻塞接收(§27.2):空通道等待 send;close 后唤醒并返回 None。
    /// 注意:调用方若经 `Mutex<ProcessRuntime>` 持锁调用本方法会与 send 死锁,
    /// 阻塞路径请使用 get_channel + Channel::recv_blocking。
    pub fn recv(&self, chan_name: &Symbol) -> Option<Value> {
        self.get_channel(chan_name).and_then(|ch| ch.recv_blocking())
    }

    /// 非阻塞接收(async-recv):空通道立即返回 None
    pub fn try_recv(&self, chan_name: &Symbol) -> Option<Value> {
        self.get_channel(chan_name).and_then(|ch| ch.try_recv_value())
    }

    /// 关闭通道:唤醒全部等待者,后续 send 无效(§20 close 语义)
    pub fn close(&self, chan_name: &Symbol) {
        if let Some(ch) = self.get_channel(chan_name) {
            ch.close_channel();
        }
    }

    pub fn is_closed(&self, chan_name: &Symbol) -> bool {
        self.get_channel(chan_name).map(|ch| ch.is_closed_channel()).unwrap_or(false)
    }

    pub fn has_channel(&self, name: &Symbol) -> bool {
        self.channels.contains_key(name)
    }
}

/// Simple model checker for reachability properties
pub struct ModelChecker {
    max_depth: usize,
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub property_holds: bool,
    pub trace: Vec<String>,
    pub depth: usize,
}

impl ModelChecker {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Check if a state is reachable from an initial state given transition rules
    pub fn check_reachability<T: Clone + Eq + std::hash::Hash + std::fmt::Debug>(
        &self,
        initial: T,
        target_predicate: impl Fn(&T) -> bool,
        transitions: impl Fn(&T) -> Vec<T>,
    ) -> VerificationResult {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        let mut parent: HashMap<String, (String, usize)> = HashMap::new();

        let init_key = format!("{:?}", initial);
        queue.push_back((initial.clone(), 0));
        visited.insert(init_key.clone());
        parent.insert(init_key, ("start".to_string(), 0));

        while let Some((state, depth)) = queue.pop_front() {
            let state_key = format!("{:?}", state);

            if target_predicate(&state) {
                // Reconstruct trace
                let mut trace = Vec::new();
                let mut current = state_key;
                while let Some((parent_key, d)) = parent.get(&current).cloned() {
                    trace.push(format!("depth {}: {}", d, current));
                    if parent_key == "start" { break; }
                    current = parent_key;
                }
                trace.reverse();
                return VerificationResult { property_holds: true, trace, depth };
            }

            if depth >= self.max_depth { continue; }

            for next in transitions(&state) {
                let next_key = format!("{:?}", next);
                if !visited.contains(&next_key) {
                    visited.insert(next_key.clone());
                    parent.insert(next_key, (state_key.clone(), depth + 1));
                    queue.push_back((next, depth + 1));
                }
            }
        }

        VerificationResult { property_holds: false, trace: vec![], depth: self.max_depth }
    }
}

impl ModelChecker {
    /// §28 find-attack:在状态空间内搜索攻击(目标状态可到达)。
    /// 攻击模型由调用方定义:状态含攻击者知识,transitions 含拦截/转发;
    /// 目标 = 机密进入攻击者知识。
    pub fn find_attack<T: Clone + Eq + std::hash::Hash + std::fmt::Debug>(
        &self,
        initial: T,
        attack_target: impl Fn(&T) -> bool,
        transitions: impl Fn(&T) -> Vec<T>,
    ) -> VerificationResult {
        self.check_reachability(initial, attack_target, transitions)
    }

    /// §28 check-equivalence:比较两个状态系统的可达状态集(观察等价近似)
    pub fn check_equivalence<T: Clone + Eq + std::hash::Hash + std::fmt::Debug>(
        &self,
        init_a: T,
        transitions_a: impl Fn(&T) -> Vec<T>,
        init_b: T,
        transitions_b: impl Fn(&T) -> Vec<T>,
    ) -> bool {
        let sa = self.reachable_states(init_a, transitions_a);
        let sb = self.reachable_states(init_b, transitions_b);
        sa == sb
    }

    /// 收集可达状态集(BFS,深度受限)
    pub fn reachable_states<T: Clone + Eq + std::hash::Hash + std::fmt::Debug>(
        &self,
        initial: T,
        transitions: impl Fn(&T) -> Vec<T>,
    ) -> std::collections::HashSet<String> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((initial.clone(), 0));
        visited.insert(format!("{:?}", initial));
        while let Some((state, depth)) = queue.pop_front() {
            if depth >= self.max_depth { continue; }
            for next in transitions(&state) {
                let key = format!("{:?}", next);
                if visited.insert(key.clone()) {
                    queue.push_back((next, depth + 1));
                }
            }
        }
        visited
    }
}

/// §28 dolev-yao 攻击者:知识集 + 合成规则(窃听/拼接/重放)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DolevYaoAttacker {
    pub knowledge: Vec<String>,
}

impl DolevYaoAttacker {
    pub fn new() -> Self { Self { knowledge: Vec::new() } }

    /// 窃听:把网络传输的消息加入知识(去重有序)
    pub fn eavesdrop(&mut self, msg: &str) {
        if !self.knowledge.iter().any(|m| m == msg) {
            self.knowledge.push(msg.to_string());
        }
    }

    /// 合成:拼接任意两条已知消息(攻击者可构造新消息)
    pub fn synthesize(&mut self) {
        let known: Vec<String> = self.knowledge.clone();
        for a in &known {
            for b in &known {
                let m = format!("{}_{}", a, b);
                if !self.knowledge.contains(&m) { self.knowledge.push(m); }
            }
        }
    }

    /// 重放:已窃听的消息可重放
    pub fn replay(&self, msg: &str) -> bool {
        self.knowledge.iter().any(|m| m == msg)
    }

    pub fn knows(&self, msg: &str) -> bool {
        self.knowledge.iter().any(|m| m == msg)
    }
}
