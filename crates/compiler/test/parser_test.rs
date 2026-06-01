//! parser 模块测试

use super::*;

#[test]
fn test_source_kind_equality() {
    assert_eq!(SourceKind::Html, SourceKind::Html);
    assert_eq!(SourceKind::Css, SourceKind::Css);
    assert_eq!(SourceKind::JavaScript, SourceKind::JavaScript);
    assert_ne!(SourceKind::Html, SourceKind::Css);
    assert_ne!(SourceKind::Html, SourceKind::JavaScript);
}

#[test]
fn test_source_kind_debug() {
    let dbg = format!("{:?}", SourceKind::Html);
    assert!(dbg.contains("Html"));
    let dbg = format!("{:?}", SourceKind::Css);
    assert!(dbg.contains("Css"));
    let dbg = format!("{:?}", SourceKind::JavaScript);
    assert!(dbg.contains("JavaScript"));
}

#[test]
fn test_source_kind_clone() {
    let kind = SourceKind::Html;
    let cloned = kind.clone();
    assert_eq!(kind, cloned);
}

#[test]
fn test_source_file_creation() {
    let sf = SourceFile {
        path: "index.html".into(),
        kind: SourceKind::Html,
        content: "<div></div>".into(),
    };
    assert_eq!(sf.path, "index.html");
    assert_eq!(sf.kind, SourceKind::Html);
    assert_eq!(sf.content, "<div></div>");
}

#[test]
fn test_source_file_clone() {
    let sf = SourceFile {
        path: "style.css".into(),
        kind: SourceKind::Css,
        content: "body { color: red; }".into(),
    };
    let cloned = sf.clone();
    assert_eq!(sf.path, cloned.path);
    assert_eq!(sf.kind, cloned.kind);
    assert_eq!(sf.content, cloned.content);
}

#[test]
fn test_compilation_unit_empty() {
    let unit = CompilationUnit {
        elements: vec![],
        css_rules: vec![],
        event_handlers: vec![],
        source_files: vec![],
    };
    assert!(unit.elements.is_empty());
    assert!(unit.css_rules.is_empty());
    assert!(unit.event_handlers.is_empty());
    assert!(unit.source_files.is_empty());
}

#[test]
fn test_compilation_unit_debug() {
    let unit = CompilationUnit {
        elements: vec![],
        css_rules: vec![],
        event_handlers: vec![],
        source_files: vec![],
    };
    let dbg = format!("{:?}", unit);
    assert!(dbg.contains("CompilationUnit"));
}
