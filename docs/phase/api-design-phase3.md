# Rust 浏览器引擎 — Phase 3 API 设计文档

> 本文档定义 Phase 3 所有公开类型、函数签名及作用。
> 中文注解标注功能说明，`// Phase 4+` 标记为后续阶段预留的扩展点。
> Phase 3 在 Phase 2 基础上新增约 12,000 行代码，DOM API 从 115 扩展到 151 个，
> CSS 从 171 扩展到 218 个，JS/Web API 从 142 扩展到 258 个，新增 3 个目标平台。

---

## 一、模块总览

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Phase 3 模块依赖关系                                                      │
│                                                                           │
│  ★ 跨平台运行时（核心新增）                                                  │
│                                                                           │
│  runtime (主循环 + 跨平台 Window trait + Observer 管理器)                    │
│       │                                                                   │
│       ├── net ────── fetch_async / WebSocket 自动重连+心跳                  │
│       │                                                                   │
│       ├── render ─── WebGpuBackend (WASM)  ← 新增                          │
│       │       │                                                           │
│       │       ├── paint (DisplayList + Canvas 2D 命令)                     │
│       │       │                                                           │
│       │       ├── layout (aspect-ratio 约束)                               │
│       │       │       │                                                   │
│       │       │       ├── css (组合器/属性选择器/:has/:is/3D/clip-path)      │
│       │       │       │       │                                           │
│       │       │       │       └── dom (PointerEvent/TouchEvent/Observer)   │
│       │       │       │                                                   │
│       │       │       └── dom (同上)                                       │
│       │       │                                                           │
│       │       └── dom (同上)                                               │
│       │                                                                   │
│       ├── storage ── (Phase 2 已完成，Phase 3 无新增)                       │
│       │                                                                   │
│       └── web2rust ── SPA 路由 / Object 完整支持 / Canvas 2D / Date ...    │
└──────────────────────────────────────────────────────────────────────────┘
```

### Phase 3 实现原则

1. **跨平台优先**：新增 wasm/ios/android 三平台，Window trait 统一抽象所有平台
2. **够用就好**：只覆盖 80% 业务场景，不追求 W3C 完整性
3. **SPA 路由**：History + Location + URL API 三位一体，单页应用核心依赖
4. **触控统一**：PointerEvent 统一鼠标+触控+笔输入，TouchEvent 保留多点触控
5. **增量重排**：脏区域追踪，布局性能从 O(n) 降至 O(k)
6. **W3C 命名**：函数名与浏览器标准 DOM API 保持一致
7. **中文注解**：所有文档注释使用中文

---

## 二、dom crate — DOM 扩展

### 2.1 模块结构

```
dom/
├── Cargo.toml
├── src/
│   ├── lib.rs                       # 公开导出更新
│   ├── node.rs                      # has_child_nodes / is_same_node
│   ├── element.rs                   # children / ElementChild 遍历 / closest / getBoundingClientRect
│   ├── document.rs                  # cookie
│   ├── event.rs                     # TouchEvent / PointerEvent / WheelEvent / CustomEvent / FocusEvent扩展
│   ├── html/
│   │   ├── mod.rs                   # 模块导出
│   │   ├── html_video_element.rs    # NEW
│   │   ├── html_audio_element.rs    # NEW
│   │   ├── html_meta_element.rs     # NEW
│   │   └── html_link_element.rs     # NEW
│   └── observer/
│       ├── mod.rs                   # 模块导出
│       ├── resize_observer.rs       # NEW
│       └── intersection_observer.rs # NEW
```

### 2.2 Node — hasChildNodes / isSameNode

```rust
// === crates/dom/src/node.rs ===

impl Node {
    // Phase 3 新增

    /// 判断是否有子节点
    /// 等价于 child_nodes.len() > 0
    pub fn has_child_nodes(&self) -> bool;

    /// 判断两个节点引用是否为同一个节点（Rc 指针比较）
    /// React/Vue diff 算法核心，框架常用
    pub fn is_same_node(&self, other: &Rc<RefCell<Node>>) -> bool;
}
```

### 2.3 ElementData — children 遍历 / closest / getBoundingClientRect

```rust
// === crates/dom/src/element.rs ===

impl ElementData {
    // Phase 3 新增

    // ===== ElementChild 遍历 =====

    /// 获取仅包含元素类型的子节点（不含文本/注释节点）
    pub fn children(&self) -> Vec<Rc<RefCell<Node>>>;

    /// 第一个元素子节点
    pub fn first_element_child(&self) -> Option<Rc<RefCell<Node>>>;

    /// 最后一个元素子节点
    pub fn last_element_child(&self) -> Option<Rc<RefCell<Node>>>;

    /// 下一个元素兄弟节点
    pub fn next_element_sibling(&self) -> Option<Rc<RefCell<Node>>>;

    /// 上一个元素兄弟节点
    pub fn previous_element_sibling(&self) -> Option<Rc<RefCell<Node>>>;

    // ===== 选择器查询 =====

    /// 向上查找匹配选择器的最近祖先（含自身）
    /// 事件委托核心：e.target.closest('.item')
    pub fn closest(&self, selector: &str) -> Option<Rc<RefCell<Node>>>;

    // ===== 布局信息 =====

    /// 获取元素边界矩形（相对于 viewport）
    /// 返回 { x, y, width, height, top, right, bottom, left }
    pub fn get_bounding_client_rect(&self) -> Rect;

    // ===== HTML 序列化 =====

    /// 完整 innerHTML 序列化（Phase 2 stub → Phase 3 完整实现）
    /// 含属性转义、自闭合标签处理
    pub fn inner_html(&self) -> String;
    pub fn set_inner_html(&mut self, html: &str);
}

/// 边界矩形
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn top(&self) -> f32 { self.y }
    pub fn right(&self) -> f32 { self.x + self.width }
    pub fn bottom(&self) -> f32 { self.y + self.height }
    pub fn left(&self) -> f32 { self.x }
}
```

### 2.4 Document — cookie

```rust
// === crates/dom/src/document.rs ===

impl Document {
    // Phase 3 新增

    /// 读取/设置 Cookie
    /// 格式: "key1=value1; key2=value2"
    pub fn cookie(&self) -> String;
    pub fn set_cookie(&mut self, cookie_str: &str);
}
```

### 2.5 TouchEvent / Touch / TouchList — 触控事件

```rust
// === crates/dom/src/event.rs ===

// ============================================================
//  Touch / TouchList / TouchEvent —— 多点触控 (Phase 3 新增)
// ============================================================

/// 单个触控点
#[derive(Debug, Clone)]
pub struct Touch {
    /// 触控点唯一标识（手指按下到抬起不变）
    pub identifier: i32,
    /// 相对视口的坐标
    pub client_x: f32,
    pub client_y: f32,
    /// 相对页面的坐标
    pub page_x: f32,
    pub page_y: f32,
    /// 相对屏幕的坐标
    pub screen_x: f32,
    pub screen_y: f32,
    /// 触控目标元素
    pub target: Option<Rc<RefCell<Node>>>,
    /// 压力值 (0.0 ~ 1.0)
    pub force: f32,
    /// 接触半径
    pub radius_x: f32,
    pub radius_y: f32,
}

/// 触控点列表
#[derive(Debug, Clone)]
pub struct TouchList {
    touches: Vec<Touch>,
}

