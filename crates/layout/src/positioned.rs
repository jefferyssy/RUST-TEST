//! 定位布局 —— position: absolute / fixed / relative
//!
//! Phase 0 实现：
//!   - position: relative（相对自身偏移）
//!   - position: absolute（相对已定位祖先）
//!
//! Phase 1+:
//!   - position: fixed（相对视口）
//!   - position: sticky（粘性定位）

use crate::layout_box::{BoxType, LayoutBox};
use dom::{Rect, Size};

/// 定位祖先信息（按值传递，避免借用冲突）
#[derive(Clone, Copy)]
struct PositionedAncestorInfo {
    rect: Rect<f32>,
    padding_left: f32,
    padding_top: f32,
    content_width: f32,
    content_height: f32,
}

/// 定位布局引擎
pub struct PositionedLayout;

impl PositionedLayout {
    /// 执行定位布局
    pub fn layout(&self, root: &mut LayoutBox, viewport: Size<f32>) {
        self.layout_positioned(root, viewport, None);
    }

    fn layout_positioned(
        &self,
        node: &mut LayoutBox,
        viewport: Size<f32>,
        positioned_ancestor: Option<PositionedAncestorInfo>,
    ) {
        match node.box_type {
            BoxType::Absolute => {
                self.layout_absolute(node, viewport, &positioned_ancestor);
            }
            BoxType::Fixed => {
                self.layout_fixed(node, viewport);
            }
            BoxType::Sticky => {
                self.apply_relative_offset(node);
            }
            _ => {
                self.apply_relative_offset(node);
            }
        }

        // 确定传递给子节点的定位祖先
        let pa = if matches!(
            node.box_type,
            BoxType::Absolute | BoxType::Fixed | BoxType::Sticky
        ) {
            Some(PositionedAncestorInfo {
                rect: node.rect,
                padding_left: node.padding.left,
                padding_top: node.padding.top,
                content_width: node.content_area().width,
                content_height: node.content_area().height,
            })
        } else {
            positioned_ancestor
        };

        // 收集子节点索引，然后通过索引访问（避免同时持有 node 的不可变和可变引用）
        let child_count = node.children.len();
        for i in 0..child_count {
            self.layout_positioned(&mut node.children[i], viewport, pa);
        }
    }

    /// 处理 position: absolute
    fn layout_absolute(
        &self,
        node: &mut LayoutBox,
        _viewport: Size<f32>,
        ancestor: &Option<PositionedAncestorInfo>,
    ) {
        let (top, right, bottom, left) = Self::parse_offsets(node);
        if let Some(anc) = ancestor {
            let ref_x = anc.rect.x + anc.padding_left;
            let ref_y = anc.rect.y + anc.padding_top;

            if left != 0.0 {
                node.rect.x = ref_x + left;
            } else if right != 0.0 {
                node.rect.x = ref_x + anc.content_width - node.rect.width - right;
            }
            if top != 0.0 {
                node.rect.y = ref_y + top;
            } else if bottom != 0.0 {
                node.rect.y = ref_y + anc.content_height - node.rect.height - bottom;
            }
        }
    }

    /// 处理 position: fixed
    fn layout_fixed(&self, node: &mut LayoutBox, viewport: Size<f32>) {
        let (top, right, bottom, left) = Self::parse_offsets(node);

        if left != 0.0 {
            node.rect.x = left;
        } else if right != 0.0 {
            node.rect.x = viewport.width - node.rect.width - right;
        }
        if top != 0.0 {
            node.rect.y = top;
        } else if bottom != 0.0 {
            node.rect.y = viewport.height - node.rect.height - bottom;
        }
    }

    /// 处理 position: relative 偏移
    fn apply_relative_offset(&self, node: &mut LayoutBox) {
        let (top, right, bottom, left) = Self::parse_offsets(node);

        if top != 0.0 || bottom != 0.0 || left != 0.0 || right != 0.0 {
            let x_offset = if left != 0.0 { left } else { -right };
            let y_offset = if top != 0.0 { top } else { -bottom };
            node.rect.x += x_offset;
            node.rect.y += y_offset;
        }
    }

    /// 解析偏移
    fn parse_offsets(node: &LayoutBox) -> (f32, f32, f32, f32) {
        let style = node.computed_style.as_ref();
        (
            Self::parse_position_value(style.and_then(|s| s.get("top"))),
            Self::parse_position_value(style.and_then(|s| s.get("right"))),
            Self::parse_position_value(style.and_then(|s| s.get("bottom"))),
            Self::parse_position_value(style.and_then(|s| s.get("left"))),
        )
    }

    fn parse_position_value(value: Option<&style::values::CSSValue>) -> f32 {
        match value {
            Some(style::values::CSSValue::Length(val, _)) => *val,
            Some(style::values::CSSValue::Keyword(s)) if s == "auto" => 0.0,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
#[path = "positioned.test.rs"]
mod tests;
