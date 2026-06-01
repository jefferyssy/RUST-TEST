//! toolchain CLI — 将 HTML+CSS+JS 前端项目编译为 Rust 原生应用
//!
//! 这是整个工具链的命令行入口，提供两个核心子命令：
//!
//! ## 用法
//! ```text
//! toolchain compile <input-dir> [-o <output-dir>] [--name <name>] [--title <title>] [--width <w>] [--height <h>]
//! toolchain run <input-dir> [-o <output-dir>] [--name <name>] [--title <title>]
//! ```
//!
//! ## 子命令说明
//! - **compile**: 读取输入目录中的 HTML/CSS/JS 源文件，调用 `toolchain` 库生成 Rust 代码，
//!   并创建完整的 Cargo 项目（含 Cargo.toml 和 main.rs），输出到指定目录。
//! - **run**: 先执行 compile，然后自动调用 `cargo run` 编译并运行生成的 Rust 项目。
//!
//! ## 数据流
//! ```text
//! index.html + style.css + app.js  →  toolchain 库  →  生成的 Rust 代码
//!                                                           ↓
//!                                              Cargo.toml + src/main.rs
//!                                                           ↓
//!                                                    cargo build/run
//!                                                           ↓
//!                                                     原生二进制
//! ```

use std::env;       // 读取命令行参数和环境变量
use std::fs;         // 文件系统操作（创建目录、写入文件）
use std::path::{Path, PathBuf};  // 路径处理
use std::process::{self, Command}; // 进程退出和子进程执行（cargo run）

// ────────────────────────────────────────────────────────────────
// 命令行参数结构体
// ────────────────────────────────────────────────────────────────

/// 解析后的命令行参数，包含编译/运行所需的全部配置信息。
struct Args {
    /// 子命令类型：compile 或 run
    command: CommandType,
    /// 输入目录路径，包含 index.html、style.css、app.js 三个源文件
    input_dir: PathBuf,
    /// 输出目录路径，生成的 Cargo 项目将放在此处（默认 target/generated/<name>）
    output_dir: PathBuf,
    /// 项目名称，同时用作 Cargo package name 和输出目录名
    name: String,
    /// 窗口标题，会传递给生成的代码，显示在窗口标题栏
    title: String,
    /// 窗口宽度（像素），默认 800
    width: u32,
    /// 窗口高度（像素），默认 600
    height: u32,
}

/// CLI 支持的两个子命令。
enum CommandType {
    /// `toolchain compile` — 仅生成代码，不运行
    Compile,
    /// `toolchain run` — 生成代码后自动 cargo run
    Run,
}

// ────────────────────────────────────────────────────────────────
// 入口函数
// ────────────────────────────────────────────────────────────────

/// CLI 主入口。
///
/// 流程：
/// 1. 调用 `parse_args()` 解析命令行参数
/// 2. 根据子命令类型分发到 `cmd_compile` 或 `cmd_run`
fn main() {
    // 解析命令行参数，失败时打印用法信息并退出
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!(
                "usage: toolchain compile <input-dir> [-o <output-dir>] [--name <name>] [--title <title>] [--width <w>] [--height <h>]"
            );
            eprintln!("       toolchain run <input-dir> [-o <output-dir>] [--name <name>] [--title <title>]");
            process::exit(1); // 非零退出码表示失败
        }
    };

    // 根据子命令类型分发
    match args.command {
        CommandType::Compile => cmd_compile(&args),
        CommandType::Run => cmd_run(&args),
    }
}

// ────────────────────────────────────────────────────────────────
// 命令行参数解析
// ────────────────────────────────────────────────────────────────

