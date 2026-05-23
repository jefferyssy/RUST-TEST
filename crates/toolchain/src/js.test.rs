use super::*;
use crate::html::HtmlElement;

fn make_element_vars() -> Vec<(String, HtmlElement)> {
    let mut d = HtmlElement::new("div");
    d.attributes.insert("class".to_string(), "display".to_string());
    let mut b = HtmlElement::new("button");
    b.attributes.insert("id".to_string(), "inc-btn".to_string());
    vec![
        ("display".to_string(), d),
        ("btn".to_string(), b),
    ]
}

#[test]
fn test_build_html_lookup() {
    let vars = make_element_vars();
    let lookup = build_html_lookup(&vars);
    assert_eq!(lookup.get(".display").unwrap(), "display");
    assert_eq!(lookup.get("#inc-btn").unwrap(), "btn");
}

#[test]
fn test_resolve_element_var_query_selector() {
    let vars = make_element_vars();
    let lookup = build_html_lookup(&vars);
    let result = resolve_element_var("document.querySelector('.display')", &lookup);
    assert_eq!(result, Some("display".to_string()));
}

#[test]
fn test_resolve_element_var_get_by_id() {
    let vars = make_element_vars();
    let lookup = build_html_lookup(&vars);
    let result = resolve_element_var("document.getElementById('inc-btn')", &lookup);
    assert_eq!(result, Some("btn".to_string()));
}

#[test]
fn test_extract_event_handlers() {
    let vars = make_element_vars();
    let lookup = build_html_lookup(&vars);
    let js = r#"
btn.addEventListener('click', function() {
  count = count + 1;
  display.textContent = count;
});
"#;
    let handlers = extract_event_handlers(js, &lookup);
    assert_eq!(handlers.len(), 1);
    assert_eq!(handlers[0].element_var, "btn");
    assert_eq!(handlers[0].event_type, "click");
    assert!(handlers[0].body_code.contains("text_content"));
}
