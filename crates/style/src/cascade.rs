//! CSS 级联计算
//!
//! 负责将多个来源的 CSS 声明按优先级叠加以计算出最终样式。
//!
//! 级联排序规则（从低到高）：
//! 1. 用户代理样式（浏览器默认）
//! 2. 用户样式（Phase 2+）
//! 3. 作者样式（开发者定义的 CSS）
//! 4. 内联样式（style 属性）
//! 5. !important 规则
//!
//! Phase 3: 支持 match_selectors_full 以正确匹配组合器选择器

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use dom::ElementData;

use crate::ComputedStyle;
use crate::selector::MatchedDeclaration;
use crate::stylesheet::{Declaration, StyleSheet};
use crate::values::{parse_css_value, CSSValue};
use crate::properties::parse_phase3_property;

// ============================================================
//  P0-4: 样式缓存
// ============================================================

/// 样式缓存版本号 —— 样式表变更时递增以全局失效缓存
static mut STYLE_VERSION: u64 = 1;

/// 通知样式缓存：样式表已变更（调用后在下次级联计算时自动重建）
pub fn increment_style_version() {
    unsafe { STYLE_VERSION = STYLE_VERSION.wrapping_add(1); }
}

pub fn current_style_version() -> u64 {
    unsafe { STYLE_VERSION }
}

/// 样式缓存键：元素标识 + 样式版本
#[derive(Hash, PartialEq, Eq)]
struct CacheKey {
    tag_hash: u64,
    classes_hash: u64,
    id_hash: u64,
    version: u64,
}

/// 计算后样式的缓存
///
/// 基于元素标识（tag + classes + id）+ 样式版本号进行缓存。
/// 样式表变更或 dirty 标记触发全局版本号递增，自然淘汰所有过期条目。
pub struct StyleCache {
    cache: std::collections::HashMap<CacheKey, ComputedStyle>,
    hits: u64,
    misses: u64,
}

impl StyleCache {
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    fn make_key(element: &ElementData) -> CacheKey {
        let mut h = DefaultHasher::new();
        element.tag_name().hash(&mut h);
        let tag_hash = h.finish();

        let mut h = DefaultHasher::new();
        for class in element.class_list() {
            class.hash(&mut h);
        }
        let classes_hash = h.finish();

        let mut h = DefaultHasher::new();
        element.id().hash(&mut h);
        let id_hash = h.finish();

        CacheKey {
            tag_hash,
            classes_hash,
            id_hash,
            version: current_style_version(),
        }
    }

