//! WebWindow —— 应用主入口
//!
//! 封装窗口创建、渲染循环、事件处理。
//! 整合 DOM、CSS、布局、渲染全管线。
//!
/// 使用方式：
/// ```rust,no_run
/// use renderer::WebWindow;
///
/// let mut window = WebWindow::new("My App", 800, 600);
/// let doc = window.document();
/// // ... 使用 DOM API 构建 UI ...
/// window.run(); // 启动事件循环
/// ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

use style::cascade::{compute_element_style, ComputedStyle};
use style::selector::SelectorEngine;
use style::stylesheet::StyleSheet;
use dom::event::MouseEvent;
use dom::node::{Node, NodeType};
use dom::Document;
use layout::build_layout_tree;
use layout::layout_box::LayoutBox;
use layout::LayoutEngine;
use render_tree::builder::DisplayListBuilder;
use render_tree::{DisplayList, PaintCommand};
use dom::Rect;
use crate::WgpuBackend;
use crate::RenderBackend;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{Key, NamedKey};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

use crate::event_loop::AnimationFrameScheduler;
use crate::hit_test::HitTester;

/// 递归统计布局树节点数（诊断用）
fn count_nodes(root: &LayoutBox) -> usize {
    1 + root.children.iter().map(count_nodes).sum::<usize>()
}

/// WebWindow —— 应用主入口
///
/// 封装窗口创建、渲染循环、事件处理。
pub struct WebWindow {
    /// 窗口标题
    title: String,
    /// 窗口初始尺寸
    size: (u32, u32),
    /// 事件循环
    event_loop: Option<EventLoop<()>>,
    /// DOM Document
    document: Rc<RefCell<Document>>,
    /// 布局引擎
    layout_engine: LayoutEngine,
    /// Phase 1: 样式表缓存
    stylesheets: Vec<StyleSheet>,
    /// Phase 1: 选择器引擎
    selector_engine: SelectorEngine,
    /// Phase 1: 动画帧调度器
    animation_scheduler: AnimationFrameScheduler,
}

impl WebWindow {
    /// 创建应用窗口
    ///
    /// title: 窗口标题
    /// width: 窗口宽度（CSS 像素）
    /// height: 窗口高度（CSS 像素）
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        let event_loop = EventLoop::new().unwrap();
        let doc = Document::new();

        Self {
            title: title.to_string(),
            size: (width, height),
            event_loop: Some(event_loop),
            document: doc,
            layout_engine: LayoutEngine::new(),
            stylesheets: Vec::new(),
            selector_engine: SelectorEngine::new(),
            animation_scheduler: AnimationFrameScheduler::new(),
        }
    }

    /// 获取 Document 对象（run() 之前调用）
    pub fn document(&self) -> Rc<RefCell<Document>> {
        self.document.clone()
    }

    /// Phase 1: 加载样式表
    pub fn load_stylesheet(&mut self, css: &str, url: &str) {
        let sheet = style::stylesheet::parse_stylesheet(css, url);
        self.stylesheets.push(sheet);
    }

    /// Phase 1: 获取动画帧调度器
    pub fn animation_scheduler(&mut self) -> &mut AnimationFrameScheduler {
        &mut self.animation_scheduler
    }

    /// Phase 1: 获取选择器引擎
    pub fn selector_engine(&self) -> &SelectorEngine {
        &self.selector_engine
    }

    /// 启动主事件循环（阻塞，直到窗口关闭）
    pub fn run(&mut self) {
        let event_loop = self.event_loop.take().expect("WebWindow already running");
        let size = self.size;

        // ================================================================
        // Phase 0 渲染管线：DOM → 计算样式 → 布局 → DisplayList → 渲染
        // ================================================================

        // 1. 从 DOM 树计算样式（内联 style 属性 + 继承）
        let doc = self.document.borrow();
        let body = doc.body();
        let dom_root = doc.document_element();

        let mut styles: HashMap<usize, ComputedStyle> = HashMap::new();
        compute_dom_styles(&dom_root, &self.stylesheets, None, &mut styles);
        drop(doc); // 释放 document 借用

        // 2. 构建布局树
        let mut layout_root = layout::build_layout_tree(&body, &styles, Some(&mut self.layout_engine.text_measurer));

        // 3. 执行布局计算
        let viewport = dom::Size::new(size.0 as f32, size.1 as f32);
        self.layout_engine.layout(&mut layout_root, viewport);

        // 4. 构建 DisplayList
        let mut builder = DisplayListBuilder::new();
        let display_list = builder.build(&layout_root);
        eprintln!("[diag] DisplayList commands: {}", display_list.commands().len());
        eprintln!("[diag] Layout tree node count: {}", count_nodes(&layout_root));
        eprintln!("[diag] Computed styles count: {}", styles.len());

        // 输出调试文件
        dump_render_debug(&dom_root, &body, &styles, &layout_root, &display_list, size);

        // 5. 启动窗口并渲染
        let layout_engine = std::mem::replace(&mut self.layout_engine, LayoutEngine::new());
        let animation_scheduler = std::mem::replace(
            &mut self.animation_scheduler,
            AnimationFrameScheduler::new(),
        );
        let mut app = App {
            title: self.title.clone(),
            size,
            viewport,
            window: None,
            renderer: None,
            display_list: Some(display_list),
            document: self.document.clone(),
            styles,
            layout_root: Some(layout_root),
            layout_engine,
            cursor_pos: (0.0, 0.0),
            animation_scheduler,
            focused_element: None,
            cursor_visible: true,
            last_cursor_toggle: 0.0,
        };

        let _ = event_loop.run_app(&mut app);
    }
}

