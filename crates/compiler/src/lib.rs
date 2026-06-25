//! # compiler — HTML+CSS+JS → Rust 编译器

pub mod html;
pub mod css;
pub mod js;
pub mod parser;
pub mod analyzer;
pub mod codegen;
pub mod builtins;
pub mod canvas_codegen;
pub mod config;
pub mod variable;
pub mod pipeline;
pub mod resolve;

use std::fs;
use std::path::PathBuf;

// ── re-export ──

pub use html::HtmlElement;
pub use parser::{Parser, CompilationUnit, SourceFile, SourceKind};
pub use analyzer::{Analyzer, AnalysisResult, VariableInfo, VariableKind, FunctionCall, DomOperation};
pub use builtins::{
    BuiltinMapping, all_builtins, lookup_builtin,
    typed_array_mappings, async_mappings, class_mappings, phase2_builtins,
    object_mappings, history_mappings, location_mappings,
    url_mappings, date_mappings, phase3_builtins,
};
pub use canvas_codegen::{
    canvas_mappings, compile_canvas_call, is_canvas_call,
};
pub use config::{CompileInput, ResolvedConfig};
pub use codegen::MultiFileOutput;

// ── 项目级编译 ──

/// 编译项目并写入输出目录。
///
/// 输出结构：
/// ```text
/// output/
///   src/
///     main.rs    ← 入口（使用 DomRegistry + RuntimeStyleManager）
///     html.rs    ← VNode 树构造
///     style.rs   ← CSS 样式注册（文件名同源文件 stem）
///     app.rs     ← JS 事件处理器（文件名同源文件 stem）
///   Cargo.toml
/// ```
pub fn compile_project_to_dir(input: CompileInput) -> Result<ResolvedConfig, String> {
    let resolved = config::resolve(&input)?;
    let output = compile_project_split(&resolved)?;

    let output_dir = &resolved.output_dir;
    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)
        .map_err(|e| format!("cannot create {}: {e}", src_dir.display()))?;

    // 写各模块文件
    fs::write(src_dir.join("main.rs"), &output.main_rs)
        .map_err(|e| format!("cannot write main.rs: {e}"))?;
    fs::write(src_dir.join("html.rs"), &output.html_rs)
        .map_err(|e| format!("cannot write html.rs: {e}"))?;
    for (stem, content) in &output.style_files {
        fs::write(src_dir.join(format!("{}.rs", stem)), content)
            .map_err(|e| format!("cannot write {}.rs: {e}", stem))?;
    }
    for (stem, content) in &output.handler_files {
        fs::write(src_dir.join(format!("{}.rs", stem)), content)
            .map_err(|e| format!("cannot write {}.rs: {e}", stem))?;
    }

    // 写 Cargo.toml
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let dom_flat_rel = resolve::rel_path(
        output_dir,
        &workspace_root.join("crates").join("runtime").join("dom_flat"),
    );
    let style_runtime_rel = resolve::rel_path(
        output_dir,
        &workspace_root.join("crates").join("runtime").join("style_runtime"),
    );
    let cargo_toml = format!(
        "[workspace]\n\
         \n\
         [package]\n\
         name = \"{}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         \n\
         [dependencies]\n\
         dom_flat = {{ path = \"{dom_flat_rel}\" }}\n\
         style_runtime = {{ path = \"{style_runtime_rel}\" }}\n",
        resolved.name,
    );
    fs::write(output_dir.join("Cargo.toml"), &cargo_toml)
        .map_err(|e| format!("cannot write Cargo.toml: {e}"))?;

    Ok(resolved)
}

// ── 内部 ──

fn compile_project_split(resolved: &config::ResolvedConfig) -> Result<codegen::MultiFileOutput, String> {
    let html_path = resolve::find_html_file(&resolved.input_dir, &resolved.entry)?;
    let html_src = fs::read_to_string(&html_path)
        .map_err(|e| format!("cannot read {}: {e}", html_path.display()))?;

    let resources = html::parse_html_document(&html_src, &resolved.input_dir);

    Ok(pipeline::compile_resources(
        &resources,
        &resolved.title,
        resolved.width,
        resolved.height,
    ))
}
