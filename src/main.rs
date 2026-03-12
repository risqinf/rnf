// ─────────────────────────────────────────────────────────────────────────────
//  RNF  –  High Performance Systems & Automation Language
//  github.com/risqinf/rnf
// ─────────────────────────────────────────────────────────────────────────────

mod ast;
mod codegen;
mod interpreter;
mod lexer;
mod parser;

use clap::{Parser as ClapParser, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};

// ── CLI Definition ────────────────────────────────────────────────────────────

#[derive(ClapParser)]
#[command(
    name    = "rnf",
    version = "0.1.0",
    author  = "risqinf <github.com/risqinf>",
    about   = "RNF — High Performance Systems & Automation Language",
)]
struct Cli {
    /// Run a .rnf file directly (interpreter mode)
    #[arg(long, value_name = "FILE")]
    run: Option<PathBuf>,

    /// Build static release binary (LLVM, stripped, musl)
    #[arg(long)]
    release: bool,

    /// Custom output path for --release build
    #[arg(long, value_name = "PATH")]
    path: Option<PathBuf>,

    /// Positional input file (used with --release)
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a .rnf file (interpreter — no compilation)
    Run {
        /// Source file to run
        file: PathBuf,
    },
    /// Build static binary (LLVM + musl, stripped)
    Release {
        /// Source file to compile
        file: PathBuf,
        /// Custom output directory/path
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Check syntax only (no execution)
    Check {
        file: PathBuf,
    },
    /// Dump token stream (debug)
    Tokens {
        file: PathBuf,
    },
    /// Dump AST (debug)
    Ast {
        file: PathBuf,
    },
    /// Emit LLVM IR to stdout
    Ir {
        file: PathBuf,
    },
    /// Create a new RNF project scaffold
    Init {
        /// Project name (default: current dir name)
        name: Option<String>,
    },
    /// Show language version and build info
    Version,
}

// ── Entry ─────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    // --run FILE (flag form)
    if let Some(file) = cli.run {
        return cmd_run(&file);
    }

    // --release [--path P] FILE
    if cli.release {
        let file = cli.file.unwrap_or_else(|| die("--release requires a source file"));
        return cmd_release(&file, cli.path.as_deref());
    }

