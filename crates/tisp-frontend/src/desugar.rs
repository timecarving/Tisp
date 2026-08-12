use tisp_core::ast::{Expr, SExpr};
use tisp_core::core_ast::*;
use tisp_core::span::{Span, Spanned};
use tisp_core::symbol::Symbol;
use tisp_core::types::{Grade, Mode, EffectRow, EffectLabel, Determinism, Predicate, CmpOp, BinOp, Lit, Type};
use tisp_core::data::{DataDecl, Constructor, Field};

#[derive(Debug, Clone)]
pub struct DesugarError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for DesugarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.message, self.span)
    }
}

impl std::error::Error for DesugarError {}

#[allow(dead_code)]
enum TopLevel {
    DataDecl(DataDecl),
    Def(CoreDef),
    TypeFamily(tisp_core::types::TypeFamilyInstance),
    ResourceAlgebra(tisp_core::types::ResourceAlgebra),
    EffectDecl(tisp_core::effects::EffectDecl),
    Namespace(Symbol, Vec<(Symbol, Symbol)>, Vec<Symbol>),
    FFIDecl(Symbol, String, Vec<tisp_core::types::Type>, Option<tisp_core::types::Type>, Vec<tisp_core::types::EffectLabel>),
    /// 声明类形式(defmacro 等):已处理,不产生 def 也不作为顶层表达式
    Ignored,
}

pub struct Desugarer {
    /// §24.1 宏表:宏名 → (参数列表, 模板表达式)
    macros: std::cell::RefCell<std::collections::HashMap<Symbol, (Vec<Symbol>, Vec<SExpr>)>>,
    /// §25 已加载的模块文件(防循环加载)
    loaded_files: std::cell::RefCell<std::collections::HashSet<String>>,
    /// §25 模块加载基准目录(require 相对路径解析)
    base_dir: std::cell::RefCell<Option<String>>,
    /// §24 宏卫生计数器(gensym 后缀)
    gensym_counter: std::cell::RefCell<usize>,
}

impl Desugarer {
    /// §25 设置模块加载基准目录(require 的 {mod}.tisp 相对该目录解析)
    pub fn set_base_dir(&self, dir: &str) {
        *self.base_dir.borrow_mut() = Some(dir.to_string());
    }
}

impl Desugarer {
    pub fn new() -> Self {
        Self {
            macros: std::cell::RefCell::new(std::collections::HashMap::new()),
            loaded_files: std::cell::RefCell::new(std::collections::HashSet::new()),
            base_dir: std::cell::RefCell::new(None),
            gensym_counter: std::cell::RefCell::new(0),
        }
    }

    pub fn desugar_program(&self, forms: Vec<SExpr>) -> Result<CoreProgram, DesugarError> {
        let mut data_decls = Vec::new();
        let mut effect_decls = Vec::new();
        let mut type_families = Vec::new();
        let mut resource_algebras = Vec::new();
        let mut defs = Vec::new();
        let mut top_exprs = Vec::new();
        for form in forms {
            match self.desugar_top_level(&form)? {
                Some(TopLevel::DataDecl(decl)) => data_decls.push(decl),
                Some(TopLevel::EffectDecl(decl)) => effect_decls.push(decl),
                Some(TopLevel::TypeFamily(inst)) => type_families.push(inst),
                Some(TopLevel::ResourceAlgebra(alg)) => resource_algebras.push(alg),
                Some(TopLevel::Def(def)) => defs.push(def),
                Some(TopLevel::Namespace(name, requires, _)) => {
                    // §25 跨文件加载:require 的模块 {mod}.tisp 合并进当前程序(防循环)
                    for (mod_name, _alias) in &requires {
                        let path = match &*self.base_dir.borrow() {
                            Some(dir) => format!("{}/{}.tisp", dir, mod_name),
                            None => format!("{}.tisp", mod_name),
                        };
                        if self.loaded_files.borrow().contains(&path) {
                            continue;
                        }
                        self.loaded_files.borrow_mut().insert(path.clone());
                        if let Ok(src) = std::fs::read_to_string(&path) {
                            if let Ok(forms) = crate::reader::read(&src) {
                                if let Ok(loaded) = self.desugar_program(forms) {
                                    data_decls.extend(loaded.data_decls);
                                    effect_decls.extend(loaded.effect_decls);
                                    defs.extend(loaded.defs);
                                }
                            }
                        }
                    }
                    defs.push(CoreDef { name: name.clone(), ty: None, effects: EffectRow::Pure, grade: Grade::Omega,
                        mode: Mode::In, determinism: Determinism::Det,
            mode_sigs: vec![],
                        body: CoreExpr::new(CoreExprNode::NSDef(name, vec![], vec![]), Span::dummy()),
                        requires: None, ensures: None, span: Span::dummy() });
                }
                Some(TopLevel::FFIDecl(name, c_name, params, ret, effects)) => {
                    defs.push(CoreDef { name: name.clone(), ty: None, effects: EffectRow::Closed(effects), grade: Grade::Omega,
                        mode: Mode::In, determinism: Determinism::Det,
            mode_sigs: vec![],
                        body: CoreExpr::new(CoreExprNode::ExternDef(name, c_name, params, ret, vec![]), Span::dummy()),
                        requires: None, ensures: None, span: Span::dummy() });
                }
                Some(TopLevel::Ignored) => {}
                None => {
                    // 顶层表达式:收集为隐式入口 __top__(§6.3 顶层求值)
                    top_exprs.push(self.desugar_expr(&form)?);
                }
            }
        }
        // 有顶层表达式时生成隐式入口 def(由解释器优先执行)
        if !top_exprs.is_empty() {
            let body = if top_exprs.len() == 1 {
                top_exprs.pop().unwrap()
            } else {
                CoreExpr::new(CoreExprNode::Do(top_exprs), Span::dummy())
            };
            defs.push(CoreDef {
                name: Symbol::new("__top__"),
                ty: None,
                effects: EffectRow::Open(vec![EffectLabel::IO, EffectLabel::Search, EffectLabel::State(Box::new(tisp_core::types::Type::unit()))], Box::new(EffectRow::Pure)),
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
            });
        }
        Ok(CoreProgram { data_decls, effect_decls, type_families, resource_algebras, defs })
    }