/// 应用处理器 —— 实现 winit 0.30 ApplicationHandler
struct App {
    title: String,
    size: (u32, u32),
    viewport: dom::Size<f32>,
    window: Option<Window>,
    renderer: Option<RefCell<WgpuBackend>>,
    display_list: Option<DisplayList>,
    // Phase 0: 运行时状态（用于鼠标事件派发 + 重渲染）
    document: Rc<RefCell<Document>>,
    styles: HashMap<usize, ComputedStyle>,
    layout_root: Option<LayoutBox>,
    layout_engine: LayoutEngine,
    cursor_pos: (f32, f32),
    animation_scheduler: AnimationFrameScheduler,
    /// Phase 2: 当前获得焦点的元素（用于键盘输入）
    focused_element: Option<Rc<RefCell<Node>>>,
    /// 光标闪烁可见状态
    cursor_visible: bool,
    /// 上次光标切换时间（毫秒）
    last_cursor_toggle: f64,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title(&self.title)
                    .with_inner_size(LogicalSize::new(self.size.0, self.size.1)),
            )
            .unwrap();

        let renderer = pollster::block_on(WgpuBackend::new(&window));
        let renderer = RefCell::new(renderer);

        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(ref renderer) = self.renderer {
                    let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                    let mut r = renderer.borrow_mut();
                    r.resize_with_scale(size.width, size.height, scale);
                }
                // 重新布局以适应新窗口尺寸
                self.relayout();
            }
            WindowEvent::RedrawRequested => {
                if let Some(ref renderer) = self.renderer {
                    let mut r = renderer.borrow_mut();
                    let mut list = self.display_list.take().unwrap_or_default();
                    // 如果输入框获得焦点且光标可见，推入光标矩形
                    if self.cursor_visible {
                        if let Some(ref focused) = self.focused_element {
                            if let Some(layout_root) = self.layout_root.as_ref() {
                                if let Some(cursor_rect) = Self::find_layout_rect(layout_root, focused) {
                                    // 光标：2px 宽，使用蓝色可见
                                    list.push(PaintCommand::FillRect {
                                        rect: Rect::new(
                                            cursor_rect.x + 4.0,
                                            cursor_rect.y + 4.0,
                                            2.0,
                                            (cursor_rect.height - 8.0).max(10.0),
                                        ),
                                        color: dom::Color::rgb(74, 144, 217), // #4a90d9 蓝色光标
                                        radius: 0.0,
                                    });
                                }
                            }
                        }
                    }
                    r.render(&list);
                    // 移除光标命令以便下一帧重新计算
                    if self.cursor_visible && self.focused_element.is_some() {
                        list.pop();
                    }
                    self.display_list = Some(list);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                self.cursor_pos = (position.x as f32 / scale as f32, position.y as f32 / scale as f32);
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                self.handle_click();
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state: ElementState::Pressed,
                    logical_key,
                    text,
                    ..
                },
                ..
            } => {
                self.handle_keyboard(&logical_key, text.as_deref());
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = std::time::Instant::now()
            .elapsed()
            .as_secs_f64() * 1000.0;
        self.animation_scheduler.tick(now);

        // 光标闪烁：每 530ms 切换一次
        if now - self.last_cursor_toggle > 530.0 {
            self.cursor_visible = !self.cursor_visible;
            self.last_cursor_toggle = now;
        }

        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

impl App {
    /// 处理鼠标点击事件：Hit Test → 派发 DOM 事件 → 重建渲染管线
    fn handle_click(&mut self) {
        let (x, y) = self.cursor_pos;

        // 1. Hit test — 找到点击的布局节点
        let layout_root = match self.layout_root.as_ref() {
            Some(root) => root,
            None => return,
        };
        let hit = match HitTester::hit_test(layout_root, x, y) {
            Some(h) => h,
            None => return,
        };

        // 2. 获取对应的 DOM 节点，如果是文本节点则向上查找父元素
        let target_node = match hit.node.as_ref() {
            Some(n) => n.clone(),
            None => return,
        };
        let target_ref = target_node.borrow();
        let dom_node = match &target_ref.node_type {
            NodeType::Text(_) => {
                // 文本节点不能派发事件，向上查找到父元素
                target_ref.parent_node().unwrap_or_else(|| target_node.clone())
            }
            _ => target_node.clone(),
        };
        drop(target_ref);

        // 3. 焦点管理：点击 input 元素时设置焦点
        {
            let node_ref = dom_node.borrow();
            if let NodeType::Element(elem) = &node_ref.node_type {
                if elem.tag_name() == "input" {
                    self.focused_element = Some(dom_node.clone());
                } else {
                    self.focused_element = None;
                }
            } else {
                self.focused_element = None;
            }
        }

        // 4. 创建并派发鼠标点击事件
        let mouse_event = MouseEvent::new("click", x as f64, y as f64, 0);
        dom_node.borrow_mut().dispatch_event(&mouse_event.event);

        // 5. 重建渲染管线（事件回调可能修改了 DOM）
        self.relayout();
    }

    /// 处理键盘输入：将文本追加/删除到聚焦的 input 元素
    fn handle_keyboard(&mut self, key: &Key, text: Option<&str>) {
        let focused = match &self.focused_element {
            Some(el) => el.clone(),
            None => return,
        };

        let mut node = focused.borrow_mut();

        match key {
            Key::Named(NamedKey::Backspace) => {
                // 删除最后一个字符
                let current = node.text_content();
                if !current.is_empty() {
                    let mut chars: Vec<char> = current.chars().collect();
                    chars.pop();
                    let new_text: String = chars.into_iter().collect();
                    // 删除旧的文本子节点，创建新的
                    node.set_text_content(&new_text);
                }
            }
            Key::Named(NamedKey::Enter) => {
                // Enter 键：可以触发提交逻辑，目前不做特殊处理
            }
            _ => {
                // 普通字符输入
                if let Some(t) = text {
                    if !t.is_empty() && !t.chars().any(|c| c.is_control()) {
                        let current = node.text_content();
                        let new_text = format!("{}{}", current, t);
                        node.set_text_content(&new_text);
                    }
                }
            }
        }
        drop(node);

        // 重建渲染管线以反映文本变更
        self.relayout();
    }

    /// 在布局树中查找指定 DOM 节点的布局矩形
    fn find_layout_rect(root: &LayoutBox, target: &Rc<RefCell<Node>>) -> Option<Rect<f32>> {
        if let Some(ref node) = root.node {
            if Rc::ptr_eq(node, target) {
                return Some(root.rect);
            }
        }
        for child in &root.children {
            if let Some(rect) = Self::find_layout_rect(child, target) {
                return Some(rect);
            }
        }
        None
    }

    /// 重建布局和 DisplayList（窗口 resize 或 DOM 变更后调用）
    fn relayout(&mut self) {
        let size = if let Some(ref window) = self.window {
            let inner = window.inner_size();
            let scale = window.scale_factor();
            let w = (inner.width as f64 / scale).max(1.0) as u32;
            let h = (inner.height as f64 / scale).max(1.0) as u32;
            (w, h)
        } else {
            self.size
        };

        let viewport = dom::Size::new(size.0 as f32, size.1 as f32);
        self.size = size;
        self.viewport = viewport;

        let doc_ref = self.document.borrow();
        let dom_root = doc_ref.document_element();
        let body = doc_ref.body();

        // 重新计算样式（DOM 可能已变更，文本节点被替换为新 Rc 指针）
        let mut styles: HashMap<usize, ComputedStyle> = HashMap::new();
        compute_dom_styles(&dom_root, &[], None, &mut styles);

        let mut new_root = build_layout_tree(&body, &styles, Some(&mut self.layout_engine.text_measurer));
        self.layout_engine.layout(&mut new_root, viewport);
        drop(doc_ref);

        let mut builder = DisplayListBuilder::new();
        let new_dl = builder.build(&new_root);

        self.styles = styles;
        self.layout_root = Some(new_root);
        self.display_list = Some(new_dl);
    }
}

/// 递归遍历 DOM 树，为每个元素计算最终样式
///
/// 处理内联 style 属性，并实现父→子样式继承。
fn compute_dom_styles(
    node: &Rc<RefCell<Node>>,
    stylesheets: &[StyleSheet],
    parent_style: Option<&ComputedStyle>,
    out: &mut HashMap<usize, ComputedStyle>,
) {
    let node_ref = node.borrow();

    match &node_ref.node_type {
        NodeType::Element(elem_data) => {
            // 解析内联 style 属性
            let inline_style = elem_data
                .get_attribute("style")
                .map(|s| style::parse_inline_style(&s))
                .unwrap_or_default();

            // 计算此元素的最终样式
            let ptr = Rc::as_ptr(node) as usize;
            let computed = compute_element_style(elem_data, parent_style, stylesheets, &inline_style);
            out.insert(ptr, computed.clone());

            // 递归处理子节点（当前样式作为父样式）
            let children = node_ref.child_nodes();
            drop(node_ref); // 释放借用，子节点处理需要 Node 的完全访问权
            for child in &children {
                compute_dom_styles(child, stylesheets, Some(&computed), out);
            }
        }
        NodeType::Text(_) => {
            // 文本节点仅继承可继承属性（color, font-size, font-family, text-align）
            // 不继承 background, margin, padding, border 等盒模型属性
            if let Some(ps) = parent_style {
                let ptr = Rc::as_ptr(node) as usize;
                let mut inherited = ComputedStyle::new();
                for prop in &["color", "font-size", "font-family", "font-weight", "font-style", "text-align"] {
                    if let Some(val) = ps.properties.get(*prop) {
                        inherited.properties.insert(prop.to_string(), val.clone());
                    }
                }
                out.insert(ptr, inherited);
            }
        }
        NodeType::Document | NodeType::DocumentFragment | NodeType::Comment(_) => {
            let children = node_ref.child_nodes();
            drop(node_ref);
            for child in &children {
                compute_dom_styles(child, stylesheets, parent_style, out);
            }
        }
    }
}

/// 输出渲染管线调试信息到文件
fn dump_render_debug(
    dom_root: &Rc<RefCell<Node>>,
    body: &Rc<RefCell<Node>>,
    styles: &HashMap<usize, ComputedStyle>,
    layout_root: &LayoutBox,
    display_list: &DisplayList,
    window_size: (u32, u32),
) {
    let mut out = String::new();
    let _ = writeln!(out, "========== 渲染管线调试输出 ==========");
    let _ = writeln!(out, "窗口尺寸: {}x{}", window_size.0, window_size.1);

    // === DOM 树 ===
    let _ = writeln!(out, "\n========== DOM 树 ==========");
    dump_dom_node(dom_root, 0, &mut out);

    // === body 子树 ===
    let _ = writeln!(out, "\n========== body 子树 ==========");
    dump_dom_node(body, 0, &mut out);

    // === 各节点计算样式 ===
    let _ = writeln!(out, "\n========== 计算样式 (ComputedStyle) ==========");
    for (ptr, style) in styles {
        let _ = writeln!(out, "  [0x{:x}]", ptr);
        for (prop, val) in &style.properties {
            let _ = writeln!(out, "    {}: {:?}", prop, val);
        }
    }

    // === 布局树 ===
    let _ = writeln!(out, "\n========== 布局树 (LayoutBox) ==========");
    dump_layout_node(layout_root, 0, &mut out);

    // === DisplayList ===
    let _ = writeln!(out, "\n========== DisplayList ({} 条命令) ==========", display_list.commands().len());
    for (i, cmd) in display_list.commands().iter().enumerate() {
        let _ = writeln!(out, "  [{}] {:?}", i, cmd);
    }

    let path = std::path::Path::new("target").join("render_tree_debug.txt");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, &out) {
        eprintln!("[diag] 写入调试文件失败: {}\n{}", e, out);
    } else {
        eprintln!("[diag] 调试文件已写入: {}", path.display());
    }
}

