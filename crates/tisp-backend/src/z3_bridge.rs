/// Z3 Solver Bridge — communicates with z3 binary via SMT-LIB2 over stdin/stdout
use std::io::{Write, BufRead, BufReader, BufWriter};
use std::process::{Command, Stdio, Child, ChildStdin};

/// 蕴含验证结果:Sat 携带反例模型(违反证据),Unsat 表示恒真,Unknown 无法判定
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyOutcome {
    Sat(std::collections::HashMap<String, i64>),
    Unsat,
    Unknown,
}

/// SMT 关键字与函数名(标识符收集时跳过,避免误声明)
const SMT_KEYWORDS: &[&str] = &[
    "and", "or", "not", "ite", "true", "false", "abs", "div", "mod", "distinct",
    "forall", "exists", "Int", "Bool",
];

/// 从 SMT 表达式中收集标识符(字母数字下划线序列),去重、排除数字字面量与关键字
fn collect_identifiers(exprs: &[String]) -> Vec<String> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = Vec::new();
    for s in exprs {
        let mut cur = String::new();
        for c in s.chars() {
            if c.is_ascii_alphanumeric() || c == '_' {
                cur.push(c);
            } else if !cur.is_empty() {
                if !cur.chars().all(|c| c.is_ascii_digit())
                    && !SMT_KEYWORDS.contains(&cur.as_str())
                    && seen.insert(cur.clone()) {
                    out.push(cur.clone());
                }
                cur.clear();
            }
        }
        if !cur.is_empty()
            && !cur.chars().all(|c| c.is_ascii_digit())
            && !SMT_KEYWORDS.contains(&cur.as_str())
            && seen.insert(cur.clone()) {
            out.push(cur);
        }
    }
    out
}

/// 解析模型值行:支持 "3"、"3)"、"(- 1)"、"(- 1))" 等 z3 输出形态;
/// 解析失败返回 None(调用方应放弃该变量并继续,避免阻塞)
fn parse_model_value(line: &str) -> Option<i64> {
    let t = line.trim();
    let digits_start = t.find(|c: char| c.is_ascii_digit())?;
    let prefix = &t[..digits_start];
    let neg = prefix.contains('-');
    let mut end = digits_start;
    for c in t[digits_start..].chars() {
        if c.is_ascii_digit() { end += c.len_utf8(); } else { break; }
    }
    let n: i64 = t[digits_start..end].parse().ok()?;
    Some(if neg { -n } else { n })
}

/// 反例格式化:`x = -1, d = 0`
pub fn format_counterexample(model: &std::collections::HashMap<String, i64>) -> String {
    let mut parts: Vec<String> = model.iter()
        .map(|(k, v)| format!("{} = {}", k, v))
        .collect();
    parts.sort();
    parts.join(", ")
}

pub struct Z3Bridge {
    process: Child,
    stdin: BufWriter<ChildStdin>,
    reader: BufReader<std::process::ChildStdout>,
    declarations: Vec<String>,
}

impl Z3Bridge {
    pub fn new() -> Result<Self, String> {
        let mut process = Command::new("z3")
            .arg("-in")
            .arg("-smt2")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start z3: {}. Install with: apt install z3", e))?;

        let stdin = process.stdin.take()
            .ok_or_else(|| "Failed to open z3 stdin".to_string())?;
        let stdout = process.stdout.take()
            .ok_or_else(|| "Failed to open z3 stdout".to_string())?;

        Ok(Z3Bridge {
            process,
            stdin: BufWriter::new(stdin),
            reader: BufReader::new(stdout),
            declarations: Vec::new(),
        })
    }

    /// Declare an integer variable
    pub fn declare_int(&mut self, name: &str) -> Result<(), String> {
        let cmd = format!("(declare-const {} Int)", name);
        self.declarations.push(cmd.clone());
        writeln!(self.stdin, "{}", cmd).map_err(|e| e.to_string())
    }

    /// Declare a boolean variable
    pub fn declare_bool(&mut self, name: &str) -> Result<(), String> {
        let cmd = format!("(declare-const {} Bool)", name);
        self.declarations.push(cmd.clone());
        writeln!(self.stdin, "{}", cmd).map_err(|e| e.to_string())
    }

    /// Assert a constraint
    pub fn assert(&mut self, constraint: &str) -> Result<(), String> {
        let cmd = format!("(assert {})", constraint);
        writeln!(self.stdin, "{}", cmd).map_err(|e| e.to_string())
    }

    /// Assert x >= n
    pub fn assert_ge(&mut self, x: &str, n: i64) -> Result<(), String> {
        self.assert(&format!("(>= {} {})", x, n))
    }

    /// Assert x > n
    pub fn assert_gt(&mut self, x: &str, n: i64) -> Result<(), String> {
        self.assert(&format!("(> {} {})", x, n))
    }