    fn desugar_top_level(&self, expr: &SExpr) -> Result<Option<TopLevel>, DesugarError> {
        match &expr.node {
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(name) = &items[0].node {
                    match name.as_str() {
                        "def" => return Ok(Some(TopLevel::Def(self.desugar_def_form(items, expr.span)?))),
                        "defn" => return Ok(Some(TopLevel::Def(self.desugar_defn_form(items, expr.span)?))),
                        "defn-" => return Ok(Some(TopLevel::Def(self.desugar_defn_form(items, expr.span)?))),
                        "def-" => return Ok(Some(TopLevel::Def(self.desugar_def_form(items, expr.span)?))),
                        "defdata" => return Ok(Some(TopLevel::DataDecl(self.desugar_defdata_form(items, expr.span)?))),
                        "defdata-hit" => return Ok(Some(TopLevel::DataDecl(self.desugar_defdata_hit_form(items, expr.span)?))),
                        "defeffect" => {
                            return Ok(Some(TopLevel::EffectDecl(self.desugar_defeffect_form(items, expr.span)?)));
                        }
                        "defpred" => return Ok(Some(TopLevel::Def(self.desugar_defpred_form(items, expr.span)?))),
                        "defclass" => return self.desugar_defclass_form(items, expr.span),
                        "defprop" => return self.desugar_defprop_form(items, expr.span),
                        "definstance" => return self.desugar_definstance_form(items, expr.span),
                        "defgeneric" => return self.desugar_defgeneric_form(items, expr.span),
                        "defmethod" => return self.desugar_defmethod_form(items, expr.span),
                        "defmacro" => return self.desugar_defmacro_form(items, expr.span),
                        "defextern" => return self.desugar_defextern_form(items, expr.span),
                        "defresource-algebra" => {
                            return Ok(Some(TopLevel::ResourceAlgebra(self.desugar_resource_algebra_form(items, expr.span)?)));
                        }
                        "typefamily" => return Ok(Some(TopLevel::TypeFamily(self.desugar_typefamily_form(items, expr.span)?))),
                        "defsession" => return self.desugar_defsession_form(items, expr.span),
                        "ns" => return self.desugar_ns_form(items, expr.span),
                        // §30 编译指示:接受并忽略(语法兼容;真实优化器接入见 §7)
                        "inline!" | "specialize!" | "opt-level" | "suppress-warning" | "noinline!" => {
                            return Ok(Some(TopLevel::Ignored));
                        }
                        _ => return Ok(None),
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// §11.1:解析 (defresource-algebra 名称 单位元 二元运算 [阶])
    /// 例:(defresource-algebra Cost 0 + <=)
    fn desugar_resource_algebra_form(&self, items: &[SExpr], span: Span) -> Result<tisp_core::types::ResourceAlgebra, DesugarError> {
        if items.len() < 4 {
            return Err(DesugarError { message: "defresource-algebra requires name, unit, and binary op".into(), span });
        }
        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "defresource-algebra name must be a symbol".into(), span: items[1].span }),
        };
        // 单位元:数字或符号,存显示文本
        let unit = match &items[2].node {
            Expr::Int(n) => n.to_string(),
            Expr::Sym(s) => s.as_str().to_string(),
            _ => return Err(DesugarError { message: "defresource-algebra unit must be a literal".into(), span: items[2].span }),
        };
        let op = match &items[3].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "defresource-algebra op must be a symbol".into(), span: items[3].span }),
        };
        let order = match items.get(4).map(|i| &i.node) {
            Some(Expr::Sym(s)) => Some(s.clone()),
            Some(Expr::Keyword(k)) => Some(Symbol::new(k.as_str())),
            _ => None,
        };
        Ok(tisp_core::types::ResourceAlgebra { name, unit, op, order })
    }

    /// §22.1:(defgeneric name [params] -> Ret)
    fn desugar_defgeneric_form(&self, items: &[SExpr], span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let name = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Err(DesugarError { message: "defgeneric requires a name".into(), span }),
        };
        let params = match items.get(2) {
            Some(p) => self.desugar_params(p)?,
            None => Vec::new(),
        };
        let def = CoreDef {
            name: name.clone(), ty: None, effects: EffectRow::Pure, grade: Grade::Omega,
            mode: Mode::In, determinism: Determinism::Det,
            mode_sigs: vec![],
            body: CoreExpr::new(CoreExprNode::GenericDef(name, params, None), span),
            requires: None, ensures: None, span,
        };
        Ok(Some(TopLevel::Def(def)))
    }

    /// §22.2/22.3:(defmethod generic [:around|:before|:after|:primary] [patterns] body...)
    fn desugar_defmethod_form(&self, items: &[SExpr], span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let gen = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Err(DesugarError { message: "defmethod requires a generic name".into(), span }),
        };
        // 可选方法类别修饰符(§22.3 方法组合)
        let mut category = MethodCategory::Primary;
        let pat_idx = match items.get(2).map(|i| &i.node) {
            Some(Expr::Keyword(k)) => {
                category = match k.as_str() {
                    "around" => MethodCategory::Around,
                    "before" => MethodCategory::Before,
                    "after" => MethodCategory::After,
                    "primary" => MethodCategory::Primary,
                    _ => return Err(DesugarError { message: format!("unknown method category :{}", k), span: items[2].span }),
                };
                3
            }
            _ => 2,
        };
        let patterns = match items.get(pat_idx).map(|i| &i.node) {
            Some(Expr::Vec(pats)) => pats.iter().map(|p| self.desugar_method_pattern(p)).collect::<Result<Vec<_>, _>>()?,
            _ => return Err(DesugarError { message: "defmethod requires pattern vector".into(), span: items.get(pat_idx).map(|i| i.span).unwrap_or(span) }),
        };
        let mut goals = Vec::new();
        for g in &items[pat_idx + 1..] {
            goals.push(self.desugar_expr(g)?);
        }
        let body = if goals.len() == 1 { goals.pop().unwrap() } else { CoreExpr::new(CoreExprNode::Do(goals), span) };
        let def = CoreDef {
            name: Symbol::new(&format!("__method_{}", gen.as_str())), ty: None,
            effects: EffectRow::Pure, grade: Grade::Omega, mode: Mode::In, determinism: Determinism::Det,
            mode_sigs: vec![],
            body: CoreExpr::new(CoreExprNode::MethodDef(gen, category, patterns, Box::new(body)), span),
            requires: None, ensures: None, span,
        };
        Ok(Some(TopLevel::Def(def)))
    }

    /// §22.2 方法模式:(name Type) → 绑定 name 匹配 Type;(Type) → 无绑定;x → 变量
    fn desugar_method_pattern(&self, expr: &SExpr) -> Result<Pattern, DesugarError> {
        match &expr.node {
            Expr::Sym(_) => self.desugar_pattern(expr),
            Expr::List(items) if items.len() == 2 => {
                if let (Expr::Sym(name), Expr::Sym(ty)) = (&items[0].node, &items[1].node) {
                    return Ok(Pattern::Con(ty.clone(), vec![Pattern::Var(name.clone())]));
                }
                self.desugar_pattern(expr)
            }
            Expr::List(items) if items.len() == 1 => {
                if let Expr::Sym(ty) = &items[0].node {
                    return Ok(Pattern::Con(ty.clone(), vec![]));
                }
                self.desugar_pattern(expr)
            }
            _ => self.desugar_pattern(expr),
        }
    }

    /// §23.1:(defclass Name typevar (method [params] -> Ret) ...)
    fn desugar_defclass_form(&self, items: &[SExpr], span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let name = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Err(DesugarError { message: "defclass requires a name".into(), span }),
        };
        let tvars: Vec<Symbol> = items.get(2).and_then(|i| match &i.node { Expr::Sym(s) => Some(vec![s.clone()]), _ => None }).unwrap_or_default();
        let mut methods = Vec::new();
        for m in &items[3..] {
            if let Expr::List(parts) = &m.node {
                if let Some(Expr::Sym(mname)) = parts.first().map(|p| &p.node) {
                    // 方法签名 (method [params] -> Ret):解析返回类型,缺省 Unit
                    let ret = if parts.len() >= 4 {
                        if let Some(Expr::Keyword(kw)) = parts.get(2).map(|p| &p.node) {
                            if kw.as_str() == "->" {
                                self.desugar_type_with_params(&parts[3], &[])?
                            } else { Type::unit() }
                        } else { Type::unit() }
                    } else { Type::unit() };
                    methods.push((mname.clone(), ret));
                }
            }
        }
        let def = CoreDef {
            name: name.clone(), ty: None, effects: EffectRow::Pure, grade: Grade::Omega,
            mode: Mode::In, determinism: Determinism::Det,
            mode_sigs: vec![],
            body: CoreExpr::new(CoreExprNode::ClassDef(name, tvars, methods), span),
            requires: None, ensures: None, span,
        };
        Ok(Some(TopLevel::Def(def)))
    }

    /// §23.2:(definstance Class Type (method [params] body) ...)
    fn desugar_definstance_form(&self, items: &[SExpr], span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let class = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Err(DesugarError { message: "definstance requires a class name".into(), span }),
        };
        let types: Vec<Type> = items.get(2).map(|t| self.desugar_type_with_params(t, &[])).transpose()?.into_iter().collect();
        let mut methods = Vec::new();
        for m in &items[3..] {
            if let Expr::List(parts) = &m.node {
                if let Some(Expr::Sym(mname)) = parts.first().map(|p| &p.node) {
                    // 方法参数(§23.2):(method [x y] body...)
                    let params = if parts.len() >= 2 {
                        if let Expr::Vec(vs) = &parts[1].node {
                            vs.iter().filter_map(|v| match &v.node {
                                Expr::Sym(s) => Some(Param { name: s.clone(), ty: None, grade: Grade::Omega, mode: Mode::In }),
                                _ => None,
                            }).collect()
                        } else { Vec::new() }
                    } else { Vec::new() };
                    let mbody = if parts.len() >= 3 { self.desugar_expr(&parts[parts.len() - 1])? }
                        else { CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span) };
                    // 包装为 Lam:实例方法闭包经 apply 绑定参数
                    let wrapped = CoreExpr::new(
                        CoreExprNode::Lam(Lambda { params, body: Box::new(mbody), ret_type: None }),
                        span,
                    );
                    methods.push((mname.clone(), Box::new(wrapped)));
                }
            }
        }
        let def = CoreDef {
            name: Symbol::new(&format!("__instance_{}", class.as_str())), ty: None,
            effects: EffectRow::Pure, grade: Grade::Omega, mode: Mode::In, determinism: Determinism::Det,
            mode_sigs: vec![],
            body: CoreExpr::new(CoreExprNode::InstanceDef(class, types, methods), span),
            requires: None, ensures: None, span,
        };
        Ok(Some(TopLevel::Def(def)))
    }

    /// §28:(defprop name expr) — 声明验证属性(定理)
    fn desugar_defprop_form(&self, items: &[SExpr], span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let name = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Err(DesugarError { message: "defprop requires a name".into(), span }),
        };
        if items.len() < 3 {
            return Err(DesugarError { message: "defprop requires an expression".into(), span });
        }
        let prop = self.desugar_expr(&items[2])?;
        let def = CoreDef {
            name: Symbol::new(&format!("__prop_{}", name.as_str())), ty: None,
            effects: EffectRow::Pure, grade: Grade::Omega, mode: Mode::In, determinism: Determinism::Det,
            mode_sigs: vec![],
            body: CoreExpr::new(CoreExprNode::TheoremDef(name, Box::new(prop)), span),
            requires: None, ensures: None, span,
        };
        Ok(Some(TopLevel::Def(def)))
    }

    fn desugar_ns_form(&self, items: &[SExpr], _span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let name = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Ok(None),
        };
        let mut requires = Vec::new();
        let mut refers = Vec::new();
        // 解析 (:require [lib])/(:require [lib :as a])/(:refer [f]) 列表形式
        for item in &items[2..] {
            let (tag, entries): (Option<String>, Option<&Vec<SExpr>>) = match &item.node {
                Expr::List(l) if !l.is_empty() => {
                    match &l[0].node {
                        Expr::Keyword(kw) => (Some(kw.as_str().to_string()), l.get(1).and_then(|e| match &e.node { Expr::Vec(v) => Some(v), _ => None })),
                        _ => (None, None),
                    }
                }
                Expr::Keyword(kw) => {
                    // 顶层形式 :require [lib] 的 entries 在下一个 item
                    (Some(kw.as_str().to_string()), None)
                }
                _ => (None, None),
            };
            if let Some(tag) = tag {
                let entries = match entries {
                    Some(v) => Some(v.clone()),
                    None => {
                        // 顶层 keyword 形式:找紧随的 Vec
                        items.iter().skip_while(|x| x.span != item.span).nth(1)
                            .and_then(|n| match &n.node { Expr::Vec(v) => Some(v.clone()), _ => None })
                    }
                };
                if let Some(v) = entries {
                    for entry in v {
                        if let Expr::Sym(m) = &entry.node {
                            if tag == "require" { requires.push((m.clone(), m.clone())); }
                            if tag == "refer" { refers.push(m.clone()); }
                        } else if let Expr::List(l) = &entry.node {
                            if tag == "require" && l.len() >= 2 {
                                if let (Expr::Sym(m), Expr::Keyword(a)) = (&l[0].node, &l[1].node) {
                                    if a.as_str() == "as" && l.len() >= 3 {
                                        if let Expr::Sym(alias) = &l[2].node {
                                            requires.push((m.clone(), alias.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(Some(TopLevel::Namespace(name, requires, refers)))
    }

    fn desugar_defextern_form(&self, items: &[SExpr], _span: Span) -> Result<Option<TopLevel>, DesugarError> {
        if items.len() < 4 { return Ok(None); }
        let name = match &items[1].node { Expr::Sym(s) => s.clone(), _ => return Ok(None) };
        let c_name = match &items[2].node { Expr::Str(s) => s.clone(), _ => return Ok(None) };
        // §26 真实 dlopen:可选第三参为动态库路径 (defextern name "c_name" "libm.so.6")
        // c_name 编码为 "libpath:sym",ExternDef 求值时按 ffi feature 解析
        let c_name = match items.get(3).and_then(|i| match &i.node { Expr::Str(lib) => Some(lib.clone()), _ => None }) {
            Some(lib) => format!("{}:{}", lib, c_name),
            None => c_name,
        };
        Ok(Some(TopLevel::FFIDecl(name, c_name, vec![], None, vec![])))
    }
    fn desugar_defdata_hit_form(&self, items: &[SExpr], span: Span) -> Result<DataDecl, DesugarError> {
        let mut decl = self.desugar_defdata_form(items, span)?;
        decl.is_hit = true;
        // §7.4/16.3 HIT :boundary 声明:路径构造器端点一致性(声明位于构造器内部)
        for item in items.iter().skip(2) {
            if let Expr::List(parts) = &item.node {
                for i in 0..parts.len() {
                    if let Expr::Keyword(k) = &parts[i].node {
                        if k.as_str() == "boundary" && i + 1 < parts.len() {
                            decl.boundary = Some(format!("{:?}", parts[i + 1].node));
                            // 一致性检查:边界引用的符号须为构造器名或端点名(i0/i1/构造器)
                            self.check_hit_boundary(&decl, &parts[i + 1], span)?;
                        }
                    }
                }
            }
        }
        Ok(decl)
    }

    /// §16.3:HIT 边界检查 —— 边界表达式引用的符号须为构造器或端点;未知符号为边界违反
    fn check_hit_boundary(&self, decl: &DataDecl, boundary: &SExpr, span: Span) -> Result<(), DesugarError> {
        let known: Vec<String> = decl.constructors.iter()
            .map(|c| c.name.as_str().to_string())
            .chain(vec!["i0".to_string(), "i1".to_string()])
            .collect();
        // 边界表达式中的运算符(如 = != < >)与关键字不参与符号检查
        let operators = ["=", "!=", "<", ">", "<=", ">=", "and", "or", "not", "end"];
        let check = |e: &SExpr| -> Result<(), DesugarError> {
            match &e.node {
                Expr::Sym(sym) => {
                    if !operators.contains(&sym.as_str()) && !known.contains(&sym.as_str().to_string()) {
                        return Err(DesugarError {
                            message: format!("HIT 边界违反:符号 '{}' 不是构造器或端点", sym),
                            span,
                        });
                    }
                }
                _ => {}
            }
            Ok(())
        };
        // 递归检查所有符号
        fn walk(e: &SExpr, f: &dyn Fn(&SExpr) -> Result<(), DesugarError>) -> Result<(), DesugarError> {
            f(e)?;
            match &e.node {
                Expr::List(items) | Expr::Vec(items) => {
                    for i in items { walk(i, f)?; }
                }
                _ => {}
            }
            Ok(())
        }
        walk(boundary, &check)
    }

    /// §20.1:(defsession name 协议体) — 解析会话协议为 SessionType
    /// 协议语法:(send T rest)/(recv T rest)/(choice (label T)...)/(offer (label T)...)/(end)
    fn desugar_defsession_form(&self, items: &[SExpr], span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let name = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Ok(None),
        };
        if items.len() < 3 {
            return Ok(None);
        }
        // §20.2 MPST:支持 :role 角色分段 —— 每段解析为角色投影(单方协议)
        // (defsession Proto (A B) :role A (send ...) :role B (recv ...))
        let mut roles: Vec<(String, tisp_core::types::SessionType)> = Vec::new();
        let mut current_role: Option<String> = None;
        let mut current_items: Vec<&SExpr> = Vec::new();
        let mut plain: Option<tisp_core::types::SessionType> = None;
        // 角色列表 (A B):若 items[2] 是 List 且后面有 :role,跳过角色列表
        let mut i = 2;
        if roles_have_marker(items) {
            if let Expr::List(_) = &items[2].node { i = 3; }
        }
        while i < items.len() {
            if let Expr::Keyword(k) = &items[i].node {
                if k.as_str() == "role" && i + 1 < items.len() {
                    // 结算上一段
                    if let Some(role) = &current_role {
                        if !current_items.is_empty() {
                            let proto = self.desugar_session_type(&current_items[0], span)?;
                            roles.push((role.clone(), proto));
                        }
                    }
                    if let Expr::Sym(r) = &items[i + 1].node {
                        current_role = Some(r.as_str().to_string());
                        current_items.clear();
                        i += 2;
                        continue;
                    }
                }
            }
            if current_role.is_some() {
                current_items.push(&items[i]);
            } else if plain.is_none() {
                plain = Some(self.desugar_session_type(&items[i], span)?);
            }
            i += 1;
        }
        if let Some(role) = &current_role {
            if !current_items.is_empty() {
                let proto = self.desugar_session_type(&current_items[0], span)?;
                roles.push((role.clone(), proto));
            }
        }
        // 投影结果:有角色段取首段为 def 类型,其余角色段校验语法(解析成功即合法)
        let proto = if roles.is_empty() {
            plain.ok_or_else(|| DesugarError { message: "defsession requires a protocol".into(), span })?
        } else {
            roles[0].1.clone()
        };
        let body = CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span);
        let def = CoreDef { name, ty: Some(tisp_core::types::Type::Session(Box::new(proto))),
            effects: EffectRow::Closed(vec![EffectLabel::Session]),
            grade: Grade::Omega, mode: Mode::In, determinism: Determinism::Det,
            mode_sigs: vec![],
            body, requires: None, ensures: None, span };
        Ok(Some(TopLevel::Def(def)))
    }

    /// §20.1:解析会话协议片段
    fn desugar_session_type(&self, expr: &SExpr, span: Span) -> Result<tisp_core::types::SessionType, DesugarError> {
        use tisp_core::types::SessionType;
        match &expr.node {
            Expr::Sym(s) if s.as_str() == "end" => Ok(SessionType::End),
            Expr::Sym(s) => Ok(SessionType::Var(s.clone())), // 协议名递归引用
            Expr::List(items) if !items.is_empty() => {
                match &items[0].node {
                    Expr::Sym(head) => match head.as_str() {
                        "end" => Ok(SessionType::End),
                        "send" => {
                            if items.len() < 3 {
                                return Err(DesugarError { message: "send requires type and continuation".into(), span });
                            }
                            let t = self.desugar_type_with_params(&items[1], &[])?;
                            let rest = self.desugar_session_type(&items[2], span)?;
                            Ok(SessionType::Send(Box::new(t), Box::new(rest)))
                        }
                        "recv" => {
                            if items.len() < 3 {
                                return Err(DesugarError { message: "recv requires type and continuation".into(), span });
                            }
                            let t = self.desugar_type_with_params(&items[1], &[])?;
                            let rest = self.desugar_session_type(&items[2], span)?;
                            Ok(SessionType::Recv(Box::new(t), Box::new(rest)))
                        }
                        "choice" | "offer" => {
                            let mut branches = Vec::new();
                            for b in &items[1..] {
                                if let Expr::List(bi) = &b.node {
                                    if bi.len() == 2 {
                                        if let Expr::Sym(label) = &bi[0].node {
                                            let sub = self.desugar_session_type(&bi[1], span)?;
                                            branches.push((label.clone(), sub));
                                            continue;
                                        }
                                    }
                                }
                                return Err(DesugarError { message: "choice/offer branch must be (label protocol)".into(), span: b.span });
                            }
                            if head.as_str() == "choice" {
                                Ok(SessionType::Choice(branches))
                            } else {
                                Ok(SessionType::Offer(branches))
                            }
                        }
                        _ => Err(DesugarError { message: format!("unknown session protocol op {}", head), span }),
                    },
                    _ => Err(DesugarError { message: "invalid session protocol".into(), span }),
                }
            }
            _ => Err(DesugarError { message: "invalid session protocol".into(), span }),
        }
    }

    fn desugar_defdata_form(&self, items: &[SExpr], span: Span) -> Result<DataDecl, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "defdata requires name and at least one constructor".into(),
                span,
            });
        }

        // Parse name (possibly with type parameters)
        let (name, type_params) = match &items[1].node {
            Expr::Sym(s) => (s.clone(), Vec::new()),
            Expr::List(name_items) if !name_items.is_empty() => {
                if let Expr::Sym(s) = &name_items[0].node {
                    let params = name_items[1..].iter()
                        .map(|item| match &item.node {
                            Expr::Sym(p) => Ok(p.clone()),
                            _ => Err(DesugarError {
                                message: "type parameter must be a symbol".into(),
                                span: item.span,
                            }),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    (s.clone(), params)
                } else {
                    return Err(DesugarError {
                        message: "defdata name must be a symbol".into(),
                        span: items[1].span,
                    });
                }
            }
            _ => return Err(DesugarError {
                message: "defdata name must be a symbol or (name params...)".into(),
                span: items[1].span,
            }),
        };

        // Parse constructors and :deriving
        let mut constructors = Vec::new();
        let mut deriving = Vec::new();
        let mut i = 2;
        while i < items.len() {
            if let Expr::Keyword(kw) = &items[i].node {
                if kw.as_str() == "deriving" && i + 1 < items.len() {
                    // §7.5 支持 [Eq Show] 与 (Eq Show) 两种形式
                    let traits: Vec<&SExpr> = match &items[i + 1].node {
                        Expr::Vec(vs) => vs.iter().collect(),
                        Expr::List(vs) => vs.iter().collect(),
                        _ => vec![],
                    };
                    for t in traits {
                        if let Expr::Sym(s) = &t.node { deriving.push(s.clone()); }
                    }
                    i += 2;
                    continue;
                }
            }
            constructors.push(self.desugar_constructor(&items[i], &type_params)?);
            i += 1;
        }

        Ok(DataDecl {
            name,
            type_params,
            constructors,
            deriving,
            is_hit: false,
            boundary: None,
            span,
        })
    }

    fn desugar_constructor(&self, expr: &SExpr, type_params: &[Symbol]) -> Result<Constructor, DesugarError> {
        match &expr.node {
            Expr::Sym(name) => {
                // Constructor with no fields
                Ok(Constructor {
                    name: name.clone(),
                    fields: Vec::new(),
                    gadt_return_type: None,
                    span: expr.span,
                })
            }
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(name) = &items[0].node {
                    let mut fields = Vec::new();
                    let mut gadt_return = None;
                    let mut i = 1;
                    while i < items.len() {
                        if let Expr::Keyword(kw) = &items[i].node {
                            if kw.as_str() == "->" && i + 1 < items.len() {
                                gadt_return = Some(self.desugar_type_with_params(&items[i + 1], type_params)?);
                                i += 2;
                                continue;
                            }
                            // §7.4 HIT :boundary 声明(路径构造器端点)跳过,由 defdata-hit 层解析
                            if kw.as_str() == "boundary" && i + 1 < items.len() {
                                i += 2;
                                continue;
                            }
                        }
                        // §7.3 GADT 字段列表语法:[T1, T2, ...] → 多个匿名字段
                        if let Expr::Vec(vs) = &items[i].node {
                            for v in vs {
                                let ty = self.desugar_type_with_params(v, type_params)?;
                                fields.push(Field { name: None, ty });
                            }
                            i += 1;
                            continue;
                        }
                        // §7.2 记录字段语法 {name : T, ...} → 多个命名字段
                        if let Expr::Map(pairs) = &items[i].node {
                            for (k, v) in pairs {
                                if let Expr::Sym(fname) = &k.node {
                                    let ty = match &v.node {
                                        Expr::List(vs) if vs.len() == 1 =>
                                            self.desugar_type_with_params(&vs[0], type_params)?,
                                        Expr::List(vs) if vs.len() >= 3 =>
                                            self.desugar_type_with_params(&vs[0], type_params)?,
                                        _ => self.desugar_type_with_params(v, type_params)?,
                                    };
                                    fields.push(Field { name: Some(fname.clone()), ty });
                                }
                            }
                            i += 1;
                            continue;
                        }
                        fields.push(self.desugar_field(&items[i], type_params)?);
                        i += 1;
                    }
                    Ok(Constructor {
                        name: name.clone(),
                        fields,
                        gadt_return_type: gadt_return,
                        span: expr.span,
                    })
                } else {
                    Err(DesugarError {
                        message: "constructor name must be a symbol".into(),
                        span: items[0].span,
                    })
                }
            }
            _ => Err(DesugarError {
                message: "invalid constructor syntax".into(),
                span: expr.span,
            }),
        }
    }

    fn desugar_field(&self, expr: &SExpr, type_params: &[Symbol]) -> Result<Field, DesugarError> {
        match &expr.node {
            Expr::Sym(name) => {
                // Check if this is a type parameter or built-in type
                if type_params.contains(name) || Self::is_builtin_type(name.as_str()) {
                    // Anonymous field with this type
                    let ty = self.desugar_type_with_params(expr, type_params)?;
                    Ok(Field { name: None, ty })
                } else {
                    // Field with no type annotation (will be inferred)
                    Ok(Field {
                        name: Some(name.clone()),
                        ty: tisp_core::types::Type::Var(tisp_core::types::TypeVar {
                            name: Symbol::new("_"),
                            kind: tisp_core::types::Kind::Star,
                            id: 0,
                        }),
                    })
                }
            }
            Expr::List(items) if !items.is_empty() => {
                // Could be either:
                // 1. (name Type) - named field with type annotation
                // 2. (Type arg1 arg2 ...) - anonymous field with type application
                
                // Check if first element is a symbol that could be a field name
                if let Expr::Sym(first) = &items[0].node {
                    // If it's a type parameter or built-in type, treat as type application
                    if type_params.contains(first) || Self::is_builtin_type(first.as_str()) {
                        let ty = self.desugar_type_with_params(expr, type_params)?;
                        Ok(Field { name: None, ty })
                    } else if items.len() == 2 {
                        // Could be (name Type) or (Type arg)
                        // If the first symbol is NOT a type parameter and NOT a built-in type,
                        // it's likely a type constructor, so treat as type application
                        match &items[1].node {
                            Expr::Sym(second) if type_params.contains(second) || Self::is_builtin_type(second.as_str()) => {
                                // If first is also a type param or builtin, it's (Type arg)
                                // Otherwise, it could be (name Type)
                                // Heuristic: if first starts with uppercase, it's likely a type constructor
                                if first.as_str().chars().next().map_or(false, |c| c.is_uppercase()) {
                                    let ty = self.desugar_type_with_params(expr, type_params)?;
                                    Ok(Field { name: None, ty })
                                } else {
                                    let ty = self.desugar_type_with_params(&items[1], type_params)?;
                                    Ok(Field { name: Some(first.clone()), ty })
                                }
                            }
                            Expr::List(_) => {
                                // (name (Type ...)) - named field with type application
                                let ty = self.desugar_type_with_params(&items[1], type_params)?;
                                Ok(Field { name: Some(first.clone()), ty })
                            }
                            _ => {
                                // Treat as type application
                                let ty = self.desugar_type_with_params(expr, type_params)?;
                                Ok(Field { name: None, ty })
                            }
                        }
                    } else {
                        // Multiple elements, treat as type application
                        let ty = self.desugar_type_with_params(expr, type_params)?;
                        Ok(Field { name: None, ty })
                    }
                } else {
                    // First element is not a symbol, treat as type application
                    let ty = self.desugar_type_with_params(expr, type_params)?;
                    Ok(Field { name: None, ty })
                }
            }
            _ => Err(DesugarError {
                message: "invalid field syntax".into(),
                span: expr.span,
            }),
        }
    }

    fn is_builtin_type(name: &str) -> bool {
        matches!(name, "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | 
                       "f32" | "f64" | "bool" | "String" | "Unit")
    }

    fn desugar_type_with_params(&self, expr: &SExpr, type_params: &[Symbol]) -> Result<tisp_core::types::Type, DesugarError> {
        match &expr.node {
            Expr::Sym(name) => {
                // Check if this is a type parameter
                if type_params.contains(name) {
                    Ok(tisp_core::types::Type::Var(tisp_core::types::TypeVar {
                        name: name.clone(),
                        kind: tisp_core::types::Kind::Star,
                        id: 0,
                    }))
                } else {
                    match name.as_str() {
                        "i8" => Ok(tisp_core::types::Type::i8()),
                        "i16" => Ok(tisp_core::types::Type::i16()),
                        "i32" => Ok(tisp_core::types::Type::i32()),
                        "i64" => Ok(tisp_core::types::Type::i64()),
                        "u8" => Ok(tisp_core::types::Type::u8()),
                        "u16" => Ok(tisp_core::types::Type::u16()),
                        "u32" => Ok(tisp_core::types::Type::u32()),
                        "u64" => Ok(tisp_core::types::Type::u64()),
                        "f32" => Ok(tisp_core::types::Type::f32()),
                        "f64" => Ok(tisp_core::types::Type::f64()),
                        "bool" => Ok(tisp_core::types::Type::bool()),
                        "String" => Ok(tisp_core::types::Type::string()),
                        "Unit" => Ok(tisp_core::types::Type::unit()),
                        // 旧风格类型名别名
                        "Int" => Ok(tisp_core::types::Type::i64()),
                        "Float" => Ok(tisp_core::types::Type::f64()),
                        "Bool" => Ok(tisp_core::types::Type::bool()),
                        "Str" => Ok(tisp_core::types::Type::string()),
                        _ => Ok(tisp_core::types::Type::Con(tisp_core::types::TypeCon {
                            name: name.clone(),
                            kind: tisp_core::types::Kind::Star,
                        })),
                    }
                }
            }
            // Refinement type: {name : Type | predicate}
            Expr::Map(pairs) if !pairs.is_empty() => {
                self.desugar_refined_type(expr, pairs, type_params)
            }
            Expr::List(items) if !items.is_empty() => {
                // §18 时序模态类型:(next T)/(always T)/(eventually T)
                if let Expr::Sym(head) = &items[0].node {
                    if matches!(head.as_str(), "next" | "always" | "eventually") && items.len() == 2 {
                        let inner = self.desugar_type_with_params(&items[1], type_params)?;
                        let op = match head.as_str() {
                            "next" => tisp_core::types::TemporalOp::Next,
                            "always" => tisp_core::types::TemporalOp::Always,
                            _ => tisp_core::types::TemporalOp::Eventually,
                        };
                        return Ok(tisp_core::types::Type::Temporal(op, Box::new(inner)));
                    }
                }
                // 依赖类型(§19.1):(pi (x : T) R) / (forall (x : T) R) / (sigma (x : T) R)
                if let Expr::Sym(head) = &items[0].node {
                    if matches!(head.as_str(), "pi" | "forall" | "sigma") && items.len() == 3 {
                        if let Expr::List(binder) = &items[1].node {
                            // binder = (x : T):[Sym(x), Keyword(:), T]
                            if binder.len() == 3 {
                                if let (Expr::Sym(bv), Expr::Keyword(colon)) = (&binder[0].node, &binder[1].node) {
                                    if colon.as_str() == ":" {
                                        let dom = self.desugar_type_with_params(&binder[2], type_params)?;
                                        let cod = self.desugar_type_with_params(&items[2], type_params)?;
                                        return if head.as_str() == "sigma" {
                                            Ok(tisp_core::types::Type::Sigma(bv.clone(), Box::new(dom), Box::new(cod)))
                                        } else {
                                            Ok(tisp_core::types::Type::Pi(bv.clone(), Box::new(dom), Box::new(cod)))
                                        };
                                    }
                                }
                            }
                        }
                        return Err(DesugarError {
                            message: format!("{} requires binder (x : T)", head),
                            span: expr.span,
                        });
                    }
                }
                // Type application: (List i32), (Map String i32), etc.
                let base = self.desugar_type_with_params(&items[0], type_params)?;
                let mut result = base;
                for item in &items[1..] {
                    let arg = self.desugar_type_with_params(item, type_params)?;
                    result = tisp_core::types::Type::App(Box::new(result), Box::new(arg));
                }
                Ok(result)
            }
            _ => Err(DesugarError {
                message: "invalid type syntax".into(),
                span: expr.span,
            }),
        }
    }

    fn desugar_refined_type(&self, _expr: &SExpr, pairs: &[(SExpr, SExpr)], type_params: &[Symbol]) -> Result<tisp_core::types::Type, DesugarError> {
        // Parse {name : baseType | predicate}
        for (key, val) in pairs {
            if let Expr::Sym(_name) = &key.node {
                if let Expr::List(val_items) = &val.node {
                    if val_items.len() >= 2 {
                        // Last element is the predicate, items before are the base type
                        // But we need to find the | separator
                        for j in 1..val_items.len() {
                            if matches!(&val_items[j].node, Expr::Sym(s) if s.as_str() == "|")
                                || matches!(&val_items[j].node, Expr::Keyword(k) if k.as_str() == "|") {
                                let type_items = &val_items[..j];
                                let pred_items = &val_items[j+1..];
                                
                                let base_type = if type_items.is_empty() {
                                    return Err(DesugarError {
                                        message: "refinement type missing base type".into(),
                                        span: key.span,
                                    });
                                } else if type_items.len() == 1 {
                                    self.desugar_type_with_params(&type_items[0], type_params)?
                                } else {
                                    let mut result = self.desugar_type_with_params(&type_items[0], type_params)?;
                                    for item in &type_items[1..] {
                                        let arg = self.desugar_type_with_params(item, type_params)?;
                                        result = tisp_core::types::Type::App(Box::new(result), Box::new(arg));
                                    }
                                    result
                                };

                                let predicate = if pred_items.is_empty() {
                                    return Err(DesugarError {
                                        message: "refinement type missing predicate".into(),
                                        span: key.span,
                                    });
                                } else if pred_items.len() == 1 {
                                    self.desugar_predicate(&pred_items[0])?
                                } else {
                                    let mut preds = Vec::new();
                                    for item in pred_items {
                                        preds.push(self.desugar_predicate(item)?);
                                    }
                                    let mut result = preds.remove(0);
                                    for p in preds {
                                        result = Predicate::And(Box::new(result), Box::new(p));
                                    }
                                    result
                                };

                                return Ok(tisp_core::types::Type::Refined(
                                    Box::new(base_type),
                                    Box::new(predicate),
                                ));
                            }
                        }
                        return Err(DesugarError {
                            message: "refinement type missing '|' separator".into(),
                            span: key.span,
                        });
                    }
                }
            }
        }
        Err(DesugarError {
            message: "invalid refinement type syntax, expected {name : Type | predicate}".into(),
            span: Span::dummy(),
        })
    }

    fn desugar_predicate(&self, expr: &SExpr) -> Result<Predicate, DesugarError> {
        match &expr.node {
            Expr::Sym(name) => {
                match name.as_str() {
                    "true" => Ok(Predicate::Lit(true)),
                    "false" => Ok(Predicate::Lit(false)),
                    _ => Ok(Predicate::Var(name.clone())),
                }
            }
            Expr::Int(n) => {
                Ok(Predicate::App(Symbol::new(&n.to_string()), vec![]))
            }
            Expr::Float(_) => {
                Ok(Predicate::Lit(true)) // placeholder
            }
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(op_name) = &items[0].node {
                    match op_name.as_str() {
                        "!" | "not" => {
                            if items.len() != 2 {
                                return Err(DesugarError { message: "not takes 1 arg".into(), span: expr.span });
                            }
                            Ok(Predicate::Not(Box::new(self.desugar_predicate(&items[1])?)))
                        }
                        "and" | "&&" => {
                            let mut preds: Vec<Predicate> = items[1..].iter()
                                .map(|i| self.desugar_predicate(i))
                                .collect::<Result<Vec<_>, _>>()?;
                            if preds.is_empty() { return Ok(Predicate::Lit(true)); }
                            let mut result = preds.remove(0);
                            for p in preds { result = Predicate::And(Box::new(result), Box::new(p)); }
                            Ok(result)
                        }
                        "or" | "||" => {
                            let mut preds: Vec<Predicate> = items[1..].iter()
                                .map(|i| self.desugar_predicate(i))
                                .collect::<Result<Vec<_>, _>>()?;
                            if preds.is_empty() { return Ok(Predicate::Lit(false)); }
                            let mut result = preds.remove(0);
                            for p in preds { result = Predicate::Or(Box::new(result), Box::new(p)); }
                            Ok(result)
                        }
                        "=" | "==" => {
                            if items.len() != 3 { return Err(DesugarError { message: "= needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Eq,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        "!=" => {
                            if items.len() != 3 { return Err(DesugarError { message: "!= needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Ne,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        "<" => {
                            if items.len() != 3 { return Err(DesugarError { message: "< needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Lt,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        "<=" => {
                            if items.len() != 3 { return Err(DesugarError { message: "<= needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Le,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        ">" => {
                            if items.len() != 3 { return Err(DesugarError { message: "> needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Gt,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        ">=" => {
                            if items.len() != 3 { return Err(DesugarError { message: ">= needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Ge,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        _ => {
                            let args: Vec<Predicate> = items[1..].iter()
                                .map(|i| self.desugar_predicate(i))
                                .collect::<Result<Vec<_>, _>>()?;
                            Ok(Predicate::App(op_name.clone(), args))
                        }
                    }
                } else {
                    Err(DesugarError { message: "predicate must start with sym".into(), span: expr.span })
                }
            }
            _ => Err(DesugarError { message: "invalid predicate".into(), span: expr.span }),
        }
    }

    fn desugar_term(&self, expr: &SExpr) -> Result<tisp_core::types::Term, DesugarError> {
        match &expr.node {
            Expr::Int(n) => Ok(tisp_core::types::Term::Lit(Lit::Int(*n))),
            Expr::Sym(name) => Ok(tisp_core::types::Term::Var(name.clone())),
            Expr::Bool(b) => Ok(tisp_core::types::Term::Lit(Lit::Bool(*b))),
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(op_name) = &items[0].node {
                    match op_name.as_str() {
                        "+" | "-" | "*" | "/" | "%" if items.len() == 3 => {
                            let op = match op_name.as_str() {
                                "+" => BinOp::Add, "-" => BinOp::Sub, "*" => BinOp::Mul,
                                "/" => BinOp::Div, "%" => BinOp::Mod, _ => unreachable!(),
                            };
                            Ok(tisp_core::types::Term::BinOp(op,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        _ => {
                            let args: Vec<tisp_core::types::Term> = items[1..].iter()
                                .map(|i| self.desugar_term(i))
                                .collect::<Result<Vec<_>, _>>()?;
                            Ok(tisp_core::types::Term::App(op_name.clone(), args))
                        }
                    }
                } else {
                    Err(DesugarError { message: "term must start with sym".into(), span: expr.span })
                }
            }
            _ => Err(DesugarError { message: "invalid term".into(), span: expr.span }),
        }
    }

    fn desugar_def_form(&self, items: &[SExpr], span: Span) -> Result<CoreDef, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "def requires name and body".into(),
                span,
            });
        }

        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError {
                message: "def name must be a symbol".into(),
                span: items[1].span,
            }),
        };

        let body = self.desugar_expr(&items[2])?;

        Ok(CoreDef {
            name,
            ty: None,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            determinism: Determinism::Det,
            mode_sigs: vec![],
            body,
            requires: None,
            ensures: None,
            span,
        })
    }

    fn desugar_defn_form(&self, items: &[SExpr], span: Span) -> Result<CoreDef, DesugarError> {
        if items.len() < 4 {
            return Err(DesugarError {
                message: "defn requires name, params, and body".into(),
                span,
            });
        }

        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError {
                message: "defn name must be a symbol".into(),
                span: items[1].span,
            }),
        };

        // ── Multi-arity detection ──
        // (defn name ([p1] b1) ([p2] b2) ...)
        if let Expr::List(first_clause) = &items[2].node {
            if !first_clause.is_empty() && matches!(&first_clause[0].node, Expr::Vec(_)) {
                return self.desugar_multi_arity_defn(name, &items[2..], span);
            }
        }

        let params = self.desugar_params(&items[2])?;

        // Look for -> ReturnType annotation
        let mut ret_type = None;
        let mut body_start = 3;
        while body_start < items.len() {
            // `->` 被词法解析为 Sym(而非 Keyword),两种都接受
            let is_arrow = match &items[body_start].node {
                Expr::Keyword(kw) => kw.as_str() == "->",
                Expr::Sym(s) => s.as_str() == "->",
                _ => false,
            };
            if is_arrow {
                if body_start + 1 < items.len() {
                    ret_type = Some(self.desugar_type_with_params(&items[body_start + 1], &[])?);
                    body_start += 2;
                } else {
                    return Err(DesugarError { message: "-> requires a return type".into(), span: items[body_start].span });
                }
            } else {
                break;
            }
        }

        let mut requires = None;
        let mut ensures = None;

        let mut i = 3;
        let mut last_non_kw = None;
        while i < items.len() {
            if let Expr::Keyword(kw) = &items[i].node {
                match kw.as_str() {
                    // lexer 产出 Keyword("requires")(无冒号);兼容带冒号形式
                    "requires" | ":requires" => {
                        if i + 1 < items.len() && !matches!(&items[i+1].node, Expr::Keyword(_)) {
                            let pred = self.desugar_predicate(&items[i + 1])?;
                            // 多个 :requires 合取为 And
                            requires = Some(match requires {
                                Some(prev) => Predicate::And(Box::new(prev), Box::new(pred)),
                                None => pred,
                            });
                            i += 2;
                        } else {
                            return Err(DesugarError { message: ":requires needs a predicate".into(), span: items[i].span });
                        }
                    }
                    "ensures" | ":ensures" => {
                        if i + 1 < items.len() && !matches!(&items[i+1].node, Expr::Keyword(_)) {
                            ensures = Some(self.desugar_predicate(&items[i + 1])?);
                            i += 2;
                        } else {
                            return Err(DesugarError { message: ":ensures needs a predicate".into(), span: items[i].span });
                        }
                    }
                    _ => {
                        last_non_kw = Some(i);
                        i += 1;
                    }
                }
            } else {
                last_non_kw = Some(i);
                i += 1;
            }
        }

        let _body_idx = last_non_kw.ok_or_else(|| DesugarError {
            message: "defn requires a body expression".into(),
            span,
        })?;

        // Collect all body expressions (non-keyword items after params/->)
        let mut body_exprs = Vec::new();
        for idx in body_start..items.len() {
            if let Expr::Keyword(_) = &items[idx].node { continue; }
            if idx > body_start {
                if let Expr::Keyword(kw) = &items[idx - 1].node {
                    if matches!(kw.as_str(), "requires" | ":requires" | "ensures" | ":ensures") { continue; }
                }
            }
            body_exprs.push(self.desugar_expr(&items[idx])?);
        }

        let body = if body_exprs.len() == 1 {
            body_exprs.into_iter().next().unwrap()
        } else {
            CoreExpr::new(CoreExprNode::Do(body_exprs), span)
        };

        let lambda = CoreExprNode::Lam(Lambda {
            params,
            body: Box::new(body),
            ret_type: None,
        });

        Ok(CoreDef {
            name,
            ty: ret_type,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            determinism: Determinism::Det,
            mode_sigs: vec![],
            body: CoreExpr::new(lambda, span),
            requires,
            ensures,
            span,
        })
    }

    fn desugar_defeffect_form(&self, items: &[SExpr], span: Span) -> Result<tisp_core::effects::EffectDecl, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError { message: "defeffect requires name and at least one operation".into(), span });
        }
        // Parse name (possibly with type params): (defeffect State s ...) or (defeffect IO ...)
        let (name, type_params) = match &items[1].node {
            Expr::Sym(s) => (s.clone(), Vec::new()),
            Expr::List(name_items) if !name_items.is_empty() => {
                if let Expr::Sym(s) = &name_items[0].node {
                    let params = name_items[1..].iter().filter_map(|i| {
                        if let Expr::Sym(p) = &i.node { Some(p.clone()) } else { None }
                    }).collect();
                    (s.clone(), params)
                } else {
                    return Err(DesugarError { message: "effect name must be symbol".into(), span: items[1].span });
                }
            }
            _ => return Err(DesugarError { message: "effect name must be symbol or (name params)".into(), span: items[1].span }),
        };

        let mut operations = Vec::new();
        for item in &items[2..] {
            if let Expr::List(op_items) = &item.node {
                if op_items.len() >= 2 {
                    if let Expr::Sym(op_name) = &op_items[0].node {
                        let mut params = Vec::new();
                        let mut return_type = tisp_core::types::Type::unit();
                        // Parse operation: (get [] -> s) or (put [s] -> Unit)
                        for op_item in &op_items[1..] {
                            match &op_item.node {
                                Expr::Vec(v) => {
                                    for p in v {
                                        params.push(self.desugar_type_with_params(p, &type_params)?);
                                    }
                                }
                                Expr::Keyword(kw) if kw.as_str() == "->" => {}
                                _ => {
                                    // Might be a return type or type app
                                    return_type = self.desugar_type_with_params(op_item, &type_params)?;
                                }
                            }
                        }
                        operations.push(tisp_core::effects::OperationDecl {
                            name: op_name.clone(),
                            params,
                            return_type,
                        });
                    }
                }
            }
        }
        Ok(tisp_core::effects::EffectDecl { name, type_params, operations })
    }

    fn desugar_defpred_form(&self, items: &[SExpr], span: Span) -> Result<CoreDef, DesugarError> {
        if items.len() < 4 {
            return Err(DesugarError { message: "defpred requires name, params, and body".into(), span });
        }
        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "defpred name must be a symbol".into(), span: items[1].span }),
        };
        let params = self.desugar_params(&items[2])?;
        // 解析确定性注解(§21.2/§14)::det/:semidet/:multi/:nondet/:cc_multi/:cc_nondet
        let mut determinism = Determinism::NonDet;
        for item in &items[3..] {
            if let Expr::Keyword(k) = &item.node {
                determinism = match k.as_str() {
                    "det" => Determinism::Det,
                    "semidet" => Determinism::SemiDet,
                    "multi" => Determinism::Multi,
                    "nondet" => Determinism::NonDet,
                    "cc_multi" => Determinism::CcMulti,
                    "cc_nondet" => Determinism::CcNonDet,
                    "failure" => Determinism::Failure,
                    "erroneous" => Determinism::Erroneous,
                    _ => break,
                };
            } else {
                break;
            }
        }
        // 多模式签名(§13)::mode (i o) / :mode (o i) — i=输入(In),o=输出(Out)
        let mut mode_sigs: Vec<Vec<Mode>> = Vec::new();
        let mut i = 3;
        while i < items.len() {
            if let Expr::Keyword(k) = &items[i].node {
                if k.as_str() == "mode" && i + 1 < items.len() {
                    if let Expr::List(sig_items) = &items[i + 1].node {
                        let mut sig = Vec::new();
                        for s in sig_items {
                            match &s.node {
                                Expr::Sym(sym) => match sym.as_str() {
                                    "i" | "in" => sig.push(Mode::In),
                                    "o" | "out" => sig.push(Mode::Out),
                                    _ => break,
                                },
                                _ => break,
                            }
                        }
                        if !sig.is_empty() { mode_sigs.push(sig); }
                    }
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }
        // 子句形式检测(§21.2 Mercury 风格):([P1 P2 ...] body...) 首项为 Vec
        // 子句形式检测(§21.2 Mercury 风格):([P1 P2 ...] body...) 首项为 Vec;
        // 跳过 :mode 及其签名列表(§13)
        let is_clause_form = items[3..].iter().enumerate().any(|(idx, c)| {
            if matches!(&c.node, Expr::Keyword(k) if k.as_str() == "mode") { return false; }
            // :mode (i o) 的签名列表是 List,须跳过
            if idx > 0 && matches!(&items[2 + idx - 1].node, Expr::Keyword(k) if k.as_str() == "mode") { return false; }
            matches!(&c.node, Expr::List(parts) if !parts.is_empty() && matches!(&parts[0].node, Expr::Vec(_)))
        });
        let body = if is_clause_form {
            // 每个子句编译为 Match 的一个 arm:参数打包成 __tuple,子句模式与之匹配;
            // 无 arm 匹配返回 Err → Search 节点据此回溯(§21.4)
            let mut arms = Vec::new();
            let mut ci = 3;
            while ci < items.len() {
                let clause = &items[ci];
                // 跳过 :det/:nondet 等模式注解与 :mode 签名(§21.2/§13)
                if matches!(&clause.node, Expr::Keyword(_)) {
                    ci += 1;
                    continue;
                }
                if ci > 3 && matches!(&items[ci - 1].node, Expr::Keyword(k) if k.as_str() == "mode") {
                    ci += 1;
                    continue;
                }
                match &clause.node {
                    Expr::List(parts) if !parts.is_empty() => {
                        // 模式列表来源(§21.2):
                        // 1. [模式...] 向量(元素即模式)
                        // 2. [([p1 p2 ...])] 向量包圆括号模式列表
                        // 3. (模式...) 圆括号模式列表
                        let pattern_exprs: Vec<&SExpr> = match &parts[0].node {
                            Expr::Vec(vs) if vs.len() == 1 => {
                                if let Expr::List(inner) = &vs[0].node {
                                    inner.iter().collect()
                                } else {
                                    vs.iter().collect()
                                }
                            }
                            Expr::Vec(vs) => vs.iter().collect(),
                            Expr::List(inner) => inner.iter().collect(),
                            _ => return Err(DesugarError {
                                message: "defpred 子句要求模式列表".into(),
                                span: clause.span,
                            }),
                        };
                        let sub_patterns: Vec<Pattern> = pattern_exprs.iter()
                            .map(|p| self.desugar_pattern(p))
                            .collect::<Result<_, _>>()?;
                        let pattern = Pattern::Con(Symbol::new("__tuple"), sub_patterns);
                        let clause_body = if parts.len() > 1 {
                            let mut goals = Vec::new();
                            for g in &parts[1..] {
                                goals.push(self.desugar_expr(g)?);
                            }
                            if goals.len() == 1 {
                                goals.pop().unwrap()
                            } else {
                                CoreExpr::new(CoreExprNode::Do(goals), span)
                            }
                        } else {
                            CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span)
                        };
                        arms.push(MatchArm { pattern, guard: None, body: Box::new(clause_body) });
                        ci += 1;
                        continue;
                    }
                    _ => {}
                }
                return Err(DesugarError {
                    message: "defpred 子句形式要求 ([pattern...] body...)".into(),
                    span: clause.span,
                });
            }
            let scrutinee = CoreExpr::new(
                CoreExprNode::Data(
                    Symbol::new("__tuple"),
                    params.iter().map(|p| {
                        CoreExpr::new(CoreExprNode::Var(p.name.clone()), span)
                    }).collect(),
                ),
                span,
            );
            // Search 包装:子句全部失败 → 返回 false 而非传播 match failure(§21.4 回溯);
            // §14.3 committed-choice:cc_multi/cc_nondet 谓词只尝试首个子句并提交(cut)
            let is_cc = matches!(determinism, Determinism::CcMulti | Determinism::CcNonDet);
            let arms = if is_cc { arms.into_iter().take(1).collect() } else { arms };
            let search = CoreExpr::new(
                CoreExprNode::Search(Box::new(CoreExpr::new(CoreExprNode::Match(Box::new(scrutinee), arms), span))),
                span,
            );
            if is_cc {
                CoreExpr::new(CoreExprNode::Commit(Box::new(search)), span)
            } else {
                search
            }
        } else {
            // 普通目标表达式形式(现有语义)
            let mut clauses = Vec::new();
            for idx in 3..items.len() {
                clauses.push(self.desugar_expr(&items[idx])?);
            }
            if clauses.len() == 1 {
                clauses.into_iter().next().unwrap()
            } else {
                CoreExpr::new(CoreExprNode::Do(clauses), span)
            }
        };
        let lambda = CoreExprNode::Lam(Lambda { params, body: Box::new(body), ret_type: None });
        Ok(CoreDef {
            name, ty: None, effects: EffectRow::Open(vec![EffectLabel::Search], Box::new(EffectRow::Pure)),
            grade: Grade::Omega, mode: Mode::Free, mode_sigs, determinism,
            body: CoreExpr::new(lambda, span), requires: None, ensures: None, span,
        })
    }

    fn desugar_params(&self, expr: &SExpr) -> Result<Vec<Param>, DesugarError> {
        match &expr.node {
            Expr::Vec(items) => {
                let mut params = Vec::new();
                let mut i = 0;
                while i < items.len() {
                    let item = &items[i];
                    match &item.node {
                        Expr::Sym(name) => {
                            let grade = Grade::Omega;
                            let mut ty = None;
                            let name = name.clone();
                            // Check if next items are : type
                            let mut mode = Mode::In;
                            if i + 2 < items.len() {
                                if let Expr::Keyword(kw) = &items[i + 1].node {
                                    if kw.as_str() == ":" {
                                        ty = Some(self.desugar_type_with_params(&items[i + 2], &[])?);
                                        i += 2; // skip : and type
                                    } else if kw.as_str() == "free" {
                                        // §21.2 模式注解:name :free(输出逻辑变量)
                                        mode = Mode::Free;
                                        i += 1;
                                    } else if kw.as_str() == "ground" {
                                        mode = Mode::In;
                                        i += 1;
                                    }
                                }
                            }
                            params.push(Param { name, ty, grade, mode });
                            i += 1;
                        }
                        // Graded parameter: (grade name : type) or (grade name)
                        Expr::List(parts) if !parts.is_empty() => {
                            self.desugar_graded_param(parts, &mut params)?;
                            i += 1;
                        }
                        // Graded parameter: {grade name : type} — parsed as Map
                        Expr::Map(pairs) if !pairs.is_empty() => {
                            let grade = self.desugar_grade_expr(&pairs[0].0)?;
                            let name = match &pairs[0].1.node {
                                Expr::Sym(s) => s.clone(),
                                _ => return Err(DesugarError { message: "graded param name must be symbol".into(), span: item.span }),
                            };
                            let ty = if pairs.len() >= 2 {
                                match &pairs[1].0.node {
                                    Expr::Keyword(k) if k.as_str() == ":" => Some(self.desugar_type_with_params(&pairs[1].1, &[])?),
                                    _ => None,
                                }
                            } else { None };
                            params.push(Param { name, ty, grade, mode: Mode::In });
                            i += 1;
                        }
                        // Graded parameter: {grade name : type} — parsed as Set
                        Expr::Set(items) if !items.is_empty() => {
                            match &items[0].node {
                                Expr::Int(0) | Expr::Int(1) | Expr::Sym(_) => {
                                    if items.len() < 2 { return Err(DesugarError { message: "graded param needs a name".into(), span: item.span }); }
                                    let grade = match &items[0].node {
                                        Expr::Int(0) => Grade::Zero,
                                        Expr::Int(1) => Grade::One,
                                        _ => Grade::Omega,
                                    };
                                    let name = match &items[1].node {
                                        Expr::Sym(s) => s.clone(),
                                        _ => return Err(DesugarError { message: "graded param needs a name symbol".into(), span: items[1].span }),
                                    };
                                    let ty = if items.len() >= 4 && matches!(&items[2].node, Expr::Keyword(k) if k.as_str() == ":") {
                                        Some(self.desugar_type_with_params(&items[3], &[])?)
                                    } else { None };
                                    params.push(Param { name, ty, grade, mode: Mode::In });
                                }
                                _ => return Err(DesugarError { message: "graded param grade must be 0, 1, or ω".into(), span: items[0].span }),
                            }
                            i += 1;
                        }
                        // Skip standalone separator tokens
                        Expr::Keyword(_) => { i += 1; }
                        _ => return Err(DesugarError {
                            message: "parameter must be a symbol or (grade name : type)".into(),
                            span: item.span,
                        }),
                    }
                }
                Ok(params)
            }
            _ => Err(DesugarError {
                message: "parameters must be a vector".into(),
                span: expr.span,
            }),
        }
    }

    /// §10 依赖等级:解析等级表达式
    /// 数字 0/1 → Zero/One;数字 n>1 → Nat(n);ω/omega → Omega;
    /// 符号 → Var(等级变量);(op a b) → Add/Mul(+/*);其余报错
    fn desugar_grade_expr(&self, expr: &SExpr) -> Result<Grade, DesugarError> {
        match &expr.node {
            Expr::Int(0) => Ok(Grade::Zero),
            Expr::Int(1) => Ok(Grade::One),
            Expr::Int(n) if *n > 1 => Ok(Grade::Nat(*n as u64)),
            Expr::Int(n) => Err(DesugarError {
                message: format!("负等级 {} 无效", n),
                span: expr.span,
            }),
            Expr::Sym(s) if s.as_str() == "ω" || s.as_str() == "omega" => Ok(Grade::Omega),
            Expr::Sym(s) => Ok(Grade::Var(s.clone())),
            Expr::List(items) if items.len() == 3 => {
                if let Expr::Sym(op) = &items[0].node {
                    let a = self.desugar_grade_expr(&items[1])?;
                    let b = self.desugar_grade_expr(&items[2])?;
                    match op.as_str() {
                        "+" => Ok(Grade::Add(Box::new(a), Box::new(b))),
                        "*" => Ok(Grade::Mul(Box::new(a), Box::new(b))),
                        _ => Err(DesugarError {
                            message: format!("不支持的等级运算 '{}'(仅 + 与 *)", op),
                            span: expr.span,
                        }),
                    }
                } else {
                    Err(DesugarError { message: "等级表达式须以运算符开头".into(), span: expr.span })
                }
            }
            _ => Err(DesugarError {
                message: "grade must be 0, 1, ω, a symbol, or a grade expression (+ *)".into(),
                span: expr.span,
            }),
        }
    }

    fn desugar_graded_param(&self, parts: &[SExpr], params: &mut Vec<Param>) -> Result<(), DesugarError> {
        if parts.is_empty() { return Ok(()); }
        // Parse grade from first element(§10 依赖等级:数字/符号/复合表达式)
        let grade = self.desugar_grade_expr(&parts[0])?;
        // Parse name from second element
        if parts.len() < 2 {
            return Err(DesugarError { message: "graded param needs a name".into(), span: parts[0].span });
        }
        let name = match &parts[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "parameter name must be a symbol".into(), span: parts[1].span }),
        };
        // Parse optional type annotation: : type
        let ty = if parts.len() >= 3 {
            // Check for : separator
            if let Expr::Keyword(kw) = &parts[2].node {
                if kw.as_str() == ":" && parts.len() >= 4 {
                    Some(self.desugar_type_with_params(&parts[3], &[])?)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        params.push(Param { name, ty, grade, mode: Mode::In });
        Ok(())
    }

    /// §24.1:注册宏(不生成 def;调用点在 desugar_expr 展开)
    fn desugar_defmacro_form(&self, items: &[SExpr], _span: Span) -> Result<Option<TopLevel>, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError { message: "defmacro requires name and params".into(), span: _span });
        }
        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "defmacro name must be a symbol".into(), span: items[1].span }),
        };
        let params = match &items[2].node {
            Expr::Vec(vs) => vs.iter().filter_map(|p| match &p.node {
                Expr::Sym(s) => Some(s.clone()),
                _ => None,
            }).collect(),
            _ => Vec::new(),
        };
        let template: Vec<SExpr> = items[3..].to_vec();
        self.macros.borrow_mut().insert(name, (params, template));
        Ok(Some(TopLevel::Ignored))
    }

    /// 展开宏调用:参数绑定到模板,递归替换后 desugar
    fn expand_macro(&self, template: &[SExpr], params: &[Symbol], args: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let mut bindings = std::collections::HashMap::new();
        for (p, a) in params.iter().zip(args) {
            bindings.insert(p.clone(), a.clone());
        }
        let mut expanded = Vec::new();
        let mut renames = std::collections::HashMap::new();
        for t in template {
            expanded.push(substitute_macro_hygienic(t, &bindings, &mut renames, &mut *self.gensym_counter.borrow_mut()));
        }
        let wrapped = if expanded.len() == 1 {
            expanded.pop().unwrap()
        } else {
            let mut items = vec![Spanned::new(Expr::Sym(Symbol::new("do")), span)];
            items.extend(expanded);
            Spanned::new(Expr::List(items), span)
        };
        self.desugar_expr(&wrapped)
    }

    pub fn desugar_expr(&self, expr: &SExpr) -> Result<CoreExpr, DesugarError> {
        match &expr.node {
            Expr::Nil => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::Unit),
                expr.span,
            )),
            Expr::Bool(b) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::Bool(*b)),
                expr.span,
            )),
            Expr::Int(n) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::I64(*n)),
                expr.span,
            )),
            Expr::Float(f) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::F64(*f)),
                expr.span,
            )),
            Expr::Str(s) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::String(s.clone())),
                expr.span,
            )),
            Expr::Char(c) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::Char(*c)),
                expr.span,
            )),
            Expr::Sym(name) => {
                if name.as_str() == "Unit" {
                    // 值上下文的 unit 字面量(如 handler 里的 (k Unit v))
                    Ok(CoreExpr::new(CoreExprNode::Lit(Literal::Unit), expr.span))
                } else if name.as_str() == "i0" {
                    Ok(CoreExpr::new(CoreExprNode::IntervalEndpoint(false), expr.span))
                } else if name.as_str() == "i1" {
                    Ok(CoreExpr::new(CoreExprNode::IntervalEndpoint(true), expr.span))
                } else if name.as_str().starts_with('?') {
                    let hole_name = Symbol::new(&name.as_str()[1..]);
                    Ok(CoreExpr::new(
                        CoreExprNode::Hole(Some(hole_name)),
                        expr.span,
                    ))
                } else {
                    Ok(CoreExpr::new(
                        CoreExprNode::Var(name.clone()),
                        expr.span,
                    ))
                }
            }
            Expr::Keyword(kw) => {
                // Keywords are self-evaluating symbols
                Ok(CoreExpr::new(
                    CoreExprNode::Lit(Literal::String(format!(":{}", kw))),
                    expr.span,
                ))
            }
            Expr::List(items) => self.desugar_list(items, expr.span),
            Expr::Vec(items) => {
                // Vector literal - desugar to Vec constructor
                let mut args = Vec::new();
                for item in items {
                    args.push(self.desugar_expr(item)?);
                }
                Ok(CoreExpr::new(
                    CoreExprNode::Data(Symbol::new("Vec"), args),
                    expr.span,
                ))
            }
            Expr::ConsPattern(items, tail) => {
                let mut args: Vec<CoreExpr> = items.iter().map(|i| self.desugar_expr(i)).collect::<Result<_, _>>()?;
                args.push(self.desugar_expr(tail)?);
                Ok(CoreExpr::new(
                    CoreExprNode::Data(Symbol::new("Cons"), args),
                    expr.span,
                ))
            }
            Expr::Map(pairs) => {
                // Map literal - desugar to Map constructor
                let mut args = Vec::new();
                for (k, v) in pairs {
                    let pair = CoreExpr::new(
                        CoreExprNode::Data(
                            Symbol::new("Pair"),
                            vec![self.desugar_expr(k)?, self.desugar_expr(v)?],
                        ),
                        k.span.merge(v.span),
                    );
                    args.push(pair);
                }
                Ok(CoreExpr::new(
                    CoreExprNode::Data(Symbol::new("Map"), args),
                    expr.span,
                ))
            }
            Expr::Set(items) => {
                // Set literal - desugar to Set constructor
                let mut args = Vec::new();
                for item in items {
                    args.push(self.desugar_expr(item)?);
                }
                Ok(CoreExpr::new(
                    CoreExprNode::Data(Symbol::new("Set"), args),
                    expr.span,
                ))
            }
            Expr::Quote(inner) => {
                // §24.1 quote:'(a b) → 构造列表数据(符号 → 字符串),不 unquote
                self.desugar_quote_template(inner, expr.span)
            }
            Expr::SyntaxQuote(inner) => {
                // §24.1 syntax-quote:`(a ~x ~@xs) → 列表构造;~ 求值插入,~@ 拼接
                self.desugar_quote_template(inner, expr.span)
            },
            Expr::Unquote(inner) => {
                // unquote ~x(在语法引号内由模板处理;独立出现时求值)
                self.desugar_expr(inner)
            },
            Expr::UnquoteSplice(inner) => {
                // unquote-splice ~@items(在语法引号内由模板处理;独立出现时求值)
                self.desugar_expr(inner)
            },
        }
    }

    fn desugar_list(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.is_empty() {
            return Ok(CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span));
        }

        match &items[0].node {
            Expr::Keyword(kw) => {
                match kw.as_str() {
                    "->" => self.desugar_thread_first(items, span),
                    "->>" => self.desugar_thread_last(items, span),
                    // §7.2 记录字段访问 (:field obj)
                    _ => {
                        if items.len() == 2 {
                            let obj = self.desugar_expr(&items[1])?;
                            Ok(CoreExpr::new(CoreExprNode::FieldGet(kw.clone(), Box::new(obj)), span))
                        } else {
                            Err(DesugarError { message: format!(":{} 字段访问需要 1 个参数", kw), span })
                        }
                    }
                }
            }
            Expr::Sym(name) => {
                // §24.1 宏展开优先于关键字与函数调用
                if let Some((params, template)) = self.macros.borrow().get(name).cloned() {
                    return self.expand_macro(&template, &params, &items[1..], span);
                }
                match name.as_str() {
                    "fn" => self.desugar_lambda(items, span),
                    "let" => self.desugar_let(items, span),
                    "if" => self.desugar_if(items, span),
                    "cond" => self.desugar_cond(items, span),
                    "if-let" => self.desugar_if_let(items, span),
                    "when-let" => self.desugar_when_let(items, span),
                    "and" => self.desugar_and(items, span),
                    "or" => self.desugar_or(items, span),
                    "->>" => self.desugar_thread_last(items, span),
                    "as->" => self.desugar_as_thread(items, span),
                    "some->" => self.desugar_some_thread(items, span),
                    "match" => self.desugar_match(items, span),
                    "handle" => self.desugar_handle(items, span),
                    "perform" => self.desugar_perform(items, span),
                    "do" => self.desugar_do(items, span),
                    // §5.9 类型标注 (ann expr Type):运行时无操作,类型检查时校验
                    "ann" => {
                        if items.len() < 3 {
                            return Err(DesugarError { message: "ann requires expr and type".into(), span });
                        }
                        let ty = self.desugar_type_with_params(&items[2], &[])?;
                        let inner = self.desugar_expr(&items[1])?;
                        Ok(CoreExpr::new(CoreExprNode::Ann(Box::new(ty), Box::new(inner)), span))
                    }
                    // §24.1 quote/syntax-quote 函数形式(等价于 'x / `x)
                    "quote" => {
                        if items.len() < 2 {
                            return Err(DesugarError { message: "quote requires an expression".into(), span });
                        }
                        self.desugar_quote_template(&items[1], span)
                    }
                    // §9 类型反射:(reflect-type name) — 运行时查询定义签名(类型/参数)
                    "reflect-type" => {
                        if items.len() < 2 {
                            return Err(DesugarError { message: "reflect-type requires a symbol".into(), span });
                        }
                        match &items[1].node {
                            Expr::Sym(s) => Ok(CoreExpr::new(CoreExprNode::MetaQuery(s.clone()), span)),
                            _ => Err(DesugarError { message: "reflect-type requires a symbol".into(), span: items[1].span }),
                        }
                    }
                    "syntax-quote" => {
                        if items.len() < 2 {
                            return Err(DesugarError { message: "syntax-quote requires an expression".into(), span });
                        }
                        self.desugar_quote_template(&items[1], span)
                    }
                    // Logic
                    "fresh"     => self.desugar_new_fresh(items, span),
                    "=="        => self.desugar_binary_wrap(|a, b| CoreExprNode::Unify(a, b), items, span),
                    "unify"     => self.desugar_binary_wrap(|a, b| CoreExprNode::Unify(a, b), items, span),
                    "search"    => self.desugar_search(items, span),
                    "commit"    => self.desugar_unary_wrap(|e| CoreExprNode::Commit(e), items, span),
                    "abduce"    => self.desugar_abduce(items, span),
                    // Constraint
                    "constrain" => self.desugar_unary_wrap(|e| CoreExprNode::Constrain(e), items, span),
                    "label"     => self.desugar_binary_wrap(|a, b| CoreExprNode::Label(a, b), items, span),
                    "all-diff"  => self.desugar_list_wrap(|xs| CoreExprNode::AllDifferent(xs), items, span),
                    "domain"    => self.desugar_ternary(|a, b, c| CoreExprNode::Domain(a, b, c), items, span),
                    // Process
                    "spawn"     => self.desugar_spawn(items, span),
                    "join"      => self.desugar_unary_wrap(|e| CoreExprNode::Join(e), items, span),
                    "chan"      => self.desugar_v0(CoreExprNode::ChannelNew, items, span),
                    "send!"     => self.desugar_binary_wrap(|a, b| CoreExprNode::ChannelSend(a, b), items, span),
                    "recv!"     => self.desugar_unary_wrap(|e| CoreExprNode::ChannelRecv(e), items, span),
                    "async-send" => self.desugar_binary_wrap(|a, b| CoreExprNode::AsyncSend(a, b), items, span),
                    "async-recv" => self.desugar_unary_wrap(|e| CoreExprNode::AsyncRecv(e), items, span),
                    "enter"     => self.desugar_binary_wrap(|a, b| CoreExprNode::AmbientEnter(a, b), items, span),
                    "exit"      => self.desugar_binary_wrap(|a, b| CoreExprNode::AmbientExit(a, b), items, span),
                    "open"      => self.desugar_binary_wrap(|a, b| CoreExprNode::AmbientOpen(a, b), items, span),
                    "ambient-new" => self.desugar_ambient_new(items, span),
                    "rho-quote" => self.desugar_unary_wrap(|e| CoreExprNode::RhoQuote(e), items, span),
                    "rho-drop"  => self.desugar_unary_wrap(|e| CoreExprNode::RhoDrop(e), items, span),
                    "rho-lift"  => self.desugar_binary_wrap(|a, b| CoreExprNode::RhoLift(a, b), items, span),
                    // Applied π-calculus
                    "encrypt"   => self.desugar_binary_wrap(|a, b| CoreExprNode::CryptoEncrypt(a, b), items, span),
                    "decrypt"   => self.desugar_binary_wrap(|a, b| CoreExprNode::CryptoDecrypt(a, b), items, span),
                    "sign"      => self.desugar_binary_wrap(|a, b| CoreExprNode::CryptoSign(a, b), items, span),
                    "verify!"   => self.desugar_binary_wrap(|a, b| CoreExprNode::CryptoVerify(a, b), items, span),
                    "hash"      => self.desugar_unary_wrap(|e| CoreExprNode::CryptoHash(e), items, span),
                    // spi-calculus
                    "secret!"   => self.desugar_unary_wrap(|e| CoreExprNode::SpiSecret(e), items, span),
                    "commit!"   => self.desugar_binary_wrap(|a, b| CoreExprNode::SpiCommit(a, b), items, span),
                    "check!"    => self.desugar_binary_wrap(|a, b| CoreExprNode::SpiCheck(a, b), items, span),
                    // SKI
                    "S"         => self.desugar_v0(CoreExprNode::SkiS, items, span),
                    "K"         => self.desugar_v0(CoreExprNode::SkiK, items, span),
                    "I"         => self.desugar_v0(CoreExprNode::SkiI, items, span),
                    "ski-app"   => self.desugar_binary_wrap(|a, b| CoreExprNode::SkiApp(a, b), items, span),
                    "ski-reduce" => self.desugar_unary_wrap(|e| CoreExprNode::SkiReduce(e), items, span),
                    // ς-calculus
                    "invoke"    => self.desugar_binary_wrap(|a, b| CoreExprNode::SigmaInvoke(a, b), items, span),
                    "update!"   => self.desugar_binary_wrap(|a, b| CoreExprNode::SigmaUpdate(a, b), items, span),
                    // HoTT
                    "flat"      => self.desugar_hott_unary(CoreExprNode::FlatMod, items, span),
                    "sharp"     => self.desugar_hott_unary(CoreExprNode::SharpMod, items, span),
                    "shape"     => self.desugar_hott_unary(CoreExprNode::ShapeMod, items, span),
                    "crisp"     => self.desugar_hott_unary(CoreExprNode::CrispMod, items, span),
                    "path-lam"  => self.desugar_path_lam(items, span),
                    "path-apply" => self.desugar_path_apply(items, span),
                    "glue"      => self.desugar_binary_wrap(|a, b| CoreExprNode::Glue(a, b), items, span),
                    "unglue"    => self.desugar_unary_wrap(|e| CoreExprNode::Unglue(e), items, span),
                    // FRP
                    "signal"    => self.desugar_unary_wrap(|e| CoreExprNode::SignalNew(e), items, span),
                    "signal-map" => self.desugar_binary_wrap(|a, b| CoreExprNode::SignalMap(a, b), items, span),
                    "signal-filter" => self.desugar_binary_wrap(|a, b| CoreExprNode::SignalFilter(a, b), items, span),
                    "signal-fold" => self.desugar_ternary(|a, b, c| CoreExprNode::SignalFold(a, b, c), items, span),
                    "delay"     => self.desugar_unary_wrap(|e| CoreExprNode::Delay(e), items, span),
                    "advance"   => self.desugar_unary_wrap(|e| CoreExprNode::Advance(e), items, span),
                    "stable"    => self.desugar_unary_wrap(|e| CoreExprNode::Stable(e), items, span),
                    "unbox!"    => self.desugar_unary_wrap(|e| CoreExprNode::Unbox(e), items, span),
                    // Metaprogramming
                    "comptime"  => self.desugar_unary_wrap(|e| CoreExprNode::Comptime(e), items, span),
                    // Memory
                    "region-alloc" => self.desugar_binary_wrap(|a, b| CoreExprNode::RegionAlloc(a, b), items, span),
                    "region-free" => self.desugar_unary_wrap(|e| CoreExprNode::RegionFree(e), items, span),
                    "ptr-read"  => self.desugar_unary_wrap(|e| CoreExprNode::PtrRead(e), items, span),
                    "ptr-write" => self.desugar_binary_wrap(|a, b| CoreExprNode::PtrWrite(a, b), items, span),
                    // Session(send/recv 走 §27 通道内置;session 协议操作用 send!/recv!)
                    "send"      => self.desugar_session(SessionOp::Send, items, span),
                    "recv"      => self.desugar_session(SessionOp::Recv, items, span),
                    "close"     => self.desugar_session(SessionOp::Close, items, span),
                    _ => self.desugar_app(items, span),
                }
            }
            _ => self.desugar_app(items, span),
        }
    }

    fn desugar_lambda(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "fn requires params and body".into(),
                span,
            });
        }

        let params = self.desugar_params(&items[1])?;
        // 收集全部 body 表达式(多表达式用 Do 包装;原实现只保留第一个)
        let mut body_exprs = Vec::new();
        for item in &items[2..] {
            body_exprs.push(self.desugar_expr(item)?);
        }
        let body = if body_exprs.len() == 1 {
            body_exprs.pop().unwrap()
        } else {
            CoreExpr::new(CoreExprNode::Do(body_exprs), span)
        };

        Ok(CoreExpr::new(
            CoreExprNode::Lam(Lambda {
                params,
                body: Box::new(body),
                ret_type: None,
            }),
            span,
        ))
    }

    fn desugar_let(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "let requires bindings and body".into(),
                span,
            });
        }

        let bindings = match &items[1].node {
            Expr::Vec(binds) => binds,
            _ => return Err(DesugarError {
                message: "let bindings must be a vector".into(),
                span: items[1].span,
            }),
        };

        if bindings.len() % 2 != 0 {
            return Err(DesugarError {
                message: "let bindings must have even number of elements".into(),
                span: items[1].span,
            });
        }

        // 收集全部 body 表达式(多表达式用 Do 包装;原实现只保留第一个)
        let mut body_exprs = Vec::new();
        for item in &items[2..] {
            body_exprs.push(self.desugar_expr(item)?);
        }
        let body = if body_exprs.len() == 1 {
            body_exprs.pop().unwrap()
        } else {
            CoreExpr::new(CoreExprNode::Do(body_exprs), span)
        };

        // Build nested lets from right to left
        let mut result = body;
        for i in (0..bindings.len()).step_by(2).rev() {
            let name = match &bindings[i].node {
                Expr::Sym(s) => s.clone(),
                _ => return Err(DesugarError {
                    message: "let binding name must be a symbol".into(),
                    span: bindings[i].span,
                }),
            };
            let value = self.desugar_expr(&bindings[i + 1])?;

            result = CoreExpr::new(
                CoreExprNode::Let(name, None, Box::new(value), Box::new(result)),
                span,
            );
        }

        Ok(result)
    }

    fn desugar_if(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 4 {
            return Err(DesugarError {
                message: "if requires condition, then, and else".into(),
                span,
            });
        }

        let cond = self.desugar_expr(&items[1])?;
        let then = self.desugar_expr(&items[2])?;
        let else_ = self.desugar_expr(&items[3])?;

        Ok(CoreExpr::new(
            CoreExprNode::If(Box::new(cond), Box::new(then), Box::new(else_)),
            span,
        ))
    }

    fn desugar_cond(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError { message: "cond requires at least one clause".into(), span });
        }
        let clauses = &items[1..];
        // 奇数个 clause 时最后一项是默认分支;偶数个时 [test body] 成对,默认 Unit
        let (pairs, default) = if clauses.len() % 2 == 1 {
            (&clauses[..clauses.len() - 1], Some(&clauses[clauses.len() - 1]))
        } else {
            (clauses, None)
        };
        let mut result = match default {
            Some(d) => self.desugar_expr(d)?,
            None => CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span),
        };
        // 从后往前构建嵌套 If(原实现会把最后一项重复求值)
        let mut i = pairs.len();
        while i >= 2 {
            let body = &pairs[i - 1];
            let test = &pairs[i - 2];
            i -= 2;
            if let Expr::Keyword(kw) = &test.node {
                if kw.as_str() == "else" {
                    result = self.desugar_expr(body)?;
                    continue;
                }
            }
            result = CoreExpr::new(
                CoreExprNode::If(
                    Box::new(self.desugar_expr(test)?),
                    Box::new(self.desugar_expr(body)?),
                    Box::new(result),
                ),
                span,
            );
        }
        Ok(result)
    }

    fn desugar_if_let(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 4 {
            return Err(DesugarError { message: "if-let requires [var expr] then else".into(), span });
        }
        let (var, val_expr) = self.desugar_let_binding(&items[1])?;
        let then_body = self.desugar_expr(&items[2])?;
        let else_body = if items.len() > 3 { self.desugar_expr(&items[3])? }
            else { CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span) };
        let var_sym = match &var.node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "if-let requires a variable name".into(), span: var.span }),
        };
        let test = CoreExpr::new(CoreExprNode::Var(var_sym.clone()), span);
        Ok(CoreExpr::new(
            CoreExprNode::Let(var_sym, None, Box::new(val_expr),
                Box::new(CoreExpr::new(
                    CoreExprNode::If(Box::new(test), Box::new(then_body), Box::new(else_body)),
                    span,
                )),
            ),
            span,
        ))
    }

    fn desugar_when_let(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError { message: "when-let requires [var expr] body".into(), span });
        }
        let (var, val_expr) = self.desugar_let_binding(&items[1])?;
        let body = self.desugar_expr(&items[2])?;
        let else_ = CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span);
        let var_sym = match &var.node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "when-let requires a variable name".into(), span: var.span }),
        };
        let test = CoreExpr::new(CoreExprNode::Var(var_sym.clone()), span);
        Ok(CoreExpr::new(
            CoreExprNode::Let(var_sym, None, Box::new(val_expr),
                Box::new(CoreExpr::new(
                    CoreExprNode::If(Box::new(test), Box::new(body), Box::new(else_)),
                    span,
                )),
            ),
            span,
        ))
    }

    fn desugar_let_binding(&self, expr: &SExpr) -> Result<(SExpr, CoreExpr), DesugarError> {
        match &expr.node {
            Expr::Vec(items) if items.len() == 2 => {
                Ok((items[0].clone(), self.desugar_expr(&items[1])?))
            }
            _ => Err(DesugarError { message: "expected [var expr] binding".into(), span: expr.span }),
        }
    }

    fn desugar_multi_arity_defn(&self, name: Symbol, clauses: &[SExpr], span: Span) -> Result<CoreDef, DesugarError> {
        let mut arms = Vec::new();
        let mut all_params = Vec::new();
        for clause in clauses {
            if let Expr::List(inner) = &clause.node {
                if inner.len() >= 2 {
                    if let Expr::Vec(param_items) = &inner[0].node {
                        let mut pat_vars = Vec::new();
                        for p in param_items {
                            if let Expr::Sym(s) = &p.node {
                                pat_vars.push(Pattern::Var(s.clone()));
                                if !all_params.contains(s) { all_params.push(s.clone()); }
                            } else {
                                pat_vars.push(Pattern::Wildcard);
                            }
                        }
                        let body = self.desugar_expr(&inner[1])?;
                        arms.push(MatchArm {
                            pattern: if pat_vars.len() == 1 { pat_vars[0].clone() } else { Pattern::Tuple(pat_vars) },
                            guard: None,
                            body: Box::new(body),
                        });
                    }
                }
            }
        }
        if arms.is_empty() {
            return Err(DesugarError { message: "defn multi-arity requires at least one valid clause".into(), span });
        }
        let params: Vec<Param> = all_params.iter().map(|s| Param {
            name: s.clone(), ty: None, grade: Grade::Omega, mode: Mode::In,
        }).collect();
        let match_body = CoreExpr::new(CoreExprNode::Match(
            Box::new(CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span)),
            arms,
        ), span);
        let lambda = CoreExprNode::Lam(Lambda { params, body: Box::new(match_body), ret_type: None });
        Ok(CoreDef {
            name,
            ty: None, effects: EffectRow::Pure, grade: Grade::Omega, mode: Mode::In,
            determinism: Determinism::Det,
            mode_sigs: vec![], body: CoreExpr::new(lambda, span),
            requires: None, ensures: None, span,
        })
    }

    fn desugar_and(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let exprs: Vec<_> = items[1..].iter().map(|e| self.desugar_expr(e)).collect::<Result<_, _>>()?;
        let falsy = CoreExpr::new(CoreExprNode::Lit(Literal::Bool(false)), span);
        let result = exprs.into_iter().rfold(falsy, |acc, e| {
            CoreExpr::new(CoreExprNode::If(Box::new(e.clone()), Box::new(acc), Box::new(e)), span)
        });
        Ok(result)
    }

    fn desugar_thread_first(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 { return Err(DesugarError { message: "-> requires at least a value".into(), span }); }
        let mut result = self.desugar_expr(&items[1])?;
        for item in &items[2..] {
            result = match &item.node {
                Expr::Sym(f) => CoreExpr::new(
                    CoreExprNode::App(Box::new(CoreExpr::new(CoreExprNode::Var(f.clone()), item.span)), Box::new(result)),
                    span,
                ),
                Expr::List(parts) if !parts.is_empty() => {
                    let mut args = vec![result];
                    for p in &parts[1..] { args.push(self.desugar_expr(p)?); }
                    let func = self.desugar_expr(&parts[0])?;
                    let mut app = func;
                    for arg in args {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(arg)), span);
                    }
                    app
                }
                _ => continue,
            };
        }
        Ok(result)
    }

    fn desugar_thread_last(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 { return Err(DesugarError { message: "->> requires at least a value".into(), span }); }
        let mut result = self.desugar_expr(&items[1])?;
        for item in &items[2..] {
            result = match &item.node {
                Expr::Sym(f) => CoreExpr::new(
                    CoreExprNode::App(Box::new(CoreExpr::new(CoreExprNode::Var(f.clone()), item.span)), Box::new(result)),
                    span,
                ),
                Expr::List(parts) if !parts.is_empty() => {
                    let mut args = Vec::new();
                    for p in &parts[1..] { args.push(self.desugar_expr(p)?); }
                    args.push(result);
                    let func = self.desugar_expr(&parts[0])?;
                    let mut app = func;
                    for arg in args {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(arg)), span);
                    }
                    app
                }
                _ => continue,
            };
        }
        Ok(result)
    }

    fn desugar_as_thread(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 { return Err(DesugarError { message: "as-> requires expr, name, and forms".into(), span }); }
        let init = self.desugar_expr(&items[1])?;
        let name = match &items[2].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "as-> requires a binding name".into(), span: items[2].span }),
        };
        let mut result = init;
        for item in &items[3..] {
            let step = match &item.node {
                Expr::List(parts) if !parts.is_empty() => {
                    let mut args: Vec<CoreExpr> = vec![CoreExpr::new(CoreExprNode::Var(name.clone()), span)];
                    for p in &parts[1..] { args.push(self.desugar_expr(p)?); }
                    let func = self.desugar_expr(&parts[0])?;
                    args.insert(0, func);
                    let mut app = args.remove(0);
                    for arg in args {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(arg)), span);
                    }
                    app
                }
                _ => {
                    let bind = CoreExpr::new(CoreExprNode::Let(name.clone(), None, Box::new(result), Box::new(self.desugar_expr(item)?)), span);
                    return Ok(bind);
                }
            };
            result = CoreExpr::new(CoreExprNode::Let(name.clone(), None, Box::new(result), Box::new(step)), span);
        }
        Ok(result)
    }

    fn desugar_some_thread(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 { return Err(DesugarError { message: "some-> requires at least a value".into(), span }); }
        // §5.8 some->:任一中间结果为 nil(Unit)时短路,返回 nil
        let nil_check = CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span);
        let mut result = self.desugar_expr(&items[1])?;
        for item in &items[2..] {
            // 构建本步调用:符号 (f) → (f result);列表 (f a b) → (f result a b)
            let step = match &item.node {
                Expr::Sym(f) => CoreExpr::new(
                    CoreExprNode::App(Box::new(CoreExpr::new(CoreExprNode::Var(f.clone()), item.span)), Box::new(result.clone())),
                    span,
                ),
                Expr::List(parts) if !parts.is_empty() => {
                    let mut args = vec![result.clone()];
                    for p in &parts[1..] { args.push(self.desugar_expr(p)?); }
                    let func = self.desugar_expr(&parts[0])?;
                    let mut app = func;
                    for arg in args {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(arg)), span);
                    }
                    app
                }
                _ => continue,
            };
            // nil 短路:result 为 nil 时直接返回 nil,否则执行本步(原实现 List 分支缺此检查)
            result = CoreExpr::new(
                CoreExprNode::If(
                    Box::new(CoreExpr::new(
                        CoreExprNode::App(
                            Box::new(CoreExpr::new(
                                CoreExprNode::App(Box::new(CoreExpr::new(CoreExprNode::Var(Symbol::new("=")), item.span)), Box::new(result)),
                                span,
                            )),
                            Box::new(nil_check.clone()),
                        ),
                        span,
                    )),
                    Box::new(nil_check.clone()),
                    Box::new(step),
                ),
                span,
            );
        }
        Ok(result)
    }

    fn desugar_or(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let exprs: Vec<_> = items[1..].iter().map(|e| self.desugar_expr(e)).collect::<Result<_, _>>()?;
        let falsy = CoreExpr::new(CoreExprNode::Lit(Literal::Bool(false)), span);
        if exprs.is_empty() { return Ok(falsy); }
        let mut result = exprs.last().unwrap().clone();
        for e in exprs.iter().rev().skip(1) {
            result = CoreExpr::new(CoreExprNode::If(Box::new(e.clone()), Box::new(e.clone()), Box::new(result)), span);
        }
        Ok(result)
    }

    fn desugar_match(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "match requires scrutinee and at least one pattern/body pair".into(),
                span,
            });
        }

        let scrutinee = self.desugar_expr(&items[1])?;
        let mut arms = Vec::new();

        let mut i = 2;
        while i < items.len() {
            // §8.2 两种 guard 形式:
            // 1. spec 形式:(when pattern guard)  pattern 包裹在 when 里
            // 2. 现状形式:pattern :when guard
            let mut when_guard: Option<CoreExpr> = None;
            let mut pat_item = &items[i];
            if let Expr::List(w) = &items[i].node {
                if w.len() == 3 {
                    if let Expr::Sym(h) = &w[0].node {
                        if h.as_str() == "when" {
                            when_guard = Some(self.desugar_expr(&w[2])?);
                            pat_item = &w[1];
                        }
                    }
                }
            }
            // §8.2 refined 模式 {x : T | pred}:变量模式 + 谓词 guard
            let mut refined_guard: Option<CoreExpr> = None;
            let mut refined_var: Option<Symbol> = None;
            if let Expr::Map(pairs) = &pat_item.node {
                if let Some((name, pred_src)) = Self::refined_pattern_parts(pairs) {
                    refined_var = Some(name.clone());
                    refined_guard = Some(self.desugar_expr(pred_src)?);
                }
            }
            // refined 模式:pattern 为变量绑定整个值;否则正常解析
            let pattern = if let Some(v) = &refined_var {
                Pattern::Var(v.clone())
            } else {
                self.desugar_pattern(pat_item)?
            };
            i += 1;
            if i >= items.len() {
                return Err(DesugarError { message: "match arm missing body after pattern".into(), span });
            }
            let guard = if when_guard.is_none() && refined_guard.is_none() &&
                i + 1 < items.len() &&
                matches!(&items[i].node, Expr::Keyword(k) if k.as_str() == "when") {
                let guard_expr = self.desugar_expr(&items[i + 1])?;
                i += 2;
                if i >= items.len() {
                    return Err(DesugarError { message: "match arm missing body after :when guard".into(), span });
                }
                Some(Box::new(guard_expr))
            } else if let Some(g) = when_guard {
                Some(Box::new(g))
            } else if let Some(g) = refined_guard {
                // refined 模式:绑定变量为整个值(Var 模式已绑定),guard 检查谓词
                let _ = refined_var;
                Some(Box::new(g))
            } else { None };
            let body = self.desugar_expr(&items[i])?;
            i += 1;
            arms.push(MatchArm {
                pattern,
                guard,
                body: Box::new(body),
            });
        }

        Ok(CoreExpr::new(
            CoreExprNode::Match(Box::new(scrutinee), arms),
            span,
        ))
    }

    /// §8.2 refined 模式 {x : T | pred}:提取 (变量名, 谓词表达式)
    fn refined_pattern_parts(pairs: &[(SExpr, SExpr)]) -> Option<(&Symbol, &SExpr)> {
        for (key, val) in pairs {
            if let Expr::Sym(name) = &key.node {
                if let Expr::List(val_items) = &val.node {
                    // 找 | 分隔符,谓词在其后
                    for j in 0..val_items.len() {
                        let is_pipe = matches!(&val_items[j].node, Expr::Sym(s) if s.as_str() == "|")
                            || matches!(&val_items[j].node, Expr::Keyword(k) if k.as_str() == "|");
                        if is_pipe {
                            if j + 1 < val_items.len() {
                                return Some((name, &val_items[j + 1]));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn desugar_pattern(&self, expr: &SExpr) -> Result<Pattern, DesugarError> {
        match &expr.node {
            Expr::Sym(name) if name.as_str() == "_" => Ok(Pattern::Wildcard),
            Expr::Sym(name) => Ok(Pattern::Var(name.clone())),
            Expr::Int(n) => Ok(Pattern::Lit(Literal::I64(*n))),
            Expr::Bool(b) => Ok(Pattern::Lit(Literal::Bool(*b))),
            Expr::Str(s) => Ok(Pattern::Lit(Literal::String(s.clone()))),
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(con_name) = &items[0].node {
                    if con_name.as_str() == "or" {
                        // (or pat1 pat2 ...)(§8.2):全部子模式,匹配任一成功
                        let mut subpats = Vec::new();
                        for item in &items[1..] {
                            subpats.push(self.desugar_pattern(item)?);
                        }
                        if subpats.is_empty() {
                            return Err(DesugarError { message: "or-pattern needs at least one subpattern".into(), span: expr.span });
                        }
                        return Ok(Pattern::Or(subpats));
                    }
                    let mut subpats = Vec::new();
                    for item in &items[1..] {
                        subpats.push(self.desugar_pattern(item)?);
                    }
                    Ok(Pattern::Con(con_name.clone(), subpats))
                } else {
                    Err(DesugarError { message: "pattern constructor must be a symbol".into(), span: items[0].span })
                }
            }
            Expr::ConsPattern(items, tail) => {
                let tail_pat = self.desugar_pattern(tail)?;
                let mut result = tail_pat;
                for item in items.iter().rev() {
                    let head = self.desugar_pattern(item)?;
                    result = Pattern::Con(
                        Symbol::new("Cons"),
                        vec![head, result],
                    );
                }
                Ok(result)
            }
            Expr::Vec(items) => {
                // 列表模式(§21.2):[a b c] → Cons 链;[] → Nil
                let mut result = Pattern::Con(Symbol::new("Nil"), vec![]);
                for item in items.iter().rev() {
                    let head = self.desugar_pattern(item)?;
                    result = Pattern::Con(Symbol::new("Cons"), vec![head, result]);
                }
                Ok(result)
            }
            _ => Err(DesugarError { message: "invalid pattern".into(), span: expr.span }),
        }
    }

    /// §24.1 语法引号/quote 模板:构造列表数据
    /// - `~x` 求值插入;`~@xs` 求值拼接(concat)
    /// - 符号原子 → 字符串(宏模板符号表示);数字/字符串/布尔原样
    fn desugar_quote_template(&self, expr: &SExpr, span: Span) -> Result<CoreExpr, DesugarError> {
        match &expr.node {
            Expr::Unquote(inner) => self.desugar_expr(inner),
            Expr::UnquoteSplice(inner) => self.desugar_expr(inner),
            Expr::List(items) => {
                // 收集普通元素为 (list ...) 片段;~@ 拼接处切开,整体用 concat 连接
                let mut segments: Vec<CoreExpr> = Vec::new();
                let mut pending: Vec<CoreExpr> = Vec::new();
                for item in items {
                    if let Expr::UnquoteSplice(inner) = &item.node {
                        if !pending.is_empty() {
                            let mut app = CoreExpr::new(CoreExprNode::Var(Symbol::new("list")), span);
                            for p in pending.drain(..) {
                                app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(p)), span);
                            }
                            segments.push(app);
                        }
                        segments.push(self.desugar_expr(inner)?);
                    } else {
                        pending.push(self.desugar_quote_template(item, span)?);
                    }
                }
                if !pending.is_empty() {
                    let mut app = CoreExpr::new(CoreExprNode::Var(Symbol::new("list")), span);
                    for p in pending {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(p)), span);
                    }
                    segments.push(app);
                }
                if segments.len() == 1 {
                    Ok(segments.pop().unwrap())
                } else {
                    // (concat seg1 seg2 ...)
                    let mut app = CoreExpr::new(CoreExprNode::Var(Symbol::new("concat")), span);
                    for p in segments {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(p)), span);
                    }
                    Ok(app)
                }
            }
            Expr::Sym(name) => {
                // 符号原子 → 字符串表示
                Ok(CoreExpr::new(CoreExprNode::Lit(Literal::String(name.as_str().into())), span))
            }
            Expr::Int(n) => Ok(CoreExpr::new(CoreExprNode::Lit(Literal::I64(*n)), span)),
            Expr::Bool(b) => Ok(CoreExpr::new(CoreExprNode::Lit(Literal::Bool(*b)), span)),
            Expr::Str(s) => Ok(CoreExpr::new(CoreExprNode::Lit(Literal::String(s.clone())), span)),
            Expr::Nil => Ok(CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span)),
            Expr::ConsPattern(items, tail) => {
                // [a b . t] 引号:构造 Cons 链数据
                let mut parts: Vec<CoreExpr> = Vec::new();
                for item in items {
                    parts.push(self.desugar_quote_template(item, span)?);
                }
                let tail_e = self.desugar_quote_template(tail, span)?;
                let mut app = CoreExpr::new(CoreExprNode::Var(Symbol::new("list")), span);
                for p in parts {
                    app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(p)), span);
                }
                // (append parts (list tail))
                Ok(CoreExpr::new(
                    CoreExprNode::App(
                        Box::new(CoreExpr::new(
                            CoreExprNode::App(Box::new(CoreExpr::new(CoreExprNode::Var(Symbol::new("append")), span)), Box::new(app)),
                            span,
                        )),
                        Box::new(CoreExpr::new(
                            CoreExprNode::App(
                                Box::new(CoreExpr::new(CoreExprNode::Var(Symbol::new("list")), span)),
                                Box::new(tail_e),
                            ),
                            span,
                        )),
                    ),
                    span,
                ))
            }
            _ => self.desugar_expr(expr),
        }
    }

    /// §27 ambients:(ambient-new name) 注册 ambient
    fn desugar_ambient_new(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError { message: "ambient-new requires a name".into(), span });
        }
        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "ambient-new name must be a symbol".into(), span: items[1].span }),
        };
        Ok(CoreExpr::new(CoreExprNode::AmbientNew(name), span))
    }

    fn desugar_do(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Ok(CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span));
        }
        let exprs: Vec<CoreExpr> = items[1..].iter()
            .map(|item| self.desugar_expr(item))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CoreExpr::new(CoreExprNode::Do(exprs), span))
    }

    fn desugar_spawn(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError { message: "spawn requires a body expression".into(), span });
        }
        let body = self.desugar_expr(&items[1])?;
        // Generate a handle name for structured concurrency
        let handle_name = Symbol::new("_join_handle");
        Ok(CoreExpr::new(CoreExprNode::Spawn(Box::new(body), Box::new(CoreExpr::new(CoreExprNode::Var(handle_name), span))), span))
    }

    // ── Generic desugar helpers for N-ary primitives ──

    fn desugar_unary_wrap(&self, wrap: fn(Box<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 2 { return Err(DesugarError { message: "expects 1 argument".into(), span }); }
        Ok(CoreExpr::new(wrap(Box::new(self.desugar_expr(&items[1])?)), span))
    }

    fn desugar_binary_wrap(&self, wrap: fn(Box<CoreExpr>, Box<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 3 { return Err(DesugarError { message: "expects 2 arguments".into(), span }); }
        Ok(CoreExpr::new(wrap(Box::new(self.desugar_expr(&items[1])?), Box::new(self.desugar_expr(&items[2])?)), span))
    }

    fn desugar_ternary(&self, wrap: fn(Box<CoreExpr>, Box<CoreExpr>, Box<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 4 { return Err(DesugarError { message: "expects 3 arguments".into(), span }); }
        Ok(CoreExpr::new(wrap(Box::new(self.desugar_expr(&items[1])?), Box::new(self.desugar_expr(&items[2])?), Box::new(self.desugar_expr(&items[3])?)), span))
    }

    fn desugar_list_wrap(&self, wrap: fn(Vec<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let exprs: Vec<CoreExpr> = items[1..].iter().map(|i| self.desugar_expr(i)).collect::<Result<Vec<_>, _>>()?;
        Ok(CoreExpr::new(wrap(exprs), span))
    }

    fn desugar_v0(&self, wrap: CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 1 { return Err(DesugarError { message: "expects no arguments".into(), span }); }
        Ok(CoreExpr::new(wrap, span))
    }

    /// §21.3:(search) 零参提交搜索 或 (search expr...) 搜索子目标
    fn desugar_search(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let inner = if items.len() >= 2 {
            let mut exprs = Vec::new();
            for item in &items[1..] {
                exprs.push(self.desugar_expr(item)?);
            }
            if exprs.len() == 1 {
                exprs.pop().unwrap()
            } else {
                CoreExpr::new(CoreExprNode::Do(exprs), span)
            }
        } else {
            CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span)
        };
        Ok(CoreExpr::new(CoreExprNode::Search(Box::new(inner)), span))
    }

    fn desugar_new_fresh(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError { message: "fresh requires variable name(s)".into(), span });
        }
        // §21.1:(fresh x) 单变量 或 (fresh [x y z] body...) 多变量
        let mut vars = Vec::new();
        match &items[1].node {
            Expr::Sym(s) => vars.push(s.clone()),
            Expr::Vec(vs) => {
                for v in vs {
                    match &v.node {
                        Expr::Sym(s) => vars.push(s.clone()),
                        _ => return Err(DesugarError { message: "fresh variables must be symbols".into(), span: v.span }),
                    }
                }
            }
            _ => return Err(DesugarError { message: "fresh requires a variable name".into(), span: items[1].span }),
        }
        // 仅单变量且无 body:保持原语义 (fresh x) → Fresh(x)
        if vars.len() == 1 && items.len() == 2 {
            return Ok(CoreExpr::new(CoreExprNode::Fresh(vars.pop().unwrap()), span));
        }
        // 多变量或带 body:Fresh(v1); Fresh(v2); ...; body
        let body = if items.len() > 2 {
            let mut exprs = Vec::new();
            for item in &items[2..] {
                exprs.push(self.desugar_expr(item)?);
            }
            if exprs.len() == 1 { exprs.pop().unwrap() }
            else { CoreExpr::new(CoreExprNode::Do(exprs), span) }
        } else {
            CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span)
        };
        let mut exprs: Vec<CoreExpr> = vars.into_iter()
            .map(|v| CoreExpr::new(CoreExprNode::Fresh(v), span))
            .collect();
        exprs.push(body);
        Ok(CoreExpr::new(CoreExprNode::Do(exprs), span))
    }

    fn desugar_abduce(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 { return Err(DesugarError { message: "abduce requires goal and abducibles".into(), span }); }
        let goal = self.desugar_expr(&items[1])?;
        let mut abducibles = Vec::new();
        for item in &items[2..] {
            if let Expr::Sym(s) = &item.node { abducibles.push(s.clone()); }
            else { return Err(DesugarError { message: "abducibles must be symbols".into(), span: item.span }); }
        }
        Ok(CoreExpr::new(CoreExprNode::Abduce(Box::new(goal), abducibles), span))
    }

    fn desugar_hott_unary(&self, wrap: fn(Box<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 2 {
            return Err(DesugarError { message: "flat/sharp expects 1 argument".into(), span });
        }
        Ok(CoreExpr::new(wrap(Box::new(self.desugar_expr(&items[1])?)), span))
    }

    fn desugar_path_lam(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 3 {
            return Err(DesugarError { message: "path-lam requires var and body".into(), span });
        }
        let var = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "path-lam var must be symbol".into(), span }),
        };
        Ok(CoreExpr::new(CoreExprNode::PathLam(var, Box::new(self.desugar_expr(&items[2])?)), span))
    }

    fn desugar_path_apply(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 3 {
            return Err(DesugarError { message: "path-apply requires path and point".into(), span });
        }
        Ok(CoreExpr::new(CoreExprNode::PathApp(
            Box::new(self.desugar_expr(&items[1])?),
            Box::new(self.desugar_expr(&items[2])?),
        ), span))
    }

    fn desugar_session(&self, op: SessionOp, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError { message: "session op requires operand".into(), span });
        }
        Ok(CoreExpr::new(CoreExprNode::Session(op, Box::new(self.desugar_expr(&items[1])?)), span))
    }

    fn desugar_handle(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError { message: "handle requires body and handler".into(), span });
        }
        let body = self.desugar_expr(&items[1])?;

        let mut effect_name = Symbol::new("Unknown");
        let mut type_args = Vec::new();
        let mut clauses = Vec::new();
        let mut i = 2;
        while i < items.len() {
            if let Expr::List(inner) = &items[i].node {
                if inner.is_empty() { i += 1; continue; }
                if let Expr::Sym(first) = &inner[0].node {
                    // Check if this is an operation clause: (op-name [params] [continuation-vars] body)
                    if inner.len() >= 4 {
                        if let (Expr::Sym(_), Expr::Vec(params), Expr::Vec(cont_vars)) =
                            (&inner[0].node, &inner[1].node, &inner[2].node) {
                            let mut op_params = Vec::new();
                            for p in params {
                                if let Expr::Sym(s) = &p.node { op_params.push(s.clone()); }
                                else { op_params.push(Symbol::new("_")); }
                            }
                            let mut cont = Vec::new();
                            for c in cont_vars {
                                if let Expr::Sym(s) = &c.node { cont.push(s.clone()); }
                            }
                            let cont_name = cont.first().cloned().unwrap_or(Symbol::new("k"));
                            let state = cont.get(1).cloned();
                            // 收集 clause 全部 body 表达式(多表达式用 Do 包装;原实现只取第一个)
                            let mut body_exprs = Vec::new();
                            for item in &inner[3..] {
                                body_exprs.push(self.desugar_expr(item)?);
                            }
                            let op_body = if body_exprs.len() == 1 {
                                body_exprs.pop().unwrap()
                            } else {
                                CoreExpr::new(CoreExprNode::Do(body_exprs), span)
                            };
                            clauses.push(HandlerClause {
                                operation: first.clone(),
                                params: op_params,
                                continuation: cont_name,
                                state,
                                body: Box::new(op_body),
                            });
                            i += 1;
                            continue;
                        }
                    }
                    // Effect type specification: (EffectName type_args...)
                    if effect_name.as_str() == "Unknown" {
                        effect_name = first.clone();
                        if inner.len() > 1 {
                            for a in &inner[1..] {
                                type_args.push(self.desugar_type_with_params(a, &[])?);
                            }
                        }
                        i += 1;
                        continue;
                    }
                }
            }
            // Return clause
            if effect_name.as_str() != "Unknown" && clauses.is_empty() {
                let ret_clause = Some(Box::new(self.desugar_expr(&items[i])?));
                let handler = Handler { effect_name, type_args, clauses, return_clause: ret_clause };
                return Ok(CoreExpr::new(CoreExprNode::Handle(Box::new(body), handler), span));
            }
            i += 1;
        }

        let handler = Handler { effect_name, type_args, clauses, return_clause: None };
        Ok(CoreExpr::new(CoreExprNode::Handle(Box::new(body), handler), span))
    }

    fn desugar_perform(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError {
                message: "perform requires operation name".into(),
                span,
            });
        }

        let op_name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError {
                message: "perform operation must be a symbol".into(),
                span: items[1].span,
            }),
        };

        let mut args = Vec::new();
        for item in &items[2..] {
            args.push(self.desugar_expr(item)?);
        }

        Ok(CoreExpr::new(
            CoreExprNode::Perform(op_name, args),
            span,
        ))
    }

    fn desugar_app(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let func = self.desugar_expr(&items[0])?;

        if items.len() == 1 {
            // (f) 零参调用:生成 App(f, Unit) 以触发调用;
            // 解释器对 0 参函数/构造函数在收到 Unit 参数时返回结果
            return Ok(CoreExpr::new(
                CoreExprNode::App(
                    Box::new(func),
                    Box::new(CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span)),
                ),
                span,
            ));
        }

        // Build left-associative application: (f x y z) = ((f x) y) z
        let mut result = func;
        for item in &items[1..] {
            let arg = self.desugar_expr(item)?;
            result = CoreExpr::new(
                CoreExprNode::App(Box::new(result), Box::new(arg)),
                span,
            );
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<SExpr> {
        crate::reader::read(src).unwrap()
    }

    fn determinism_of(src: &str, name: &str) -> Determinism {
        let d = Desugarer::new();
        let prog = d.desugar_program(parse(src)).unwrap();
        prog.defs.iter().find(|def| def.name.as_str() == name).unwrap().determinism.clone()
    }

    #[test]
    fn test_defpred_determinism_annotations() {
        // §13/§14:defpred 的 :det/:nondet/:cc_multi/:cc_nondet 注解写入 CoreDef.determinism
        assert_eq!(determinism_of("(defpred p [x] :det (== x 1))", "p"), Determinism::Det);
        assert_eq!(determinism_of("(defpred q [x] :nondet ([x]))", "q"), Determinism::NonDet);
        assert_eq!(determinism_of("(defpred r [x] :cc_multi ([x]))", "r"), Determinism::CcMulti);
        assert_eq!(determinism_of("(defpred s [x] :cc_nondet ([x]))", "s"), Determinism::CcNonDet);
        // 无注解默认 NonDet
        assert_eq!(determinism_of("(defpred t [x] ([x]))", "t"), Determinism::NonDet);
    }

    #[test]
    fn test_defsession_protocol_parsing() {
        // §20.1:defsession 协议解析为 SessionType(Send/Recv/Choice/Offer/End)
        let d = Desugarer::new();
        let prog = d.desugar_program(parse(
            "(defsession proto (send i64 (recv i64 (end))))\n\
             (defsession calc (choice (add (recv i64 (end))) (sub (recv i64 (end)))))\n\
             (defsession offer-p (offer (a (send i64 (end))) (b (recv i64 (end)))))\n",
        )).unwrap();
        let proto = prog.defs.iter().find(|def| def.name.as_str() == "proto").unwrap();
        assert!(matches!(proto.ty, Some(tisp_core::types::Type::Session(_))));
        let calc = prog.defs.iter().find(|def| def.name.as_str() == "calc").unwrap();
        if let Some(tisp_core::types::Type::Session(st)) = &calc.ty {
            assert!(matches!(**st, tisp_core::types::SessionType::Choice(_)));
        } else {
            panic!("calc 应解析为 Session(Choice)");
        }
        let offer = prog.defs.iter().find(|def| def.name.as_str() == "offer-p").unwrap();
        if let Some(tisp_core::types::Type::Session(st)) = &offer.ty {
            assert!(matches!(**st, tisp_core::types::SessionType::Offer(_)));
        } else {
            panic!("offer-p 应解析为 Session(Offer)");
        }
    }
}

