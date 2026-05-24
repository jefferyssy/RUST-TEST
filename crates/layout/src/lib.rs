//! # layout crate — 布局引擎
//!
//! 负责将 DOM 树 + CSS 计算样式转换为每个节点的位置和尺寸。
//! 输出 LayoutTree（每个节点有精确的 Rect）。
//!
//! 独立模块设计，不依赖 render/runtime。

pub mod layout_box;
pub mod flex;
pub mod block;
pub mod positioned;
pub mod text;
pub mod inline;
pub mod table;
pub mod grid;
pub mod float;

pub use layout_box::{LayoutBox, BoxType, EdgeSizes, Overflow, BorderRadius, Visibility};
pub use flex::FlexLayout;
pub use block::BlockLayout;
pub use positioned::PositionedLayout;
pub use text::{TextMeasurer, char_width_factor};
pub use inline::InlineLayout;
pub use table::TableLayout;
pub use grid::GridLayout;
pub use float::{FloatLayout, FloatDirection, ClearMode};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use style::cascade::ComputedStyle;
use dom::{Node, NodeType, Size};

/// 布局引擎 —— 负责计算每个节点的位置和尺寸
///
/// 使用流程：
/// 1. 构建布局树：build_layout_tree(&dom_root, &styles)
/// 2. 执行布局：LayoutEngine::new().layout(&mut layout_root, viewport)
/// 3. 读取结果：每个 LayoutBox.rect 为最终位置
pub struct LayoutEngine {
    /// taffy 布局实例（用于 Flexbox）
    taffy: Option<taffy::TaffyTree>,
    /// 文本测量器（rustybuzz + fontdb 精确测量）
    pub text_measurer: TextMeasurer,
    // Phase 1+: 布局缓存
}

impl LayoutEngine {
    /// 创建新布局引擎
    pub fn new() -> Self {
        Self {
            taffy: Some(taffy::TaffyTree::new()),
            text_measurer: TextMeasurer::new(),
        }
    }

    /// 主入口：执行完整布局计算
    ///
    /// 参数：
    ///   - root: 布局树根节点
    ///   - viewport: 视口尺寸（通常为窗口尺寸）
    ///
    /// 输出：root 及其所有后代的 rect 被填充
    pub fn layout(&mut self, root: &mut LayoutBox, viewport: Size<f32>) {
        // Phase 0: 设置根节点宽度为视口宽度（块级元素默认行为）
        root.rect.width = viewport.width;

        // 递归计算所有节点尺寸（内部容器会收缩包裹内容高度）
        self.calculate_sizes(root, viewport, None);

        // 根节点高度：若内容不足视口则撑满，若内容溢出则跟随内容
        // 这样 body 背景能覆盖整个窗口，类似 Chrome 行为
        if root.rect.height < viewport.height {
            root.rect.height = viewport.height;
        }

        // 执行定位布局（在 main layout 之后）
        let positioned = PositionedLayout;
        positioned.layout(root, viewport);
    }

