//! Flexbox 布局 —— 通过 taffy crate 实现
//!
//! 支持的 flex 属性：
//!   flex-direction, flex-wrap, justify-content,
//!   align-items, align-content, gap,
//!   flex-grow, flex-shrink, flex-basis

use taffy::{prelude::*, TaffyTree};

use crate::layout_box::LayoutBox;
use dom::Size;
use style::ComputedStyle;
use style::values::CSSValue;

/// Flexbox 布局引擎
pub struct FlexLayout;

impl FlexLayout {
    /// 对 Flex 容器执行布局
    ///
    /// taffy: taffy 布局引擎实例
    /// container: 必须是 FlexContainer 类型
    /// viewport: 当前视口尺寸
    /// constrain_width: 父容器是否为 Block（true=约束宽度以支持 space-between 等）
    pub fn layout(
        &mut self,
        taffy: &mut Option<TaffyTree>,
        container: &mut LayoutBox,
        viewport: Size<f32>,
        constrain_width: bool,
    ) {
        if container.children.is_empty() {
            return;
        }

        let Some(taffy_tree) = taffy.as_mut() else {
            return;
        };

        // 1. 为容器及其子节点创建 taffy node
        let mut node_map: Vec<taffy::NodeId> = Vec::new();

        // 创建容器节点：容器尺寸由内容决定，不信任构建阶段估计值
        let container_style = Self::convert_container_style(container, viewport, constrain_width);
        let container_id = taffy_tree.new_leaf(container_style).unwrap();
        node_map.push(container_id);

        // 创建子节点：此时子级已完成自底向上布局，rect 为正确内容尺寸
        for child in &container.children {
            let child_style = Self::convert_child_style(child, viewport);
            let child_id = taffy_tree.new_leaf(child_style).unwrap();
            taffy_tree.add_child(container_id, child_id).unwrap();
            node_map.push(child_id);
        }

        // 2. 执行布局计算
        //   - constrain_width: 父级为 Block，宽度由 BlockLayout 设置，可信赖
        //   - !constrain_width: 父级为 Flex/Grid，宽度为构建阶段估计值(Block规则)，不可信
        let available = taffy::Size {
            width: if constrain_width {
                taffy::AvailableSpace::Definite(container.rect.width.max(0.0))
            } else {
                taffy::AvailableSpace::MaxContent
            },
            height: taffy::AvailableSpace::MaxContent,
        };
        taffy_tree
            .compute_layout(container_id, available)
            .unwrap();

        // 3. 将 taffy 结果写回 LayoutBox
        // 保存容器原始位置和宽度（由父级 block layout 确定，taffy 不可覆盖）
        let container_x = container.rect.x;
        let container_y = container.rect.y;
        let container_w = container.rect.width;

        // 容器自身的布局结果 — 计算高度，宽度保持 block layout 设置值
        if let Ok(layout) = taffy_tree.layout(container_id) {
            container.rect.width = container_w.max(layout.size.width);
            container.rect.height = layout.size.height;
        }

        // 子节点的布局结果（taffy 坐标相对于容器内容区原点）
        for (i, child) in container.children.iter_mut().enumerate() {
            if i < node_map.len() - 1 {
                let child_id = node_map[i + 1];
                if let Ok(layout) = taffy_tree.layout(child_id) {
                    child.rect.x = container_x + layout.location.x;
                    child.rect.y = container_y + layout.location.y;
                    child.rect.width = layout.size.width;
                    child.rect.height = layout.size.height;
                }
            }
        }

        // 清理 taffy tree（为下一次布局做准备）
        taffy_tree.clear();
    }

    /// 容器样式：宽由父级决定（rect.width 可信），高由子级内容决定（rect.height 不可信）
    fn convert_container_style(container: &LayoutBox, _viewport: Size<f32>, constrain_width: bool) -> taffy::Style {
        let mut style = taffy::Style::default();
        Self::apply_common_style(&mut style, container);
        Self::apply_flex_properties(&mut style, container);

        // 容器宽度：仅当父级为 Block 时 rect.width 可信赖
        // 父级为 Flex/Grid 时，rect.width 为构建阶段估计值（Block 规则不适用于 Flex）
        if constrain_width && container.rect.width > 0.0 {
            style.size.width = taffy::Dimension::Length(container.rect.width);
        }

        // 容器高度：构建阶段估计值不可信（Block 规则用 sum 而非 max）
        // 仅设 padding+border 作为 min_size，taffy 根据子级内容计算实际高度
        let min_h = container.padding.top + container.padding.bottom
            + container.border.top + container.border.bottom;
        if min_h > 0.0 {
            style.min_size.height = taffy::Dimension::Length(min_h);
        }

        // CSS 显式 width/height 覆盖
        if let Some(ref cs) = container.computed_style {
            let is_border_box = is_border_box_style(cs);
            if let Some(w) = cs.get("width") {
                if let Some(val) = resolve_length(w) {
                    // taffy 的 size 是 content-box，border-box 需要减去 padding+border
                    let content_w = if is_border_box {
                        (val - container.padding.left - container.padding.right
                         - container.border.left - container.border.right).max(0.0)
                    } else {
                        val
                    };
                    style.size.width = Dimension::Length(content_w);
                }
            }
            if let Some(h) = cs.get("height") {
                if let Some(val) = resolve_length(h) {
                    let content_h = if is_border_box {
                        (val - container.padding.top - container.padding.bottom
                         - container.border.top - container.border.bottom).max(0.0)
                    } else {
                        val
                    };
                    style.size.height = Dimension::Length(content_h);
                }
            }
        }

        style
    }