#[cfg(test)]
mod pi_tests {
    use super::*;
    use tisp_core::types::Type;

    #[test]
    fn test_pi_sigma_type_syntax() {
        let d = Desugarer::new();
        // (pi (x : i64) i64) → Type::Pi
        let t = d.desugar_type_with_params(
            &crate::reader::read("(pi (x : i64) i64)").unwrap().pop().unwrap(),
            &[],
        ).unwrap();
        assert!(matches!(t, Type::Pi(..)), "expected Pi, got {:?}", t);
        // (sigma (x : i64) i64) → Type::Sigma
        let t2 = d.desugar_type_with_params(
            &crate::reader::read("(sigma (x : i64) i64)").unwrap().pop().unwrap(),
            &[],
        ).unwrap();
        assert!(matches!(t2, Type::Sigma(..)), "expected Sigma, got {:?}", t2);
    }
}

#[cfg(test)]
mod quote_tests {
    use super::*;
    use tisp_core::core_ast::CoreExprNode;

    fn parse(src: &str) -> Vec<SExpr> {
        crate::reader::read(src).unwrap()
    }

    #[test]
    fn test_syntax_quote_builds_list() {
        // §24.1:`(foo ~x) → (list "foo" x);'(a b) → (list "a" "b")
        let d = Desugarer::new();
        let e1 = d.desugar_expr(&parse("(quote (a b))").pop().unwrap()).unwrap();
        // 应生成 (list "a" "b") 应用:App(App(Var(list), Str(a)), Str(b))
        if let CoreExprNode::App(f1, _) = &e1.node {
            if let CoreExprNode::App(f2, a) = &f1.node {
                assert!(matches!(f2.node, CoreExprNode::Var(ref s) if s.as_str() == "list"));
                assert!(matches!(a.node, CoreExprNode::Lit(Literal::String(ref s)) if s == "a"));
            } else {
                panic!("quote 应生成 list 应用");
            }
        } else {
            panic!("quote 应生成 list 应用");
        }
        // syntax-quote 带 unquote 不报错
        let e2 = d.desugar_expr(&parse("(syntax-quote (foo ~x))").pop().unwrap()).unwrap();
        assert!(matches!(e2.node, CoreExprNode::App(..)));
    }

