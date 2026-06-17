//! 文件查找与路径解析工具。

use std::path::{Path, PathBuf};

/// 获取 workspace 根目录（当前工作目录）。
pub fn workspace_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 在项目目录中定位 HTML 入口文件。
pub fn find_html_file(dir: &Path, entry: &str) -> Result<PathBuf, String> {
    let path = dir.join(entry);
    if path.exists() {
        return Ok(path);
    }
    Err(format!("entry '{}' not found in {}", entry, dir.display()))
}

/// 计算从 `from` 目录到 `to` 路径的相对路径（始终正斜杠）。
pub fn rel_path(from: &Path, to: &Path) -> String {
    let from_abs = from.canonicalize().unwrap_or_else(|_| {
        if let Some(parent) = from.parent() {
            if let Ok(parent_abs) = parent.canonicalize() {
                return parent_abs.join(from.file_name().unwrap_or_default());
            }
        }
        from.to_path_buf()
    });

    let to_abs = to.canonicalize().unwrap_or_else(|_| to.to_path_buf());

    let from_parts: Vec<_> = from_abs.components().collect();
    let to_parts: Vec<_> = to_abs.components().collect();

    let common = from_parts.iter().zip(to_parts.iter()).take_while(|(a, b)| a == b).count();
    let up_count = from_parts.len() - common;

    let mut result = String::new();
    for _ in 0..up_count {
        result.push_str("../");
    }
    for comp in &to_parts[common..] {
        result.push_str(&comp.as_os_str().to_string_lossy());
        result.push('/');
    }
    if result.ends_with('/') {
        result.pop();
    }

    result.replace('\\', "/")
}
