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
//!
//! ## 职责边界
//! CLI 仅负责**参数解析**和**命令分发**。编译逻辑（找文件、解析引用、代码生成、
//! 文件写入、Cargo.toml 生成）全部封装在 [`compiler`] crate 中。
//!
//! ## 数据流
//! ```text
//! 命令行参数 → clap 解析 → Args → compiler::compile_project_to_dir
//!                                         ↓
//!                             Cargo.toml + src/main.rs
//!                                         ↓
//!                                   cargo run
//! ```

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::{self, Command as StdCommand};

// ── clap 命令行定义 ──

/// 将 HTML+CSS+JS 前端项目编译为 Rust 原生应用。
///
/// 通过 clap derive 自动生成 `--help` 和 `--version` 输出。
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
///
/// 所有 `Option` 字段直接透传给 compiler，由 compiler 按 `CLI > .ruft.* > 硬编码` 优先级解析。
/// CLI 层不做任何默认值处理。
#[derive(clap::Args, Debug, Clone)]
struct Args {
    /// 输入目录路径，应包含 `index.html`、`style.css`、`app.js` 三个源文件。
    input_dir: PathBuf,

    /// 输出目录路径。不指定时由 compiler 根据 name 推导。
    #[arg(short = 'o', long)]
    output_dir: Option<PathBuf>,

    /// 项目名称，用作 Cargo package name。不指定时由配置文件或目录名兜底。
    #[arg(long)]
    name: Option<String>,

    /// 窗口标题，显示在窗口标题栏。不指定时由配置文件或默认值兜底。
    #[arg(long)]
    title: Option<String>,

    /// 窗口宽度（像素）。不指定时由配置文件或默认值兜底。
    #[arg(long)]
    width: Option<u32>,

    /// 窗口高度（像素）。不指定时由配置文件或默认值兜底。
    #[arg(long)]
    height: Option<u32>,
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

/// `cli compile` — 委托给 [`compiler::compile_project_to_dir`]。
///
/// 所有默认值由 compiler 按优先级解析。返回整合后的配置集合。
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

/// `cli run` — 先 compile 再 `cargo run`。
///
/// 复用 [`cmd_compile`] 生成项目，然后通过 `StdCommand` 调用
/// `cargo run --manifest-path <output_dir>/Cargo.toml`。
/// cargo 返回非零退出码时，原样向上传递。
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