impl TouchList {
    pub fn new() -> Self;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn item(&self, index: usize) -> Option<&Touch>;
    pub fn iter(&self) -> impl Iterator<Item = &Touch>;
}

/// 触控事件
pub struct TouchEvent {
    pub event: Event,
    /// 当前屏幕上的所有触控点
    pub touches: TouchList,
    /// 当前元素上的触控点
    pub target_touches: TouchList,
    /// 引发事件的变更触控点
    pub changed_touches: TouchList,
    /// 是否按住 alt/ctrl/shift/meta
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub meta_key: bool,
}

impl TouchEvent {
    /// event_type: "touchstart" / "touchmove" / "touchend" / "touchcancel"
    pub fn new(
        event_type: &str,
        touches: TouchList,
        target_touches: TouchList,
        changed_touches: TouchList,
    ) -> Self;
}
```

### 2.6 PointerEvent — 统一指针事件

```rust
// === crates/dom/src/event.rs ===

// ============================================================
//  PointerEvent —— 统一鼠标+触控+笔 (Phase 3 新增)
// ============================================================

/// 指针事件 —— 对应 W3C PointerEvent 接口
/// 跨平台统一输入模型，替代分别处理 MouseEvent + TouchEvent
pub struct PointerEvent {
    pub event: Event,

    // ===== 指针标识 =====
    /// 指针唯一 ID（同一次按下到抬起不变）
    pub pointer_id: i32,
    /// 指针类型: "mouse" / "touch" / "pen"
    pub pointer_type: PointerType,

    // ===== 坐标 =====
    pub client_x: f32,
    pub client_y: f32,
    pub page_x: f32,
    pub page_y: f32,
    pub screen_x: f32,
    pub screen_y: f32,

    // ===== 压力与接触 =====
    /// 压力 0.0~1.0（鼠标按下=0.5，否则=0）
    pub pressure: f32,
    /// 接触宽度/高度（触控/笔）
    pub width: f32,
    pub height: f32,
    /// 笔的倾斜角度 (-90 ~ 90)
    pub tilt_x: f32,
    pub tilt_y: f32,

    // ===== 修饰键 =====
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub meta_key: bool,

    // ===== 按钮 =====
    /// 0=主按钮(左手), 1=滚轮, 2=右键
    pub button: i16,
    /// 按位掩码表示哪些按钮被按下
    pub buttons: u16,

    /// 是否为触控/笔的主指针
    pub is_primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerType {
    Mouse,
    Touch,
    Pen,
}

impl PointerEvent {
    /// event_type: "pointerdown" / "pointermove" / "pointerup" / "pointercancel"
    pub fn new(
        event_type: &str,
        pointer_id: i32,
        pointer_type: PointerType,
        client_x: f32,
        client_y: f32,
    ) -> Self;

    /// 从 MouseEvent 转换（桌面端兼容）
    pub fn from_mouse_event(event: &MouseEvent, pointer_id: i32) -> Self;

    /// 从 Touch 转换（移动端兼容）
    pub fn from_touch(touch: &Touch, pointer_id: i32, event_type: &str) -> Self;
}
```

### 2.7 WheelEvent 完善 & CustomEvent

```rust
// === crates/dom/src/event.rs ===

// ============================================================
//  WheelEvent 完善 (Phase 3)
// ============================================================

impl WheelEvent {
    /// 滚轮事件的 delta 模式
    pub fn delta_mode(&self) -> DeltaMode;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeltaMode {
    /// 像素
    Pixel = 0,
    /// 行
    Line = 1,
    /// 页
    Page = 2,
}

// ============================================================
//  CustomEvent —— 自定义事件 (Phase 3 新增)
// ============================================================

/// 自定义事件 —— 携带任意 detail 数据
pub struct CustomEvent {
    pub event: Event,
    /// 附加数据
    pub detail: serde_json::Value,
}

impl CustomEvent {
    pub fn new(event_type: &str, detail: serde_json::Value) -> Self;
}
```

### 2.8 新增鼠标/焦点/表单/剪贴板事件

```rust
// === crates/dom/src/event.rs ===

// ============================================================
//  事件类型枚举扩展 (Phase 3)
// ============================================================

/// 事件类型枚举 —— 新增
pub enum EventType {
    // ...Phase 0/1/2 已有 ...

    // 鼠标扩展
    DblClick,
    MouseEnter,
    MouseLeave,
    MouseOver,
    MouseOut,
    ContextMenu,

    // 焦点扩展
    FocusIn,
    FocusOut,

    // 表单
    Submit,
    Reset,
    Change,

    // 触控
    TouchStart,
    TouchMove,
    TouchEnd,
    TouchCancel,

    // 指针
    PointerDown,
    PointerMove,
    PointerUp,
    PointerCancel,

    // 剪贴板
    Copy,
    Cut,
    Paste,

    // 自定义
    Custom(String),
}
```

### 2.9 HTML 元素 — Video / Audio / Meta / Link

```rust
// === crates/dom/src/html/html_video_element.rs ===

/// HTMLVideoElement —— <video src="...">
pub struct HTMLVideoElement {
    pub element: ElementData,

    pub src: String,
    pub width: u32,
    pub height: u32,
    pub poster: String,
    pub controls: bool,
    pub autoplay: bool,
    pub loop_: bool,
    pub muted: bool,

    // 播放状态
    pub current_time: f64,
    pub duration: f64,
    pub volume: f64,       // 0.0 ~ 1.0
    pub playback_rate: f64, // 1.0 = 正常速度
    pub paused: bool,
    pub ended: bool,
}

impl HTMLVideoElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;

    /// 播放控制
    pub fn play(&mut self);
    pub fn pause(&mut self);

    /// 加载视频资源
    pub fn load(&mut self);

    /// 请求全屏（平台相关）
    pub fn request_fullscreen(&mut self);
}

// === crates/dom/src/html/html_audio_element.rs ===

/// HTMLAudioElement —— <audio src="...">
pub struct HTMLAudioElement {
    pub element: ElementData,

    pub src: String,
    pub controls: bool,
    pub autoplay: bool,
    pub loop_: bool,
    pub muted: bool,

    pub current_time: f64,
    pub duration: f64,
    pub volume: f64,
    pub paused: bool,
}

impl HTMLAudioElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;
    pub fn play(&mut self);
    pub fn pause(&mut self);
    pub fn load(&mut self);
}

// === crates/dom/src/html/html_meta_element.rs ===

/// HTMLMetaElement —— <meta charset="utf-8">
pub struct HTMLMetaElement {
    pub element: ElementData,
    pub name: String,       // "viewport", "description", etc.
    pub content: String,
    pub charset: String,
}

impl HTMLMetaElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;
}

// === crates/dom/src/html/html_link_element.rs ===

/// HTMLLinkElement —— <link rel="stylesheet" href="...">
pub struct HTMLLinkElement {
    pub element: ElementData,
    pub rel: String,
    pub href: String,
    pub media: String,
    pub disabled: bool,
}

impl HTMLLinkElement {
    pub fn from_node(node: &Rc<RefCell<Node>>) -> Option<Self>;
}
```

### 2.10 ResizeObserver

```rust
// === crates/dom/src/observer/resize_observer.rs ===

