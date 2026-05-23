//! Flexbox 布局 —— 通过 taffy crate 实现
//!
//! 支持的 flex 属性：
//!   flex-direction, flex-wrap, justify-content,
//!   align-items, align-content, gap,
//!   flex-grow, flex-shrink, flex-basis

use taffy::{prelude::*, TaffyTree};

use crate::layout_box::LayoutBox;
use dom::Size;

/// Flexbox 布局引擎
pub struct FlexLayout;

impl FlexLayout {
    /// 对 Flex 容器执行布局
    ///
    /// taffy: taffy 布局引擎实例
    /// container: 必须是 FlexContainer 类型
    /// viewport: 当前视口尺寸
    pub fn layout(
        &mut self,
        taffy: &mut Option<TaffyTree>,
        container: &mut LayoutBox,
        viewport: Size<f32>,
    ) {
        if container.children.is_empty() {
            return;
        }

        let Some(taffy_tree) = taffy.as_mut() else {
            return;
        };

        // 1. 为容器及其子节点创建 taffy node
        let mut node_map: Vec<taffy::NodeId> = Vec::new();

        // 创建容器节点
        let container_style = Self::convert_style(container, viewport);
        let container_id = taffy_tree.new_leaf(container_style).unwrap();
        node_map.push(container_id);

        // 创建子节点
        for child in &container.children {
            let child_style = Self::convert_style(child, viewport);
            let child_id = taffy_tree.new_leaf(child_style).unwrap();
            taffy_tree.add_child(container_id, child_id).unwrap();
            node_map.push(child_id);
        }

        // 2. 执行布局计算
        taffy_tree
            .compute_layout(container_id, taffy::Size::MAX_CONTENT)
            .unwrap();

        // 3. 将 taffy 结果写回 LayoutBox
        // 容器自身的布局结果
        if let Ok(layout) = taffy_tree.layout(container_id) {
            container.rect.x = layout.location.x;
            container.rect.y = layout.location.y;
            container.rect.width = layout.size.width;
            container.rect.height = layout.size.height;
        }

        // 子节点的布局结果
        for (i, child) in container.children.iter_mut().enumerate() {
            if i < node_map.len() - 1 {
                let child_id = node_map[i + 1];
                if let Ok(layout) = taffy_tree.layout(child_id) {
                    child.rect.x = layout.location.x;
                    child.rect.y = layout.location.y;
                    child.rect.width = layout.size.width;
                    child.rect.height = layout.size.height;
                }
            }
        }

        // 清理 taffy tree（为下一次布局做准备）
        taffy_tree.clear();
    }