    #[test]
    fn test_contracts_desugar() {
        // §15.3:defn 的 :requires/:ensures 解析进 CoreDef;多个 :requires 合取为 And;
        // 契约谓词不得混入函数体
        let d = Desugarer::new();
        let src = "(defn divide [n d] :requires (!= d 0) :requires (> d 0) :ensures (> result 0) n)";
        let prog = d.desugar_program(parse(src)).unwrap();
        let def = prog.defs.iter().find(|def| def.name.as_str() == "divide").unwrap();
        let req = def.requires.clone().expect("requires 应被解析");
        assert!(matches!(req, Predicate::And(..)), "两个 requires 应合取为 And,实际 {:?}", req);
        assert!(def.ensures.is_some(), "ensures 应被解析");
        // body 只含一个表达式 n(契约不混入)
        if let CoreExprNode::Lam(Lambda { body, .. }) = &def.body.node {
            assert!(matches!(body.node, CoreExprNode::Var(_)), "body 应为单个 Var,实际 {:?}", body.node);
        } else {
            panic!("body 应为 Lam");
        }
    }

    #[test]
    fn test_resource_algebra_desugar() {
        // §11.1:(defresource-algebra Cost 0 + <=) 解析为 ResourceAlgebra
        let d = Desugarer::new();
        let prog = d.desugar_program(parse("(defresource-algebra Cost 0 + <=)")).unwrap();
        assert_eq!(prog.resource_algebras.len(), 1);
        let alg = &prog.resource_algebras[0];
        assert_eq!(alg.name.as_str(), "Cost");
        assert_eq!(alg.unit, "0");
        assert_eq!(alg.op.as_str(), "+");
        assert_eq!(alg.order.as_ref().map(|o| o.as_str()), Some("<="));
        // 缺参报错
        let err = d.desugar_program(parse("(defresource-algebra Cost)")).unwrap_err();
        assert!(err.message.contains("requires"), "应报缺参错误,实际: {}", err.message);
    }

