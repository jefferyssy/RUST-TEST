//! cli — 将 HTML+CSS+JS 前端项目编译为 Rust 原生应用
//!
//! 这是整个工具链的命令行入口，提供两个子命令：
//!
//! ## 用法
//! ```text
//! cli compile <input-dir> [-o <output-dir>] [--name <name>] [--title <title>] [--width <w>] [--height <h>]
//! cli run     <input-dir> [-o <output-dir>] [--name <name>] [--title <title>] [--width <w>] [--height <h>]
//! ```
//!
//! ## 子命令
//! - **compile** — 读取输入目录中的 HTML/CSS/JS 源文件，委托 [`compiler::compile_project_to_dir`]
//!   生成完整的 Cargo 项目（含 `src/main.rs` 和 `Cargo.toml`），输出到指定目录
//! - **run** — 先执行 compile，然后自动 `cargo run` 运行生成的二进制

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::{self, Command as StdCommand};

// ── clap 命令行定义 ──

/// 将 HTML+CSS+JS 前端项目编译为 Rust 原生应用。
#[derive(Parser, Debug)]
#[command(name = "cli", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// CLI 子命令。
#[derive(Subcommand, Debug)]
enum Command {
    /// 仅生成 Rust 项目代码，不运行。
    Compile(Args),
    /// 生成代码后自动调用 `cargo run` 编译并运行。
    Run(Args),
}

/// `compile` 和 `run` 共用的构建参数。
#[derive(clap::Args, Debug, Clone)]
struct Args {
    /// 输入目录路径
    input_dir: PathBuf,

    /// 输出目录路径
    #[arg(short = 'o', long)]
    output_dir: Option<PathBuf>,

    /// 项目名称
    #[arg(long)]
    name: Option<String>,

    /// 窗口标题
    #[arg(long)]
    title: Option<String>,

    /// 窗口宽度（像素）
    #[arg(long)]
    width: Option<u32>,

    /// 窗口高度（像素）
    #[arg(long)]
    height: Option<u32>,

    /// 启用运行时可视化仪表盘（DevTools）
    #[arg(long)]
    view: bool,
}

// ── Args → CompileInput ──

impl From<&Args> for compiler::CompileInput {
    fn from(a: &Args) -> Self {
        Self {
            input_dir: a.input_dir.clone(),
            output_dir: a.output_dir.clone(),
            name: a.name.clone(),
            title: a.title.clone(),
            width: a.width,
            height: a.height,
            dev_view: a.view,
        }
    }
}

// ── 入口 ──

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Compile(a) => {
            cmd_compile(&a);
        }
        Command::Run(a) => cmd_run(&a),
    }
}

// ── 子命令 ──

fn cmd_compile(args: &Args) -> compiler::ResolvedConfig {
    match compiler::compile_project_to_dir(args.into()) {
        Ok(out) => {
            eprintln!(
                "Done. Run with: cargo run --manifest-path {}/Cargo.toml",
                out.output_dir.display()
            );
            dbg!(&out);
            out
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

fn cmd_run(args: &Args) {
    let out = cmd_compile(args);

    eprintln!("Running...");

    let status = StdCommand::new("cargo")
        .args(["run", "--manifest-path"])
        .arg(out.output_dir.join("Cargo.toml"))
        .status()
        .unwrap_or_else(|e| {
            eprintln!("error: cargo run failed: {e}");
            process::exit(1);
        });

    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

#[cfg(test)]
#[path = "test/main.test.rs"]
mod tests;
