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

    // Desugar to Core AST
    let desugarer = tisp_frontend::desugar::Desugarer::new();
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

fn repl(cli: &Cli) -> miette::Result<()> {
    println!("Tisp v0.1.0 — type (exit) to quit, or an expression to evaluate");
    let mut rl = DefaultEditor::new()
        .map_err(|e| miette::miette!("{}", e))?;

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

        // Try to evaluate as an expression
        let wrapped = format!("(defn main [] {})", trimmed);
        match tisp_frontend::reader::read(&wrapped) {
            Ok(forms) => {
                let desugarer = tisp_frontend::desugar::Desugarer::new();
                if let Ok(program) = desugarer.desugar_program(forms) {
                    let mut interpreter = tisp_backend::interpreter::Interpreter::new();
                    match interpreter.run_program(&program) {
                        Ok(Some(result)) => println!("=> {:?}", result),
                        Ok(None) => eprintln!("; evaluation returned nothing"),
                        Err(e) => eprintln!("error: {}", e),
                    }
                } else if cli.print_ast {
                    let forms = tisp_frontend::reader::read(trimmed).unwrap_or_else(|_| vec![]);
                    for form in &forms { println!("{:#?}", form); }
                }
            }
            Err(e) => eprintln!("parse error: {}", e),
        }
    }

    Ok(())
}