/// ResizeObserver —— 元素尺寸变化监听
///
/// 响应式布局核心，没有它很多组件写不了。
///
/// 使用示例：
/// ```ignore
/// let observer = ResizeObserver::new(Box::new(|entries| {
///     for entry in entries {
///         println!("{} resized: {}x{}", entry.target.borrow().tag_name(), entry.content_rect.width, entry.content_rect.height);
///     }
/// }));
/// observer.observe(&element);
/// ```
pub struct ResizeObserver {
    callback: Box<dyn Fn(&[ResizeObserverEntry])>,
    observed: HashMap<usize, (Rc<RefCell<Node>>, Rect)>,
}

/// 尺寸变化条目
#[derive(Debug, Clone)]
pub struct ResizeObserverEntry {
    /// 被观察的目标元素
    pub target: Rc<RefCell<Node>>,
    /// 新尺寸
    pub content_rect: Rect,
    /// 边框盒尺寸
    pub border_box_size: (f32, f32),
    /// 内容盒尺寸
    pub content_box_size: (f32, f32),
}

impl ResizeObserver {
    /// 创建观察器
    /// callback: 尺寸变化时调用，参数为所有变化的条目
    pub fn new(callback: Box<dyn Fn(&[ResizeObserverEntry])>) -> Self;

    /// 开始观察元素
    pub fn observe(&mut self, target: &Rc<RefCell<Node>>);

    /// 停止观察元素
    pub fn unobserve(&mut self, target: &Rc<RefCell<Node>>);

    /// 停止所有观察
    pub fn disconnect(&mut self);

    /// 检查变化并触发回调（运行时每帧调用）
    pub(crate) fn poll(&mut self);
}
```

### 2.11 IntersectionObserver

```rust
// === crates/dom/src/observer/intersection_observer.rs ===

/// IntersectionObserver —— 元素可见性监听
///
/// 懒加载、无限滚动、曝光埋点的基石。
///
/// 使用示例：
/// ```ignore
/// let observer = IntersectionObserver::new(Box::new(|entries| {
///     for entry in entries {
///         if entry.is_intersecting {
///             // 加载图片或上报曝光
///         }
///     }
/// }), IntersectionObserverOptions { threshold: 0.5, ..Default::default() });
/// observer.observe(&element);
/// ```
pub struct IntersectionObserver {
    callback: Box<dyn Fn(&[IntersectionObserverEntry])>,
    options: IntersectionObserverOptions,
    observed: HashMap<usize, Rc<RefCell<Node>>>,
    /// 根元素（默认=viewport）
    root: Option<Rc<RefCell<Node>>>,
}

#[derive(Debug, Clone)]
pub struct IntersectionObserverOptions {
    /// 触发回调的可见比例阈值（0.0 ~ 1.0）
    pub threshold: f32,
    /// 根元素的外边距（扩大/缩小判定区域）
    pub root_margin: (f32, f32, f32, f32), // top, right, bottom, left
}

impl Default for IntersectionObserverOptions {
    fn default() -> Self {
        Self {
            threshold: 0.0,
            root_margin: (0.0, 0.0, 0.0, 0.0),
        }
    }
}

/// 可见性变化条目
#[derive(Debug, Clone)]
pub struct IntersectionObserverEntry {
    pub target: Rc<RefCell<Node>>,
    /// 目标与根元素的交叉比例 (0.0 ~ 1.0)
    pub intersection_ratio: f32,
    /// 是否与根元素相交
    pub is_intersecting: bool,
    /// 交叉区域的边界矩形
    pub intersection_rect: Rect,
    /// 目标的边界矩形
    pub bounding_client_rect: Rect,
    /// 根元素的边界矩形
    pub root_bounds: Rect,
    /// 交叉发生的时间戳
    pub time: f64,
}

impl IntersectionObserver {
    pub fn new(
        callback: Box<dyn Fn(&[IntersectionObserverEntry])>,
        options: IntersectionObserverOptions,
    ) -> Self;

    pub fn observe(&mut self, target: &Rc<RefCell<Node>>);
    pub fn unobserve(&mut self, target: &Rc<RefCell<Node>>);
    pub fn disconnect(&mut self);

    /// 计算交叉比并触发回调（运行时每帧调用）
    pub(crate) fn poll(&mut self, viewport: Rect);
}
```

---

## 三、css crate — CSS 引擎扩展

### 3.1 模块结构

```
css/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── selector.rs          # 组合器/属性选择器/:has/:is/:where/::before/::after
│   ├── stylesheet.rs        # @import 支持
│   ├── values.rs            # min/max/clamp/currentColor/3D transforms/clip-path shapes
│   ├── properties.rs        # aspect-ratio/contain/content-visibility 等新属性
│   └── cascade.rs           # 组合器匹配更新
```

### 3.2 选择器引擎 — 组合器

```rust
// === crates/css/src/selector.rs ===

/// 组合器类型 (Phase 3 新增)
#[derive(Debug, Clone, PartialEq)]
pub enum Combinator {
    /// 后代: A B
    Descendant,
    /// 子代: A > B
    Child,
    /// 相邻兄弟: A + B
    AdjacentSibling,
    /// 通用兄弟: A ~ B
    GeneralSibling,
}

/// 复合选择器片段（带组合器）
#[derive(Debug, Clone)]
pub struct SelectorSegment {
    pub parts: Vec<SelectorPart>,
    pub combinator: Combinator, // 与上一段的连接方式（第一段为 Descendant，等同于后代）
}

/// 选择器解析结果（Phase 3 升级为多段）
#[derive(Debug, Clone)]
pub struct ParsedSelector {
    pub segments: Vec<SelectorSegment>,
}

// Phase 3: SelectorEngine 升级

impl SelectorEngine {
    /// 匹配复合选择器（含组合器）
    /// 例: "div.container > span.active"
    pub fn matches_complex(
        &self,
        element: &Rc<RefCell<dom::Node>>,
        selector: &ParsedSelector,
    ) -> bool;
}
```

### 3.3 选择器引擎 — 属性选择器（6 个）

```rust
// === crates/css/src/selector.rs ===

/// 属性选择器操作符 (Phase 3 新增)
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeOp {
    /// [attr] — 属性存在
    Exists,
    /// [attr=value] — 完全匹配
    Equals(String),
    /// [attr~=value] — 空格分隔的词列表中包含
    Contains(String),
    /// [attr^=value] — 前缀匹配
    StartsWith(String),
    /// [attr$=value] — 后缀匹配
    EndsWith(String),
    /// [attr*=value] — 包含子串
    ContainsSubstring(String),
}

/// 选择器片段扩展
pub enum SelectorPart {
    Tag(String),
    Class(String),
    Id(String),
    /// 属性选择器 (Phase 3 新增)
    Attribute { name: String, op: AttributeOp },
    PseudoClass(PseudoClass),
}
```

### 3.4 选择器引擎 — 伪类 (:has / :is / :where / :focus-visible / :focus-within)

```rust
// === crates/css/src/selector.rs ===

/// 伪类枚举扩展 (Phase 3 新增)
pub enum PseudoClass {
    // ...Phase 2 已有: Hover, Active, Focus, Visited, Link, Root, Empty,
    //    FirstChild, LastChild, OnlyChild, FirstOfType, LastOfType, OnlyOfType,
    //    NthChild, NthLastChild, NthOfType, NthLastOfType, Not, Enabled, Disabled, Checked ...

