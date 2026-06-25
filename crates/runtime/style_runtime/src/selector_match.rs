//! selector_match — 从右向左选择器匹配引擎。
//!
//! 选择器类型定义移入 dom_flat::selector_match。

use dom_flat::{
    AttrOp, BasicSelector, Combinator, ComplexSelector, DomNode,
    DomRegistry, NodeId, PseudoClass, SelectSegment,
};

/// ComplexSelector 匹配扩展 trait（因 Rust orphan rule，不能直接给 dom_flat 类型加 impl）。
pub trait SelectorMatchExt {
    fn matches(&self, registry: &DomRegistry, node_id: NodeId) -> bool;
}

impl SelectorMatchExt for ComplexSelector {
    fn matches(&self, registry: &DomRegistry, node_id: NodeId) -> bool {
        if self.segments.is_empty() {
            return false;
        }

        let mut current_id = Some(node_id);
        let mut seg_idx = self.segments.len();

        loop {
            if seg_idx == 0 {
                return true;
            }
            seg_idx -= 1;
            let seg = &self.segments[seg_idx];

            let id = match current_id {
                Some(i) => i,
                None => return false,
            };

            if !segment_matches(seg, registry, id) {
                return false;
            }

            if seg_idx == 0 {
                return true;
            }

            current_id = match seg.combinator {
                Combinator::Child => registry.allNodes.get(&id).and_then(|n| n.parentNode),
                Combinator::Adjacent => prev_sibling_element(registry, id),
                Combinator::Descendant => registry.allNodes.get(&id).and_then(|n| n.parentNode),
                Combinator::Sibling => prev_sibling_element(registry, id),
            };

            match seg.combinator {
                Combinator::Descendant | Combinator::Sibling => {
                    loop {
                        if current_id.is_none() {
                            return false;
                        }
                        let cid = current_id.unwrap();
                        if seg_idx > 0 {
                            let prev_seg = &self.segments[seg_idx - 1];
                            if segment_matches(prev_seg, registry, cid) {
                                seg_idx -= 1;
                                break;
                            }
                        }
                        current_id = match seg.combinator {
                            Combinator::Descendant => registry.allNodes.get(&cid).and_then(|n| n.parentNode),
                            Combinator::Sibling => prev_sibling_element(registry, cid),
                            _ => None,
                        };
                    }
                }
                _ => {}
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
//  匹配辅助函数
// ═══════════════════════════════════════════════════════════

fn segment_matches(seg: &SelectSegment, registry: &DomRegistry, node_id: NodeId) -> bool {
    let node = match registry.allNodes.get(&node_id) {
        Some(n) => n,
        None => return false,
    };

    if !node.is_element() {
        return false;
    }

    for part in &seg.parts {
        if !part_matches(part, node, registry, node_id) {
            return false;
        }
    }
    true
}

fn part_matches(part: &BasicSelector, node: &DomNode, registry: &DomRegistry, node_id: NodeId) -> bool {
    match part {
        BasicSelector::Tag(tag) => node.tagName.as_deref() == Some(tag.as_str()),
        BasicSelector::Id(id) => node.id.as_deref() == Some(id.as_str()),
        BasicSelector::Class(class) => node.classList.contains(class),
        BasicSelector::Universal => true,
        BasicSelector::Attribute { name, op, value } => attribute_matches(node, name, *op, value),
        BasicSelector::PseudoClass(pc) => pseudo_class_matches(pc, node, registry, node_id),
        BasicSelector::PseudoElement(_) => true,
    }
}

fn attribute_matches(node: &DomNode, name: &str, op: AttrOp, value: &str) -> bool {
    if !node.is_element() {
        return false;
    }
    let attr_val = node.attrs.iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str());

    match op {
        AttrOp::Has => attr_val.is_some(),
        AttrOp::Exact => attr_val == Some(value),
        AttrOp::Contains => attr_val.map_or(false, |v| v.contains(value)),
        AttrOp::StartsWith => attr_val.map_or(false, |v| v.starts_with(value)),
        AttrOp::EndsWith => attr_val.map_or(false, |v| v.ends_with(value)),
        AttrOp::WordMatch => attr_val.map_or(false, |v| {
            v.split_whitespace().any(|w| w == value)
        }),
        AttrOp::HyphenMatch => attr_val.map_or(false, |v| {
            v == value || v.starts_with(&format!("{}-", value))
        }),
    }
}

fn pseudo_class_matches(pc: &PseudoClass, node: &DomNode, registry: &DomRegistry, node_id: NodeId) -> bool {
    match pc {
        PseudoClass::Root => node.parentNode.is_none(),
        PseudoClass::Empty => node.childNodes.is_empty(),
        PseudoClass::FirstChild => {
            if let Some(parent_id) = node.parentNode {
                if let Some(parent) = registry.allNodes.get(&parent_id) {
                    return parent.childNodes.first() == Some(&node_id);
                }
            }
            false
        }
        PseudoClass::LastChild => {
            if let Some(parent_id) = node.parentNode {
                if let Some(parent) = registry.allNodes.get(&parent_id) {
                    return parent.childNodes.last() == Some(&node_id);
                }
            }
            false
        }
        PseudoClass::OnlyChild => {
            if let Some(parent_id) = node.parentNode {
                if let Some(parent) = registry.allNodes.get(&parent_id) {
                    return parent.childNodes.len() == 1;
                }
            }
            false
        }
        PseudoClass::FirstOfType => {
            is_nth_of_type(node, registry, node_id, 1)
        }
        PseudoClass::LastOfType => {
            is_last_of_type(node, registry, node_id)
        }
        PseudoClass::OnlyOfType => {
            is_nth_of_type(node, registry, node_id, 1) && is_last_of_type(node, registry, node_id)
        }
        PseudoClass::NthChild(a, b) => {
            let (a, b) = (*a, *b);
            let pos = sibling_position(node, registry, node_id, |_| true);
            matches_nth(pos, a, b)
        }
        PseudoClass::NthLastChild(a, b) => {
            let (a, b) = (*a, *b);
            let pos = sibling_position_reverse(node, registry, node_id, |_| true);
            matches_nth(pos, a, b)
        }
        PseudoClass::NthOfType(a, b) => {
            let (a, b) = (*a, *b);
            let tag = node.tagName.clone().unwrap_or_default();
            let pos = sibling_position(node, registry, node_id, |n| n.tagName == Some(tag.clone()));
            matches_nth(pos, a, b)
        }
        PseudoClass::NthLastOfType(a, b) => {
            let (a, b) = (*a, *b);
            let tag = node.tagName.clone().unwrap_or_default();
            let pos = sibling_position_reverse(node, registry, node_id, |n| n.tagName == Some(tag.clone()));
            matches_nth(pos, a, b)
        }
        PseudoClass::Not(inner) => !inner.matches(registry, node_id),
        PseudoClass::Is(list) => list.iter().any(|s| s.matches(registry, node_id)),
        PseudoClass::Where(list) => list.iter().any(|s| s.matches(registry, node_id)),
        PseudoClass::Has(inner) => {
            has_descendant_match(registry, node_id, inner)
        }
        // 动态伪类 — 暂返回 true（无状态存储时跳过）
        PseudoClass::Hover
        | PseudoClass::Active
        | PseudoClass::Focus
        | PseudoClass::FocusVisible
        | PseudoClass::FocusWithin
        | PseudoClass::Target
        | PseudoClass::Link
        | PseudoClass::Visited
        | PseudoClass::AnyLink
        | PseudoClass::Checked
        | PseudoClass::Disabled
        | PseudoClass::Enabled
        | PseudoClass::Required
        | PseudoClass::Optional
        | PseudoClass::ReadOnly
        | PseudoClass::ReadWrite
        | PseudoClass::Valid
        | PseudoClass::Invalid
        | PseudoClass::InRange
        | PseudoClass::OutOfRange
        | PseudoClass::PlaceholderShown
        | PseudoClass::Default
        | PseudoClass::Indeterminate => true,
        PseudoClass::Lang(l) => {
            node.attrs.iter().any(|(k, v)| k == "lang" && v == l)
        }
        PseudoClass::Dir(d) => {
            node.attrs.iter().any(|(k, v)| k == "dir" && v == d)
        }
    }
}

fn sibling_position<F>(node: &DomNode, registry: &DomRegistry, node_id: NodeId, filter: F) -> i32
where F: Fn(&DomNode) -> bool
{
    let parent_id = match node.parentNode {
        Some(id) => id,
        None => return 1,
    };
    let parent = match registry.allNodes.get(&parent_id) {
        Some(p) => p,
        None => return 1,
    };

    let mut pos = 0i32;
    for &child_id in &parent.childNodes {
        if let Some(child) = registry.allNodes.get(&child_id) {
            if child.is_element() && filter(child) {
                pos += 1;
                if child_id == node_id {
                    return pos;
                }
            }
        }
    }
    pos
}

fn sibling_position_reverse<F>(node: &DomNode, registry: &DomRegistry, node_id: NodeId, filter: F) -> i32
where F: Fn(&DomNode) -> bool
{
    let parent_id = match node.parentNode {
        Some(id) => id,
        None => return 1,
    };
    let parent = match registry.allNodes.get(&parent_id) {
        Some(p) => p,
        None => return 1,
    };

    let mut pos = 0i32;
    for &child_id in parent.childNodes.iter().rev() {
        if let Some(child) = registry.allNodes.get(&child_id) {
            if child.is_element() && filter(child) {
                pos += 1;
                if child_id == node_id {
                    return pos;
                }
            }
        }
    }
    pos
}

fn is_nth_of_type(node: &DomNode, registry: &DomRegistry, node_id: NodeId, target: i32) -> bool {
    let pos = sibling_position(node, registry, node_id, |n| {
        n.tagName == node.tagName
    });
    pos == target
}

fn is_last_of_type(node: &DomNode, registry: &DomRegistry, node_id: NodeId) -> bool {
    let parent_id = match node.parentNode {
        Some(id) => id,
        None => return true,
    };
    let parent = match registry.allNodes.get(&parent_id) {
        Some(p) => p,
        None => return true,
    };

    if let Some(last_of_type) = parent.childNodes.iter().rev()
        .filter_map(|&cid| registry.allNodes.get(&cid))
        .filter(|n| n.is_element() && n.tagName == node.tagName)
        .next()
    {
        last_of_type.nodeId == node_id
    } else {
        true
    }
}

fn matches_nth(pos: i32, a: i32, b: i32) -> bool {
    if a == 0 { return pos == b; }
    if pos < b { return false; }
    (pos - b) % a == 0
}

fn has_descendant_match(registry: &DomRegistry, root_id: NodeId, selector: &ComplexSelector) -> bool {
    let node = match registry.allNodes.get(&root_id) {
        Some(n) => n,
        None => return false,
    };
    for &child_id in &node.childNodes {
        if selector.matches(registry, child_id) {
            return true;
        }
        if has_descendant_match(registry, child_id, selector) {
            return true;
        }
    }
    false
}

fn prev_sibling_element(registry: &DomRegistry, node_id: NodeId) -> Option<NodeId> {
    let node = registry.allNodes.get(&node_id)?;
    let parent_id = node.parentNode?;
    let parent = registry.allNodes.get(&parent_id)?;

    let my_pos = parent.childNodes.iter().position(|&id| id == node_id)?;
    for i in (0..my_pos).rev() {
        let sid = parent.childNodes[i];
        if let Some(sibling) = registry.allNodes.get(&sid) {
            if sibling.is_element() {
                return Some(sid);
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "test/selector_match.test.rs"]
mod tests;
