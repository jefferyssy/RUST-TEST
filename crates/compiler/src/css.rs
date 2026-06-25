//! # CSS 处理器
//!
//! CSS 解析 + 选择器结构化输出（供 codegen 生成 Select builder 代码）。

/// 已解析的 CSS 规则。
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selector: String,
    pub declarations: Vec<(String, String)>,
}

/// 选择器段（供 codegen 用）。
#[derive(Debug, Clone)]
pub struct SelectorSegment {
    pub combinator: Option<&'static str>, // None=首段, Some("child")= >, Some("descendant")=空格, etc.
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: Vec<(String, String, String)>, // (name, op, value)
    pub pseudo_classes: Vec<String>,
    pub pseudo_elements: Vec<String>,
}

/// 解析所有 CSS 来源。
pub fn parse_css_sources(sources: &[crate::html::CssSource]) -> Vec<CssRule> {
    let mut all_rules = Vec::new();
    for src in sources {
        let content = match src {
            crate::html::CssSource::File(path) => {
                std::fs::read_to_string(path).unwrap_or_default()
            }
            crate::html::CssSource::Inline(content) => content.clone(),
            crate::html::CssSource::InlineAttr { content, .. } => content.clone(),
        };
        all_rules.extend(parse_css(&content));
    }
    all_rules
}

/// 解析 CSS 规则列表。
pub fn parse_css(css: &str) -> Vec<CssRule> {
    let cleaned = strip_css_comments(css);
    let mut rules = Vec::new();
    let bytes = cleaned.as_bytes();
    let len = cleaned.len();
    let mut i = 0;

    while i < len {
        while i < len && bytes[i].is_ascii_whitespace() { i += 1; }
        if i >= len { break; }

        let sel_start = i;
        let mut brace_depth = 0;
        while i < len && brace_depth == 0 {
            if bytes[i] == b'{' { break; }
            if bytes[i] == b'(' { brace_depth += 1; }
            if bytes[i] == b')' { if brace_depth > 0 { brace_depth -= 1; } }
            i += 1;
        }
        let selector_raw = normalize_selector_whitespace(&cleaned[sel_start..i]);
        if selector_raw.is_empty() { i += 1; continue; }
        if i >= len || bytes[i] != b'{' { continue; }
        i += 1;

        let decl_start = i;
        brace_depth = 0;
        while i < len {
            if bytes[i] == b'}' && brace_depth == 0 { break; }
            if bytes[i] == b'(' { brace_depth += 1; }
            if bytes[i] == b')' { if brace_depth > 0 { brace_depth -= 1; } }
            i += 1;
        }
        let decl_block = cleaned[decl_start..i].trim().to_string();
        if i < len { i += 1; }

        let declarations = parse_declarations(&decl_block);
        // 拆分逗号分隔的选择器列表（如 .av, .ab → 两条规则）
        for sel in split_selector_list(&selector_raw) {
            if sel.is_empty() { continue; }
            rules.push(CssRule { selector: sel.to_string(), declarations: declarations.clone() });
        }
    }
    rules
}

/// 将选择器字符串解析为结构化段列表。
pub fn parse_selector(selector: &str) -> Vec<SelectorSegment> {
    let selector = selector.trim();
    if selector.is_empty() { return vec![]; }

    let mut segments = Vec::new();
    let mut remaining = selector;
    // 首段无组合器
    let mut combinator: Option<&'static str> = None;

    loop {
        // 解析当前段
        let (seg, rest) = parse_compound_segment(remaining);
        if seg.tag.is_none() && seg.id.is_none() && seg.classes.is_empty()
            && seg.attrs.is_empty() && seg.pseudo_classes.is_empty() && seg.pseudo_elements.is_empty() {
            break;
        }

        segments.push(SelectorSegment { combinator, ..seg });
        remaining = rest;

        // 检测到下一段的组合器
        remaining = remaining.trim_start();
        if remaining.is_empty() { break; }

        combinator = if remaining.starts_with('>') {
            remaining = remaining[1..].trim_start();
            Some("child")
        } else if remaining.starts_with('+') {
            remaining = remaining[1..].trim_start();
            Some("adjacent")
        } else if remaining.starts_with('~') {
            remaining = remaining[1..].trim_start();
            Some("sibling")
        } else {
            Some("descendant") // 隐式空格
        };
    }

    segments
}

