# Rust 浏览器引擎 — Phase 2 API 设计文档

> 本文档定义 Phase 2 所有公开类型、函数签名及作用。
> 中文注解标注功能说明，`// Phase 3+` 标记为后续阶段预留的扩展点。
> Phase 2 在 Phase 1 基础上新增约 21,500 行代码，CSS 属性从 80 扩展到 200+ 个，
> 新增 net/storage 两个 crate，实现完整动画/过渡引擎、Grid/Float 布局、JS 异步编译。

---

## 一、模块总览

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Phase 2 模块依赖关系                                                      │
│                                                                           │
│  examples/todo_app                                                        │
│  examples/counter              examples/dashboard                         │
│       │                                                                   │
│       ▼                                                                   │
│  ┌─────────────────┐                                                      │
│  │    web2rust      │  ← 编译器（构建期）                                   │
│  │  ┌─────────────┐ │     Promise/async/Class 完整编译                      │
│  │  │ parser.rs   │ │     Proxy/Reflect/TypedArray 支持                    │
│  │  │ analyzer.rs │ │                                                      │
│  │  │ codegen.rs  │ │                                                      │
│  │  │ builtins.rs │ │                                                      │
│  │  └─────────────┘ │                                                      │
│  └────────┬────────┘                                                      │
│           │  生成 main() 函数体                                            │
│           ▼                                                               │
│  runtime (主循环 + 动画帧 + 滚动 + 触摸)                                    │
│       │                                                                   │
│       ├── net (全新) ─── fetch API + WebSocket 基础                        │
│       │                                                                   │
│       ├── storage (全新) ─── localStorage / sessionStorage                 │
│       │                                                                   │
│       ├── render (wgpu 渲染后端 + 变换/滤镜/渐变管线)                        │
│       │       │                                                            │
│       │       ▼                                                            │
│       │   paint (DisplayList + Transform/Filter + 渐变)                    │
│       │                                                                   │
│       ├── layout (布局引擎 + Grid/Float)                                    │
│       │       │                                                            │
│       │       ├── css (CSS 引擎 + 动画/过渡/媒体查询/自定义属性)              │
│       │       │       │                                                    │
│       │       │       ▼                                                    │
│       │       └── dom (DOM 树 + HTML 元素特化)                              │
│       │                                                                   │
│       └── dom (DOM 树 - 同上引用)                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Phase 2 实现原则

1. **功能完善**：CSS 属性覆盖 200+，动画/过渡/变换/滤镜全部实现
2. **网络+存储**：新增 net 和 storage 两个独立 crate，完善 Web API 能力
3. **异步支持**：Promise/async/await 编译为 Rust Future，开启异步编程
4. **HTML 特化**：Canvas/Input/Image/Anchor 等元素拥有专用 API
5. **预留扩展点**：用 `// Phase 3+` 标记后续阶段位置
6. **W3C 命名**：函数名与浏览器标准 DOM API 保持一致
7. **中文注解**：所有文档注释使用中文

---

## 二、dom crate — Phase 2 扩展

### 2.1 模块结构

```
dom/
├── Cargo.toml
├── src/
│   ├── lib.rs                  # 公开导出更新
│   ├── node.rs                 # document_position 常量 + 扩展
│   ├── element.rs              # scroll 系列方法
│   ├── document.rs             # create_element_ns / import_node / adopt_node
│   ├── text.rs                 # (Phase 1 已完成)
│   ├── event.rs                # AnimationEvent / TransitionEvent / InputEvent
│   ├── dom_token_list.rs       # (Phase 1 已完成)
│   ├── mutation_observer.rs    # 批量异步回调（微任务队列）
│   └── html/                   # 新目录：HTML 元素特化
│       ├── mod.rs              # 模块导出
│       ├── html_anchor_element.rs
│       ├── html_image_element.rs
│       ├── html_input_element.rs
│       ├── html_canvas_element.rs
│       ├── html_form_element.rs
│       ├── html_select_element.rs
│       └── html_text_area_element.rs
```

### 2.2 Node — document_position 常量

```rust
// === crates/dom/src/node.rs ===

/// W3C DocumentPosition 位掩码常量（Phase 2 完善）
pub mod document_position {
    /// 两个节点在不同文档中
    pub const DOCUMENT_POSITION_DISCONNECTED: u16 = 1;
    /// other 在 this 之前（文档顺序）
    pub const DOCUMENT_POSITION_PRECEDING: u16 = 2;
    /// other 在 this 之后（文档顺序）
    pub const DOCUMENT_POSITION_FOLLOWING: u16 = 4;
    /// other 是 this 的后代
    pub const DOCUMENT_POSITION_CONTAINS: u16 = 8;
    /// other 是 this 的祖先
    pub const DOCUMENT_POSITION_CONTAINED_BY: u16 = 16;
    /// 实现决定的结果（需额外处理）
    pub const DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC: u16 = 32;
}

impl Node {
    // Phase 2 新增

    /// 判断元素是否支持指定特性（feature + version）
    /// 例: node.is_supported("Events", "2.0") → true
    pub fn is_supported(&self, feature: &str, version: &str) -> bool;

    /// 查找指定命名空间的前缀
    pub fn lookup_prefix(&self, namespace: &str) -> Option<String>;

    /// 查找指定前缀的命名空间 URI
    pub fn lookup_namespace_uri(&self, prefix: &str) -> Option<String>;
}
```

### 2.3 ElementData — scroll 系列方法

```rust
// === crates/dom/src/element.rs ===

impl ElementData {
    // Phase 2 新增

    /// 将元素滚动到可视区域内
    /// options: ScrollIntoViewOptions { behavior: "smooth"/"auto", block: "start"/"center"/"end"/"nearest" }
    pub fn scroll_into_view(&mut self, options: Option<ScrollIntoViewOptions>);

    /// 绝对滚动到指定坐标
    pub fn scroll_to(&mut self, x: f32, y: f32);

    /// 相对滚动指定偏移量
    pub fn scroll_by(&mut self, x: f32, y: f32);

    /// 获取元素的多个边界矩形
    /// 用于 inline 元素跨行场景
    pub fn get_client_rects(&self) -> Vec<Rect<f32>>;

    /// 滚动位置（只读）
    pub fn scroll_top(&self) -> f32;
    pub fn scroll_left(&self) -> f32;

    /// 完整 HTML 序列化（含属性转义、自闭合标签）
    /// Phase 1 为 stub，Phase 2 完整实现
    // pub fn inner_html(&self) -> String;       // 完善实现
    // pub fn set_inner_html(&mut self, html: &str);  // 使用 html5ever 解析片段
}

/// 滚动行为选项
#[derive(Debug, Clone)]
pub struct ScrollIntoViewOptions {
    /// 滚动行为
    pub behavior: ScrollBehavior,
    /// 垂直对齐
    pub block: ScrollLogicalPosition,
    /// 水平对齐
    pub inline: ScrollLogicalPosition,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollBehavior {
    Auto,
    Smooth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollLogicalPosition {
    Start,
    Center,
    End,
    Nearest,
}
```

### 2.4 Document — create_element_ns / import_node / adopt_node

```rust
// === crates/dom/src/document.rs ===

impl Document {
    // Phase 2 新增

    /// 创建带命名空间的元素
    /// 例: create_element_ns("http://www.w3.org/2000/svg", "svg")
    pub fn create_element_ns(
        &self,
        namespace: &str,
        qualified_name: &str,
    ) -> Rc<RefCell<Node>>;

    /// 从外部文档导入节点（deep=true 递归导入子树）
    /// 导入的节点不自动挂载，需手动 append_child
    pub fn import_node(
        &self,
        node: &Node,
        deep: bool,
    ) -> Rc<RefCell<Node>>;

    /// 将外部文档节点收养到当前文档
    /// 移除原文档中的节点并转移到当前文档
    pub fn adopt_node(&self, node: Rc<RefCell<Node>>);

    // Phase 3+:
    // pub fn create_node_iterator(&self, root: &Node, what_to_show: u32) -> NodeIterator;
    // pub fn create_tree_walker(&self, root: &Node, what_to_show: u32) -> TreeWalker;
    // pub fn create_range(&self) -> Range;
}
```

### 2.5 Event — 新增动画/过渡/输入事件

```rust
// === crates/dom/src/event.rs ===

// ============================================================
//  AnimationEvent —— 动画事件 (Phase 2 新增)
// ============================================================

/// CSS 动画事件 —— 对应 W3C AnimationEvent 接口
pub struct AnimationEvent {
    pub event: Event,
    /// 触发事件的动画名称
    pub animation_name: String,
    /// 动画已运行时间（秒）
    pub elapsed_time: f32,
    /// 动画伪元素选择器
    pub pseudo_element: String,
}

impl AnimationEvent {
    /// 创建动画事件
    /// event_type: "animationstart" / "animationend" / "animationiteration"
    pub fn new(event_type: &str, animation_name: &str, elapsed_time: f32) -> Self;
}

// ============================================================
//  TransitionEvent —— 过渡事件 (Phase 2 新增)
// ============================================================

/// CSS 过渡事件 —— 对应 W3C TransitionEvent 接口
pub struct TransitionEvent {
    pub event: Event,
    /// 触发过渡的 CSS 属性名
    pub property_name: String,
    /// 过渡已运行时间（秒）
    pub elapsed_time: f32,
    /// 过渡伪元素选择器
    pub pseudo_element: String,
}

impl TransitionEvent {
    /// 创建过渡事件
    /// event_type: "transitionstart" / "transitionend" / "transitioncancel" / "transitionrun"
    pub fn new(event_type: &str, property_name: &str, elapsed_time: f32) -> Self;
}

// ============================================================
//  InputEvent —— 输入事件 (Phase 2 新增)
// ============================================================

/// 输入事件 —— 对应 W3C InputEvent 接口
pub struct InputEvent {
    pub event: Event,
    /// 插入/删除的文本
    pub data: Option<String>,
    /// 输入类型: "insertText", "insertFromPaste", "deleteContentBackward", 等
    pub input_type: String,
    /// 是否为 IME 组合中的中间状态
    pub is_composing: bool,
}

impl InputEvent {
    pub fn new(event_type: &str, input_type: &str, data: Option<&str>) -> Self;
}

// ============================================================
//  Event 扩展字段 (Phase 2)
// ============================================================

impl Event {
    /// 返回事件穿透 Shadow DOM 的完整路径
    // Phase 3+: Shadow DOM 支持后实现
    // pub fn composed_path(&self) -> Vec<Rc<RefCell<Node>>>;
}

impl KeyboardEvent {
    // Phase 2: IME 组合状态
    /// 是否在 IME 组合中
    pub is_composing: bool,
}

// Phase 3+:
// pub struct TouchEvent { ... }    // 触摸事件
// pub struct PointerEvent { ... }  // 指针事件（W3C Pointer Events）
```

### 2.6 MutationObserver — 批量异步回调

```rust
// === crates/dom/src/mutation_observer.rs ===

impl MutationObserver {
    // Phase 2 新增

    /// 将回调排入微任务队列
    /// Phase 1 为同步触发，Phase 2 改为批量异步：
    /// 1. 同一帧内的多次 DOM 变更累积到 pending_records
    /// 2. 微任务队列在帧末尾统一触发回调
    /// 3. take_records 清空待处理记录
    pub(crate) fn queue_microtask(&self);
}

// ============================================================
//  微任务队列 (Phase 2 新增)
// ============================================================

/// 微任务队列 —— 在每帧渲染前执行
///
/// 队列顺序（W3C 标准）：
/// 1. MutationObserver 回调
/// 2. Promise.then/catch/finally 回调
/// 3. queueMicrotask 回调
pub(crate) struct MicrotaskQueue {
    /// 待处理任务
    tasks: Vec<Microtask>,
}

enum Microtask {
    MutationObserver(Box<dyn FnOnce()>),
    PromiseThen(Box<dyn FnOnce()>),
    QueueMicrotask(Box<dyn FnOnce()>),
}

impl MicrotaskQueue {
    pub fn new() -> Self;
    pub fn enqueue(&mut self, task: Microtask);
    pub fn flush(&mut self);
    pub fn is_empty(&self) -> bool;
}
```

### 2.7 html/ — HTML 元素特化（全新子模块）

```rust
// === crates/dom/src/html/html_anchor_element.rs ===

/// HTMLAnchorElement —— <a href="...">
pub struct HTMLAnchorElement {
    pub element: ElementData,

    // ===== W3C 属性 =====
    pub href: String,
    pub target: String,
    pub download: String,
    pub rel: String,
    pub hreflang: String,
    pub media_type: String, // type 是 Rust 关键字
    pub text: String,       // textContent 快捷访问
}

impl HTMLAnchorElement {
    /// 从 Element 节点包装
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;

    /// 获取完整解析后的 URL
    pub fn origin(&self) -> String;
    pub fn protocol(&self) -> String;
    pub fn host(&self) -> String;
    pub fn hostname(&self) -> String;
    pub fn port(&self) -> String;
    pub fn pathname(&self) -> String;
    pub fn search(&self) -> String;
    pub fn hash(&self) -> String;
}
```

```rust
// === crates/dom/src/html/html_image_element.rs ===

/// HTMLImageElement —— <img src="..." alt="...">
pub struct HTMLImageElement {
    pub element: ElementData,

    pub src: String,
    pub alt: String,
    pub width: u32,
    pub height: u32,
    pub natural_width: u32,   // 图像原始宽度（加载后填充）
    pub natural_height: u32,  // 图像原始高度（加载后填充）
    pub complete: bool,       // 图像是否加载完成
    pub cross_origin: String,
}

impl HTMLImageElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;

    /// 解码图像（异步加载）
    /// Phase 2: 使用 image crate 解码 → GPU 纹理
    pub fn decode(&mut self) -> Result<(), ImageLoadError>;
}

pub enum ImageLoadError {
    NetworkError(String),
    DecodeError(String),
    InvalidSource,
}
```

```rust
// === crates/dom/src/html/html_input_element.rs ===

/// HTMLInputElement —— <input type="text" value="...">
pub struct HTMLInputElement {
    pub element: ElementData,

    pub input_type: String,      // "text", "checkbox", "radio", "number", "password", etc.
    pub value: String,
    pub placeholder: String,
    pub disabled: bool,
    pub read_only: bool,
    pub required: bool,
    pub checked: bool,           // checkbox/radio
    pub name: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub max_length: i32,
    pub pattern: String,
    pub auto_focus: bool,
}

impl HTMLInputElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;

    /// 选中输入内容
    pub fn select(&mut self);

    /// 步进值（根据 step 属性）
    pub fn step_up(&mut self, n: i32);
    pub fn step_down(&mut self, n: i32);

    /// 表单验证
    pub fn check_validity(&self) -> bool;
    pub fn validation_message(&self) -> String;

    /// 报告验证错误
    pub fn report_validity(&self) -> bool;
}
```

