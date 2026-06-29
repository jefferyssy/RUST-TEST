//! 选择器匹配单元测试。

use crate::{BasicSelector, ComplexSelector, Specificity};

#[test]
fn test_parse_tag_selector() {
    let sel = ComplexSelector::parse("div").unwrap();
    assert_eq!(sel.segments.len(), 1);
    assert_eq!(sel.segments[0].parts.len(), 1);
    assert!(matches!(&sel.segments[0].parts[0], BasicSelector::Tag(t) if t == "div"));
}

#[test]
fn test_parse_class_selector() {
    let sel = ComplexSelector::parse(".container").unwrap();
    assert_eq!(sel.segments.len(), 1);
    assert!(matches!(&sel.segments[0].parts[0], BasicSelector::Class(c) if c == "container"));
}

#[test]
fn test_parse_id_selector() {
    let sel = ComplexSelector::parse("#app").unwrap();
    assert_eq!(sel.segments.len(), 1);
    assert!(matches!(&sel.segments[0].parts[0], BasicSelector::Id(i) if i == "app"));
}

#[test]
fn test_parse_compound_selector() {
    let sel = ComplexSelector::parse("div#app.container").unwrap();
    assert_eq!(sel.segments.len(), 1);
    let parts = &sel.segments[0].parts;
    assert_eq!(parts.len(), 3);
    assert!(matches!(&parts[0], BasicSelector::Tag(t) if t == "div"));
    assert!(matches!(&parts[1], BasicSelector::Id(i) if i == "app"));
    assert!(matches!(&parts[2], BasicSelector::Class(c) if c == "container"));
}

#[test]
fn test_parse_child_combinator() {
    let sel = ComplexSelector::parse("div > .c").unwrap();
    assert_eq!(sel.segments.len(), 2);
    assert!(matches!(&sel.segments[0].parts[0], BasicSelector::Tag(t) if t == "div"));
    assert!(matches!(&sel.segments[1].parts[0], BasicSelector::Class(c) if c == "c"));
}

#[test]
fn test_specificity() {
    let tag = ComplexSelector::parse("div").unwrap();
    assert_eq!(tag.specificity, Specificity(0, 0, 1));

    let class = ComplexSelector::parse(".foo").unwrap();
    assert_eq!(class.specificity, Specificity(0, 1, 0));

    let id = ComplexSelector::parse("#bar").unwrap();
    assert_eq!(id.specificity, Specificity(1, 0, 0));

    let compound = ComplexSelector::parse("div#bar.foo").unwrap();
    assert_eq!(compound.specificity, Specificity(1, 1, 1));
}
