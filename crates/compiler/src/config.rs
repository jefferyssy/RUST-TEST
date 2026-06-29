//! 项目配置：文件解析 + 入参合并。
//!
//! # 优先级链
//!
//! ```text
//! CLI 显式传参  >  .ruft.toml  >  .ruft.json  >  硬编码默认值
//! ```
//!
//! # 使用
//!
//! ```rust,ignore
//! let resolved = config::resolve(&input)?;
//! // resolved 中所有字段已确定，不再含 Option
//! ```

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════════════════════════════
// 入参 —— 调用方传入的原始配置
// ═══════════════════════════════════════════════════════════════════════

/// CLI / build.rs 传入的原始参数。
///
/// `Option` 字段表示未指定，最终由 [`resolve`] 按优先级解析。
#[derive(Debug, Clone)]
pub struct CompileInput {
    pub input_dir: PathBuf,
    pub output_dir: Option<PathBuf>, // None → input_dir/target
    pub name: Option<String>,       // None → 目录名
    pub title: Option<String>,      // None → "Demo"
    pub width: Option<u32>,         // None → 800
    pub height: Option<u32>,        // None → 600
    pub dev_view: bool,             // 启用运行时可视化仪表盘
}

// ═══════════════════════════════════════════════════════════════════════
// 配置文件 —— .ruft.toml / .ruft.json 的 Rust 表示
// ═══════════════════════════════════════════════════════════════════════

/// 项目配置文件的反序列化结果。
///
/// 所有子段均为可选，`None` 表示未在文件中指定。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default)] pub package: PackageConfig,
    #[serde(default)] pub window: WindowConfig,
    #[serde(default)] pub output: OutputConfig,
    #[serde(default)] pub html: HtmlConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PackageConfig {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WindowConfig {
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OutputConfig {
    pub dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HtmlConfig {
    pub entry: Option<String>,
}

/// 从项目目录读取配置文件。
///
/// 查找顺序：`.ruft.toml` > `.ruft.json`，都不存在返回全 `None` 的默认值。
pub fn read_project_config(dir: &Path) -> Result<ProjectConfig, String> {
    let toml_path = dir.join(".ruft.toml");
    let json_path = dir.join(".ruft.json");

    if toml_path.exists() {
        let content = fs::read_to_string(&toml_path)
            .map_err(|e| format!("cannot read {}: {e}", toml_path.display()))?;
        return toml::from_str(&content).map_err(|e| format!("invalid .ruft.toml: {e}"));
    }
    if json_path.exists() {
        let content = fs::read_to_string(&json_path)
            .map_err(|e| format!("cannot read {}: {e}", json_path.display()))?;
        return serde_json::from_str(&content).map_err(|e| format!("invalid .ruft.json: {e}"));
    }

    Ok(ProjectConfig::default())
}

// ═══════════════════════════════════════════════════════════════════════
// 出参 —— 已解析的最终配置
// ═══════════════════════════════════════════════════════════════════════

/// 合并后的最终配置，所有字段已确定。
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub name: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub entry: String,
    pub dev_view: bool,
}

/// 三步覆盖，产出 [`ResolvedConfig`]。
///
/// ```text
/// 1. 设置默认值
/// 2. 配置文件覆盖（.ruft.toml / .ruft.json）
/// 3. CLI 入参覆盖（最高优先级）
/// ```
pub fn resolve(input: &CompileInput) -> Result<ResolvedConfig, String> {
    let cfg = read_project_config(&input.input_dir)?;

    // ══ 步骤 1: 默认值 ══
    let default_name = input
        .input_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("app"));

    let mut r = ResolvedConfig {
        input_dir: input.input_dir.clone(),
        output_dir: input.input_dir.join("generated"),
        name: default_name,
        title: String::from("Demo"),
        width: 800,
        height: 600,
        entry: String::from("index.html"),
        dev_view: input.dev_view,
    };

    // ══ 步骤 2: 配置文件覆盖 ══
    if let Some(v) = cfg.package.name.clone() { r.name = v; }
    if let Some(v) = cfg.window.title.clone() { r.title = v; }
    if let Some(v) = cfg.window.width         { r.width = v; }
    if let Some(v) = cfg.window.height        { r.height = v; }
    if let Some(v) = cfg.output.dir.clone()   { r.output_dir = PathBuf::from(v); }
    if let Some(v) = cfg.html.entry           { r.entry = v; }

    // ══ 步骤 3: CLI 覆盖（最高优先级） ══
    if let Some(v) = input.name.clone()       { r.name = v; }
    if let Some(v) = input.title.clone()      { r.title = v; }
    if let Some(v) = input.width              { r.width = v; }
    if let Some(v) = input.height             { r.height = v; }
    if let Some(v) = input.output_dir.clone() { r.output_dir = v; }

    Ok(r)
}

#[cfg(test)]
#[path = "../test/config.test.rs"]
mod tests;
