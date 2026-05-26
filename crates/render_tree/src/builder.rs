//! DisplayList 构建器
//!
//! 负责遍历布局树并生成 DisplayList 绘制命令。
//!
//! 转换过程：
//! 1. 遍历 LayoutTree，按 depth-first 顺序处理每个节点
//! 2. 从节点的 ComputedStyle 中提取背景色、边框、文本等属性
//! 3. 生成对应的 PaintCommand 并插入 DisplayList

use std::sync::atomic::{AtomicU64, Ordering};

use layout::layout_box::{BoxType, LayoutBox};
use dom::Color;

use crate::command::{BorderStyle, DisplayList, PaintCommand, TextDecoration};

/// 全局 DisplayList 版本号，每次 build() 时单调递增
static DL_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Paint 命令构建器 —— 将布局树转换为绘制命令
pub struct DisplayListBuilder {
    /// 输出列表
    display_list: DisplayList,
    /// P0-5: 当前视口（用于裁剪不可见节点）
    viewport: dom::Rect<f32>,
}

impl DisplayListBuilder {
    /// 创建构建器
    pub fn new() -> Self {
        Self {
            display_list: DisplayList::new(),
            viewport: dom::Rect::new(0.0, 0.0, f32::MAX, f32::MAX),
        }
    }

    /// 设置视口范围（P0-5: 视口裁剪）
    pub fn with_viewport(mut self, viewport: dom::Rect<f32>) -> Self {
        self.viewport = viewport;
        self
    }

    /// 主入口：从布局树构建 DisplayList
    pub fn build(&mut self, layout_root: &LayoutBox) -> DisplayList {
        self.display_list.clear();
        self.process_node(layout_root);
        self.display_list.sort_by_z_order();

        // P1-9: 分配唯一版本号，渲染器据此检测 DL 变更
        let gen = DL_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        self.display_list.set_generation(gen);

        std::mem::take(&mut self.display_list)
    }

    /// 处理单个布局节点
    fn process_node(&mut self, node: &LayoutBox) {
        // P0-5: 视口裁剪 — 完全不可见的节点跳过绘制
        if !is_visible_in_viewport(&node.rect, &self.viewport) {
            return;
        }

        // 0. 盒阴影 → BoxShadow（渲染在背景之下）
        if let Some(shadow) = Self::extract_box_shadow(node) {
            self.display_list.push(shadow);
        }

        // 1. 背景色 → FillRect
        if let Some(color) = Self::extract_bg_color(node) {
            self.display_list.push(PaintCommand::FillRect {
                rect: node.rect,
                color,
                radius: node.border_radius.top_left,
            });
        }

        // 2. 边框 → Border
        if let Some(border) = Self::extract_border(node) {
            self.display_list.push(border);
        }
        for side in &["border-top", "border-right", "border-bottom", "border-left"] {
            if let Some(border) = Self::extract_single_border(node, side) {
                self.display_list.push(border);
            }
        }

        // 3. 文本 → Text
        if let Some(text_cmd) = Self::extract_text(node) {
            self.display_list.push(text_cmd);
        }

        // 4. 递归处理子节点
        for child in &node.children {
            self.process_node(child);
        }
    }

    /// 从 computed style 中提取背景色
    fn extract_bg_color(node: &LayoutBox) -> Option<Color> {
        let style = node.computed_style.as_ref()?;

        // 检查 background-color
        if let Some(bg) = style.get("background-color") {
            let bg_str = format!("{:?}", bg);
            if bg_str != "transparent" && !bg_str.contains("initial") {
                return parse_css_value_color(bg);
            }
        }
        // 检查简写 background
        if let Some(bg) = style.get("background") {
            let bg_str = format!("{:?}", bg);
            if bg_str != "none" && bg_str != "transparent" {
                return parse_css_value_color(bg);
            }
        }
        None
    }

    /// 从 computed style 中提取边框
    fn extract_border(node: &LayoutBox) -> Option<PaintCommand> {
        // 文本节点不绘制边框（继承的 border 属于父元素）
        if node.box_type == BoxType::Text {
            return None;
        }
        let style = node.computed_style.as_ref()?;
        let border_val = style.get("border")?;

        // 提取原始关键字字符串（非 Debug 格式）
        let raw: &str = match border_val {
            style::values::CSSValue::Keyword(s) => s,
            _ => return None,
        };

        if raw == "none" || raw == "initial" {
            return None;
        }

        let tokens: Vec<&str> = raw.split_whitespace().collect();

        let mut width = 1.0f32;
        let mut color = Color::rgb(0, 0, 0);

        for token in &tokens {
            if let Some(px) = token.strip_suffix("px") {
                if let Ok(w) = px.parse::<f32>() {
                    width = w;
                }
            }
            let parsed = style::values::parse_color(token);
            if parsed != Color::BLACK || *token == "black" {
                color = parsed;
            }
        }

        Some(PaintCommand::Border {
            rect: node.rect,
            widths: [width; 4],
            colors: [color; 4],
            radius: node.border_radius.top_left,
            style: BorderStyle::Solid,
        })
    }