    #[test]
    fn test_dependent_grade_desugar() {
        // §10:数字/符号/复合等级解析;0/1/ω 兼容
        let d = Desugarer::new();
        let prog = d.desugar_program(parse("(defn f [n (n x : i64) (5 y : i64)] x)")).unwrap();
        let def = prog.defs.iter().find(|d| d.name.as_str() == "f").unwrap();
        let params = match &def.body.node { CoreExprNode::Lam(lam) => &lam.params, _ => panic!("应为 Lam") };
        assert_eq!(params[1].grade, Grade::Var(Symbol::new("n")));
        assert_eq!(params[2].grade, Grade::Nat(5));

        // 复合等级 (+ n 1) → Add(Var(n), Nat(1))
        let prog2 = d.desugar_program(parse("(defn g [n ((+ n 1) z : i64)] z)")).unwrap();
        let def2 = prog2.defs.iter().find(|d| d.name.as_str() == "g").unwrap();
        let params2 = match &def2.body.node { CoreExprNode::Lam(lam) => &lam.params, _ => panic!("应为 Lam") };
        assert!(matches!(params2[1].grade, Grade::Add(..)), "复合等级应为 Add,实际 {:?}", params2[1].grade);

        // 0/1/ω 兼容
        let prog3 = d.desugar_program(parse("(defn h [{0 a : i64} {1 b : i64} {omega c : i64}] a)")).unwrap();
        let def3 = prog3.defs.iter().find(|d| d.name.as_str() == "h").unwrap();
        let params3 = match &def3.body.node { CoreExprNode::Lam(lam) => &lam.params, _ => panic!("应为 Lam") };
        assert_eq!(params3[0].grade, Grade::Zero);
        assert_eq!(params3[1].grade, Grade::One);
        assert_eq!(params3[2].grade, Grade::Omega);

        // 非法等级报错
        let err = d.desugar_program(parse("(defn k [((/ n 1)) w : i64] w)")).unwrap_err();
        assert!(err.message.contains("等级") || err.message.contains("grade"), "应报等级错误,实际: {}", err.message);
    }

