//! CSS 选择器匹配引擎
//!
//! Phase 0 支持三种基本选择器：
//! - 标签选择器：`div`, `h1`, `button`
//! - 类选择器：`.container`, `.active`
//! - ID 选择器：`#main`, `#header`
//!
//! Phase 1: SelectorEngine 查询子树
//! Phase 2: 完整伪类支持
//! Phase 3: 组合器 + 属性选择器 + :has/:is/:where + 伪元素

use std::cell::RefCell;
use std::rc::Rc;

use dom::{self, ElementData, NodeType};

use crate::stylesheet::{Declaration, StyleSheet};

// ============================================================
//  Phase 3: 组合器
// ============================================================

/// 组合器类型 (Phase 3 新增)
#[derive(Debug, Clone, PartialEq)]
pub enum Combinator {
    /// 后代: A B (空格)
    Descendant,
    /// 子代: A > B
    Child,
    /// 相邻兄弟: A + B
    AdjacentSibling,
    /// 通用兄弟: A ~ B
    GeneralSibling,
}

/// 复合选择器片段（带组合器）
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorSegment {
    pub parts: Vec<SelectorPart>,
    /// 与上一段的连接方式（第一段为 Descendant）
    pub combinator: Combinator,
}

/// 选择器解析结果（Phase 3 升级为多段）
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSelector {
    pub segments: Vec<SelectorSegment>,
}

// ============================================================
//  Phase 3: 属性选择器
// ============================================================

/// 属性选择器操作符 (Phase 3 新增)
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeOp {
    /// [attr] — 属性存在
    Exists,
    /// [attr=value] — 完全匹配
    Equals(String),
    /// [attr~=value] — 空格分隔的词列表中包含
    Contains(String),
    /// [attr^=value] — 前缀匹配
    StartsWith(String),
    /// [attr$=value] — 后缀匹配
    EndsWith(String),
    /// [attr*=value] — 包含子串
    ContainsSubstring(String),
}

// ============================================================
//  Phase 3: 伪元素
// ============================================================

/// 伪元素类型 (Phase 3 新增)
#[derive(Debug, Clone, PartialEq)]
pub enum PseudoElement {
    Before,
    After,
}

/// 选择器匹配结果：特异性 + 声明
#[derive(Debug, Clone)]
pub struct MatchedDeclaration {
    /// 特异性值 (ID, Class, Tag 三元组)
    pub specificity: (u32, u32, u32),
    /// 匹配的声明
    pub declaration: Declaration,
}

/// 伪类 (Phase 2 + Phase 3 扩展)
#[derive(Debug, Clone, PartialEq)]
pub enum PseudoClass {
    /// 交互状态
    Hover,
    Active,
    Focus,
    /// 链接相关
    Visited,
    Link,
    /// 结构化
    Root,
    Empty,
    FirstChild,
    LastChild,
    OnlyChild,
    FirstOfType,
    LastOfType,
    OnlyOfType,
    NthChild(i32, i32),      // an + b
    NthLastChild(i32, i32),  // an + b (from end)
    NthOfType(i32, i32),
    NthLastOfType(i32, i32),
    /// 否定
    Not(Box<SelectorPart>),
    /// 其他
    Enabled,
    Disabled,
    Checked,
    Custom(String),
    // Phase 3 新增: 关系型伪类
    /// :has(selector) — 包含匹配子元素的父元素
    Has(Box<ParsedSelector>),
    /// :is(selectors) — 匹配列表中任一选择器
    Is(Vec<ParsedSelector>),
    /// :where(selectors) — 同 :is() 但特异性始终为 0
    Where(Vec<ParsedSelector>),
    // Phase 3 新增: 焦点扩展
    /// :focus-visible — 仅键盘焦点
    FocusVisible,
    /// :focus-within — 自身或任意后代有焦点
    FocusWithin,
}

/// 选择器片段类型 (Phase 2 扩展 + Phase 3 属性选择器)
#[derive(Debug, Clone, PartialEq)]
pub enum SelectorPart {
    Tag(String),
    Class(String),
    Id(String),
    /// 属性选择器 (Phase 3 新增)
    Attribute { name: String, op: AttributeOp },
    PseudoClass(PseudoClass),
}

/// 针对单个元素的匹配：找出所有匹配的规则声明 (ElementData 版本)
///
/// element: 目标元素数据
/// stylesheets: 全局样式表列表
/// 返回所有匹配的声明（含特异性权重）
pub fn match_selectors(
    element: &ElementData,
    stylesheets: &[StyleSheet],
) -> Vec<MatchedDeclaration> {
    let mut results = Vec::new();
    for sheet in stylesheets {
        for rule in &sheet.rules {
            for selector in &rule.selectors {
                if element_matches_selector(element, selector) {
                    let specificity = compute_specificity(selector);
                    for decl in &rule.declarations {
                        results.push(MatchedDeclaration {
                            specificity,
                            declaration: decl.clone(),
                        });
                    }
                }
            }
        }
    }
    results
}

