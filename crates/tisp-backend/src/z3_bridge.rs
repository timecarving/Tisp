/// Z3 Solver Bridge — communicates with z3 binary via SMT-LIB2 over stdin/stdout
use std::io::{Write, BufRead, BufReader, BufWriter};
use std::process::{Command, Stdio, Child, ChildStdin};

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

    /// Get model values for a list of variables
    pub fn get_model(&mut self, _vars: &[&str]) -> Result<std::collections::HashMap<String, i64>, String> {
        writeln!(self.stdin, "(get-model)").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;

        let mut model = std::collections::HashMap::new();
        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line).map_err(|e| e.to_string())?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == ")" { break; }
            // Parse: (define-fun x () Int 42)
            if trimmed.starts_with("(define-fun ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 5 && parts[2] == "()" {
                    let name = parts[1].to_string();
                    if let Ok(val) = parts[4].trim_end_matches(')').parse::<i64>() {
                        model.insert(name, val);
                    }
                }
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
        self.assert(&format!("(not ({}))", predicate))?;
        let result = self.check_sat()?;
        self.pop()?;
        Ok(result == "unsat") // unsat means predicate always holds
    }
}

impl Drop for Z3Bridge {
    fn drop(&mut self) {
        // Z3 will exit when stdin is closed (which happens when BufWriter is dropped)
        let _ = self.process.kill();
    }
}
