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