```rust
// === crates/dom/src/html/html_canvas_element.rs ===

/// HTMLCanvasElement —— <canvas width="300" height="150">
pub struct HTMLCanvasElement {
    pub element: ElementData,

    pub width: u32,
    pub height: u32,
}

impl HTMLCanvasElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;

    /// 获取 2D 渲染上下文
    /// Phase 2: 基础 2D API（fillRect, strokeRect, fillText, drawImage）
    /// Phase 3+: 完整 Canvas 2D 规范
    pub fn get_context_2d(&mut self) -> CanvasRenderingContext2D;

    /// 获取 WebGL 上下文 (Phase 3+)
    // pub fn get_context_webgl(&mut self) -> WebGLRenderingContext;

    /// 导出为 PNG 数据（Phase 3+）
    // pub fn to_data_url(&self, mime_type: &str) -> String;

    /// 导出为 ImageData（Phase 3+）
    // pub fn to_blob(&self) -> Vec<u8>;
}

/// Canvas 2D 渲染上下文 —— Phase 2 基础版
pub struct CanvasRenderingContext2D {
    /// 像素缓冲区
    pixels: Vec<u8>,
    width: u32,
    height: u32,

    // 状态
    fill_style: CanvasStyle,
    stroke_style: CanvasStyle,
    line_width: f32,
    font: String,
    text_align: CanvasTextAlign,
    global_alpha: f32,
}

impl CanvasRenderingContext2D {
    // ===== 矩形 =====
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32);
    pub fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32);
    pub fn clear_rect(&mut self, x: f32, y: f32, w: f32, h: f32);

    // ===== 文本 =====
    pub fn fill_text(&mut self, text: &str, x: f32, y: f32);
    pub fn stroke_text(&mut self, text: &str, x: f32, y: f32);

    // ===== 图像 =====
    pub fn draw_image(&mut self, image: &HTMLImageElement, dx: f32, dy: f32);
    pub fn draw_image_scaled(&mut self, image: &HTMLImageElement, dx: f32, dy: f32, dw: f32, dh: f32);

    // ===== 路径 (Phase 3+) =====
    // pub fn begin_path(&mut self);
    // pub fn move_to(&mut self, x: f32, y: f32);
    // pub fn line_to(&mut self, x: f32, y: f32);
    // pub fn arc(&mut self, x: f32, y: f32, r: f32, start: f32, end: f32);
    // pub fn fill(&mut self);
    // pub fn stroke(&mut self);

    // ===== 变换 (Phase 3+) =====
    // pub fn save(&mut self);
    // pub fn restore(&mut self);
    // pub fn translate(&mut self, x: f32, y: f32);
    // pub fn scale(&mut self, x: f32, y: f32);
    // pub fn rotate(&mut self, angle: f32);

    // ===== 像素操作 =====
    pub fn get_image_data(&self, x: u32, y: u32, w: u32, h: u32) -> Vec<u8>;
    pub fn put_image_data(&mut self, data: &[u8], x: u32, y: u32, w: u32, h: u32);
}

pub enum CanvasStyle {
    Color(Color),
    Gradient(CanvasGradient),
    // Phase 3+: Pattern(CanvasPattern),
}

pub enum CanvasTextAlign {
    Left,
    Center,
    Right,
    Start,
    End,
}

/// Canvas 渐变 (Phase 2 基础)
pub struct CanvasGradient {
    pub gradient_type: CanvasGradientType,
    pub stops: Vec<(f32, Color)>,
}

pub enum CanvasGradientType {
    Linear { x0: f32, y0: f32, x1: f32, y1: f32 },
    Radial { x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32 },
}
```

```rust
// === crates/dom/src/html/html_form_element.rs ===

/// HTMLFormElement —— <form action="..." method="...">
pub struct HTMLFormElement {
    pub element: ElementData,

    pub action: String,
    pub method: String,
    pub enctype: String,
}

impl HTMLFormElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;

    /// 提交表单（触发 submit 事件）
    /// Phase 2: 收集表单数据 → fetch POST
    pub fn submit(&mut self);

    /// 重置表单
    pub fn reset(&mut self);

    /// 返回所有表单控件
    pub fn elements(&self) -> Vec<Rc<RefCell<Node>>>;
}
```

```rust
// === crates/dom/src/html/html_select_element.rs ===

/// HTMLSelectElement —— <select>
pub struct HTMLSelectElement {
    pub element: ElementData,

    pub value: String,
    pub selected_index: i32,
    pub multiple: bool,
    pub disabled: bool,
    pub required: bool,
    pub size: u32,
}

impl HTMLSelectElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;

    /// 添加 <option>
    pub fn add(&mut self, option: Rc<RefCell<Node>>, before: Option<Rc<RefCell<Node>>>);

    /// 移除指定索引的 <option>
    pub fn remove(&mut self, index: i32);

    /// 获取所有 <option> 节点
    pub fn options(&self) -> Vec<Rc<RefCell<Node>>>;
}
```

```rust
// === crates/dom/src/html/html_text_area_element.rs ===

/// HTMLTextAreaElement —— <textarea>
pub struct HTMLTextAreaElement {
    pub element: ElementData,

    pub value: String,
    pub placeholder: String,
    pub rows: u32,
    pub cols: u32,
    pub disabled: bool,
    pub read_only: bool,
    pub required: bool,
    pub max_length: i32,
}

impl HTMLTextAreaElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;
    pub fn select(&mut self);
}
```

### 2.8 dom 公开导出更新

```rust
// === crates/dom/src/lib.rs ===

// Phase 0/1 已有导出
// Node, NodeType, ElementData, Document, Text, Event, MouseEvent,
// KeyboardEvent, FocusEvent, WheelEvent, DOMTokenList,
// MutationObserver, MutationRecord, ...

// Phase 2 新增导出
pub mod document_position;

pub use event::{
    AnimationEvent,        // Phase 2 新增
    TransitionEvent,       // Phase 2 新增
    InputEvent,            // Phase 2 新增
};

pub use element::{
    ScrollIntoViewOptions,
    ScrollBehavior,
    ScrollLogicalPosition,
};

// HTML 元素特化
pub use html::html_anchor_element::HTMLAnchorElement;
pub use html::html_image_element::{HTMLImageElement, ImageLoadError};
pub use html::html_input_element::HTMLInputElement;
pub use html::html_canvas_element::{
    HTMLCanvasElement, CanvasRenderingContext2D,
    CanvasStyle, CanvasGradient, CanvasGradientType, CanvasTextAlign,
};
pub use html::html_form_element::HTMLFormElement;
pub use html::html_select_element::HTMLSelectElement;
pub use html::html_text_area_element::HTMLTextAreaElement;

// Phase 3+:
// pub use html::html_media_element::{HTMLVideoElement, HTMLAudioElement};
// pub use html::html_iframe_element::HTMLIFrameElement;
// pub use html::html_source_element::HTMLSourceElement;
```

---

## 三、css crate — CSS 引擎 200+ 属性扩展

### 3.1 模块结构

```
css/
├── Cargo.toml
├── properties.toml          # 从 80 扩展到 ~200 属性
└── src/
    ├── lib.rs               # 公开导出更新
    ├── stylesheet.rs        # (Phase 1 已完成)
    ├── selector.rs          # 完整伪类支持 (:hover, :nth-child, :not(), 等)
    ├── cascade.rs           # 级联缓存完善 + revert 关键字 + 用户样式层
    ├── values.rs            # Transform 3D, Filter, BlendMode 等新类型
    ├── animations.rs        # 新文件：CSS 动画引擎
    ├── transitions.rs       # 新文件：CSS 过渡引擎
    ├── media.rs             # 新文件：媒体查询求值引擎
    └── custom_props.rs      # 新文件：CSS 自定义属性（--*）
```

### 3.2 properties.toml — 新增约 120 个属性

```toml
# === crates/css/properties.toml ===
# Phase 2 CSS 属性定义（200+ 个属性）
# Phase 3+ 属性预留见末尾

# ===== Phase 0/1 已有（80 个）=====
# width, height, margin-*, padding-*, border-*, box-sizing
# display, overflow, overflow-x, overflow-y, visibility, z-index
# flex-direction, flex-wrap, justify-content, align-items, align-content,
# gap, flex-grow, flex-shrink, flex-basis, flex, flex-flow, order, align-self
# position, top, right, bottom, left
# color, font-size, font-weight, font-family, font-style, line-height,
# text-align, text-decoration, text-decoration-color, text-decoration-style,
# white-space, word-break, letter-spacing, word-spacing, text-transform, text-indent
# background-color, background-image, background-size, background-position,
# background-repeat, background-attachment, background, opacity,
# border-radius, border-top-left-radius, etc.
# outline-width, outline-style, outline-color, outline
# list-style-type, list-style-position, list-style-image, list-style
# cursor, pointer-events, user-select
# table-layout, border-collapse, border-spacing
# min-width, min-height, max-width, max-height

# ============================================================
# Phase 2 新增：动画（animation-*）
# ============================================================

[animation-name]
initial = "none"
inherited = false

[animation-duration]
initial = "0s"
inherited = false

[animation-timing-function]
initial = "ease"
inherited = false
# values: ease, ease-in, ease-out, ease-in-out, linear,
#         step-start, step-end, steps(n, start|end), cubic-bezier(...)

[animation-delay]
initial = "0s"
inherited = false

[animation-iteration-count]
initial = "1"
inherited = false
# values: <number>, infinite

[animation-direction]
initial = "normal"
inherited = false
# values: normal, reverse, alternate, alternate-reverse

[animation-fill-mode]
initial = "none"
inherited = false
# values: none, forwards, backwards, both

[animation-play-state]
initial = "running"
inherited = false
# values: running, paused

[animation]
initial = "none 0s ease 0s 1 normal none running"
inherited = false
# shorthand

# ============================================================
# Phase 2 新增：过渡（transition-*）
# ============================================================

[transition-property]
initial = "all"
inherited = false

[transition-duration]
initial = "0s"
inherited = false

[transition-timing-function]
initial = "ease"
inherited = false

[transition-delay]
initial = "0s"
inherited = false

[transition]
initial = "all 0s ease 0s"
inherited = false
# shorthand

# ============================================================
# Phase 2 新增：2D/3D 变换（transform-*）
# ============================================================

[transform]
initial = "none"
inherited = false

[transform-origin]
initial = "50% 50% 0"
inherited = false

[transform-style]
initial = "flat"
inherited = false
# values: flat, preserve-3d

[perspective]
initial = "none"
inherited = false

[perspective-origin]
initial = "50% 50%"
inherited = false

[backface-visibility]
initial = "visible"
inherited = false
# values: visible, hidden

# ============================================================
# Phase 2 新增：Grid 布局（grid-*）
# ============================================================

[grid-template-columns]
initial = "none"
inherited = false

[grid-template-rows]
initial = "none"
inherited = false

[grid-template-areas]
initial = "none"
inherited = false

[grid-template]
initial = "none"
inherited = false
# shorthand

[grid-column-gap]
initial = "normal"
inherited = false

[grid-row-gap]
initial = "normal"
inherited = false

[grid-auto-columns]
initial = "auto"
inherited = false

[grid-auto-rows]
initial = "auto"
inherited = false

[grid-auto-flow]
initial = "row"
inherited = false
# values: row, column, row dense, column dense

[grid-column-start]
initial = "auto"
inherited = false

[grid-column-end]
initial = "auto"
inherited = false

[grid-row-start]
initial = "auto"
inherited = false

[grid-row-end]
initial = "auto"
inherited = false

[grid-column]
initial = "auto / auto"
inherited = false
# shorthand

[grid-row]
initial = "auto / auto"
inherited = false
# shorthand

[grid-area]
initial = "auto"
inherited = false
# shorthand

[grid-gap]
initial = "normal normal"
inherited = false
# shorthand (deprecated 但仍需支持)

# ============================================================
# Phase 2 新增：浮动（float + clear）
# ============================================================

[float]
initial = "none"
inherited = false
# values: none, left, right
# Phase 3+: inline-start, inline-end

[clear]
initial = "none"
inherited = false
# values: none, left, right, both

# ============================================================
# Phase 2 新增：阴影
# ============================================================

[box-shadow]
initial = "none"
inherited = false
# values: none, <inset>? <offset-x> <offset-y> <blur-radius>? <spread-radius>? <color>?

[text-shadow]
initial = "none"
inherited = true
# values: none, <offset-x> <offset-y> <blur-radius>? <color>?

# ============================================================
# Phase 2 新增：滤镜
# ============================================================

[filter]
initial = "none"
inherited = false
# values: none, blur(), brightness(), contrast(), drop-shadow(),
#         grayscale(), hue-rotate(), invert(), opacity(), saturate(), sepia()

[backdrop-filter]
initial = "none"
inherited = false
# values: 同 filter

# ============================================================
# Phase 2 新增：混合模式
# ============================================================

[mix-blend-mode]
initial = "normal"
inherited = false
# values: normal, multiply, screen, overlay, darken, lighten,
#         color-dodge, color-burn, hard-light, soft-light,
#         difference, exclusion, hue, saturation, color, luminosity

[background-blend-mode]
initial = "normal"
inherited = false
# values: 同 mix-blend-mode

# ============================================================
# Phase 2 新增：边框图像
# ============================================================

[border-image-source]
initial = "none"
inherited = false

[border-image-slice]
initial = "100%"
inherited = false

[border-image-width]
initial = "1"
inherited = false

[border-image-outset]
initial = "0"
inherited = false

[border-image-repeat]
initial = "stretch"
inherited = false
# values: stretch, repeat, round, space

[border-image]
initial = "none 100% 1 0 stretch"
inherited = false
# shorthand

# ============================================================
# Phase 2 新增：多列布局
# ============================================================

[column-count]
initial = "auto"
inherited = false

[column-width]
initial = "auto"
inherited = false

[column-gap-normal]
initial = "normal"
inherited = false

[column-rule-width]
initial = "medium"
inherited = false

[column-rule-style]
initial = "none"
inherited = false

[column-rule-color]
initial = "currentColor"
inherited = false

[column-rule]
initial = "medium none currentColor"
inherited = false
# shorthand

[column-span]
initial = "none"
inherited = false
# values: none, all

[column-fill]
initial = "balance"
inherited = false
# values: balance, auto

[columns]
initial = "auto auto"
inherited = false
# shorthand

# ============================================================
# Phase 2 新增：书写模式
# ============================================================

[writing-mode]
initial = "horizontal-tb"
inherited = true
# values: horizontal-tb, vertical-lr, vertical-rl

[direction]
initial = "ltr"
inherited = true
# values: ltr, rtl

# ============================================================
# Phase 2 新增：盒模型扩展
# ============================================================

[box-decoration-break]
initial = "slice"
inherited = false
# values: slice, clone

[resize]
initial = "none"
inherited = false
# values: none, both, horizontal, vertical

[caret-color]
initial = "auto"
inherited = true

# ============================================================
# Phase 2 新增：SVG 相关属性
# ============================================================

[fill]
initial = "black"
inherited = true

[stroke]
initial = "none"
inherited = true

[stroke-width]
initial = "1"
inherited = true

[stroke-linecap]
initial = "butt"
inherited = true

[stroke-linejoin]
initial = "miter"
inherited = true

[stroke-dasharray]
initial = "none"
inherited = true

# ============================================================
# Phase 3+ 属性预留
# ============================================================
# [mask-*] [clip-path] [shape-outside]
# [scroll-behavior] [scroll-snap-*]
# [offset-*] (CSS Motion Path)
# [contain] [content-visibility]
# [aspect-ratio] [accent-color]
```