    /// 递归计算尺寸
    fn calculate_sizes(&mut self, node: &mut LayoutBox, viewport: Size<f32>, parent_box_type: Option<BoxType>) {
        // 根据布局类型分发
        let box_type = node.box_type.clone();
        match box_type {
            BoxType::FlexContainer => {
                // Phase 0: 自底向上先计算子级尺寸（嵌套 flex 容器需先确定内容尺寸）
                // 这样 taffy 才能用正确的子级尺寸进行 space-between / align-items 计算
                self.calculate_sizes_children(node, viewport);
                let mut flex = FlexLayout;
                // 仅当父级为 Block 时容器宽度可信赖（由 BlockLayout 设定）
                // 父级为 Flex/Grid 时，构建阶段宽度估计使用 Block 规则（max），不适用于 Flex
                let constrain_width = parent_box_type.as_ref().map_or(true, |pt| {
                    matches!(pt, BoxType::Block | BoxType::Anonymous)
                });
                flex.layout(&mut self.taffy, node, viewport, constrain_width);
            }
            BoxType::Block | BoxType::Anonymous => {
                let block = BlockLayout;
                block.layout(node, viewport);
            }
            BoxType::Text => {
                // 文本节点的尺寸由父节点决定
            }
            BoxType::FlexItem => {
                // FlexItem 的尺寸由 FlexLayout 处理
                self.calculate_sizes_children(node, viewport);
                return;
            }
            BoxType::Inline => {
                let block = BlockLayout;
                block.layout(node, viewport);
            }
            // Phase 1: 新布局类型
            BoxType::InlineBlock => {
                let block = BlockLayout;
                block.layout(node, viewport);
            }
            BoxType::Table => {
                // Phase 1: 表格布局暂委托给 BlockLayout
                let block = BlockLayout;
                block.layout(node, viewport);
            }
            BoxType::TableRow | BoxType::TableCell => {
                // 表格行/单元格由 Table 容器处理
                self.calculate_sizes_children(node, viewport);
                return;
            }
            BoxType::Absolute | BoxType::Fixed => {
                // 定位元素在 PositionedLayout 中处理
                let block = BlockLayout;
                block.layout(node, viewport);
            }
            BoxType::Sticky => {
                // Phase 1: sticky 暂按 relative 处理
                let block = BlockLayout;
                block.layout(node, viewport);
            }
            BoxType::GridContainer => {
                let grid = GridLayout;
                grid.layout(node, viewport);
            }
            BoxType::GridItem => {
                // GridItem 尺寸由 Grid 容器决定
                self.calculate_sizes_children(node, viewport);
                return;
            }
            BoxType::Float => {
                // Float 元素尺寸先按 Block 计算
                let block = BlockLayout;
                block.layout(node, viewport);
            }
        }

        // Phase 3: 应用 aspect-ratio 约束
        self.apply_aspect_ratio(node);

        // 递归处理子节点（子节点高度在此确定）
        self.calculate_sizes_children(node, viewport);

        // 子节点布局完成后，收缩包裹当前节点的高度
        self.shrink_to_content(node);

        // 子节点布局完成后，Block 容器内兄弟节点的高度可能已改变，
        // 必须重跑 BlockLayout 以确保所有兄弟节点位置正确。
        // 典型场景：空 #todo-list 初始高度 0，子 li 布局后高度增长，
        // 其后的 footer 需要向下推移。仅检查容器自身高度变化不够，
        // 因为 shrink_to_content 基于的是旧的兄弟节点 rect.y。
        match box_type {
            BoxType::Block | BoxType::Anonymous | BoxType::Inline | BoxType::InlineBlock => {
                let block = BlockLayout;
                block.layout(node, viewport);
            }
            _ => {}
        }
    }

