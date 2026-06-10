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
//! 命令行参数 → clap 解析 → BuildArgs → Args（默认值解析）→ compiler::compile_project_to_dir
//!                                                                     ↓
//!                                                         Cargo.toml + src/main.rs
//!                                                                     ↓
//!                                                               cargo run
//! ```

use clap::{Parser, Subcommand};
use std::env;
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
    ///
    /// 读取 `<input-dir>` 中的 HTML/CSS/JS 源文件，生成完整的 Cargo 项目
    /// （`main.rs` + `Cargo.toml`）到 `--output-dir`（默认 `target/generated/<name>`）。
    Compile(BuildArgs),
    /// 生成代码后自动调用 `cargo run` 编译并运行。
    Run(BuildArgs),
}

/// `compile` 和 `run` 共用的构建参数。
///
/// 静态默认值（`title`、`width`、`height`）由 clap 处理；
/// 动态默认值（`name` 取自 `input_dir` 文件夹名、`output_dir` 基于 `name`）
/// 在 [`Args::from_build`] 中解析。
#[derive(clap::Args, Debug)]
struct BuildArgs {
    /// 输入目录路径，应包含 `index.html`、`style.css`、`app.js` 三个源文件。
    input_dir: PathBuf,

    /// 输出目录路径。默认 `target/generated/<项目名>`。
    #[arg(short = 'o', long)]
    output_dir: Option<PathBuf>,

    /// 项目名称，用作 Cargo package name 和输出目录名。默认取输入目录文件夹名。
    #[arg(long)]
    name: Option<String>,

    /// 窗口标题，显示在窗口标题栏。
    #[arg(long, default_value = "Demo")]
    title: String,

    /// 窗口宽度（像素）。
    #[arg(long, default_value_t = 800)]
    width: u32,

    /// 窗口高度（像素）。
    #[arg(long, default_value_t = 600)]
    height: u32,
}

// ── 内部参数（默认值已解析） ──

/// 解析后的命令行参数，所有动态默认值已计算完毕。
///
/// 与 [`BuildArgs`] 的区别：`name` 和 `output_dir` 在这里已经是确定的非 `Option` 值，
/// 下游代码无需再处理 `None` 情况。
struct Args {
    /// 输入目录路径。
    input_dir: PathBuf,
    /// 输出目录路径（已解析默认值）。
    output_dir: PathBuf,
    /// 项目名称（已解析默认值）。
    name: String,
    /// 窗口标题。
    title: String,
    /// 窗口宽度（像素）。
    width: u32,
    /// 窗口高度（像素）。
    height: u32,
}

impl Args {
    /// 从 clap 解析结果构造，处理动态默认值：
    /// - `name`: `None` → 取 `input_dir` 的文件夹名，兜底 `"app"`
    /// - `output_dir`: `None` → `target/generated/<name>`
    fn from_build(a: BuildArgs) -> Self {
        let name = a.name.unwrap_or_else(|| {
            a.input_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "app".into())
        });
        let output_dir = a
            .output_dir
            .unwrap_or_else(|| PathBuf::from("target/generated").join(&name));
        Self {
            input_dir: a.input_dir,
            output_dir,
            name,
            title: a.title,
            width: a.width,
            height: a.height,
        }
    }
}

// ── 入口 ──

/// CLI 主入口。
///
/// 1. 通过 clap 解析命令行参数
/// 2. `BuildArgs` → [`Args::from_build`] 解析动态默认值
/// 3. 根据子命令分发到 [`cmd_compile`] 或 [`cmd_run`]
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Compile(a) => cmd_compile(&Args::from_build(a)),
        Command::Run(a) => cmd_run(&Args::from_build(a)),
    }
}

// ── 子命令 ──

/// `cli compile` — 委托给 [`compiler::compile_project_to_dir`]。
///
/// 构造 [`compiler::CompileOptions`]，以当前工作目录为 workspace 根，
/// 将编译和文件输出全部交给 compiler crate 完成。
fn cmd_compile(args: &Args) {
    let workspace_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let opts = compiler::CompileOptions {
        title: args.title.clone(),
        width: args.width,
        height: args.height,
    };

    eprintln!(
        "Compiling {} from {:?} -> {:?}",
        args.name, args.input_dir, args.output_dir
    );

    if let Err(e) = compiler::compile_project_to_dir(
        &args.input_dir,
        &args.output_dir,
        &args.name,
        &workspace_root,
        &opts,
    ) {
        eprintln!("error: {e}");
        process::exit(1);
    }

    eprintln!(
        "Done. Run with: cargo run --manifest-path {}/Cargo.toml",
        args.output_dir.display()
    );
}

/// `cli run` — 先 compile 再 `cargo run`。
///
/// 复用 [`cmd_compile`] 生成项目，然后通过 `StdCommand` 调用
/// `cargo run --manifest-path <output_dir>/Cargo.toml`。
/// cargo 返回非零退出码时，原样向上传递。
fn cmd_run(args: &Args) {
    cmd_compile(args);

    eprintln!("Running...");

    let status = StdCommand::new("cargo")
        .args(["run", "--manifest-path"])
        .arg(args.output_dir.join("Cargo.toml"))
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