### 3.3 selector.rs — 完整伪类支持

```rust
// === crates/css/src/selector.rs ===

// Phase 2 新增完整伪类支持

/// 伪类枚举 —— Phase 2 完整实现
#[derive(Debug, Clone, PartialEq)]
pub enum PseudoClass {
    // ===== 用户交互伪类 =====
    /// :hover —— 鼠标悬停
    Hover,
    /// :active —— 激活中（鼠标按下）
    Active,
    /// :focus —— 获得焦点
    Focus,
    /// :focus-visible —— 可见焦点（键盘导航时）
    FocusVisible,
    /// :focus-within —— 自身或后代获得焦点
    FocusWithin,

    // ===== 链接伪类 =====
    /// :link —— 未访问的链接
    Link,
    /// :visited —— 已访问的链接
    Visited,

    // ===== 表单状态伪类 =====
    /// :checked —— 已选中的 checkbox/radio/option
    Checked,
    /// :disabled —— 已禁用的表单元素
    Disabled,
    /// :enabled —— 可用的表单元素
    Enabled,
    /// :required —— 必填的 input/select/textarea
    Required,
    /// :optional —— 可选的 input/select/textarea
    Optional,
    /// :valid —— 验证通过的表单元素
    Valid,
    /// :invalid —— 验证失败的表单元素
    Invalid,
    /// :in-range —— 值在 min/max 范围内
    InRange,
    /// :out-of-range —— 值超出 min/max 范围
    OutOfRange,
    /// :read-only —— 只读表单元素
    ReadOnly,
    /// :read-write —— 可写表单元素
    ReadWrite,
    /// :placeholder-shown —— 正在显示 placeholder
    PlaceholderShown,
    /// :default —— 默认选中的元素
    Default,
    /// :indeterminate —— 不确定状态
    Indeterminate,

    // ===== 结构伪类 =====
    /// :first-child —— 父元素的第一个子元素
    FirstChild,
    /// :last-child —— 父元素的最后一个子元素
    LastChild,
    /// :only-child —— 父元素的唯一子元素
    OnlyChild,
    /// :first-of-type —— 同类型中的第一个
    FirstOfType,
    /// :last-of-type —— 同类型中的最后一个
    LastOfType,
    /// :only-of-type —— 同类型中的唯一
    OnlyOfType,
    /// :nth-child(an+b) —— an+b 公式计算
    NthChild(i32, i32),
    /// :nth-last-child(an+b) —— 从末尾计数
    NthLastChild(i32, i32),
    /// :nth-of-type(an+b) —— 同类型按公式
    NthOfType(i32, i32),
    /// :nth-last-of-type(an+b) —— 同类型从末尾按公式
    NthLastOfType(i32, i32),

    // ===== 其他 =====
    /// :empty —— 无子节点（含文本节点）
    Empty,
    /// :root —— 文档根元素
    Root,
    /// :not(selector) —— 排除匹配
    Not(Box<Selector>),
    /// :is(selector-list) —— 匹配列表中任一选择器
    Is(Vec<Selector>),
    /// :where(selector-list) —— 同 :is() 但不增加特异性
    Where(Vec<Selector>),
    // Phase 3+: :has(selector) —— 包含匹配后代的选择器（性能开销大）
}

impl SelectorEngine {
    // Phase 2 新增

    /// 带伪类状态的元素匹配
    ///
    /// pseudo_state: 当前元素激活的伪类集合
    /// 例: { Hover, Focus } 表示元素处于 hover + focus 状态
    pub fn matches_with_pseudo(
        &self,
        element: &ElementData,
        selector: &Selector,
        pseudo_state: &HashSet<PseudoClass>,
    ) -> bool;

    /// 计算选择器的特异性（新增伪类贡献）
    /// 规则：:is() / :not() 取其内部最高特异性
    ///       :where() 不增加特异性
    ///       :nth-child 等结构伪类贡献 (0,1,0)
    pub fn compute_specificity_phase2(selector: &Selector) -> (u32, u32, u32);

    /// 评估 nth-child(an+b) 公式
    fn eval_nth(a: i32, b: i32, index: usize, total: usize) -> bool;

    /// 获取元素在同级同类型中的索引
    fn index_of_type(element: &ElementData, parent: &Node) -> usize;

    /// 获取元素在同级所有子元素中的索引
    fn index_of_child(element: &ElementData, parent: &Node) -> usize;
}

/// 伪类状态集合 —— 运行时动态维护
///
/// 每帧渲染前更新：
/// - Hover: hit_test → 当前鼠标位置下的元素
/// - Focus: 跟踪当前聚焦元素
/// - Active: 鼠标按下状态
/// - Checked/Disabled/Enabled: DOM 属性变更时更新
pub type PseudoState = HashSet<PseudoClass>;
```

### 3.4 cascade.rs — 级联增强

```rust
// === crates/css/src/cascade.rs ===

// Phase 2 新增

/// 级联缓存 —— 避免重复计算样式
///
/// 缓存键：(node_ptr, parent_node_ptr, dirty_flag_version)
/// 缓存无效时自动重算
pub struct CascadeCache {
    entries: HashMap<CascadeCacheKey, ComputedStyle>,
    /// 缓存命中计数
    hits: usize,
    /// 缓存未命中计数
    misses: usize,
}

#[derive(Hash, PartialEq, Eq)]
struct CascadeCacheKey {
    node_ptr: usize,
    parent_ptr: usize,
    version: u64,
}

impl CascadeCache {
    pub fn new() -> Self;
    pub fn get(&self, key: &CascadeCacheKey) -> Option<&ComputedStyle>;
    pub fn insert(&mut self, key: CascadeCacheKey, style: ComputedStyle);
    pub fn invalidate_node(&mut self, node_ptr: usize);
    pub fn invalidate_all(&mut self);
    pub fn hit_rate(&self) -> f32;
}

impl ComputedStyle {
    // Phase 2 新增

    /// 样式失效检测：对比两个 ComputedStyle 是否产生相同的布局/绘制结果
    /// 用于增量更新判断
    pub fn has_layout_diff(&self, other: &ComputedStyle) -> bool;
    pub fn has_paint_diff(&self, other: &ComputedStyle) -> bool;
}

/// 用户代理默认样式表（Phase 2）
///
/// 内置 HTML 元素的默认样式：
/// - h1-h6: 字号 + 粗体 + margin
/// - p: margin-top/bottom
/// - a: color + text-decoration
/// - ul/ol: padding-left + list-style
/// - table: border-spacing
/// - 等
pub const USER_AGENT_STYLESHEET: &str = include_str!("../assets/user_agent.css");

/// 解析关键字扩展：新增 revert
///
/// revert: 回退到 user-agent 层或 user-layer 的值
/// 级联层次（优先级从高到低）:
///   Phase 0/1: !important author > author > user-agent
///   Phase 2:   !important author > !important user > author > user > user-agent
pub fn resolve_keyword_phase2(
    keyword: &str,
    property: &str,
    parent_value: Option<&CSSValue>,
    user_agent_value: Option<&CSSValue>,
) -> Option<CSSValue>;
```

### 3.5 values.rs — 扩展类型

```rust
// === crates/css/src/values.rs ===

// ============================================================
//  Phase 2 新增 CSSValue 变体
// ============================================================

pub enum CSSValue {
    // Phase 0/1 已有变体 ...
    // Length, Percentage, Color, Keyword, Number, String,
    // Initial, Calc, Transform, Gradient, LengthPercentage

    // Phase 2 新增
    /// CSS 滤镜函数列表
    Filter(Vec<Filter>),
    /// 混合模式
    BlendMode(BlendModeKind),
    /// 网格弹性系数（用于 grid-template-* 中的 fr 单位）
    /// Phase 2: CSSUnit 新增 Fr 变体
}

// ============================================================
//  Phase 2 新增单位
// ============================================================

pub enum CSSUnit {
    // Phase 0/1 已有 ...
    // Px, Em, Rem, Percent, Vw, Vh, Vmin, Vmax,
    // Deg, Rad, Grad, Turn, S, Ms, Dpi, Dpcm

    // Phase 2 新增
    /// Grid 弹性系数（1fr = 1 份剩余空间）
    Fr,
}

// ============================================================
//  Phase 2 新增 Transform 3D 变体
// ============================================================

pub enum Transform {
    // Phase 1 已有 2D 变体 ...
    // Matrix([f32; 6]), Translate(f32, f32), Scale(f32, f32),
    // Rotate(f32), Skew(f32, f32), TranslateX/Y, ScaleX/Y, SkewX/Y

    // Phase 2 新增 3D 变体
    /// 3D 矩阵 (4x4 = 16 元素)
    Matrix3d([f32; 16]),
    /// Z 轴平移
    TranslateZ(f32),
    /// 3D 平移
    Translate3d(f32, f32, f32),
    /// Z 轴缩放
    ScaleZ(f32),
    /// 3D 缩放
    Scale3d(f32, f32, f32),
    /// X/Y/Z 轴旋转
    RotateX(f32),
    RotateY(f32),
    RotateZ(f32),
    /// 3D 旋转（绕任意轴）
    Rotate3d(f32, f32, f32, f32),
    /// 透视
    Perspective(f32),
}

// ============================================================
//  Phase 2 新增类型
// ============================================================

/// CSS 滤镜
#[derive(Debug, Clone)]
pub enum Filter {
    /// blur(5px) —— 高斯模糊
    Blur(f32),
    /// brightness(1.5) —— 亮度倍率
    Brightness(f32),
    /// contrast(1.2) —— 对比度倍率
    Contrast(f32),
    /// drop-shadow(2px 2px 4px black) —— 投影
    DropShadow(f32, f32, f32, Color),
    /// grayscale(100%) —— 灰度百分比
    Grayscale(f32),
    /// hue-rotate(90deg) —— 色相旋转角度
    HueRotate(f32),
    /// invert(100%) —— 反相百分比
    Invert(f32),
    /// opacity(50%) —— 透明度百分比
    Opacity(f32),
    /// saturate(200%) —— 饱和度倍率
    Saturate(f32),
    /// sepia(100%) —— 复古棕色调百分比
    Sepia(f32),
}

/// 混合模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendModeKind {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

/// 缓动函数 (CSS timing-function)
#[derive(Debug, Clone)]
pub enum EasingFunction {
    /// 线性: linear
    Linear,
    /// 缓入缓出: ease, ease-in, ease-out, ease-in-out
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// 自定义贝塞尔曲线: cubic-bezier(x1, y1, x2, y2)
    CubicBezier(f32, f32, f32, f32),
    /// 阶跃: steps(n, start|end) / step-start / step-end
    Steps(u32, StepDirection),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepDirection {
    Start,
    End,
}

// ============================================================
//  Phase 2 新增解析函数
// ============================================================

/// 解析 filter 属性值
pub fn parse_filter(value: &str) -> Vec<Filter>;

/// 解析 backdrop-filter 属性值
pub fn parse_backdrop_filter(value: &str) -> Vec<Filter>;

/// 解析 CSS 动画简写
pub fn parse_animation(value: &str) -> Vec<Animation>;

/// 解析 CSS 过渡简写
pub fn parse_transition(value: &str) -> Vec<Transition>;

/// 解析 timing-function
pub fn parse_timing_function(value: &str) -> EasingFunction;

/// 解析 transform 3D 扩展
pub fn parse_transform_3d(value: &str) -> Vec<Transform>;

/// 解析混合模式
pub fn parse_blend_mode(value: &str) -> BlendModeKind;

/// 解析 box-shadow / text-shadow
pub fn parse_shadow(value: &str, is_text_shadow: bool) -> Vec<Shadow>;

// Phase 3+:
// pub fn parse_clip_path(value: &str) -> ClipPath;
// pub fn parse_mask(value: &str) -> Vec<MaskLayer>;
```

### 3.6 animations.rs — CSS 动画引擎（全新）