    // Phase 3 新增: 关系型伪类
    /// :has(selector) — 父元素匹配，包含匹配子元素的父元素
    /// 例: div:has(> img) — 包含直接子 img 的 div
    Has(Box<ParsedSelector>),
    /// :is(selectors) — 匹配列表中任一选择器
    /// 例: :is(h1, h2, h3) span — 等价于 h1 span, h2 span, h3 span
    Is(Vec<ParsedSelector>),
    /// :where(selectors) — 同 :is() 但特异性始终为 0
    Where(Vec<ParsedSelector>),

    // Phase 3 新增: 焦点扩展
    /// :focus-visible — 仅键盘焦点（鼠标点击获取焦点不匹配）
    FocusVisible,
    /// :focus-within — 自身或任意后代有焦点
    FocusWithin,
}

// 伪类匹配逻辑
fn matches_pseudo_class_node(
    node: &Rc<RefCell<dom::Node>>,
    pc: &PseudoClass,
) -> bool {
    match pc {
        // Phase 3 新增
        PseudoClass::Has(selector) => {
            // 检查是否存在匹配的子元素
            let n = node.borrow();
            for child in &n.child_nodes() {
                if selector_matches_complex(child, selector) {
                    return true;
                }
            }
            false
        }
        PseudoClass::Is(selectors) => {
            selectors.iter().any(|s| selector_matches_complex(node, s))
        }
        PseudoClass::Where(selectors) => {
            // 与 :is() 相同，但特异性为 0（由 cascade 层处理）
            selectors.iter().any(|s| selector_matches_complex(node, s))
        }
        PseudoClass::FocusVisible => {
            // 仅当焦点是通过键盘获得的
            node_is_focused(node) && !focus_via_pointer(node)
        }
        PseudoClass::FocusWithin => {
            // 自身或后代有焦点
            node_is_focused(node) || has_focused_descendant(node)
        }
        // ... Phase 2 已有的匹配逻辑 ...
    }
}
```

### 3.5 伪元素 — ::before / ::after

```rust
// === crates/css/src/selector.rs ===

/// 伪元素类型 (Phase 3 新增)
#[derive(Debug, Clone, PartialEq)]
pub enum PseudoElement {
    Before,
    After,
}

/// 选择器解析：支持伪元素后缀
/// 例: div.container::before
pub fn parse_selector(selector: &str) -> (ParsedSelector, Option<PseudoElement>);

/// 为伪元素生成虚拟 DOM 节点
/// ::before → 插入到 content 属性指定的文本节点
/// ::after  → 追加到 content 属性指定的文本节点
pub fn create_pseudo_element_node(
    element: &Rc<RefCell<dom::Node>>,
    pseudo: &PseudoElement,
    content: &str,
) -> Rc<RefCell<dom::Node>>;
```

### 3.6 新增 CSS 属性

```rust
// === crates/css/src/properties.rs ===

// ============================================================
//  布局属性
// ============================================================

/// aspect-ratio: 宽高比约束
/// 值: auto | <ratio>
/// 例: aspect-ratio: 16/9
pub fn parse_aspect_ratio(value: &str) -> CSSValue;

/// contain: 渲染隔离
/// 值: none | strict | content | [size || layout || style || paint]
/// 例: contain: layout paint
pub fn parse_contain(value: &str) -> CSSValue;

/// content-visibility: 可见性渲染控制
/// 值: visible | auto | hidden
pub fn parse_content_visibility(value: &str) -> CSSValue;

// ============================================================
//  盒模型属性
// ============================================================

/// outline-offset: outline 偏移距离
/// 值: <length>
pub fn parse_outline_offset(value: &str) -> CSSValue;

// ============================================================
//  排版属性
// ============================================================

/// font-variant: 字体变体
/// 值: normal | small-caps | ...
pub fn parse_font_variant(value: &str) -> CSSValue;

/// font-stretch: 字体拉伸
/// 值: normal | ultra-condensed | ... | ultra-expanded | <percentage>
pub fn parse_font_stretch(value: &str) -> CSSValue;

/// word-break: 断词规则
/// 值: normal | break-all | keep-all | break-word
pub fn parse_word_break(value: &str) -> CSSValue;

/// overflow-wrap: 溢出换行
/// 值: normal | anywhere | break-word
pub fn parse_overflow_wrap(value: &str) -> CSSValue;

// ============================================================
//  变换属性 — 3D
// ============================================================

/// transform-style: 3D 空间上下文
/// 值: flat | preserve-3d
pub fn parse_transform_style(value: &str) -> CSSValue;

/// perspective: 3D 透视距离
/// 值: none | <length>
pub fn parse_perspective(value: &str) -> CSSValue;

/// perspective-origin: 透视原点
/// 值: <position>
pub fn parse_perspective_origin(value: &str) -> CSSValue;

/// backface-visibility: 背面可见性
/// 值: visible | hidden
pub fn parse_backface_visibility(value: &str) -> CSSValue;

// ============================================================
//  交互属性
// ============================================================

/// touch-action: 触控行为控制（移动端关键）
/// 值: auto | none | pan-x | pan-y | pinch-zoom | manipulation
pub fn parse_touch_action(value: &str) -> CSSValue;

/// will-change: GPU 合成层提示
/// 值: auto | transform | opacity | scroll-position | contents | <custom>
pub fn parse_will_change(value: &str) -> CSSValue;
```

### 3.7 CSS 值扩展

```rust
// === crates/css/src/values.rs ===

// ============================================================
//  CSS 数学函数: min() / max() / clamp()
// ============================================================

impl CSSValue {
    /// 解析 CSS 函数值
    /// 新增支持: min(a, b, ...), max(a, b, ...), clamp(min, ideal, max)
    pub fn parse_function(name: &str, args: &str) -> CSSValue;
}

// ============================================================
//  3D Transform 函数 (10 个)
// ============================================================

/// Transform 函数枚举扩展
pub enum TransformFunction {
    // Phase 1 已有: Matrix, Translate, Rotate, Scale, Skew ...

    /// matrix3d(a1, a2, a3, a4, b1, b2, b3, b4, c1, c2, c3, c4, d1, d2, d3, d4)
    Matrix3d([f32; 16]),
    /// translateZ(z)
    TranslateZ(f32),
    /// translate3d(x, y, z)
    Translate3d(f32, f32, f32),
    /// rotateX(angle)
    RotateX(f32),
    /// rotateY(angle)
    RotateY(f32),
    /// rotateZ(angle) — 等价于 rotate(angle)
    RotateZ(f32),
    /// rotate3d(x, y, z, angle)
    Rotate3d(f32, f32, f32, f32),
    /// scaleZ(z)
    ScaleZ(f32),
    /// scale3d(x, y, z)
    Scale3d(f32, f32, f32),
    /// perspective(d)
    Perspective(f32),
}

// ============================================================
//  Clip Path 形状函数 (4 个)
// ============================================================

pub enum ClipPathShape {
    /// circle(r at x y)
    Circle { radius: f32, center: (f32, f32) },
    /// ellipse(rx ry at x y)
    Ellipse { rx: f32, ry: f32, center: (f32, f32) },
    /// polygon(x1 y1, x2 y2, ...)
    Polygon(Vec<(f32, f32)>),
    /// inset(top right bottom left round rx ry)
    Inset {
        top: f32,
        right: f32,
        bottom: f32,
        left: f32,
        border_radius: Option<(f32, f32)>,
    },
}

// ============================================================
//  Timing 函数
// ============================================================

