use super::*;

#[test]
fn test_new_element() {
    let e = ElementData::new("div");
    assert_eq!(e.tag_name(), "div");
    assert_eq!(e.get_attribute("class"), None);
}

#[test]
fn test_set_get_attribute() {
    let mut e = ElementData::new("div");
    e.set_attribute("id", "main");
    assert_eq!(e.get_attribute("id"), Some("main".to_string()));
    assert!(e.has_attribute("id"));
}

#[test]
fn test_remove_attribute() {
    let mut e = ElementData::new("div");
    e.set_attribute("class", "foo");
    e.remove_attribute("class");
    assert!(!e.has_attribute("class"));
}

#[test]
fn test_class_operations() {
    let mut e = ElementData::new("div");
    e.add_class("foo");
    e.add_class("bar");
    assert!(e.has_class("foo"));
    assert!(e.has_class("bar"));
    assert_eq!(e.class_name(), "foo bar");

    e.remove_class("foo");
    assert!(!e.has_class("foo"));
    assert_eq!(e.class_name(), "bar");
}

#[test]
fn test_toggle_class() {
    let mut e = ElementData::new("div");
    assert!(e.toggle_class("foo"));  // 添加
    assert!(e.has_class("foo"));
    assert!(!e.toggle_class("foo")); // 移除
    assert!(!e.has_class("foo"));
}

#[test]
fn test_style_operations() {
    let mut e = ElementData::new("div");
    e.set_style_value("color", "red");
    assert_eq!(e.get_style_value("color"), Some(&"red".to_string()));
    e.remove_style_value("color");
    assert_eq!(e.get_style_value("color"), None);
}

#[test]
fn test_parse_and_set_style() {
    let mut e = ElementData::new("div");
    e.parse_and_set_style("color: red; font-size: 16px");
    assert_eq!(e.get_style_value("color"), Some(&"red".to_string()));
    assert_eq!(e.get_style_value("font-size"), Some(&"16px".to_string()));
    // parse_and_set_style 也应同步到 attributes["style"]
    assert_eq!(e.get_attribute("style"), Some("color: red; font-size: 16px".to_string()));
}

#[test]
fn test_id() {
    let mut e = ElementData::new("div");
    assert!(e.id().is_none());
    e.set_id("main");
    assert_eq!(e.id(), Some(&"main".to_string()));
}

#[test]
fn test_attribute_syncs_class() {
    let mut e = ElementData::new("div");
    e.set_attribute("class", "foo bar");
    assert!(e.has_class("foo"));
    assert!(e.has_class("bar"));
}

#[test]
fn test_event_listener() {
    let mut e = ElementData::new("button");
    let id = e.add_event_listener("click", Box::new(|_| {}));
    assert_eq!(e.get_event_listeners("click").len(), 1);
    e.remove_event_listener("click", id);
    assert_eq!(e.get_event_listeners("click").len(), 0);
}
