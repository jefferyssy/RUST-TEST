//! selector_match — CSS 选择器类型定义 + 解析 + 匹配引擎。

use crate::specificity::Specificity;
use crate::{DomNode, DomRegistry, NodeId};

// ═══════════════════════════════════════════════════════════
//  基本选择器类型
// ═══════════════════════════════════════════════════════════

/// 属性选择器操作符。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttrOp {
    Has,          // [attr]
    Exact,        // [attr=val]
    Contains,     // [attr*=val]
    StartsWith,   // [attr^=val]
    EndsWith,     // [attr$=val]
    WordMatch,    // [attr~=val]
    HyphenMatch,  // [attr|=val]
}

/// 伪类。
#[derive(Debug, Clone, PartialEq)]
pub enum PseudoClass {
    Root,
    Empty,
    FirstChild,
    LastChild,
    OnlyChild,
    FirstOfType,
    LastOfType,
    OnlyOfType,
    NthChild(i32, i32),
    NthLastChild(i32, i32),
    NthOfType(i32, i32),
    NthLastOfType(i32, i32),
    Not(Box<ComplexSelector>),
    Is(Vec<ComplexSelector>),
    Where(Vec<ComplexSelector>),
    Has(Box<ComplexSelector>),
    Hover,
    Active,
    Focus,
    FocusVisible,
    FocusWithin,
    Target,
    Link,
    Visited,
    AnyLink,
    Checked,
    Disabled,
    Enabled,
    Required,
    Optional,
    ReadOnly,
    ReadWrite,
    Valid,
    Invalid,
    InRange,
    OutOfRange,
    PlaceholderShown,
    Default,
    Indeterminate,
    Lang(String),
    Dir(String),
}

/// 伪元素。
#[derive(Debug, Clone, PartialEq)]
pub enum PseudoElement {
    Before,
    After,
    FirstLine,
    FirstLetter,
    Marker,
    Placeholder,
    Selection,
    Backdrop,
}

/// 基本选择器片段。
#[derive(Debug, Clone, PartialEq)]
pub enum BasicSelector {
    Tag(String),
    Id(String),
    Class(String),
    Universal,
    Attribute { name: String, op: AttrOp, value: String },
    PseudoClass(PseudoClass),
    PseudoElement(PseudoElement),
}

/// CSS 组合器。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Combinator {
    Descendant,   // 空格
    Child,        // >
    Adjacent,     // +
    Sibling,      // ~
}

/// 一段选择器。
#[derive(Debug, Clone, PartialEq)]
pub struct SelectSegment {
    /// 到上一段的组合器（首段的组合器无意义，匹配时跳过）
    pub combinator: Combinator,
    pub parts: Vec<BasicSelector>,
}

/// 完整选择器。
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexSelector {
    pub segments: Vec<SelectSegment>,
    pub specificity: Specificity,
}

impl ComplexSelector {
    /// 从选择器字符串解析。
    pub fn parse(selector: &str) -> Option<Self> {
        let selector = selector.trim();
        if selector.is_empty() {
            return None;
        }

        let mut segments: Vec<SelectSegment> = Vec::new();
        let mut remaining = selector;
        let mut total_specificity = Specificity(0, 0, 0);

        let mut combinator = Combinator::Descendant;

        loop {
            remaining = remaining.trim_start();
            if remaining.is_empty() {
                break;
            }

            if remaining.starts_with('>') {
                combinator = Combinator::Child;
                remaining = remaining[1..].trim_start();
            } else if remaining.starts_with('+') {
                combinator = Combinator::Adjacent;
                remaining = remaining[1..].trim_start();
            } else if remaining.starts_with('~') {
                combinator = Combinator::Sibling;
                remaining = remaining[1..].trim_start();
            }

            let (parts, rest, seg_spec) = parse_compound_selector(remaining);
            if parts.is_empty() {
                break;
            }

            total_specificity.0 += seg_spec.0;
            total_specificity.1 += seg_spec.1;
            total_specificity.2 += seg_spec.2;

            segments.push(SelectSegment {
                combinator,
                parts,
            });

            remaining = rest;
            combinator = Combinator::Descendant;
        }

        if segments.is_empty() {
            return None;
        }

        Some(Self {
            segments,
            specificity: total_specificity,
        })
    }

    /// 获取最右段（建索引用）。
    pub fn last_segment(&self) -> Option<&SelectSegment> {
        self.segments.last()
    }
}

// ═══════════════════════════════════════════════════════════
//  ComplexSelector 匹配扩展 trait
// ═══════════════════════════════════════════════════════════

/// ComplexSelector 匹配扩展 trait。
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
//  解析辅助函数
// ═══════════════════════════════════════════════════════════