    /// 子级样式：此时子级已完成自底向上布局，rect 为正确内容尺寸
    ///
    /// 关键：taffy 的 flex 分配以 content-box 为单位，gap 也在 content-box 之间。
    /// 为避免 padding/border 从 content-box 向外溢出到 gap 区域造成重叠，
    /// 子元素不向 taffy 报告 padding/border，让 taffy 直接以 border-box 作为
    /// "content-box" 参与 flex 分配。写回时 layout.size 就是 border-box。
    fn convert_child_style(child: &LayoutBox, _viewport: Size<f32>) -> taffy::Style {
        let mut style = taffy::Style::default();
        Self::apply_common_style(&mut style, child);
        Self::apply_flex_properties(&mut style, child);

        // 清除 padding/border —— 直接以 border-box 参与 taffy flex 分配
        style.padding = taffy::Rect::zero();
        style.border = taffy::Rect::zero();

        // rect.width 已是 border-box，作为 min_size 防止塌陷
        if child.rect.width > 0.0 {
            style.min_size.width = taffy::Dimension::Length(child.rect.width);
        }
        if child.rect.height > 0.0 {
            style.min_size.height = taffy::Dimension::Length(child.rect.height);
        }

        // 显式 CSS width/height 覆盖
        if let Some(ref cs) = child.computed_style {
            let is_border_box = is_border_box_style(cs);
            if let Some(w) = cs.get("width") {
                if let Some(val) = resolve_length(w) {
                    // 子元素不向 taffy 报告 padding/border，width 直接作为 taffy content-box
                    // border-box: CSS width 即 border-box，content = width - padding - border
                    // content-box: CSS width 即 content，border-box = width + padding + border
                    let taffy_w = if is_border_box {
                        val
                    } else {
                        val + child.padding.left + child.padding.right
                            + child.border.left + child.border.right
                    };
                    style.size.width = Dimension::Length(taffy_w);
                }
            }
            if let Some(h) = cs.get("height") {
                if let Some(val) = resolve_length(h) {
                    let taffy_h = if is_border_box {
                        val
                    } else {
                        val + child.padding.top + child.padding.bottom
                            + child.border.top + child.border.bottom
                    };
                    style.size.height = Dimension::Length(taffy_h);
                }
            }
        }

        style
    }

    /// 应用盒模型（padding/border）和 min/max 约束
    fn apply_common_style(style: &mut taffy::Style, box_node: &LayoutBox) {
        style.padding.left = taffy::LengthPercentage::Length(box_node.padding.left);
        style.padding.right = taffy::LengthPercentage::Length(box_node.padding.right);
        style.padding.top = taffy::LengthPercentage::Length(box_node.padding.top);
        style.padding.bottom = taffy::LengthPercentage::Length(box_node.padding.bottom);

        style.border.left = taffy::LengthPercentage::Length(box_node.border.left);
        style.border.right = taffy::LengthPercentage::Length(box_node.border.right);
        style.border.top = taffy::LengthPercentage::Length(box_node.border.top);
        style.border.bottom = taffy::LengthPercentage::Length(box_node.border.bottom);

        if let Some(ref cs) = box_node.computed_style {
            if let Some(v) = cs.get("min-width") {
                if let Some(val) = resolve_length(v) {
                    style.min_size.width = Dimension::Length(val);
                }
            }
            if let Some(v) = cs.get("max-width") {
                if let Some(val) = resolve_length(v) {
                    style.max_size.width = Dimension::Length(val);
                }
            }
            if let Some(v) = cs.get("min-height") {
                if let Some(val) = resolve_length(v) {
                    style.min_size.height = Dimension::Length(val);
                }
            }
            if let Some(v) = cs.get("max-height") {
                if let Some(val) = resolve_length(v) {
                    style.max_size.height = Dimension::Length(val);
                }
            }
        }
    }