```rust
// === crates/css/src/animations.rs ===

/// CSS 动画引擎 —— 管理所有活跃动画的生命周期
///
/// 核心流程：
/// 1. 解析 @keyframes 规则 → 关键帧序列
/// 2. 元素 animation-* 属性 → 动画配置
/// 3. 每帧插值计算 → 动画属性当前值
/// 4. 更新 ComputedStyle → 触发布局和重绘
pub struct AnimationEngine {
    /// 所有活跃动画
    animations: Vec<ActiveAnimation>,
    /// 全局动画时间线
    timeline: AnimationTimeline,
    /// 已注册的 @keyframes 规则
    keyframes: HashMap<String, Vec<Keyframe>>,
    /// 下一动画 ID
    next_id: u64,
}

/// 单个活跃动画
struct ActiveAnimation {
    /// 动画唯一标识
    id: u64,
    /// 动画名称（引用 @keyframes 规则）
    name: String,
    /// 目标 DOM 节点
    target_ptr: usize,
    /// 动画配置
    config: AnimationConfig,
    /// 当前播放状态
    state: PlayState,
    /// 当前时间偏移（秒）
    current_time: f64,
    /// 当前迭代次数
    current_iteration: u32,
}

/// 动画配置（从 animation-* 属性提取）
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    pub duration: f64,              // animation-duration（秒）
    pub timing_function: EasingFunction,
    pub delay: f64,                 // animation-delay（秒）
    pub iteration_count: AnimationIterationCount,
    pub direction: AnimationDirection,
    pub fill_mode: AnimationFillMode,
}

#[derive(Debug, Clone)]
pub enum AnimationIterationCount {
    /// 播放指定次数
    Count(u32),
    /// 无限循环
    Infinite,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationFillMode {
    None,
    Forwards,
    Backwards,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayState {
    /// 动画运行中
    Running,
    /// 动画暂停
    Paused,
    /// 动画已完成
    Finished,
    /// 动画已取消（元素移除或样式变更）
    Cancelled,
}

/// 动画时间线
pub struct AnimationTimeline {
    /// 时间线开始时间（monotonic clock）
    start_time: std::time::Instant,
}

impl AnimationTimeline {
    /// 获取当前时间（秒）
    pub fn current_time(&self) -> f64;
}

impl AnimationEngine {
    /// 创建动画引擎
    pub fn new() -> Self;

    /// 注册 @keyframes 规则
    pub fn register_keyframes(&mut self, name: &str, keyframes: Vec<Keyframe>);

    /// 为元素启动动画
    pub fn start_animation(
        &mut self,
        name: &str,
        target_ptr: usize,
        config: AnimationConfig,
    ) -> u64;

    /// 暂停指定动画
    pub fn pause(&mut self, id: u64);

    /// 恢复指定动画
    pub fn resume(&mut self, id: u64);

    /// 取消指定动画
    pub fn cancel(&mut self, id: u64);

    /// 取消元素的所有动画
    pub fn cancel_all(&mut self, target_ptr: usize);

    /// 每帧更新：推进所有动画时间、计算当前属性值
    ///
    /// delta: 帧间隔（秒）
    /// 返回：发生属性变更的 (node_ptr, property, new_value) 列表
    pub fn tick(
        &mut self,
        delta: f64,
    ) -> Vec<(usize, String, CSSValue)>;

    /// 在关键帧之间插值
    ///
    /// from: 起始关键帧
    /// to: 结束关键帧
    /// progress: [0.0, 1.0] 进度
    /// easing: 缓动函数
    fn interpolate(
        from: &Keyframe,
        to: &Keyframe,
        progress: f64,
        easing: &EasingFunction,
    ) -> HashMap<String, CSSValue>;

    /// 在两个 CSS 值之间线性插值
    fn interpolate_value(
        from: &CSSValue,
        to: &CSSValue,
        progress: f64,
    ) -> CSSValue;

    /// 获取元素当前动画计算值
    pub fn get_animated_value(
        &self,
        target_ptr: usize,
        property: &str,
    ) -> Option<CSSValue>;

    /// 是否有活跃动画
    pub fn has_active_animations(&self) -> bool;
}
```

### 3.7 transitions.rs — CSS 过渡引擎（全新）

```rust
// === crates/css/src/transitions.rs ===

/// CSS 过渡引擎 —— 监听属性变更并平滑过渡
///
/// 与动画的区别：
/// - 动画：主动播放，由 @keyframes 定义
/// - 过渡：被动触发，由属性变更驱动
///
/// 工作流：
/// 1. 元素 ComputedStyle 变更 → 检测过渡属性
/// 2. 匹配 transition-property → transition-duration > 0s
/// 3. 记录变更前的旧值作为起始状态
/// 4. 每帧插值计算中间值
/// 5. 到达 duration 后移除过渡状态
pub struct TransitionEngine {
    /// 所有活跃过渡
    active_transitions: Vec<ActiveTransition>,
}

/// 单个活跃过渡
struct ActiveTransition {
    /// 目标 DOM 节点
    target_ptr: usize,
    /// 过渡的属性名
    property: String,
    /// 起始值
    from: CSSValue,
    /// 目标值
    to: CSSValue,
    /// 持续时间（秒）
    duration: f64,
    /// 缓动函数
    easing: EasingFunction,
    /// 延迟时间（秒）
    delay: f64,
    /// 已运行时间（秒）
    elapsed: f64,
}

impl TransitionEngine {
    /// 创建过渡引擎
    pub fn new() -> Self;

    /// 检测属性变更是否需要触发过渡
    ///
    /// 比较新旧 ComputedStyle，对有过渡配置的属性创建 ActiveTransition
    pub fn detect_transitions(
        &mut self,
        target_ptr: usize,
        old_style: &ComputedStyle,
        new_style: &ComputedStyle,
        transition_config: &TransitionConfig,
    );

    /// 每帧更新：推进所有过渡
    ///
    /// delta: 帧间隔（秒）
    /// 返回：发生过渡中属性变更的 (node_ptr, property, current_value) 列表
    pub fn tick(
        &mut self,
        delta: f64,
    ) -> Vec<(usize, String, CSSValue)>;

    /// 取消元素的所有过渡
    pub fn cancel_all(&mut self, target_ptr: usize);

    /// 是否有活跃过渡
    pub fn has_active_transitions(&self) -> bool;
}

/// 过渡配置（从 transition-* 属性提取）
#[derive(Debug, Clone)]
pub struct TransitionConfig {
    /// 过渡属性列表（"all" = 全部属性）
    pub properties: Vec<String>,
    /// 持续时间（秒）
    pub duration: f64,
    /// 缓动函数
    pub timing_function: EasingFunction,
    /// 延迟时间（秒）
    pub delay: f64,
}
```

### 3.8 media.rs — 媒体查询求值引擎（全新）

```rust
// === crates/css/src/media.rs ===

/// 媒体查询求值引擎 —— 评估 @media 规则是否匹配
///
/// 支持的媒体特征（Phase 2）:
/// - width / min-width / max-width
/// - height / min-height / max-height
/// - orientation: portrait / landscape
/// - aspect-ratio / min-aspect-ratio / max-aspect-ratio
/// - resolution / min-resolution / max-resolution
/// - prefers-color-scheme: light / dark
/// - prefers-reduced-motion: reduce / no-preference
/// - pointer: none / coarse / fine
/// - hover: none / hover
pub struct MediaQueryEvaluator {
    /// 当前设备信息
    device_info: DeviceInfo,
}

/// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 视口宽度（像素）
    pub viewport_width: f32,
    /// 视口高度（像素）
    pub viewport_height: f32,
    /// 设备像素比
    pub device_pixel_ratio: f32,
    /// 首选色彩方案
    pub prefers_color_scheme: ColorScheme,
    /// 是否偏好减少动画
    pub prefers_reduced_motion: bool,
    /// 主输入设备类型
    pub pointer_type: PointerType,
    /// 是否支持 hover
    pub hover_capable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorScheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerType {
    None,
    Coarse,  // 触摸
    Fine,    // 鼠标/触控笔
}

impl MediaQueryEvaluator {
    /// 创建求值器
    pub fn new(device_info: DeviceInfo) -> Self;

    /// 更新设备信息（窗口 resize 时调用）
    pub fn update_device_info(&mut self, info: DeviceInfo);

    /// 求值媒体查询
    pub fn evaluate(&self, query: &MediaQuery) -> bool;

    /// 求值单个媒体条件
    fn evaluate_condition(&self, condition: &MediaCondition) -> bool;

    /// 在多个样式表中筛选匹配的媒体规则
    pub fn filter_stylesheets(
        &self,
        stylesheets: &[StyleSheet],
    ) -> Vec<&StyleSheet>;
}
```

### 3.9 custom_props.rs — CSS 自定义属性（全新）

```rust
// === crates/css/src/custom_props.rs ===

/// CSS 自定义属性处理器 —— --my-var: value
///
/// 支持：
/// - 声明: --primary-color: #ff0000;
/// - 引用: color: var(--primary-color);
/// - 回退: color: var(--primary-color, blue);
/// - 继承: 自定义属性默认继承
pub struct CustomPropertyHandler {
    /// 全局自定义属性映射（:root 上声明的）
    global_props: HashMap<String, CSSValue>,
    /// 每个元素的自定义属性（key = node_ptr）
    element_props: HashMap<usize, HashMap<String, CSSValue>>,
}

impl CustomPropertyHandler {
    /// 创建处理器
    pub fn new() -> Self;

    /// 设置元素上的自定义属性
    pub fn set(
        &mut self,
        node_ptr: usize,
        name: &str,
        value: CSSValue,
    );

    /// 设置全局自定义属性（:root 节点）
    pub fn set_global(&mut self, name: &str, value: CSSValue);

    /// 获取元素的自定义属性值
    /// 如果当前元素未定义，沿继承链向根查找
    pub fn get(
        &self,
        node_ptr: usize,
        parent_ptr: Option<usize>,
        name: &str,
    ) -> Option<CSSValue>;

    /// 解析 var() 引用
    ///
    /// var(--name) → 递归查找值
    /// var(--name, fallback) → 查找失败时使用 fallback
    pub fn resolve_var(
        &self,
        node_ptr: usize,
        parent_ptr: Option<usize>,
        var_name: &str,
        fallback: Option<CSSValue>,
    ) -> CSSValue;

    /// 递归解析值中所有 var() 引用
    pub fn resolve_all(
        &self,
        node_ptr: usize,
        parent_ptr: Option<usize>,
        value: &CSSValue,
    ) -> CSSValue;

    /// 移除元素上的自定义属性
    pub fn remove(&mut self, node_ptr: usize, name: &str);

    /// 清除元素所有自定义属性
    pub fn clear(&mut self, node_ptr: usize);
}
```

### 3.10 css 公开导出更新

```rust
// === crates/css/src/lib.rs ===

// Phase 0/1 已有导出 ...

// Phase 2 新增导出
pub use selector::{
    PseudoClass, PseudoState,
};
pub use cascade::CascadeCache;
pub use values::{
    Filter, BlendModeKind, EasingFunction, StepDirection,
    parse_filter, parse_backdrop_filter, parse_animation,
    parse_transition, parse_timing_function, parse_transform_3d,
    parse_blend_mode, parse_shadow,
};
pub use animations::{
    AnimationEngine, AnimationConfig, AnimationIterationCount,
    AnimationDirection, AnimationFillMode, PlayState,
};
pub use transitions::{TransitionEngine, TransitionConfig};
pub use media::{
    MediaQueryEvaluator, DeviceInfo, ColorScheme, PointerType,
};
pub use custom_props::CustomPropertyHandler;

// Phase 3+:
// pub use animations::ScrollDrivenAnimation;  // scroll-driven animations
// pub use media::MediaQueryList;              // JS MediaQueryList 接口
```

---

## 四、layout crate — 布局引擎 Grid + Float

### 4.1 模块结构

```
layout/
├── Cargo.toml
└── src/
    ├── lib.rs              # LayoutEngine 扩展 + 公开导出更新
    ├── layout_box.rs       # BoxType 新增 GridContainer/GridItem/Float
    ├── flex.rs             # (Phase 1 已完成)
    ├── block.rs            # float 环绕 + clear 处理 (Phase 2 扩展)
    ├── positioned.rs       # (Phase 1 已完成)
    ├── inline.rs           # (Phase 1 已完成)
    ├── table.rs            # colspan/rowspan + border-collapse 完善
    ├── text.rs             # (Phase 1 已完成)
    ├── grid.rs             # 新文件：GridLayout（taffy Grid 集成）
    └── float.rs            # 新文件：FloatLayout（浮动布局 + clear）
```

### 4.2 BoxType — 新增变体

```rust
// === crates/layout/src/layout_box.rs ===

pub enum BoxType {
    // Phase 0/1 已有
    Block, Inline, InlineBlock, FlexContainer, FlexItem,
    Table, TableRow, TableRowGroup, TableCell, TableCaption,
    Absolute, Fixed, Sticky, Text, Anonymous,

    // Phase 2 新增
    /// display: grid
    GridContainer,
    /// grid 子项
    GridItem,
    /// float: left / right
    Float,

    // Phase 3+:
    // Ruby, RubyBase, RubyText, // CSS Ruby
    // FlowRoot,                 // display: flow-root
}
```

### 4.3 LayoutBox — 新增字段

```rust
// === crates/layout/src/layout_box.rs ===

pub struct LayoutBox {
    // Phase 0/1 已有字段 ...

    // Phase 2 新增
    /// CSS transform 变换矩阵（累积变换）
    pub transform: Option<Vec<Transform>>,
    /// CSS filter 滤镜列表
    pub filters: Option<Vec<Filter>>,
    /// 混合模式
    pub mix_blend_mode: BlendModeKind,
    /// 是否创建新的层叠上下文（Phase 2: 自动检测 transform/filter/opacity 等触发条件）
    // pub stacking_context: bool,  // Phase 1 已引入，Phase 2 完善自动检测

    /// 表格单元格跨列数 (Phase 2)
    pub colspan: u32,
    /// 表格单元格跨行数 (Phase 2)
    pub rowspan: u32,

    // Phase 3+:
    // pub aspect_ratio: Option<f32>,
    // pub container_type: Option<ContainerType>,  // CSS Container Queries
}
```

### 4.4 grid.rs — 网格布局（全新）

