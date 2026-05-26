//! Block 布局 —— 从顶到底逐行排列
//!
//! 实现 W3C CSS 2.1 Block 布局规范的基础版本。
//! 块级元素从上到下排列，每个元素占满可用宽度。
//! P1-10: BFC 基础 —— overflow != visible 的元素创建独立 BFC，
//!   BFC 内 margin 不与外部折叠。

use crate::layout_box::{BoxType, LayoutBox, Overflow};
use dom::Size;
use style::values::CSSValue;

/// Block 布局引擎
pub struct BlockLayout;

impl BlockLayout {
    /// 对 Block 容器执行布局
    pub fn layout(&self, container: &mut LayoutBox, _viewport: Size<f32>) {
        let content_width = container.content_area().width;
        if content_width <= 0.0 {
            return;
        }

        let mut current_y = container.rect.y;
        let mut prev_margin_bottom = 0.0f32;
        let mut prev_was_bfc = false;

        let text_align = container
            .computed_style
            .as_ref()
            .and_then(|s| s.get("text-align"))
            .map(|v| match v {
                CSSValue::Keyword(k) => k.as_ref(),
                _ => "left",
            })
            .unwrap_or("left");

        // P1-10: 容器自身是否创建 BFC
        let container_is_bfc = is_bfc_container(container);

        for child in &mut container.children {
            // P1-10: BFC 子元素不与上一个兄弟折叠 margin
            let child_is_bfc = is_bfc_container(child);
            let collapsed_offset = if child_is_bfc && !prev_was_bfc {
                // BFC 不与上一个非 BFC 兄弟折叠
                child.margin.top
            } else if !child_is_bfc && prev_was_bfc {
                // 前一个 BFC 不与当前非 BFC 兄弟折叠
                prev_margin_bottom + child.margin.top
            } else {
                Self::collapse_margins(prev_margin_bottom, child.margin.top)
            };

            // P1-10: BFC 容器内部 margin 不与外部折叠
            // 当容器自身是 BFC 时，其首个子元素的 margin-top 和末个子元素的 margin-bottom
            // 不与容器外部元素折叠
            let top_offset = if container_is_bfc {
                0.0
            } else {
                collapsed_offset
            };

            child.rect.x = container.rect.x
                + container.padding.left
                + container.border.left;

            match child.box_type {
                BoxType::Block | BoxType::Anonymous | BoxType::FlexContainer
                | BoxType::GridContainer | BoxType::Table => {
                    child.rect.width = content_width;
                }
                BoxType::Inline | BoxType::InlineBlock | BoxType::Text
                | BoxType::FlexItem | BoxType::GridItem => {
                    let available = content_width - child.rect.width;
                    match text_align {
                        "center" => child.rect.x += (available / 2.0).max(0.0),
                        "right" | "end" => child.rect.x += available.max(0.0),
                        _ => {}
                    }
                }
                _ => {}
            }

            current_y += top_offset;
            child.rect.y = current_y
                + container.padding.top
                + container.border.top;

            current_y += child.rect.height;
            prev_margin_bottom = child.margin.bottom;
            prev_was_bfc = child_is_bfc;
        }
    }

    /// 外边距折叠：相邻块级元素的垂直 margin 取较大值
    fn collapse_margins(prev_bottom: f32, current_top: f32) -> f32 {
        if prev_bottom >= 0.0 && current_top >= 0.0 {
            prev_bottom.max(current_top)
        } else if prev_bottom <= 0.0 && current_top <= 0.0 {
            prev_bottom.min(current_top)
        } else {
            prev_bottom + current_top
        }
    }
}

/// P1-10: 判断容器是否创建独立的 BFC（Block Formatting Context）
///
/// BFC 触发条件：
/// - overflow != visible（hidden / scroll / auto）
/// - display: flow-root（未来）
/// - float != none（未来）
/// - position: absolute / fixed（未来）
pub fn is_bfc_container(node: &LayoutBox) -> bool {
    if node.overflow != Overflow::Visible {
        return true;
    }
    false
}

#[cfg(test)]
#[path = "block.test.rs"]
mod tests;
