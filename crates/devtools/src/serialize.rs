//! serialize — 将 DomRegistry 完整状态导出为 JSON。
//!
//! 手动构建 JSON 字符串，零依赖（不引入 serde）。
//! 供 DevTools 仪表盘和其他调试工具使用。

use runtime::{ComputedStyle, CssRule, CssStyleSheet, DomRegistry, NodeId};

/// 将 DomRegistry 的 DOM 树 + CSS 规则序列化为 JSON。
pub fn serialize_tree(registry: &DomRegistry) -> String {
    let mut json = String::with_capacity(4096);
    json.push_str("{\n");

    // ── DOM 树 ──
    json.push_str("  \"dom\": ");
    if let Some(body_id) = registry.bodyId {
        // 从 body 的父节点（html）开始，或直接从 body 开始
        let root_id = registry
            .allNodes
            .get(&body_id)
            .and_then(|n| n.parentNode)
            .unwrap_or(body_id);
        // 如果 parent 还有 parent（不太可能），继续往上
        let root_id = registry
            .allNodes
            .get(&root_id)
            .and_then(|n| n.parentNode)
            .unwrap_or(root_id);
        serialize_node(registry, root_id, &mut json, "  ");
    } else {
        json.push_str("null");
    }
    json.push_str(",\n");

    // ── CSS 规则 ──
    json.push_str("  \"rules\": ");
    serialize_rules(registry, &mut json, "  ");
    json.push_str("\n");

    json.push_str("}");
    json
}

/// 递归序列化单个 DOM 节点。
fn serialize_node(registry: &DomRegistry, node_id: NodeId, json: &mut String, indent: &str) {
    let node = match registry.allNodes.get(&node_id) {
        Some(n) => n,
        None => {
            json.push_str("null");
            return;
        }
    };

    json.push_str("{\n");
    let inner = format!("{}  ", indent);

    // nodeId
    write_str(&inner, json, "nodeId", &node_id.value().to_string());
    json.push_str(",\n");

    // nodeType: "element" | "text"
    let node_type = if node.is_text() { "text" } else { "element" };
    write_str(&inner, json, "nodeType", node_type);
    json.push_str(",\n");

    // tagName
    if let Some(ref tag) = node.tagName {
        write_str(&inner, json, "tagName", tag);
    } else {
        json.push_str(&format!("{}\"tagName\": null", inner));
    }
    json.push_str(",\n");

    // id
    if let Some(ref id) = node.id {
        write_str(&inner, json, "id", id);
    } else {
        json.push_str(&format!("{}\"id\": null", inner));
    }
    json.push_str(",\n");

    // classes
    json.push_str(&format!("{}\"classes\": [", inner));
    for (i, class) in node.classList.iter().enumerate() {
        if i > 0 {
            json.push_str(", ");
        }
        json.push_str(&json_str(class));
    }
    json.push_str("]");
    json.push_str(",\n");

    // text (文本节点)
    if node.is_text() {
        let text = node.text_content().unwrap_or("");
        write_str(&inner, json, "text", text);
        json.push_str(",\n");
    }

    // flags
    json.push_str(&format!("{}\"styleDirty\": {},", inner, node.styleDirty));
    json.push_str(&format!("\n{}\"layoutDirty\": {},", inner, node.layoutDirty));
    json.push_str(&format!("\n{}\"paintDirty\": {}", inner, node.paintDirty));
    json.push_str(",\n");

    // computedStyle
    json.push_str(&format!("{}\"computedStyle\": ", inner));
    serialize_computed_style(&node.computedStyle, json, &inner);
    json.push_str(",\n");

    // children
    json.push_str(&format!("{}\"children\": [", inner));
    for (i, &child_id) in node.childNodes.iter().enumerate() {
        if i > 0 {
            json.push_str(", ");
        }
        json.push('\n');
        serialize_node(registry, child_id, json, &format!("{}    ", inner));
    }
    if !node.childNodes.is_empty() {
        json.push('\n');
        json.push_str(&inner);
    }
    json.push(']');

    json.push('\n');
    json.push_str(indent);
    json.push('}');
}

