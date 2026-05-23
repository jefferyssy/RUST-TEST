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
    assert!(selector_matches("div", "div", &[], None));
    assert!(!selector_matches("span", "div", &[], None));
}

#[test]
fn test_selector_matches_class() {
    assert!(selector_matches(".foo", "div", &["foo".to_string()], None));
    assert!(!selector_matches(".bar", "div", &["foo".to_string()], None));
}

#[test]
fn test_selector_matches_id() {
    assert!(selector_matches("#main", "div", &[], Some("main")));
    assert!(!selector_matches("#other", "div", &[], Some("main")));
}

#[test]
fn test_selector_matches_complex() {
    assert!(selector_matches("div.container#main", "div", &["container".to_string()], Some("main")));
}

#[test]
fn test_strip_comments() {
    let result = strip_css_comments("a { color: red; } /* comment */ b { color: blue; }");
    assert!(!result.contains("/*"));
}
