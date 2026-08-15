use clap::Parser;
use rustyline::DefaultEditor;

use tisp_core::core_ast::CoreProgram;
use tisp_core::symbol::Symbol;

/// 统一静态检查报告:六维 pass 与液态类型的结果汇总
struct CheckReport {
    typed_defs: Vec<(Symbol, tisp_core::types::Type)>,
    effects: Vec<(Symbol, tisp_core::types::EffectRow)>,
    dets: Vec<(Symbol, tisp_core::types::Determinism)>,
    modes: Vec<(Symbol, tisp_core::types::Mode)>,
    regions: Vec<(Symbol, Vec<tisp_core::regions::Region>)>,
    monad_candidates: usize,
    hole_report: Option<String>,
    liquid_report: tisp_backend::liquid_verify::LiquidReport,
}

/// 运行全部静态检查(type/effect/grade/mode/determinism/region/liquid)。
/// 各维度冲突聚合进共享约束图后统一报告;任一维度失败即返回 Err。
fn static_checks(program: &CoreProgram) -> Result<CheckReport, String> {
    use tisp_middle::constraint::{ConstraintGraph, Dimension};

    let mut graph = ConstraintGraph::new();

    // ── Type ──
    let mut type_infer = tisp_middle::type_infer::TypeInfer::new();
    let typed_defs = match type_infer.infer_program(program) {
        Ok(defs) => defs,
        Err(e) => {
            graph.record(Dimension::Type, None, e.message, e.span);
            Vec::new()
        }
    };
    let hole_report = (!type_infer.hole_env.is_empty()).then(|| type_infer.hole_env.report());

    // ── Effect ──
    let mut effect_infer = tisp_middle::effect_infer::EffectInferrer::new();
    let effects = match effect_infer.infer_program(program) {
        Ok(effects) => effects,
        Err(e) => {
            graph.record(Dimension::Effect, None, e.message, e.span);
            Vec::new()
        }
    };
    // §26/§7.5 Unsafe 门控:推断含 Unsafe 而定义未声明/处理 Unsafe → 拒绝
    for def in &program.defs {
        if let Some((_, inferred)) = effects.iter().find(|(n, _)| n == &def.name) {
            let inferred_unsafe = tisp_core::effects::row_contains(inferred,
                &tisp_core::types::EffectLabel::Named(Symbol::new("Unsafe")));
            let declared_unsafe = tisp_core::effects::row_contains(&def.effects,
                &tisp_core::types::EffectLabel::Named(Symbol::new("Unsafe")));
            if inferred_unsafe && !declared_unsafe {
                graph.record(Dimension::Effect, Some(def.name.clone()),
                    format!("Unsafe 效应缺失:定义 {} 调用 ptr-read/ptr-write/region-alloc 等裸内存操作,但效应行未声明 Unsafe(纯声明式门控)", def.name),
                    def.span);
            }
        }
    }
    // §2 纯声明式范式门控:State/Signal 同样须声明或经 handler 处理
    let row_has = |row: &tisp_core::types::EffectRow, named: &str, kind: fn(&tisp_core::types::EffectLabel) -> bool| -> bool {
        use tisp_core::types::EffectLabel;
        if tisp_core::effects::row_contains(row, &EffectLabel::Named(Symbol::new(named))) {
            return true;
        }
        match row {
            tisp_core::types::EffectRow::Pure => false,
            tisp_core::types::EffectRow::Closed(labels) | tisp_core::types::EffectRow::Open(labels, _) => labels.iter().any(kind),
            tisp_core::types::EffectRow::Var(_) => true,
        }
    };
    for def in &program.defs {
        if let Some((_, inferred)) = effects.iter().find(|(n, _)| n == &def.name) {
            let checks: Vec<(&str, fn(&tisp_core::types::EffectLabel) -> bool)> = vec![
                ("State", |l: &tisp_core::types::EffectLabel| matches!(l, tisp_core::types::EffectLabel::State(_))),
                ("Signal", |l: &tisp_core::types::EffectLabel| matches!(l, tisp_core::types::EffectLabel::Signal)),
            ];
            for (name, kind) in checks {
                let inferred_has = row_has(inferred, name, kind);
                let declared_has = row_has(&def.effects, name, kind);
                if inferred_has && !declared_has {
                    graph.record(Dimension::Effect, Some(def.name.clone()),
                        format!("{} 效应缺失:定义 {} 调用状态/信号类范式操作,但效应行未声明或处理该效应(纯声明式副作用管理)", name, def.name),
                        def.span);
                }
            }
        }
    }

    // ── Grade ──
    let mut grade_checker = tisp_middle::grade_check::GradeChecker::new();
    let grade_ok = match grade_checker.check_program(program) {
        Ok(()) => true,
        Err(e) => {
            graph.record(Dimension::Grade, None, e.message, e.span);
            false
        }
    };

    // ── Determinism ──
    let mut det_analyzer = tisp_middle::determinism_analysis::DeterminismAnalyzer::new();
    let dets = match det_analyzer.analyze_program(program) {
        Ok(dets) => dets,
        Err(e) => {
            graph.record(Dimension::Determinism, None, e.message, e.span);
            Vec::new()
        }
    };

    // ── Mode ──
    let mut mode_analyzer = tisp_middle::mode_analysis::ModeAnalyzer::new();
    let modes = match mode_analyzer.analyze_program(program) {
        Ok(modes) => modes,
        Err(e) => {
            graph.record(Dimension::Mode, None, e.message, e.span);
            Vec::new()
        }
    };

    // ── Region ──
    let mut region_infer = tisp_middle::region_infer::RegionInfer::new();
    let regions = match region_infer.infer_program(program) {
        Ok(regions) => regions,
        Err(e) => {
            graph.record(Dimension::Region, None, e.message, e.span);
            Vec::new()
        }
    };

    if !graph.is_empty() {
        return Err(tisp_middle::solve::format_report(&graph));
    }

    // ── §12.6 Monad 候选 ──
    let compiler = tisp_middle::effect_compile::EffectCompiler::new();
    let monad_candidates = program.defs.iter().filter(|def| {
        if let tisp_core::core_ast::CoreExprNode::Handle(_, handler) = &def.body.node {
            compiler.detect_single_handler(handler)
        } else {
            false
        }
    }).count();

    // ── Liquid(§15)+ 依赖等级不等式(Z3) ──
    let mut liquid_verifier = tisp_backend::liquid_verify::LiquidVerifier::new();
    liquid_verifier.verify_program(program);
    if grade_ok {
        liquid_verifier.verify_grade_inequalities(&grade_checker.inequalities);
    }
    let liquid_report = liquid_verifier.report().clone();
    if liquid_report.violated > 0 {
        let mut msgs: Vec<String> = liquid_report.errors.iter().map(|e| e.message.clone()).collect();
        msgs.insert(0, format!("liquid type verification failed: {} violation(s)", liquid_report.violated));
        return Err(msgs.join("\n"));
    }

    Ok(CheckReport { typed_defs, effects, dets, modes, regions, monad_candidates, hole_report, liquid_report })
}