/// 序列化 ComputedStyle。
fn serialize_computed_style(cs: &ComputedStyle, json: &mut String, indent: &str) {
    json.push('{');

    if cs.bgColor != 0 {
        json.push_str(&format!("\n{}  \"bgColor\": \"{}\"", indent, ComputedStyle::color_to_hex(cs.bgColor)));
        json.push(',');
    }
    if cs.textColor != 0 {
        json.push_str(&format!("\n{}  \"textColor\": \"{}\"", indent, ComputedStyle::color_to_hex(cs.textColor)));
        json.push(',');
    }
    if let Some(w) = cs.width {
        json.push_str(&format!("\n{}  \"width\": {}", indent, w));
        json.push(',');
    }
    if let Some(h) = cs.height {
        json.push_str(&format!("\n{}  \"height\": {}", indent, h));
        json.push(',');
    }
    if cs.padding != 0.0 {
        json.push_str(&format!("\n{}  \"padding\": {}", indent, cs.padding));
        json.push(',');
    }
    if cs.margin != 0.0 {
        json.push_str(&format!("\n{}  \"margin\": {}", indent, cs.margin));
        json.push(',');
    }
    if cs.fontSize != 0.0 {
        json.push_str(&format!("\n{}  \"fontSize\": {}", indent, cs.fontSize));
        json.push(',');
    }
    if let Some(ref ff) = cs.fontFamily {
        write_str_no_comma(indent, json, "fontFamily", ff);
        json.push(',');
    }
    if let Some(ref ta) = cs.textAlign {
        write_str_no_comma(indent, json, "textAlign", ta);
        json.push(',');
    }
    if let Some(ref td) = cs.textDecoration {
        write_str_no_comma(indent, json, "textDecoration", td);
        json.push(',');
    }

    // 去掉末尾逗号
    if json.ends_with(',') {
        json.pop();
    }

    if json.ends_with('{') {
        json.push('}');
    } else {
        json.push('\n');
        json.push_str(indent);
        json.push('}');
    }
}

/// 序列化所有样式表的 CSS 规则。
fn serialize_rules(registry: &DomRegistry, json: &mut String, _indent: &str) {
    json.push_str("[\n");

    let mut first = true;
    for sheet in registry.styleSheets.iter() {
        for rule in &sheet.cssRules {
            if !first {
                json.push_str(",\n");
            }
            first = false;
            serialize_one_rule(rule, sheet, json);
        }
    }

    if first {
        // 没有规则
    } else {
        json.push('\n');
    }
    json.push_str("  ]");
}

/// 序列化单条 CSS 规则。
fn serialize_one_rule(rule: &CssRule, sheet: &CssStyleSheet, json: &mut String) {
    json.push_str("    {\n");
    write_str("      ", json, "selectorText", &rule.selectorText);
    json.push_str(",\n");

    // 来源
    let source = sheet.href.as_deref().unwrap_or("<inline>");
    write_str("      ", json, "source", source);
    json.push_str(",\n");

    // declarations
    json.push_str("      \"declarations\": [");
    for (i, (prop, val)) in rule.style.iter().enumerate() {
        if i > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!("[\"{}\", \"{}\"]", json_escape(prop), json_escape(val)));
    }
    json.push(']');

    json.push('\n');
    json.push_str("    }");
}

// ── 辅助函数 ──

fn json_str(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn write_str(indent: &str, json: &mut String, key: &str, value: &str) {
    json.push_str(&format!("{}\"{}\": \"{}\"", indent, key, json_escape(value)));
}

fn write_str_no_comma(indent: &str, json: &mut String, key: &str, value: &str) {
    json.push_str(&format!("\n{}  \"{}\": \"{}\"", indent, key, json_escape(value)));
}