fn parse_compound_selector(s: &str) -> (Vec<BasicSelector>, &str, Specificity) {
    let mut parts = Vec::new();
    let mut remaining = s;
    let mut spec = Specificity(0, 0, 0);

    loop {
        if remaining.is_empty() {
            break;
        }

        let c = remaining.chars().next().unwrap();

        if c == '#' {
            let end = remaining[1..]
                .find(|ch: char| ch == '.' || ch == '#' || ch == ':' || ch == '[' || ch == ' ' || ch == '>' || ch == '+' || ch == '~')
                .map(|i| i + 1)
                .unwrap_or(remaining.len());
            let id = &remaining[1..end];
            if !id.is_empty() {
                parts.push(BasicSelector::Id(id.to_string()));
                spec.0 += 1;
            }
            remaining = &remaining[end..];
        } else if c == '.' {
            let end = remaining[1..]
                .find(|ch: char| ch == '.' || ch == '#' || ch == ':' || ch == '[' || ch == ' ' || ch == '>' || ch == '+' || ch == '~')
                .map(|i| i + 1)
                .unwrap_or(remaining.len());
            let class = &remaining[1..end];
            if !class.is_empty() {
                parts.push(BasicSelector::Class(class.to_string()));
                spec.1 += 1;
            }
            remaining = &remaining[end..];
        } else if c == ':' {
            let is_double = remaining.starts_with("::");
            let offset = if is_double { 2 } else { 1 };
            let name_end = remaining[offset..]
                .find(|ch: char| !ch.is_alphanumeric() && ch != '-')
                .unwrap_or(remaining.len() - offset);
            let name = &remaining[offset..offset + name_end];

            if is_double {
                if let Some(pe) = parse_pseudo_element(name) {
                    parts.push(BasicSelector::PseudoElement(pe));
                }
            } else {
                if let Some(pc) = parse_pseudo_class(name, &remaining[offset + name_end..]) {
                    let consumed = if matches!(&pc, PseudoClass::Not(_) | PseudoClass::Is(_) | PseudoClass::Where(_) | PseudoClass::Has(_) | PseudoClass::Lang(_) | PseudoClass::Dir(_)) {
                        find_paren_end(&remaining[offset + name_end..]).map(|i| i + 1).unwrap_or(0)
                    } else {
                        0
                    };
                    parts.push(BasicSelector::PseudoClass(pc));
                    spec.1 += 1;
                    remaining = &remaining[offset + name_end + consumed..];
                    continue;
                }
            }
            remaining = &remaining[offset + name_end..];
        } else if c == '[' {
            if let Some(bracket_end) = remaining.find(']') {
                let attr_content = &remaining[1..bracket_end];
                if let Some((name, op, value)) = parse_attribute(attr_content) {
                    parts.push(BasicSelector::Attribute { name, op, value });
                    spec.1 += 1;
                }
                remaining = &remaining[bracket_end + 1..];
            } else {
                break;
            }
        } else if c == '*' {
            parts.push(BasicSelector::Universal);
            remaining = &remaining[1..];
        } else if c == ' ' || c == '>' || c == '+' || c == '~' {
            break;
        } else {
            let end = remaining
                .find(|ch: char| ch == '.' || ch == '#' || ch == ':' || ch == '[' || ch == ' ' || ch == '>' || ch == '+' || ch == '~')
                .unwrap_or(remaining.len());
            let tag = &remaining[..end];
            if !tag.is_empty() {
                parts.push(BasicSelector::Tag(tag.to_lowercase()));
                spec.2 += 1;
            }
            remaining = &remaining[end..];
        }
    }

    (parts, remaining, spec)
}

fn parse_pseudo_element(name: &str) -> Option<PseudoElement> {
    match name.to_lowercase().as_str() {
        "before"       => Some(PseudoElement::Before),
        "after"        => Some(PseudoElement::After),
        "first-line"   => Some(PseudoElement::FirstLine),
        "first-letter" => Some(PseudoElement::FirstLetter),
        "marker"       => Some(PseudoElement::Marker),
        "placeholder"  => Some(PseudoElement::Placeholder),
        "selection"    => Some(PseudoElement::Selection),
        "backdrop"     => Some(PseudoElement::Backdrop),
        _ => None,
    }
}