pub enum TimingFunction {
    // Phase 2 已有
    Ease, EaseIn, EaseOut, EaseInOut, Linear,
    // Phase 3 新增
    CubicBezier(f32, f32, f32, f32), // cubic-bezier(x1, y1, x2, y2)
    Steps(u32, StepDirection),       // steps(n, start|end)
}

pub enum StepDirection { Start, End }

// ============================================================
//  其他
// ============================================================

/// currentColor 关键字
/// 已在 CSS 级联中作为特殊处理，解析为当前元素的 color 值
// pub fn resolve_current_color(element: &ElementData) -> Color;

/// radial-gradient()
pub fn parse_radial_gradient(value: &str) -> CSSValue;

/// backdrop-filter
/// 解析逻辑同 filter 属性，但应用于元素背后的区域
pub fn parse_backdrop_filter(value: &str) -> CSSValue;
```

### 3.8 @import 支持

```rust
// === crates/css/src/stylesheet.rs ===

/// @import 规则 (Phase 3 新增)
#[derive(Debug, Clone)]
pub struct ImportRule {
    /// 导入路径
    pub url: String,
    /// 媒体查询条件（可选）
    pub media: Option<String>,
}

impl StyleSheet {
    /// Phase 3: 支持 @import 递归加载
    /// 解析样式表中的 @import 规则并递归加载
    pub fn resolve_imports(&mut self, base_path: &str) -> Result<(), ImportError>;
}

pub enum ImportError {
    NotFound(String),
    CircularReference(String),
    ParseError(String),
}
```

### 3.9 特异性计算更新

```rust
// === crates/css/src/selector.rs ===

/// 特异性计算更新（Phase 3）
///
/// :is() 和 :not() 使用内部最特异的选择器的特异性
/// :where() 和 :has() 的特异性始终为 0
pub fn compute_specificity(selector: &ParsedSelector) -> (u32, u32, u32) {
    // (ID数, Class数, Tag数)
    // Attribute 选择器计入 Class 级
    // :where() 不计特异性
    // :is()/:not() 取内部最大值
    // ::before/::after 不计特异性
}
```

---

## 四、layout crate — 布局扩展

### 4.1 aspect-ratio 约束

```rust
// === crates/layout/src/layout.rs ===

impl LayoutEngine {
    /// 应用 aspect-ratio 约束
    ///
    /// 当只设置 width 或 height 之一时，根据 aspect-ratio 推导另一维度。
    /// 自动值: aspect-ratio: auto 从内容（图片原始尺寸）推导。
    fn apply_aspect_ratio(
        &self,
        box_: &mut LayoutBox,
        style: &ComputedStyle,
    ) {
        if let Some(ratio) = style.aspect_ratio() {
            let (w, h) = ratio;
            if style.width.is_some() && style.height.is_none() {
                box_.rect.height = box_.rect.width * h / w;
            } else if style.height.is_some() && style.width.is_none() {
                box_.rect.width = box_.rect.height * w / h;
            }
        }
    }
}
```

---

## 五、render crate — WebGPU WASM 后端

### 5.1 webgpu_backend.rs

```rust
// === crates/render/src/webgpu_backend.rs ===

/// WebGPU WASM 渲染后端
///
/// WASM 平台没有 wgpu native，必须通过 web-sys 调用浏览器 WebGPU API。
/// 与原生 wgpu 后端共享相同的 shader 和渲染管线逻辑。
pub struct WebGpuBackend {
    /// web-sys GPUDevice
    device: web_sys::GpuDevice,
    /// web-sys GPUQueue
    queue: web_sys::GpuQueue,
    /// 渲染管线
    pipeline: web_sys::GpuRenderPipeline,
    /// 交换链格式
    format: web_sys::GpuTextureFormat,
    /// 顶点缓冲区
    vertex_buffer: web_sys::GpuBuffer,
    /// 屏幕尺寸
    width: u32,
    height: u32,
}

impl WebGpuBackend {
    /// 从 HTMLCanvasElement 初始化 WebGPU 上下文
    pub async fn new(
        canvas: &web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Result<Self, GpuError>;

    /// 渲染一帧 DisplayList
    /// 与 wgpu 后端使用相同的命令生成逻辑
    pub fn render(&mut self, display_list: &DisplayList);

    /// 更新屏幕尺寸
    pub fn resize(&mut self, width: u32, height: u32);

    /// 获取 web-sys GPUDevice 引用
    pub fn device(&self) -> &web_sys::GpuDevice;
}

pub enum GpuError {
    NoAdapter,
    NoDevice,
    NoCanvasContext,
    ShaderCompileError(String),
}
```

---

## 六、runtime crate — 跨平台运行时

### 6.1 模块结构

```
runtime/
├── Cargo.toml                    # 新增 cfg 条件编译
├── src/
│   ├── lib.rs                    # Platform 枚举 + Window trait 导出
│   ├── window.rs                 # Window trait 定义
│   ├── native.rs                 # winit + wgpu (已有，重命名)
│   ├── wasm.rs                   # NEW: wasm32 + WebGPU
│   ├── ios.rs                    # NEW: aarch64-apple-ios
│   ├── android.rs                # NEW: aarch64-linux-android
│   └── observer_manager.rs       # NEW: Resize/IntersectionObserver 驱动
```

### 6.2 Window trait 统一抽象

```rust
// === crates/runtime/src/window.rs ===

/// 跨平台 Window 抽象
pub trait Window {
    /// 窗口逻辑尺寸（像素）
    fn size(&self) -> (f32, f32);

    /// DPI 缩放因子
    fn scale_factor(&self) -> f32;

    /// 设置窗口标题
    fn set_title(&mut self, title: &str);

    /// 请求下一帧重绘
    fn request_redraw(&self);

    /// 获取当前平台
    fn platform(&self) -> Platform;

    /// 窗口是否已关闭
    fn is_closed(&self) -> bool;
}

/// 目标平台枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platform {
    /// 桌面 (Windows/macOS/Linux)
    Desktop,
    /// 浏览器 WASM
    Wasm,
    /// iOS
    IOS,
    /// Android
    Android,
}

impl Platform {
    /// 是否为移动端平台
    pub fn is_mobile(&self) -> bool {
        matches!(self, Platform::IOS | Platform::Android)
    }

    /// 是否为触摸优先平台
    pub fn is_touch_primary(&self) -> bool {
        matches!(self, Platform::IOS | Platform::Android)
    }
}
```

### 6.3 WASM 运行时

```rust
// === crates/runtime/src/wasm.rs === (wasm32-unknown-unknown)

/// WASM 运行时窗口
///
/// 通过 web-sys 绑定浏览器 API：
/// - 事件循环：requestAnimationFrame
/// - 输入：web-sys 鼠标/键盘/触控事件
/// - 渲染：WebGpuBackend
pub struct WasmWindow {
    width: f32,
    height: f32,
    scale_factor: f32,
    title: String,
    closed: bool,
    /// 渲染后端
    gpu: WebGpuBackend,
    /// Observer 管理器
    observer_manager: ObserverManager,
}

impl Window for WasmWindow {
    fn size(&self) -> (f32, f32) { (self.width, self.height) }
    fn scale_factor(&self) -> f32 { self.scale_factor }
    fn set_title(&mut self, title: &str) { self.title = title.to_string(); }
    fn request_redraw(&self) { /* rAF 自动调度 */ }
    fn platform(&self) -> Platform { Platform::Wasm }
    fn is_closed(&self) -> bool { self.closed }
}

