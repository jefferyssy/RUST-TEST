//! # CSS 处理器 — Phase 0
//!
//! 简化版 CSS 解析器，解析 CSS 规则并与 HTML 元素树匹配。
//! Phase 1+ 将替换为 cssparser + selectors crate。

/// CSS 规则
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selector: String,
    pub declarations: Vec<(String, String)>,
}

/// 去除 CSS 注释 /* ... */
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

/// 解析 CSS 规则列表
pub fn parse_css(css: &str) -> Vec<CssRule> {
    let cleaned = strip_css_comments(css);
    let mut rules = Vec::new();
    let bytes = cleaned.as_bytes();
    let len = cleaned.len();
    let mut i = 0;

    while i < len {
        // 跳过空白
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        // 读取选择器（直到 {）
        let sel_start = i;
        let mut brace_depth = 0;
        while i < len && brace_depth == 0 {
            if bytes[i] == b'{' {
                break;
            }
            if bytes[i] == b'(' {
                brace_depth += 1;
            }
            if bytes[i] == b')' {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
            }
            i += 1;
        }
        let selector = cleaned[sel_start..i].trim().to_string();
        if selector.is_empty() {
            i += 1;
            continue;
        }

        if i >= len || bytes[i] != b'{' {
            continue;
        }
        i += 1; // 跳过 {

        // 读取声明（直到 }）
        let decl_start = i;
        brace_depth = 0;
        while i < len {
            if bytes[i] == b'}' && brace_depth == 0 {
                break;
            }
            if bytes[i] == b'(' { brace_depth += 1; }
            if bytes[i] == b')' { if brace_depth > 0 { brace_depth -= 1; } }
            i += 1;
        }
        let decl_block = cleaned[decl_start..i].trim().to_string();
        if i < len { i += 1; } // 跳过 }

        // 解析声明块
        let declarations = parse_declarations(&decl_block);
        rules.push(CssRule { selector, declarations });
    }

    rules
}

