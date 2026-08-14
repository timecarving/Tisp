//! EVOLP / DLP 求解器:稳定模型、演化、动态稳定模型(纯声明式)
//!
//! 全部操作是纯函数:`Program -> Program` 的演化、稳定模型不动点、DLP 状态序列。
//! 基于 tisp-core 的一等 `Rule`/`Program`/`EvolInstr` ADT,不引入命令式逃逸。
use std::collections::HashMap;

use tisp_core::evolp::{EvolInstr, Literal, LTerm, Program, Rule};
use tisp_core::symbol::Symbol;

/// 解释 = 地面原子集合
pub type Interpretation = im::HashSet<LTerm>;

// ── 地面化(grounding) ──

/// 收集程序中所有常量项(Int/Str),作为变量替换的 Herbrand 全域
fn collect_constants(p: &Program) -> Vec<LTerm> {
    let mut set = im::HashSet::new();
    for r in p.iter() {
        collect_term_constants(&r.head, &mut set);
        for l in &r.body {
            match l {
                Literal::Pos(t) | Literal::Neg(t) => collect_term_constants(t, &mut set),
            }
        }
        for instr in &r.evol {
            if let EvolInstr::Assert(rule) = instr {
                collect_term_constants(&rule.head, &mut set);
            }
        }
    }
    set.into_iter().collect()
}

fn collect_term_constants(t: &LTerm, set: &mut im::HashSet<LTerm>) {
    match t {
        LTerm::Int(_) | LTerm::Str(_) => {
            set.insert(t.clone());
        }
        LTerm::Fun(_, args) => {
            for a in args {
                collect_term_constants(a, set);
            }
        }
        LTerm::Var(_) => {}
    }
}

/// 收集规则中的(去重后)逻辑变量
fn collect_vars(r: &Rule) -> Vec<Symbol> {
    let mut seen = im::HashSet::new();
    let mut out = Vec::new();
    collect_term_vars(&r.head, &mut seen, &mut out);
    for l in &r.body {
        match l {
            Literal::Pos(t) | Literal::Neg(t) => collect_term_vars(t, &mut seen, &mut out),
        }
    }
    out
}

fn collect_term_vars(t: &LTerm, seen: &mut im::HashSet<Symbol>, out: &mut Vec<Symbol>) {
    match t {
        LTerm::Var(v) => {
            if seen.insert(v.clone()).is_none() {
                out.push(v.clone());
            }
        }
        LTerm::Fun(_, args) => {
            for a in args {
                collect_term_vars(a, seen, out);
            }
        }
        _ => {}
    }
}

/// 变量替换的笛卡尔积
fn substitutions(vars: &[Symbol], universe: &[LTerm]) -> Vec<HashMap<Symbol, LTerm>> {
    if vars.is_empty() || universe.is_empty() {
        return if vars.is_empty() { vec![HashMap::new()] } else { Vec::new() };
    }
    let mut results = vec![HashMap::new()];
    for v in vars {
        let mut next = Vec::new();
        for sigma in &results {
            for c in universe {
                let mut s = sigma.clone();
                s.insert(v.clone(), c.clone());
                next.push(s);
            }
        }
        results = next;
    }
    results
}

fn subst_term(t: &LTerm, sigma: &HashMap<Symbol, LTerm>) -> LTerm {
    match t {
        LTerm::Var(v) => sigma.get(v).cloned().unwrap_or_else(|| t.clone()),
        LTerm::Fun(name, args) => {
            LTerm::Fun(name.clone(), args.iter().map(|a| subst_term(a, sigma)).collect())
        }
        other => other.clone(),
    }
}

fn subst_rule(r: &Rule, sigma: &HashMap<Symbol, LTerm>) -> Rule {
    Rule {
        id: r.id.clone(),
        head: subst_term(&r.head, sigma),
        body: r
            .body
            .iter()
            .map(|l| match l {
                Literal::Pos(t) => Literal::Pos(subst_term(t, sigma)),
                Literal::Neg(t) => Literal::Neg(subst_term(t, sigma)),
            })
            .collect(),
        evol: r
            .evol
            .iter()
            .map(|e| match e {
                EvolInstr::Assert(rule) => EvolInstr::Assert(subst_rule(rule, sigma)),
                EvolInstr::Retract(id) => EvolInstr::Retract(id.clone()),
            })
            .collect(),
    }
}

/// 地面化:把含变量的规则实例化为地面规则
pub fn ground_rules(p: &Program) -> Vec<Rule> {
    let universe = collect_constants(p);
    let mut out = Vec::new();
    for r in p.iter() {
        let vars = collect_vars(r);
        if vars.is_empty() {
            out.push(r.clone());
        } else {
            for sigma in substitutions(&vars, &universe) {
                out.push(subst_rule(r, &sigma));
            }
        }
    }
    out
}