impl WasmWindow {
    /// 创建 WASM 窗口
    /// canvas_id: HTML 中 <canvas> 元素的 id
    pub async fn new(canvas_id: &str, width: f32, height: f32) -> Result<Self, String>;

    /// 每帧 tick（由 rAF 回调调用）
    /// 1. 轮询 fetch_async
    /// 2. AnimationEngine::tick
    /// 3. compute_dom_styles (dirty only)
    /// 4. LayoutEngine::partial_layout (dirty subtrees)
    /// 5. DisplayListBuilder::build_dirty
    /// 6. WebGpuBackend::render
    /// 7. ObserverManager::poll
    pub fn tick(&mut self, dom: &mut Document, animation: &mut AnimationEngine);

    /// 处理 web-sys 事件 → 转换为 PointerEvent / KeyboardEvent 等
    pub fn handle_web_event(&mut self, event: &web_sys::Event);
}
```

### 6.4 iOS 运行时

```rust
// === crates/runtime/src/ios.rs === (aarch64-apple-ios)

/// iOS 运行时窗口
///
/// 通过 wgpu (Metal backend) 渲染，使用 UIView 触控事件。
pub struct IosWindow {
    width: f32,
    height: f32,
    scale_factor: f32,
    title: String,
    closed: bool,
    /// wgpu Surface (Metal)
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    backend: WgpuBackend,
    observer_manager: ObserverManager,
}

impl Window for IosWindow {
    fn size(&self) -> (f32, f32) { (self.width, self.height) }
    fn scale_factor(&self) -> f32 { self.scale_factor }
    fn set_title(&mut self, title: &str) { self.title = title.to_string(); }
    fn request_redraw(&self) { /* CADisplayLink 驱动 */ }
    fn platform(&self) -> Platform { Platform::IOS }
    fn is_closed(&self) -> bool { self.closed }
}

impl IosWindow {
    pub fn new(view: &raw_window_handle::RawWindowHandle, width: f32, height: f32) -> Self;

    /// 处理 UITouch → 转换为 TouchEvent + PointerEvent
    pub fn handle_touches(&mut self, touches: &[UITouch], phase: TouchPhase);

    pub fn tick(&mut self, dom: &mut Document, animation: &mut AnimationEngine);
}

/// UITouch phase 映射
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchPhase {
    Began,
    Moved,
    Ended,
    Cancelled,
}

/// iOS UITouch 数据（由平台层填充）
#[derive(Debug, Clone)]
pub struct UITouch {
    pub identifier: i32,
    pub x: f32,
    pub y: f32,
    pub force: f32,
    pub major_radius: f32,
}
```

### 6.5 Android 运行时

```rust
// === crates/runtime/src/android.rs === (aarch64-linux-android)

/// Android 运行时窗口
///
/// 通过 wgpu (Vulkan backend) 渲染，使用 MotionEvent 触控事件。
pub struct AndroidWindow {
    width: f32,
    height: f32,
    scale_factor: f32,
    title: String,
    closed: bool,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    backend: WgpuBackend,
    observer_manager: ObserverManager,
}

impl Window for AndroidWindow {
    fn size(&self) -> (f32, f32) { (self.width, self.height) }
    fn scale_factor(&self) -> f32 { self.scale_factor }
    fn set_title(&mut self, title: &str) { self.title = title.to_string(); }
    fn request_redraw(&self) { /* Choreographer 驱动 */ }
    fn platform(&self) -> Platform { Platform::Android }
    fn is_closed(&self) -> bool { self.closed }
}

impl AndroidWindow {
    pub fn new(native_window: &raw_window_handle::RawWindowHandle, width: f32, height: f32) -> Self;

    /// 处理 MotionEvent → 转换为 TouchEvent + PointerEvent
    pub fn handle_motion_event(&mut self, event: &AndroidMotionEvent);

    pub fn tick(&mut self, dom: &mut Document, animation: &mut AnimationEngine);
}

/// Android MotionEvent 封装
#[derive(Debug, Clone)]
pub struct AndroidMotionEvent {
    pub action: AndroidAction,
    pub pointers: Vec<(i32, f32, f32, f32)>, // (id, x, y, pressure)
    pub pointer_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AndroidAction {
    Down,
    Up,
    Move,
    Cancel,
    PointerDown,
    PointerUp,
}
```

### 6.6 ObserverManager

```rust
// === crates/runtime/src/observer_manager.rs ===

/// Observer 管理器 —— 每帧统一轮询
///
/// 运行时主循环中调用 poll()，按顺序检查：
/// 1. ResizeObserver — 比较元素 contentRect 变化
/// 2. IntersectionObserver — 计算与 viewport 交叉比
pub struct ObserverManager {
    resize_observers: Vec<ResizeObserver>,
    intersection_observers: Vec<IntersectionObserver>,
}

impl ObserverManager {
    pub fn new() -> Self;

    /// 注册 ResizeObserver
    pub fn register_resize(&mut self, observer: ResizeObserver);

    /// 注册 IntersectionObserver
    pub fn register_intersection(&mut self, observer: IntersectionObserver);

    /// 每帧轮询所有 observer
    pub fn poll(&mut self, viewport: Rect);
}
```

---

## 七、net crate — 网络层增强

### 7.1 fetch_async

```rust
// === crates/net/src/fetch_async.rs ===

/// 真正异步的 fetch 实现（tokio 非阻塞）
///
/// Phase 2 同步 fetch 会卡 UI，Phase 3 升级为非阻塞异步版本。
pub async fn fetch_async(url: &str, options: FetchOptions) -> Result<Response, FetchError>;

/// Request 对象 (Phase 3：独立可构造)
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub mode: RequestMode,
    pub credentials: RequestCredentials,
}

impl Request {
    pub fn new(url: &str) -> Self;
    pub fn with_method(mut self, method: &str) -> Self;
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self;
    pub fn with_body(mut self, body: Vec<u8>) -> Self;
}

/// Headers 对象 (Phase 3：独立可构造)
pub struct Headers {
    inner: HashMap<String, String>,
}

impl Headers {
    pub fn new() -> Self;
    pub fn set(&mut self, name: &str, value: &str);
    pub fn get(&self, name: &str) -> Option<&str>;
    pub fn has(&self, name: &str) -> bool;
    pub fn delete(&mut self, name: &str);
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)>;

    // 常用标准头
    pub fn content_type(&self) -> Option<&str>;
    pub fn authorization(&self) -> Option<&str>;
    pub fn set_content_type(&mut self, value: &str);
    pub fn set_authorization(&mut self, value: &str);
}
```

### 7.2 WebSocket 增强

```rust
// === crates/net/src/websocket.rs ===

/// WebSocket 配置 (Phase 3 增强)
pub struct WebSocketConfig {
    /// 自动重连
    pub auto_reconnect: bool,
    /// 最大重连次数（0 = 无限）
    pub max_reconnect_attempts: u32,
    /// 重连退避基数（毫秒）
    pub reconnect_base_delay_ms: u64,
    /// 最大重连延迟（毫秒）
    pub reconnect_max_delay_ms: u64,
    /// PING 间隔（毫秒，0 = 不发送心跳）
    pub ping_interval_ms: u64,
    /// PONG 超时（毫秒，超时后断开重连）
    pub pong_timeout_ms: u64,
    /// Per-Message Deflate 压缩
    pub compression: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_base_delay_ms: 1000,
            reconnect_max_delay_ms: 30000,
            ping_interval_ms: 30000,
            pong_timeout_ms: 10000,
            compression: false,
        }
    }
}

