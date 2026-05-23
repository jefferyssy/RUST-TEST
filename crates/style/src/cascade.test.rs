use super::*;
use crate::stylesheet::Rule;
use crate::values::{CSSUnit, CSSValue};
use dom::Color;

#[test]
fn test_computed_style_new() {
    let cs = ComputedStyle::new();
    assert!(cs.properties.is_empty());
}

#[test]
fn test_computed_style_get_set() {
    let mut cs = ComputedStyle::new();
    cs.set("color", CSSValue::Keyword("red".to_string()));
    assert_eq!(cs.get("color"), Some(&CSSValue::Keyword("red".to_string())));
    assert_eq!(cs.get("nonexistent"), None);
}

#[test]
fn test_get_or_initial() {
    let cs = ComputedStyle::new();
    assert_eq!(cs.get_or_initial("color"), CSSValue::Initial);
}

#[test]
fn test_merge() {
    let mut base = ComputedStyle::new();
    base.set("color", CSSValue::Keyword("red".to_string()));

    let mut other = ComputedStyle::new();
    other.set("color", CSSValue::Keyword("blue".to_string()));
    other.set("font-size", CSSValue::Length(16.0, CSSUnit::Px));

    base.merge(&other);
    // 不覆盖已有属性
    assert_eq!(base.get("color"), Some(&CSSValue::Keyword("red".to_string())));
    // 添加不存在的属性
    assert_eq!(base.get("font-size"), Some(&CSSValue::Length(16.0, CSSUnit::Px)));
}

#[test]
fn test_compute_element_style_basic() {
    let element = ElementData::new("div");
    let sheet = StyleSheet {
        url: "test.css".to_string(),
        rules: vec![Rule {
            selectors: vec!["div".to_string()],
            declarations: vec![
                Declaration { property: "color".to_string(), value: "red".to_string(), important: false },
                Declaration { property: "font-size".to_string(), value: "16px".to_string(), important: false },
            ],
        }],
    };
    let style = compute_element_style(&element, None, &[sheet], &[]);
    assert_eq!(style.get("color"), Some(&CSSValue::Color(Color::rgb(255, 0, 0))));
    assert_eq!(style.get("font-size"), Some(&CSSValue::Length(16.0, CSSUnit::Px)));
}

#[test]
fn test_inline_style_overrides() {
    let element = ElementData::new("div");
    let sheet = StyleSheet {
        url: "test.css".to_string(),
        rules: vec![Rule {
            selectors: vec!["div".to_string()],
            declarations: vec![
                Declaration { property: "color".to_string(), value: "blue".to_string(), important: false },
            ],
        }],
    };
    let inline = vec![
        Declaration { property: "color".to_string(), value: "red".to_string(), important: false },
    ];
    let style = compute_element_style(&element, None, &[sheet], &inline);
    // 内联样式优先级高
    assert_eq!(style.get("color"), Some(&CSSValue::Color(Color::rgb(255, 0, 0))));
}

#[test]
fn test_important_overrides_inline() {
    let element = ElementData::new("div");
    let sheet = StyleSheet {
        url: "test.css".to_string(),
        rules: vec![Rule {
            selectors: vec!["div".to_string()],
            declarations: vec![
                Declaration { property: "color".to_string(), value: "blue".to_string(), important: true },
            ],
        }],
    };
    let inline = vec![
        Declaration { property: "color".to_string(), value: "red".to_string(), important: false },
    ];
    let style = compute_element_style(&element, None, &[sheet], &inline);
    // !important 优先级高于内联
    assert_eq!(style.get("color"), Some(&CSSValue::Color(Color::rgb(0, 0, 255))));
}

#[test]
fn test_inherited_properties() {
    let mut parent = ComputedStyle::new();
    parent.set("color", CSSValue::Keyword("blue".to_string()));
    parent.set("text-align", CSSValue::Keyword("center".to_string()));

    let element = ElementData::new("span");
    let style = compute_element_style(&element, Some(&parent), &[], &[]);
    // 继承父元素的 color
    assert_eq!(style.get("color"), Some(&CSSValue::Keyword("blue".to_string())));
    // 继承父元素的 text-align
    assert_eq!(style.get("text-align"), Some(&CSSValue::Keyword("center".to_string())));
}

#[test]
fn test_specificity_ordering() {
    let mut element = ElementData::new("div");
    // 类选择器比标签选择器优先级高
    let sheet = StyleSheet {
        url: "test.css".to_string(),
        rules: vec![
            Rule {
                selectors: vec!["div".to_string()],
                declarations: vec![
                    Declaration { property: "color".to_string(), value: "red".to_string(), important: false },
                ],
            },
            Rule {
                selectors: vec![".special".to_string()],
                declarations: vec![
                    Declaration { property: "color".to_string(), value: "green".to_string(), important: false },
                ],
            },
        ],
    };
    element.set_attribute("class", "special");
    let style = compute_element_style(&element, None, &[sheet], &[]);
    // .special 优先级高于 div，所以颜色应为 green
    assert_eq!(style.get("color"), Some(&CSSValue::Color(Color::rgb(0, 128, 0))));
}