```rust
// === crates/layout/src/grid.rs ===

/// 网格布局 —— 基于 taffy Grid 算法的完整实现
///
/// 实现 CSS Grid Layout Module Level 1 核心功能
///
/// 核心流程：
/// 1. 解析 grid-template-columns/rows → 轨道定义列表
/// 2. 解析 grid-template-areas → 命名区域映射
/// 3. 根据 grid-auto-flow + grid-column/row 放置子项
/// 4. 使用 taffy Grid 计算实际尺寸
/// 5. 应用 grid-gap 间距
pub struct GridLayout;

/// 网格轨道定义
#[derive(Debug, Clone)]
pub enum TrackSize {
    /// 固定像素
    Fixed(f32),
    /// 弹性比例（fr 单位）
    Flex(f32),
    /// 内容自适应
    Auto,
    /// 百分比
    Percent(f32),
    /// minmax(min, max)
    MinMax(Box<TrackSize>, Box<TrackSize>),
    /// fit-content(limit)
    FitContent(Option<f32>),
}

/// 网格线位置
#[derive(Debug, Clone)]
pub struct GridLine {
    /// 网格线编号（正数从 1 开始，负数从末尾计数）
    pub line_num: i32,
    /// 命名网格线（可选）
    pub named_line: Option<String>,
}

/// 网格区域
#[derive(Debug, Clone)]
pub struct GridArea {
    pub row_start: GridLine,
    pub row_end: GridLine,
    pub column_start: GridLine,
    pub column_end: GridLine,
}

/// 已解析的网格项放置信息
#[derive(Debug, Clone)]
struct GridItemPlacement {
    /// 子节点索引
    child_index: usize,
    /// 网格区域
    area: GridArea,
}

impl GridLayout {
    /// 对 Grid 容器执行网格布局
    ///
    /// container: GridContainer 类型的 LayoutBox
    /// taffy: taffy 布局实例
    /// viewport: 视口尺寸
    pub fn layout(
        &mut self,
        container: &mut LayoutBox,
        taffy: &mut taffy::TaffyTree,
        viewport: Size<f32>,
    );

    /// 解析 grid-template-columns/rows 为轨道列表
    ///
    /// "1fr 2fr 200px" → [Flex(1.0), Flex(2.0), Fixed(200.0)]
    /// "repeat(3, 1fr)" → [Flex(1.0), Flex(1.0), Flex(1.0)]
    /// "minmax(100px, 1fr)" → [MinMax(Fixed(100.0), Flex(1.0))]
    fn parse_track_list(style: &ComputedStyle, axis: GridAxis) -> Vec<TrackSize>;

    /// 解析 grid-template-areas → 命名区域的位置映射
    ///
    /// "a a ."      Area("a"): row1/col1→row1/col3
    /// "b b ."      Area("b"): row2/col1→row2/col3
    /// ". . c"      Area("c"): row3/col3→row3/col4
    fn parse_template_areas(
        style: &ComputedStyle,
        row_count: usize,
        col_count: usize,
    ) -> HashMap<String, GridArea>;

    /// 自动放置未明确指定位置的子项
    ///
    /// 根据 grid-auto-flow 决定放置方向（行优先 / 列优先）
    fn auto_place_items(
        children: &[LayoutBox],
        occupied_cells: &HashSet<(usize, usize)>,
        col_count: usize,
        row_count: usize,
        auto_flow: GridAutoFlow,
    ) -> Vec<GridItemPlacement>;

    /// 解析 grid-column / grid-row 简写为 GridArea
    fn parse_grid_area(style: &ComputedStyle) -> GridArea;

    /// 处理 repeat() 函数
    fn resolve_repeat(
        repeat_count: i32,
        track_defs: Vec<TrackSize>,
    ) -> Vec<TrackSize>;

    /// 将 TrackSize 转换为 taffy 可用的尺寸
    fn convert_to_taffy_size(track: &TrackSize) -> taffy::Dimension;

    /// 应用 gap 到容器子项
    fn apply_taffy_gap(container: &LayoutBox, taffy_style: &mut taffy::Style);

    // Phase 3+:
    // fn layout_subgrid(container: &mut LayoutBox);  // CSS Subgrid
}

#[derive(Debug, Clone, Copy)]
pub enum GridAxis {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridAutoFlow {
    Row,
    Column,
    RowDense,
    ColumnDense,
}
```

### 4.5 float.rs — 浮动布局（全新）

```rust
// === crates/layout/src/float.rs ===

/// 浮动布局 —— 实现 CSS float + clear
///
/// 浮动元素脱离普通流，向左或向右移动，直到边缘碰到容器或另一个浮动元素。
/// 后续的 inline/block 内容环绕浮动元素排列。
///
/// 处理流程：
/// 1. 收集浮动元素
/// 2. 为每个浮动元素计算浮动位置
/// 3. 后续元素检测 clear 属性并调整 Y 位置
/// 4. 行内内容按浮动区域约束水平空间
pub struct FloatLayout;

/// 浮动元素在容器中的占位信息
#[derive(Debug, Clone)]
pub struct FloatArea {
    /// 浮动区域矩形
    pub rect: Rect<f32>,
    /// 浮动方向
    pub float_direction: FloatDirection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatDirection {
    Left,
    Right,
}

/// clear 属性值
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClearValue {
    None,
    Left,
    Right,
    Both,
}

impl FloatLayout {
    /// 对包含浮动元素的容器执行浮动布局
    ///
    /// 1. 遍历子节点，将 Float 类型提取为浮动元素
    /// 2. 计算每个浮动元素的最终位置（靠左/靠右、避免重叠）
    /// 3. 记录浮动区域列表（FloatArea）
    /// 4. BlockLayout/InlineLayout 根据 FloatArea 调整内容区域宽度
    pub fn layout(
        &mut self,
        container: &mut LayoutBox,
        viewport: Size<f32>,
    );

    /// 放置浮动元素：计算其在容器中的水平和垂直位置
    ///
    /// 规则：
    /// - float: left → 排在左侧，从上到下
    /// - float: right → 排在右侧，从上到下
    /// - 同侧浮动元素依次排布，不可重叠
    /// - 宽度超出容器时自动换到下一行
    fn place_float(
        &mut self,
        float_box: &mut LayoutBox,
        existing_floats: &[FloatArea],
        container_width: f32,
        current_y: f32,
    ) -> Rect<f32>;

    /// 计算给定 Y 位置处可用水平空间（考虑浮动遮挡）
    ///
    /// 返回 (left_available_x, right_available_x, available_width)
    pub fn compute_available_width(
        float_areas: &[FloatArea],
        y: f32,
        container_width: f32,
    ) -> (f32, f32, f32);

    /// 计算 clear 元素的新 Y 位置
    ///
    /// clear: left → Y >= 所有 left 浮动元素的底部
    /// clear: right → Y >= 所有 right 浮动元素的底部
    /// clear: both → Y >= 所有浮动元素的底部
    pub fn compute_clear_position(
        clear_value: ClearValue,
        float_areas: &[FloatArea],
        current_y: f32,
    ) -> f32;

    /// 获取指定方向的浮动区域列表
    pub fn get_float_areas_for_side(
        float_areas: &[FloatArea],
        direction: FloatDirection,
    ) -> Vec<&FloatArea>;
}
```

### 4.6 block.rs — Float 集成扩展

```rust
// === crates/layout/src/block.rs ===

impl BlockLayout {
    // Phase 2 新增

    /// 处理浮动元素的 clear 属性
    ///
    /// 当块级元素设置 clear 时，计算需要向下偏移的 Y 距离
    /// get_clear_offset(element, float_areas) → 需要偏移的 Y 值
    pub fn compute_clear_offset(
        box_node: &LayoutBox,
        float_areas: &[FloatArea],
    ) -> f32;

    /// 在浮动环境下计算块级元素的宽度
    ///
    /// 块级元素在浮动环境下的宽度 = 容器宽度 - 浮动元素占用宽度
    fn compute_width_with_floats(
        box_node: &LayoutBox,
        float_areas: &[FloatArea],
        y: f32,
        parent_width: f32,
    ) -> f32;
}
```

### 4.7 table.rs — colspan/rowspan + border-collapse 完善

```rust
// === crates/layout/src/table.rs ===

impl TableLayout {
    // Phase 2 新增

    /// 处理 colspan / rowspan 单元格
    ///
    /// colspan=2 → 该单元格跨越两列，合并后续列宽
    /// rowspan=2 → 该单元格跨越两行，后续行的该列留空
    /// 返回：每行/列的真实单元格占用信息
    fn resolve_colspan_rowspan(
        table: &LayoutBox,
    ) -> Vec<CellSpanInfo>;

    /// 处理 border-collapse: collapse 的边框合并
    ///
    /// 边框冲突解决方案（W3C CSS 2.1 17.6.2.1）:
    /// 1. border-style: hidden 优先级最高
    /// 2. 较宽的边框优先
    /// 3. 宽度相同时，按样式优先级: double > solid > dashed > dotted > ridge > outset > groove > inset > none
    /// 4. 样式也相同时，按元素类型: cell > row > row-group > column > column-group > table
    /// 5. 位置仍然相同时，离左上角更近的优先
    fn resolve_collapsed_borders(table: &mut LayoutBox);

    /// 计算两个相邻边框的冲突结果
    fn compute_border_conflict(
        cell_a: &LayoutBox,
        cell_b: &LayoutBox,
        edge: TableEdge,
    ) -> (f32, BorderStyle, Color);

    /// 支持 table-layout: fixed
    /// 列宽由第一行单元格 + table 宽度 + col 元素决定，忽略后续行的内容
    fn fixed_table_layout(
        table: &mut LayoutBox,
        viewport: Size<f32>,
    );
}

/// 单元格跨度信息
struct CellSpanInfo {
    /// 在 children 中的索引
    pub child_index: usize,
    /// 起始行
    pub row_start: usize,
    /// 起始列
    pub col_start: usize,
    /// 跨行数
    pub row_span: u32,
    /// 跨列数
    pub col_span: u32,
}

/// 表格边框边
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableEdge {
    Top,
    Right,
    Bottom,
    Left,
}
```

### 4.8 LayoutEngine — Grid/Float 集成

```rust
// === crates/layout/src/lib.rs ===

impl LayoutEngine {
    // Phase 2 新增方法

    /// 获取当前所有浮动区域（用于 Block/Inline 布局的宽度计算）
    pub fn get_float_areas(&self) -> &[FloatArea];

    /// 清除浮动区域（每个 BFC 开始时重置）
    fn clear_float_areas(&mut self);
}

// Phase 2 新增：在 calculate_sizes 中新增分支

// match box_type {
//     GridContainer => {
//         let mut grid = GridLayout;
//         grid.layout(&mut self.taffy, node, viewport);
//     }
//     Float => {
//         // 由 FloatLayout 统一处理，此处 no-op
//     }
//     // ... 已有分支
// }
```

### 4.9 layout 公开导出更新

```rust
// === crates/layout/src/lib.rs ===

// Phase 0/1 已有导出 ...

// Phase 2 新增导出
pub use grid::{
    GridLayout, TrackSize, GridLine, GridArea,
    GridAxis, GridAutoFlow,
};
pub use float::{
    FloatLayout, FloatArea, FloatDirection, ClearValue,
};
pub use table::{CellSpanInfo, TableEdge};

// Phase 3+:
// pub use grid::SubgridLayout;
```

---

## 五、net crate — 网络层（全新）

### 5.1 模块结构

```
net/
├── Cargo.toml              # 依赖: reqwest, tokio, tungstenite
└── src/
    ├── lib.rs              # 模块导出
    ├── fetch.rs            # fetch API（基于 reqwest）
    └── websocket.rs        # WebSocket 实现（基于 tokio-tungstenite）
```

### 5.2 fetch.rs — fetch API

```rust
// === crates/net/src/fetch.rs ===

/// fetch API —— 与浏览器 fetch() 接口对齐
///
/// 基于 reqwest HTTP 客户端
/// 支持 HTTP/HTTPS GET/POST/PUT/DELETE
///
/// Phase 2: 同步/阻塞版本（在独立线程中驱动 tokio runtime）
/// Phase 3+: 真正异步版本（集成 tokio runtime）
///
/// 使用示例：
/// ```ignore
/// let resp = fetch("https://api.example.com/data", FetchRequest::default());
/// let json = resp.json::<MyType>().unwrap();
/// ```
pub fn fetch(url: &str, request: FetchRequest) -> Result<FetchResponse, FetchError>;

/// 异步 fetch (Phase 2 基础：通过独立线程+channel 模拟)
/// Phase 3+: 改为真正的 async fn
// pub async fn fetch_async(url: &str, request: FetchRequest) -> Result<FetchResponse, FetchError>;

