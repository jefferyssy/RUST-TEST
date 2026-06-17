//! 配置文件解析测试。

use super::*;
use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

static DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 创建独立的临时目录并写入文件，执行测试后自动清理。
fn with_temp_dir<F>(files: &[(&str, &str)], f: F)
where
    F: FnOnce(&Path),
{
    let n = DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = Path::new("target/test-tmp").join(format!("config-test-{n}"));
    // 确保干净状态
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    for (name, content) in files {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).expect("create temp file");
        file.write_all(content.as_bytes()).expect("write temp file");
    }
    f(&dir);
    let _ = fs::remove_dir_all(&dir);
}

// ── 文件不存在 → 默认配置 ──

#[test]
fn no_config_file_returns_default() {
    with_temp_dir(&[], |dir| {
        let config = read_project_config(dir).expect("should succeed");
        assert_eq!(config.package.name, None);
        assert_eq!(config.window.title, None);
        assert_eq!(config.window.width, None);
        assert_eq!(config.window.height, None);
        assert_eq!(config.html.entry, None);
    });
}

// ── TOML 解析 ──

#[test]
fn toml_full_config() {
    let toml_content = "\
[package]\n\
name = \"my-app\"\n\
\n\
[window]\n\
title = \"My App\"\n\
width = 640\n\
height = 480\n\
\n\
[html]\n\
entry = \"main.html\"\n\
";
    with_temp_dir(&[(".ruft.toml", toml_content)], |dir| {
        let config = read_project_config(dir).expect("should parse toml");
        assert_eq!(config.package.name.as_deref(), Some("my-app"));
        assert_eq!(config.window.title.as_deref(), Some("My App"));
        assert_eq!(config.window.width, Some(640));
        assert_eq!(config.window.height, Some(480));
        assert_eq!(config.html.entry.as_deref(), Some("main.html"));
    });
}

#[test]
fn toml_partial_config() {
    let toml_content = "\
[package]\n\
name = \"partial\"\n\
\n\
[window]\n\
title = \"Partial\"\n\
";
    with_temp_dir(&[(".ruft.toml", toml_content)], |dir| {
        let config = read_project_config(dir).expect("should parse toml");
        assert_eq!(config.package.name.as_deref(), Some("partial"));
        assert_eq!(config.window.title.as_deref(), Some("Partial"));
        assert_eq!(config.window.width, None);
        assert_eq!(config.window.height, None);
        assert_eq!(config.html.entry, None);
    });
}

#[test]
fn toml_empty_file() {
    with_temp_dir(&[(".ruft.toml", "")], |dir| {
        let config = read_project_config(dir).expect("should parse empty toml");
        assert_eq!(config.package.name, None);
        assert_eq!(config.window.title, None);
    });
}

#[test]
fn toml_invalid_syntax() {
    with_temp_dir(&[(".ruft.toml", "not valid toml {{{")], |dir| {
        let err = read_project_config(dir).unwrap_err();
        assert!(err.contains("invalid .ruft.toml"), "got: {err}");
    });
}

// ── JSON 解析 ──

#[test]
fn json_full_config() {
    let json_content = r#"{"package":{"name":"json-app"},"window":{"title":"JSON App","width":320,"height":240},"html":{"entry":"index.htm"}}"#;
    with_temp_dir(&[(".ruft.json", json_content)], |dir| {
        let config = read_project_config(dir).expect("should parse json");
        assert_eq!(config.package.name.as_deref(), Some("json-app"));
        assert_eq!(config.window.title.as_deref(), Some("JSON App"));
        assert_eq!(config.window.width, Some(320));
        assert_eq!(config.window.height, Some(240));
        assert_eq!(config.html.entry.as_deref(), Some("index.htm"));
    });
}

#[test]
fn json_partial_config() {
    let json_content = r#"{"package":{},"window":{"title":"Only Title"}}"#;
    with_temp_dir(&[(".ruft.json", json_content)], |dir| {
        let config = read_project_config(dir).expect("should parse json");
        assert_eq!(config.package.name, None);
        assert_eq!(config.window.title.as_deref(), Some("Only Title"));
        assert_eq!(config.window.width, None);
    });
}

#[test]
fn json_invalid_syntax() {
    with_temp_dir(&[(".ruft.json", "{ not json }")], |dir| {
        let err = read_project_config(dir).unwrap_err();
        assert!(err.contains("invalid .ruft.json"), "got: {err}");
    });
}

// ── 优先级：TOML > JSON ──

#[test]
fn toml_priority_over_json() {
    let toml_content = "\
[package]\n\
name = \"toml-wins\"\n\
";
    let json_content = r#"{"package":{"name":"json-loses"}}"#;
    with_temp_dir(
        &[(".ruft.toml", toml_content), (".ruft.json", json_content)],
        |dir| {
            let config = read_project_config(dir).expect("should read toml first");
            assert_eq!(config.package.name.as_deref(), Some("toml-wins"));
        },
    );
}

#[test]
fn json_used_when_no_toml() {
    let json_content = r#"{"package":{"name":"json-only"}}"#;
    with_temp_dir(&[(".ruft.json", json_content)], |dir| {
        let config = read_project_config(dir).expect("should read json");
        assert_eq!(config.package.name.as_deref(), Some("json-only"));
    });
}
