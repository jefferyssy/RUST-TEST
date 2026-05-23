//! Inline 布局 —— 行内元素从左到右排列
//!
//! Phase 1: 基础行内布局实现
//! 行内元素在同一行内水平排列，超出宽度时自动换行。

use crate::layout_box::LayoutBox;
use dom::Size;

/// Inline 布局引擎
pub struct InlineLayout;

impl InlineLayout {
    /// 对 inline 容器执行布局
    ///
    /// 行内元素从左到右排列，超出宽度换行
    pub fn layout(&self, container: &mut LayoutBox, _viewport: Size<f32>) {
        let content_width = container.content_area().width;
        if content_width <= 0.0 {
            return;
        }

        let mut current_x = container.rect.x
            + container.padding.left
            + container.border.left;
        let mut current_y = container.rect.y
            + container.padding.top
            + container.border.top;
        let mut line_height = 0.0f32;

        for child in &mut container.children {
            // 检查是否需要换行
            if current_x + child.rect.width > container.rect.x + container.rect.width
                && current_x > container.rect.x + container.padding.left + container.border.left
            {
                current_x = container.rect.x
                    + container.padding.left
                    + container.border.left;
                current_y += line_height;
                line_height = 0.0;
            }

            child.rect.x = current_x;
            child.rect.y = current_y;

            current_x += child.rect.width + child.margin.left + child.margin.right;
            let child_line_h = child.rect.height + child.margin.top + child.margin.bottom;
            if child_line_h > line_height {
                line_height = child_line_h;
            }
        }
    }
}

// Phase 2+: 双向文本、vertical-align、text-align: justify
