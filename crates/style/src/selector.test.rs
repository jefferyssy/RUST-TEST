use super::*;
use crate::stylesheet::Rule;

fn make_element(tag: &str, id: Option<&str>, classes: &[&str]) -> ElementData {
    let mut e = ElementData::new(tag);
    if let Some(id_val) = id {
        e.set_id(id_val);
    }
    for cls in classes {
        e.add_class(cls);
    }
    e
}

#[test]
fn test_tag_selector_match() {
    let e = make_element("div", None, &[]);
    assert!(element_matches_selector(&e, "div"));
    assert!(!element_matches_selector(&e, "span"));
}

#[test]
fn test_class_selector_match() {
    let e = make_element("div", None, &["container"]);
    assert!(element_matches_selector(&e, ".container"));
    assert!(!element_matches_selector(&e, ".other"));
}

#[test]
fn test_id_selector_match() {
    let e = make_element("div", Some("main"), &[]);
    assert!(element_matches_selector(&e, "#main"));
}

#[test]
fn test_complex_selector_match() {
    let e = make_element("div", Some("main"), &["container", "active"]);
    assert!(element_matches_selector(&e, "div.container#main"));
    assert!(element_matches_selector(&e, "div.container.active"));
    assert!(!element_matches_selector(&e, "span.container"));
}

#[test]
fn test_wildcard_selector() {
    let e = make_element("div", None, &[]);
    assert!(element_matches_selector(&e, "*"));
}

#[test]
fn test_match_selectors() {
    let e = make_element("div", None, &["foo"]);
    let sheet = StyleSheet {
        url: "test.css".to_string(),
        selector_index: None,
        rules: vec![
            Rule {
                selectors: vec!["div".to_string()],
                declarations: vec![
                    Declaration { property: "color".to_string(), value: "red".to_string(), important: false },
                ],
            },
            Rule {
                selectors: vec![".bar".to_string()],
                declarations: vec![
                    Declaration { property: "color".to_string(), value: "blue".to_string(), important: false },
                ],
            },
        ],
    };
    let results = match_selectors(&e, &[sheet]);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].declaration.value, "red");
}

#[test]
fn test_specificity_tag() {
    assert_eq!(compute_specificity("div"), (0, 0, 1));
}

#[test]
fn test_specificity_class() {
    assert_eq!(compute_specificity(".container"), (0, 1, 0));
}

#[test]
fn test_specificity_id() {
    assert_eq!(compute_specificity("#main"), (1, 0, 0));
}

#[test]
fn test_specificity_complex() {
    assert_eq!(compute_specificity("div.container#main"), (1, 1, 1));
    assert_eq!(compute_specificity("a.b.c#d"), (1, 2, 1));
}