    /// Assert x < n
    pub fn assert_lt(&mut self, x: &str, n: i64) -> Result<(), String> {
        self.assert(&format!("(< {} {})", x, n))
    }

    /// Assert x = n
    pub fn assert_eq(&mut self, x: &str, n: i64) -> Result<(), String> {
        self.assert(&format!("(= {} {})", x, n))
    }

    /// Assert x != n
    pub fn assert_neq(&mut self, x: &str, n: i64) -> Result<(), String> {
        self.assert(&format!("(not (= {} {}))", x, n))
    }

    /// Push a solver context (for backtracking)
    pub fn push(&mut self) -> Result<(), String> {
        writeln!(self.stdin, "(push 1)").map_err(|e| e.to_string())
    }

    /// Pop a solver context
    pub fn pop(&mut self) -> Result<(), String> {
        writeln!(self.stdin, "(pop 1)").map_err(|e| e.to_string())
    }

    /// Check satisfiability — returns "sat", "unsat", or "unknown"
    pub fn check_sat(&mut self) -> Result<String, String> {
        writeln!(self.stdin, "(check-sat)").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        self.reader.read_line(&mut line).map_err(|e| e.to_string())?;
        Ok(line.trim().to_string())
    }

    /// Get model values for a list of variables.
    /// z3 输出为多行格式:`(define-fun x () Int` 换行后跟 `    4)`,逐行解析。
    pub fn get_model(&mut self, _vars: &[&str]) -> Result<std::collections::HashMap<String, i64>, String> {
        writeln!(self.stdin, "(get-model)").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;

        let mut model = std::collections::HashMap::new();
        let mut current: Option<String> = None;
        loop {
            let mut line = String::new();
            if self.reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                break; // EOF
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed == "(" {
                continue; // 模型列表开头
            }
            // 顶层模型结束:单独一个 ")" 且没有正在解析的变量
            if trimmed == ")" {
                if current.is_none() {
                    break;
                }
                current = None;
                continue;
            }
            // 解析 (define-fun name () Int
            if trimmed.starts_with("(define-fun ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    current = Some(parts[1].to_string());
                }
                continue;
            }
            // 值行:整数(可能带尾括号)或 z3 负数形态 "(- 1))";解析失败则放弃该变量继续
            if let Some(name) = &current {
                if let Some(val) = parse_model_value(trimmed) {
                    model.insert(name.clone(), val);
                }
                current = None; // 无论成败都消费,避免阻塞等待下一行
                continue;
            }
        }
        Ok(model)
    }

    /// Verify a refinement predicate with given variable bindings
    pub fn verify_refinement(&mut self, var_bindings: &[(&str, i64)], predicate: &str) -> Result<bool, String> {
        self.push()?;
        for (var, val) in var_bindings {
            self.declare_int(var)?;
            self.assert_eq(var, *val)?;
        }
        // predicate 形如 "(>= x 0)",直接套 not
        self.assert(&format!("(not {})", predicate))?;
        let result = self.check_sat()?;
        self.pop()?;
        Ok(result == "unsat") // unsat means predicate always holds
    }

    /// 验证蕴含:前提 premises(合取)⇒ conclusion。
    /// 自由变量(在表达式中出现的标识符)声明为 Int。
    /// - unsat:不存在反例,蕴含恒真 → Unsat
    /// - sat:存在反例 → Sat(模型即反例)
    /// - unknown:无法判定 → Unknown
    pub fn verify_implication(&mut self, premises: &[String], conclusion: &str) -> Result<VerifyOutcome, String> {
        self.push()?;
        // 闭包保证任何错误路径都先 pop 恢复求解上下文
        let outcome = (|| -> Result<VerifyOutcome, String> {
            let mut exprs: Vec<String> = premises.to_vec();
            exprs.push(conclusion.to_string());
            for var in collect_identifiers(&exprs) {
                self.declare_int(&var)?;
            }
            for p in premises {
                self.assert(p)?;
            }
            self.assert(&format!("(not {})", conclusion))?;
            match self.check_sat()?.as_str() {
                "unsat" => Ok(VerifyOutcome::Unsat),
                "sat" => Ok(VerifyOutcome::Sat(self.get_model(&[])?)),
                _ => Ok(VerifyOutcome::Unknown),
            }
        })();
        let _ = self.pop();
        outcome
    }
}