    /// 递归处理所有子节点的尺寸
    fn calculate_sizes_children(&mut self, node: &mut LayoutBox, viewport: Size<f32>) {
        let parent_content_width = node.content_area().width;
        // Flex/Grid 容器内子元素尺寸由 taffy/grid 决定，不强制拉伸
        let skip_stretch = matches!(
            node.box_type,
            BoxType::FlexContainer | BoxType::GridContainer
        );
        let child_count = node.children.len();
        for i in 0..child_count {
            let child = &mut node.children[i];
            if !skip_stretch {
                match child.box_type {
                    BoxType::Block | BoxType::Anonymous => {
                        if child.rect.width < parent_content_width {
                            child.rect.width = parent_content_width;
                        }
                    }
                    _ => {}
                }
            }

            // Phase 0: 应用 max-width 约束（拉伸之后、布局之前）
            if let Some(ref cs) = child.computed_style {
                if let Some(max_w_val) = cs.get("max-width") {
                    if let Some(max_w) = resolve_length_cs(max_w_val) {
                        if child.rect.width > max_w {
                            child.rect.width = max_w;
                        }
                    }
                }
            }

            // Phase 0: margin: 0 auto 水平居中（仅 Block 容器子级，
            // Flex/Grid 容器由 taffy 的 auto margin 处理）
            // 跳过 Text/Inline 节点：它们继承父元素的 computed_style，
            // 但 margin 不应应用于内联级节点
            if !skip_stretch && !matches!(
                child.box_type,
                BoxType::Text | BoxType::Inline
            ) {
                if let Some(ref cs) = child.computed_style {
                    let margin_shorthand = cs.get("margin");
                    let margin_left = cs.get("margin-left");
                    let margin_right = cs.get("margin-right");
                    let auto_left = is_auto_cs(margin_left)
                        || is_auto_shorthand_cs(margin_shorthand, 3);
                    let auto_right = is_auto_cs(margin_right)
                        || is_auto_shorthand_cs(margin_shorthand, 1);
                    if auto_left || auto_right {
                        let remaining = parent_content_width - child.rect.width;
                        if auto_left && auto_right {
                            child.rect.x += (remaining / 2.0).max(0.0);
                        } else if auto_left {
                            child.rect.x += remaining.max(0.0);
                        }
                    }
                }
            }

            let child_viewport = Size::new(viewport.width - node.rect.x, viewport.height - node.rect.y);
            self.calculate_sizes(&mut node.children[i], child_viewport, Some(node.box_type.clone()));
        }
    }

    // ============================================================
    //  Phase 3: aspect-ratio 约束
    // ============================================================

    /// 应用 aspect-ratio 约束
    ///
    /// 当只设置 width 或 height 之一时，根据 aspect-ratio 推导另一维度。
    /// 自动值: aspect-ratio: auto 从内容（图片原始尺寸）推导。
    fn apply_aspect_ratio(&self, node: &mut LayoutBox) {
        let ratio = match self.get_aspect_ratio(node) {
            Some(r) => r,
            None => return,
        };
        let (w_ratio, h_ratio) = ratio;
        if h_ratio == 0.0 {
            return;
        }

        let has_width = node.rect.width > 0.0;
        let has_height = node.rect.height > 0.0;

        // 从 content area 获取实际尺寸（排除 padding/border）
        let content = node.content_area();

        if has_width && !has_height {
            // 宽度已设置，推导高度
            let derived_h = content.width * h_ratio / w_ratio;
            node.set_content_area(Size::new(content.width, derived_h));
        } else if has_height && !has_width {
            // 高度已设置，推导宽度
            let derived_w = content.height * w_ratio / h_ratio;
            node.set_content_area(Size::new(derived_w, content.height));
        }
    }

    /// 收缩包裹：若容器未设置显式 height，按内容高度收缩
    ///
    /// 在 `calculate_sizes_children` 之后调用，此时子节点高度已经确定。
    /// 仅对 Block/Anonymous/Inline 类容器生效。
    fn shrink_to_content(&self, node: &mut LayoutBox) {
        // 有显式 height 的节点不收缩
        let has_explicit_height = node
            .computed_style
            .as_ref()
            .and_then(|s| s.get("height"))
            .is_some();
        if has_explicit_height || node.children.is_empty() {
            return;
        }

        match node.box_type {
            BoxType::Block | BoxType::Anonymous | BoxType::Inline => {}
            _ => return,
        }

        let mut content_bottom = 0.0f32;
        for child in &node.children {
            let child_bottom = child.rect.y - node.rect.y
                + child.rect.height
                + child.margin.bottom;
            if child_bottom > content_bottom {
                content_bottom = child_bottom;
            }
        }

        let new_height = content_bottom
            + node.padding.bottom
            + node.border.bottom;
        if new_height > 0.0 {
            node.rect.height = new_height;
        }
    }

