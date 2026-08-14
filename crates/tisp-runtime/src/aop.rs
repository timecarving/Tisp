//! AOP(面向切面编程):基于编译器纯声明式 MOP,辅助 OOP
//!
//! 切面/切入点/建议在编译期经 MOP 反射解析并编织为 OOP 方法组合(§22.3),
//! 编织是纯函数变换,运行时无动态反射。
use std::sync::Arc;

/// 连接点:方法名 + 实参
#[derive(Debug, Clone)]
pub struct JoinPoint {
    pub method: String,
    pub args: Vec<i64>,
}

/// 切入点:按方法名子串(或注解)匹配
#[derive(Debug, Clone)]
pub struct Pointcut {
    pub method_pattern: String,
}

impl Pointcut {
    pub fn by_name(pattern: &str) -> Self {
        Pointcut { method_pattern: pattern.to_string() }
    }
    pub fn matches(&self, name: &str) -> bool {
        name.contains(&self.method_pattern)
    }
}

/// 建议:before / after / around
#[derive(Clone)]
pub enum Advice {
    Before(Arc<dyn Fn(&JoinPoint) + Send + Sync>),
    After(Arc<dyn Fn(&JoinPoint, i64) + Send + Sync>),
    Around(Arc<dyn Fn(&JoinPoint, &dyn Fn(&[i64]) -> i64) -> i64 + Send + Sync>),
}

/// 切面 = 切入点 + 建议
#[derive(Clone)]
pub struct Aspect {
    pub pointcut: Pointcut,
    pub advice: Advice,
}

/// AOP 编织器:把匹配建议编织到方法调用
#[derive(Clone, Default)]
pub struct AopWeaver {
    aspects: Vec<Aspect>,
}

impl AopWeaver {
    pub fn new() -> Self {
        AopWeaver { aspects: Vec::new() }
    }

    pub fn add(&mut self, aspect: Aspect) {
        self.aspects.push(aspect);
    }

    /// 编织:before → proceed → after,around 包裹整体
    pub fn weave(&self, method: &str, args: &[i64], proceed: &dyn Fn(&[i64]) -> i64) -> i64 {
        let jp = JoinPoint { method: method.to_string(), args: args.to_vec() };
        let mut around: Option<&Advice> = None;
        let mut before: Vec<&Advice> = Vec::new();
        let mut after: Vec<&Advice> = Vec::new();
        for a in &self.aspects {
            if a.pointcut.matches(method) {
                match &a.advice {
                    Advice::Around(_) => around = Some(&a.advice),
                    Advice::Before(_) => before.push(&a.advice),
                    Advice::After(_) => after.push(&a.advice),
                }
            }
        }

        let core = |args: &[i64]| -> i64 {
            for b in &before {
                if let Advice::Before(f) = b {
                    f(&jp);
                }
            }
            let r = proceed(args);
            for a in &after {
                if let Advice::After(f) = a {
                    f(&jp, r);
                }
            }
            r
        };

        match around {
            Some(Advice::Around(f)) => f(&jp, &core),
            _ => core(args),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_aop_before_after_order() {
        let mut w = AopWeaver::new();
        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let lb = log.clone();
        let la = log.clone();
        w.add(Aspect {
            pointcut: Pointcut::by_name("calc"),
            advice: Advice::Before(Arc::new(move |_| lb.lock().unwrap().push("before".into()))),
        });
        w.add(Aspect {
            pointcut: Pointcut::by_name("calc"),
            advice: Advice::After(Arc::new(move |_, _| la.lock().unwrap().push("after".into()))),
        });
        let r = w.weave("calc", &[1, 2], &|a| a[0] + a[1]);
        assert_eq!(r, 3);
        assert_eq!(*log.lock().unwrap(), vec!["before".to_string(), "after".to_string()]);
    }

    #[test]
    fn test_aop_around_wrapping() {
        let mut w = AopWeaver::new();
        w.add(Aspect {
            pointcut: Pointcut::by_name("calc"),
            advice: Advice::Around(Arc::new(|_jp, proceed| proceed(&[10, 20]) + 100)),
        });
        // around 改变实参并包裹结果
        let r = w.weave("calc", &[1, 2], &|a| a[0] + a[1]);
        assert_eq!(r, 130); // (10+20)+100
    }

    #[test]
    fn test_pointcut_no_match() {
        let w = AopWeaver::new();
        // 无切面:直接 proceed
        let r = w.weave("other", &[5, 7], &|a| a[0] * a[1]);
        assert_eq!(r, 35);
    }
}
