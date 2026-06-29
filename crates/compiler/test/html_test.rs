use super::*;

#[test]
fn test_parse_empty() {
    let elements = parse_html("<body></body>");
    assert!(elements.is_empty());
}

#[test]
fn test_parse_simple_div() {
    let html = "<body><div class=\"foo\">hello</div></body>";
    let elements = parse_html(html);
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].tag, "div");
    assert_eq!(elements[0].attributes.get("class").unwrap(), "foo");
    assert_eq!(elements[0].text_content, "hello");
}

#[test]
fn test_parse_nested() {
    let html = r#"<body>
        <div class="container">
            <h1>Title</h1>
            <p>text</p>
        </div>
    </body>"#;
    let elements = parse_html(html);
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].tag, "div");
    assert_eq!(elements[0].children.len(), 2);
    assert_eq!(elements[0].children[0].tag, "h1");
    assert_eq!(elements[0].children[0].text_content, "Title");
    assert_eq!(elements[0].children[1].tag, "p");
    assert_eq!(elements[0].children[1].text_content, "text");
}

#[test]
fn test_parse_with_body_and_html() {
    let html = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
    <div id="main">content</div>
</body>
</html>"#;
    let elements = parse_html(html);
    assert_eq!(elements.len(), 1);
    // 顶层是 <html>，含 <head> + <body>
    assert_eq!(elements[0].tag, "html");
    assert_eq!(elements[0].children.len(), 2);
    assert_eq!(elements[0].children[0].tag, "head");
    assert_eq!(elements[0].children[1].tag, "body");
    // <body> 含 <div>
    let body = &elements[0].children[1];
    assert_eq!(body.children.len(), 1);
    assert_eq!(body.children[0].tag, "div");
    assert_eq!(body.children[0].attributes.get("id").unwrap(), "main");
    assert_eq!(body.children[0].text_content, "content");
}

#[test]
fn test_multiple_elements() {
    let html = "<body><span>a</span><span>b</span></body>";
    let elements = parse_html(html);
    assert_eq!(elements.len(), 2);
    assert_eq!(elements[0].tag, "span");
    assert_eq!(elements[1].tag, "span");
}