/// HTTP 请求
#[derive(Debug, Clone)]
pub struct FetchRequest {
    /// HTTP 方法
    pub method: HttpMethod,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 请求体
    pub body: Option<Vec<u8>>,
    /// 超时时间（毫秒，None = 无超时）
    pub timeout_ms: Option<u64>,
    /// 是否跟随重定向
    pub redirect: RedirectMode,
    /// 跨域模式 (Phase 2: 默认 no-cors)
    pub mode: RequestMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Patch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RedirectMode {
    Follow,
    Error,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestMode {
    SameOrigin,
    NoCors,
    Cors,
    // Phase 3+: Navigate
}

impl Default for FetchRequest {
    fn default() -> Self {
        Self {
            method: HttpMethod::Get,
            headers: HashMap::new(),
            body: None,
            timeout_ms: Some(30000),
            redirect: RedirectMode::Follow,
            mode: RequestMode::NoCors,
        }
    }
}

/// HTTP 响应
#[derive(Debug, Clone)]
pub struct FetchResponse {
    /// HTTP 状态码
    pub status: u16,
    /// 状态文本
    pub status_text: String,
    /// HTTP 版本
    pub http_version: String,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体（原始字节）
    pub body: Vec<u8>,
    /// 响应 URL（可能因重定向与请求 URL 不同）
    pub url: String,
    /// 是否成功（200-299）
    pub ok: bool,
    /// 响应类型
    pub response_type: ResponseType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResponseType {
    Basic,
    Cors,
    Error,
    Opaque,
}

impl FetchResponse {
    /// 解析 JSON 响应体
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, FetchError>;

    /// 获取 UTF-8 文本响应体
    pub fn text(&self) -> Result<String, FetchError>;

    /// 获取指定响应头的值
    pub fn get_header(&self, name: &str) -> Option<&str>;

    /// 内容长度（字节）
    pub fn content_length(&self) -> usize;
}

/// 请求错误
#[derive(Debug)]
pub enum FetchError {
    /// 网络连接错误
    NetworkError(String),
    /// URL 解析错误
    UrlParseError(String),
    /// HTTP 错误（4xx/5xx 等，仅当 redirect=Error 时）
    HttpError(u16, String),
    /// 超时
    Timeout,
    /// JSON 解析错误
    JsonParseError(String),
    /// 请求被取消
    Aborted,
}

/// 便捷函数
impl FetchRequest {
    /// 创建 GET 请求
    pub fn get() -> Self { Self { method: HttpMethod::Get, ..Default::default() } }

    /// 创建 POST 请求（JSON body）
    pub fn post_json<T: serde::Serialize>(body: &T) -> Result<Self, FetchError>;

    /// 设置请求头
    pub fn set_header(mut self, key: &str, value: &str) -> Self;

    /// 设置 Authorization Bearer token
    pub fn bearer_auth(mut self, token: &str) -> Self;

    /// 设置超时
    pub fn timeout(mut self, ms: u64) -> Self;
}
```

### 5.3 websocket.rs — WebSocket 基础

```rust
// === crates/net/src/websocket.rs ===

/// WebSocket 连接 —— 基于 tokio-tungstenite
///
/// Phase 2: 基础功能（连接、发送、接收文本/二进制消息）
/// Phase 3+: 完整（自动重连、心跳 ping/pong、压缩扩展）
pub struct WebSocket {
    /// 连接 URL
    url: String,
    /// 连接状态
    state: WebSocketState,
    /// 内部消息通道
    message_tx: Option<std::sync::mpsc::Sender<WebSocketMessage>>,
    message_rx: Option<std::sync::mpsc::Receiver<WebSocketMessage>>,
    /// 事件回调
    on_open: Option<Box<dyn Fn()>>,
    on_message: Option<Box<dyn Fn(WebSocketMessage)>>,
    on_error: Option<Box<dyn Fn(String)>>,
    on_close: Option<Box<dyn Fn(u16, String)>>,
}

/// WebSocket 连接状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebSocketState {
    Connecting,
    Open,
    Closing,
    Closed,
}

/// WebSocket 消息
#[derive(Debug, Clone)]
pub enum WebSocketMessage {
    /// 文本消息
    Text(String),
    /// 二进制消息
    Binary(Vec<u8>),
    /// Ping
    Ping(Vec<u8>),
    /// Pong
    Pong(Vec<u8>),
    /// 关闭帧
    Close(Option<u16>, Option<String>),
}

impl WebSocket {
    /// 创建 WebSocket 连接
    ///
    /// url: ws:// 或 wss:// 地址
    pub fn new(url: &str) -> Self;

    /// 打开连接（在后台线程创建 tokio runtime 并连接）
    pub fn open(&mut self);

    /// 发送文本消息
    pub fn send_text(&self, text: &str) -> Result<(), WebSocketError>;

    /// 发送二进制消息
    pub fn send_binary(&self, data: &[u8]) -> Result<(), WebSocketError>;

    /// 发送 Ping
    pub fn ping(&self, data: &[u8]) -> Result<(), WebSocketError>;

    /// 关闭连接
    /// code: 关闭码（1000=正常关闭）, reason: 关闭原因
    pub fn close(&mut self, code: Option<u16>, reason: Option<&str>);

    /// 获取当前状态
    pub fn state(&self) -> WebSocketState;

    /// 轮询接收消息（非阻塞）
    /// 返回所有待处理消息
    pub fn poll_messages(&mut self) -> Vec<WebSocketMessage>;

    /// 设置事件回调
    pub fn set_on_open(&mut self, callback: Box<dyn Fn()>);
    pub fn set_on_message(&mut self, callback: Box<dyn Fn(WebSocketMessage)>);
    pub fn set_on_error(&mut self, callback: Box<dyn Fn(String)>);
    pub fn set_on_close(&mut self, callback: Box<dyn Fn(u16, String)>);
}

/// WebSocket 错误
#[derive(Debug)]
pub enum WebSocketError {
    ConnectionFailed(String),
    SendFailed(String),
    NotConnected,
    AlreadyClosed,
    InvalidUrl(String),
    TlsError(String),
}

// ============================================================
//  WebSocket 标准关闭码
// ============================================================
pub mod close_code {
    pub const NORMAL_CLOSURE: u16 = 1000;
    pub const GOING_AWAY: u16 = 1001;
    pub const PROTOCOL_ERROR: u16 = 1002;
    pub const UNSUPPORTED_DATA: u16 = 1003;
    pub const NO_STATUS_RECEIVED: u16 = 1005;
    pub const ABNORMAL_CLOSURE: u16 = 1006;
    pub const INVALID_FRAME_PAYLOAD_DATA: u16 = 1007;
    pub const POLICY_VIOLATION: u16 = 1008;
    pub const MESSAGE_TOO_BIG: u16 = 1009;
    pub const MANDATORY_EXTENSION: u16 = 1010;
    pub const INTERNAL_ERROR: u16 = 1011;
}
```

### 5.4 net 公开导出

```rust
// === crates/net/src/lib.rs ===

pub use fetch::{
    fetch, FetchRequest, FetchResponse, FetchError,
    HttpMethod, RedirectMode, RequestMode, ResponseType,
};

pub use websocket::{
    WebSocket, WebSocketMessage, WebSocketState, WebSocketError,
    close_code,
};

// Phase 3+:
// pub use fetch::fetch_async;
// pub use websocket::WebSocketEventLoop;
// pub use server_sent_events::EventSource;    // SSE
// pub use webrtc::RTCPeerConnection;           // WebRTC
```

---

## 六、storage crate — 存储层（全新）

### 6.1 模块结构

```
storage/
├── Cargo.toml
└── src/
    ├── lib.rs              # 公开导出 + StorageBackend trait
    ├── local_storage.rs    # LocalStorage 实现
    └── session_storage.rs  # SessionStorage 实现
```

### 6.2 StorageBackend trait

```rust
// === crates/storage/src/lib.rs ===

/// 存储后端 trait —— 统一 localStorage / sessionStorage 接口
///
/// 对应 W3C Storage 接口
pub trait StorageBackend {
    /// 存储键值对数量
    fn length(&self) -> usize;

    /// 按索引获取键名（用于遍历）
    fn key(&self, index: usize) -> Option<String>;

    /// 获取键对应的值
    fn get_item(&self, key: &str) -> Option<String>;

    /// 设置键值对
    fn set_item(&mut self, key: &str, value: &str);

    /// 移除键值对
    fn remove_item(&mut self, key: &str);

    /// 清空所有键值对
    fn clear(&mut self);

    /// 检查键是否存在
    fn contains_key(&self, key: &str) -> bool;
}
```

### 6.3 local_storage.rs — LocalStorage

```rust
// === crates/storage/src/local_storage.rs ===

/// localStorage —— 持久化本地存储
///
/// 数据持久化到磁盘（应用关闭后保留）
/// 存储位置: ~/.local/share/<app_name>/local_storage.json
///
/// 特性：
/// - 容量限制: 默认 5MB（可配置）
/// - 仅支持 String 类型（W3C 标准）
/// - 同步读写（Phase 2） / 异步版本（Phase 3+）
pub struct LocalStorage {
    /// 内存缓存
    entries: HashMap<String, String>,
    /// 持久化文件路径
    file_path: Option<std::path::PathBuf>,
    /// 最大容量（字节）
    max_size: usize,
    /// 应用名称（用于文件路径）
    app_name: String,
    /// 是否已修改（需要 flush）
    dirty: bool,
}

impl LocalStorage {
    /// 创建 LocalStorage 实例
    ///
    /// app_name: 用于生成存储文件名
    /// auto_load: 是否自动从磁盘加载已有数据
    pub fn new(app_name: &str, auto_load: bool) -> Self;

    /// 从磁盘加载之前保存的数据
    pub fn load(&mut self) -> Result<(), StorageError>;

    /// 持久化到磁盘
    pub fn flush(&mut self) -> Result<(), StorageError>;

    /// 获取当前使用大小（字节）
    pub fn current_size(&self) -> usize;

    /// 获取最大容量（字节）
    pub fn max_size(&self) -> usize;

    /// 设置最大容量（字节）
    pub fn set_max_size(&mut self, bytes: usize);
}

impl StorageBackend for LocalStorage {
    fn length(&self) -> usize;
    fn key(&self, index: usize) -> Option<String>;
    fn get_item(&self, key: &str) -> Option<String>;
    fn set_item(&mut self, key: &str, value: &str);
    fn remove_item(&mut self, key: &str);
    fn clear(&mut self);
    fn contains_key(&self, key: &str) -> bool;
}

/// 存储错误
#[derive(Debug)]
pub enum StorageError {
    /// IO 错误（磁盘读写失败）
    IoError(std::io::Error),
    /// 序列化错误
    SerializeError(String),
    /// 超出最大容量
    QuotaExceeded { current: usize, max: usize, requested: usize },
}
```

### 6.4 session_storage.rs — SessionStorage

```rust
// === crates/storage/src/session_storage.rs ===

/// sessionStorage —— 会话级临时存储
///
/// 数据仅在应用运行期间保留（窗口关闭后清空）
/// 纯内存实现，不持久化到磁盘
///
/// 特性：
/// - 无容量硬限制（机器内存限制）
/// - 仅支持 String 类型（W3C 标准）
/// - 同步读写
pub struct SessionStorage {
    /// 内存存储
    entries: HashMap<String, String>,
}

impl SessionStorage {
    /// 创建 SessionStorage 实例
    pub fn new() -> Self;
}

impl StorageBackend for SessionStorage {
    fn length(&self) -> usize;
    fn key(&self, index: usize) -> Option<String>;
    fn get_item(&self, key: &str) -> Option<String>;
    fn set_item(&mut self, key: &str, value: &str);
    fn remove_item(&mut self, key: &str);
    fn clear(&mut self);
    fn contains_key(&self, key: &str) -> bool;
}
```

### 6.5 storage 公开导出

```rust
// === crates/storage/src/lib.rs ===

pub use local_storage::{LocalStorage, StorageError};
pub use session_storage::SessionStorage;

pub trait StorageBackend { /* ... */ }

// Phase 3+:
// pub use indexed_db::IndexedDB;   // IndexedDB 实现
// pub use cache_storage::CacheStorage;  // Cache API (Service Worker)
```

---

## 七、paint crate — Transform / Filter / Gradient 扩展

### 7.1 PaintCommand — 新增变体

```rust
// === crates/paint/src/command.rs ===

pub enum PaintCommand {
    // Phase 0/1 已有变体 ...
    // FillRect, Text, Border, BoxShadow, Image, Clip, Opacity

    // Phase 2 新增
    /// 2D/3D 变换
    Transform {
        /// 变换矩阵（6 元素 2D 或 16 元素 3D）
        matrix: TransformMatrix,
        /// 变换范围内的子命令
        children: Vec<PaintCommand>,
    },
    /// CSS 滤镜
    Filter {
        /// 滤镜列表（按声明顺序应用）
        filters: Vec<Filter>,
        /// 滤镜作用范围内的子命令
        children: Vec<PaintCommand>,
    },
    /// 渐变填充
    GradientFill {
        rect: Rect<Pixel>,
        /// 渐变定义
        gradient: Gradient,
        /// 边框圆角（与渐变区域匹配）
        border_radius: [f32; 4],
    },
    /// 3D 透视图元 (Phase 2 基础)
    // Phase 3+: 完整 3D 场景图

    // Phase 3+:
    // BackdropFilter { filters: Vec<Filter>, children: Vec<PaintCommand> },
}

/// 变换矩阵
#[derive(Debug, Clone)]
pub enum TransformMatrix {
    /// 2D 仿射矩阵 [a, b, c, d, tx, ty]
    Matrix2D([f32; 6]),
    /// 3D 矩阵 (4×4, 列主序)
    Matrix3D([f32; 16]),
}

impl TransformMatrix {
    /// 单位矩阵
    pub fn identity_2d() -> Self;
    pub fn identity_3d() -> Self;

    /// 平移
    pub fn translate_2d(tx: f32, ty: f32) -> Self;
    /// 缩放
    pub fn scale_2d(sx: f32, sy: f32) -> Self;
    /// 旋转
    pub fn rotate_2d(angle: f32) -> Self;
    /// 矩阵乘法
    pub fn multiply(&self, other: &Self) -> Self;
    /// 应用到点
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32);
    /// 应用到矩形（返回包围盒）
    pub fn transform_rect(&self, rect: &Rect<f32>) -> Rect<f32>;
}

/// 渐变类型扩展 (Phase 1 为基础定义)
// Phase 2 使用 css::values::Gradient 进行渲染
```

### 7.2 DisplayListBuilder — 新增处理

```rust
// === crates/paint/src/builder.rs ===

impl DisplayListBuilder {
    // Phase 2 新增

    /// 提取并生成 Transform 命令
    fn extract_transform(
        &self,
        node: &LayoutBox,
        children: Vec<PaintCommand>,
    ) -> Option<PaintCommand>;

    /// 提取并生成 Filter 命令
    fn extract_filter(
        &self,
        node: &LayoutBox,
        children: Vec<PaintCommand>,
    ) -> Option<PaintCommand>;

    /// 提取并生成渐变填充命令
    fn extract_gradient_fill(
        &self,
        node: &LayoutBox,
    ) -> Option<PaintCommand>;

    /// 处理混合模式（更新 state，影响后续绘制命令）
    fn apply_blend_mode(&self, node: &LayoutBox);
}
```

### 7.3 optimizer.rs — 扩展

```rust
// === crates/paint/src/optimizer.rs ===

// Phase 2 新增 BatchType 变体
pub enum BatchType {
    // Phase 1 已有
    SolidRect, TexturedRect, Text, Border, Shadow,

    // Phase 2 新增
    /// 渐变填充
    Gradient,
    /// 变换组（需要单独的 GPU pass）
    TransformGroup,
    /// 滤镜组
    FilterGroup,
}

impl BatchOptimizer {
    // Phase 2 新增

    /// 对 Transform 命令进行矩阵合并优化
    ///
    /// 连续嵌套的 Transform 可合并为单个矩阵
    fn merge_nested_transforms(commands: &[PaintCommand]) -> Vec<PaintCommand>;

    /// 对滤镜组进行简化
    ///
    /// blur(0) / opacity(1) / grayscale(0) 等无效果滤镜可安全移除
    fn simplify_filters(commands: &[PaintCommand]) -> Vec<PaintCommand>;
}
```

### 7.4 paint 公开导出更新

```rust
// === crates/paint/src/lib.rs ===

// Phase 0/1 已有导出 ...

// Phase 2 新增导出
pub use command::TransformMatrix;
```

---

## 八、render crate — 变换/滤镜/渐变管线

### 8.1 模块结构

```
render/
├── Cargo.toml
├── shaders/
│   ├── rect.wgsl            # Phase 0 已有
│   ├── border.wgsl          # Phase 1 已有
│   ├── image.wgsl           # Phase 1 已有
│   ├── shadow.wgsl          # Phase 1 已有
│   ├── gradient.wgsl        # Phase 2: 渐变着色器
│   └── filter.wgsl          # Phase 2: 滤镜着色器（模糊/亮度/对比度等）
└── src/
    ├── lib.rs               # 公开导出更新
    ├── wgpu_backend.rs      # 新增渐变/滤镜/变换管线
    └── text_renderer.rs     # 多字体回退 + 图集动态增长
```

### 8.2 WgpuBackend — 新增管线

```rust
// === crates/render/src/wgpu_backend.rs ===

pub struct WgpuBackend {
    // Phase 0/1 已有字段 ...

    // Phase 2 新增
    /// 渐变渲染管线
    gradient_pipeline: wgpu::RenderPipeline,
    /// 滤镜渲染管线
    filter_pipeline: wgpu::RenderPipeline,
    /// 离屏渲染纹理（用于滤镜中间结果）
    offscreen_texture: Option<wgpu::Texture>,
    /// 变换矩阵栈（用于嵌套 Transform 命令）
    transform_stack: Vec<TransformMatrix>,
    /// 当前混合模式
    blend_mode: BlendModeKind,
}

impl WgpuBackend {
    // Phase 2 新增 —— 管线创建

    /// 创建渐变渲染管线（WGSL 着色器：gradient.wgsl）
    fn create_gradient_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline;

