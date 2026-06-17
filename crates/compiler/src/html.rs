//! # HTML 解析器 — Phase 0
//!
//! 一次解析 HTML，同时收集所有 CSS/JS 资源 + 构建 DOM 树。
//! Phase 1+ 将替换为 html5ever。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════════════════════════════
// 类型定义
// ═══════════════════════════════════════════════════════════════════════

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

/// CSS 来源
#[derive(Debug, Clone)]
pub enum CssSource {
    /// `<link rel="stylesheet" href="...">` — 外部文件路径
    File(PathBuf),
    /// `<style>...</style>` — 内联内容
    Inline(String),
    /// `style="color: red"` — 元素属性
    InlineAttr { var_name: String, content: String },
}

/// JS 来源
#[derive(Debug, Clone)]
pub enum JsSource {
    /// `<script src="...">` — 外部文件路径
    File(PathBuf),
    /// `<script>...</script>` — 内联内容
    Inline(String),
}

/// 一次解析 HTML 的完整产出
#[derive(Debug, Clone)]
pub struct HtmlResources {
    /// DOM 元素树（仅 `<body>` 内可见元素）
    pub elements: Vec<HtmlElement>,
    /// 所有 CSS 来源（按出现顺序）
    pub css_sources: Vec<CssSource>,
    /// 所有 JS 来源（按出现顺序）
    pub js_sources: Vec<JsSource>,
    /// `import('./xxx')` 动态引用清单（Phase 1+ 用）
    pub dynamic_imports: Vec<String>,
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

/// 一次遍历令牌流，构建 DOM 树的同时收集 CSS/JS 资源。
fn build_document(tokens: &[Token], input_dir: &Path) -> HtmlResources {
    let mut roots: Vec<HtmlElement> = Vec::new();
    let mut stack: Vec<HtmlElement> = Vec::new();
    let mut css_sources = Vec::new();
    let mut js_sources = Vec::new();
    let mut dynamic_imports = Vec::new();
    let ignored_tags = ["meta", "title", "head", "!doctype"];

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::OpenTag { name, attrs, self_closing } => {
                // ── 处理 <style> ──
                if name == "style" && !self_closing {
                    i += 1;
                    let mut content = String::new();
                    while i < tokens.len() {
                        if let Token::CloseTag { name: n } = &tokens[i] {
                            if n == "style" { i += 1; break; }
                        }
                        if let Token::Text(t) = &tokens[i] {
                            content.push_str(t);
                        }
                        i += 1;
                    }
                    css_sources.push(CssSource::Inline(content));
                    continue;
                }

                // ── 处理 <link rel="stylesheet"> ──
                if name == "link"
                    && attrs.get("rel").map(|r| r.to_lowercase()) == Some("stylesheet".into())
                {
                    if let Some(href) = attrs.get("href") {
                        css_sources.push(CssSource::File(input_dir.join(href)));
                    }
                    i += 1;
                    continue;
                }

                // ── 处理 <script src="..."> ──
                if name == "script" {
                    if let Some(src) = attrs.get("src") {
                        js_sources.push(JsSource::File(input_dir.join(src)));
                        i += 1;
                        // 跳过闭合标签
                        while i < tokens.len() {
                            if let Token::CloseTag { name: n } = &tokens[i] {
                                if n == "script" { i += 1; break; }
                            }
                            i += 1;
                        }
                        continue;
                    }
                    // <script> 内联脚本
                    i += 1;
                    let mut content = String::new();
                    while i < tokens.len() {
                        if let Token::CloseTag { name: n } = &tokens[i] {
                            if n == "script" { i += 1; break; }
                        }
                        if let Token::Text(t) = &tokens[i] {
                            content.push_str(t);
                            // 检测 import('...') 动态引用
                            detect_dynamic_imports(t, &mut dynamic_imports);
                        }
                        i += 1;
                    }
                    js_sources.push(JsSource::Inline(content));
                    continue;
                }

                // ── 跳过其他被忽略标签 ──
                if ignored_tags.contains(&name.as_str()) {
                    if *self_closing {
                        i += 1;
                        continue;
                    }
                    let mut depth = 1;
                    i += 1;
                    while i < tokens.len() && depth > 0 {
                        match &tokens[i] {
                            Token::OpenTag { name: n, self_closing: sc, .. } => {
                                if !ignored_tags.contains(&n.as_str()) && n != "style" && n != "script" && !sc { depth += 1; }
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

                // ── 正规元素：加入 DOM 树 ──
                let mut el = HtmlElement::new(name);
                el.attributes = attrs.clone();

                // 收集 inline style
                if let Some(style_val) = attrs.get("style") {
                    let var_name = attrs.get("id").map(String::as_str)
                        .or_else(|| attrs.get("class").and_then(|c| c.split_whitespace().next()))
                        .unwrap_or(name)
                        .replace('-', "_");
                    css_sources.push(CssSource::InlineAttr {
                        var_name,
                        content: style_val.clone(),
                    });
                }

                if *self_closing {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(el);
                    }
                } else {
                    stack.push(el);
                }
                i += 1;
            }
            Token::CloseTag { name } => {
                if let Some(pos) = stack.iter().rposition(|e| e.tag == *name) {
                    let mut completed = stack.split_off(pos);
                    let el = completed.remove(0);
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(el);
                    } else {
                        roots.push(el);
                    }
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

    let elements = roots
        .into_iter()
        .filter(|e| e.tag != "html" && e.tag != "body" && e.tag != "head")
        .collect();

    HtmlResources {
        elements,
        css_sources,
        js_sources,
        dynamic_imports,
    }
}

/// 在 JS 源码中检测 `import('...')` 动态引用。
fn detect_dynamic_imports(js: &str, out: &mut Vec<String>) {
    let mut pos = 0;
    while let Some(idx) = js[pos..].find("import(") {
        let start = pos + idx + 7; // after "import("
        let rest = &js[start..];
        if let Some(quote) = rest.chars().next() {
            if quote == '\'' || quote == '"' {
                if let Some(end) = rest[1..].find(quote) {
                    let path = &rest[1..=end];
                    if !path.is_empty() {
                        out.push(path.to_string());
                    }
                }
            }
        }
        pos = start;
    }
}

/// 主入口：一次解析 HTML，同时收集所有资源 + 构建元素树。
pub fn parse_html_document(html_src: &str, input_dir: &Path) -> HtmlResources {
    let cleaned = strip_comments(html_src);
    let body = extract_body(&cleaned);
    let tokens = tokenize(body);
    build_document(&tokens, input_dir)
}

/// 解析 HTML 字符串，返回 body 内的元素树（`parse_html_document` 的简化包装）。
pub fn parse_html(html: &str) -> Vec<HtmlElement> {
    parse_html_document(html, Path::new(".")).elements
}

/// 从 HTML 源码中提取 CSS 和 JS 文件引用。
///
/// 返回 `(css_refs, js_refs)`，每个元素为文件路径字符串。
pub fn extract_references(html_src: &str) -> (Vec<String>, Vec<String>) {
    let mut css_refs = Vec::new();
    let mut js_refs = Vec::new();

    for line in html_src.lines() {
        // <link rel="stylesheet" href="...">
        if line.contains("stylesheet") && line.contains("href=") {
            if let Some(href) = extract_attr(line, "href") {
                css_refs.push(href);
            }
        }
        // <script src="...">
        if line.contains("<script") && line.contains("src=") {
            if let Some(src) = extract_attr(line, "src") {
                if !src.is_empty() {
                    js_refs.push(src);
                }
            }
        }
    }

    (css_refs, js_refs)
}

/// 从 HTML 标签行中提取指定属性的值。
fn extract_attr(line: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=", attr);
    let start = line.find(&pattern)?;
    let rest = &line[start + pattern.len()..];
    let delim = rest.chars().next()?;
    let rest = &rest[1..];
    let end = rest.find(delim)?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
#[path = "../test/html_test.rs"]
mod tests;
