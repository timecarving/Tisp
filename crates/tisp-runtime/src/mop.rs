//! 元对象协议(MOP):GetKB/SetKB 效应操作 + Handler 元解释器 + 编译期元编程
//! 以及 State Effect 引用管理(`ref`/`deref`/`set!`)
use std::collections::HashMap;

use tisp_core::evolp::Program;

/// 知识库操作:GetKB(读) / SetKB(写)——建模为效应操作
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KbOp {
    GetKb,
    SetKb(Program),
}

/// KB 操作的执行结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KbResult {
    Program(Program),
    Unit,
}

/// Handler 元解释器:捕获 GetKB/SetKB 并解释其语义(充当元解释器)
#[derive(Debug, Clone)]
pub struct MetaInterpreter {
    pub kb: Program,
}

impl MetaInterpreter {
    pub fn new(kb: Program) -> Self {
        MetaInterpreter { kb }
    }

    /// 解释一个 KB 操作:GetKB 返回当前知识库,SetKB 写入知识库
    pub fn interpret(&mut self, op: &KbOp) -> KbResult {
        match op {
            KbOp::GetKb => KbResult::Program(self.kb.clone()),
            KbOp::SetKb(p) => {
                self.kb = p.clone();
                KbResult::Unit
            }
        }
    }

    /// 顺序解释一段元程序(操作序列)
    pub fn run(&mut self, ops: &[KbOp]) -> Vec<KbResult> {
        ops.iter().map(|op| self.interpret(op)).collect()
    }
}

/// 编译期元编程:对编译期可见的操作序列做纯函数静态折叠,无需运行时 handler
pub fn compile_time_resolve(kb: &Program, ops: &[KbOp]) -> Program {
    ops.iter().fold(kb.clone(), |acc, op| match op {
        KbOp::GetKb => acc,
        KbOp::SetKb(p) => p.clone(),
    })
}

// ── State Effect 引用管理 ──

/// 类型化引用句柄:由 `ref` 分配,`deref` 读取,`set!` 消费(线性)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ref<T> {
    pub(crate) id: u64,
    pub(crate) _marker: std::marker::PhantomData<T>,
}

/// State 效应运行时:可变引用存储(get/put + ref/deref/set!)
#[derive(Debug, Clone)]
pub struct StateRuntime<T: Clone> {
    store: HashMap<u64, T>,
    next_id: u64,
}

impl<T: Clone> StateRuntime<T> {
    pub fn new() -> Self {
        StateRuntime { store: HashMap::new(), next_id: 0 }
    }

    /// 创建引用(返回 1 级线性能力句柄)
    pub fn ref_(&mut self, val: T) -> Ref<T> {
        let id = self.next_id;
        self.next_id += 1;
        self.store.insert(id, val);
        Ref { id, _marker: std::marker::PhantomData }
    }

    /// 读取引用(借用读,不消费)
    pub fn deref_(&self, r: &Ref<T>) -> Option<&T> {
        self.store.get(&r.id)
    }

    /// 写入引用(消费句柄:线性——写后原句柄不可复用)
    pub fn set_(&mut self, r: Ref<T>, val: T) -> &mut Self {
        self.store.insert(r.id, val);
        self
    }

    /// 释放引用(消费句柄)
    pub fn drop_(&mut self, r: Ref<T>) {
        self.store.remove(&r.id);
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }
}

impl<T: Clone> Default for StateRuntime<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_core::evolp::{LTerm, Rule};

    #[test]
    fn test_handler_get_set_kb() {
        let kb = Program::from_rules([Rule::fact("r1", LTerm::atom("a"))]);
        let mut m = MetaInterpreter::new(kb);
        match m.interpret(&KbOp::GetKb) {
            KbResult::Program(p) => assert_eq!(p.len(), 1),
            _ => panic!("expected program"),
        }
        let kb2 = Program::from_rules([Rule::fact("r2", LTerm::atom("b"))]);
        m.interpret(&KbOp::SetKb(kb2.clone()));
        match m.interpret(&KbOp::GetKb) {
            KbResult::Program(p) => assert!(p.contains(&tisp_core::symbol::Symbol::new("r2"))),
            _ => panic!("expected program"),
        }
    }

    #[test]
    fn test_compile_time_resolve_matches_handler() {
        let kb0 = Program::new();
        let kb1 = Program::from_rules([Rule::fact("r1", LTerm::atom("a"))]);
        let kb2 = Program::from_rules([Rule::fact("r2", LTerm::atom("b"))]);
        let ops = vec![KbOp::SetKb(kb1.clone()), KbOp::SetKb(kb2.clone()), KbOp::GetKb];

        // 编译期纯函数折叠
        let ct = compile_time_resolve(&kb0, &ops);
        // 运行时 handler 元解释
        let mut m = MetaInterpreter::new(kb0);
        m.run(&ops);
        assert_eq!(ct, m.kb);
    }

    #[test]
    fn test_ref_read_write() {
        let mut s: StateRuntime<i64> = StateRuntime::new();
        let r = s.ref_(42);
        assert_eq!(*s.deref_(&r).unwrap(), 42);
        // 线性写:消费 r
        s.set_(r, 100);
        assert_eq!(s.len(), 1);
        assert_eq!(*s.deref_(&Ref { id: 0, _marker: std::marker::PhantomData }).unwrap(), 100);
    }
}