    /// 创建滤镜渲染管线（WGSL 着色器：filter.wgsl）
    fn create_filter_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline;

    // Phase 2 新增 —— 渲染编码

    /// 编码渐变绘制命令
    fn encode_gradient(
        &self,
        pass: &mut wgpu::RenderPass,
        cmd: &PaintCommand,
    );

    /// 编码变换 — 推入变换矩阵
    fn push_transform(&mut self, matrix: &TransformMatrix);

    /// 编码变换 — 弹出变换矩阵
    fn pop_transform(&mut self);

    /// 应用滤镜——需要离屏渲染
    ///
    /// 1. 将 children 渲染到离屏纹理
    /// 2. 对离屏纹理应用滤镜
    /// 3. 将结果绘制到目标
    fn encode_filter(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        cmd: &PaintCommand,
    );

    /// 设置混合模式
    fn set_blend_mode(&mut self, mode: BlendModeKind);

    /// 创建离屏纹理
    fn ensure_offscreen_texture(
        &mut self,
        width: u32,
        height: u32,
    );
}
```

### 8.3 TextRenderer — 多字体回退 + 图集动态增长

```rust
// === crates/render/src/text_renderer.rs ===

impl TextRenderer {
    // Phase 2 新增

    /// 多字体回退渲染
    ///
    /// 当首选字体缺少字形时，按回退链查找：
    /// 1. 首选字体 → 2. 系统 sans-serif → 3. 系统 serif → 4. 内置后备
    ///
    /// 返回每个字形实际使用的字体信息
    pub fn shape_text_with_fallback(
        &mut self,
        text: &str,
        families: &[&str],
        font_size: f32,
        font_weight: u16,
        font_style: FontStyle,
    ) -> Vec<ShapedGlyphWithFont>;

    /// 字形图集动态增长
    ///
    /// Phase 1: 固定大小图集
    /// Phase 2: 图谱空间不足时自动扩展（翻倍增长 512→1024→2048）
    fn grow_atlas(&mut self, min_size: (u32, u32));

    /// 从系统字体目录自动发现可用字体
    ///
    /// 扫描路径: /usr/share/fonts, C:\Windows\Fonts, /System/Library/Fonts
    pub fn discover_system_fonts(&mut self);

    /// 注册自定义字体（从字节数据）
    pub fn register_font_from_bytes(
        &mut self,
        data: &[u8],
        family: &str,
        weight: u16,
        style: FontStyle,
    ) -> Result<(), FontError>;
}

/// 带字体信息的字形
#[derive(Debug, Clone)]
pub struct ShapedGlyphWithFont {
    pub glyph: ShapedGlyph,
    /// 实际使用的字体名
    pub font_family: String,
    /// 是否使用了回退字体
    pub is_fallback: bool,
}

/// 字体错误
#[derive(Debug)]
pub enum FontError {
    InvalidFont(String),
    UnsupportedFormat(String),
    SystemFontNotFound,
}
```

### 8.4 render 公开导出更新

```rust
// === crates/render/src/lib.rs ===

// Phase 0/1 已有导出 ...

// Phase 2 新增导出
pub use wgpu_backend::WgpuBackend;
pub use text_renderer::{TextRenderer, ShapedGlyphWithFont, FontError};

// Phase 3+:
// pub use wgpu_backend::OffscreenRenderer;
// pub use video_renderer::VideoRenderer;
```

---

## 九、web2rust crate — JS 编译器扩展

### 9.1 Phase 2 JS 编译新增能力

```rust
// === crates/web2rust/src/js.rs ===

/// Phase 2 JS 编译能力
pub struct JsCompileCapabilitiesPhase2;

impl JsCompileCapabilitiesPhase2 {
    /// 获取能力清单
    pub fn capabilities() -> Vec<JsCapability>;
}

/// JS 编译能力项
pub struct JsCapability {
    pub name: String,
    pub category: JsCategory,
    pub rust_mapping: String,
    pub phase: u8,  // 2
}

pub enum JsCategory {
    Syntax,
    DOM,
    Async,
    Builtin,
    Module,
}

// ============================================================
//  Phase 2 JS 编译能力表（增量）
// ============================================================
//
// 语法扩展:
// try/catch/finally    → Rust Result<T, E> + match
// for...in             → for key in obj.keys()
// spread (...)         → .clone() or .into_iter().chain()
// destructuring        → let (a, b) = ... (Rust pattern)
// template literals    → format!("text {var} text")
// tagged templates     → 函数调用 + 数组参数
// optional chaining    → Option::and_then() 链
// nullish coalescing   → Option::unwrap_or()
//
// 异步:
// Promise.resolve      → futures::future::ready(val)
// Promise.reject       → futures::future::err(err)
// Promise.all          → futures::future::join_all
// Promise.race         → futures::future::select_all
// async function       → async fn() -> impl Future<Output = T>
// await                → .await
// .then(cb)            → .then(|val| cb(val))
// .catch(cb)           → .map_err(|e| cb(e))
//
// Class 系统:
// class MyClass {}     → struct MyClass { ... }
// constructor()        → fn new() -> Self { ... }
// method()             → impl MyClass { fn method(&self) { ... } }
// static method()      → impl MyClass { fn method() { ... } }
// get prop()           → fn prop(&self) -> T { ... }  // 需特殊标记
// set prop(v)          → fn set_prop(&mut self, v: T) { ... }
// extends Base         → 组合: struct MyClass { base: Base, ... }
// super.method()       → self.base.method()
// new.target           → 编译时类型信息（编译时常量）
// private #field       → 模块私有字段 (Rust pub(crate) 或 private)
//
// Proxy/Reflect:
// const p = new Proxy(target, handler)
//   → 编译时访问器重写：target.prop 编译为 handler.get(target, "prop")
//   Phase 2: 仅支持 get/set trap 的编译时重写
//   Phase 3+: 完整 Proxy 运行时支持
// Reflect.get(obj, prop) → obj.prop (直接访问)
// Reflect.set(obj, prop, val) → obj.prop = val
// Reflect.has(obj, prop) → obj.contains_key("prop")
//
// TypedArray / ArrayBuffer:
// new Uint8Array(10)   → vec![0u8; 10]
// new Float32Array([1,2,3]) → vec![1.0f32, 2.0, 3.0]
// new ArrayBuffer(16)  → vec![0u8; 16]
// DataView             → 编译时字节偏移访问
//
// 模块系统扩展:
// export default       → pub use
// import * as name     → use crate::module as name;
// dynamic import()     → Phase 2: 编译时静态分析 → Phase 3+: 运行时加载
// import.meta.url      → env!("CARGO_MANIFEST_DIR")
//
// 新增内置对象:
// Map                  → HashMap
// Set                  → HashSet
// WeakMap              → HashMap<*const (), V>  (指针键)
// WeakSet              → HashSet<*const ()>     (指针值)
// Array.from()         → iter.collect::<Vec<_>>()
// Array.isArray()      → 类型检查（编译时常量 true/false）
// Object.keys()        → hashmap.keys().cloned().collect()
// Object.values()      → hashmap.values().cloned().collect()
// Object.entries()     → hashmap.iter().map(|(k,v)| (k.clone(), v.clone()))
// Object.assign()      → 字段逐个赋值 + ..Default::default()
// Number.isNaN()       → f64::is_nan()
// Number.isFinite()    → f64::is_finite()
// Number.parseInt()    → String::parse::<i32>()
// String.prototype.trim() → .trim()
// String.prototype.includes() → .contains()
// String.prototype.startsWith() → .starts_with()
// String.prototype.endsWith() → .ends_with()
// String.prototype.split() → .split()
// String.prototype.slice() → [start..end]
// Array.prototype.map() → .iter().map().collect()
// Array.prototype.filter() → .into_iter().filter().collect()
// Array.prototype.reduce() → .into_iter().fold()
// Array.prototype.find() → .iter().find()
// Array.prototype.includes() → .contains()
// Array.prototype.join() → .join()
// Array.prototype.push() → .push()
// Array.prototype.pop() → .pop()
// Array.prototype.shift() → .remove(0) (效率低，内部用 VecDeque)
// Array.prototype.unshift() → .insert(0, val) (效率低)
// Array.prototype.slice() → [start..end].to_vec()
// Array.prototype.splice() → .splice(start, count, replacements)
// Error / TypeError / RangeError / SyntaxError → Rust std::error::Error trait impls
// AggregateError      → Vec<Box<dyn Error>>
```

### 9.2 analyzer.rs — 扩展

```rust
// === crates/web2rust/src/analyzer.rs ===

impl JsAnalyzer {
    // Phase 2 新增

    /// 识别 Promise 链
    /// .then(cb) / .catch(cb) / .finally(cb) → Future::then / map_err / inspect
    fn identify_promise_chains(
        module: &swc_ecma_ast::Module,
    ) -> Vec<PromiseChain>;

    /// 识别 Class 定义
    /// class MyClass { constructor, methods, static, getters, setters }
    fn identify_class_definitions(
        module: &swc_ecma_ast::Module,
    ) -> Vec<ClassDefinition>;

    /// 识别 Proxy 使用
    /// new Proxy(target, handler) → 编译时访问器重写标记
    fn identify_proxy_usage(
        module: &swc_ecma_ast::Module,
    ) -> Vec<ProxyUsage>;

    /// 识别 async/await 模式
    fn identify_async_functions(
        module: &swc_ecma_ast::Module,
    ) -> Vec<AsyncFunctionInfo>;

    /// 复杂类型推导（泛型 + union type）
    /// Phase 2: 基础推导 → Phase 3+: 完整类型推断
    fn infer_complex_types(
        module: &swc_ecma_ast::Module,
    ) -> HashMap<String, RustType>;
}

/// Promise 链信息
pub struct PromiseChain {
    pub target_var: String,
    pub then_callbacks: Vec<String>,
    pub catch_callback: Option<String>,
    pub finally_callback: Option<String>,
}

/// Class 定义信息
pub struct ClassDefinition {
    pub name: String,
    pub base_class: Option<String>,
    pub constructor: Option<ConstructorInfo>,
    pub methods: Vec<MethodInfo>,
    pub static_methods: Vec<MethodInfo>,
    pub getters: Vec<String>,
    pub setters: Vec<String>,
    pub private_fields: Vec<String>,
}

pub struct ConstructorInfo {
    pub params: Vec<(String, JsType)>,
    pub body: Vec<String>,
}

pub struct MethodInfo {
    pub name: String,
    pub params: Vec<(String, JsType)>,
    pub return_type: Option<JsType>,
    pub is_async: bool,
    pub body: Vec<String>,
}

/// Proxy 使用信息
pub struct ProxyUsage {
    pub target_var: String,
    pub handler_var: String,
    pub proxy_traps: Vec<ProxyTrap>,
}

pub enum ProxyTrap {
    Get { property: String, handler_fn: String },
    Set { property: String, handler_fn: String },
    Has { property: String, handler_fn: String },
    DeleteProperty { property: String, handler_fn: String },
    // Phase 3+: apply, construct, getPrototypeOf, ownKeys, ...
}

/// 异步函数信息
pub struct AsyncFunctionInfo {
    pub name: String,
    pub params: Vec<(String, JsType)>,
    pub return_type: JsType,
    pub await_expressions: Vec<String>,
    pub body: Vec<String>,
}

/// Rust 类型（增强版）
pub enum RustType {
    // Phase 1 已有
    String, Number, Boolean, Object, Array, Function, Null, Undefined, DomElement, Unknown,
    // Phase 2 新增
    VecOf(Box<RustType>),
    HashMapOf(Box<RustType>, Box<RustType>),
    OptionOf(Box<RustType>),
    ResultOf(Box<RustType>, Box<RustType>),
    FutureOf(Box<RustType>),
    Tuple(Vec<RustType>),
    Union(Vec<RustType>),
    Named(String),   // 自定义类型名
}
```

### 9.3 codegen.rs — 扩展

```rust
// === crates/web2rust/src/codegen.rs ===

impl RustCodegen {
    // Phase 2 新增

    /// 生成 async fn
    fn emit_async_function(
        &mut self,
        func: &AsyncFunctionInfo,
    );

    /// 生成 struct 定义
    fn emit_class_struct(
        &mut self,
        class: &ClassDefinition,
    );

    /// 生成 impl 块
    fn emit_class_impl(
        &mut self,
        class: &ClassDefinition,
    );

    /// 生成 Promise 链
    fn emit_promise_chain(
        &mut self,
        chain: &PromiseChain,
    );

    /// 生成 Proxy 编译时访问器重写
    fn emit_proxy_rewrite(
        &mut self,
        proxy: &ProxyUsage,
    );

    /// 生成 TypedArray
    fn emit_typed_array(
        &mut self,
        array_type: TypedArrayType,
        elements: &[String],
    );