/// 解析命令行参数并返回结构化的 `Args`。
///
/// 位置参数：
/// - 第 1 个参数（索引 1）：子命令名 compile / run
/// - 第 2 个参数（索引 2）：输入目录路径（必需的，除非被误判为选项值）
///
/// 可选参数：
/// - `-o` / `--output <dir>`: 输出目录
/// - `--name <name>`: 项目名称（默认取输入目录名）
/// - `--title <title>`: 窗口标题（默认 "Demo"）
/// - `--width <w>`: 窗口宽度（默认 800）
/// - `--height <h>`: 窗口高度（默认 600）
///
/// 返回 `Err(String)` 在以下情况：缺少命令、未知命令、缺少必填参数值、意外的参数。
fn parse_args() -> Result<Args, String> {
    // 获取所有命令行原始参数（包括程序名本身）
    let raw: Vec<String> = env::args().collect();

    // 至少需要程序名 + 子命令 = 2 个参数
    if raw.len() < 2 {
        return Err("missing command".into());
    }

    // ── 解析子命令（索引 1） ──
    let command = match raw[1].as_str() {
        "compile" => CommandType::Compile,
        "run" => CommandType::Run,
        other => return Err(format!("unknown command: {other}")),
    };

    // ── 初始化各字段的默认值 ──
    let mut input_dir: Option<PathBuf> = None;   // 输入目录（必填，解析过程中赋值）
    let mut output_dir: Option<PathBuf> = None;   // 输出目录（可选，有默认值）
    let mut name: Option<String> = None;           // 项目名（可选，默认取输入目录名）
    let mut title = "Demo".to_string();            // 窗口标题（可选，默认 "Demo"）
    let mut width = 800u32;                        // 窗口宽度（可选，默认 800px）
    let mut height = 600u32;                       // 窗口高度（可选，默认 600px）

    // ── 遍历剩余参数（从索引 2 开始） ──
    let mut i = 2;
    while i < raw.len() {
        match raw[i].as_str() {
            // -o / --output：指定输出目录路径
            "-o" | "--output" => {
                i += 1; // 消费当前标志，移动到下一个参数（值）
                if i >= raw.len() {
                    return Err("missing value for -o".into());
                }
                output_dir = Some(PathBuf::from(&raw[i]));
            }
            // --name：指定项目名称
            "--name" => {
                i += 1;
                if i >= raw.len() {
                    return Err("missing value for --name".into());
                }
                name = Some(raw[i].clone());
            }
            // --title：指定窗口标题
            "--title" => {
                i += 1;
                if i >= raw.len() {
                    return Err("missing value for --title".into());
                }
                title = raw[i].clone();
            }
            // --width：指定窗口宽度（像素值，需可解析为 u32）
            "--width" => {
                i += 1;
                if i >= raw.len() {
                    return Err("missing value for --width".into());
                }
                width = raw[i].parse().map_err(|_| "invalid --width")?;
            }
            // --height：指定窗口高度（像素值，需可解析为 u32）
            "--height" => {
                i += 1;
                if i >= raw.len() {
                    return Err("missing value for --height".into());
                }
                height = raw[i].parse().map_err(|_| "invalid --height")?;
            }
            // 非选项参数且尚未设置输入目录 → 视为输入目录路径
            arg if !arg.starts_with('-') && input_dir.is_none() => {
                input_dir = Some(PathBuf::from(arg));
            }
            // 其他无法识别的参数 → 报错
            other => return Err(format!("unexpected argument: {other}")),
        }
        i += 1; // 移至下一个参数
    }

    // ── 校验必填参数并计算默认值 ──
    let input_dir = input_dir.ok_or("missing input directory")?;

    // 项目名：优先用 --name，否则取输入目录的文件夹名，都不行则默认 "app"
    let name = name.unwrap_or_else(|| {
        input_dir
            .file_name()                           // 提取目录路径的最后一段（文件夹名）
            .map(|n| n.to_string_lossy().to_string())  // OsStr → String
            .unwrap_or_else(|| "app".into())        // 兜底值
    });

    // 输出目录：优先用 -o，默认 target/generated/<项目名>
    let output_dir = output_dir.unwrap_or_else(|| PathBuf::from("target/generated").join(&name));

    // ── 构造并返回 Args ──
    Ok(Args {
        command,
        input_dir,
        output_dir,
        name,
        title,
        width,
        height,
    })
}

// ────────────────────────────────────────────────────────────────
// 子命令实现
// ────────────────────────────────────────────────────────────────

