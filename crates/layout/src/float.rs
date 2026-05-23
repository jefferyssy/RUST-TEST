//! Float 布局 —— Phase 2
//!
//! 对应 CSS float 和 clear 属性。
//! 基础实现：左浮动/右浮动 + clear 清除。

use crate::layout_box::{BoxType, LayoutBox};
use dom::Size;

/// 浮动方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatDirection {
    Left,
    Right,
}

/// Clear 清除模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClearMode {
    None,
    Left,
    Right,
    Both,
}

/// Float 布局引擎
pub struct FloatLayout;

impl FloatLayout {
    /// 对容器执行浮动布局
    ///
    /// 浮动元素从正常流中取出，向左/右移动直到触及容器边界或另一个浮动元素。
    pub fn layout(&self, container: &mut LayoutBox, _viewport: Size<f32>) {
        let content_width = container.content_area().width;
        if content_width <= 0.0 {
            return;
        }

        let container_x = container.rect.x + container.padding.left + container.border.left;
        let container_y = container.rect.y + container.padding.top + container.border.top;

        let mut left_x = container_x;
        let mut right_x = container_x + content_width;
        let mut current_y = container_y;
        let mut max_float_height = 0.0f32;

        for child in container.children.iter_mut() {
            match child.box_type {
                BoxType::Float => {
                    let float_dir = Self::get_float_direction(child);
                    let clear = Self::get_clear_mode(child);

                    // 处理 clear
                    match clear {
                        ClearMode::Left => {
                            // Phase 2: 清除左边浮动
                        }
                        ClearMode::Right => {
                            // Phase 2: 清除右边浮动
                        }
                        ClearMode::Both => {
                            // Phase 2: 清除两侧浮动
                        }
                        ClearMode::None => {}
                    }

                    match float_dir {
                        FloatDirection::Left => {
                            child.rect.x = left_x;
                            child.rect.y = current_y;
                            left_x += child.rect.width + child.margin.left + child.margin.right;
                        }
                        FloatDirection::Right => {
                            child.rect.x = right_x - child.rect.width
                                - child.margin.left - child.margin.right;
                            child.rect.y = current_y;
                            right_x -= child.rect.width + child.margin.left + child.margin.right;
                        }
                    }

                    let float_h = child.rect.height + child.margin.top + child.margin.bottom;
                    if float_h > max_float_height {
                        max_float_height = float_h;
                    }
                }
                BoxType::Block => {
                    // 处理 clear 属性
                    let clear = Self::get_clear_mode(child);
                    let need_clear = match clear {
                        ClearMode::Left | ClearMode::Right | ClearMode::Both => {
                            max_float_height > 0.0
                        }
                        ClearMode::None => false,
                    };

                    if need_clear {
                        current_y += max_float_height;
                        max_float_height = 0.0;
                        left_x = container_x;
                        right_x = container_x + content_width;
                    }

                    // 调整块级元素宽度以避让浮动
                    if left_x > container_x {
                        child.rect.x = left_x;
                        child.rect.width = (right_x - left_x).max(0.0);
                    }

                    child.rect.y = current_y;
                    current_y += child.rect.height + child.margin.top + child.margin.bottom;
                }
                _ => {
                    // 其他类型正常流
                    child.rect.y = current_y;
                    current_y += child.rect.height + child.margin.top + child.margin.bottom;
                }
            }
        }

        // 更新容器高度以适应浮动
        if max_float_height > 0.0 && current_y < container_y + max_float_height {
            current_y = container_y + max_float_height;
        }

        if current_y > container.rect.y + container.rect.height {
            container.rect.height = current_y - container.rect.y + container.padding.bottom;
        }
    }

    /// 从 ComputedStyle 提取 float 方向
    fn get_float_direction(box_ref: &LayoutBox) -> FloatDirection {
        if let Some(style) = &box_ref.computed_style {
            if let Some(val) = style.get("float") {
                match val {
                    style::values::CSSValue::Keyword(s) if s == "left" => return FloatDirection::Left,
                    style::values::CSSValue::Keyword(s) if s == "right" => return FloatDirection::Right,
                    _ => {}
                }
            }
        }
        FloatDirection::Left // 默认
    }

    /// 从 ComputedStyle 提取 clear 模式
    fn get_clear_mode(box_ref: &LayoutBox) -> ClearMode {
        if let Some(style) = &box_ref.computed_style {
            if let Some(val) = style.get("clear") {
                match val {
                    style::values::CSSValue::Keyword(s) if s == "left" => return ClearMode::Left,
                    style::values::CSSValue::Keyword(s) if s == "right" => return ClearMode::Right,
                    style::values::CSSValue::Keyword(s) if s == "both" => return ClearMode::Both,
                    _ => {}
                }
            }
        }
        ClearMode::None
    }
}