    /// 生成模块入口
    fn emit_module(
        &mut self,
        module_name: &str,
        exports: &[ModuleExport],
        imports: &[ModuleImport],
    );
}

/// TypedArray 类型
pub enum TypedArrayType {
    Int8Array,
    Uint8Array,
    Uint8ClampedArray,
    Int16Array,
    Uint16Array,
    Int32Array,
    Uint32Array,
    Float32Array,
    Float64Array,
    BigInt64Array,
    BigUint64Array,
}

/// 模块导出项
pub struct ModuleExport {
    pub name: String,
    pub kind: ModuleExportKind,
    pub is_default: bool,
}

pub enum ModuleExportKind {
    Function,
    Class,
    Variable,
    Type,
}

/// 模块导入项
pub struct ModuleImport {
    pub source: String,
    pub bindings: Vec<ImportBinding>,
}

pub enum ImportBinding {
    Default(String),
    Named { original: String, alias: Option<String> },
    Namespace(String),
}
```

### 9.4 web2rust 公开导出更新

```rust
// === crates/web2rust/src/lib.rs ===

// Phase 0/1 已有导出 ...

// Phase 2 新增导出
pub use js::JsCompileCapabilitiesPhase2;
pub use analyzer::{
    PromiseChain, ClassDefinition, ConstructorInfo, MethodInfo,
    ProxyUsage, ProxyTrap, AsyncFunctionInfo, RustType,
};
pub use codegen::{TypedArrayType, ModuleExport, ModuleExportKind, ModuleImport, ImportBinding};

// Phase 3+:
// pub use js::JsCompileCapabilitiesPhase3;
// pub use analyzer::GeneratorInfo;  // function* / yield
```

---

## 十、新增 HTML 元素支持

Phase 2 新增对以下 HTML 元素的解析和 DOM API 支持：

| 元素 | 对应 Rust 类型 | 关键 API |
|------|---------------|----------|
| `<canvas>` | HTMLCanvasElement | getContext('2d'), width, height, toDataURL |
| `<iframe>` | HTMLIFrameElement (Phase 2 stub) | src, srcdoc, sandbox, contentWindow |
| `<script>` | HTMLScriptElement | src, type, async, defer, text |
| `<style>` | HTMLStyleElement | media, type, disabled |
| `<link>` | HTMLLinkElement | href, rel, media, type |
| `<meta>` | HTMLMetaElement | name, content, charset, http_equiv |
| `<base>` | HTMLBaseElement | href, target |

```rust
// === crates/dom/src/html/html_iframe_element.rs ===

/// HTMLIFrameElement — <iframe src="...">
/// Phase 2: stub 实现（不加载嵌套文档）
/// Phase 3+: 完整嵌套浏览上下文
pub struct HTMLIFrameElement {
    pub element: ElementData,
    pub src: String,
    pub srcdoc: String,
    pub sandbox: String,
    pub width: u32,
    pub height: u32,
}
```

```rust
// === crates/dom/src/html/html_script_element.rs ===

/// HTMLScriptElement — <script>
/// Phase 2: web2rust 编译时处理，运行时 stub
pub struct HTMLScriptElement {
    pub element: ElementData,
    pub src: String,
    pub script_type: String,    // "module"/"text/javascript"/"importmap"
    pub is_async: bool,
    pub is_defer: bool,
    pub text: String,           // 内联脚本文本
}
```

---

## 十一、examples/todo_app — TODO 应用示例

### 11.1 项目结构

```
examples/todo_app/
├── build.rs                  # 调用 web2rust 编译源文件
├── Cargo.toml                # 依赖 workspace crates + serde + reqwest
├── index.html                # 标准 HTML5 结构
├── style.css                 # CSS Grid + Transition 样式
├── app.js                    # fetch + localStorage + class + Promise
├── src/
│   └── main.rs               # 通过 include! 引入生成的代码
└── README.md
```

### 11.2 演示特性

| 特性 | 验证能力 |
|------|----------|
| 增加 Todo | DOM 创建 + appendChild |
| 删除 Todo | DOM 移除 + removeChild |
| 标记完成 | classList.toggle + transition |
| localStorage 持久化 | 页面 reload 后数据不丢失 |
| fetch 加载初始数据 | 网络请求 + JSON 解析 |
| CSS Grid 布局 | 响应式多列布局 |
| transition 动画 | 完成/删除时的平滑过渡 |
| 计数器统计 | 活跃/已完成数量实时更新 |

### 11.3 app.js 代码示例

```javascript
// 使用 class + Promise + fetch + localStorage
class TodoApp {
    constructor() {
        this.todos = JSON.parse(localStorage.getItem('todos') || '[]');
        this.init();
    }

    async init() {
        // 首次加载从服务器获取初始数据
        if (this.todos.length === 0) {
            const resp = await fetch('/api/todos');
            this.todos = await resp.json();
            this.save();
        }
        this.render();
    }

    addTodo(text) {
        this.todos.push({ id: Date.now(), text, done: false });
        this.save();
        this.render();
    }

    toggleTodo(id) {
        const todo = this.todos.find(t => t.id === id);
        if (todo) {
            todo.done = !todo.done;
            this.save();
            this.render();
        }
    }

    removeTodo(id) {
        this.todos = this.todos.filter(t => t.id !== id);
        this.save();
        this.render();
    }

    save() {
        localStorage.setItem('todos', JSON.stringify(this.todos));
    }

    render() { /* DOM 操作 ... */ }
}

document.addEventListener('DOMContentLoaded', () => {
    new TodoApp();
});
```

### 11.4 预期 web2rust 编译结果（简化）

```rust
// generated by web2rust — todo_app example
fn run() {
    let window = WebWindow::new("TODO App", 800, 600);
    let doc = window.document();

    // 编译产物：let/const → let, class → struct, async → Future, fetch → net::fetch
    struct TodoApp {
        todos: Vec<TodoItem>,
    }

    #[derive(Clone, Serialize, Deserialize)]
    struct TodoItem {
        id: u64,
        text: String,
        done: bool,
    }

    impl TodoApp {
        fn new() -> Self { /* ... */ }
        async fn init(&mut self) { /* ... */ }
        fn add_todo(&mut self, text: String) { /* ... */ }
        fn toggle_todo(&mut self, id: u64) { /* ... */ }
        fn remove_todo(&mut self, id: u64) { /* ... */ }
        fn save(&self) { /* localStorage.setItem ... */ }
        fn render(&self) { /* DOM 操作 ... */ }
    }

    // document.addEventListener('DOMContentLoaded', ...) →
    // EventDispatcher::dispatch → 直接调用 main
    let mut app = TodoApp::new();
    futures::executor::block_on(app.init());

    window.run();
}
```

---

## 十二、Phase 2 完整调用链路

### 12.1 完整渲染帧（含动画+过渡）

```
Frame Tick (16.67ms @ 60fps)
  │
  ├── MicrotaskQueue::flush()
  │     ├── MutationObserver 回调
  │     └── Promise.then 回调
  │
  ├── AnimationEngine::tick(delta) → Vec<(node_ptr, property, new_value)>
  │     ├── 活跃动画时间推进
  │     ├── 关键帧插值计算
  │     ├── 更新 ComputedStyle
  │     └── 标记脏节点
  │
  ├── TransitionEngine::tick(delta)
  │     ├── 活跃过渡时间推进
  │     ├── 属性值插值计算
  │     ├── 检查过渡完成 → 触发 transitionend 事件
  │     └── 标记脏节点
  │
  ├── AnimationFrameScheduler::tick(timestamp)
  │     └── 执行 requestAnimationFrame 回调（用户代码）
  │           └── 可能产生新的 DOM 变更 → 标记脏节点
  │
  ├── compute_dom_styles (仅脏节点)
  │     ├── cascade.rs 级联计算
  │     ├── CustomPropertyHandler::resolve_all → 解析 var()
  │     └── CascadeCache 缓存查询/更新
  │
  ├── LayoutEngine::partial_layout (仅脏子树)
  │     ├── GridLayout::layout() → taffy Grid
  │     ├── FloatLayout::layout() → 浮动放置 + 环绕计算
  │     ├── margin collapse / clear / BFC
  │     └── 返回 dirty_rects
  │
  ├── DisplayListBuilder::build_dirty (仅脏区域)
  │     ├── Transform 命令生成
  │     ├── Filter 命令生成
  │     ├── GradientFill 命令生成
  │     └── 混合模式标记
  │
  └── WgpuBackend::render
        ├── for each PaintCommand:
        │     ├── Transform → push_transform / pop_transform
        │     ├── Filter → 离屏渲染 + filter_pipeline
        │     ├── GradientFill → gradient_pipeline
        │     ├── (Phase 1 commands: FillRect, Border, Text, BoxShadow, Image, Clip, Opacity)
        │     └── blend_mode → 设置 GPU blend state
        └── queue.submit()
```

### 12.2 Promise/async 编译调用链

```
JS 源文件 (app.js)
  │
  ▼
web2rust::parse_js(js_text)
  ├── swc → swc_ecma_ast::Module
  │
  ▼
JsAnalyzer::analyze(module)
  ├── build_scope_tree → 作用域链
  ├── identify_promise_chains → .then/.catch 链
  ├── identify_async_functions → async fn + await
  ├── identify_class_definitions → struct + impl
  └── infer_complex_types → 类型推导
  │
  ▼
RustCodegen::generate(analysis)
  ├── async function → fn name() -> impl Future<Output = T>
  ├── await expr → .await
  ├── Promise.then(cb) → .then(|val| cb(val))
  ├── Promise.catch(cb) → .map_err(|e| cb(e))
  ├── Promise.all([...]) → futures::future::join_all
  ├── class → struct + impl block
  │
  ▼
generated.rs (输出)
  ├── use futures::Future;
  ├── use net::fetch;
  ├── use storage::LocalStorage;
  │
  └── fn main() { ... }
```

### 12.3 fetch 请求调用链

```
fetch("https://api.example.com/data", { method: "GET" })
  │
  ▼ (编译为)
let resp = net::fetch("https://api.example.com/data", FetchRequest {
    method: HttpMethod::Get,
    ..Default::default()
})?;
  │
  ▼ (运行时)
net::fetch(url, request)
  ├── 解析 URL → scheme, host, port, path
  ├── reqwest::blocking::Client::new()
  │     ├── GET → client.get(url)
  │     ├── 设置 headers
  │     ├── 设置 timeout
  │     └── client.execute() → HttpResponse
  ├── 处理 RedirectMode
  │     ├── Follow: reqwest 自动跟随
  │     ├── Error: 检测 3xx → 返回 FetchError::HttpError
  │     └── Manual: 返回 3xx 响应体
  ├── 读取响应 → FetchResponse { status, headers, body, url, ok }
  └── 返回 Ok(response)
  │
  ▼
resp.json::<MyType>()   → serde_json::from_slice(&resp.body)
resp.text()             → String::from_utf8(resp.body)
```

### 12.4 CSS 动画帧计算

```
@keyframes fadeIn {
    from { opacity: 0; transform: translateY(20px); }
    to   { opacity: 1; transform: translateY(0); }
}

div { animation: fadeIn 0.3s ease-out; }
  │
  ▼ (每帧 tick)
AnimationEngine::tick(delta)
  ├── 查找活跃动画 → name="fadeIn", target_ptr=0x...
  ├── 获取关键帧
  │     from: { opacity: 0, transform: Translate(0, 20) }
  │     to:   { opacity: 1, transform: Translate(0, 0) }
  ├── 计算进度 progress = elapsed / duration = 0.15 / 0.3 = 0.5
  ├── ease-out 缓动 → eased_progress = f(t) = 1 - (1-t)³ = 0.875
  ├── 插值计算
  │     opacity: 0 + (1-0) * 0.875 = 0.875
  │     translateY: 20 + (0-20) * 0.875 = 2.5px
  ├── 更新 ComputedStyle → opacity=0.875, transform=translateY(2.5px)
  └── 标记 node_ptr 为脏 → 触发布局+重绘
```

### 12.5 localStorage 读写调用链

```
localStorage.setItem('todos', JSON.stringify(todos));
  │
  ▼ (编译为)
window.local_storage.set_item("todos", serde_json::to_string(&todos).unwrap());
  │
  ▼ (运行时)
LocalStorage::set_item("todos", "[{\"id\":1,\"text\":\"Buy milk\",\"done\":false}]")
  ├── entries.insert("todos".to_string(), value.clone())
  ├── 检查容量: current_size + value.len() <= max_size
  │     ├── OK → 继续
  │     └── Err → StorageError::QuotaExceeded
  ├── dirty = true
  └── (应用退出时 flush → 写入 JSON 文件)
```

---

## 十三、Phase 2 → Phase 3 扩展点汇总

| 文件位置 | `// Phase 3+` 标记 | 即将新增 |
|----------|-------------------|----------|
| `dom/src/node.rs` | NodeIterator / TreeWalker / Range | 遍历器 + 选择范围 API |
| `dom/src/event.rs` | PointerEvent / TouchEvent | W3C Pointer Events + 触摸事件 |
| `dom/src/event.rs` | composed_path | Shadow DOM 事件跨边界路径 |
| `dom/src/html/html_canvas_element.rs` | 完整 Canvas 2D API | 路径/变换/图案/图片数据导出 |
| `dom/src/html/html_canvas_element.rs` | WebGLRenderingContext | WebGL 上下文 |
| `dom/src/html/html_iframe_element.rs` | 嵌套浏览上下文 | 独立 Document + 沙箱安全 |
| `dom/src/html/html_media_element.rs` | 新文件 | HTMLVideoElement / HTMLAudioElement |
| `css/properties.toml` | mask-* / clip-path / scroll-snap-* / contain / aspect-ratio | ~50 新属性 |
| `css/src/selector.rs` | :has() 选择器 | 性能开销大的后代匹配 |
| `css/src/animations.rs` | scroll-driven animations | ScrollTimeline / ViewTimeline |
| `css/src/values.rs` | parse_clip_path / parse_mask | 裁剪路径 + 遮罩 |
| `layout/src/layout_box.rs` | Ruby / FlowRoot / aspect-ratio | 新 BoxType + 新属性 |
| `layout/src/grid.rs` | Subgrid | CSS Subgrid (Grid Level 2) |
| `paint/src/command.rs` | BackdropFilter | 背景滤镜命令 |
| `render/src/wgpu_backend.rs` | OffscreenRenderer / VideoRenderer | 离屏渲染 + 视频 |
| `render/src/text_renderer.rs` | Emoji / 连字处理 | 复杂字形特性 |
| `net/src/fetch.rs` | fetch_async / Server-Sent Events | 真正异步 fetch + EventSource |
| `net/src/websocket.rs` | 自动重连 / 心跳 / 压缩扩展 | 生产级 WebSocket |
| `net/src/webrtc.rs` | 新文件 | WebRTC (RTCPeerConnection) |
| `storage/src/indexed_db.rs` | 新文件 | IndexedDB 实现 |
| `storage/src/cache_storage.rs` | 新文件 | Cache API (Service Worker) |
| `web2rust/src/analyzer.rs` | Generator / yield | function* / Generator 编译 |
| `web2rust/src/codegen.rs` | dynamic import() | 运行时动态模块加载 |
| `web2rust/src/codegen.rs` | Proxy 完整运行时 | 所有 13 种 trap |
| `runtime/src/window.rs` | ResizeObserver / IntersectionObserver | 新观察器 API |
| 跨平台 target | wasm32 / aarch64-apple-ios / aarch64-linux-android | WASM + iOS + Android |

---

## 统计

| 指标 | Phase 1 | Phase 2 | 增量 |
|------|---------|---------|------|
| 代码量（预估） | ~17,000 行 | ~21,500 行 | +4,500 行 |
| CSS 属性 | 80 个 | 200+ 个 | +120+ 个 |
| 公开 API 数量 | ~200 个 | ~350 个 | +~150 个 |
| crate 数量 | 8 个 | 10 个 | +2 (net, storage) |
| crate 模块文件数 | 37 个 | 52 个 | +15 个 |
| 测试预估 | ~300 个 | ~500 个 | +~200 个 |
| JS 编译能力 | 20+ 种 | 50+ 种 | +30 种 |
| GPU 管线 | 4 个 | 6 个 | +2 (gradient, filter) |
| 新增 HTML 元素 | - | 7 个 | Canvas/Input/Image/... |
| Examples | 4 个 | 5 个 | +1 (todo_app) |