/// 递归输出 DOM 节点
fn dump_dom_node(node: &Rc<RefCell<Node>>, depth: usize, out: &mut String) {
    let n = node.borrow();
    let indent = "  ".repeat(depth);
    match &n.node_type {
        NodeType::Element(elem) => {
            let _ = writeln!(out, "{}<{}> id={:?} class={:?} style={:?}",
                indent, elem.tag_name(),
                elem.get_attribute("id"),
                elem.get_attribute("class"),
                elem.get_attribute("style"));
        }
        NodeType::Text(_) => {
            let _ = writeln!(out, "{}\"{}\"", indent, n.text_content());
        }
        NodeType::Document => {
            let _ = writeln!(out, "{}[Document]", indent);
        }
        NodeType::DocumentFragment => {
            let _ = writeln!(out, "{}[DocumentFragment]", indent);
        }
        NodeType::Comment(s) => {
            let _ = writeln!(out, "{}<!-- {} -->", indent, s);
        }
    }
    let children = n.child_nodes();
    drop(n);
    for child in &children {
        dump_dom_node(child, depth + 1, out);
    }
}

/// 递归输出布局节点
fn dump_layout_node(node: &LayoutBox, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    let _ = writeln!(out, "{}{:?} rect=({:.0}, {:.0}, {:.0}x{:.0}) pad=({:.0},{:.0},{:.0},{:.0}) margin=({:.0},{:.0},{:.0},{:.0})",
        indent,
        node.box_type,
        node.rect.x, node.rect.y, node.rect.width, node.rect.height,
        node.padding.top, node.padding.right, node.padding.bottom, node.padding.left,
        node.margin.top, node.margin.right, node.margin.bottom, node.margin.left,
    );
    if let Some(ref dom_node) = node.node {
        if let NodeType::Text(_) = &dom_node.borrow().node_type {
            let _ = writeln!(out, "{}  text=\"{}\"", indent, dom_node.borrow().text_content());
        }
    }
    if let Some(ref cs) = node.computed_style {
        if let Some(fs) = cs.get("font-size") {
            let _ = writeln!(out, "{}  font-size={:?}", indent, fs);
        }
        if let Some(c) = cs.get("color") {
            let _ = writeln!(out, "{}  color={:?}", indent, c);
        }
        if let Some(bg) = cs.get("background") {
            let _ = writeln!(out, "{}  background={:?}", indent, bg);
        }
    }
    for child in &node.children {
        dump_layout_node(child, depth + 1, out);
    }
}