impl WebSocket {
    /// 使用配置创建 WebSocket（Phase 3 增强）
    pub fn with_config(url: &str, config: WebSocketConfig) -> Self;

    /// 手动发送 PING 帧
    pub fn ping(&mut self) -> Result<(), WebSocketError>;

    /// 手动重连
    pub fn reconnect(&mut self) -> Result<(), WebSocketError>;

    /// 获取当前连接状态
    pub fn connection_state(&self) -> ConnectionState;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Reconnecting { attempt: u32, next_delay_ms: u64 },
    Closed,
}
```

---

## 八、storage crate

Phase 3 无新增。localStorage / sessionStorage 已在 Phase 2 完成全部 6 个 API。IndexedDB 延期至 Phase 4+。

---

## 九、web2rust crate — 编译器大幅扩展

### 9.1 模块结构

```
web2rust/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── parser.rs           # JS 语法扩展: var/++/--/位运算/instanceof
│   ├── analyzer.rs         # 语义分析增强
│   ├── codegen.rs          # 代码生成: SPA路由/Object/Date/RegExp/Canvas 2D...
│   ├── builtins.rs          # 内置对象映射扩展
│   └── canvas_codegen.rs   # NEW: Canvas 2D API 编译
```

### 9.2 Object 静态方法编译映射（10 个）

```rust
// === crates/web2rust/src/builtins.rs ===

/// Object 静态方法 → Rust 映射表 (Phase 3 扩展)
///
/// JS                              → Rust
/// ───────────────────────────────────────────────────
/// Object.create(proto)            → Object::create(proto)
/// Object.defineProperty(o,k,desc) → Object::define_property(&mut o, k, desc)
/// Object.defineProperties(o,ps)   → Object::define_properties(&mut o, ps)
/// Object.freeze(o)                → Object::freeze(&mut o)
/// Object.seal(o)                  → Object::seal(&mut o)
/// Object.is(a, b)                 → Object::is(a, b)  // NaN/±0 精确比较
/// Object.hasOwn(o, k)             → Object::has_own(&o, k)
/// Object.fromEntries(entries)     → Object::from_entries(entries)
/// Object.getPrototypeOf(o)        → Object::get_prototype_of(&o)
/// Object.setPrototypeOf(o, p)     → Object::set_prototype_of(&mut o, p)

