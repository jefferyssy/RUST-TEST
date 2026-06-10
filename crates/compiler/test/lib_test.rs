use super::*;

#[test]
fn test_generate_variable_name_with_id() {
    let mut el = HtmlElement::new("div");
    el.attributes.insert("id".to_string(), "main".to_string());
    let mut counts = HashMap::new();
    let name = generate_var_name(&el, &mut counts);
    assert_eq!(name, "main");
}

#[test]
fn test_generate_variable_name_with_class() {
    let mut el = HtmlElement::new("div");
    el.attributes.insert("class".to_string(), "container".to_string());
    let mut counts = HashMap::new();
    let name = generate_var_name(&el, &mut counts);
    assert_eq!(name, "container");
}

#[test]
fn test_generate_variable_name_with_tag() {
    let el = HtmlElement::new("h1");
    let mut counts = HashMap::new();
    let name = generate_var_name(&el, &mut counts);
    assert_eq!(name, "h1");
}

#[test]
fn test_generate_variable_name_deduplicates() {
    let mut counts = HashMap::new();
    let e1 = HtmlElement::new("div");
    let e2 = HtmlElement::new("div");
    let n1 = generate_var_name(&e1, &mut counts);
    let n2 = generate_var_name(&e2, &mut counts);
    assert_eq!(n1, "div");
    assert_eq!(n2, "div_2");
}

#[test]
fn test_generate_variable_name_id_with_hyphen() {
    let mut el = HtmlElement::new("button");
    el.attributes.insert("id".to_string(), "inc-btn".to_string());
    let mut counts = HashMap::new();
    let name = generate_var_name(&el, &mut counts);
    assert_eq!(name, "inc_btn");
}

// ── rel_path 测试 ──

#[test]
fn rel_path_basic() {
    let result = rel_path(
        &Path::new("target/generated/counter"),
        &Path::new("crates/renderer"),
    );
    assert!(result.contains("crates/renderer"), "got: {result}");
    assert!(!result.contains('\\'), "should use forward slashes: {result}");
}

#[test]
fn rel_path_no_backslashes() {
    let result = rel_path(&Path::new("a\\b\\c"), &Path::new("x\\y"));
    assert!(!result.contains('\\'), "expected no backslashes, got: {result}");
}

// ── find_file_by_ext 测试 ──

#[test]
fn find_file_by_ext_finds_html() {
    // examples/counter 中有 index.html
    let dir = Path::new("../examples/counter");
    if dir.exists() {
        let found = find_file_by_ext(dir, "html");
        assert!(found.is_some(), "should find index.html in counter");
    }
}

#[test]
fn find_file_by_ext_no_match() {
    // examples/counter 中没有 .pdf
    let dir = Path::new("../examples/counter");
    if dir.exists() {
        let found = find_file_by_ext(dir, "pdf");
        assert!(found.is_none(), "should not find any .pdf");
    }
}

// ── compile_project 测试 ──

#[test]
fn compile_project_counter() {
    let input_dir = Path::new("../examples/counter");
    if !input_dir.exists() {
        return; // 跳过（测试环境路径不同）
    }
    let opts = CompileOptions::default();
    let output = compile_project(input_dir, &opts).expect("should compile counter");

    assert!(!output.rust_code.is_empty());
    assert!(output.rust_code.contains("fn main()"));
    assert!(output.html_path.ends_with("index.html"));
}