/// 解析声明块（分号分隔的 property: value 对）
fn parse_declarations(block: &str) -> Vec<(String, String)> {
    let mut decls = Vec::new();
    let mut i = 0;
    let bytes = block.as_bytes();
    let len = block.len();

    while i < len {
        // 跳过空白和分号
        while i < len && (bytes[i].is_ascii_whitespace() || bytes[i] == b';') {
            i += 1;
        }
        if i >= len {
            break;
        }

        // 读取属性名（直到 :）
        let prop_start = i;
        while i < len && bytes[i] != b':' {
            i += 1;
        }
        let property = block[prop_start..i].trim().to_string();
        if i < len { i += 1; } // 跳过 :

        // 跳过空白
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        // 读取值（直到 ; 或块结束或 }）
        let val_start = i;
        let mut paren_depth = 0;
        while i < len {
            if paren_depth == 0 && (bytes[i] == b';' || bytes[i] == b'}') {
                break;
            }
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

/// 检查选择器是否与标签/类/ID 匹配
/// 支持: tag, .class, #id, tag.class, tag#id, tag.class#id
/// 支持伪类: :last-child（通过 is_last_child 参数判断）
pub fn selector_matches(selector: &str, tag: &str, classes: &[String], id: Option<&str>, is_last_child: bool) -> bool {
    let selector = selector.trim();

    // 通配符
    if selector == "*" {
        return true;
    }

    // 提取并移除 :last-child 伪类
    let (selector, has_last_child) = strip_last_child(selector);

    // 包含空格（后代选择器）— Phase 0 简化处理，只检查最后一个
    let simple_sel = match selector.split_whitespace().last() {
        Some(last) => last,
        None => return false,
    };

    // 拆分复合选择器为 tag/class/id 部分
    let mut required_tag: Option<&str> = None;
    let mut required_classes: Vec<&str> = Vec::new();
    let mut required_id: Option<&str> = None;

    // 提取 #id
    let body = if let Some(pos) = simple_sel.find('#') {
        required_id = Some(&simple_sel[pos + 1..]);
        &simple_sel[..pos]
    } else {
        simple_sel
    };

    // 按 . 分割：第一个非空段是标签，后续是类名
    for (idx, part) in body.split('.').enumerate() {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        if idx == 0 {
            required_tag = Some(p);
        } else {
            required_classes.push(p);
        }
    }

    // 检查标签
    if let Some(rt) = required_tag {
        if rt != tag && rt != "*" {
            return false;
        }
    }

    // 检查类
    for rc in &required_classes {
        if !classes.iter().any(|c| c == rc) {
            return false;
        }
    }

    // 检查 ID
    if let Some(ri) = required_id {
        match id {
            Some(id_val) if id_val == ri => {}
            _ => return false,
        }
    }

    // 检查 :last-child 条件
    if has_last_child && !is_last_child {
        return false;
    }

    true
}

/// 从选择器中剥离 :last-child 伪类，返回 (剩余选择器, 是否含有 :last-child)
fn strip_last_child(selector: &str) -> (&str, bool) {
    if let Some(stripped) = selector.strip_suffix(":last-child") {
        (stripped, true)
    } else {
        (selector, false)
    }
}

/// 检查元素是否为其父节点的最后一个子元素
fn is_last_child_in_tree(target: &super::HtmlElement, tree: &[super::HtmlElement]) -> bool {
    is_last_child_inner(target, tree).unwrap_or(false)
}

fn is_last_child_inner(target: &super::HtmlElement, tree: &[super::HtmlElement]) -> Option<bool> {
    for (i, el) in tree.iter().enumerate() {
        if el.tag == target.tag && el.attributes == target.attributes && el.text_content == target.text_content {
            return Some(i == tree.len() - 1);
        }
        if let Some(result) = is_last_child_inner(target, &el.children) {
            return Some(result);
        }
    }
    None
}

/// 将 CSS 规则匹配到元素树，返回 (variable_name, style_string) 列表
pub fn match_css_to_elements(
    css_rules: &[CssRule],
    elements: &[super::HtmlElement],
    element_vars: &[(String, super::HtmlElement)],
) -> Vec<(String, String)> {
    let mut results: Vec<(String, Vec<(String, String)>)> = Vec::new();

    for (var_name, el) in element_vars {
        let mut matched_decls: Vec<(String, String)> = Vec::new();
        for rule in css_rules {
            let classes: Vec<String> = el.attributes.get("class")
                .map(|c| c.split_whitespace().map(String::from).collect())
                .unwrap_or_default();
            let id = el.attributes.get("id").map(|s| s.as_str());

            if selector_matches(&rule.selector, &el.tag, &classes, id, is_last_child_in_tree(el, elements)) {
                // 后代选择器需要验证祖先部分是否匹配
                if !check_descendant_ancestor(el, &rule.selector, elements) {
                    continue;
                }
                for decl in &rule.declarations {
                    matched_decls.push(decl.clone());
                }
            }
        }
        if !matched_decls.is_empty() {
            results.push((var_name.clone(), matched_decls));
        }
    }

    results.into_iter()
        .map(|(var, decls)| {
            let style_str = decls.iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join("; ");
            (var, style_str)
        })
        .collect()
}

/// 对于包含空格的后代选择器（如 `.todo-item span`），验证元素的祖先
/// 是否匹配选择器的祖先部分。单层选择器直接返回 true。
fn check_descendant_ancestor(
    el: &super::HtmlElement,
    selector: &str,
    tree: &[super::HtmlElement],
) -> bool {
    let parts: Vec<&str> = selector.split_whitespace().collect();
    if parts.len() <= 1 {
        return true; // 不是后代选择器，无需检查
    }

    // 祖先选择器 = 除最后一部分之外的所有部分
    let ancestor_sel = parts[..parts.len() - 1].join(" ");

    // 在树中查找 el 的父元素，检查父元素是否匹配祖先选择器
    find_and_check_ancestor(el, &ancestor_sel, tree)
}

/// 在树中递归搜索 el，找到其父元素后检查父元素是否匹配 ancestor_sel
fn find_and_check_ancestor(
    target: &super::HtmlElement,
    ancestor_sel: &str,
    siblings: &[super::HtmlElement],
) -> bool {
    for parent in siblings {
        // 检查 target 是否为当前 parent 的直接子元素
        for child in &parent.children {
            if element_same(child, target) {
                // 找到了！检查 parent 是否匹配祖先选择器
                let classes: Vec<String> = parent.attributes.get("class")
                    .map(|c| c.split_whitespace().map(String::from).collect())
                    .unwrap_or_default();
                let id = parent.attributes.get("id").map(|s| s.as_str());
                return selector_matches(ancestor_sel, &parent.tag, &classes, id, false);
            }
        }
        // 递归搜索更深层
        if find_and_check_ancestor(target, ancestor_sel, &parent.children) {
            return true;
        }
    }
    false
}

/// 比较两个 HtmlElement 是否代表同一个元素（按 tag + id 属性）
fn element_same(a: &super::HtmlElement, b: &super::HtmlElement) -> bool {
    a.tag == b.tag && a.attributes.get("id") == b.attributes.get("id")
}

#[cfg(test)]
#[path = "css.test.rs"]
mod tests;