    /// 应用 flex 布局属性（flex-direction, justify-content, align-items, gap, flex-wrap, flex 简写等）
    fn apply_flex_properties(style: &mut taffy::Style, box_node: &LayoutBox) {
        let Some(ref cs) = box_node.computed_style else { return };

        // flex-direction
        if let Some(dir) = cs.get("flex-direction") {
            style.flex_direction = match css_keyword(dir) {
                "row" => FlexDirection::Row,
                "row-reverse" => FlexDirection::RowReverse,
                "column" => FlexDirection::Column,
                "column-reverse" => FlexDirection::ColumnReverse,
                _ => FlexDirection::Row,
            };
        }

        // justify-content
        if let Some(jc) = cs.get("justify-content") {
            style.justify_content = Some(match css_keyword(jc) {
                "flex-start" => JustifyContent::FlexStart,
                "flex-end" => JustifyContent::FlexEnd,
                "center" => JustifyContent::Center,
                "space-between" => JustifyContent::SpaceBetween,
                "space-around" => JustifyContent::SpaceAround,
                "space-evenly" => JustifyContent::SpaceEvenly,
                _ => JustifyContent::FlexStart,
            });
        }

        // align-items
        if let Some(ai) = cs.get("align-items") {
            style.align_items = Some(match css_keyword(ai) {
                "flex-start" => AlignItems::FlexStart,
                "flex-end" => AlignItems::FlexEnd,
                "center" => AlignItems::Center,
                "stretch" => AlignItems::Stretch,
                "baseline" => AlignItems::Baseline,
                _ => AlignItems::Stretch,
            });
        }

        // gap
        if let Some(g) = cs.get("gap") {
            if let Some(val) = resolve_length(g) {
                let lp = taffy::LengthPercentage::Length(val);
                style.gap = taffy::Size { width: lp, height: lp };
            }
        }

        // flex-wrap
        if let Some(wrap) = cs.get("flex-wrap") {
            style.flex_wrap = match css_keyword(wrap) {
                "wrap" => FlexWrap::Wrap,
                "wrap-reverse" => FlexWrap::WrapReverse,
                _ => FlexWrap::NoWrap,
            };
        }

        // flex 简写
        if let Some(flex_val) = cs.get("flex") {
            let num = css_number(flex_val);
            let is_auto = keyword_eq(flex_val, "auto");
            if let Some(n) = num {
                style.flex_grow = n;
                style.flex_shrink = 1.0;
                style.flex_basis = Dimension::Length(0.0);
            } else if is_auto {
                style.flex_grow = 1.0;
                style.flex_shrink = 1.0;
                style.flex_basis = Dimension::Auto;
            }
        }

        if let Some(grow) = cs.get("flex-grow") {
            if let Some(val) = css_number(grow) {
                style.flex_grow = val;
            }
        }
        if let Some(shrink) = cs.get("flex-shrink") {
            if let Some(val) = css_number(shrink) {
                style.flex_shrink = val;
            }
        }
        if let Some(basis) = cs.get("flex-basis") {
            if let Some(val) = resolve_length(basis) {
                style.flex_basis = Dimension::Length(val);
            } else if keyword_eq(basis, "auto") {
                style.flex_basis = Dimension::Auto;
            }
        }
    }
}

// ---- 辅助函数：从 CSSValue 中提取值 ----

/// 从 CSSValue 中提取长度值（px 单位）
fn resolve_length(value: &CSSValue) -> Option<f32> {
    match value {
        CSSValue::Length(val, _) => Some(*val),
        CSSValue::Keyword(k) => {
            if let Some(px) = k.strip_suffix("px") {
                px.parse::<f32>().ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 获取 CSSValue 的关键字字符串
fn css_keyword(value: &CSSValue) -> &str {
    match value {
        CSSValue::Keyword(k) => k.as_str(),
        _ => "",
    }
}

/// 检查 CSSValue 是否匹配指定关键字
fn keyword_eq(value: &CSSValue, target: &str) -> bool {
    matches!(value, CSSValue::Keyword(k) if k == target)
}

/// 从 CSSValue 中提取数字（Number 或 可解析为数字的 Keyword）
fn css_number(value: &CSSValue) -> Option<f32> {
    match value {
        CSSValue::Number(n) => Some(*n),
        CSSValue::Keyword(k) => k.parse::<f32>().ok(),
        _ => None,
    }
}

/// 检查 ComputedStyle 是否使用 border-box 盒模型
fn is_border_box_style(cs: &ComputedStyle) -> bool {
    match cs.get("box-sizing") {
        Some(CSSValue::Keyword(k)) => k == "border-box",
        _ => false,
    }
}

// Phase 2+: Grid 布局