    #[test]
    fn test_typefamily_desugar() {
        // §9:(typefamily Elem (List a) a) 解析为实例,小写 a 是类型变量
        let d = Desugarer::new();
        let src = "(typefamily Elem (List a) a)\n(defn f [x : (Elem (List i64))] -> i64 x)";
        let prog = d.desugar_program(parse(src)).unwrap();
        assert_eq!(prog.type_families.len(), 1);
        let inst = &prog.type_families[0];
        assert_eq!(inst.name.as_str(), "Elem");
        assert_eq!(inst.params.len(), 1, "参数模式应为一个完整类型,实际 {:?}", inst.params);
        assert!(matches!(inst.params[0], Type::App(..)), "模式应为 App(Con(List), Var(a)),实际 {:?}", inst.params[0]);
        assert!(matches!(inst.result, Type::Var(_)), "结果应为类型变量 a,实际 {:?}", inst.result);
    }

    #[test]
    fn test_mode_sigs_desugar() {
        // §13:defpred 的 :mode (i o) / :mode (o i) 注解写入 CoreDef.mode_sigs
        let d = Desugarer::new();
        let src = "(defpred p [x y] :mode (i o) :mode (o i) :det ([x] y))";
        let prog = d.desugar_program(parse(src)).unwrap();
        let def = prog.defs.iter().find(|def| def.name.as_str() == "p").unwrap();
        assert_eq!(def.mode_sigs.len(), 2, "应解析两个模式签名,实际 {:?}", def.mode_sigs);
        assert_eq!(def.mode_sigs[0], vec![Mode::In, Mode::Out]);
        assert_eq!(def.mode_sigs[1], vec![Mode::Out, Mode::In]);
        // 无 :mode 注解 → 空
        let src2 = "(defpred q [x] :det ([x]))";
        let prog2 = d.desugar_program(parse(src2)).unwrap();
        let def2 = prog2.defs.iter().find(|def| def.name.as_str() == "q").unwrap();
        assert!(def2.mode_sigs.is_empty());
    }