/// `toolchain compile` 子命令的处理函数。
///
/// 执行步骤：
/// 1. 获取当前工作目录（用于计算 Cargo.toml 中的相对路径依赖）
/// 2. 检查输入目录中是否存在 HTML/CSS/JS 源文件
/// 3. 调用 `toolchain::compile_with_options()` 生成 Rust 源代码
/// 4. 创建输出目录结构
/// 5. 写入 `src/main.rs`（生成的 Rust 代码）
/// 6. 写入 `Cargo.toml`（包含对 workspace 中 renderer/dom crate 的路径依赖）
/// 7. 打印运行指引
fn cmd_compile(args: &Args) {
    // 获取当前工作目录作为 workspace 根目录的参考
    let workspace_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    eprintln!(
        "Compiling {} from {:?} -> {:?}",
        args.name, args.input_dir, args.output_dir
    );

    // ── 步骤 1：找到 HTML 入口文件 ──
    // 优先使用 index.html，否则扫描输入目录中第一个 .html 文件
    let html_path = {
        let default_html = args.input_dir.join("index.html");
        if default_html.exists() {
            default_html
        } else {
            match find_file_by_ext(&args.input_dir, "html") {
                Some(p) => p,
                None => {
                    eprintln!("error: no .html file found in {}", args.input_dir.display());
                    process::exit(1);
                }
            }
        }
    };
    eprintln!("  HTML: {}", html_path.display());

    // ── 步骤 2：从 HTML 中提取 CSS/JS 文件引用 ──
    let html_src = fs::read_to_string(&html_path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", html_path.display());
        process::exit(1);
    });
    let (css_refs, js_refs) = toolchain::html::extract_references(&html_src);

    // ── 步骤 3：解析 CSS/JS 文件路径（相对于 HTML 所在目录） ──
    let css_path = resolve_ref(&args.input_dir, &css_refs, "CSS");
    let js_path = resolve_ref(&args.input_dir, &js_refs, "JS");

    if let Some(ref p) = css_path {
        eprintln!("  CSS: {}", p.display());
    }
    if let Some(ref p) = js_path {
        eprintln!("  JS: {}", p.display());
    }

    // ── 步骤 4：调用 toolchain 库生成 Rust 源代码 ──
    let opts = toolchain::CompileOptions {
        title: args.title.clone(),
        width: args.width,
        height: args.height,
    };
    let css_path_str = css_path.as_ref().map_or(String::new(), |p| p.to_string_lossy().to_string());
    let js_path_str = js_path.as_ref().map_or(String::new(), |p| p.to_string_lossy().to_string());
    let rust_code = toolchain::compile_with_options(
        &html_path.to_string_lossy(),
        &css_path_str,
        &js_path_str,
        &opts,
    );

    // ── 步骤 3：创建输出目录 src/ ──
    let src_dir = args.output_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap_or_else(|e| {
        eprintln!("error: cannot create {}: {e}", src_dir.display());
        process::exit(1);
    });

    // ── 步骤 4：写入 src/main.rs ──
    fs::write(src_dir.join("main.rs"), &rust_code).unwrap_or_else(|e| {
        eprintln!("error: cannot write main.rs: {e}");
        process::exit(1);
    });
    eprintln!("  wrote {}", src_dir.join("main.rs").display());

    // ── 步骤 5：计算依赖 crate 的相对路径 ──
    // 生成的 Cargo.toml 需要通过相对路径引用 workspace 中的 renderer 和 dom crate
    let renderer_rel = rel_path(&args.output_dir, &workspace_root.join("crates").join("renderer"));
    let dom_rel = rel_path(&args.output_dir, &workspace_root.join("crates").join("dom"));

    // ── 步骤 6：生成并写入 Cargo.toml ──
    // 使用 [workspace] 声明使其成为独立的 workspace 成员，避免与父 workspace 冲突
    let cargo_toml = format!(
        "[workspace]\n\
         \n\
         [package]\n\
         name = \"{}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         \n\
         [dependencies]\n\
         renderer = {{ path = \"{}\" }}\n\
         dom = {{ path = \"{}\" }}\n",
        args.name, renderer_rel, dom_rel,
    );
    fs::write(args.output_dir.join("Cargo.toml"), &cargo_toml).unwrap_or_else(|e| {
        eprintln!("error: cannot write Cargo.toml: {e}");
        process::exit(1);
    });
    eprintln!("  wrote {}", args.output_dir.join("Cargo.toml").display());

    // ── 步骤 7：打印运行提示 ──
    eprintln!(
        "Done. Run with: cargo run --manifest-path {}/Cargo.toml",
        args.output_dir.display()
    );
}

/// `toolchain run` 子命令的处理函数。
///
/// 先调用 `cmd_compile` 生成项目，再通过 `cargo run` 编译并执行生成的 Rust 代码。
/// 如果 cargo run 失败或返回非零退出码，则将该退出码向上传递。
fn cmd_run(args: &Args) {
    // 先执行编译步骤（复用 cmd_compile）
    cmd_compile(args);

    eprintln!("Running...");

    // 通过 Command 调用子进程 `cargo run --manifest-path <output_dir>/Cargo.toml`
    let status = Command::new("cargo")
        .args(["run", "--manifest-path"])
        .arg(args.output_dir.join("Cargo.toml"))
        .status()
        .unwrap_or_else(|e| {
            eprintln!("error: cargo run failed: {e}");
            process::exit(1);
        });

    // 如果 cargo run 返回非零退出码，则用相同的退出码终止当前进程
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

// ────────────────────────────────────────────────────────────────
// 工具函数
// ────────────────────────────────────────────────────────────────

/// 在目录中按扩展名查找第一个匹配的文件。
///
/// 仅检查文件（跳过目录和隐藏文件），按文件名排序后返回第一个匹配项。
fn find_file_by_ext(dir: &Path, ext: &str) -> Option<PathBuf> {
    let mut matches: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().map(|e| e.to_str()) == Some(Some(ext)))
        .collect();
    matches.sort();
    matches.into_iter().next()
}