/// Phase 2: 带 Node 上下文的完整匹配（支持结构伪类）
pub fn match_selectors_full(
    node: &Rc<RefCell<dom::Node>>,
    stylesheets: &[StyleSheet],
) -> Vec<MatchedDeclaration> {
    let mut results = Vec::new();
    for sheet in stylesheets {
        for rule in &sheet.rules {
            for selector in &rule.selectors {
                if element_matches_selector_with_node(node, selector) {
                    let specificity = compute_specificity(selector);
                    for decl in &rule.declarations {
                        results.push(MatchedDeclaration {
                            specificity,
                            declaration: decl.clone(),
                        });
                    }
                }
            }
        }
    }
    results
}

/// 判断元素是否匹配选择器字符串 (ElementData 版本，不含伪类)
///
/// Phase 0: 支持 tag, .class, #id
/// Phase 2: 伪类需要 Node 上下文，使用 element_matches_selector_with_node
pub fn element_matches_selector(element: &ElementData, selector: &str) -> bool {
    let selector = selector.trim();
    if selector == "*" {
        return true;
    }
    let parts = parse_selector_parts(selector);
    for part in parts {
        if !element_matches_selector_part(element, &part) {
            return false;
        }
    }
    true
}

/// Phase 2: 带 Node 上下文的完整匹配（支持所有伪类）
pub fn element_matches_selector_with_node(
    node: &Rc<RefCell<dom::Node>>,
    selector: &str,
) -> bool {
    let selector = selector.trim();
    if selector == "*" {
        return true;
    }

    let n = node.borrow();
    let parts = parse_selector_parts(selector);

    for part in &parts {
        let matched = match &n.node_type {
            NodeType::Element(elem) => {
                if matches_pseudo_class_node(node, &part) {
                    true
                } else {
                    element_matches_selector_part(elem, part)
                }
            }
            _ => false,
        };
        if !matched {
            return false;
        }
    }
    true
}

#[derive(PartialEq)]
enum Mode { Tag, Class, Id }