/// 解析单个复合选择器段（无组合器）。
fn parse_compound_segment(s: &str) -> (SelectorSegment, &str) {
    let mut tag = None;
    let mut id = None;
    let mut classes = Vec::new();
    let mut attrs = Vec::new();
    let mut pseudo_classes = Vec::new();
    let mut pseudo_elements = Vec::new();

    let mut remaining = s;

    loop {
        if remaining.is_empty() { break; }
        let c = remaining.chars().next().unwrap();

        if c == '#' {
            let end = remaining[1..].find(|ch: char| ch == '.' || ch == '#' || ch == ':' || ch == '[' || ch == ' ' || ch == '>' || ch == '+' || ch == '~' || ch == ',').map(|i| i + 1).unwrap_or(remaining.len());
            id = Some(remaining[1..end].to_string());
            remaining = &remaining[end..];
        } else if c == '.' {
            let end = remaining[1..].find(|ch: char| ch == '.' || ch == '#' || ch == ':' || ch == '[' || ch == ' ' || ch == '>' || ch == '+' || ch == '~' || ch == ',').map(|i| i + 1).unwrap_or(remaining.len());
            classes.push(remaining[1..end].to_string());
            remaining = &remaining[end..];
        } else if c == ':' {
            let is_double = remaining.starts_with("::");
            let off = if is_double { 2 } else { 1 };
            let end = remaining[off..].find(|ch: char| !ch.is_alphanumeric() && ch != '-').unwrap_or(remaining.len() - off);
            let name = &remaining[off..off + end];
            if is_double {
                pseudo_elements.push(name.to_string());
            } else {
                pseudo_classes.push(name.to_string());
            }
            remaining = &remaining[off + end..];
        } else if c == '[' {
            if let Some(bracket_end) = remaining.find(']') {
                let attr_content = &remaining[1..bracket_end];
                let (attr_name, op, val) = parse_attr(attr_content);
                attrs.push((attr_name, op, val));
                remaining = &remaining[bracket_end + 1..];
            } else { break; }
        } else if c == '*' {
            tag = Some("*".into());
            remaining = &remaining[1..];
        } else if c == ' ' || c == '>' || c == '+' || c == '~' || c == ',' {
            break;
        } else {
            let end = remaining.find(|ch: char| ch == '.' || ch == '#' || ch == ':' || ch == '[' || ch == ' ' || ch == '>' || ch == '+' || ch == '~' || ch == ',').unwrap_or(remaining.len());
            tag = Some(remaining[..end].to_lowercase());
            remaining = &remaining[end..];
        }
    }

    (SelectorSegment { combinator: None, tag, id, classes, attrs, pseudo_classes, pseudo_elements }, remaining)
}

fn parse_attr(content: &str) -> (String, String, String) {
    let content = content.trim();
    if let Some(eq_pos) = content.find('=') {
        let name = content[..eq_pos].trim().trim_end_matches(&['~', '|', '^', '$', '*'] as &[_]).trim().to_string();
        let op_start = content[..eq_pos].trim().chars().rev().next().unwrap_or('=');
        let op = match op_start {
            '~' => "WordMatch", '|' => "HyphenMatch", '^' => "StartsWith",
            '$' => "EndsWith", '*' => "Contains", _ => "Exact",
        };
        let val = content[eq_pos + 1..].trim().trim_matches('"').trim_matches('\'').to_string();
        (name, op.to_string(), val)
    } else {
        (content.to_string(), "Has".to_string(), String::new())
    }
}