impl Drop for Z3Bridge {
    fn drop(&mut self) {
        // Z3 will exit when stdin is closed (which happens when BufWriter is dropped)
        let _ = self.process.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 求解器往返:声明/断言/check-sat/push/pop 基础行为
    #[test]
    fn test_sat_unsat_roundtrip() {
        let mut z3 = Z3Bridge::new().expect("z3 应可用(apt install z3)");
        z3.declare_int("x").unwrap();
        z3.assert_ge("x", 3).unwrap();
        assert_eq!(z3.check_sat().unwrap(), "sat");

        // 追加冲突约束后应 unsat
        z3.assert_lt("x", 3).unwrap();
        assert_eq!(z3.check_sat().unwrap(), "unsat");
    }

    #[test]
    fn test_push_pop_backtrack() {
        let mut z3 = Z3Bridge::new().unwrap();
        z3.declare_int("x").unwrap();
        z3.assert_ge("x", 10).unwrap();
        assert_eq!(z3.check_sat().unwrap(), "sat");

        // 在 push 层内加冲突,pop 后恢复
        z3.push().unwrap();
        z3.assert_lt("x", 5).unwrap();
        assert_eq!(z3.check_sat().unwrap(), "unsat");
        z3.pop().unwrap();
        assert_eq!(z3.check_sat().unwrap(), "sat");
    }

    #[test]
    fn test_verify_refinement() {
        let mut z3 = Z3Bridge::new().unwrap();
        // (>= x 0) 在 x = 3 下恒真
        assert!(z3.verify_refinement(&[("x", 3)], "(>= x 0)").unwrap());
        // (>= x 0) 在 x = -1 下可违反
        assert!(!z3.verify_refinement(&[("x", -1)], "(>= x 0)").unwrap());
    }

    #[test]
    fn test_get_model_values() {
        let mut z3 = Z3Bridge::new().unwrap();
        z3.declare_int("x").unwrap();
        z3.declare_int("y").unwrap();
        z3.assert("(= (+ x y) 7)").unwrap();
        z3.assert("(> x 3)").unwrap();
        assert_eq!(z3.check_sat().unwrap(), "sat");
        let model = z3.get_model(&["x", "y"]).unwrap();
        assert_eq!(model.get("x").unwrap() + model.get("y").unwrap(), 7);
        assert!(model["x"] > 3);
    }

    #[test]
    fn test_get_model_negative_values() {
        // z3 用 (- n) 表示负数,须正确解析
        let mut z3 = Z3Bridge::new().unwrap();
        z3.declare_int("x").unwrap();
        z3.assert("(= x (- 1))").unwrap();
        assert_eq!(z3.check_sat().unwrap(), "sat");
        let model = z3.get_model(&["x"]).unwrap();
        assert_eq!(model.get("x").copied(), Some(-1));
    }

    #[test]
    fn test_verify_implication_holds() {
        // (>= x 1) ⇒ (> x 0):恒真 → Unsat(无反例)
        let mut z3 = Z3Bridge::new().unwrap();
        let out = z3.verify_implication(&["(>= x 1)".to_string()], "(> x 0)").unwrap();
        assert_eq!(out, VerifyOutcome::Unsat);
    }

    #[test]
    fn test_verify_implication_counterexample() {
        // (>= x 0) ⇒ (> x 0):x = 0 为反例 → Sat 且模型含 x = 0
        let mut z3 = Z3Bridge::new().unwrap();
        let out = z3.verify_implication(&["(>= x 0)".to_string()], "(> x 0)").unwrap();
        match out {
            VerifyOutcome::Sat(model) => {
                assert_eq!(model.get("x").copied(), Some(0));
                let s = format_counterexample(&model);
                assert!(s.contains("x = 0"), "反例格式应为 'x = 0',实际 '{}'", s);
            }
            other => panic!("应返回 Sat 反例,实际 {:?}", other),
        }
    }

    #[test]
    fn test_verify_implication_no_premises() {
        // 无前提:验证结论恒真(重言式)→ Unsat(无反例)
        let mut z3 = Z3Bridge::new().unwrap();
        let out = z3.verify_implication(&[], "(or (> x 0) (<= x 0))").unwrap();
        assert_eq!(out, VerifyOutcome::Unsat);
        // 非恒真结论 → Sat 反例
        let out2 = z3.verify_implication(&[], "(and (> x 0) (< x 0))").unwrap();
        assert!(matches!(out2, VerifyOutcome::Sat(_)), "矛盾式非恒真,应给反例,实际 {:?}", out2);
    }

    #[test]
    fn test_collect_identifiers_filters_keywords() {
        let exprs = vec![
            "(and (>= x 0) (<= x 10))".to_string(),
            "(forall ((n Int)) (> n 0))".to_string(),
        ];
        let ids = collect_identifiers(&exprs);
        assert!(ids.contains(&"x".to_string()));
        assert!(ids.contains(&"n".to_string()));
        assert!(!ids.contains(&"and".to_string()));
        assert!(!ids.contains(&"Int".to_string()));
        assert!(!ids.contains(&"forall".to_string()));
    }
}

