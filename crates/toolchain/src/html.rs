//! # HTML 解析器 — Phase 0
//!
//! 简化版 HTML 标签解析器，用于将 HTML 转为元素树。
//! Phase 1+ 将替换为 html5ever。
//!
//! 处理：
//! - 开闭标签 `<div>...</div>`
//! - 属性 `class="foo" id="bar"`
//! - 自闭合标签 `<br>`, `<link>`
//! - 文本内容提取
//! - 过滤 <head>/<script>/<link>/<title>/<!DOCTYPE>
//! - 只处理 <body> 内元素

use std::collections::HashMap;

/// HTML 元素节点
#[derive(Debug, Clone)]
pub struct HtmlElement {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub text_content: String,
    pub children: Vec<HtmlElement>,
}

impl HtmlElement {
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_lowercase(),
            attributes: HashMap::new(),
            text_content: String::new(),
            children: Vec::new(),
        }
    }
}

/// 提取 <body>...</body> 之间的内容，若没有则用全文
fn extract_body(html: &str) -> &str {
    let lower = html.to_lowercase();
    if let Some(start) = lower.find("<body") {
        // 找到 <body> 的结束 '>'
        let after_tag = html[start..].find('>').map(|i| start + i + 1).unwrap_or(0);
        if let Some(end) = lower[after_tag..].find("</body>") {
            return &html[after_tag..after_tag + end];
        }
        return &html[after_tag..];
    }
    html
}

/// 去除 HTML 注释 <!-- ... -->
fn strip_comments(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < s.len() {
        if i + 3 < s.len() && bytes[i] == b'<' && bytes[i+1] == b'!' && bytes[i+2] == b'-' && bytes[i+3] == b'-' {
            if let Some(end) = s[i+4..].find("-->") {
                i += end + 7;
                continue;
            }
        }
        result.push(s.as_bytes()[i] as char);
        i += 1;
    }
    result
}

/// HTML 令牌
#[derive(Debug, Clone)]
enum Token {
    OpenTag {
        name: String,
        attrs: HashMap<String, String>,
        self_closing: bool,
    },
    CloseTag {
        name: String,
    },
    Text(String),
}

/// 从属性字符串解析属性对
fn parse_attributes(s: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < s.len() {
        // 跳过空白
        while i < s.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= s.len() {
            break;
        }
        // 读取属性名（字母、数字、-、_）
        let name_start = i;
        while i < s.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_' || bytes[i] == b':') {
            i += 1;
        }
        if name_start == i {
            i += 1;
            continue;
        }
        let name = s[name_start..i].to_string();

        // 跳过空白和 '='
        while i < s.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < s.len() && bytes[i] == b'=' {
            i += 1;
        } else {
            // 无值属性（如 disabled）
            attrs.insert(name, String::new());
            continue;
        }
        // 跳过 '=' 后的空白
        while i < s.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        // 读取属性值
        if i < s.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
            let quote = bytes[i];
            i += 1;
            let val_start = i;
            while i < s.len() && bytes[i] != quote {
                i += 1;
            }
            let value = s[val_start..i].to_string();
            if i < s.len() { i += 1; }
            attrs.insert(name, value);
        } else {
            // 无引号值
            let val_start = i;
            while i < s.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                i += 1;
            }
            let value = s[val_start..i].to_string();
            attrs.insert(name, value);
        }
    }
    attrs
}