    /// 从 computed_style 中提取 aspect-ratio 值
    fn get_aspect_ratio(&self, node: &LayoutBox) -> Option<(f32, f32)> {
        let style = node.computed_style.as_ref()?;
        let aspect_val = style.get("aspect-ratio")?;

        match aspect_val {
            style::values::CSSValue::Keyword(k) if k == "auto" => None,
            style::values::CSSValue::Composite(vals) if vals.len() >= 2 => {
                let w = match &vals[0] {
                    style::values::CSSValue::Number(n) => *n,
                    _ => return None,
                };
                let h = match &vals[1] {
                    style::values::CSSValue::Number(n) => *n,
                    _ => return None,
                };
                Some((w, h))
            }
            style::values::CSSValue::Number(n) => Some((*n, 1.0)),
            _ => None,
        }
    }

    // Phase 1: 局部重排
    pub fn partial_layout(&mut self, root: &mut LayoutBox, dirty_nodes: &[Rc<RefCell<Node>>], viewport: Size<f32>) {
        for dirty in dirty_nodes {
            if let Some(layout_node) = root.find_layout_node(dirty) {
                let idx = layout_node as *const LayoutBox as usize;
                self.relayout_node(root, idx, viewport, None);
            }
        }
    }

    fn relayout_node(&mut self, root: &mut LayoutBox, target_ptr: usize, viewport: Size<f32>, parent_box_type: Option<BoxType>) {
        let current_ptr = root as *const LayoutBox as usize;
        if current_ptr == target_ptr {
            self.calculate_sizes(root, viewport, parent_box_type);
            return;
        }
        for child in &mut root.children {
            self.relayout_node(child, target_ptr, viewport, Some(root.box_type.clone()));
        }
    }

    /// 更新布局树（增量更新）
    pub fn update_layout_tree(&mut self, root: &mut LayoutBox, viewport: Size<f32>) {
        if root.has_dirty() {
            self.layout(root, viewport);
            root.clear_dirty();
        }
    }

    // Phase 2+: 表格布局、Grid 布局
}

/// 从 DOM 树 + 样式映射构建布局树
///
/// computed_styles: 从 style::cascade::compute_element_style 获取
/// key 为 DOM 节点的 Rc 指针地址
/// text_measurer: 可选的文本测量器，用于精确测量文本宽度
pub fn build_layout_tree(
    dom_root: &Rc<RefCell<Node>>,
    computed_styles: &HashMap<usize, ComputedStyle>,
    text_measurer: Option<&mut TextMeasurer>,
) -> LayoutBox {
    build_layout_box(dom_root, computed_styles, 0, text_measurer)
}

