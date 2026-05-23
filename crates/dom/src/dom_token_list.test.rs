use super::*;

#[test]
fn test_from_string() {
    let list = DOMTokenList::from_string("foo bar baz");
    assert!(list.contains("foo"));
    assert!(list.contains("bar"));
    assert!(list.contains("baz"));
    assert!(!list.contains("qux"));
}

#[test]
fn test_add_no_duplicate() {
    let mut list = DOMTokenList::from_string("");
    list.add("foo");
    assert!(list.contains("foo"));
    list.add("foo");
    assert_eq!(list.to_string(), "foo");
}

#[test]
fn test_remove() {
    let mut list = DOMTokenList::from_string("foo bar baz");
    list.remove("bar");
    assert!(!list.contains("bar"));
    assert!(list.contains("foo"));
    assert!(list.contains("baz"));
}

#[test]
fn test_toggle_add() {
    let mut list = DOMTokenList::from_string("");
    assert!(list.toggle("foo"));
    assert!(list.contains("foo"));
}

#[test]
fn test_toggle_remove() {
    let mut list = DOMTokenList::from_string("foo");
    assert!(!list.toggle("foo"));
    assert!(!list.contains("foo"));
}

#[test]
fn test_to_string_whitespace() {
    let list = DOMTokenList::from_string("foo  bar  baz");
    assert_eq!(list.to_string(), "foo bar baz");
}

#[test]
fn test_empty() {
    let list = DOMTokenList::from_string("");
    assert!(!list.contains("x"));
    assert_eq!(list.to_string(), "");
}
