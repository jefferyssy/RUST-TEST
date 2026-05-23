//! LayoutBox —— 布局树节点
//!
//! 布局树是 DOM 树经过 CSS 计算后的"平面化"表示。
//! 每个 DOM 节点对应一个或多个 LayoutBox。
//! 布局引擎的输入：DOM 树 + ComputedStyle 映射
//! 布局引擎的输出：每个 LayoutBox 的 rect 被填充

use std::cell::RefCell;
use std::rc::Rc;

use style::cascade::ComputedStyle;
use dom::{Node, Rect, Size};

/// 布局框类型 —— 参与布局计算的基本单元
#[derive(Debug, Clone, PartialEq)]
pub enum BoxType {
    /// 块级框（display: block）
    Block,
    /// 行内框（display: inline）
    Inline,
    /// Flex 容器（display: flex）
    FlexContainer,
    /// Flex 子项（Flex 容器的直接子元素）
    FlexItem,
    /// 文本行框
    Text,
    /// 匿名框（包裹行内元素的不可见框）
    Anonymous,
    // Phase 1 新增
    InlineBlock,
    Table,
    TableRow,
    TableCell,
    Absolute,
    Fixed,
    Sticky,
    // Phase 2 新增
    GridContainer,
    GridItem,
    Float,
}

/// 四边尺寸 —— 用于 margin、padding、border
#[derive(Debug, Clone, Copy, Default)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeSizes {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }
}

/// 溢出处理方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Default for Overflow {
    fn default() -> Self {
        Overflow::Visible
    }
}

/// 边框圆角
#[derive(Debug, Clone, Copy, Default)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl BorderRadius {
    pub fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }
}

/// 可见性
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Visible,
    Hidden,
    Collapse,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Visible
    }
}

/// 布局树节点 —— 每个 DOM 节点对应一个或多个 LayoutBox
///
/// 布局引擎的输入：DOM 树 + ComputedStyle
/// 布局引擎的输出：每个 LayoutBox 的 rect 被填充为最终位置
pub struct LayoutBox {
    /// 布局框类型
    pub box_type: BoxType,
    /// 对应的 DOM 节点指针（Text 节点可能为 None）
    pub node: Option<Rc<RefCell<Node>>>,
    /// 子布局框
    pub children: Vec<LayoutBox>,
    /// 计算结果：在视口中的位置和尺寸
    pub rect: Rect<f32>,
    /// 内边距（从 computedStyle 解析）
    pub padding: EdgeSizes,
    /// 外边距
    pub margin: EdgeSizes,
    /// 边框
    pub border: EdgeSizes,
    /// 计算样式
    pub computed_style: Option<ComputedStyle>,
    // Phase 1 新增字段
    pub z_index: i32,
    pub stacking_context: bool,
    pub overflow: Overflow,
    pub border_radius: BorderRadius,
    pub visibility: Visibility,
    // Phase 3: 脏标记（增量布局）
    pub dirty: bool,
}

impl LayoutBox {
    /// 创建新布局框
    pub fn new(box_type: BoxType, node: Option<Rc<RefCell<Node>>>) -> Self {
        Self {
            box_type,
            node,
            children: Vec::new(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            padding: EdgeSizes::default(),
            margin: EdgeSizes::default(),
            border: EdgeSizes::default(),
            computed_style: None,
            z_index: 0,
            stacking_context: false,
            overflow: Overflow::default(),
            border_radius: BorderRadius::default(),
            visibility: Visibility::default(),
            dirty: false,
        }
    }

    /// 添加子布局框
    pub fn append_child(&mut self, child: LayoutBox) {
        self.children.push(child);
    }

    /// 获取内容区域尺寸（去除 padding + border）
    pub fn content_area(&self) -> Size<f32> {
        Size::new(
            (self.rect.width - self.padding.left - self.padding.right
                - self.border.left - self.border.right)
                .max(0.0),
            (self.rect.height - self.padding.top - self.padding.bottom
                - self.border.top - self.border.bottom)
                .max(0.0),
        )
    }

    /// 设置内容区域尺寸（加上 padding + border 得到总尺寸）
    pub fn set_content_area(&mut self, size: Size<f32>) {
        self.rect.width = size.width
            + self.padding.left + self.padding.right
            + self.border.left + self.border.right;
        self.rect.height = size.height
            + self.padding.top + self.padding.bottom
            + self.border.top + self.border.bottom;
    }

    /// 获取包含边框的尺寸
    pub fn border_box(&self) -> Size<f32> {
        Size::new(self.rect.width, self.rect.height)
    }

    /// 获取包含 margin 的尺寸
    pub fn margin_box(&self) -> Size<f32> {
        Size::new(
            self.rect.width + self.margin.left + self.margin.right,
            self.rect.height + self.margin.top + self.margin.bottom,
        )
    }

    /// 遍历所有后代（深度优先）
    pub fn traverse<F: FnMut(&LayoutBox)>(&self, f: &mut F) {
        f(self);
        for child in &self.children {
            child.traverse(f);
        }
    }

    /// 遍历所有后代（可变引用，深度优先）
    pub fn traverse_mut<F: FnMut(&mut LayoutBox)>(&mut self, f: &mut F) {
        f(self);
        for child in &mut self.children {
            child.traverse_mut(f);
        }
    }

    /// 收集所有匹配条件的布局框
    pub fn find<F: Fn(&LayoutBox) -> bool>(&self, f: &F) -> Vec<&LayoutBox> {
        let mut result = Vec::new();
        if f(self) {
            result.push(self);
        }
        for child in &self.children {
            result.extend(child.find(f));
        }
        result
    }

    // Phase 3: 脏标记（增量布局）
    /// 检查自身或任意后代是否为脏
    pub fn has_dirty(&self) -> bool {
        self.dirty || self.children.iter().any(|c| c.has_dirty())
    }

    /// 标记自身为脏
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 清除自身及所有后代的脏标记
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
        for child in &mut self.children {
            child.clear_dirty();
        }
    }

    /// 查找包含指定 DOM 节点的布局框
    pub fn find_layout_node(&self, dom_node: &Rc<RefCell<Node>>) -> Option<&LayoutBox> {
        let target_ptr = Rc::as_ptr(dom_node) as usize;
        if let Some(ref node) = self.node {
            if Rc::as_ptr(node) as usize == target_ptr {
                return Some(self);
            }
        }
        for child in &self.children {
            if let Some(found) = child.find_layout_node(dom_node) {
                return Some(found);
            }
        }
        None
    }
}

// Phase 2+: GridContainer, GridItem, Float 等布局类型

#[cfg(test)]
#[path = "layout_box.test.rs"]
mod tests;

impl std::fmt::Debug for LayoutBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutBox")
            .field("box_type", &self.box_type)
            .field("children", &self.children.len())
            .field("rect", &self.rect)
            .field("padding", &self.padding)
            .field("margin", &self.margin)
            .field("border", &self.border)
            .finish_non_exhaustive()
    }
}

// Phase 1+: InlineBlock, Table*, Grid* 等布局类型