    match cli.command {
        Some(Commands::Run     { file })             => cmd_run(&file),
        Some(Commands::Release { file, path })       => cmd_release(&file, path.as_deref()),
        Some(Commands::Check   { file })             => cmd_check(&file),
        Some(Commands::Tokens  { file })             => cmd_tokens(&file),
        Some(Commands::Ast     { file })             => cmd_ast(&file),
        Some(Commands::Ir      { file })             => cmd_ir(&file),
        Some(Commands::Init    { name })             => cmd_init(name.as_deref()),
        Some(Commands::Version)                      => cmd_version(),
        None => {
            if let Some(file) = cli.file {
                // bare: rnf main.rnf  → run it
                cmd_run(&file);
            } else {
                print_banner();
                println!("{}", "Usage:".bold());
                println!("  rnf --run main.rnf             Run a script");
                println!("  rnf --release main.rnf         Build static binary");
                println!("  rnf --release --path /out main.rnf");
                println!("  rnf init myproject             Create new project");
                println!("  rnf --help                     Full help");
                println!();
                println!("{}", "Examples:".bold());
                println!("  rnf --run examples/hello.rnf");
                println!("  rnf --release --path /usr/local/bin examples/hello.rnf");
                std::process::exit(1);
            }
        }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

fn cmd_run(path: &Path) {
    let source  = read_file(path);
    let program = parse(path, &source);
    let mut interp = interpreter::Interpreter::new();
    if let Err(e) = interp.run(&program) {
        eprintln!("{} {}", "Runtime Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn cmd_release(path: &Path, out: Option<&Path>) {
    let source  = read_file(path);
    let stem    = stem_of(path);
    let program = parse(path, &source);

    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
    println!("  {} {}  →  static binary", "RNF Release".cyan().bold(), stem.yellow());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());

    // Determine output directory
    let (out_dir, bin_name) = match out {
        Some(p) if p.extension().is_some() => {
            // Full file path given
            (p.parent().unwrap_or(Path::new(".")).to_path_buf(), p.file_name().unwrap().to_string_lossy().to_string())
        }
        Some(p) => (p.to_path_buf(), stem.clone()),
        None    => {
            let dir = PathBuf::from("release").join("binary");
            (dir, stem.clone())
        }
    };

    std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| die(&format!("Cannot create output dir: {}", e)));

    let ir_path  = std::env::temp_dir().join(format!("rnf_{}.ll", stem));
    let obj_path = std::env::temp_dir().join(format!("rnf_{}.o",  stem));
    let bin_path = out_dir.join(&bin_name);

    // Codegen → LLVM IR
    println!("{} Generating LLVM IR…", "→".cyan());
    let mut cg = codegen::LlvmCodegen::new(&stem);
    let ir = cg.generate(&program).unwrap_or_else(|e| die(&format!("Codegen: {}", e)));
    std::fs::write(&ir_path, &ir).unwrap_or_else(|e| die(&format!("Write IR: {}", e)));
    println!("{} LLVM IR  → {}", "✓".green(), ir_path.display());

    // Compile
    codegen::compile_to_binary(&ir_path, &obj_path, &bin_path);

    println!();
    println!("  {} Binary: {}", "✓".green().bold(), bin_path.display().to_string().yellow());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
}

fn cmd_check(path: &Path) {
    let source = read_file(path);
    let _prog  = parse(path, &source);
    println!("{} {} — syntax OK", "✓".green().bold(), path.display());
}

fn cmd_tokens(path: &Path) {
    let source = read_file(path);
    let mut lex = lexer::Lexer::new(&source);
    match lex.tokenize() {
        Ok(tokens) => {
            for tok in &tokens {
                println!("{:>5}:{:<4} {:?}", tok.line, tok.col, tok.kind);
            }
            println!("\n{} tokens total", tokens.len().to_string().cyan());
        }
        Err(e) => die(&e),
    }
}

fn cmd_ast(path: &Path) {
    let source = read_file(path);
    let prog   = parse(path, &source);
    println!("{:#?}", prog);
}

fn cmd_ir(path: &Path) {
    let source = read_file(path);
    let stem   = stem_of(path);
    let prog   = parse(path, &source);
    let mut cg = codegen::LlvmCodegen::new(&stem);
    match cg.generate(&prog) {
        Ok(ir) => println!("{}", ir),
        Err(e) => die(&format!("Codegen: {}", e)),
    }
}

fn cmd_init(name: Option<&str>) {
    let project = name.unwrap_or("rnf_project");
    let root    = PathBuf::from(project);
    let src_dir = root.join("src");

    std::fs::create_dir_all(&src_dir).ok();

    let main_src = format!(
r#"// {} — RNF Project
// Run:   rnf --run src/main.rnf
// Build: rnf --release src/main.rnf

fn main() -> int {{
    name = "{}"
    print("Hello from " + name + "!")

    // System automation
    exec "echo 'RNF is running!'"

    return 0
}}
"#,
        project, project
    );

    std::fs::write(src_dir.join("main.rnf"), main_src).ok();

    std::fs::write(root.join("README.md"), format!(
"# {}\n\nAn RNF project.\n\n## Run\n```sh\nrnf --run src/main.rnf\n```\n\n## Build\n```sh\nrnf --release src/main.rnf\n```\n\n## Custom output path\n```sh\nrnf --release --path /custom/path src/main.rnf\n```\n",
        project
    )).ok();

    std::fs::write(root.join(".gitignore"), "release/\n*.ll\n*.o\n").ok();

    println!("{} Project '{}' created!", "✓".green().bold(), project.yellow());
    println!();
    println!("  {}", format!("cd {}", project).cyan());
    println!("  {}", "rnf --run src/main.rnf".cyan());
}

fn cmd_version() {
    print_banner();
    println!("  Version    : {}", "0.1.0".cyan());
    println!("  Author     : {}", "risqinf (github.com/risqinf)".cyan());
    println!("  License    : {}", "MIT".cyan());
    println!("  Backend    : {}", "LLVM (static, stripped, musl)".cyan());
    println!("  Extension  : {}", ".rnf".cyan());
    println!("  Repository : {}", "https://github.com/risqinf/rnf".cyan());
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_file(path: &Path) -> String {
    if !path.exists() {
        die(&format!("File not found: {}", path.display()));
    }
    if path.extension().and_then(|e| e.to_str()) != Some("rnf") {
        eprintln!("{} Expected .rnf extension — continuing anyway", "Warning:".yellow());
    }
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| die(&format!("Cannot read '{}': {}", path.display(), e)))
}

fn parse(path: &Path, source: &str) -> ast::Program {
    let file = path.file_name().unwrap_or_default().to_string_lossy().into_owned();

    let mut lex = lexer::Lexer::new(source);
    let tokens  = lex.tokenize().unwrap_or_else(|e| {
        eprintln!("{} [{}] {}", "Lexer Error:".red().bold(), file, e);
        std::process::exit(1);
    });

    let mut p = parser::Parser::new(tokens);
    p.parse().unwrap_or_else(|e| {
        eprintln!("{} [{}] {}", "Parse Error:".red().bold(), file, e);
        std::process::exit(1);
    })
}

fn stem_of(path: &Path) -> String {
    path.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}

fn die(msg: &str) -> ! {
    eprintln!("{} {}", "Error:".red().bold(), msg);
    std::process::exit(1);
}

fn print_banner() {
    println!();
    println!("{}", "  ██████╗ ███╗   ██╗███████╗".cyan().bold());
    println!("{}", "  ██╔══██╗████╗  ██║██╔════╝".cyan().bold());
    println!("{}", "  ██████╔╝██╔██╗ ██║█████╗  ".cyan().bold());
    println!("{}", "  ██╔══██╗██║╚██╗██║██╔══╝  ".cyan().bold());
    println!("{}", "  ██║  ██║██║ ╚████║██║     ".cyan().bold());
    println!("{}", "  ╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝     ".cyan().bold());
    println!();
    println!("  {} {}",
        "RNF Programming Language".bold(),
        "v0.1.0".dimmed()
    );
    println!("  {}", "High Performance · Systems · Automation".dimmed());
    println!("  {}", "github.com/risqinf/rnf".dimmed());
    println!();
}