/// 判定验证结果是否成立:Bool true 或 VerifyResult 的首字段 Bool true
fn verify_holds(value: &tisp_backend::interpreter::Value) -> bool {
    use tisp_backend::interpreter::Value;
    match value {
        Value::Bool(b) => *b,
        Value::Data(name, fields) if name.as_str() == "VerifyResult" => {
            matches!(fields.first(), Some(Value::Bool(true)))
        }
        _ => false,
    }
}

/// 把推断后的签名回填 CoreDef,使解释器反射表(type-of/effects-of/...)返回真实静态信息
fn apply_checked_signatures(program: &mut CoreProgram, report: &CheckReport) {
    for def in &mut program.defs {
        if let Some((_, ty)) = report.typed_defs.iter().find(|(n, _)| n == &def.name) {
            def.ty = Some(ty.clone());
        }
        if let Some((_, eff)) = report.effects.iter().find(|(n, _)| n == &def.name) {
            def.effects = eff.clone();
        }
        if let Some((_, det)) = report.dets.iter().find(|(n, _)| n == &def.name) {
            def.determinism = det.clone();
        }
        if let Some((_, mode)) = report.modes.iter().find(|(n, _)| n == &def.name) {
            def.mode = mode.clone();
        }
    }
}

/// 打印静态检查报告(--typecheck 输出)
fn print_check_report(program: &CoreProgram, report: &CheckReport) {    for (name, ty) in &report.typed_defs {
        println!("{} : {}", name, ty);
    }
    for (name, eff) in &report.effects {
        println!("{} effects: {:?}", name, eff);
    }
    if report.monad_candidates > 0 {
        println!("; {} handle(s) eligible for monadic optimization (§12.6)", report.monad_candidates);
    }
    if let Some(holes) = &report.hole_report {
        println!("{}", holes);
    }
    for (name, det) in &report.dets {
        println!("{} determinism: {:?}", name, det);
    }
    for (name, mode) in &report.modes {
        println!("{} mode: {:?}", name, mode);
    }
    for (name, region_list) in &report.regions {
        println!("{} regions: {:?}", name, region_list);
    }
    if report.liquid_report.degraded {
        println!("; liquid types: z3 solver unavailable, degraded to constant folding (apt install z3)");
    } else {
        println!("; liquid types: {} verified, {} violated, {} warned",
                 report.liquid_report.verified, report.liquid_report.violated, report.liquid_report.warned);
    }
    let suppressed = |cat: &str| {
        program.pragmas.iter().any(|(name, targets)|
            name.as_str() == "suppress-warning" && targets.iter().any(|t| t.as_str() == cat))
    };
    if !suppressed("liquid") {
        for err in &report.liquid_report.errors {
            eprintln!("liquid type error: {}", err.message);
        }
    }
}