/// 规范化选择器中的空白字符，将换行/制表符替换为空格，并压缩连续空格。
fn normalize_selector_whitespace(selector: &str) -> String {
    selector
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// 按逗号拆分选择器列表，避开括号/方括号内的逗号。
fn split_selector_list(selector: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let bytes = selector.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => paren_depth += 1,
            b')' => if paren_depth > 0 { paren_depth -= 1 },
            b'[' => bracket_depth += 1,
            b']' => if bracket_depth > 0 { bracket_depth -= 1 },
            b',' if paren_depth == 0 && bracket_depth == 0 => {
                let part = selector[start..i].trim().to_string();
                if !part.is_empty() {
                    parts.push(part);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = selector[start..].trim().to_string();
    if !last.is_empty() {
        parts.push(last);
    }
    parts
}

fn strip_css_comments(css: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let bytes = css.as_bytes();
    let mut i = 0;
    while i < css.len() {
        if i + 1 < css.len() && bytes[i] == b'/' && bytes[i+1] == b'*' {
            if let Some(end) = css[i+2..].find("*/") {
                i += end + 4;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn parse_declarations(block: &str) -> Vec<(String, String)> {
    let mut decls = Vec::new();
    let mut i = 0;
    let bytes = block.as_bytes();
    let len = block.len();
    while i < len {
        while i < len && (bytes[i].is_ascii_whitespace() || bytes[i] == b';') { i += 1; }
        if i >= len { break; }
        let prop_start = i;
        while i < len && bytes[i] != b':' { i += 1; }
        let property = block[prop_start..i].trim().to_string();
        if i < len { i += 1; }
        while i < len && bytes[i].is_ascii_whitespace() { i += 1; }
        let val_start = i;
        let mut paren_depth = 0;
        while i < len {
            if paren_depth == 0 && (bytes[i] == b';' || bytes[i] == b'}') { break; }
            if bytes[i] == b'(' { paren_depth += 1; }
            if bytes[i] == b')' { if paren_depth > 0 { paren_depth -= 1; } }
            i += 1;
        }
        let value = block[val_start..i].trim().to_string();
        if !property.is_empty() && !value.is_empty() {
            decls.push((property, value));
        }
    }
    decls
}

/// 快速选择器匹配（供 js.rs 用的编译期匹配）。
/// 支持 tag、.class、#id、tag.class#id、:last-child 等单层选择器。
pub fn selector_matches(
    selector: &str,
    tag: &str,
    classes: &[String],
    id: Option<&str>,
    is_last_child: bool,
) -> bool {
    let selector = selector.trim();
    if selector == "*" { return true; }

    let (selector, has_last_child) = if let Some(stripped) = selector.strip_suffix(":last-child") {
        (stripped, true)
    } else {
        (selector, false)
    };

    let simple_sel = match selector.split_whitespace().last() {
        Some(last) => last,
        None => return false,
    };

    let mut required_tag: Option<&str> = None;
    let mut required_classes: Vec<&str> = Vec::new();
    let mut required_id: Option<&str> = None;

    let body = if let Some(pos) = simple_sel.find('#') {
        required_id = Some(&simple_sel[pos + 1..]);
        &simple_sel[..pos]
    } else {
        simple_sel
    };

    for (idx, part) in body.split('.').enumerate() {
        let p = part.trim();
        if p.is_empty() { continue; }
        if idx == 0 { required_tag = Some(p); }
        else { required_classes.push(p); }
    }

    if let Some(rt) = required_tag {
        if rt != tag && rt != "*" { return false; }
    }
    for rc in &required_classes {
        if !classes.iter().any(|c| c == rc) { return false; }
    }
    if let Some(ri) = required_id {
        match id { Some(v) if v == ri => {}, _ => return false }
    }
    if has_last_child && !is_last_child { return false; }

    true
}

#[cfg(test)]
#[path = "../test/css_test.rs"]
mod tests;