    /// 从 computed style 中提取单边边框（border-top/right/bottom/left）
    fn extract_single_border(node: &LayoutBox, property: &str) -> Option<PaintCommand> {
        if node.box_type == BoxType::Text {
            return None;
        }
        let style = node.computed_style.as_ref()?;
        let border_val = style.get(property)?;
        let raw: &str = match border_val {
            style::values::CSSValue::Keyword(s) => s,
            _ => return None,
        };
        if raw == "none" || raw == "initial" {
            return None;
        }
        let tokens: Vec<&str> = raw.split_whitespace().collect();
        let mut width = 1.0f32;
        let mut color = Color::rgb(0, 0, 0);
        for token in &tokens {
            if let Some(px) = token.strip_suffix("px") {
                if let Ok(w) = px.parse::<f32>() {
                    width = w;
                }
            }
            let parsed = style::values::parse_color(token);
            if parsed != Color::BLACK || *token == "black" {
                color = parsed;
            }
        }

        let side = match property {
            "border-top" => 0,
            "border-right" => 1,
            "border-bottom" => 2,
            "border-left" => 3,
            _ => return None,
        };
        let mut widths = [0.0f32; 4];
        let mut colors = [Color::TRANSPARENT; 4];
        widths[side] = width;
        colors[side] = color;

        Some(PaintCommand::Border {
            rect: node.rect,
            widths,
            colors,
            radius: node.border_radius.top_left,
            style: BorderStyle::Solid,
        })
    }

    /// 从 computed style 中提取盒阴影
    fn extract_box_shadow(node: &LayoutBox) -> Option<PaintCommand> {
        let style = node.computed_style.as_ref()?;
        let shadow = style.get("box-shadow")?;
        match shadow {
            style::values::CSSValue::BoxShadow(box_shadow) => {
                Some(PaintCommand::BoxShadow {
                    rect: node.rect,
                    offset_x: box_shadow.offset_x,
                    offset_y: box_shadow.offset_y,
                    blur_radius: box_shadow.blur_radius,
                    spread_radius: box_shadow.spread_radius,
                    color: box_shadow.color,
                    inset: box_shadow.inset,
                    radius: node.border_radius.top_left,
                })
            }
            _ => None,
        }
    }

    /// 从布局节点提取文本内容
    fn extract_text(node: &LayoutBox) -> Option<PaintCommand> {
        if node.box_type != BoxType::Text {
            return None;
        }

        let dom_node = node.node.as_ref()?;
        let node_borrow = dom_node.borrow();
        let text = node_borrow.text_content();

        // 文本为空时，检查父节点是否有 placeholder 属性
        let (display_text, is_placeholder) = if text.is_empty() {
            let placeholder = node_borrow.parent_node().and_then(|parent| {
                let p = parent.borrow();
                if let dom::NodeType::Element(elem) = &p.node_type {
                    elem.get_attribute("placeholder")
                } else {
                    None
                }
            });
            match placeholder {
                Some(p) => (p, true),
                None => return None,
            }
        } else {
            (text, false)
        };

        let font_size = node
            .computed_style
            .as_ref()
            .and_then(|s| s.get("font-size"))
            .map(|v| match v {
                style::values::CSSValue::Length(px, _) => *px,
                _ => 16.0,
            })
            .unwrap_or(16.0);

        let font_family = node
            .computed_style
            .as_ref()
            .and_then(|s| s.get("font-family"))
            .map(|v| match v {
                style::values::CSSValue::Keyword(k) => k.to_string(),
                _ => "sans-serif".to_string(),
            })
            .unwrap_or_else(|| "sans-serif".to_string());

        let color = if is_placeholder {
            Color::rgba(153, 153, 153, 200) // 浅灰色 placeholder
        } else {
            node.computed_style
                .as_ref()
                .and_then(|s| s.get("color"))
                .and_then(parse_css_value_color)
                .unwrap_or(Color::rgb(0, 0, 0))
        };

        let font_weight = node
            .computed_style
            .as_ref()
            .and_then(|s| s.get("font-weight"))
            .map(|v| match v {
                style::values::CSSValue::Keyword(k) => match k.as_ref() {
                    "bold" => 700,
                    "bolder" => 900,
                    "lighter" => 100,
                    "normal" => 400,
                    _ => 400,
                },
                style::values::CSSValue::Number(n) => *n as u16,
                _ => 400,
            })
            .unwrap_or(400);

        // y = text area top + half-leading (centers em square in line box)
        // text_renderer 会加上 font ascent 得到正确基线
        let half_leading = (node.rect.height - font_size).max(0.0) / 2.0;

        Some(PaintCommand::Text {
            text: display_text,
            font_size,
            font_family,
            font_weight,
            x: node.rect.x,
            y: node.rect.y + half_leading,
            color,
            decoration: TextDecoration::None,
        })
    }
}

/// 从 CSSValue 引用中尝试提取 Color
fn parse_css_value_color(value: &style::values::CSSValue) -> Option<Color> {
    match value {
        style::values::CSSValue::Color(c) => Some(*c),
        style::values::CSSValue::Keyword(s) => {
            // 尝试作为颜色名称解析
            let color = style::values::parse_color(s);
            if color != Color::BLACK || s == "black" {
                Some(color)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// P0-5: 检查矩形容器是否在视口内可见（含扩展边距以覆盖阴影/边框）
fn is_visible_in_viewport(rect: &dom::Rect<f32>, viewport: &dom::Rect<f32>) -> bool {
    let margin = 50.0; // 扩展边距覆盖阴影/blur
    rect.x + rect.width + margin > viewport.x
        && rect.x - margin < viewport.x + viewport.width
        && rect.y + rect.height + margin > viewport.y
        && rect.y - margin < viewport.y + viewport.height
}

// Phase 1+: BatchOptimizer 合批优化

#[cfg(test)]
#[path = "builder.test.rs"]
mod tests;