#[derive(Parser, Debug)]
#[command(name = "tisp", version, about = "Tisp programming language")]
struct Cli {
    #[arg(help = "Source file to compile")]
    file: Option<String>,

    #[arg(short = 'e', long, help = "Evaluate expression")]
    eval: Option<String>,

    #[arg(long, help = "Print AST")]
    print_ast: bool,

    #[arg(long, help = "Print tokens")]
    print_tokens: bool,

    #[arg(long, help = "Desugar and print Core AST")]
    desugar: bool,

    #[arg(long, help = "Run type inference")]
    typecheck: bool,

    #[arg(long, help = "Run the program (interpret)")]
    run: bool,

    #[arg(long, help = "Run model checker")]
    verify: bool,

    #[arg(long, help = "Compile to LLVM IR (requires --features llvm)")]
    ir: bool,

    #[arg(long, help = "Compile via llc/clang and run (requires llvm feature)")]
    compile: bool,
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    if let Some(ref expr) = cli.eval {
        eval_expr(expr, &cli)?;
    } else if let Some(ref file) = cli.file {
        compile_file(file, &cli)?;
    } else {
        repl(&cli)?;
    }

    Ok(())
}

fn eval_expr(input: &str, cli: &Cli) -> miette::Result<()> {
    let wrapped = format!("(defn main [] {})", input);
    let forms = tisp_frontend::reader::read(&wrapped)
        .map_err(|e| miette::miette!("{}", e))?;

    if cli.print_ast {
        for form in &forms {
            println!("{:#?}", form);
        }
    }

    let desugarer = tisp_frontend::desugar::Desugarer::new();
    let mut program = desugarer.desugar_program(forms)
        .map_err(|e| miette::miette!("{}", e))?;
    program = tisp_backend::comptime::ComptimePass::new().run(&program)
        .map_err(|e| miette::miette!("{}", e))?;

    // 静态类型 + 纯声明约束:先检查后求值
    let report = static_checks(&program).map_err(|e| miette::miette!("{}", e))?;
    apply_checked_signatures(&mut program, &report);

    let mut interpreter = tisp_backend::interpreter::Interpreter::new();
    match interpreter.run_program(&program) {
        Ok(Some(result)) => println!("=> {:?}", result),
        Ok(None) => println!("; evaluation returned nothing"),
        Err(e) => miette::bail!("{}", e),
    }
    Ok(())
}

