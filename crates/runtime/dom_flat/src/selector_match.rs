//! selector_match — CSS 选择器类型定义 + 解析（不含 DomRegistry 匹配逻辑）。

use super::specificity::Specificity;

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