/// 解析选择器片段（含伪类、属性选择器）
pub fn parse_selector_parts(selector: &str) -> Vec<SelectorPart> {
    let mut parts = Vec::new();
    let mut current = String::new();

    let mut mode = Mode::Tag;
    let mut in_pseudo = false;
    let mut pseudo_depth = 0;
    let mut in_attr = false; // Phase 3: [attr] 状态
    let mut attr_buffer = String::new();

    for ch in selector.chars() {
        if in_attr {
            // Phase 3: 属性选择器解析
            if ch == ']' {
                in_attr = false;
                if let Some(attr_part) = parse_attribute_selector(&attr_buffer) {
                    parts.push(attr_part);
                }
                attr_buffer.clear();
            } else {
                attr_buffer.push(ch);
            }
            continue;
        }

        match ch {
            '.' if !in_pseudo => {
                flush_part_mode(&mut current, &mode, &mut parts);
                mode = Mode::Class;
            }
            '#' if !in_pseudo => {
                flush_part_mode(&mut current, &mode, &mut parts);
                mode = Mode::Id;
            }
            '[' if !in_pseudo => {
                flush_part_mode(&mut current, &mode, &mut parts);
                in_attr = true;
                mode = Mode::Tag;
            }
            ':' if !in_pseudo => {
                flush_part_mode(&mut current, &mode, &mut parts);
                in_pseudo = true;
                mode = Mode::Tag;
            }
            '(' if in_pseudo => {
                pseudo_depth += 1;
                current.push(ch);
            }
            ')' if in_pseudo => {
                pseudo_depth -= 1;
                current.push(ch);
                if pseudo_depth == 0 {
                    let pseudo = parse_pseudo_class(&current);
                    parts.push(SelectorPart::PseudoClass(pseudo));
                    current.clear();
                    in_pseudo = false;
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if in_pseudo && !current.is_empty() {
        let pseudo = parse_pseudo_class(&current);
        parts.push(SelectorPart::PseudoClass(pseudo));
    } else if !current.is_empty() {
        flush_part_mode(&mut current, &mode, &mut parts);
    }

    parts
}

/// Phase 3: 解析属性选择器内容 (attr, attr=val, attr~=val 等)
fn parse_attribute_selector(attr_str: &str) -> Option<SelectorPart> {
    let attr_str = attr_str.trim();
    if attr_str.is_empty() {
        return None;
    }

    // 尝试匹配各类操作符
    for (op_char, constructor) in &[
        ("~=", AttributeOp::Contains as fn(String) -> AttributeOp),
        ("^=", |v| AttributeOp::StartsWith(v)),
        ("$=", |v| AttributeOp::EndsWith(v)),
        ("*=", |v| AttributeOp::ContainsSubstring(v)),
        ("=", |v| AttributeOp::Equals(v)),
    ] {
        if let Some((name, val)) = attr_str.split_once(op_char) {
            let name = name.trim().to_string();
            let val = val.trim().to_string();
            if !name.is_empty() {
                return Some(SelectorPart::Attribute {
                    name,
                    op: constructor(val),
                });
            }
        }
    }

    // 仅属性存在: [attr]
    Some(SelectorPart::Attribute {
        name: attr_str.to_string(),
        op: AttributeOp::Exists,
    })
}

fn flush_part_mode(current: &mut String, mode: &Mode, parts: &mut Vec<SelectorPart>) {
    if !current.is_empty() {
        match mode {
            Mode::Tag => parts.push(SelectorPart::Tag(current.clone())),
            Mode::Class => parts.push(SelectorPart::Class(current.clone())),
            Mode::Id => parts.push(SelectorPart::Id(current.clone())),
        }
        current.clear();
    }
}

/// 解析伪类字符串（不含前置冒号）
fn parse_pseudo_class(s: &str) -> PseudoClass {
    let s = s.trim();
    // :not(selector)
    if let Some(inner) = s.strip_prefix("not(").and_then(|r| r.strip_suffix(')')) {
        let inner_parts = parse_selector_parts(inner);
        if let Some(first) = inner_parts.into_iter().next() {
            return PseudoClass::Not(Box::new(first));
        }
        return PseudoClass::Not(Box::new(SelectorPart::Tag("*".to_string())));
    }
    // :nth-child(an+b)
    if let Some(inner) = s.strip_prefix("nth-child(").and_then(|r| r.strip_suffix(')')) {
        let (a, b) = parse_an_b(inner);
        return PseudoClass::NthChild(a, b);
    }
    if let Some(inner) = s.strip_prefix("nth-last-child(").and_then(|r| r.strip_suffix(')')) {
        let (a, b) = parse_an_b(inner);
        return PseudoClass::NthLastChild(a, b);
    }
    if let Some(inner) = s.strip_prefix("nth-of-type(").and_then(|r| r.strip_suffix(')')) {
        let (a, b) = parse_an_b(inner);
        return PseudoClass::NthOfType(a, b);
    }
    if let Some(inner) = s.strip_prefix("nth-last-of-type(").and_then(|r| r.strip_suffix(')')) {
        let (a, b) = parse_an_b(inner);
        return PseudoClass::NthLastOfType(a, b);
    }
    // Phase 3: :has(selector)
    if let Some(inner) = s.strip_prefix("has(").and_then(|r| r.strip_suffix(')')) {
        let parsed = parse_selector(inner);
        return PseudoClass::Has(Box::new(parsed.0));
    }
    // Phase 3: :is(selectors)
    if let Some(inner) = s.strip_prefix("is(").and_then(|r| r.strip_suffix(')')) {
        let selectors = parse_selector_list(inner);
        return PseudoClass::Is(selectors);
    }
    // Phase 3: :where(selectors)
    if let Some(inner) = s.strip_prefix("where(").and_then(|r| r.strip_suffix(')')) {
        let selectors = parse_selector_list(inner);
        return PseudoClass::Where(selectors);
    }
    // 无参数伪类
    match s {
        "hover" => PseudoClass::Hover,
        "active" => PseudoClass::Active,
        "focus" => PseudoClass::Focus,
        "visited" => PseudoClass::Visited,
        "link" => PseudoClass::Link,
        "root" => PseudoClass::Root,
        "empty" => PseudoClass::Empty,
        "first-child" => PseudoClass::FirstChild,
        "last-child" => PseudoClass::LastChild,
        "only-child" => PseudoClass::OnlyChild,
        "first-of-type" => PseudoClass::FirstOfType,
        "last-of-type" => PseudoClass::LastOfType,
        "only-of-type" => PseudoClass::OnlyOfType,
        "enabled" => PseudoClass::Enabled,
        "disabled" => PseudoClass::Disabled,
        "checked" => PseudoClass::Checked,
        "focus-visible" => PseudoClass::FocusVisible,
        "focus-within" => PseudoClass::FocusWithin,
        _ => PseudoClass::Custom(s.to_string()),
    }
}

/// 解析 an+b 语法 (nth-child 参数)
fn parse_an_b(s: &str) -> (i32, i32) {
    let s = s.trim();
    if s == "even" {
        return (2, 0);
    }
    if s == "odd" {
        return (2, 1);
    }
    // 尝试解析纯数字 "3"
    if let Ok(n) = s.parse::<i32>() {
        return (0, n);
    }
    // 尝试解析 "2n+1", "-n+3", "3n" 等
    let s = s.replace(' ', "");
    if let Some((a_str, b_str)) = s.split_once('n') {
        let a: i32 = match a_str.trim() {
            "" | "+" => 1,
            "-" => -1,
            other => other.parse().unwrap_or(1),
        };
        let b: i32 = if b_str.is_empty() {
            0
        } else {
            b_str.trim().parse().unwrap_or(0)
        };
        return (a, b);
    }
    (0, 0)
}

/// 检查元素是否匹配某个选择器片段
fn element_matches_selector_part(element: &ElementData, part: &SelectorPart) -> bool {
    match part {
        SelectorPart::Tag(tag) => element.tag_name() == tag.as_str(),
        SelectorPart::Class(class) => element.has_class(class),
        SelectorPart::Id(id) => element.id() == Some(id),
        SelectorPart::Attribute { name, op } => match_attribute(element, name, op),
        // 基础伪类（不需要 Node 上下文的）
        SelectorPart::PseudoClass(pc) => matches_simple_pseudo_class(element, pc),
    }
}

/// Phase 3: 属性选择器匹配
fn match_attribute(element: &ElementData, name: &str, op: &AttributeOp) -> bool {
    match op {
        AttributeOp::Exists => element.has_attribute(name),
        AttributeOp::Equals(val) => element.get_attribute(name).as_deref() == Some(val.as_str()),
        AttributeOp::Contains(val) => {
            element.get_attribute(name)
                .map(|v| v.split_whitespace().any(|w| w == val.as_str()))
                .unwrap_or(false)
        }
        AttributeOp::StartsWith(val) => {
            element.get_attribute(name)
                .map(|v| v.starts_with(val.as_str()))
                .unwrap_or(false)
        }
        AttributeOp::EndsWith(val) => {
            element.get_attribute(name)
                .map(|v| v.ends_with(val.as_str()))
                .unwrap_or(false)
        }
        AttributeOp::ContainsSubstring(val) => {
            element.get_attribute(name)
                .map(|v| v.contains(val.as_str()))
                .unwrap_or(false)
        }
    }
}

/// 匹配不需要父节点上下文的简单伪类
fn matches_simple_pseudo_class(element: &ElementData, pc: &PseudoClass) -> bool {
    match pc {
        PseudoClass::Hover | PseudoClass::Active | PseudoClass::Focus => {
            // Phase 2: 交互状态由运行时维护
            element.is_focused()
        }
        PseudoClass::Enabled => !element.has_attribute("disabled"),
        PseudoClass::Disabled => element.has_attribute("disabled"),
        PseudoClass::Checked => element.has_attribute("checked"),
        PseudoClass::Root => element.tag_name() == "html",
        PseudoClass::Link => element.tag_name() == "a" && element.has_attribute("href"),
        PseudoClass::Visited => element.tag_name() == "a" && element.has_attribute("href"),
        // Not 伪类：内部选择器不匹配
        PseudoClass::Not(inner) => !element_matches_selector_part(element, inner),
        // 结构化伪类返回 true（需要 Node 上下文做精确判断）
        _ => true,
    }
}

/// Phase 2: 带 Node 上下文的结构化伪类匹配
fn matches_pseudo_class_node(
    node: &Rc<RefCell<dom::Node>>,
    part: &SelectorPart,
) -> bool {
    match part {
        SelectorPart::PseudoClass(pc) => match pc {
            PseudoClass::Empty => {
                let n = node.borrow();
                n.child_nodes().is_empty()
            }
            PseudoClass::FirstChild => is_nth_child(node, 0, 1),
            PseudoClass::LastChild => is_nth_last_child(node, 0, 1),
            PseudoClass::OnlyChild => {
                let n = node.borrow();
                if let Some(parent) = n.parent_node() {
                    parent.borrow().child_nodes().len() == 1
                } else {
                    false
                }
            }
            PseudoClass::FirstOfType => is_nth_of_type(node, 0, 1),
            PseudoClass::LastOfType => is_nth_last_of_type(node, 0, 1),
            PseudoClass::OnlyOfType => {
                let n = node.borrow();
                let tag = n.tag_name();
                let same_type_count = n.parent_node()
                    .map(|p| {
                        p.borrow().child_nodes().iter()
                            .filter(|c| c.borrow().tag_name() == tag)
                            .count()
                    })
                    .unwrap_or(0);
                same_type_count == 1
            }
            PseudoClass::NthChild(a, b) => is_nth_child(node, *a, *b),
            PseudoClass::NthLastChild(a, b) => is_nth_last_child(node, *a, *b),
            PseudoClass::NthOfType(a, b) => is_nth_of_type(node, *a, *b),
            PseudoClass::NthLastOfType(a, b) => is_nth_last_of_type(node, *a, *b),
            // Phase 3: :has(selector) — 检查是否存在匹配的后代
            PseudoClass::Has(selector) => {
                has_matching_descendant(node, selector)
            }
            // Phase 3: :is(selectors) — 匹配列表中任一选择器
            PseudoClass::Is(selectors) => {
                selectors.iter().any(|s| matches_complex_selector(node, s))
            }
            // Phase 3: :where(selectors) — 同 :is() 但特异性为 0
            PseudoClass::Where(selectors) => {
                selectors.iter().any(|s| matches_complex_selector(node, s))
            }
            // Phase 3: :focus-visible — 仅键盘焦点
            PseudoClass::FocusVisible => {
                let n = node.borrow();
                match &n.node_type {
                    NodeType::Element(e) => e.is_focused(),
                    _ => false,
                }
            }
            // Phase 3: :focus-within — 自身或后代有焦点
            PseudoClass::FocusWithin => {
                node_is_focused_or_has_focused_descendant(node)
            }
            _ => false, // 已由 matches_simple_pseudo_class 处理
        },
        _ => false,
    }
}

/// Phase 3: 在后代中搜索匹配 Has 选择器的元素
fn has_matching_descendant(
    node: &Rc<RefCell<dom::Node>>,
    selector: &ParsedSelector,
) -> bool {
    let children = node.borrow().child_nodes();
    for child in &children {
        if matches_complex_selector(child, selector) {
            return true;
        }
        if has_matching_descendant(child, selector) {
            return true;
        }
    }
    false
}

/// Phase 3: :focus-within 辅助 — 自身或后代是否有焦点
fn node_is_focused_or_has_focused_descendant(node: &Rc<RefCell<dom::Node>>) -> bool {
    let is_focused = {
        let n = node.borrow();
        match &n.node_type {
            NodeType::Element(e) => e.is_focused(),
            _ => false,
        }
    };
    if is_focused {
        return true;
    }
    let children = node.borrow().child_nodes();
    for child in &children {
        if node_is_focused_or_has_focused_descendant(child) {
            return true;
        }
    }
    false
}

/// 检查节点是否为父节点中的第 N 个孩子（1-based index）
fn child_index(node: &Rc<RefCell<dom::Node>>) -> Option<usize> {
    let n = node.borrow();
    let parent = n.parent_node()?;
    let p = parent.borrow();
    // 比较 Rc 指针地址
    p.child_nodes().iter()
        .position(|c| Rc::as_ptr(c) == Rc::as_ptr(node))
}

fn is_nth_child(node: &Rc<RefCell<dom::Node>>, a: i32, b: i32) -> bool {
    let idx = match child_index(node) {
        Some(i) => i as i32 + 1, // 1-based
        None => return false,
    };
    matches_an_b(a, b, idx)
}

fn is_nth_last_child(node: &Rc<RefCell<dom::Node>>, a: i32, b: i32) -> bool {
    let n = node.borrow();
    let parent = match n.parent_node() {
        Some(p) => p,
        None => return false,
    };
    let total = parent.borrow().child_nodes().len() as i32;
    let idx = match child_index(node) {
        Some(i) => i as i32 + 1,
        None => return false,
    };
    let from_end = total - idx + 1;
    matches_an_b(a, b, from_end)
}

fn is_nth_of_type(node: &Rc<RefCell<dom::Node>>, a: i32, b: i32) -> bool {
    let n = node.borrow();
    let tag = n.tag_name();
    let parent = match n.parent_node() {
        Some(p) => p,
        None => return false,
    };
    let p = parent.borrow();
    let idx = p.child_nodes().iter()
        .filter(|c| c.borrow().tag_name() == tag)
        .position(|c| Rc::as_ptr(&c) == Rc::as_ptr(node))
        .map(|i| i as i32 + 1)
        .unwrap_or(0);
    matches_an_b(a, b, idx)
}

fn is_nth_last_of_type(node: &Rc<RefCell<dom::Node>>, a: i32, b: i32) -> bool {
    let n = node.borrow();
    let tag = n.tag_name();
    let parent = match n.parent_node() {
        Some(p) => p,
        None => return false,
    };
    let p = parent.borrow();
    let total = p.child_nodes().iter()
        .filter(|c| c.borrow().tag_name() == tag)
        .count() as i32;
    let idx = p.child_nodes().iter()
        .filter(|c| c.borrow().tag_name() == tag)
        .position(|c| Rc::as_ptr(&c) == Rc::as_ptr(node))
        .map(|i| i as i32 + 1)
        .unwrap_or(0);
    matches_an_b(a, b, total - idx + 1)
}

/// 判断 index 是否匹配 an+b 模式
fn matches_an_b(a: i32, b: i32, index: i32) -> bool {
    if a == 0 {
        return index == b;
    }
    let diff = index - b;
    if diff < 0 {
        return false;
    }
    diff % a == 0
}

/// 计算选择器特异性 (ID, Class, Tag)
///
/// 规则：ID 选择器 > Class/属性/伪类 > Tag/伪元素
/// 返回 (ID数, Class数, Tag数) 三元组
pub fn compute_specificity(selector: &str) -> (u32, u32, u32) {
    let mut id_count = 0u32;
    let mut class_count = 0u32;
    let mut tag_count = 0u32;

    let parts = parse_selector_parts(selector);
    for part in parts {
        match part {
            SelectorPart::Id(_) => id_count += 1,
            SelectorPart::Class(_) => class_count += 1,
            SelectorPart::Attribute { .. } => class_count += 1, // Phase 3
            SelectorPart::PseudoClass(_) => class_count += 1,
            SelectorPart::Tag(_) => tag_count += 1,
        }
    }
    (id_count, class_count, tag_count)
}

/// Phase 3: 计算 ParsedSelector 的特异性
///
/// :is() 和 :not() 使用内部最特异的选择器的特异性
/// :where() 和 :has() 的特异性始终为 0
/// ::before/::after 不计特异性
pub fn compute_specificity_parsed(selector: &ParsedSelector) -> (u32, u32, u32) {
    let mut id_count = 0u32;
    let mut class_count = 0u32;
    let mut tag_count = 0u32;

    for segment in &selector.segments {
        for part in &segment.parts {
            match part {
                SelectorPart::Id(_) => id_count += 1,
                SelectorPart::Class(_) => class_count += 1,
                SelectorPart::Attribute { .. } => class_count += 1,
                SelectorPart::Tag(_) => tag_count += 1,
                SelectorPart::PseudoClass(pc) => match pc {
                    // :where() 不计特异性
                    PseudoClass::Where(_) => {}
                    // :has() 不计特异性
                    PseudoClass::Has(_) => {}
                    // :is() 取内部最大值
                    PseudoClass::Is(selectors) => {
                        let max_spec = selectors.iter()
                            .map(|s| compute_specificity_parsed(s))
                            .max()
                            .unwrap_or((0, 0, 0));
                        id_count += max_spec.0;
                        class_count += max_spec.1;
                        tag_count += max_spec.2;
                    }
                    // :not() 取内部最大值
                    PseudoClass::Not(inner) => {
                        match inner.as_ref() {
                            SelectorPart::Id(_) => id_count += 1,
                            SelectorPart::Class(_) => class_count += 1,
                            SelectorPart::Attribute { .. } => class_count += 1,
                            SelectorPart::Tag(_) => tag_count += 1,
                            SelectorPart::PseudoClass(_) => class_count += 1,
                        }
                    }
                    _ => class_count += 1,
                },
            }
        }
    }
    (id_count, class_count, tag_count)
}

// ============================================================
//  Phase 3: 复杂选择器解析（组合器 + 伪元素）
// ============================================================

/// Phase 3: 解析逗号分隔的选择器列表 (用于 :is(), :where())
pub fn parse_selector_list(list_str: &str) -> Vec<ParsedSelector> {
    let mut selectors = Vec::new();
    let mut depth = 0;
    let mut start = 0;

    for (i, ch) in list_str.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                let sel = list_str[start..i].trim().to_string();
                if !sel.is_empty() {
                    let (parsed, _) = parse_selector(&sel);
                    selectors.push(parsed);
                }
                start = i + 1;
            }
            _ => {}
        }
    }

    let last = list_str[start..].trim().to_string();
    if !last.is_empty() {
        let (parsed, _) = parse_selector(&last);
        selectors.push(parsed);
    }

    selectors
}

/// Phase 3: 解析完整选择器（支持组合器 + 伪元素）
///
/// 返回 (ParsedSelector, 可选的伪元素)
/// 例: "div.container > span.active::before" →
///   segments: [
///     { parts: [Tag("div"), Class("container")], combinator: Descendant },
///     { parts: [Tag("span"), Class("active")], combinator: Child }
///   ],
///   pseudo: Some(Before)
pub fn parse_selector(selector: &str) -> (ParsedSelector, Option<PseudoElement>) {
    let selector = selector.trim();
    if selector.is_empty() {
        return (ParsedSelector { segments: Vec::new() }, None);
    }

    // 1. 检测伪元素后缀
    let (base, pseudo) = if let Some(stripped) = selector.strip_suffix("::before") {
        (stripped.trim(), Some(PseudoElement::Before))
    } else if let Some(stripped) = selector.strip_suffix("::after") {
        (stripped.trim(), Some(PseudoElement::After))
    } else {
        (selector, None)
    };

    // 2. 按组合器分割为段
    let segments = parse_segments(base);
    (ParsedSelector { segments }, pseudo)
}

/// 解析带组合器的段
fn parse_segments(selector: &str) -> Vec<SelectorSegment> {
    if selector.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let chars: Vec<char> = selector.chars().collect();
    let len = chars.len();

    let mut pos = 0;
    let mut current_start = 0;
    let mut combinator = Combinator::Descendant;
    let mut depth = 0;
    let mut in_attr = false;

    while pos < len {
        match chars[pos] {
            '(' => { depth += 1; pos += 1; }
            ')' => { depth -= 1; pos += 1; }
            '[' => { in_attr = true; pos += 1; }
            ']' => { in_attr = false; pos += 1; }
            '>' if depth == 0 && !in_attr => {
                let part = selector[current_start..pos].trim().to_string();
                if !part.is_empty() {
                    segments.push(SelectorSegment {
                        parts: parse_selector_parts(&part),
                        combinator: combinator.clone(),
                    });
                }
                combinator = Combinator::Child;
                pos += 1;
                current_start = pos;
                while current_start < len && chars[current_start].is_whitespace() {
                    current_start += 1;
                }
                pos = current_start;
            }
            '+' if depth == 0 && !in_attr => {
                let part = selector[current_start..pos].trim().to_string();
                if !part.is_empty() {
                    segments.push(SelectorSegment {
                        parts: parse_selector_parts(&part),
                        combinator: combinator.clone(),
                    });
                }
                combinator = Combinator::AdjacentSibling;
                pos += 1;
                current_start = pos;
                while current_start < len && chars[current_start].is_whitespace() {
                    current_start += 1;
                }
                pos = current_start;
            }
            '~' if depth == 0 && !in_attr => {
                let part = selector[current_start..pos].trim().to_string();
                if !part.is_empty() {
                    segments.push(SelectorSegment {
                        parts: parse_selector_parts(&part),
                        combinator: combinator.clone(),
                    });
                }
                combinator = Combinator::GeneralSibling;
                pos += 1;
                current_start = pos;
                while current_start < len && chars[current_start].is_whitespace() {
                    current_start += 1;
                }
                pos = current_start;
            }
            c if c.is_whitespace() && depth == 0 && !in_attr => {
                let mut next = pos + 1;
                while next < len && chars[next].is_whitespace() {
                    next += 1;
                }
                if next < len {
                    match chars[next] {
                        '>' | '+' | '~' => {
                            pos = next;
                            continue;
                        }
                        _ => {
                            let part = selector[current_start..pos].trim().to_string();
                            if !part.is_empty() {
                                segments.push(SelectorSegment {
                                    parts: parse_selector_parts(&part),
                                    combinator: combinator.clone(),
                                });
                            }
                            combinator = Combinator::Descendant;
                            current_start = next;
                            pos = next;
                        }
                    }
                } else {
                    pos = next;
                }
            }
            _ => {
                pos += 1;
            }
        }
    }

    // 最后一段
    let last_part = selector[current_start..].trim().to_string();
    if !last_part.is_empty() {
        segments.push(SelectorSegment {
            parts: parse_selector_parts(&last_part),
            combinator,
        });
    }

    segments
}

/// Phase 3: 为伪元素生成虚拟 DOM 节点
pub fn create_pseudo_element_node(
    element: &Rc<RefCell<dom::Node>>,
    pseudo: &PseudoElement,
    content: &str,
) -> Rc<RefCell<dom::Node>> {
    let _ = element;
    let tag = match pseudo {
        PseudoElement::Before => "::before",
        PseudoElement::After => "::after",
    };
    let node = dom::Node::new(dom::NodeType::Element(dom::ElementData::new(tag)));
    node.borrow_mut().set_text_content(content);
    node
}

// ============================================================
//  Phase 3: 复杂选择器匹配
// ============================================================

/// Phase 3: 检查节点是否匹配复杂选择器（含组合器）
pub fn matches_complex_selector(
    node: &Rc<RefCell<dom::Node>>,
    selector: &ParsedSelector,
) -> bool {
    let segments = &selector.segments;
    if segments.is_empty() {
        return false;
    }

    let last_segment = &segments[segments.len() - 1];
    if !matches_segment_parts(node, &last_segment.parts) {
        return false;
    }

    if segments.len() == 1 {
        return true;
    }

    let mut current = node.clone();
    for i in (0..segments.len() - 1).rev() {
        let segment = &segments[i + 1];
        let prev_segment = &segments[i];

        match segment.combinator {
            Combinator::Descendant => {
                match find_ancestor_matching(&current, &prev_segment.parts) {
                    Some(ancestor) => current = ancestor,
                    None => return false,
                }
            }
            Combinator::Child => {
                match find_parent_matching(&current, &prev_segment.parts) {
                    Some(parent) => current = parent,
                    None => return false,
                }
            }
            Combinator::AdjacentSibling => {
                match find_prev_sibling_matching(&current, &prev_segment.parts) {
                    Some(sibling) => current = sibling,
                    None => return false,
                }
            }
            Combinator::GeneralSibling => {
                match find_prev_sibling_general(&current, &prev_segment.parts) {
                    Some(sibling) => current = sibling,
                    None => return false,
                }
            }
        }
    }

    true
}

/// 检查节点是否匹配段中的所有 parts
fn matches_segment_parts(node: &Rc<RefCell<dom::Node>>, parts: &[SelectorPart]) -> bool {
    if parts.is_empty() {
        return true;
    }
    let n = node.borrow();
    for part in parts {
        let matched = match &n.node_type {
            NodeType::Element(elem) => {
                if matches_pseudo_class_node(node, part) {
                    true
                } else {
                    element_matches_selector_part(elem, part)
                }
            }
            _ => false,
        };
        if !matched {
            return false;
        }
    }
    true
}

/// 查找匹配的祖先节点（后代组合器）
fn find_ancestor_matching(
    node: &Rc<RefCell<dom::Node>>,
    parts: &[SelectorPart],
) -> Option<Rc<RefCell<dom::Node>>> {
    let mut current = node.borrow().parent_node()?;
    loop {
        if matches_segment_parts(&current, parts) {
            return Some(current.clone());
        }
        let parent = current.borrow().parent_node()?;
        current = parent;
    }
}

/// 查找匹配的父节点（子代组合器）
fn find_parent_matching(
    node: &Rc<RefCell<dom::Node>>,
    parts: &[SelectorPart],
) -> Option<Rc<RefCell<dom::Node>>> {
    let parent = node.borrow().parent_node()?;
    if matches_segment_parts(&parent, parts) {
        Some(parent.clone())
    } else {
        None
    }
}

/// 查找匹配的前一个兄弟（相邻兄弟组合器）
fn find_prev_sibling_matching(
    node: &Rc<RefCell<dom::Node>>,
    parts: &[SelectorPart],
) -> Option<Rc<RefCell<dom::Node>>> {
    let sibling = node.borrow().previous_sibling()?;
    if matches_segment_parts(&sibling, parts) {
        Some(sibling)
    } else {
        None
    }
}

/// 查找任意匹配的前兄弟（通用兄弟组合器）
fn find_prev_sibling_general(
    node: &Rc<RefCell<dom::Node>>,
    parts: &[SelectorPart],
) -> Option<Rc<RefCell<dom::Node>>> {
    let mut current = node.borrow().previous_sibling()?;
    loop {
        if matches_segment_parts(&current, parts) {
            return Some(current.clone());
        }
        let next = current.borrow().previous_sibling()?;
        current = next;
    }
}

// ============================================================
//  SelectorEngine (Phase 1)
// ============================================================

/// 选择器引擎 —— Phase 1 基础实现，Phase 3 扩展复杂选择器
///
/// Phase 1+ 支持完整选择器 Level 3
/// Phase 2+ 完整伪类支持
/// Phase 3: 组合器 + 属性选择器 + 伪元素
pub struct SelectorEngine;

impl SelectorEngine {
    /// 创建选择器引擎
    pub fn new() -> Self {
        Self
    }

    /// 匹配单个元素是否满足选择器
    pub fn matches(
        &self,
        element: &dom::ElementData,
        selector_str: &str,
    ) -> bool {
        element_matches_selector(element, selector_str)
    }

    /// Phase 3: 匹配复杂选择器（含组合器）
    /// 例: "div.container > span.active"
    pub fn matches_complex(
        &self,
        node: &Rc<RefCell<dom::Node>>,
        selector: &ParsedSelector,
    ) -> bool {
        matches_complex_selector(node, selector)
    }

    /// 在子树中查找第一个匹配元素（深度优先）
    pub fn query_selector(
        &self,
        root: &Rc<RefCell<dom::Node>>,
        selector_str: &str,
    ) -> Option<Rc<RefCell<dom::Node>>> {
        let mut result = None;
        self.search_first(root, selector_str, &mut result);
        result
    }

    fn search_first(
        &self,
        node: &Rc<RefCell<dom::Node>>,
        selector_str: &str,
        result: &mut Option<Rc<RefCell<dom::Node>>>,
    ) {
        if result.is_some() {
            return;
        }
        let children = {
            let n = node.borrow();
            if let dom::NodeType::Element(elem) = &n.node_type {
                if element_matches_selector(elem, selector_str) {
                    *result = Some(node.clone());
                    return;
                }
            }
            n.child_nodes()
        };
        for child in &children {
            self.search_first(child, selector_str, result);
            if result.is_some() {
                return;
            }
        }
    }

    /// 在子树中查找所有匹配元素
    pub fn query_selector_all(
        &self,
        root: &Rc<RefCell<dom::Node>>,
        selector_str: &str,
    ) -> Vec<Rc<RefCell<dom::Node>>> {
        let mut results = Vec::new();
        self.search_all(root, selector_str, &mut results);
        results
    }

    fn search_all(
        &self,
        node: &Rc<RefCell<dom::Node>>,
        selector_str: &str,
        results: &mut Vec<Rc<RefCell<dom::Node>>>,
    ) {
        let children = {
            let n = node.borrow();
            if let dom::NodeType::Element(elem) = &n.node_type {
                if element_matches_selector(elem, selector_str) {
                    results.push(node.clone());
                }
            }
            n.child_nodes()
        };
        for child in &children {
            self.search_all(child, selector_str, results);
        }
    }
}

// Phase 2+: 伪类支持 (:hover, :nth-child)

#[cfg(test)]
#[path = "selector.test.rs"]
mod tests;