fn compile_file(path: &str, cli: &Cli) -> miette::Result<()> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| miette::miette!("failed to read {}: {}", path, e))?;

    if cli.print_tokens {
        let tokens = tisp_frontend::lexer::tokenize(&source)
            .map_err(|e| miette::miette!("{}", e))?;
        for tok in &tokens {
            println!("{:?} @ {}", tok.token, tok.span);
        }
        return Ok(());
    }

    let forms = tisp_frontend::reader::read(&source)
        .map_err(|e| miette::miette!("{}", e))?;

    if cli.print_ast {
        for form in &forms {
            println!("{:#?}", form);
        }
        return Ok(());
    }

    // Desugar to Core AST(带模块加载基准目录)
    let desugarer = tisp_frontend::desugar::Desugarer::new();
    if let Some(dir) = std::path::Path::new(path).parent() {
        desugarer.set_base_dir(&dir.to_string_lossy());
    }
    let mut core_program = desugarer.desugar_program(forms)
        .map_err(|e| miette::miette!("{}", e))?;

    // comptime 编译期 pass:求值 Comptime 节点 + MOP 知识库 + AOP 切面编织
    core_program = tisp_backend::comptime::ComptimePass::new().run(&core_program)
        .map_err(|e| miette::miette!("{}", e))?;

    if cli.desugar {
        // §11.1 资源代数
        for alg in &core_program.resource_algebras {
            println!("; resource-algebra {}: unit={}, op={}{}",
                     alg.name, alg.unit, alg.op,
                     alg.order.as_ref().map(|o| format!(", order={}", o)).unwrap_or_default());
        }
        for def in &core_program.defs {
            println!("def {} : {} = {:#?}", def.name, def.ty.as_ref().map(|t| format!("{}", t)).unwrap_or_else(|| "?".into()), def.body);
            if let Some(req) = &def.requires {
                println!("  requires: {:#?}", req);
            }
            if let Some(ens) = &def.ensures {
                println!("  ensures: {:#?}", ens);
            }
        }
        return Ok(());
    }

    // Type inference
    if cli.typecheck {
        let report = static_checks(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;
        print_check_report(&core_program, &report);

        // §22.4 泛型编译期特化 + §30 优化(仅 --typecheck 展示统计)
        let mut specializer = tisp_middle::specialize::Specializer::new();
        let specialized_program = specializer.specialize(&core_program);
        if specializer.specialized > 0 {
            println!("; specialization: {} generic call(s) specialized", specializer.specialized);
        }
        let mut optimizer = tisp_middle::optimize::optimizer::Optimizer::new();
        optimizer.configure(&core_program.pragmas);
        let opt_program = optimizer.optimize(&specialized_program);
        println!("; optimizations: {} inlined, {} folded, {} dead-eliminated",
                 optimizer.stats.inlined, optimizer.stats.folded, optimizer.stats.dead_eliminated);
        println!("; program size: {} defs → {} defs after optimization",
                 core_program.defs.len(), opt_program.defs.len());

        println!("; type checking passed");
        return Ok(());
    }

    // Run program(先静态检查,再执行)
    if cli.run {
        let report = static_checks(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;
        apply_checked_signatures(&mut core_program, &report);
        // §22.4 泛型编译期特化接入执行路径(非仅 --typecheck 展示)
        let mut specializer = tisp_middle::specialize::Specializer::new();
        let run_program = specializer.specialize(&core_program);
        // §8.2 在较大栈线程中执行:debug 构建下 eval_expr 是巨型函数(帧较大),
        // 深递归会耗尽主线程 8MB 栈;256MB 栈给非尾递归留足余量(尾递归已由 TCO 保证 O(1) 栈)
        let run_result = std::thread::Builder::new()
            .name("tisp-run".into())
            .stack_size(256 * 1024 * 1024)
            .spawn(move || {
                let mut interpreter = tisp_backend::interpreter::Interpreter::new();
                let r = interpreter.run_program(&run_program);
                let stats = interpreter.region_stats().clone();
                (r, stats, interpreter.monadic_handles)
            })
            .map_err(|e| miette::miette!("failed to spawn interpreter thread: {}", e))?
            .join()
            .map_err(|_| miette::miette!("interpreter thread panicked"))?;
        match run_result.0 {
            Ok(Some(result)) => {
                println!("=> {:?}", result);
                if run_result.2 > 0 {
                    println!("; monadic optimization (§12.6): {} single-handler handle(s) via direct state passing", run_result.2);
                }
                println!("; region stats: {} allocs, {} deallocs, {} bytes (peak: {})",
                         run_result.1.regions_allocated, run_result.1.regions_deallocated,
                         run_result.1.bytes_allocated, run_result.1.bytes_peak);
            },
            Ok(None) => println!("; no main function"),
            Err(e) => miette::bail!("{}", e),
        }
        return Ok(());
    }

    // LLVM IR / 编译运行
    if cli.ir || cli.compile {
        let report = static_checks(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;
        apply_checked_signatures(&mut core_program, &report);
        let mut ir_gen = tisp_backend::codegen::IrGenerator::new();
        let ir_text = ir_gen.generate(&core_program);
        if cli.ir {
            println!("{}", ir_text);
            return Ok(());
        }
        // --compile:llvm feature 下 llc/clang 编译并运行;默认构建显式报错
        #[cfg(feature = "llvm")]
        {
            compile_and_run(&ir_text)?;
        }
        #[cfg(not(feature = "llvm"))]
        {
            miette::bail!("--compile 需要启用 llvm feature(以 --features llvm 构建)");
        }
    }

    if cli.verify {
        let report = static_checks(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;
        apply_checked_signatures(&mut core_program, &report);
        let program = core_program.clone();
        let run_result = std::thread::Builder::new()
            .name("tisp-verify".into())
            .stack_size(256 * 1024 * 1024)
            .spawn(move || {
                let mut interpreter = tisp_backend::interpreter::Interpreter::new();
                interpreter.verify_properties(&program)
            })
            .map_err(|e| miette::miette!("failed to spawn verify thread: {}", e))?
            .join()
            .map_err(|_| miette::miette!("verify thread panicked"))?;
        match run_result {
            Ok(results) if !results.is_empty() => {
                for (name, value) in &results {
                    let holds = verify_holds(value);
                    println!("; property {}: {:?} (holds: {})", name, value, holds);
                }
                println!("; verification result: {} property/properties checked", results.len());
            }
            Ok(_) => miette::bail!("无可验证属性:程序未包含 defprop 声明"),
            Err(e) => miette::bail!("{}", e),
        }
        return Ok(());
    }

    println!("; compiled {} definition(s) from {}", core_program.defs.len(), path);
    Ok(())
}

/// --compile(llvm feature):IR → llc-17 目标文件 → clang-17 链接 → 运行
#[cfg(feature = "llvm")]
fn compile_and_run(ir_text: &str) -> miette::Result<()> {
    use std::process::Command;
    let dir = std::env::temp_dir().join(format!("tisp-compile-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| miette::miette!("创建临时目录失败: {}", e))?;
    let ll_path = dir.join("program.ll");
    let obj_path = dir.join("program.o");
    let exe_path = dir.join("program");
    std::fs::write(&ll_path, ir_text).map_err(|e| miette::miette!("写入 IR 失败: {}", e))?;

    let llc = Command::new("llc-17").arg("-filetype=obj").arg(&ll_path).arg("-o").arg(&obj_path).output()
        .map_err(|e| miette::miette!("llc-17 不可用(需 llvm-17): {}", e))?;
    if !llc.status.success() {
        miette::bail!("llc-17 编译失败: {}", String::from_utf8_lossy(&llc.stderr));
    }
    let clang = ["clang-17", "clang-19", "gcc"].iter().find_map(|tool| {
        Command::new(tool).arg(&obj_path).arg("-o").arg(&exe_path).output().ok()
    }).ok_or_else(|| miette::miette!("clang/gcc 不可用,无法链接编译产物"))?;
    if !clang.status.success() {
        miette::bail!("clang-17 链接失败: {}", String::from_utf8_lossy(&clang.stderr));
    }
    let run = Command::new(&exe_path).output()
        .map_err(|e| miette::miette!("运行编译产物失败: {}", e))?;
    println!("; --compile: result {}", run.status.code().unwrap_or(-1));
    print!("{}", String::from_utf8_lossy(&run.stdout));
    if !run.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&run.stderr));
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

/// 判断顶层 form 是否为定义形式(defn/defdata/defpred/defmacro/...)
fn is_definition_form(form: &tisp_core::ast::SExpr) -> bool {
    if let tisp_core::ast::Expr::List(parts) = &form.node {
        if let Some(first) = parts.first() {
            if let tisp_core::ast::Expr::Sym(name) = &first.node {
                return matches!(name.as_str(),
                    "defn" | "defn-" | "def" | "def-" | "defdata" | "defdata-hit" | "defpred"
                    | "defmacro" | "defeffect" | "defgeneric" | "defmethod" | "defclass"
                    | "definstance" | "defsession" | "defextern" | "ns");
            }
        }
    }
    false
}

fn repl(cli: &Cli) -> miette::Result<()> {
    println!("Tisp v0.1.0 — (exit) to quit; expressions show inferred type; :type EXPR queries type only");
    let mut rl = DefaultEditor::new()
        .map_err(|e| miette::miette!("{}", e))?;

    // 跨行状态:同一 Desugarer(宏表跨行保留)+ 累积的程序定义
    let desugarer = tisp_frontend::desugar::Desugarer::new();
    let mut data_decls: Vec<tisp_core::data::DataDecl> = Vec::new();
    let mut effect_decls: Vec<tisp_core::effects::EffectDecl> = Vec::new();
    let mut defs: Vec<tisp_core::core_ast::CoreDef> = Vec::new();

    loop {
        let line = match rl.readline("tisp> ") {
            Ok(line) => line,
            Err(rustyline::error::ReadlineError::Interrupted | rustyline::error::ReadlineError::Eof) => break,
            Err(e) => return Err(miette::miette!("{}", e)),
        };

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "(exit)" || trimmed == "(quit)" {
            break;
        }

        rl.add_history_entry(&line).ok();

        // :type EXPR — 只查类型不求值
        if let Some(expr_str) = trimmed.strip_prefix(":type ").or_else(|| trimmed.strip_prefix(":t ")) {
            let wrapped = format!("(defn main [] {})", expr_str);
            if let Ok(forms) = tisp_frontend::reader::read(&wrapped) {
                if let Ok(program) = desugarer.desugar_program(forms) {
                    let mut all_defs = defs.clone();
                    all_defs.extend(program.defs);
                    let program_all = tisp_core::core_ast::CoreProgram {
                        data_decls: data_decls.clone(),
                        type_families: vec![],
            resource_algebras: vec![],
                        effect_decls: effect_decls.clone(),
                        defs: all_defs,
                        pragmas: vec![],
                    };
                    match static_checks(&program_all) {
                        Ok(report) => {
                            if let Some((_, ty)) = report.typed_defs.iter().find(|(n, _)| n.as_str() == "main") {
                                println!("; main : {}", ty);
                            } else {
                                eprintln!("; no type inferred");
                            }
                        }
                        Err(e) => eprintln!("; static check error: {}", e),
                    }                } else if cli.print_ast {
                    let forms = tisp_frontend::reader::read(trimmed).unwrap_or_else(|_| vec![]);
                    for form in &forms { println!("{:#?}", form); }
                }
            }
            continue;
        }

        // 定义行(defn/defdata/defpred/defmacro/...)并入累积;否则作为表达式求值
        let is_def = tisp_frontend::reader::read(trimmed)
            .ok()
            .and_then(|forms| forms.into_iter().next())
            .map(|f| is_definition_form(&f))
            .unwrap_or(false);

        let wrapped = if is_def {
            trimmed.to_string()
        } else {
            format!("(defn main [] {})", trimmed)
        };

        match tisp_frontend::reader::read(&wrapped) {
            Ok(forms) => {
                match desugarer.desugar_program(forms) {
                    Ok(program) => {
                        if is_def {
                            // 定义行:并入累积,不求值
                            data_decls.extend(program.data_decls);
                            effect_decls.extend(program.effect_decls);
                            let names: Vec<String> = program.defs.iter()
                                .map(|d| d.name.as_str().to_string()).collect();
                            defs.extend(program.defs);
                            println!("; defined{}", if names.is_empty() {
                                String::new()
                            } else {
                                format!(": {}", names.join(", "))
                            });
                        } else {
                            // 表达式行:先统一静态检查(强静态类型:错误不求值),再求值
                            let mut all_defs = defs.clone();
                            all_defs.extend(program.defs);
                            let program_all = tisp_core::core_ast::CoreProgram {
                                data_decls: data_decls.clone(),
                        type_families: vec![],
            resource_algebras: vec![],
                                effect_decls: effect_decls.clone(),
                                defs: all_defs,
                                pragmas: vec![],
                            };
                            match static_checks(&program_all) {
                                Ok(report) => {
                                    if let Some((_, ty)) = report.typed_defs.iter().find(|(n, _)| n.as_str() == "main") {
                                        println!("; main : {}", ty);
                                    }
                                }
                                Err(e) => { eprintln!("; static check error: {}", e); continue; }
                            }
                            let mut interpreter = tisp_backend::interpreter::Interpreter::new();
                            match interpreter.run_program(&program_all) {
                                Ok(Some(result)) => println!("=> {:?}", result),
                                Ok(None) => eprintln!("; evaluation returned nothing"),
                                Err(e) => eprintln!("error: {}", e),
                            }
                        }
                    }
                    Err(e) => eprintln!("; desugar error: {}", e),
                }
            }
            Err(e) => eprintln!("parse error: {}", e),
        }
    }

    Ok(())
}
