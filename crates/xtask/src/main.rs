//! Automação de build, CI e tarefas de desenvolvimento.
//!
//! Ferramenta de linha de comando para tarefas comuns do projeto.

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", version, about = "Automação de build e CI do Hank")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verifica formatação (cargo fmt --check)
    FmtCheck,
    /// Executa clippy em todo o workspace
    Clippy,
    /// Executa testes com coverage
    Test {
        #[arg(long)]
        coverage: bool,
    },
    /// Verifica build limpo
    Check,
    /// Gera documentação
    Doc,
    /// Valida arquitetura (forbidden imports, cycles)
    ArchCheck,
    /// Prepara release (check + test + doc)
    ReleasePrep,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::FmtCheck => run_fmt_check(),
        Commands::Clippy => run_clippy(),
        Commands::Test { coverage } => run_test(coverage),
        Commands::Check => run_check(),
        Commands::Doc => run_doc(),
        Commands::ArchCheck => run_arch_check(),
        Commands::ReleasePrep => run_release_prep(),
    }
}

fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new(cmd).args(args).status()?;

    if !status.success() {
        anyhow::bail!("comando falhou: {} {}", cmd, args.join(" "));
    }
    Ok(())
}

fn run_fmt_check() -> Result<()> {
    println!("🔍 Verificando formatação...");
    run_command("cargo", &["fmt", "--check", "--workspace"])
}

fn run_clippy() -> Result<()> {
    println!("🔍 Executando clippy...");
    run_command("cargo", &["clippy", "--workspace", "--", "-D", "warnings"])
}

fn run_test(coverage: bool) -> Result<()> {
    println!("🧪 Executando testes...");
    let mut args = vec!["test", "--workspace"];
    if coverage {
        args.extend(["--", "--coverage"]);
    }
    run_command("cargo", &args)
}

fn run_check() -> Result<()> {
    println!("🔨 Verificando build...");
    run_command("cargo", &["check", "--workspace"])
}

fn run_doc() -> Result<()> {
    println!("📚 Gerando documentação...");
    run_command("cargo", &["doc", "--workspace", "--no-deps"])
}

fn run_arch_check() -> Result<()> {
    println!("🏗️  Validando arquitetura...");
    run_command(
        "cargo",
        &[
            "test",
            "-p",
            "test-support",
            "arch_forbidden_imports",
            "detect_cycles",
            "--",
            "--nocapture",
        ],
    )
}

fn run_release_prep() -> Result<()> {
    println!("🚀 Preparando release...");
    run_fmt_check()?;
    run_clippy()?;
    run_check()?;
    run_test(false)?;
    run_arch_check()?;
    println!("✅ Release prep concluído com sucesso!");
    Ok(())
}