// ── 稳定模型语义 ──

/// 解释是否满足规则体(NAF:Neg(a) 真 iff a ∉ I)
fn body_satisfied(r: &Rule, interp: &Interpretation) -> bool {
    r.body.iter().all(|l| match l {
        Literal::Pos(a) => interp.contains(a),
        Literal::Neg(a) => !interp.contains(a),
    })
}

/// Gelfond-Lifschitz 约化:删除体含「被 I 满足的 Neg 原子」的规则,再去掉所有 Neg 字面量
fn reduct(rules: &[Rule], interp: &Interpretation) -> Vec<Rule> {
    rules
        .iter()
        .filter(|r| r.body.iter().all(|l| !matches!(l, Literal::Neg(a) if interp.contains(a))))
        .map(|r| Rule {
            id: r.id.clone(),
            head: r.head.clone(),
            body: r
                .body
                .iter()
                .filter(|l| matches!(l, Literal::Pos(_)))
                .cloned()
                .collect(),
            evol: r.evol.clone(),
        })
        .collect()
}

/// 正程序的最小模型(T_P 不动点)
fn minimal_model(rules: &[Rule]) -> Interpretation {
    let mut interp: Interpretation = im::HashSet::new();
    loop {
        let mut changed = false;
        for r in rules {
            if body_satisfied(r, &interp) && !interp.contains(&r.head) {
                interp.insert(r.head.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    interp
}

/// 稳定模型判定:reduct(P, I) 的最小模型 == I
fn is_stable(rules: &[Rule], interp: &Interpretation) -> bool {
    minimal_model(&reduct(rules, interp)) == *interp
}

/// 地面程序的 Herbrand 基(所有出现在头/体的原子)
fn herbrand_base(rules: &[Rule]) -> Vec<LTerm> {
    let mut set = im::HashSet::new();
    for r in rules {
        set.insert(r.head.clone());
        for l in &r.body {
            match l {
                Literal::Pos(t) | Literal::Neg(t) => {
                    set.insert(t.clone());
                }
            }
        }
    }
    set.into_iter().collect()
}

/// 枚举所有稳定模型(小规模:幂集穷举)
pub fn stable_models(p: &Program) -> Vec<Interpretation> {
    let rules = ground_rules(p);
    let base = herbrand_base(&rules);
    let mut results = Vec::new();
    for subset in powerset(&base) {
        let interp: Interpretation = subset.into_iter().collect();
        if is_stable(&rules, &interp) {
            results.push(interp);
        }
    }
    results
}

fn powerset<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    let mut results = vec![Vec::new()];
    for item in items {
        let mut next = results.clone();
        for r in &mut next {
            r.push(item.clone());
        }
        results.extend(next);
    }
    results
}

// ── 演化 ──

/// 应用一条演化指令(纯函数)
pub fn evolve(program: &Program, instr: &EvolInstr) -> Program {
    let mut p = program.clone();
    match instr {
        EvolInstr::Assert(rule) => p.add(rule.clone()),
        EvolInstr::Retract(id) => {
            p.remove(id);
        }
    }
    p
}

/// foldl 折叠演化:按序应用指令序列
pub fn evolve_all(program: &Program, instrs: &[EvolInstr]) -> Program {
    instrs.iter().fold(program.clone(), |acc, i| evolve(&acc, i))
}

/// EVOLP 不动点:迭代计算稳定模型并触发演化指令,直到程序不再变化
pub fn evolve_fixpoint(program: &Program) -> Program {
    let mut current = program.clone();
    loop {
        let models = stable_models(&current);
        // 收集所有稳定模型中成立规则的演化指令(并集,确定性)
        let mut instrs = Vec::new();
        for r in current.iter() {
            let head_true = models.iter().any(|m| m.contains(&r.head));
            if head_true {
                instrs.extend(r.evol.iter().cloned());
            }
        }
        let next = evolve_all(&current, &instrs);
        if next == current {
            return current;
        }
        current = next;
    }
}

// ── DLP 动态稳定模型 ──

/// 每个状态中被缺省否定的原子集合
fn negated_per_state(states: &[Program]) -> Vec<im::HashSet<LTerm>> {
    states
        .iter()
        .map(|p| {
            let mut s = im::HashSet::new();
            for r in p.iter() {
                for l in &r.body {
                    if let Literal::Neg(a) = l {
                        s.insert(a.clone());
                    }
                }
            }
            s
        })
        .collect()
}

/// DLP 动态稳定模型:对每个状态拒绝被后续状态否定的规则,对剩余规则做约化求最小模型
pub fn dynamic_stable_models(states: &[Program]) -> Vec<Interpretation> {
    if states.is_empty() {
        return Vec::new();
    }
    let negated = negated_per_state(states);
    let mut surviving = Vec::new();
    for (i, state) in states.iter().enumerate() {
        for r in state.iter() {
            let rejected = (i + 1..states.len()).any(|j| negated[j].contains(&r.head));
            if !rejected {
                surviving.push(r.clone());
            }
        }
    }
    let mut p = Program::new();
    for r in surviving {
        p.add(r);
    }
    stable_models(&p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ground_rules_instantiates_vars() {
        let mut p = Program::new();
        p.add(Rule::rule("r1", LTerm::atom1("p", LTerm::Var(Symbol::new("X"))), vec![Literal::Pos(LTerm::atom1("q", LTerm::Var(Symbol::new("X"))))]));
        p.add(Rule::fact("c1", LTerm::atom1("q", LTerm::int(1))));
        p.add(Rule::fact("c2", LTerm::atom1("q", LTerm::int(2))));
        let ground = ground_rules(&p);
        // 变量规则实例化为 2 条地面规则
        let var_rules: Vec<_> = ground.iter().filter(|r| r.id == Symbol::new("r1")).collect();
        assert_eq!(var_rules.len(), 2);
    }

    #[test]
    fn test_minimal_model_positive() {
        let p = Program::from_rules([
            Rule::fact("f1", LTerm::atom("a")),
            Rule::rule("f2", LTerm::atom("b"), vec![Literal::Pos(LTerm::atom("a"))]),
        ]);
        let m = minimal_model(&ground_rules(&p));
        assert!(m.contains(&LTerm::atom("a")));
        assert!(m.contains(&LTerm::atom("b")));
    }

    #[test]
    fn test_stable_model_negation() {
        // p :- not q.  → {p}
        let p = Program::from_rules([
            Rule::rule("r1", LTerm::atom("p"), vec![Literal::Neg(LTerm::atom("q"))]),
        ]);
        let models = stable_models(&p);
        assert_eq!(models.len(), 1);
        assert!(models[0].contains(&LTerm::atom("p")));
        assert!(!models[0].contains(&LTerm::atom("q")));
    }

    #[test]
    fn test_evolve_assert_retract() {
        let mut p = Program::new();
        p.add(Rule::fact("r1", LTerm::atom("a")));
        let p2 = evolve(&p, &EvolInstr::Assert(Rule::fact("r2", LTerm::atom("b"))));
        assert!(p2.contains(&Symbol::new("r2")));
        assert!(!p.contains(&Symbol::new("r2"))); // 原程序不可变

        let p3 = evolve(&p2, &EvolInstr::Retract(Symbol::new("r1")));
        assert!(!p3.contains(&Symbol::new("r1")));
    }

    #[test]
    fn test_evolve_foldl() {
        let p = Program::new();
        let instrs = vec![
            EvolInstr::Assert(Rule::fact("r1", LTerm::atom("a"))),
            EvolInstr::Assert(Rule::fact("r2", LTerm::atom("b"))),
            EvolInstr::Retract(Symbol::new("r1")),
        ];
        let result = evolve_all(&p, &instrs);
        assert!(!result.contains(&Symbol::new("r1")));
        assert!(result.contains(&Symbol::new("r2")));
    }

    #[test]
    fn test_evolve_fixpoint() {
        // 规则 a 携带 assert(b);b 携带 retract(a) → 演化到 a 消失、b 存在
        let p = Program::from_rules([
            Rule::evolving("ra", LTerm::atom("a"), vec![], vec![EvolInstr::Assert(Rule::fact("rb", LTerm::atom("b")))]),
            Rule::evolving("rb", LTerm::atom("b"), vec![], vec![EvolInstr::Retract(Symbol::new("ra"))]),
        ]);
        let fp = evolve_fixpoint(&p);
        assert!(!fp.contains(&Symbol::new("ra")));
        assert!(fp.contains(&Symbol::new("rb")));
    }

    #[test]
    fn test_dlp_dynamic_stable_model() {
        // P1: p.  P2: not p.  → p 被后续状态否定,拒绝后 p 不成立
        let p1 = Program::from_rules([Rule::fact("r1", LTerm::atom("p"))]);
        let p2 = Program::from_rules([Rule::rule("r2", LTerm::atom("q"), vec![Literal::Neg(LTerm::atom("p"))])]);
        let models = dynamic_stable_models(&[p1, p2]);
        // p 在 P2 中被否定 → P1 的 p 被拒绝;q 由 P2 推出
        assert!(!models.is_empty());
        assert!(!models[0].contains(&LTerm::atom("p")));
        assert!(models[0].contains(&LTerm::atom("q")));
    }
}