    #[test]
    fn test_refined_type_desugar() {
        // §15.1:精化类型参数 {x : i64 | (>= n 0)} 解析为 Type::Refined,谓词保留
        let d = Desugarer::new();
        let src = "(defn sqrt [x : {n : i64 | (>= n 0)}] -> i64 x)";
        let prog = d.desugar_program(parse(src)).unwrap();
        let def = prog.defs.iter().find(|def| def.name.as_str() == "sqrt").unwrap();
        let params = match &def.body.node {
            CoreExprNode::Lam(Lambda { params, .. }) => params.clone(),
            _ => panic!("body 应为 Lam"),
        };
        let ty = params[0].ty.clone().expect("精化参数应有类型注解");
        assert!(matches!(ty, Type::Refined(..)), "参数类型应为 Refined,实际 {:?}", ty);
    }
}

impl Desugarer {
    /// §9:解析 (typefamily 名称 参数模式 结果)
    /// 例:(typefamily Elem (List a) a)
    fn desugar_typefamily_form(&self, items: &[SExpr], span: Span) -> Result<tisp_core::types::TypeFamilyInstance, DesugarError> {
        if items.len() < 4 {
            return Err(DesugarError { message: "typefamily requires name, param pattern, and result type".into(), span });
        }
        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "typefamily name must be a symbol".into(), span: items[1].span }),
        };
        // 收集模式与结果中的小写符号作为类型变量(Haskell 惯例:小写=变量,大写=构造器)
        let mut type_params: Vec<Symbol> = Vec::new();
        for item in items.iter().skip(2) {
            self.collect_type_vars(item, &mut type_params);
        }
        // 参数模式:整个 items[2] 是一个类型模式(如 (List a))
        let params = vec![self.desugar_type_with_params(&items[2], &type_params)?];
        let result = self.desugar_type_with_params(&items[3], &type_params)?;
        Ok(tisp_core::types::TypeFamilyInstance { name, params, result })
    }
    /// 收集 SExpr 中的小写开头符号作为类型变量(§9 类型族模式)
    fn collect_type_vars(&self, expr: &SExpr, out: &mut Vec<Symbol>) {
        match &expr.node {
            Expr::Sym(sym) => {
                let first = sym.as_str().chars().next();
                if matches!(first, Some(c) if c.is_ascii_lowercase()) && !out.contains(sym) {
                    out.push(sym.clone());
                }
            }
            Expr::List(items) | Expr::Vec(items) => {
                for i in items { self.collect_type_vars(i, out); }
            }
            _ => {}
        }
    }
}

