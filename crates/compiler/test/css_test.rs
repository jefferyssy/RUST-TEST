use super::*;

#[test]
fn test_parse_empty() {
    let rules = parse_css("");
    assert!(rules.is_empty());
}

#[test]
fn test_parse_single_rule() {
    let rules = parse_css(".foo { color: red; }");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].selector, ".foo");
    assert_eq!(rules[0].declarations.len(), 1);
    assert_eq!(rules[0].declarations[0], ("color".to_string(), "red".to_string()));
}

#[test]
fn test_parse_multiple_declarations() {
    let rules = parse_css("div { color: red; font-size: 16px; }");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].declarations.len(), 2);
}

#[test]
fn test_parse_multiple_rules() {
    let css = r#"
        .container { background: #f5f5f5; padding: 20px; }
        h1 { color: #333; }
    "#;
    let rules = parse_css(css);
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].selector, ".container");
    assert_eq!(rules[1].selector, "h1");
}

#[test]
fn test_selector_matches_tag() {
    assert!(selector_matches("div", "div", &[], None, false));
    assert!(!selector_matches("span", "div", &[], None, false));
}

#[test]
fn test_selector_matches_class() {
    assert!(selector_matches(".foo", "div", &["foo".to_string()], None, false));
    assert!(!selector_matches(".bar", "div", &["foo".to_string()], None, false));
}

#[test]
fn test_selector_matches_id() {
    assert!(selector_matches("#main", "div", &[], Some("main"), false));
    assert!(!selector_matches("#other", "div", &[], Some("main"), false));
}

#[test]
fn test_selector_matches_complex() {
    assert!(selector_matches("div.container#main", "div", &["container".to_string()], Some("main"), false));
}

#[test]
fn test_selector_matches_last_child() {
    // :last-child 伪类：is_last_child=true 时匹配，false 时不匹配
    assert!(selector_matches(".todo-item:last-child", "li", &["todo-item".to_string()], None, true));
    assert!(!selector_matches(".todo-item:last-child", "li", &["todo-item".to_string()], None, false));
    // 无 :last-child 的规则不受 is_last_child 影响
    assert!(selector_matches(".todo-item", "li", &["todo-item".to_string()], None, false));
    assert!(selector_matches(".todo-item", "li", &["todo-item".to_string()], None, true));
}

#[test]
fn test_strip_comments() {
    let result = strip_css_comments("a { color: red; } /* comment */ b { color: blue; }");
    assert!(!result.contains("/*"));
}
