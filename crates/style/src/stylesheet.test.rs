use super::*;

#[test]
fn test_stylesheet_new() {
    let ss = StyleSheet::new("test.css");
    assert!(ss.rules.is_empty());
    assert_eq!(ss.url, "test.css");
}

#[test]
fn test_parse_inline_style_single() {
    let decls = parse_inline_style("color: red");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "color");
    assert_eq!(decls[0].value, "red");
    assert!(!decls[0].important);
}

#[test]
fn test_parse_inline_style_multiple() {
    let decls = parse_inline_style("color: red; font-size: 16px; background: blue");
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[1].property, "font-size");
    assert_eq!(decls[1].value, "16px");
}

#[test]
fn test_parse_inline_style_empty() {
    let decls = parse_inline_style("");
    assert!(decls.is_empty());
}

#[test]
fn test_parse_inline_style_no_colon() {
    let decls = parse_inline_style("just-text");
    assert!(decls.is_empty());
}

#[test]
fn test_parse_inline_style_extra_spaces() {
    let decls = parse_inline_style("  margin :  10px ");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].property, "margin");
    assert_eq!(decls[0].value, "10px");
}