fn parse_pseudo_class(name: &str, rest: &str) -> Option<PseudoClass> {
    let name_lower = name.to_lowercase();
    match name_lower.as_str() {
        "root"              => Some(PseudoClass::Root),
        "empty"             => Some(PseudoClass::Empty),
        "first-child"       => Some(PseudoClass::FirstChild),
        "last-child"        => Some(PseudoClass::LastChild),
        "only-child"        => Some(PseudoClass::OnlyChild),
        "first-of-type"     => Some(PseudoClass::FirstOfType),
        "last-of-type"      => Some(PseudoClass::LastOfType),
        "only-of-type"      => Some(PseudoClass::OnlyOfType),
        "hover"             => Some(PseudoClass::Hover),
        "active"            => Some(PseudoClass::Active),
        "focus"             => Some(PseudoClass::Focus),
        "focus-visible"     => Some(PseudoClass::FocusVisible),
        "focus-within"      => Some(PseudoClass::FocusWithin),
        "target"            => Some(PseudoClass::Target),
        "link"              => Some(PseudoClass::Link),
        "visited"           => Some(PseudoClass::Visited),
        "any-link"          => Some(PseudoClass::AnyLink),
        "checked"           => Some(PseudoClass::Checked),
        "disabled"          => Some(PseudoClass::Disabled),
        "enabled"           => Some(PseudoClass::Enabled),
        "required"          => Some(PseudoClass::Required),
        "optional"          => Some(PseudoClass::Optional),
        "read-only"         => Some(PseudoClass::ReadOnly),
        "read-write"        => Some(PseudoClass::ReadWrite),
        "valid"             => Some(PseudoClass::Valid),
        "invalid"           => Some(PseudoClass::Invalid),
        "in-range"          => Some(PseudoClass::InRange),
        "out-of-range"      => Some(PseudoClass::OutOfRange),
        "placeholder-shown" => Some(PseudoClass::PlaceholderShown),
        "default"           => Some(PseudoClass::Default),
        "indeterminate"     => Some(PseudoClass::Indeterminate),
        "nth-child"         => parse_nth_args(rest).map(|(a, b)| PseudoClass::NthChild(a, b)),
        "nth-last-child"    => parse_nth_args(rest).map(|(a, b)| PseudoClass::NthLastChild(a, b)),
        "nth-of-type"       => parse_nth_args(rest).map(|(a, b)| PseudoClass::NthOfType(a, b)),
        "nth-last-of-type"  => parse_nth_args(rest).map(|(a, b)| PseudoClass::NthLastOfType(a, b)),
        "not"               => parse_paren_selector(rest).map(|s| PseudoClass::Not(Box::new(s))),
        "is"                => parse_paren_selector_list(rest).map(PseudoClass::Is),
        "where"             => parse_paren_selector_list(rest).map(PseudoClass::Where),
        "has"               => parse_paren_selector(rest).map(|s| PseudoClass::Has(Box::new(s))),
        "lang"              => parse_paren_string(rest).map(PseudoClass::Lang),
        "dir"               => parse_paren_string(rest).map(PseudoClass::Dir),
        _ => None,
    }
}

fn parse_nth_args(s: &str) -> Option<(i32, i32)> {
    let inner = s.trim().strip_prefix('(')?.strip_suffix(')')?.trim();
    if inner == "even" { return Some((2, 0)); }
    if inner == "odd"  { return Some((2, 1)); }
    if let Ok(n) = inner.parse::<i32>() {
        return Some((0, n));
    }
    if let Some(pos) = inner.find('n') {
        let a_str = inner[..pos].trim();
        let a: i32 = if a_str.is_empty() || a_str == "+" { 1 }
                     else if a_str == "-" { -1 }
                     else { a_str.parse().ok()? };
        let b_str = inner[pos + 1..].trim();
        let b: i32 = if b_str.is_empty() { 0 }
                     else { b_str.trim_start_matches('+').parse().ok()? };
        return Some((a, b));
    }
    None
}

fn parse_paren_selector(s: &str) -> Option<ComplexSelector> {
    let inner = find_paren_content(s)?;
    ComplexSelector::parse(inner)
}

fn parse_paren_selector_list(s: &str) -> Option<Vec<ComplexSelector>> {
    let inner = find_paren_content(s)?;
    let mut list = Vec::new();
    for part in inner.split(',') {
        if let Some(sel) = ComplexSelector::parse(part.trim()) {
            list.push(sel);
        }
    }
    if list.is_empty() { None } else { Some(list) }
}

fn parse_paren_string(s: &str) -> Option<String> {
    let inner = find_paren_content(s)?;
    Some(inner.trim().trim_matches('"').trim_matches('\'').to_string())
}

fn find_paren_content(s: &str) -> Option<&str> {
    let s = s.trim().strip_prefix('(')?;
    find_paren_end(s).map(|i| &s[..i])
}

pub(crate) fn find_paren_end(s: &str) -> Option<usize> {
    let mut depth = 1i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => { depth -= 1; if depth == 0 { return Some(i); } }
            _ => {}
        }
    }
    None
}

fn parse_attribute(content: &str) -> Option<(String, AttrOp, String)> {
    let content = content.trim();
    let op_pos = content.find(|c: char| c == '=' || c == '~' || c == '|' || c == '^' || c == '$' || c == '*');

    if let Some(pos) = op_pos {
        let name = content[..pos].trim().to_string();
        let rest = &content[pos..];
        if let Some(eq_pos) = rest.find('=') {
            let op = match &rest[..eq_pos] {
                "~=" => AttrOp::WordMatch,
                "|=" => AttrOp::HyphenMatch,
                "^=" => AttrOp::StartsWith,
                "$=" => AttrOp::EndsWith,
                "*=" => AttrOp::Contains,
                _    => AttrOp::Exact,
            };
            let val = rest[eq_pos + 1..].trim().trim_matches('"').trim_matches('\'').to_string();
            Some((name, op, val))
        } else {
            Some((name, AttrOp::Has, String::new()))
        }
    } else {
        Some((content.to_string(), AttrOp::Has, String::new()))
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