    /// 将 LayoutBox 的属性转换为 taffy 的 Style
    fn convert_style(box_node: &LayoutBox, _viewport: Size<f32>) -> taffy::Style {
        let mut style = taffy::Style::default();

        // 从 LayoutBox 的 padding/border 转发到 taffy（盒模型）
        style.padding.left = taffy::LengthPercentage::Length(box_node.padding.left);
        style.padding.right = taffy::LengthPercentage::Length(box_node.padding.right);
        style.padding.top = taffy::LengthPercentage::Length(box_node.padding.top);
        style.padding.bottom = taffy::LengthPercentage::Length(box_node.padding.bottom);

        style.border.left = taffy::LengthPercentage::Length(box_node.border.left);
        style.border.right = taffy::LengthPercentage::Length(box_node.border.right);
        style.border.top = taffy::LengthPercentage::Length(box_node.border.top);
        style.border.bottom = taffy::LengthPercentage::Length(box_node.border.bottom);

        // 将 LayoutBox 的 rect 尺寸作为 min_size，确保 taffy 不会将元素缩为 0
        if box_node.rect.width > 0.0 {
            style.min_size.width = taffy::Dimension::Length(box_node.rect.width);
        }
        if box_node.rect.height > 0.0 {
            style.min_size.height = taffy::Dimension::Length(box_node.rect.height);
        }

        // 从 computed_style 中读取 flex 属性
        if let Some(ref cs) = box_node.computed_style {
            // flex-direction
            if let Some(dir) = cs.get("flex-direction") {
                let dir_str = format!("{:?}", dir);
                style.flex_direction = match dir_str.as_str() {
                    "row" => FlexDirection::Row,
                    "row-reverse" => FlexDirection::RowReverse,
                    "column" => FlexDirection::Column,
                    "column-reverse" => FlexDirection::ColumnReverse,
                    _ => FlexDirection::Row,
                };
            }

            // justify-content
            if let Some(jc) = cs.get("justify-content") {
                let jc_str = format!("{:?}", jc);
                style.justify_content = Some(match jc_str.as_str() {
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
                let ai_str = format!("{:?}", ai);
                style.align_items = Some(match ai_str.as_str() {
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
                let g_str = format!("{:?}", g);
                if let Some(px) = g_str.strip_suffix("px") {
                    if let Ok(val) = px.parse::<f32>() {
                        let lp = taffy::LengthPercentage::Length(val);
                        style.gap = taffy::Size { width: lp, height: lp };
                    }
                }
            }

            // width / height → taffy 的 size 字段
            if let Some(w) = cs.get("width") {
                let w_str = format!("{:?}", w);
                if w_str == "auto" {
                    style.size.width = Dimension::Auto;
                } else if let Some(px) = w_str.strip_suffix("px") {
                    if let Ok(val) = px.parse::<f32>() {
                        style.size.width = Dimension::Length(val);
                    }
                }
            }

            if let Some(h) = cs.get("height") {
                let h_str = format!("{:?}", h);
                if h_str == "auto" {
                    style.size.height = Dimension::Auto;
                } else if let Some(px) = h_str.strip_suffix("px") {
                    if let Ok(val) = px.parse::<f32>() {
                        style.size.height = Dimension::Length(val);
                    }
                }
            }

            // Phase 1: min/max-width 约束
            if let Some(min_w) = cs.get("min-width") {
                let w_str = format!("{:?}", min_w);
                if let Some(px) = w_str.strip_suffix("px") {
                    if let Ok(val) = px.parse::<f32>() {
                        style.min_size.width = Dimension::Length(val);
                    }
                }
            }
            if let Some(max_w) = cs.get("max-width") {
                let w_str = format!("{:?}", max_w);
                if let Some(px) = w_str.strip_suffix("px") {
                    if let Ok(val) = px.parse::<f32>() {
                        style.max_size.width = Dimension::Length(val);
                    }
                }
            }
            if let Some(min_h) = cs.get("min-height") {
                let h_str = format!("{:?}", min_h);
                if let Some(px) = h_str.strip_suffix("px") {
                    if let Ok(val) = px.parse::<f32>() {
                        style.min_size.height = Dimension::Length(val);
                    }
                }
            }
            if let Some(max_h) = cs.get("max-height") {
                let h_str = format!("{:?}", max_h);
                if let Some(px) = h_str.strip_suffix("px") {
                    if let Ok(val) = px.parse::<f32>() {
                        style.max_size.height = Dimension::Length(val);
                    }
                }
            }

            // Phase 1: flex-wrap
            if let Some(wrap) = cs.get("flex-wrap") {
                let wrap_str = format!("{:?}", wrap);
                style.flex_wrap = match wrap_str.as_str() {
                    "wrap" => FlexWrap::Wrap,
                    "wrap-reverse" => FlexWrap::WrapReverse,
                    _ => FlexWrap::NoWrap,
                };
            }

            // Phase 1: flex 简写属性（被单独的 flex-grow/flex-shrink/flex-basis 覆盖）
            if let Some(flex_val) = cs.get("flex") {
                let val_str = format!("{:?}", flex_val);
                if val_str == "auto" {
                    style.flex_grow = 1.0;
                    style.flex_shrink = 1.0;
                    style.flex_basis = Dimension::Auto;
                } else if let Ok(val) = val_str.parse::<f32>() {
                    style.flex_grow = val;
                    style.flex_shrink = 1.0;
                    style.flex_basis = Dimension::Length(0.0);
                }
            }

            // Phase 1: flex-grow, flex-shrink, flex-basis（覆盖 flex 简写）
            if let Some(grow) = cs.get("flex-grow") {
                let grow_str = format!("{:?}", grow);
                if let Ok(val) = grow_str.parse::<f32>() {
                    style.flex_grow = val;
                }
            }
            if let Some(shrink) = cs.get("flex-shrink") {
                let shrink_str = format!("{:?}", shrink);
                if let Ok(val) = shrink_str.parse::<f32>() {
                    style.flex_shrink = val;
                }
            }
            if let Some(basis) = cs.get("flex-basis") {
                let basis_str = format!("{:?}", basis);
                if basis_str == "auto" {
                    style.flex_basis = Dimension::Auto;
                } else if let Some(px) = basis_str.strip_suffix("px") {
                    if let Ok(val) = px.parse::<f32>() {
                        style.flex_basis = Dimension::Length(val);
                    }
                }
            }
        }

        style
    }
}

// Phase 2+: Grid 布局