/// 将 HTML 正文解析为令牌流
fn tokenize(body: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    let len = body.len();

    // 自闭合标签列表
    let self_closing = ["br", "hr", "img", "input", "meta", "link", "area", "base", "col", "embed", "source", "track", "wbr"];

    while i < len {
        if bytes[i] == b'<' {
            // 处理标签
            let _tag_start = i;
            i += 1;

            if i < len && bytes[i] == b'/' {
                // 闭合标签 </tagname>
                i += 1;
                let name_start = i;
                while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                    i += 1;
                }
                let name = body[name_start..i].to_lowercase();
                // 跳过直到 '>'
                while i < len && bytes[i] != b'>' { i += 1; }
                if i < len { i += 1; }
                tokens.push(Token::CloseTag { name });
            } else if i < len && bytes[i] == b'!' {
                // <!DOCTYPE> 或注释（跳过后面的内容）
                while i < len && bytes[i] != b'>' { i += 1; }
                if i < len { i += 1; }
            } else {
                // 开标签 <tagname ...>
                let name_start = i;
                while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' && bytes[i] != b'/' {
                    i += 1;
                }
                let name = body[name_start..i].to_lowercase();

                // 是否是自闭合
                let mut is_self_closing = self_closing.contains(&name.as_str());

                // 读取属性直到 '>' 或 '/>'
                let attr_start = i;
                let mut found_slash_close = false;
                while i < len {
                    if bytes[i] == b'>' {
                        i += 1;
                        break;
                    }
                    if bytes[i] == b'/' && i + 1 < len && bytes[i+1] == b'>' {
                        is_self_closing = true;
                        found_slash_close = true;
                        i += 2;
                        break;
                    }
                    // 跳过引号内的内容
                    if bytes[i] == b'"' {
                        i += 1;
                        while i < len && bytes[i] != b'"' { i += 1; }
                        if i < len { i += 1; }
                        continue;
                    }
                    if bytes[i] == b'\'' {
                        i += 1;
                        while i < len && bytes[i] != b'\'' { i += 1; }
                        if i < len { i += 1; }
                        continue;
                    }
                    i += 1;
                }

                // 提取属性字符串（在 <tag 和 > 之间）
                let attrs_str = if i > attr_start {
                    let end = i - if found_slash_close { 2 } else { 1 };
                    if end > attr_start { &body[attr_start..end] } else { "" }
                } else { "" };

                let attrs = parse_attributes(attrs_str);

                tokens.push(Token::OpenTag { name, attrs, self_closing: is_self_closing });
            }
        } else {
            // 文本内容
            let text_start = i;
            while i < len && bytes[i] != b'<' {
                i += 1;
            }
            let text = body[text_start..i].trim().to_string();
            if !text.is_empty() {
                tokens.push(Token::Text(text));
            }
        }
    }

    tokens
}

/// 树构建
fn collect_elements(tokens: &[Token]) -> Vec<HtmlElement> {
    let mut roots: Vec<HtmlElement> = Vec::new();
    let mut stack: Vec<HtmlElement> = Vec::new();
    let ignored_tags = ["script", "style", "link", "meta", "title", "head", "!doctype"];

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::OpenTag { name, attrs, self_closing } => {
                if ignored_tags.contains(&name.as_str()) {
                    // 跳过被忽略标签的内容
                    if *self_closing {
                        i += 1;
                        continue;
                    }
                    // 查找匹配的闭合标签
                    let mut depth = 1;
                    i += 1;
                    while i < tokens.len() && depth > 0 {
                        match &tokens[i] {
                            Token::OpenTag { name: n, self_closing: sc, .. } => {
                                if !ignored_tags.contains(&n.as_str()) && !sc { depth += 1; }
                            }
                            Token::CloseTag { name: n } => {
                                if n == name { depth -= 1; }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    continue;
                }

                let mut el = HtmlElement::new(name);
                el.attributes = attrs.clone();

                if *self_closing {
                    // 自闭合 → 直接加入父元素或根
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(el);
                    }
                } else {
                    stack.push(el);
                }
                i += 1;
            }
            Token::CloseTag { name } => {
                // 从栈中弹出匹配的元素
                if let Some(pos) = stack.iter().rposition(|e| e.tag == *name) {
                    let mut completed = stack.split_off(pos);
                    let el = completed.remove(0);
                    // 更新栈顶元素的 children（如果有）
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(el);
                    } else {
                        roots.push(el);
                    }
                    // 将剩下的元素拉回栈
                    // 实际上 split_off 后 completed 是后半段 [el, ...]
                    // el 已被移出，剩下的应该是同级的后续元素
                    for remaining in completed {
                        if let Some(parent) = stack.last_mut() {
                            parent.children.push(remaining);
                        } else {
                            roots.push(remaining);
                        }
                    }
                }
                i += 1;
            }
            Token::Text(text) => {
                if let Some(top) = stack.last_mut() {
                    if !top.text_content.is_empty() {
                        top.text_content.push(' ');
                    }
                    top.text_content.push_str(text);
                }
                i += 1;
            }
        }
    }

    // 栈中剩余的元素作为根
    roots
}

/// 主入口：解析 HTML 字符串，返回 body 内的元素树
pub fn parse_html(html: &str) -> Vec<HtmlElement> {
    let cleaned = strip_comments(html);
    let body = extract_body(&cleaned);
    let tokens = tokenize(body);
    let elements = collect_elements(&tokens);

    // 过滤掉特殊的或空的顶层元素
    elements
        .into_iter()
        .filter(|e| e.tag != "html" && e.tag != "body" && e.tag != "head")
        .collect()
}

#[cfg(test)]
#[path = "html.test.rs"]
mod tests;