pub fn compile_object_static_call(method: &str, args: &[Expr]) -> TokenStream {
    match method {
        "create"          => quote! { Object::create(#(#args),*) },
        "defineProperty"  => quote! { Object::define_property(#(#args),*) },
        "defineProperties"=> quote! { Object::define_properties(#(#args),*) },
        "freeze"          => quote! { Object::freeze(#(#args),*) },
        "seal"            => quote! { Object::seal(#(#args),*) },
        "is"              => quote! { Object::is(#(#args),*) },
        "hasOwn"          => quote! { Object::has_own(#(#args),*) },
        "fromEntries"     => quote! { Object::from_entries(#(#args),*) },
        "getPrototypeOf"  => quote! { Object::get_prototype_of(#(#args),*) },
        "setPrototypeOf"  => quote! { Object::set_prototype_of(#(#args),*) },
        _ => unsupported_method("Object", method),
    }
}
```

### 9.3 History API 编译

```rust
// === crates/web2rust/src/codegen.rs ===

/// History API 编译映射
///
/// JS                                    → Rust
/// ───────────────────────────────────────────────────────
/// history.pushState(state, title, url)  → history.push_state(state, title, url)
/// history.replaceState(state, title, url)→ history.replace_state(state, title, url)
/// history.back()                        → history.back()
/// history.forward()                     → history.forward()
/// history.go(n)                         → history.go(n)
/// history.length                        → history.length()
/// history.state                         → history.state()
/// window.onpopstate = fn                → history.on_pop_state(fn)

/// SPA 路由编译示例:
///
/// ```js
/// history.pushState({ id: 1 }, '', '/page2');
/// window.onpopstate = () => { render(); };
/// ```
/// ↓ 编译为：
/// ```rust
/// window.history().push_state(&HistoryState::new().with("id", 1), "", "/page2");
/// window.history().on_pop_state(Box::new(move || {
///     render();
/// }));
/// ```
```

### 9.4 Location API 编译

```rust
/// Location API 编译映射
///
/// JS                        → Rust
/// ─────────────────────────────────────────
/// location.href             → location.href()
/// location.host             → location.host()
/// location.pathname         → location.pathname()
/// location.search           → location.search()
/// location.hash             → location.hash()
/// location.protocol         → location.protocol()
/// location.assign(url)      → location.assign(url)
/// location.replace(url)     → location.replace(url)
/// location.reload()         → location.reload()
```

### 9.5 URL / URLSearchParams 编译

```rust
/// URL 编译映射
///
/// JS                                    → Rust
/// ────────────────────────────────────────────────────────────
/// new URL(url, base)                    → URL::new(url, base)
/// url.href / host / pathname / search   → url.href() / host() / pathname() / search()
/// url.searchParams                      → url.search_params()
/// new URLSearchParams(query)            → URLSearchParams::new(query)
/// params.get(name)                      → params.get(name)
/// params.set(name, value)               → params.set(name, value)
/// params.has(name)                      → params.has(name)
/// params.delete(name)                   → params.delete(name)
/// params.toString()                     → params.to_string()
/// params.forEach(cb)                    → params.for_each(|k, v| { ... })
```

### 9.6 Date 编译

```rust
/// Date 编译映射 (Phase 3 新增)
///
/// JS                              → Rust
/// ────────────────────────────────────────────
/// new Date()                      → Date::now()
/// new Date(timestamp)             → Date::from_timestamp(ts)
/// new Date("2024-01-15")          → Date::parse("2024-01-15")
/// date.getTime()                  → date.timestamp_ms()
/// date.getFullYear()              → date.year()
/// date.getMonth()                 → date.month() // 0-based
/// date.getDate()                  → date.day()
/// date.getHours()                 → date.hours()
/// date.getMinutes()               → date.minutes()
/// date.getSeconds()               → date.seconds()
/// date.getDay()                   → date.weekday()
/// date.toISOString()              → date.to_iso_string()
/// date.toJSON()                   → date.to_iso_string()  // 兼容
```

### 9.7 Canvas 2D 编译

```rust
// === crates/web2rust/src/canvas_codegen.rs ===

/// Canvas 2D API 编译映射 (Phase 3 新增，18 个 API)
///
/// JS                                    → Rust
/// ─────────────────────────────────────────────────────────
/// const ctx = canvas.getContext('2d')   → let ctx = canvas.get_context_2d();
/// ctx.fillStyle = 'red'                 → ctx.set_fill_style(Color::red());
/// ctx.strokeStyle = '#333'             → ctx.set_stroke_style(Color::hex("#333"));
/// ctx.lineWidth = 2                     → ctx.set_line_width(2.0);
/// ctx.globalAlpha = 0.5                → ctx.set_global_alpha(0.5);
///
/// ctx.fillRect(x, y, w, h)             → ctx.fill_rect(x, y, w, h);
/// ctx.strokeRect(x, y, w, h)           → ctx.stroke_rect(x, y, w, h);
/// ctx.clearRect(x, y, w, h)            → ctx.clear_rect(x, y, w, h);
///
/// ctx.beginPath()                       → ctx.begin_path();
/// ctx.moveTo(x, y)                      → ctx.move_to(x, y);
/// ctx.lineTo(x, y)                      → ctx.line_to(x, y);
/// ctx.rect(x, y, w, h)                 → ctx.rect(x, y, w, h);
/// ctx.arc(x, y, r, start, end)         → ctx.arc(x, y, r, start, end);
/// ctx.fill()                            → ctx.fill();
/// ctx.stroke()                          → ctx.stroke();
///
/// ctx.save()                            → ctx.save();
/// ctx.restore()                         → ctx.restore();
/// ctx.translate(x, y)                   → ctx.translate(x, y);
/// ctx.rotate(angle)                     → ctx.rotate(angle);
/// ctx.scale(x, y)                       → ctx.scale(x, y);
/// ctx.setTransform(a,b,c,d,e,f)        → ctx.set_transform(a,b,c,d,e,f);
///
/// ctx.fillText(text, x, y)             → ctx.fill_text(text, x, y);
/// ctx.font = '16px sans-serif'          → ctx.set_font("16px sans-serif");
/// ctx.textAlign = 'center'             → ctx.set_text_align(TextAlign::Center);
/// ctx.measureText(text)                 → ctx.measure_text(text);
///
/// ctx.drawImage(img, dx, dy)            → ctx.draw_image(&img, dx, dy);
/// canvas.toDataURL()                    → canvas.to_data_url();
```

### 9.8 其他语法扩展

```rust
// === crates/web2rust/src/parser.rs ===

/// Phase 3 JS 语法新增支持

// var 声明 → 编译为 let (兼容旧代码)
// var x = 1;  →  let mut x = 1;

// 自增/自减
// x++;  →  x += 1;
// x--;  →  x -= 1;

// 位运算
// a & b   → a & b
// a | b   → a | b
// a ^ b   → a ^ b
// ~a      → !a
// a << b  → a << b
// a >> b  → a >> b
// a >>> b → ((a as u32) >> b) as i32

// instanceof
// x instanceof Foo  → x.is_instance_of::<Foo>()

// queueMicrotask(fn)  → queue_microtask(fn)
```

---

## 十、性能优化 API

### 10.1 增量重排

```rust
// === crates/layout/src/dirty_tracker.rs ===

/// 脏区域追踪器 (Phase 3 新增)
///
/// 仅重排标记为 dirty 的子树，将全树 O(n) layout 降为 O(k)。
pub struct DirtyTracker {
    dirty_nodes: HashSet<usize>, // Rc::as_ptr as usize
}

impl DirtyTracker {
    pub fn new() -> Self;

    /// 标记节点为脏（及其祖先 up to BFC root）
    pub fn mark_dirty(&mut self, node: &Rc<RefCell<Node>>);

    /// 获取脏的布局根节点列表
    pub fn dirty_roots(&self) -> Vec<Rc<RefCell<Node>>>;

    /// 清除所有脏标记
    pub fn clear(&mut self);
}

impl LayoutEngine {
    /// 增量布局：仅重排脏子树
    pub fn partial_layout(
        &mut self,
        root: &mut LayoutBox,
        dirty: &DirtyTracker,
        viewport: Rect,
    );
}
```

### 10.2 GPU 实例化渲染

```rust
// === crates/render/src/instanced_render.rs ===

/// GPU 实例化渲染 (Phase 3 新增)
///
/// 将同类图元合并为一次 instanced draw call，
/// 每个实例的变换矩阵通过 instance buffer 传递。
pub struct InstancedRenderer {
    /// 实例数据缓冲区: [transform_matrix; instance_count]
    instance_buffer: Vec<[f32; 16]>,
    /// 批处理阈值（同类型图元累积到此数量后提交）
    batch_threshold: usize,
}

impl InstancedRenderer {
    pub fn new(batch_threshold: usize) -> Self;

    /// 添加一个矩形实例
    pub fn push_rect(&mut self, rect: Rect, color: Color, transform: [f32; 16]);

    /// 添加一个文本字形实例
    pub fn push_glyph(&mut self, glyph: &Glyph, transform: [f32; 16]);

    /// 刷新所有待处理的批次
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);
}
```

### 10.3 纹理图集缓存

```rust
// === crates/render/src/texture_atlas.rs ===

/// 纹理图集 (Phase 3 新增)
///
/// 将字形/图标/小图打入一张大纹理，减少纹理绑定切换。
pub struct TextureAtlas {
    /// 图集纹理
    texture: wgpu::Texture,
    /// 已分配区域 { (glyph_id / icon_id) → (u, v, w, h) }
    allocations: HashMap<u64, (f32, f32, f32, f32)>,
    /// 图集尺寸
    width: u32,
    height: u32,
    /// 空闲矩形（simple shelf packing）
    free_shelves: Vec<AtlasRect>,
}

impl TextureAtlas {
    /// 创建图集（默认 2048x2048）
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self;

    /// 分配一个区域（返回 UV 坐标）
    pub fn allocate(
        &mut self,
        id: u64,
        data: &[u8],
        w: u32,
        h: u32,
        queue: &wgpu::Queue,
    ) -> Option<(f32, f32, f32, f32)>;

    /// 查询已分配的区域
    pub fn get(&self, id: u64) -> Option<(f32, f32, f32, f32)>;

    /// 获取图集纹理（用于着色器采样）
    pub fn texture(&self) -> &wgpu::Texture;
}
```

---

## 十一、Phase 4+ 预留扩展点

以下在 Phase 3 代码中用 `// Phase 4+:` 注释预留：

```rust
// crates/dom/src/element.rs
// Phase 4+:
// pub fn attach_shadow(&mut self, mode: ShadowRootMode) -> ShadowRoot;
// pub fn shadow_root(&self) -> Option<&ShadowRoot>;
// pub fn slot(&self) -> Option<String>;

// crates/dom/src/event.rs
// Phase 4+:
// pub fn composed_path(&self) -> Vec<Rc<RefCell<Node>>>;
// pub composed: bool,

// crates/dom/src/event.rs (拖拽事件)
// Phase 4+:
// DragEvent, dragstart, drag, dragend, dragenter, dragover, dragleave, drop

// crates/storage/src/indexed_db.rs
// Phase 4+:
// IndexedDB::open(name, version), transaction, put, get, delete
```

---

## 十二、统计

| 指标 | Phase 2 | Phase 3 新增 | Phase 3 累计 |
|------|---------|-------------|-------------|
| 代码量 | ~21,500 | ~12,000 | ~33,500 |
| DOM API | 115 ✅ | +36 🔲 | 151 |
| CSS 属性 | 171 ✅ | +47 🔲 | 218 |
| JS/Web API | 142 ✅ | +116 🔲 | 258 |
| crate 数量 | 10 | 0 | 10 |
| 新增模块文件 | — | ~13 | ~65 |
| 支持平台 | 1 (Desktop) | +3 (WASM/iOS/Android) | 4 |
| Canvas 2D API | 1 (stub) | +18 | 19 |
| Observer | 1 (Mutation) | +2 (Resize/Intersection) | 3 |

> **覆盖率（目标范围内）**：
> - HTML/DOM: 115 → 151 (100%)
> - CSS: 171 → 218 (96%)
> - JS/Web API: 142 → 258 (100%)
>
> 参见 [phase3-features.md](phase3-features.md) 了解完整功能清单。
