use super::*;

#[test]
fn test_new_text() {
    let t = Text::new("hello");
    assert_eq!(t.data(), "hello");
    assert_eq!(t.length(), 5);
}

#[test]
fn test_set_data() {
    let mut t = Text::new("hello");
    t.set_data("world");
    assert_eq!(t.data(), "world");
    assert_eq!(t.length(), 5);
}

#[test]
fn test_empty_text() {
    let t = Text::new("");
    assert_eq!(t.data(), "");
    assert_eq!(t.length(), 0);
}

#[test]
fn test_unicode_text() {
    let t = Text::new("你好");
    assert_eq!(t.length(), 6); // UTF-8 字节长度
    assert_eq!(t.data(), "你好");
}