/// §20.2:判断 defsession 是否有 :role 角色分段
fn roles_have_marker(items: &[SExpr]) -> bool {
    items.iter().skip(2).any(|i| matches!(&i.node, Expr::Keyword(k) if k.as_str() == "role"))
}

/// §24 hygiene:宏展开时对模板中 let 绑定重命名(避免捕获调用点变量),
/// 参数符号保持替换,模板内绑定引用同步重命名
fn substitute_macro_hygienic(
    template: &SExpr,
    bindings: &std::collections::HashMap<Symbol, SExpr>,
    renames: &mut std::collections::HashMap<Symbol, Symbol>,
    counter: &mut usize,
) -> SExpr {
    match &template.node {
        Expr::Sym(s) => {
            if let Some(repl) = bindings.get(s) {
                repl.clone()
            } else if let Some(new) = renames.get(s) {
                Spanned::new(Expr::Sym(new.clone()), template.span)
            } else {
                template.clone()
            }
        }
        Expr::List(items) => {
            // let 绑定重命名:模板引入的绑定名加唯一后缀(§24 hygiene)
            if let Some(Expr::Sym(head)) = items.first().map(|i| &i.node) {
                if head.as_str() == "let" && items.len() >= 2 {
                    if let Expr::Vec(bs) = &items[1].node {
                        let mut new_bs = Vec::new();
                        let mut i = 0;
                        while i < bs.len() {
                            if let Expr::Sym(n) = &bs[i].node {
                                if !bindings.contains_key(n) && !renames.contains_key(n) {
                                    *counter += 1;
                                    let fresh = Symbol::new(&format!("{}_g{}", n, counter));
                                    renames.insert(n.clone(), fresh.clone());
                                    new_bs.push(Spanned::new(Expr::Sym(fresh), bs[i].span));
                                } else if let Some(new) = renames.get(n) {
                                    new_bs.push(Spanned::new(Expr::Sym(new.clone()), bs[i].span));
                                } else {
                                    new_bs.push(bs[i].clone());
                                }
                                if i + 1 < bs.len() {
                                    new_bs.push(substitute_macro_hygienic(&bs[i + 1], bindings, renames, counter));
                                }
                                i += 2;
                            } else {
                                new_bs.push(bs[i].clone());
                                i += 1;
                            }
                        }
                        let new_items: Vec<SExpr> = items.iter().enumerate().map(|(idx, item)| {
                            if idx == 0 {
                                item.clone()
                            } else if idx == 1 {
                                Spanned::new(Expr::Vec(new_bs.clone()), item.span)
                            } else {
                                substitute_macro_hygienic(item, bindings, renames, counter)
                            }
                        }).collect();
                        return Spanned::new(Expr::List(new_items), template.span);
                    }
                }
            }
            let new_items: Vec<SExpr> = items.iter()
                .map(|i| substitute_macro_hygienic(i, bindings, renames, counter))
                .collect();
            Spanned::new(Expr::List(new_items), template.span)
        }
        Expr::Vec(items) => {
            let new_items: Vec<SExpr> = items.iter()
                .map(|i| substitute_macro_hygienic(i, bindings, renames, counter))
                .collect();
            Spanned::new(Expr::Vec(new_items), template.span)
        }
        Expr::ConsPattern(items, tail) => {
            let new_items: Vec<SExpr> = items.iter()
                .map(|i| substitute_macro_hygienic(i, bindings, renames, counter))
                .collect();
            Spanned::new(Expr::ConsPattern(new_items, Box::new(substitute_macro_hygienic(tail, bindings, renames, counter))), template.span)
        }
        _ => template.clone(),
    }
}
