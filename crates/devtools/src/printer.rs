//! printer — DOM 树 + ComputedStyle 文本打印。
//!
//! 用于一期验证：将 DomRegistry 中的 DOM 结构和计算样式以可读文本输出。

use runtime::{ComputedStyle, DomRegistry, NodeId};

/// 打印整棵 DOM 树及每个节点的计算样式。
///
/// 输出格式：
/// ```text
/// DOM Tree:
/// └─ body
///    └─ div#app.container
///       ├─ h1              bgColor=#E8E8E8, fontSize=24
///       │  └─ Text: "计数器"
///       └─ button#inc_btn  bgColor=#007BFF
///          └─ Text: "+"
///
/// Computed Styles:
///   div#app:       bgColor=#F5F5F5, padding=20
///   h1:            bgColor=#E8E8E8, fontSize=24
///   button#inc_btn: bgColor=#007BFF, textColor=#FFFFFF
/// ```
pub fn print_dom_tree(registry: &DomRegistry, rootId: NodeId) -> String {
    let mut output = String::new();

    // ── DOM 树结构 ──
    output.push_str("=== DOM Tree ===\n");
    print_node(registry, rootId, "", true, &mut output);

    // ── 脏节点集合 ──
    output.push_str("\n=== Style Dirty Set ===\n");
    if registry.styleDirtySet.is_empty() {
        output.push_str("  (empty)\n");
    } else {
        let mut ids: Vec<&NodeId> = registry.styleDirtySet.iter().collect();
        ids.sort_by_key(|id| id.value());
        for id in ids {
            let desc = node_label(registry, *id);
            output.push_str(&format!("  {}  {}\n", id, desc));
        }
    }

    // ── 计算样式详情 ──
    output.push_str("\n=== Computed Styles ===\n");
    print_computed_styles(registry, rootId, &mut output);

    output
}

/// 递归打印节点。
fn print_node(
    registry: &DomRegistry,
    nodeId: NodeId,
    prefix: &str,
    is_last: bool,
    output: &mut String,
) {
    let node = match registry.allNodes.get(&nodeId) {
        Some(n) => n,
        None => return,
    };

    // 画树线
    let connector = if prefix.is_empty() {
        "".to_string()
    } else if is_last {
        format!("{}   └─ ", &prefix[..prefix.len() - 4])
    } else {
        format!("{}   ├─ ", &prefix[..prefix.len() - 4])
    };

    // 节点描述
    let label = node_label(registry, nodeId);

    // 样式摘要
    let style_summary = style_summary(&node.computedStyle);

    output.push_str(&format!("{}{}  {}\n", connector, label, style_summary));

    // 子节点
    let child_count = node.childNodes.len();
    let new_prefix = if prefix.is_empty() {
        "    ".to_string()
    } else if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}   │", prefix)
    };

    for (i, &child_id) in node.childNodes.iter().enumerate() {
        let is_last_child = i == child_count - 1;
        print_node(registry, child_id, &new_prefix, is_last_child, output);
    }
}

/// 生成节点标签（如 `div#app.container` 或 `Text: "hello"`）。
fn node_label(registry: &DomRegistry, nodeId: NodeId) -> String {
    let node = match registry.allNodes.get(&nodeId) {
        Some(n) => n,
        None => return format!("{} (deleted)", nodeId),
    };

    if node.is_text() {
        let text = node.text_content().unwrap_or("(empty)");
        return format!("Text: {:?}", text);
    }

    let mut label = node.tagName.clone().unwrap_or_else(|| "?".to_string());

    if let Some(ref id) = node.id {
        label.push_str(&format!("#{}", id));
    }
    for class in &node.classList {
        label.push_str(&format!(".{}", class));
    }

    // styleDirty 标记
    if node.styleDirty {
        label.push_str(" [dirty]");
    }

    label
}

/// 生成样式摘要字符串。
fn style_summary(style: &ComputedStyle) -> String {
    let mut parts: Vec<String> = Vec::new();

    if style.bgColor != 0 {
        parts.push(format!("bg={}", ComputedStyle::color_to_hex(style.bgColor)));
    }
    if style.textColor != 0 {
        parts.push(format!("fg={}", ComputedStyle::color_to_hex(style.textColor)));
    }
    if let Some(w) = style.width {
        parts.push(format!("w={}", w));
    }
    if let Some(h) = style.height {
        parts.push(format!("h={}", h));
    }
    if style.padding != 0.0 {
        parts.push(format!("pad={}", style.padding));
    }
    if style.margin != 0.0 {
        parts.push(format!("mar={}", style.margin));
    }
    if style.fontSize != 0.0 {
        parts.push(format!("fs={}", style.fontSize));
    }
    if let Some(ref ff) = style.fontFamily {
        parts.push(format!("ff={}", ff));
    }
    if let Some(ref ta) = style.textAlign {
        parts.push(format!("ta={}", ta));
    }
    if let Some(ref td) = style.textDecoration {
        parts.push(format!("td={}", td));
    }

    if parts.is_empty() {
        String::new()
    } else {
        parts.join(", ")
    }
}

/// 递归打印计算样式详情。
fn print_computed_styles(registry: &DomRegistry, nodeId: NodeId, output: &mut String) {
    let node = match registry.allNodes.get(&nodeId) {
        Some(n) => n,
        None => return,
    };

    if node.is_element() {
        let label = node_label(registry, nodeId);
        let style = &node.computedStyle;

        let mut parts: Vec<String> = Vec::new();

        if style.bgColor != 0 {
            parts.push(format!("bgColor={}", ComputedStyle::color_to_hex(style.bgColor)));
        }
        if style.textColor != 0 {
            parts.push(format!("textColor={}", ComputedStyle::color_to_hex(style.textColor)));
        }
        if style.fontSize != 0.0 {
            parts.push(format!("fontSize={}", style.fontSize));
        }
        if style.padding != 0.0 {
            parts.push(format!("padding={}", style.padding));
        }
        if style.margin != 0.0 {
            parts.push(format!("margin={}", style.margin));
        }
        if let Some(w) = style.width {
            parts.push(format!("width={}", w));
        }
        if let Some(h) = style.height {
            parts.push(format!("height={}", h));
        }
        if let Some(ref ff) = style.fontFamily {
            parts.push(format!("fontFamily={}", ff));
        }
        if let Some(ref ta) = style.textAlign {
            parts.push(format!("textAlign={}", ta));
        }
        if let Some(ref td) = style.textDecoration {
            parts.push(format!("textDecoration={}", td));
        }

        if !parts.is_empty() {
            output.push_str(&format!("  {:30}  {}\n", label, parts.join(", ")));
        }
    }

    for &child_id in &node.childNodes {
        print_computed_styles(registry, child_id, output);
    }
}

#[cfg(test)]
#[path = "test/printer.test.rs"]
mod tests;
