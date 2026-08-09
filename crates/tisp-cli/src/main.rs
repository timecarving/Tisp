use clap::Parser;
use rustyline::DefaultEditor;

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

    #[arg(long, help = "JIT compile and run (requires --features llvm)")]
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
    let forms = tisp_frontend::reader::read(input)
        .map_err(|e| miette::miette!("{}", e))?;

    if cli.print_ast {
        for form in &forms {
            println!("{:#?}", form);
        }
    }

    println!("; {} form(s) read", forms.len());
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
    let core_program = desugarer.desugar_program(forms)
        .map_err(|e| miette::miette!("{}", e))?;

    if cli.desugar {
        for def in &core_program.defs {
            println!("def {} : {} = {:#?}", def.name, def.ty.as_ref().map(|t| format!("{}", t)).unwrap_or_else(|| "?".into()), def.body);
        }
        return Ok(());
    }

    // Type inference
    if cli.typecheck {
        let mut type_infer = tisp_middle::type_infer::TypeInfer::new();
        let typed_defs = type_infer.infer_program(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;

        for (name, ty) in typed_defs {
            println!("{} : {}", name, ty);
        }

        // Effect inference
        let mut effect_infer = tisp_middle::effect_infer::EffectInferrer::new();
        let effects = effect_infer.infer_program(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;

        for (name, eff) in effects {
            println!("{} effects: {:?}", name, eff);
        }

        // §12.6 Monad 优化路径:检测单处理器效果(可降级为 monadic 编码)
        let compiler = tisp_middle::effect_compile::EffectCompiler::new();
        let mut monad_candidates = 0;
        for def in &core_program.defs {
            if let tisp_core::core_ast::CoreExprNode::Handle(_, handler) = &def.body.node {
                if compiler.detect_single_handler(handler) {
                    monad_candidates += 1;
                }
            }
        }
        if monad_candidates > 0 {
            println!("; {} handle(s) eligible for monadic optimization (§12.6)", monad_candidates);
        }

        // Grade checking
        let mut grade_checker = tisp_middle::grade_check::GradeChecker::new();
        grade_checker.check_program(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;

        // Hole reporting
        if !type_infer.hole_env.is_empty() {
            println!("{}", type_infer.hole_env.report());
        }

        // Determinism analysis
        let mut det_analyzer = tisp_middle::determinism_analysis::DeterminismAnalyzer::new();
        let dets = det_analyzer.analyze_program(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;
        for (name, det) in dets {
            println!("{} determinism: {:?}", name, det);
        }

        // Mode analysis
        let mut mode_analyzer = tisp_middle::mode_analysis::ModeAnalyzer::new();
        let modes = mode_analyzer.analyze_program(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;
        for (name, mode) in modes {
            println!("{} mode: {:?}", name, mode);
        }

        // Region inference
        let mut region_infer = tisp_middle::region_infer::RegionInfer::new();
        let regions = region_infer.infer_program(&core_program)
            .map_err(|e| miette::miette!("{}", e))?;
        for (name, region_list) in regions {
            println!("{} regions: {:?}", name, region_list);
        }

        // Optimization
        let mut optimizer = tisp_middle::optimize::optimizer::Optimizer::new();
        let opt_program = optimizer.optimize(&core_program);
        println!("; optimizations: {} inlined, {} folded, {} dead-eliminated",
                 optimizer.stats.inlined, optimizer.stats.folded, optimizer.stats.dead_eliminated);
        println!("; program size: {} defs → {} defs after optimization",
                 core_program.defs.len(), opt_program.defs.len());

        println!("; type checking passed");
        return Ok(());
    }

    // Run program
    if cli.run {
        let mut interpreter = tisp_backend::interpreter::Interpreter::new();
        match interpreter.run_program(&core_program) {
            Ok(Some(result)) => {
                let stats = interpreter.region_stats();
                println!("=> {:?}", result);
                println!("; region stats: {} allocs, {} deallocs, {} bytes (peak: {})",
                         stats.regions_allocated, stats.regions_deallocated,
                         stats.bytes_allocated, stats.bytes_peak);
            },
            Ok(None) => println!("; no main function"),
            Err(e) => miette::bail!("{}", e),
        }
        return Ok(());
    }

    // LLVM IR output (text-based, no inkwell needed)
    if cli.ir || cli.compile {
        let mut ir_gen = tisp_backend::codegen::IrGenerator::new();
        let ir_text = ir_gen.generate(&core_program);
        println!("{}", ir_text);
        if cli.compile {
            println!("; Note: run 'llc' to compile the IR to native code");
        }
        return Ok(());
    }

    if cli.verify {
        let checker = tisp_backend::process::ModelChecker::new(20);
        let result = checker.check_reachability(0i64, |n| *n == 5, |n| vec![n + 1, n + 2]);
        println!("; verification result:");
        println!(";   property holds: {}", result.property_holds);
        println!(";   search depth: {}", result.depth);
        if !result.trace.is_empty() {
            println!(";   trace: {}", result.trace.join(" → "));
        }
        return Ok(());
    }

    println!("; compiled {} definition(s) from {}", core_program.defs.len(), path);
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
                        effect_decls: effect_decls.clone(),
                        defs: all_defs,
                    };
                    let mut type_infer = tisp_middle::type_infer::TypeInfer::new();
                    match type_infer.infer_program(&program_all) {
                        Ok(typed) => {
                            if let Some((_, ty)) = typed.iter().find(|(n, _)| n.as_str() == "main") {
                                println!("; main : {}", ty);
                            } else {
                                eprintln!("; no type inferred");
                            }
                        }
                        Err(e) => eprintln!("; type error: {}", e),
                    }
                } else if cli.print_ast {
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
                            // 表达式行:先类型检查(强静态类型:类型错误不求值),再求值
                            let mut all_defs = defs.clone();
                            all_defs.extend(program.defs);
                            let program_all = tisp_core::core_ast::CoreProgram {
                                data_decls: data_decls.clone(),
                                effect_decls: effect_decls.clone(),
                                defs: all_defs,
                            };
                            let mut type_infer = tisp_middle::type_infer::TypeInfer::new();
                            match type_infer.infer_program(&program_all) {
                                Ok(typed) => {
                                    if let Some((_, ty)) = typed.iter().find(|(n, _)| n.as_str() == "main") {
                                        println!("; main : {}", ty);
                                    }
                                }
                                Err(e) => { eprintln!("; type error: {}", e); continue; }
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
