//! Block 布局 —— 从顶到底逐行排列
//!
//! 实现 W3C CSS 2.1 Block 布局规范的基础版本。
//! 块级元素从上到下排列，每个元素占满可用宽度。
//! Phase 0: 支持 text-align（left / center / right）对齐行内子元素。

use crate::layout_box::{BoxType, LayoutBox};
use dom::Size;
use style::values::CSSValue;

/// Block 布局引擎
pub struct BlockLayout;

impl BlockLayout {
    /// 对 Block 容器执行布局
    ///
    /// container.children 按顺序从上到下排列
    /// Block 子节点宽度 = container.content_area.width
    /// Inline/InlineBlock/Text 子节点根据 text-align 水平对齐
    /// 垂直收缩包裹：若容器未设置显式 height，则收缩到内容高度
    pub fn layout(&self, container: &mut LayoutBox, _viewport: Size<f32>) {
        let content_width = container.content_area().width;
        if content_width <= 0.0 {
            return;
        }

        let mut current_y = container.rect.y;
        let mut prev_margin_bottom = 0.0f32;

        let text_align = container
            .computed_style
            .as_ref()
            .and_then(|s| s.get("text-align"))
            .map(|v| match v {
                CSSValue::Keyword(k) => k.as_ref(),
                _ => "left",
            })
            .unwrap_or("left");

        for child in &mut container.children {
            let collapsed_offset = Self::collapse_margins(prev_margin_bottom, child.margin.top);

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
                    // Shrink-to-fit: don't stretch, apply text-align
                    let available = content_width - child.rect.width;
                    match text_align {
                        "center" => child.rect.x += (available / 2.0).max(0.0),
                        "right" | "end" => child.rect.x += available.max(0.0),
                        _ => {}
                    }
                }
                _ => {}
            }

            // 在放置子节点前加入外边距间隙
            current_y += collapsed_offset;
            child.rect.y = current_y
                + container.padding.top
                + container.border.top;

            current_y += child.rect.height;
            prev_margin_bottom = child.margin.bottom;
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

// Phase 2+: BFC, clear

#[cfg(test)]
#[path = "block.test.rs"]
mod tests;
