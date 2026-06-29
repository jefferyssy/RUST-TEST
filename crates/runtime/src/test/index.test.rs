//! 反向索引单元测试。

use crate::index::RuleIndex;
use crate::{ComplexSelector, CssRule};

fn make_rule(selector_str: &str) -> CssRule {
    CssRule {
        selectorText: selector_str.to_string(),
        selector: ComplexSelector::parse(selector_str).unwrap(),
        style: vec![],
    }
}

#[test]
fn test_build_index_tag() {
    let rules = vec![
        make_rule("div"),
        make_rule("h1"),
    ];
    let index = RuleIndex::build(&rules);
    assert_eq!(index.tag.len(), 2);
    assert!(index.tag.contains_key("div"));
    assert!(index.tag.contains_key("h1"));
}

#[test]
fn test_build_index_class() {
    let rules = vec![
        make_rule(".foo"),
        make_rule(".bar"),
    ];
    let index = RuleIndex::build(&rules);
    assert_eq!(index.class.len(), 2);
    assert!(index.class.contains_key("foo"));
    assert!(index.class.contains_key("bar"));
}

#[test]
fn test_build_index_priority() {
    // div#app.container — 最右段有 id => 进 id map
    let rules = vec![make_rule("div#app.container")];
    let index = RuleIndex::build(&rules);
    assert_eq!(index.id.len(), 1);
    assert!(index.id.contains_key("app"));
    assert!(index.class.is_empty());
    assert!(index.tag.is_empty());
}

#[test]
fn test_find_candidates() {
    let rules = vec![
        make_rule("button"),
        make_rule(".btn"),
        make_rule("#inc_btn"),
    ];
    let index = RuleIndex::build(&rules);

    let candidates = index.find_candidates(
        Some("button"),
        &["btn".to_string()],
        None,
        &[],
    );
    // button tag + .btn class => 2 rules
    assert_eq!(candidates.len(), 2);

    let candidates = index.find_candidates(
        Some("button"),
        &[],
        Some("inc_btn"),
        &[],
    );
    // button tag + #inc_btn id => 2 rules
    assert_eq!(candidates.len(), 2);
}