/// 从 HTML 提取的文件引用列表中解析第一个存在的文件路径。
///
/// 依次尝试 `input_dir/ref_path` 是否存在，返回第一个找到的。
/// 若列表为空则发出警告；若全部不存在则发出警告并返回 `None`。
fn resolve_ref(input_dir: &Path, refs: &[String], label: &str) -> Option<PathBuf> {
    if refs.is_empty() {
        eprintln!("warning: no {label} file referenced in HTML");
        return None;
    }
    for r in refs {
        let candidate = input_dir.join(r);
        if candidate.exists() {
            return Some(candidate);
        }
        eprintln!("warning: {label} file not found: {}", candidate.display());
    }
    eprintln!("warning: no {label} file found for references: {:?}", refs);
    None
}

/// 计算从一个目录到另一个路径的相对路径（始终使用正斜杠）。
///
/// 用于在生成的 Cargo.toml 中写入正确的 path 依赖——因为输出目录可能在任意位置，
/// 需要计算出回到 workspace 中 `crates/renderer` 和 `crates/dom` 的相对路径。
///
/// ## 参数
/// - `from`: 起始目录路径（即生成的 Cargo.toml 所在目录）
/// - `to`: 目标路径（workspace 中的 crate 目录）
///
/// ## 实现策略
/// 1. 优先对两个路径做 `canonicalize()` 获取绝对路径进行比较
/// 2. 如果 `from` 目录尚不存在（编译前输出目录尚未创建），则：
///    - 尝试规范化父目录，再拼接目录名
///    - 回退到原始的相对路径
/// 3. 找出两个绝对路径的公共前缀，计算需要 `../` 回溯的层数
/// 4. 拼接剩余路径段，统一为正斜杠格式
///
/// ## 示例
/// ```text
/// rel_path("target/generated/counter", "crates/renderer")
/// → "../../../crates/renderer"
/// ```
fn rel_path(from: &Path, to: &Path) -> String {
    // ── 规范化 `from` 路径为绝对路径 ──
    // canonicalize 要求路径已存在，如果 from 目录尚未创建则逐步回退
    let from_abs = from.canonicalize().unwrap_or_else(|_| {
        // 目录不存在时，尝试规范化父目录然后拼接最后一段路径
        if let Some(parent) = from.parent() {
            if let Ok(parent_abs) = parent.canonicalize() {
                return parent_abs.join(from.file_name().unwrap_or_default());
            }
        }
        // 彻底失败时直接用原始路径
        from.to_path_buf()
    });

    // ── 规范化 `to` 路径为绝对路径 ──
    let to_abs = to.canonicalize().unwrap_or_else(|_| to.to_path_buf());

    // ── 将路径拆解为组件列表以便比较 ──
    let from_parts: Vec<_> = from_abs.components().collect();
    let to_parts: Vec<_> = to_abs.components().collect();

    // ── 找出公共前缀的长度 ──
    let common = from_parts
        .iter()
        .zip(to_parts.iter())
        .take_while(|(a, b)| a == b)  // 逐组件比较，直到发现不同
        .count();

    // ── 构造相对路径 ──
    // 需要向上回溯的层数 = from 的总组件数 - 公共前缀数
    let up_count = from_parts.len() - common;
    let mut result = String::new();

    // 拼接 `../` 回到公共祖先
    for _ in 0..up_count {
        result.push_str("../");
    }

    // 拼接从公共祖先到目标路径的剩余部分
    for comp in &to_parts[common..] {
        result.push_str(&comp.as_os_str().to_string_lossy());
        result.push('/');
    }

    // 移除末尾多余的斜杠（如果从根目录算起，最后会多一个 `/`）
    if result.ends_with('/') {
        result.pop();
    }

    // ── 统一使用正斜杠（Cargo.toml 和跨平台兼容） ──
    result.replace('\\', "/")
}
