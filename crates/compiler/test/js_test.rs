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
    let (handlers, shared_state) = extract_event_handlers(js, &lookup, &[]);
    assert_eq!(handlers.len(), 1);
    assert_eq!(handlers[0].element_var, "btn");
    assert_eq!(handlers[0].event_type, "click");
    assert!(handlers[0].body_code.contains("text_content"));
    // 验证 count 被识别为共享状态
    assert_eq!(shared_state.len(), 0); // count 未声明为 let count = 0; 所以不是共享状态
}

#[test]
fn test_nested_addEventListener_cross_line() {
    // 模拟内联后的 body_lines：嵌套 addEventListener 的 { } 已被消耗
    let mut li_el = HtmlElement::new("li");
    li_el.attributes.insert("class".to_string(), "todo-item".to_string());
    let element_vars = vec![
        ("btn".to_string(), HtmlElement::new("button")),
        ("li".to_string(), li_el),
        ("span".to_string(), HtmlElement::new("span")),
    ];
    let lookup = build_html_lookup(&element_vars);
    let js = r#"
btn.addEventListener('click', function() {
  const li = document.createElement('li');
  const span = document.createElement('span');
  span.addEventListener('click', function() {
    li.classList.toggle('completed');
  });
});
"#;
    let (handlers, _shared_state) = extract_event_handlers(js, &lookup, &[]);
    eprintln!("=== Handler count: {} ===", handlers.len());
    eprintln!("=== Handler body ===");
    eprintln!("{}", handlers[0].body_code);
    eprintln!("=== END ===");
    assert_eq!(handlers.len(), 1);
    assert!(handlers[0].body_code.contains("add_event_listener"),
        "Should contain nested add_event_listener, got:\n{}", handlers[0].body_code);
}