/// 递归构建 LayoutBox
fn build_layout_box(
    dom_node: &Rc<RefCell<Node>>,
    computed_styles: &HashMap<usize, ComputedStyle>,
    _depth: usize,
    mut text_measurer: Option<&mut TextMeasurer>,
) -> LayoutBox {
    let node = dom_node.borrow();
    let ptr = Rc::as_ptr(dom_node) as usize;
    let style = computed_styles.get(&ptr);

    match &node.node_type {
        NodeType::Element(elem_data) => {
            // 判断盒子类型
            let display = style
                .and_then(|s| s.get("display"))
                .map(|v| format!("{:?}", v))
                .unwrap_or_default();

            let box_type = if display.contains("grid") {
                BoxType::GridContainer
            } else if display.contains("flex") {
                BoxType::FlexContainer
            } else if display.contains("inline-block") {
                BoxType::InlineBlock
            } else if display.contains("inline") {
                BoxType::Inline
            } else if display.contains("table-cell") {
                BoxType::TableCell
            } else if display.contains("table-row") {
                BoxType::TableRow
            } else if display.contains("table") {
                BoxType::Table
            } else {
                BoxType::Block
            };

            // Phase 0: Apply default display based on HTML tag (user agent styles)
            let box_type = if display.is_empty() && box_type == BoxType::Block {
                match elem_data.tag_name() {
                    "button" | "input" | "select" | "textarea" => BoxType::InlineBlock,
                    "span" | "a" | "em" | "strong" | "b" | "i" | "u" | "label" | "small" | "code" | "img" => BoxType::Inline,
                    _ => BoxType::Block,
                }
            } else {
                box_type
            };

            // Phase 2: 检测 float
            let box_type = {
                let float = style
                    .and_then(|s| s.get("float"))
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_default();
                if float.contains("left") || float.contains("right") {
                    BoxType::Float
                } else {
                    box_type
                }
            };

            // Phase 1: 检测 position 类型（position 优先级最高）
            let box_type = {
                let position = style
                    .and_then(|s| s.get("position"))
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_default();
                match position.as_str() {
                    "absolute" => BoxType::Absolute,
                    "fixed" => BoxType::Fixed,
                    "sticky" => BoxType::Sticky,
                    _ => box_type,
                }
            };

            let mut layout_box = LayoutBox::new(box_type, Some(dom_node.clone()));
            if let Some(s) = style {
                layout_box.computed_style = Some(s.clone());
            }

            // Phase 0: 从 computed style 解析盒模型简写值（padding / margin）
            apply_shorthand_property(style, &mut layout_box);

            // 递归构建子节点
            drop(node);
            let children = dom_node.borrow().child_nodes();
            for child in &children {
                let child_box = build_layout_box(child, computed_styles, _depth + 1, text_measurer.as_deref_mut());
                layout_box.append_child(child_box);
            }

            // Phase 0: 估算容器宽度 = 最大子节点宽度 + padding（必须在高度之前，避免 set_content_area 污染 width）
            if layout_box.rect.width <= 0.0 {
                let mut max_child_w = 0.0f32;
                for child in &layout_box.children {
                    let child_w = child.rect.width + child.margin.left + child.margin.right;
                    if child_w > max_child_w {
                        max_child_w = child_w;
                    }
                }
                if max_child_w > 0.0 {
                    layout_box.rect.width = max_child_w
                        + layout_box.padding.left + layout_box.padding.right;
                }
            }
            // Phase 0: 估算容器高度 = 子节点累积高度 + padding
            if layout_box.rect.height <= 0.0 {
                let mut content_h = 0.0;
                for child in &layout_box.children {
                    content_h += child.rect.height
                        + child.margin.top + child.margin.bottom;
                }
                if content_h > 0.0 {
                    layout_box.set_content_area(Size::new(
                        layout_box.content_area().width,
                        content_h,
                    ));
                }
            }
            layout_box
        }
        NodeType::Text(_) => {
            let mut layout_box = LayoutBox::new(BoxType::Text, Some(dom_node.clone()));
            if let Some(s) = style {
                layout_box.computed_style = Some(s.clone());
            }
            // Phase 0: 从字体大小估算文本尺寸，使块级布局正确堆叠
            let text = node.text_content();
            if !text.is_empty() {
                let font_size = style
                    .and_then(|s| s.get("font-size"))
                    .and_then(|v| {
                        if let style::values::CSSValue::Length(px, _) = v {
                            Some(*px)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(16.0);

                let font_family = style
                    .and_then(|s| s.get("font-family"))
                    .map(|v| match v {
                        style::values::CSSValue::Keyword(k) => k.as_str(),
                        _ => "sans-serif",
                    })
                    .unwrap_or("sans-serif");

                let font_weight = style
                    .and_then(|s| s.get("font-weight"))
                    .map(|v| match v {
                        style::values::CSSValue::Keyword(k) => match k.as_ref() {
                            "bold" => 700u16,
                            "bolder" => 900,
                            "lighter" => 100,
                            "normal" => 400,
                            _ => 400,
                        },
                        style::values::CSSValue::Number(n) => *n as u16,
                        _ => 400,
                    })
                    .unwrap_or(400);

                layout_box.rect.height = font_size * 1.2;

                // 优先使用 rustybuzz 精确测量，回退到字符感知估算
                if let Some(ref mut measurer) = text_measurer {
                    layout_box.rect.width = measurer.measure_width(
                        &text,
                        font_size,
                        font_family,
                        font_weight,
                    );
                } else {
                    layout_box.rect.width = text.chars()
                        .map(|c| text::char_width_factor(c) * font_size)
                        .sum();
                }
            }
            layout_box
        }
        NodeType::Document => {
            let mut layout_box = LayoutBox::new(BoxType::Block, Some(dom_node.clone()));
            drop(node);
            let children = dom_node.borrow().child_nodes();
            for child in &children {
                let child_box = build_layout_box(child, computed_styles, _depth + 1, text_measurer.as_deref_mut());
                layout_box.append_child(child_box);
            }
            // Document 高度 = 视口高度（在 layout 阶段设置）
            layout_box
        }
        NodeType::DocumentFragment => {
            // 文档片段：创建匿名块容器，展开子节点
            let mut layout_box = LayoutBox::new(BoxType::Anonymous, Some(dom_node.clone()));
            drop(node);
            let children = dom_node.borrow().child_nodes();
            for child in &children {
                let child_box = build_layout_box(child, computed_styles, _depth + 1, text_measurer.as_deref_mut());
                layout_box.append_child(child_box);
            }
            layout_box
        }
        NodeType::Comment(_) => {
            // 注释节点不参与布局
            LayoutBox::new(BoxType::Anonymous, Some(dom_node.clone()))
        }
    }
}

/// Phase 0: 从 ComputedStyle 解析简写属性到 LayoutBox 的 padding/margin/border
///
/// 支持简写（padding/margin/border）和单边属性（padding-top, margin-left 等）。
fn apply_shorthand_property(
    style: Option<&ComputedStyle>,
    layout_box: &mut LayoutBox,
) {
    let Some(s) = style else { return };

    // 解析简写属性（可被单边属性覆盖）
    if let Some(v) = s.get("padding") {
        if let Some((t, r, b, l)) = parse_four_sides(v) {
            layout_box.padding = EdgeSizes::new(t, r, b, l);
        }
    }
    if let Some(v) = s.get("margin") {
        if let Some((t, r, b, l)) = parse_four_sides(v) {
            layout_box.margin = EdgeSizes::new(t, r, b, l);
        }
    }
    // 从 border 简写中提取宽度（格式: "1px solid #ddd" → 提取 1px）
    if let Some(v) = s.get("border") {
        if let Some(w) = parse_border_width(v) {
            layout_box.border = EdgeSizes::new(w, w, w, w);
        }
    }
    // 解析 border-radius
    if let Some(v) = s.get("border-radius") {
        if let Some(val) = parse_length_value(v) {
            layout_box.border_radius = BorderRadius::uniform(val);
        }
    }
    // 解析 overflow（支持 overflow-x / overflow-y，后者覆盖简写）
    if let Some(v) = s.get("overflow") {
        layout_box.overflow = parse_overflow_value(v);
    }
    if let Some(v) = s.get("overflow-y") {
        layout_box.overflow = parse_overflow_value(v);
    }

    // 解析单边属性（覆盖简写对应边）
    if let Some(v) = s.get("padding-top") {
        if let Some(val) = parse_length_value(v) {
            layout_box.padding.top = val;
        }
    }
    if let Some(v) = s.get("padding-right") {
        if let Some(val) = parse_length_value(v) {
            layout_box.padding.right = val;
        }
    }
    if let Some(v) = s.get("padding-bottom") {
        if let Some(val) = parse_length_value(v) {
            layout_box.padding.bottom = val;
        }
    }
    if let Some(v) = s.get("padding-left") {
        if let Some(val) = parse_length_value(v) {
            layout_box.padding.left = val;
        }
    }
    if let Some(v) = s.get("margin-top") {
        if let Some(val) = parse_length_value(v) {
            layout_box.margin.top = val;
        }
    }
    if let Some(v) = s.get("margin-right") {
        if let Some(val) = parse_length_value(v) {
            layout_box.margin.right = val;
        }
    }
    if let Some(v) = s.get("margin-bottom") {
        if let Some(val) = parse_length_value(v) {
            layout_box.margin.bottom = val;
        }
    }
    if let Some(v) = s.get("margin-left") {
        if let Some(val) = parse_length_value(v) {
            layout_box.margin.left = val;
        }
    }
}

/// 从 CSSValue 提取单个长度值（px）
fn parse_length_value(value: &style::values::CSSValue) -> Option<f32> {
    match value {
        style::values::CSSValue::Length(px, _) => Some(*px),
        _ => None,
    }
}

/// 从 CSSValue 解析 overflow 值
fn parse_overflow_value(value: &style::values::CSSValue) -> Overflow {
    match value {
        style::values::CSSValue::Keyword(k) => match k.as_ref() {
            "hidden" => Overflow::Hidden,
            "scroll" => Overflow::Scroll,
            "auto" => Overflow::Auto,
            _ => Overflow::Visible,
        },
        _ => Overflow::Visible,
    }
}

/// 按 CSS 简写规则解析 1-4 个边长值，返回 (top, right, bottom, left)
fn parse_four_sides(value: &style::values::CSSValue) -> Option<(f32, f32, f32, f32)> {
    let raw = match value {
        style::values::CSSValue::Length(px, _) => return Some((*px, *px, *px, *px)),
        style::values::CSSValue::Keyword(s) => s.as_str(),
        _ => return None,
    };

    let values: Vec<f32> = raw
        .split_whitespace()
        .map(|t| style::values::parse_length(t).0)
        .collect();

    match values.len() {
        1 => Some((values[0], values[0], values[0], values[0])),
        2 => Some((values[0], values[1], values[0], values[1])),
        3 => Some((values[0], values[1], values[2], values[1])),
        4 => Some((values[0], values[1], values[2], values[3])),
        _ => None,
    }
}

/// 从 border 简写中提取宽度（如 "1px solid #ddd" → 1.0）
fn parse_border_width(value: &style::values::CSSValue) -> Option<f32> {
    let raw = match value {
        style::values::CSSValue::Keyword(s) => s.as_str(),
        _ => return None,
    };
    // 取第一个空格分隔的 token，如果含 px 则提取数值
    let first = raw.split_whitespace().next()?;
    first.strip_suffix("px")?.parse::<f32>().ok()
}

/// 从 CSSValue 提取长度值（px），用于 max-width/min-width 等
fn resolve_length_cs(value: &style::values::CSSValue) -> Option<f32> {
    match value {
        style::values::CSSValue::Length(val, _) => Some(*val),
        style::values::CSSValue::Keyword(k) => {
            if let Some(px) = k.strip_suffix("px") {
                px.parse::<f32>().ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 检查 CSSValue 是否为 auto 关键字
fn is_auto_cs(value: Option<&style::values::CSSValue>) -> bool {
    matches!(value, Some(style::values::CSSValue::Keyword(k)) if k == "auto")
}

/// 检查 margin 简写中某一边是否为 auto
/// index: 0=top, 1=right, 2=bottom, 3=left
fn is_auto_shorthand_cs(value: Option<&style::values::CSSValue>, index: usize) -> bool {
    let kw = match value {
        Some(style::values::CSSValue::Keyword(k)) => k.as_str(),
        _ => return false,
    };
    let parts: Vec<&str> = kw.split_whitespace().collect();
    match parts.len() {
        1 => parts[0] == "auto",
        2 => match index {
            0 | 2 => parts[0] == "auto",
            1 | 3 => parts[1] == "auto",
            _ => false,
        },
        3 => match index {
            0 => parts[0] == "auto",
            1 | 3 => parts[1] == "auto",
            2 => parts[2] == "auto",
            _ => false,
        },
        4 => parts.get(index).map_or(false, |&p| p == "auto"),
        _ => false,
    }
}

/// 布局模式 —— 决定使用哪种布局算法
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutMode {
    /// 普通流（Block + Inline）
    NormalFlow,
    /// Flexbox
    Flex,
    /// Grid (Phase 2+)
    // Grid,
    /// 绝对定位
    Absolute,
    /// 固定定位
    Fixed,
    // Phase 2+: Float
    // Phase 1+: Table
}

// Phase 1+: update_layout_tree, find_layout_node