    pub fn get(&mut self, element: &ElementData) -> Option<ComputedStyle> {
        let key = Self::make_key(element);
        if let Some(style) = self.cache.get(&key) {
            self.hits += 1;
            Some(style.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn put(&mut self, element: &ElementData, style: &ComputedStyle) {
        let key = Self::make_key(element);
        self.cache.insert(key, style.clone());
    }

    /// 返回命中率
    #[allow(dead_code)]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            1.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// 计算元素的最终样式
///
/// 参数：
///   - element: 目标元素
///   - parent_style: 父元素计算后样式（用于继承属性）
///   - stylesheets: 所有样式表
///   - inline_style: 内联样式声明（style 属性值）
///   - cache: P0-4 样式缓存（可选，传递 &mut StyleCache 启用缓存）
///
/// 返回：属性名 → CSSValue 的映射
pub fn compute_element_style(
    element: &ElementData,
    _parent_style: Option<&ComputedStyle>,
    stylesheets: &[StyleSheet],
    inline_style: &[Declaration],
) -> ComputedStyle {
    compute_element_style_cached(element, _parent_style, stylesheets, inline_style, None)
}

/// P0-4: 带缓存的样式计算
pub fn compute_element_style_cached(
    element: &ElementData,
    _parent_style: Option<&ComputedStyle>,
    stylesheets: &[StyleSheet],
    inline_style: &[Declaration],
    mut cache: Option<&mut StyleCache>,
) -> ComputedStyle {
    // P0-4: 检查缓存
    if let Some(ref mut c) = cache {
        if let Some(cached) = c.get(element) {
            return cached;
        }
    }

    // 1. 收集所有匹配的声明
    let mut matched = crate::selector::match_selectors(element, stylesheets);

    // 2. 将内联声明加入（最高优先级，除 !important 外）
    // 使用 u32::MAX 特异性确保内联样式覆盖所有非 !important 的作者样式
    for decl in inline_style {
        matched.push(MatchedDeclaration {
            specificity: (u32::MAX, u32::MAX, u32::MAX),
            declaration: decl.clone(),
        });
    }

    // 3. 按特异性排序（低 → 高），内联样式在最后
    matched.sort_by(|a, b| {
        if a.declaration.important && !b.declaration.important {
            return std::cmp::Ordering::Greater;
        }
        if !a.declaration.important && b.declaration.important {
            return std::cmp::Ordering::Less;
        }
        a.specificity.cmp(&b.specificity)
    });

    // 4. 依次应用到 ComputedStyle（后应用的覆盖先应用的）
    let mut style = ComputedStyle::new();
    for m in &matched {
        let value = parse_css_value(&m.declaration.property, &m.declaration.value);
        style.set(&m.declaration.property, value);
    }

    // 5. 继承父元素样式（仅继承属性）
    if let Some(parent) = _parent_style {
        apply_inherited(parent, &mut style);
    }

    // 6. 应用用户代理默认样式（最低优先级）
    apply_user_agent_defaults(element, &mut style);

    // P0-4: 写入缓存
    if let Some(ref mut c) = cache {
        c.put(element, &style);
    }

    style
}

/// Phase 3: 带 Node 上下文计算样式（支持组合器选择器）
///
/// 与 compute_element_style 类似，但使用 match_selectors_full
/// 以便正确匹配使用了组合器的选择器（如 "div > span"）。
pub fn compute_element_style_with_node(
    node: &Rc<RefCell<dom::Node>>,
    parent_style: Option<&ComputedStyle>,
    stylesheets: &[StyleSheet],
    inline_style: &[Declaration],
) -> ComputedStyle {
    compute_element_style_with_node_cached(node, parent_style, stylesheets, inline_style, None)
}

/// P0-4: 带缓存的 Node 感知样式计算
pub fn compute_element_style_with_node_cached(
    node: &Rc<RefCell<dom::Node>>,
    parent_style: Option<&ComputedStyle>,
    stylesheets: &[StyleSheet],
    inline_style: &[Declaration],
    mut cache: Option<&mut StyleCache>,
) -> ComputedStyle {
    // P0-4: 检查缓存
    if let Some(ref mut c) = cache {
        if let dom::NodeType::Element(elem_data) = &node.borrow().node_type {
            if let Some(cached) = c.get(elem_data) {
                return cached;
            }
        }
    }

    // 1. 收集所有匹配的声明（使用 Node 感知的匹配）
    let mut matched = crate::selector::match_selectors_full(node, stylesheets);

    // 2. 内联声明加入
    for decl in inline_style {
        matched.push(MatchedDeclaration {
            specificity: (u32::MAX, u32::MAX, u32::MAX),
            declaration: decl.clone(),
        });
    }

    // 3. 按特异性排序
    matched.sort_by(|a, b| {
        if a.declaration.important && !b.declaration.important {
            return std::cmp::Ordering::Greater;
        }
        if !a.declaration.important && b.declaration.important {
            return std::cmp::Ordering::Less;
        }
        a.specificity.cmp(&b.specificity)
    });

    // 4. 应用声明到 ComputedStyle
    let mut style = ComputedStyle::new();
    for m in &matched {
        // Phase 3: 先尝试使用新属性解析器
        let value = match parse_phase3_property(&m.declaration.property, &m.declaration.value) {
            Some(v) => v,
            None => parse_css_value(&m.declaration.property, &m.declaration.value),
        };
        style.set(&m.declaration.property, value);
    }

    // 5. 继承
    if let Some(parent) = parent_style {
        apply_inherited(parent, &mut style);
    }

    // 6. 应用用户代理默认样式
    if let dom::NodeType::Element(elem_data) = &node.borrow().node_type {
        apply_user_agent_defaults(elem_data, &mut style);
    }

    // P0-4: 写入缓存
    if let Some(ref mut c) = cache {
        if let dom::NodeType::Element(elem_data) = &node.borrow().node_type {
            c.put(elem_data, &style);
        }
    }

    style
}

/// 应用继承属性：从父元素继承 CSS 属性
///
/// CSS 继承属性包括：color, font-*, line-height, text-align, white-space 等
/// Phase 0: 仅复制父元素中存在的且子元素未设置的属性
fn apply_inherited(parent: &ComputedStyle, child: &mut ComputedStyle) {
    // 从父元素继承 color
    if !child.contains_key("color") {
        if let Some(val) = parent.get("color") {
            child.set("color", val.clone());
        }
    }
    // 从父元素继承 font-size
    if !child.contains_key("font-size") {
        if let Some(val) = parent.get("font-size") {
            child.set("font-size", val.clone());
        }
    }
    // 从父元素继承 font-family
    if !child.contains_key("font-family") {
        if let Some(val) = parent.get("font-family") {
            child.set("font-family", val.clone());
        }
    }
    // 从父元素继承 text-align
    if !child.contains_key("text-align") {
        if let Some(val) = parent.get("text-align") {
            child.set("text-align", val.clone());
        }
    }
    // Phase 1+: 通过 properties.toml 的 inherited 标记自动判断
}

/// 用户代理默认样式：为 HTML 元素提供浏览器默认值
///
/// 优先级最低，仅当元素未通过 CSS/内联设置对应属性时生效。
fn apply_user_agent_defaults(element: &ElementData, style: &mut ComputedStyle) {
    // 默认 content-box（与 Chrome 一致，CSS 规范初始值）
    if !style.contains_key("box-sizing") {
        style.set("box-sizing", CSSValue::Keyword("content-box".into()));
    }

    let tag = element.tag_name();
    match tag.as_ref() {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            if !style.contains_key("font-weight") {
                style.set("font-weight", CSSValue::Keyword("bold".into()));
            }
        }
        "strong" | "b" | "th" => {
            if !style.contains_key("font-weight") {
                style.set("font-weight", CSSValue::Keyword("bold".into()));
            }
        }
        "em" | "i" | "cite" | "dfn" => {
            if !style.contains_key("font-style") {
                style.set("font-style", CSSValue::Keyword("italic".into()));
            }
        }
        "a" | "link" => {
            if !style.contains_key("color") {
                style.set("color", CSSValue::Color(dom::Color::rgb(0, 0, 238)));
            }
            if !style.contains_key("text-decoration") {
                style.set("text-decoration", CSSValue::Keyword("underline".into()));
            }
        }
        "code" | "pre" | "kbd" | "samp" => {
            if !style.contains_key("font-family") {
                style.set("font-family", CSSValue::Keyword("monospace".into()));
            }
        }
        _ => {}
    }
}

// Phase 1+: 级联缓存、initial/inherit/unset 处理

#[cfg(test)]
#[path = "cascade.test.rs"]
mod tests;
