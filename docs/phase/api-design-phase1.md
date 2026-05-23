# Rust 浏览器引擎 — Phase 1 API 设计文档

> 本文档定义 Phase 1 所有公开类型、函数签名及作用。
> 中文注解标注功能说明，`// Phase 2+` 标记为后续阶段预留的扩展点。
> Phase 1 在 Phase 0 基础上新增约 8,000 行代码，CSS 属性从 30 扩展到 80 个，
> 实现完整事件系统、行内/表格布局、图像渲染、JS 基础编译器。

---

## 一、模块总览

```
┌──────────────────────────────────────────────────────────────────┐
│  Phase 1 模块依赖关系                                              │
│                                                                   │
│  examples/counter              examples/todo_app (Phase 2)         │
│       │                                                           │
│       ▼                                                           │
│  ┌─────────────────┐                                              │
│  │    web2rust      │  ← 编译器（构建期）                           │
│  │  ┌─────────────┐ │     html5ever + cssparser + selectors + swc │
│  │  │ parser.rs   │ │     HTML+CSS+JS → Rust 代码                  │
│  │  │ analyzer.rs │ │                                              │
│  │  │ codegen.rs  │ │                                              │
│  │  │ builtins.rs │ │                                              │
│  │  └─────────────┘ │                                              │
│  └────────┬────────┘                                              │
│           │  生成 main() 函数体                                    │
│           ▼                                                       │
│  runtime (主循环 + 事件 + AnimationFrameScheduler)                  │
│       │                                                           │
│       ├── render (wgpu 渲染后端 + 边框/阴影/图像管线)                │
│       │       │                                                    │
│       │       ▼                                                    │
│       │   paint (DisplayList + BatchOptimizer)                     │
│       │                                                           │
│       ├── layout (布局引擎)                                        │
│       │       │                                                    │
│       │       ├── css (CSS 引擎 + SelectorEngine)                  │
│       │       │       │                                            │
│       │       │       ▼                                            │
│       │       └── dom (DOM 树 + MutationObserver)                  │
│       │                                                           │
│       └── dom (DOM 树 - 同上引用)                                  │
└──────────────────────────────────────────────────────────────────┘
```

### Phase 1 实现原则

1. **标准完整**：事件冒泡、完整选择器、MutationObserver 全部按 W3C 标准实现
2. **增量更新**：布局和渲染支持增量更新，避免全量重排
3. **编译器升级**：web2rust 从模式识别升级为 html5ever + cssparser + swc 完整编译
4. **预留扩展点**：用 `// Phase 2+` 标记后续阶段位置
5. **W3C 命名**：函数名与浏览器标准 DOM API 保持一致
6. **中文注解**：所有文档注释使用中文

---

## 二、dom crate — DOM 标准扩展

### 2.1 模块结构

```
dom/
├── Cargo.toml
└── src/
    ├── lib.rs                  # 模块导出 + 新增公开类型
    ├── node.rs                 # Node 扩展 + DocumentFragment/Comment
    ├── element.rs              # ElementData 扩展（查询/焦点/客户端矩形）
    ├── document.rs             # Document 扩展（查询/片段/注释/索引）
    ├── text.rs                 # Text 扩展（分割/追加/删除/插入/替换/子串）
    ├── event.rs                # 新增 KeyboardEvent/FocusEvent/WheelEvent/冒泡
    ├── dom_token_list.rs       # DOMTokenList 扩展（length/item/replace/supports）
    └── mutation_observer.rs    # 新文件：MutationObserver/MutationRecord
```

### 2.2 NodeType — 新增枚举变体

```rust
// === crates/dom/src/node.rs ===

/// 节点类型枚举 —— 对应 W3C DOM 标准
pub enum NodeType {
    /// 元素节点：<div>, <h1>, <button> 等
    Element(ElementData),
    /// 文本节点：元素的文本内容
    Text(Text),
    /// 文档根节点
    Document,
    /// 文档片段（Phase 1：支持批量插入操作）
    DocumentFragment,
    /// 注释节点（Phase 1：存储注释文本）
    Comment(String),
}

/// W3C 标准节点类型常量
pub mod node_type_constants {
    pub const ELEMENT_NODE: u16 = 1;
    pub const TEXT_NODE: u16 = 3;
    pub const DOCUMENT_NODE: u16 = 9;
    pub const COMMENT_NODE: u16 = 8;              // Phase 1 新增
    pub const DOCUMENT_FRAGMENT_NODE: u16 = 11;   // Phase 1 新增
    // Phase 2+:
    // pub const DOCUMENT_TYPE_NODE: u16 = 10;
}
```

### 2.3 Node — 新增方法

```rust
// === crates/dom/src/node.rs ===

impl Node {
    // ===== Phase 0 已有方法 =====
    // append_child / remove_child / insert_before / replace_child
    // contains / clone_node / text_content / set_text_content
    // parent_node / child_nodes / first_child / last_child
    // previous_sibling / next_sibling / node_type / node_name / child_element_count

    // ============================================================
    //  Phase 1 新增方法
    // ============================================================

    /// 查找包含本节点的 Document 节点
    /// 沿着 parent 链向上遍历，返回第一个 Document 类型节点
    pub fn owner_document(&self) -> Option<Rc<RefCell<Document>>>;

    /// 规范化子节点：合并相邻 Text 节点，移除空 Text 节点
    pub fn normalize(&mut self);

    /// 深度相等比较（忽略 DOM 树位置）
    /// 比较节点类型、属性、子节点
    pub fn is_equal_node(&self, other: &Node) -> bool;

    /// 比较两个节点在文档中的位置关系
    /// 返回 document_position 常量组成的位掩码
    /// Phase 2+: 完整实现位常量
    pub fn compare_document_position(&self, other: &Node) -> u16;

    /// 完整克隆节点（Phase 0 是 stub，Phase 1 实现完整递归克隆）
    /// deep=true: 递归克隆子树; deep=false: 只克隆自身
    /// EventListener 克隆策略: deep=true 时不拷贝事件监听器（W3C 标准行为）
    pub fn clone_node(&self, deep: bool) -> Rc<RefCell<Node>>;

    // Phase 2+:
    // pub fn is_supported(&self, feature: &str, version: &str) -> bool;
    // pub fn lookup_prefix(&self, namespace: &str) -> Option<String>;
    // pub fn lookup_namespace_uri(&self, prefix: &str) -> Option<String>;
}
```

### 2.4 ElementData — 新增方法

```rust
// === crates/dom/src/element.rs ===

/// 事件监听器 + 选项（Phase 1 扩展）
pub struct EventListener {
    /// 回调函数
    pub callback: Box<dyn Fn(&Event)>,
    /// 监听器唯一标识
    pub id: usize,
    /// Phase 1: 监听器选项
    pub options: EventListenerOptions,
}

/// 事件监听器选项 —— W3C AddEventListenerOptions
#[derive(Debug, Clone)]
pub struct EventListenerOptions {
    /// 是否在捕获阶段触发（默认 false = 冒泡阶段）
    pub capture: bool,
    /// 是否只触发一次后自动移除
    pub once: bool,
    /// 是否永不调用 preventDefault（滚动优化）
    pub passive: bool,
}

impl ElementData {
    // ===== Phase 0 已有方法 =====
    // get_attribute / set_attribute / remove_attribute / has_attribute
    // class_list / class_list_mut / add_class / remove_class / toggle_class / has_class
    // get_style_value / set_style_value / remove_style_value / style_map / parse_and_set_style
    // add_event_listener / remove_event_listener / dispatch_event / get_event_listeners
    // id / set_id / class_name / set_class_name / tag_name

    // ============================================================
    //  Phase 1 新增方法 —— W3C Element API
    // ============================================================

    /// 序列化子节点为 HTML 字符串
    /// Phase 2+: 完整 HTML 序列化（含属性转义）
    pub fn inner_html(&self) -> String;

    /// 解析 HTML 片段替换所有子节点
    /// Phase 2+: 使用 html5ever 解析片段
    pub fn set_inner_html(&mut self, html: &str);

    /// CSS 选择器查询 —— 返回第一个匹配的后代元素
    pub fn query_selector(&self, selector: &str) -> Option<Rc<RefCell<Node>>>;

    /// CSS 选择器查询 —— 返回所有匹配的后代元素
    pub fn query_selector_all(&self, selector: &str) -> Vec<Rc<RefCell<Node>>>;

    /// 向上查找最近的匹配祖先元素（含自身）
    pub fn closest(&self, selector: &str) -> Option<Rc<RefCell<Node>>>;

    /// 判断自身是否匹配指定选择器
    pub fn matches(&self, selector: &str) -> bool;

    /// 设置焦点，触发 focus 事件
    pub fn focus(&mut self);

    /// 移除焦点，触发 blur 事件
    pub fn blur(&mut self);

    /// 返回布局计算后的边界矩形
    /// 需要关联 LayoutBox 获取 rect
    /// 首次调用触发布局计算（Phase 1：同步强制布局）
    pub fn get_bounding_client_rect(&self) -> Rect<f32>;

    // Phase 2+:
    // pub fn scroll_into_view(&mut self);
    // pub fn scroll_to(&mut self, x: f32, y: f32);
    // pub fn scroll_by(&mut self, x: f32, y: f32);
    // pub fn get_client_rects(&self) -> Vec<Rect<f32>>;
}
```

### 2.5 Document — 新增方法和索引

```rust
// === crates/dom/src/document.rs ===

/// Document 对象 —— W3C document 接口
pub struct Document {
    /// 文档元素 (<html>)
    document_element: Rc<RefCell<Node>>,
    /// body 元素
    body: Rc<RefCell<Node>>,
    /// Phase 1: 按 ID 索引元素的 HashMap（O(1) 查找）
    element_id_map: RefCell<HashMap<String, Weak<RefCell<Node>>>>,
}

impl Document {
    // ===== Phase 0 已有方法 =====
    // new / create_element / create_text_node / document_element / body

    // ============================================================
    //  Phase 1 新增 —— 查询
    // ============================================================

    /// 通过 ID 查找元素（O(1) HashMap 索引）
    pub fn get_element_by_id(&self, id: &str) -> Option<Rc<RefCell<Node>>>;

    /// CSS 选择器查询（从 document_element 开始）
    pub fn query_selector(&self, selector: &str) -> Option<Rc<RefCell<Node>>>;

    /// CSS 选择器查询全部匹配
    pub fn query_selector_all(&self, selector: &str) -> Vec<Rc<RefCell<Node>>>;

    /// 按标签名查找所有元素（当前为快照，Phase 2+ 实时集合）
    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<Rc<RefCell<Node>>>;

    /// 按类名查找所有元素
    pub fn get_elements_by_class_name(&self, class: &str) -> Vec<Rc<RefCell<Node>>>;

    // ============================================================
    //  Phase 1 新增 —— 节点创建
    // ============================================================

    /// 创建文档片段节点（用于批量插入优化）
    pub fn create_document_fragment(&self) -> Rc<RefCell<Node>>;

    /// 创建注释节点
    pub fn create_comment(&self, data: &str) -> Rc<RefCell<Node>>;

    // ============================================================
    //  Phase 1 新增 —— 文档属性
    // ============================================================

    /// 读取 <title> 文本内容
    pub fn title(&self) -> String;

    /// 设置 <title> 文本内容
    pub fn set_title(&self, title: &str);

    // Phase 2+:
    // pub fn create_element_ns(&self, namespace: &str, tag: &str) -> Rc<RefCell<Node>>;
    // pub fn import_node(&self, node: &Node, deep: bool) -> Rc<RefCell<Node>>;
    // pub fn adopt_node(&self, node: Rc<RefCell<Node>>);
}
```

### 2.6 Text — 完整文本操作方法

```rust
// === crates/dom/src/text.rs ===

impl Text {
    // ===== Phase 0 已有方法 =====
    // new / data / set_data / length

    // ============================================================
    //  Phase 1 新增 —— 文本操作方法（W3C Text 接口）
    // ============================================================

    /// 在 offset 位置分割文本节点
    /// 当前节点保留 [0..offset)，返回包含 [offset..] 的新节点
    pub fn split_text(&mut self, offset: usize) -> Rc<RefCell<Node>>;

    /// 追加文本到末尾
    pub fn append_data(&mut self, data: &str);

    /// 删除 offset 开始、count 长度的文本
    pub fn delete_data(&mut self, offset: usize, count: usize);

    /// 在 offset 位置插入文本
    pub fn insert_data(&mut self, offset: usize, data: &str);

    /// 替换 offset 开始、count 长度的文本
    pub fn replace_data(&mut self, offset: usize, count: usize, data: &str);

    /// 提取 offset 开始、count 长度的子串
    pub fn substring_data(&self, offset: usize, count: usize) -> String;
}
```

### 2.7 Event — Phase 1 事件系统扩展

```rust
// === crates/dom/src/event.rs ===

// ============================================================
//  KeyboardEvent —— 键盘事件 (Phase 1 新增)
// ============================================================

/// 键盘事件 —— 对应 W3C KeyboardEvent 接口
pub struct KeyboardEvent {
    /// 基础事件
    pub event: Event,
    /// 按键值："Enter", "a", "ArrowUp", "Escape"
    pub key: String,
    /// 物理键码："KeyA", "Digit1", "ArrowUp"
    pub code: String,
    /// 是否按下 Alt 键
    pub alt_key: bool,
    /// 是否按下 Ctrl 键
    pub ctrl_key: bool,
    /// 是否按下 Shift 键
    pub shift_key: bool,
    /// 是否按下 Meta 键（Win/Cmd）
    pub meta_key: bool,
    /// 是否长按重复触发
    pub repeat: bool,
    /// Phase 2+: 是否在 IME 组合中
    // pub is_composing: bool,
}

impl KeyboardEvent {
    /// 创建键盘事件
    pub fn new(event_type: &str, key: &str, code: &str) -> Self;
}

// ============================================================
//  FocusEvent —— 焦点事件 (Phase 1 新增)
// ============================================================

/// 焦点事件 —— 对应 W3C FocusEvent 接口
pub struct FocusEvent {
    /// 基础事件
    pub event: Event,
    /// 关联的焦点转移目标（失去焦点时为下一个焦点元素，获得焦点时为一个焦点元素）
    pub related_target: Option<Rc<RefCell<Node>>>,
}

impl FocusEvent {
    pub fn new(event_type: &str) -> Self;
}

// ============================================================
//  WheelEvent —— 滚轮事件 (Phase 1 新增)
// ============================================================

/// 滚轮事件 —— 对应 W3C WheelEvent 接口
pub struct WheelEvent {
    /// 基础事件
    pub event: Event,
    /// X 轴滚动量
    pub delta_x: f64,
    /// Y 轴滚动量
    pub delta_y: f64,
    /// Z 轴滚动量
    pub delta_z: f64,
    /// 滚动量的计算模式
    pub delta_mode: WheelDeltaMode,
}

/// 滚轮增量模式
#[derive(Debug, Clone, Copy)]
pub enum WheelDeltaMode {
    /// 像素模式
    Pixel = 0,
    /// 行模式
    Line = 1,
    /// 页模式
    Page = 2,
}

impl WheelEvent {
    pub fn new(event_type: &str, delta_x: f64, delta_y: f64, delta_mode: WheelDeltaMode) -> Self;
}
```

### 2.8 Event — 事件冒泡完整实现

```rust
// === crates/dom/src/event.rs ===

impl Event {
    // ===== Phase 0 已有方法 =====
    // new / prevent_default / default_prevented / stop_propagation / propagation_stopped

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 阻止同级其他监听器触发（比 stop_propagation 更强）
    pub fn stop_immediate_propagation(&self);

    /// 是否已停止立即传播
    pub fn immediate_propagation_stopped(&self) -> bool;

    /// Phase 2+: 返回事件穿透 Shadow DOM 的完整路径
    // pub fn composed_path(&self) -> Vec<Rc<RefCell<Node>>>;
}

// ============================================================
//  事件冒泡实现（Phase 1 核心新增）
// ============================================================

/// 事件传播阶段
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventPhase {
    /// 未派发
    None = 0,
    /// 捕获阶段：从 document_element 向下到 target 的父元素
    CapturingPhase = 1,
    /// 目标阶段：在事件目标元素上触发
    AtTarget = 2,
    /// 冒泡阶段：从 target 的父元素向上到 document_element
    BubblingPhase = 3,
}

/// 事件派发器 —— 负责完整的事件传播流程
///
/// 传播流程：
/// 1. 构建事件路径（从 target 到 document_element 的祖先链）
/// 2. 捕获阶段：按路径自上而下触发 capture=true 的监听器
/// 3. 目标阶段：在 target 上触发所有监听器（无论 capture 值）
/// 4. 冒泡阶段：按路径自下而上触发 capture=false 的监听器（仅 bubbles=true）
/// 5. 任一阶段调用 stop_propagation 中止后续传播
pub(crate) struct EventDispatcher;

impl EventDispatcher {
    /// 派发事件到目标元素，执行完整传播过程
    pub fn dispatch(
        target: &Rc<RefCell<Node>>,
        event: &Event,
    ) -> bool; // 返回是否被 preventDefault

    /// 构建从 target 到根的祖先路径
    fn build_path(target: &Rc<RefCell<Node>>) -> Vec<Rc<RefCell<Node>>>;
}
```

### 2.9 DOMTokenList — 完善

```rust
// === crates/dom/src/dom_token_list.rs ===

impl DOMTokenList {
    // ===== Phase 0 已有方法 =====
    // from_string / contains / add / remove / toggle / to_string

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 类名数量
    pub fn length(&self) -> usize;

    /// 通过索引获取类名（越界返回 None）
    pub fn item(&self, index: usize) -> Option<&str>;

    /// 替换类名：old_token 存在则替换为 new_token，返回是否成功替换
    pub fn replace(&mut self, old_token: &str, new_token: &str) -> bool;

    /// 检查 token 是否为有效的 CSS 类名
    pub fn supports(&self, token: &str) -> bool;
}
```

### 2.10 mutation_observer.rs — MutationObserver（全新）

```rust
// === crates/dom/src/mutation_observer.rs ===

/// MutationObserver —— W3C DOM 变更观察器
///
/// 替代 Phase 0 的 Cell<bool> 脏标记机制
/// Phase 1：同步触发（微任务队列在 Phase 2+）
pub struct MutationObserver {
    /// 回调函数
    callback: Box<dyn Fn(&[MutationRecord], &MutationObserver)>,
    /// 待处理的变更记录
    pending_records: RefCell<Vec<MutationRecord>>,
}

/// 单条变更记录
#[derive(Debug, Clone)]
pub struct MutationRecord {
    /// 变更类型
    pub record_type: MutationRecordType,
    /// 变更目标节点
    pub target: Rc<RefCell<Node>>,
    /// 添加的节点（childList 时有效）
    pub added_nodes: Vec<Rc<RefCell<Node>>>,
    /// 移除的节点（childList 时有效）
    pub removed_nodes: Vec<Rc<RefCell<Node>>>,
    /// 上一个兄弟节点（childList 时有效）
    pub previous_sibling: Option<Rc<RefCell<Node>>>,
    /// 下一个兄弟节点（childList 时有效）
    pub next_sibling: Option<Rc<RefCell<Node>>>,
    /// 变更的属性名（attributes 时有效）
    pub attribute_name: Option<String>,
    /// Phase 2+: 属性命名空间
    // pub attribute_namespace: Option<String>,
    /// 旧值（需在 observe 时指定 oldValue 选项）
    pub old_value: Option<String>,
}

/// 变更记录类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MutationRecordType {
    /// 属性变更
    Attributes,
    /// 子节点列表变更
    ChildList,
    /// 文本节点内容变更
    CharacterData,
}

/// MutationObserver 初始化选项
#[derive(Debug, Clone)]
pub struct MutationObserverInit {
    /// 是否观察子节点列表变更
    pub child_list: bool,
    /// 是否观察属性变更
    pub attributes: bool,
    /// 是否观察文本内容变更
    pub character_data: bool,
    /// 是否观察后代节点（subtree=true）
    pub subtree: bool,
    /// 是否记录属性旧值
    pub attribute_old_value: bool,
    /// 是否记录文本旧值
    pub character_data_old_value: bool,
    /// 限制观察的属性名列表（None = 全部属性）
    pub attribute_filter: Option<Vec<String>>,
}

impl MutationObserver {
    /// 创建观察器
    /// callback: 收到变更记录时调用的回调
    pub fn new(callback: Box<dyn Fn(&[MutationRecord], &MutationObserver)>) -> Self;

    /// 开始观察目标节点
    /// options: 观察选项（定义需要监听的变更类型）
    pub fn observe(&self, target: &Rc<RefCell<Node>>, options: MutationObserverInit);

    /// 停止观察并清空待处理记录
    pub fn disconnect(&self);

    /// 提取并清空当前待处理的变更记录
    pub fn take_records(&self) -> Vec<MutationRecord>;

    // Phase 2+:
    // /// 将回调排入微任务队列（Phase 2: 批量异步）
    // pub(crate) fn queue_microtask(&self);
}
```

### 2.11 dom 公开导出更新

```rust
// === crates/dom/src/lib.rs ===

// Phase 0 已有导出
pub use node::Node;
pub use node::NodeType;
pub use node::node_type_constants;
pub use element::ElementData;
pub use element::EventListenerOptions;      // Phase 1 新增
pub use document::Document;
pub use text::Text;
pub use event::Event;
pub use event::EventPhase;                 // Phase 1 新增
pub use event::MouseEvent;
pub use event::KeyboardEvent;              // Phase 1 新增
pub use event::FocusEvent;                 // Phase 1 新增
pub use event::WheelEvent;                 // Phase 1 新增
pub use event::WheelDeltaMode;             // Phase 1 新增
pub use dom_token_list::DOMTokenList;
pub use mutation_observer::{                // Phase 1 新增
    MutationObserver,
    MutationRecord,
    MutationRecordType,
    MutationObserverInit,
};

// Phase 1 新增基础类型
/// 二维坐标
#[derive(Debug, Clone, Copy)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

/// 四边尺寸（W3C 标准边距表示）
#[derive(Debug, Clone, Copy)]
pub struct EdgeInsets<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

// Phase 0 已有类型
pub struct Color { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }
pub struct Rect<T> { pub x: T, pub y: T, pub width: T, pub height: T }
pub struct Size<T> { pub width: T, pub height: T }
```

---

## 三、css crate — CSS 引擎扩展

### 3.1 模块结构

```
css/
├── Cargo.toml
├── properties.toml        # 从 30 扩展到 80 属性
└── src/
    ├── lib.rs              # 公开导出更新
    ├── stylesheet.rs       # 新增 parse_media_query/parse_keyframes/parse_font_face
    ├── selector.rs         # 新增 SelectorEngine + 组合器 + 基础伪类
    ├── cascade.rs          # 属性继承自动检测 + 级联缓存 + initial/inherit/unset
    └── values.rs           # 新增 Calc/Vw/Vh/Deg 等类型和单位
```

### 3.2 properties.toml — 新增约 50 个属性

```toml
# === crates/css/properties.toml ===
# Phase 1 CSS 属性定义（80 个属性）
# Phase 2+ 属性预留见末尾

# ===== Phase 0 已有（30 个）=====
# width, height, margin-*, padding-*, display, box-sizing, overflow
# flex-direction, flex-wrap, justify-content, align-items, align-content,
# gap, flex-grow, flex-shrink, flex-basis
# position, top, right, bottom, left
# color, font-size, font-weight, font-family, line-height, text-align, white-space
# background-color, background, opacity, border

# ===== Phase 1 新增：盒模型扩展 =====

[min-width]
initial = "auto"
inherited = false

[min-height]
initial = "auto"
inherited = false

[max-width]
initial = "none"
inherited = false

[max-height]
initial = "none"
inherited = false

# ===== Phase 1 新增：边框详细 =====

[border-top-width]
initial = "medium"
inherited = false

[border-right-width]
initial = "medium"
inherited = false

[border-bottom-width]
initial = "medium"
inherited = false

[border-left-width]
initial = "medium"
inherited = false

[border-top-style]
initial = "none"
inherited = false

[border-right-style]
initial = "none"
inherited = false

[border-bottom-style]
initial = "none"
inherited = false

[border-left-style]
initial = "none"
inherited = false

[border-top-color]
initial = "currentColor"
inherited = false

[border-right-color]
initial = "currentColor"
inherited = false

[border-bottom-color]
initial = "currentColor"
inherited = false

[border-left-color]
initial = "currentColor"
inherited = false

[border-radius]
initial = "0px"
inherited = false

[border-top-left-radius]
initial = "0px"
inherited = false

[border-top-right-radius]
initial = "0px"
inherited = false

[border-bottom-left-radius]
initial = "0px"
inherited = false

[border-bottom-right-radius]
initial = "0px"
inherited = false

# ===== Phase 1 新增：可见性 =====

[visibility]
initial = "visible"
inherited = false
# values: visible, hidden, collapse

# ===== Phase 1 新增：文字扩展 =====

[font-style]
initial = "normal"
inherited = true
# values: normal, italic, oblique

[text-decoration]
initial = "none"
inherited = false
# values: none, underline, overline, line-through

[text-decoration-color]
initial = "currentColor"
inherited = false

[text-decoration-style]
initial = "solid"
inherited = false

[word-break]
initial = "normal"
inherited = true
# values: normal, break-all, keep-all, break-word

[letter-spacing]
initial = "normal"
inherited = true

[word-spacing]
initial = "normal"
inherited = true

[text-transform]
initial = "none"
inherited = true
# values: none, uppercase, lowercase, capitalize

[text-indent]
initial = "0px"
inherited = true

# ===== Phase 1 新增：背景扩展 =====

[background-image]
initial = "none"
inherited = false

[background-size]
initial = "auto"
inherited = false
# values: auto, cover, contain, <length>

[background-position]
initial = "0% 0%"
inherited = false

[background-repeat]
initial = "repeat"
inherited = false
# values: repeat, repeat-x, repeat-y, no-repeat

[background-attachment]
initial = "scroll"
inherited = false

# ===== Phase 1 新增：定位扩展 =====

[z-index]
initial = "auto"
inherited = false

[overflow-x]
initial = "visible"
inherited = false

[overflow-y]
initial = "visible"
inherited = false

[cursor]
initial = "auto"
inherited = false
# values: auto, default, pointer, text, crosshair, move, not-allowed, grab, zoom-in, ...

# ===== Phase 1 新增：Flexbox 扩展 =====

[flex]
initial = "0 1 auto"
inherited = false

[order]
initial = "0"
inherited = false

[align-self]
initial = "auto"
inherited = false

[flex-flow]
initial = "row nowrap"
inherited = false

# ===== Phase 1 新增：列表 =====

[list-style-type]
initial = "disc"
inherited = true

[list-style-position]
initial = "outside"
inherited = true

[list-style-image]
initial = "none"
inherited = true

[list-style]
initial = "disc outside none"
inherited = true

# ===== Phase 1 新增：表格 =====

[table-layout]
initial = "auto"
inherited = false
# values: auto, fixed

[border-collapse]
initial = "separate"
inherited = false
# values: collapse, separate

[border-spacing]
initial = "0px"
inherited = true

# ===== Phase 1 新增：轮廓 =====

[outline-width]
initial = "medium"
inherited = false

[outline-style]
initial = "none"
inherited = false

[outline-color]
initial = "currentColor"
inherited = false

[outline]
initial = "medium none currentColor"
inherited = false

# ===== Phase 1 新增：交互 =====

[pointer-events]
initial = "auto"
inherited = false
# values: auto, none

[user-select]
initial = "auto"
inherited = false
# values: auto, none, text, all

# ============================================================
# Phase 2+ 属性预留
# ============================================================

# Phase 2:
# [animation-*] [transition-*] [transform-*]
# [grid-*] [float] [clear]
# [box-shadow] [text-shadow] [filter] [backdrop-filter]
# [mix-blend-mode] [background-blend-mode]
# [border-image-*]
# [column-*]
# [writing-mode] [direction]
# [--*] CSS 自定义属性
```

### 3.3 stylesheet.rs — 扩展

```rust
// === crates/css/src/stylesheet.rs ===

/// 选择器类型从 String 升级为 selectors crate 类型
pub type Selector = selectors::parser::Selector<SimpleSelector>;

// Phase 0 已有类型
// StyleSheet / Rule / Declaration / parse_stylesheet / parse_inline_style

// ============================================================
//  Phase 1 新增
// ============================================================

/// 媒体查询
#[derive(Debug, Clone)]
pub struct MediaQuery {
    /// 媒体类型
    pub media_type: MediaType,
    /// 条件列表
    pub conditions: Vec<MediaCondition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MediaType {
    /// 所有设备
    All,
    /// 打印
    Print,
    /// 屏幕
    Screen,
}

/// 媒体查询条件
#[derive(Debug, Clone)]
pub struct MediaCondition {
    /// 条件特征："min-width", "max-width", "orientation"
    pub feature: String,
    /// 条件值
    pub value: CSSValue,
}

/// 关键帧
#[derive(Debug, Clone)]
pub struct Keyframe {
    /// 关键帧选择器（"from", "to", "50%"）
    pub selector: KeyframeSelector,
    /// 该帧的 CSS 声明
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub enum KeyframeSelector {
    /// 起始帧 "from" = 0%
    From,
    /// 结束帧 "to" = 100%
    To,
    /// 百分比关键帧
    Percentage(f32),
}

/// @font-face 规则
#[derive(Debug, Clone)]
pub struct FontFace {
    /// 字体系列名
    pub family: String,
    /// 字体来源 url("font.woff2")
    pub source: String,
    /// 字重
    pub weight: Option<u16>,
    /// 字体样式
    pub style: Option<String>,
}

/// Phase 1 新增解析函数
pub fn parse_media_query(query: &str) -> MediaQuery;
pub fn parse_keyframes(keyframes_text: &str) -> Vec<Keyframe>;
pub fn parse_font_face(font_face_text: &str) -> FontFace;
```

### 3.4 selector.rs — SelectorEngine

```rust
// === crates/css/src/selector.rs ===

// Phase 0 已有函数
// match_selectors / element_matches_selector / compute_specificity

/// 完整选择器引擎 —— 基于 selectors crate
///
/// 支持 CSS 选择器 Level 3 的全部组合器和基础伪类
pub struct SelectorEngine {
    /// 编译后的选择器缓存（避免重复解析）
    selector_cache: HashMap<String, selectors::parser::Selector<SimpleSelector>>,
}

impl SelectorEngine {
    /// 创建选择器引擎
    pub fn new() -> Self;

    /// 解析选择器字符串为内部表示
    /// 解析失败返回错误
    pub fn parse(
        &mut self,
        selector_str: &str,
    ) -> Result<&selectors::parser::Selector<SimpleSelector>, SelectorParseError>;

    /// 匹配单个元素是否满足选择器
    pub fn matches(
        &self,
        element: &ElementData,
        selector: &selectors::parser::Selector<SimpleSelector>,
    ) -> bool;

    /// 在子树中查找第一个匹配元素（深度优先）
    pub fn query_selector(
        &self,
        root: &Rc<RefCell<Node>>,
        selector: &selectors::parser::Selector<SimpleSelector>,
    ) -> Option<Rc<RefCell<Node>>>;

    /// 在子树中查找所有匹配元素
    pub fn query_selector_all(
        &self,
        root: &Rc<RefCell<Node>>,
        selector: &selectors::parser::Selector<SimpleSelector>,
    ) -> Vec<Rc<RefCell<Node>>>;
}

/// 选择器解析错误
pub enum SelectorParseError {
    InvalidSyntax(String),
    UnsupportedFeature(String),
}

// ============================================================
//  Phase 1 支持的选择器功能
// ============================================================
//
// 组合器（Combinators）:
//   - 后代组合器（空格）: "div span"
//   - 子组合器（>）: "div > span"
//   - 相邻兄弟（+）: "div + span"
//   - 通用兄弟（~）: "div ~ span"
//
// 基础伪类:
//   - :first-child, :last-child, :only-child
//   - :empty, :root
//
// Phase 2+:
//   - :hover, :focus, :active, :visited
//   - :nth-child(an+b), :nth-last-child, :nth-of-type
//   - :not(selector), :checked, :disabled, :enabled, :required, :optional
//   - :is(), :where(), :has()

/// Phase 2+: 伪类枚举
// pub enum PseudoClass {
//     Hover, Active, Focus, FocusVisible, FocusWithin,
//     Visited, Link, Checked, Disabled, Enabled,
//     Required, Optional, Valid, Invalid,
//     FirstChild, LastChild, OnlyChild, FirstOfType, LastOfType, OnlyOfType,
//     NthChild(i32, i32), NthLastChild(i32, i32), NthOfType(i32, i32),
//     Empty, Not(Box<Selector>), Root,
// }
```

### 3.5 cascade.rs — 增强

```rust
// === crates/css/src/cascade.rs ===

// Phase 0 已有：compute_element_style / ComputedStyle / cascade_sort / apply_inherited

impl ComputedStyle {
    // ===== Phase 0 已有 =====
    // empty / get / get_or_initial

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 创建包含所有已知属性初始值的 ComputedStyle
    /// 初始值从 properties.toml 编译期生成
    pub fn initial() -> Self;

    /// 自动继承：根据 properties.toml 的 inherited 字段自动判断
    /// 替代 Phase 0 的手动继承列表
    pub fn apply_inheritance(&mut self, parent: &ComputedStyle);

    /// 检查属性是否可继承（由 properties.toml 决定）
    pub fn is_inherited(property: &str) -> bool;
}

// ============================================================
//  CSS 关键字值处理（Phase 1 新增）
// ============================================================

/// 解析 CSS 特殊关键字
/// - initial: 重置为属性初始值
/// - inherit: 强制继承父元素值
/// - unset: inherited 属性 → inherit，否则 → initial
/// Phase 2+:
/// - revert: 回退到用户代理样式
pub fn resolve_keyword(
    keyword: &str,
    property: &str,
    parent_value: Option<&CSSValue>,
) -> Option<CSSValue>;
```

### 3.6 values.rs — 扩展

```rust
// === crates/css/src/values.rs ===

// Phase 0 已有
// CSSValue / CSSUnit / parse_css_value / parse_color / parse_length

// ============================================================
//  Phase 1 新增 CSSValue 变体
// ============================================================

pub enum CSSValue {
    // Phase 0 已有变体
    Length(f32, CSSUnit),
    Percentage(f32),
    Color(Color),
    Keyword(String),
    Number(f32),
    String(String),
    Initial,

    // Phase 1 新增
    /// calc(100% - 20px) 表达式
    Calc(Box<CalcValue>),
    /// transform 变换函数列表
    Transform(Vec<Transform>),
    /// 渐变
    Gradient(Box<Gradient>),
    /// 长度+百分比组合（用于 calc 中间结果）
    LengthPercentage(f32, f32, CSSUnit),
}

// ============================================================
//  Phase 1 新增单位
// ============================================================

pub enum CSSUnit {
    // Phase 0 已有
    Px, Em, Rem, Percent,

    // Phase 1 新增
    /// 视口宽度 1%
    Vw,
    /// 视口高度 1%
    Vh,
    /// min(vw, vh) * 1%
    Vmin,
    /// max(vw, vh) * 1%
    Vmax,
    /// 角度单位：度
    Deg,
    /// 角度单位：弧度
    Rad,
    /// 角度单位：百分度
    Grad,
    /// 角度单位：圈数
    Turn,
    /// 时间单位：秒
    S,
    /// 时间单位：毫秒
    Ms,
    /// 分辨率单位：点/英寸
    Dpi,
    /// 分辨率单位：点/厘米
    Dpcm,

    // Phase 2+:
    // Fr,    // Grid 弹性系数
}

// ============================================================
//  Phase 1 新增类型
// ============================================================

/// Calc 表达式节点 —— 二进制树结构
pub struct CalcValue {
    pub root: CalcNode,
}

pub enum CalcNode {
    /// 叶子：CSS 值
    Value(CSSValue),
    /// 加法
    Sum(Box<CalcNode>, Box<CalcNode>),
    /// 乘法
    Product(Box<CalcNode>, Box<CalcNode>),
    /// 取负
    Negate(Box<CalcNode>),
    /// 取倒数
    Invert(Box<CalcNode>),
}

/// Transform 变换函数
#[derive(Debug, Clone)]
pub enum Transform {
    /// 2D 矩阵
    Matrix([f32; 6]),
    /// 平移
    Translate(f32, f32),
    TranslateX(f32),
    TranslateY(f32),
    /// 缩放
    Scale(f32, f32),
    ScaleX(f32),
    ScaleY(f32),
    /// 旋转
    Rotate(f32), // 单位：角度
    /// 倾斜
    Skew(f32, f32),
    SkewX(f32),
    SkewY(f32),
    // Phase 2+: 3D 变换
    // Matrix3d([f32; 16]),
    // TranslateZ(f32), Translate3d(f32, f32, f32),
    // ScaleZ(f32), Scale3d(f32, f32, f32),
    // RotateX(f32), RotateY(f32), RotateZ(f32), Rotate3d(f32, f32, f32, f32),
    // Perspective(f32),
}

/// 渐变
#[derive(Debug, Clone)]
pub struct Gradient {
    pub gradient_type: GradientType,
    pub direction: GradientDirection,
    pub stops: Vec<ColorStop>,
}

#[derive(Debug, Clone)]
pub enum GradientType {
    Linear,
    Radial,
    Conic,
}

#[derive(Debug, Clone)]
pub enum GradientDirection {
    Angle(f32),
    Side(TopOrBottom, LeftOrRight),
}

#[derive(Debug, Clone)]
pub enum TopOrBottom { Top, Bottom }

#[derive(Debug, Clone)]
pub enum LeftOrRight { Left, Right }

/// 渐变色标
#[derive(Debug, Clone)]
pub struct ColorStop {
    pub color: Color,
    pub position: Option<CSSValue>,
}

// ============================================================
//  Phase 1 新增解析函数
// ============================================================

/// 解析 transform 属性值
pub fn parse_transform(value: &str) -> Vec<Transform>;

/// 解析渐变值
pub fn parse_gradient(value: &str) -> Gradient;

/// 解析 calc() 表达式
pub fn parse_calc_expression(value: &str) -> CalcValue;

// Phase 2+:
// pub fn parse_animation(value: &str) -> Vec<Animation>;
// pub fn parse_filter(value: &str) -> Vec<Filter>;
```

### 3.7 css 公开导出更新

```rust
// === crates/css/src/lib.rs ===

// Phase 0 已有导出
pub use stylesheet::{parse_stylesheet, parse_inline_style, StyleSheet, Rule, Declaration};
pub use selector::{
    match_selectors, element_matches_selector, MatchedDeclaration,
    SelectorEngine, SelectorParseError,          // Phase 1 新增
};
pub use cascade::{compute_element_style, ComputedStyle, resolve_keyword};
pub use values::{
    CSSValue, CSSUnit, parse_css_value, parse_color, parse_length,
    CalcValue, CalcNode, Transform, Gradient,     // Phase 1 新增
    GradientType, GradientDirection, ColorStop,
    parse_transform, parse_gradient, parse_calc_expression,
};

// Phase 1 新增导出
pub use stylesheet::{
    parse_media_query, parse_keyframes, parse_font_face,
    MediaQuery, MediaType, MediaCondition,
    Keyframe, KeyframeSelector,
    FontFace,
};

// Phase 2+:
// pub use values::parse_animation;
// pub use animations::AnimationEngine;
// pub use transitions::TransitionEngine;
// pub use media::MediaQueryEvaluator;
```

---

## 四、layout crate — 布局引擎扩展

### 4.1 模块结构

```
layout/
├── Cargo.toml
└── src/
    ├── lib.rs              # LayoutEngine 扩展 + 新增公开导出
    ├── layout_box.rs       # BoxType 新增变体 + LayoutBox 新增字段
    ├── flex.rs             # min/max 约束 + flex-wrap 完整支持
    ├── block.rs            # margin collapse 外边距合并
    ├── positioned.rs       # position: fixed / sticky
    ├── inline.rs           # 新文件：InlineLayout 行内布局
    ├── table.rs            # 新文件：TableLayout 表格布局
    └── text.rs             # break_text / font_fallback / line_height
```

### 4.2 BoxType — 新增枚举变体

```rust
// === crates/layout/src/layout_box.rs ===

pub enum BoxType {
    // Phase 0 已有
    Block,
    Inline,
    FlexContainer,
    FlexItem,
    Text,
    Anonymous,

    // Phase 1 新增
    /// display: inline-block
    InlineBlock,
    /// display: table
    Table,
    /// display: table-row
    TableRow,
    /// display: table-row-group
    TableRowGroup,
    /// display: table-cell
    TableCell,
    /// display: table-caption (Phase 2+ 完善)
    TableCaption,
    /// position: absolute
    Absolute,
    /// position: fixed
    Fixed,
    /// position: sticky
    Sticky,

    // Phase 2+:
    // GridContainer, GridItem,
    // Float,
}
```

### 4.3 LayoutBox — 新增字段

```rust
// === crates/layout/src/layout_box.rs ===

pub struct LayoutBox {
    // Phase 0 已有字段
    pub box_type: BoxType,
    pub node: Option<Rc<RefCell<Node>>>,
    pub children: Vec<LayoutBox>,
    pub rect: Rect<f32>,
    pub padding: EdgeSizes,
    pub margin: EdgeSizes,
    pub border: EdgeSizes,

    // Phase 1 新增
    /// z-index 层级值
    pub z_index: i32,
    /// 是否创建新的层叠上下文
    pub stacking_context: bool,
    /// overflow-x / overflow-y
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    /// 四角圆角半径 (top-left, top-right, bottom-right, bottom-left)
    pub border_radius: [f32; 4],
    /// 可见性
    pub visibility: Visibility,
    /// Phase 2+: CSS transform
    // pub transform: Option<Vec<Transform>>,
}

/// overflow 取值
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

/// visibility 取值
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visibility {
    Visible,
    Hidden,
    /// table-row / table-column 专用
    Collapse,
}
```

### 4.4 LayoutEngine — 新方法

```rust
// === crates/layout/src/lib.rs ===

impl LayoutEngine {
    // ===== Phase 0 已有 =====
    // new / layout

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 局部重排：仅重排脏子树
    ///
    /// dirty_paths: 从根到脏节点的路径列表（每项是 child_index 序列）
    /// viewport: 当前视口尺寸
    ///
    /// 返回受影响的区域列表（用于局部重绘）
    pub fn partial_layout(
        &mut self,
        root: &mut LayoutBox,
        dirty_paths: &[Vec<usize>],
        viewport: Size<f32>,
    ) -> Vec<Rect<f32>>;

    /// 更新布局树：DOM 变更后增量更新 LayoutBox
    ///
    /// dirty_nodes: 脏节点的 Rc::as_ptr 地址集合
    /// styles: 最新的计算样式映射
    ///
    /// 返回是否有影响视口的尺寸变化
    pub fn update_layout_tree(
        &mut self,
        root: &mut LayoutBox,
        dirty_nodes: &HashSet<usize>,
        styles: &HashMap<usize, ComputedStyle>,
    ) -> bool;

    /// 按 DOM 节点地址在布局树中查找 LayoutBox
    pub fn find_layout_node<'a>(
        root: &'a LayoutBox,
        node_ptr: usize,
    ) -> Option<&'a LayoutBox>;

    /// 按 DOM 节点地址查找可变引用
    pub fn find_layout_node_mut<'a>(
        root: &'a mut LayoutBox,
        node_ptr: usize,
    ) -> Option<&'a mut LayoutBox>;
}

// Phase 1 新增公开函数
pub fn find_layout_node<'a>(root: &'a LayoutBox, node_ptr: usize) -> Option<&'a LayoutBox>;
```

### 4.5 inline.rs — 行内布局（全新）

```rust
// === crates/layout/src/inline.rs ===

/// 行内布局 —— 实现 W3C CSS Inline Layout
///
/// 将行内级元素和文本排列成行框（line box）
/// 处理：
/// - 水平文字排列（LTR / RTL）
/// - 行框高度计算（取行内最大 line-height）
/// - vertical-align 对齐
/// - 文本换行（由 word-break / white-space 驱动）
/// - inline-block 元素作为整体参与行内排列
pub struct InlineLayout;

impl InlineLayout {
    /// 对 Inline 类型的容器执行行内布局
    ///
    /// container: Block 容器（其子节点按行内排列）
    /// text_measurer: 文本测量器（用于计算字形宽度）
    /// viewport: 视口尺寸
    pub fn layout(
        &mut self,
        container: &mut LayoutBox,
        text_measurer: &mut TextMeasurer,
        viewport: Size<f32>,
    );

    /// 将行内子节点按容器宽度分行为行框
    ///
    /// 返回：每行对应的子节点索引范围
    fn break_into_lines(
        children: &[LayoutBox],
        max_width: f32,
        text_measurer: &mut TextMeasurer,
    ) -> Vec<Vec<usize>>;

    /// 计算单行内各子项的水平位置
    ///
    /// 考虑 text-align 对齐方式
    fn layout_line(
        children: &mut [LayoutBox],
        line_indices: &[usize],
        line_y: f32,
        line_width: f32,
        total_content_width: f32,
        text_align: &str,
        direction: TextDirection,
    );

    /// 计算行框高度（取该行所有子项的最大 computed line-height）
    fn compute_line_height(
        children: &[LayoutBox],
        line_indices: &[usize],
        text_measurer: &TextMeasurer,
    ) -> f32;
}

/// 文字方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextDirection {
    Ltr,
    Rtl,
}
```

### 4.6 table.rs — 表格布局（全新）

```rust
// === crates/layout/src/table.rs ===

/// 表格布局 —— W3C CSS Table Layout
///
/// 实现 CSS 2.1 自动表格布局算法
/// Phase 1 限制：
/// - 不支持 colspan / rowspan 跨单元格
/// - 不支持 border-collapse: collapse 的复杂边框合并
///
/// Phase 2+ 完善：
/// - colspan / rowspan 支持
/// - fixed table layout（table-layout: fixed）
/// - border-collapse 合并算法
pub struct TableLayout;

impl TableLayout {
    /// 对 Table 容器执行表格布局
    ///
    /// 分为四个 pass：
    /// 1. compute_column_widths: 计算每列的最小/最大宽度
    /// 2. assign_column_widths: 分配最终列宽
    /// 3. compute_row_heights: 计算每行高度
    /// 4. place_cells: 放置每个单元格的最终位置
    pub fn layout(
        &mut self,
        table: &mut LayoutBox,
        viewport: Size<f32>,
    );

    /// 第一 pass: 计算每列最小/最大宽度
    fn compute_column_widths(table: &LayoutBox) -> Vec<ColumnWidth>;

    /// 第二 pass: 将可用宽度分配给各列
    fn assign_column_widths(
        available_width: f32,
        col_widths: &[ColumnWidth],
        table_layout_mode: TableLayoutMode,
    ) -> Vec<f32>;

    /// 第三 pass: 计算每行高度
    fn compute_row_heights(table: &LayoutBox) -> Vec<f32>;

    /// 第四 pass: 将单元格放置到最终位置
    fn place_cells(
        table: &mut LayoutBox,
        col_widths: &[f32],
        row_heights: &[f32],
    );

    // Phase 2+:
    // fn resolve_colspan_rowspan(table: &LayoutBox) -> Vec<CellSpanInfo>;
    // fn resolve_collapsed_borders(table: &mut LayoutBox);
    // fn compute_border_conflict(cell_a: &LayoutBox, cell_b: &LayoutBox, edge: TableEdge) -> f32;
}

/// 表格布局模式
#[derive(Debug, Clone, Copy)]
pub enum TableLayoutMode {
    /// table-layout: auto（内容驱动宽度）
    Auto,
    /// table-layout: fixed（列宽等分 + 第一行决定）
    Fixed,
}

/// 列宽信息
struct ColumnWidth {
    pub min_width: f32,
    pub max_width: f32,
}
```

### 4.7 flex.rs — 约束扩展

```rust
// === crates/layout/src/flex.rs ===

impl FlexLayout {
    // ===== Phase 0 已有 =====
    // layout / convert_style / apply_result

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 应用 min-width / max-width 约束
    /// 在 taffy 布局计算后执行，限制最终宽度
    fn apply_min_max_constraints(
        container: &mut LayoutBox,
        taffy: &taffy::TaffyTree,
        child_map: &HashMap<usize, taffy::NodeId>,
    );

    /// flex-wrap 完整支持
    /// - nowrap: 单行（Phase 0 已支持）
    /// - wrap: 超出换行
    /// - wrap-reverse: 反向换行
    fn apply_flex_wrap(style: &mut taffy::Style, wrap_value: &str);

    /// align-content 完整支持（多行场景）
    /// - stretch / flex-start / flex-end / center / space-between / space-around / space-evenly
    fn apply_align_content(style: &mut taffy::Style, align_content_value: &str);
}
```

### 4.8 block.rs — margin collapse

```rust
// === crates/layout/src/block.rs ===

impl BlockLayout {
    // ===== Phase 0 已有 =====
    // layout

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 外边距合并：相邻块级元素的垂直 margin
    ///
    /// 合并规则：max(prev.margin_bottom, current.margin_top)
    ///
    /// 特殊情况：
    /// - 父元素的 margin-top 与第一个子元素的 margin-top 合并
    /// - 父元素的 margin-bottom 与最后一个子元素的 margin-bottom 合并
    /// - 空元素自身的 margin-top 和 margin-bottom 合并
    fn compute_collapsed_margin(
        prev_sibling: Option<&LayoutBox>,
        current: &LayoutBox,
        parent: &LayoutBox,
    ) -> f32;

    /// 检查元素是否创建 BFC (Block Formatting Context)
    /// BFC 阻止外边距合并
    fn creates_bfc(box_node: &LayoutBox) -> bool;

    // Phase 2+:
    // fn compute_clear_position(clear_value: ClearValue, float_areas: &[FloatArea]) -> f32;
}
```

### 4.9 positioned.rs — fixed/sticky

```rust
// === crates/layout/src/positioned.rs ===

impl PositionedLayout {
    // ===== Phase 0 已有 =====
    // layout / find_positioned_ancestor

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// position: fixed 处理
    /// 相对于视口定位，不随滚动偏移
    fn layout_fixed(
        &self,
        root: &mut LayoutBox,
        viewport: Size<f32>,
    );

    /// position: sticky 处理
    /// 在普通流中定位，但当滚动到阈值时"粘住"
    fn layout_sticky(
        &self,
        root: &mut LayoutBox,
        scroll_offset: (f32, f32),
        viewport: Size<f32>,
    );

    /// 提取 sticky 约束值
    fn extract_sticky_constraints(style: &ComputedStyle) -> StickyConstraint;
}

/// Sticky 定位约束
struct StickyConstraint {
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
}
```

### 4.10 text.rs — 扩展

```rust
// === crates/layout/src/text.rs ===

impl TextMeasurer {
    // ===== Phase 0 已有 =====
    // new / measure / measure_lines

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 断字换行：在指定宽度内将文本分行
    ///
    /// 基于 unicode-linebreak 和 word-break 属性
    /// 返回：每行的文本片段
    pub fn break_text(
        &self,
        text: &str,
        max_width: f32,
        font_size: f32,
        font_family: &str,
        word_break: &str,
    ) -> Vec<String>;

    /// 字体回退链：中英文混排时按优先级查找字体
    ///
    /// 返回：文本片段到字体的映射列表
    fn font_fallback(
        &mut self,
        text: &str,
        families: &[&str],
        font_size: f32,
    ) -> Vec<FontSpan>;

    /// 行高计算：根据 line-height 属性计算实际行高（像素）
    ///
    /// line-height_value: "normal" → font_size * 1.2
    /// line-height_value: "1.5" → font_size * 1.5
    /// line-height_value: "24px" → 24.0
    /// line-height_value: "150%" → font_size * 1.5
    pub fn line_height(font_size: f32, line_height_value: &CSSValue) -> f32;
}

/// 字体片段：某段文本使用指定字体
#[derive(Debug, Clone)]
pub struct FontSpan {
    pub text: String,
    pub font: String,
    pub font_size: f32,
}
```

### 4.11 layout 公开导出更新

```rust
// === crates/layout/src/lib.rs ===

// Phase 0 已有导出
pub use layout_box::{LayoutBox, BoxType, EdgeSizes};
pub use flex::FlexLayout;
pub use block::BlockLayout;
pub use positioned::PositionedLayout;
pub use text::TextMeasurer;

// Phase 1 新增导出
pub use inline::InlineLayout;
pub use table::TableLayout;
pub use table::TableLayoutMode;
pub use text::FontSpan;
pub use layout_box::{Overflow, Visibility, TextDirection};

// Phase 2+:
// pub use grid::GridLayout;
// pub use float::FloatLayout;
```

---

## 五、paint crate — DisplayList 扩展

### 5.1 模块结构

```
paint/
├── Cargo.toml
└── src/
    ├── lib.rs              # 公开导出更新
    ├── command.rs           # PaintCommand 新增变体 + 枚举类型
    ├── builder.rs           # DisplayListBuilder 新增处理
    └── optimizer.rs         # 新文件：BatchOptimizer 合批优化器
```

### 5.2 PaintCommand — 新增变体和增强

```rust
// === crates/paint/src/command.rs ===

pub enum PaintCommand {
    // Phase 0 已有（增强）
    FillRect {
        rect: Rect<Pixel>,
        color: Color,
        border_radius: [f32; 4],       // Phase 1: 四角圆角
    },
    Text {
        text: String,
        font_size: f32,
        font_family: String,
        font_weight: u16,
        position: (Pixel, Pixel),
        color: Color,
        text_decoration: TextDecoration,  // Phase 1 新增
    },
    Border {
        rect: Rect<Pixel>,
        widths: [f32; 4],
        colors: [Color; 4],
        radius: f32,
        style: BorderStyle,              // Phase 1 新增
    },

    // ============================================================
    //  Phase 1 新增变体
    // ============================================================

    /// 盒子阴影
    BoxShadow {
        rect: Rect<Pixel>,
        /// 阴影偏移
        offset: (Pixel, Pixel),
        /// 模糊半径
        blur_radius: f32,
        /// 扩散半径
        spread_radius: f32,
        /// 阴影颜色
        color: Color,
        /// 是否为内阴影
        inset: bool,
    },
    /// 图像
    Image {
        rect: Rect<Pixel>,
        /// 图像纹理句柄
        image_id: ImageId,
        /// 适应模式
        object_fit: ObjectFit,
        /// 图像在矩形内的偏移
        object_position: (Pixel, Pixel),
        // Phase 2+: border_radius
    },
    /// 裁剪区域
    Clip {
        rect: Rect<Pixel>,
        border_radius: [f32; 4],
        /// 被裁剪的子命令
        children: Vec<PaintCommand>,
    },
    /// 透明度组
    Opacity {
        opacity: f32,
        /// 透明组内的子命令
        children: Vec<PaintCommand>,
    },

    // Phase 2+:
    // Transform { matrix: [f32; 6], children: Vec<PaintCommand> },
    // Filter { filter_type: FilterType, children: Vec<PaintCommand> },
}

// ============================================================
//  新增枚举类型
// ============================================================

/// 边框样式（Phase 1 新增）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderStyle {
    None,
    Hidden,
    Solid,
    Dotted,
    Dashed,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
}

/// 文本装饰（Phase 1 新增）
#[derive(Debug, Clone)]
pub struct TextDecoration {
    pub underline: bool,
    pub overline: bool,
    pub line_through: bool,
    // Phase 2+:
    // pub color: Option<Color>,
    // pub style: Option<TextDecorationStyle>,
}

/// 图像适应模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectFit {
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
}

/// 图像纹理句柄
pub type ImageId = u64;
```

### 5.3 DisplayListBuilder — 新处理函数

```rust
// === crates/paint/src/builder.rs ===

impl DisplayListBuilder {
    // ===== Phase 0 已有 =====
    // new / build / extract_bg_color / extract_border / extract_text

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 提取并生成 BoxShadow 命令
    fn extract_box_shadow(&self, node: &LayoutBox) -> Option<PaintCommand>;

    /// 提取并生成 Image 命令
    fn extract_image(&self, node: &LayoutBox) -> Option<PaintCommand>;

    /// 生成 Clip 命令（包装子命令）
    fn build_clip(
        &self,
        node: &LayoutBox,
        children: Vec<PaintCommand>,
    ) -> Option<PaintCommand>;

    /// 生成 Opacity 命令（包装子命令）
    fn build_opacity(
        &self,
        node: &LayoutBox,
        children: Vec<PaintCommand>,
    ) -> Option<PaintCommand>;

    /// 提取 border-radius 值为 [f32; 4]
    fn extract_border_radius(&self, node: &LayoutBox) -> [f32; 4];

    /// 提取 text-decoration
    fn extract_text_decoration(&self, node: &LayoutBox) -> TextDecoration;
}
```

### 5.4 optimizer.rs — 合批优化器（全新）

```rust
// === crates/paint/src/optimizer.rs ===

/// 合批优化器 —— 减少 GPU draw call 数量
///
/// 策略：
/// 1. 按 PaintCommand 类型分组
/// 2. 同类型命令按纹理/颜色二次分组（减少状态切换）
/// 3. 保持 z-order 正确的绘制顺序
pub struct BatchOptimizer {
    batches: Vec<RenderBatch>,
}

/// 单个渲染批次
pub struct RenderBatch {
    /// 批次类型
    pub batch_type: BatchType,
    /// 批次内的绘制命令
    pub commands: Vec<PaintCommand>,
    /// 批次包围盒（用于 GPU 裁剪优化）
    pub bounding_box: Rect<Pixel>,
}

/// 批次类型
pub enum BatchType {
    /// 纯色矩形（可合并为 GPU 实例化绘制）
    SolidRect,
    /// 带纹理矩形（图像）
    TexturedRect,
    /// 文本
    Text,
    /// 边框
    Border,
    /// 阴影
    Shadow,
}

impl BatchOptimizer {
    /// 从 DisplayList 构建优化后的批次
    pub fn optimize(display_list: &DisplayList) -> Self;

    /// 获取所有批次
    pub fn batches(&self) -> &[RenderBatch];

    /// 批次数量（≈ GPU draw call 数量）
    pub fn batch_count(&self) -> usize;

    /// 合批节省的 draw call 数量
    pub fn saved_draw_calls(&self) -> usize;
}
```

### 5.5 paint 公开导出更新

```rust
// === crates/paint/src/lib.rs ===

pub use command::{
    PaintCommand, DisplayList, BorderStyle, TextDecoration,
    ObjectFit, ImageId,
};
pub use builder::DisplayListBuilder;
pub use optimizer::{BatchOptimizer, RenderBatch, BatchType};  // Phase 1 新增
```

---

## 六、render crate — 渲染后端扩展

### 6.1 模块结构

```
render/
├── Cargo.toml
├── shaders/
│   ├── rect.wgsl          # Phase 0 已有
│   ├── border.wgsl        # Phase 1: 边框着色器（支持圆角和样式）
│   ├── image.wgsl         # Phase 1: 图像纹理着色器
│   └── shadow.wgsl        # Phase 1: 阴影着色器
└── src/
    ├── lib.rs              # RenderBackend trait + 公开导出
    ├── wgpu_backend.rs     # 新增多渲染管线 + 裁剪/透明度栈
    └── text_renderer.rs    # 完整 rustybuzz 字形渲染 + SDF 模式
```

### 6.2 WgpuBackend — 新增管线

```rust
// === crates/render/src/wgpu_backend.rs ===

pub struct WgpuBackend {
    // Phase 0 已有字段
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (u32, u32),
    rect_pipeline: wgpu::RenderPipeline,
    text_renderer: TextRenderer,

    // Phase 1 新增
    /// 边框渲染管线
    border_pipeline: wgpu::RenderPipeline,
    /// 阴影渲染管线
    shadow_pipeline: wgpu::RenderPipeline,
    /// 图像纹理渲染管线
    image_pipeline: wgpu::RenderPipeline,
    /// 裁剪矩形栈（用于 Clip 命令的嵌套裁剪）
    clip_stack: Vec<Rect<f32>>,
    /// 透明度栈（用于 Opacity 命令的嵌套透明度）
    opacity_stack: Vec<f32>,
    /// 图像纹理缓存
    texture_cache: HashMap<ImageId, wgpu::Texture>,
}

impl WgpuBackend {
    // ===== Phase 0 已有 =====
    // new / init_device / encode_rect / encode_text

    // ============================================================
    //  Phase 1 新增 —— 管线创建
    // ============================================================

    /// 创建边框渲染管线（WGSL 着色器：border.wgsl）
    fn create_border_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline;

    /// 创建阴影渲染管线（WGSL 着色器：shadow.wgsl）
    fn create_shadow_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline;

    /// 创建图像渲染管线（WGSL 着色器：image.wgsl）
    fn create_image_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline;

    // ============================================================
    //  Phase 1 新增 —— 渲染编码
    // ============================================================

    /// 编码边框绘制命令（支持圆角和样式）
    fn encode_border(
        &self,
        pass: &mut wgpu::RenderPass,
        cmd: &PaintCommand,
    );

    /// 编码阴影绘制命令（支持模糊和扩散）
    fn encode_shadow(
        &self,
        pass: &mut wgpu::RenderPass,
        cmd: &PaintCommand,
    );

    /// 编码图像绘制命令
    fn encode_image(
        &mut self,
        pass: &mut wgpu::RenderPass,
        cmd: &PaintCommand,
    );

    /// 推入裁剪区域
    fn push_clip(&mut self, rect: Rect<f32>, pass: &mut wgpu::RenderPass);

    /// 弹出裁剪区域
    fn pop_clip(&mut self, pass: &mut wgpu::RenderPass);

    /// 设置渲染透明度
    fn push_opacity(&mut self, opacity: f32);

    /// 恢复透明度
    fn pop_opacity(&mut self);

    // ============================================================
    //  Phase 1 新增 —— 纹理管理
    // ============================================================

    /// 从文件加载 PNG/JPEG 图像并上传为 GPU 纹理
    fn load_image_texture(
        &mut self,
        path: &str,
    ) -> Result<ImageId, TextureError>;

    /// 获取已缓存的纹理
    fn get_texture(&self, image_id: ImageId) -> Option<&wgpu::Texture>;

    /// 从内存中的图像数据创建纹理
    fn create_texture_from_bytes(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<ImageId, TextureError>;
}

pub enum TextureError {
    FileNotFound(String),
    DecodeError(String),
    UploadError(String),
}
```

### 6.3 TextRenderer — 完整字形渲染管线

```rust
// === crates/render/src/text_renderer.rs ===

pub struct TextRenderer {
    // Phase 0 已有字段
    glyph_cache: HashMap<(u16, u16), GlyphTexture>,
    font_cache: HashMap<String, rustybuzz::Face>,
    font_db: fontdb::Database,
    atlas_texture: wgpu::Texture,
    atlas_position: (u16, u16),

    // Phase 1 新增
    /// SDF 渲染管线（可选）
    sdf_pipeline: Option<wgpu::RenderPipeline>,
    /// SDF 模式开关
    sdf_enabled: bool,
    /// 字形 shaping 缓冲区
    shaping_buffer: rustybuzz::UnicodeBuffer,
    /// 文本渲染管线（bitmap 模式）
    text_pipeline: wgpu::RenderPipeline,
}

impl TextRenderer {
    // ===== Phase 0 已有 =====
    // new / render_text / get_or_cache_glyph

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 使用 rustybuzz 做完整字形 shaping
    ///
    /// 替代 Phase 0 的简单字符宽度估算
    /// 返回：字形序列（含位置、步进、簇索引）
    pub fn shape_text(
        &mut self,
        text: &str,
        font_size: f32,
        font_family: &str,
        font_weight: u16,
        font_style: FontStyle,
    ) -> Vec<ShapedGlyph>;

    /// 切换到 SDF（Signed Distance Field）渲染模式
    ///
    /// SDF 优势：缩放时字形边缘保持清晰，节省纹理空间
    pub fn set_sdf_mode(&mut self, enabled: bool);

    /// 生成 SDF 字形纹理
    ///
    /// sdf_radius: SDF 扩散半径（像素），值越大边缘越柔和
    fn generate_sdf_glyph(
        &mut self,
        face: &rustybuzz::Face,
        glyph_id: u16,
        font_size: f32,
        sdf_radius: f32,
    ) -> GlyphTexture;

    /// 从 fontdb 加载字体
    fn load_font(&mut self, family: &str, weight: u16, style: FontStyle) -> Option<&rustybuzz::Face>;
}

/// 字形 shaping 结果
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    /// 字形 ID
    pub glyph_id: u16,
    /// 水平偏移（相对基线原点）
    pub x_offset: f32,
    /// 垂直偏移（相对基线原点）
    pub y_offset: f32,
    /// 水平步进（到下一个字形）
    pub x_advance: f32,
    /// 垂直步进
    pub y_advance: f32,
    /// 原始文本簇索引
    pub cluster: usize,
}

/// 字体样式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}
```

### 6.4 render 公开导出更新

```rust
// === crates/render/src/lib.rs ===

pub use wgpu_backend::WgpuBackend;
pub use text_renderer::{TextRenderer, ShapedGlyph, FontStyle};

// Phase 2+:
// pub use wgpu_backend::TextureError;
```

---

## 七、runtime crate — 运行时扩展

### 7.1 模块结构

```
runtime/
├── Cargo.toml
└── src/
    ├── lib.rs              # 公开导出更新
    ├── window.rs           # WebWindow 扩展（样式缓存/选择器引擎）
    ├── event_loop.rs       # AnimationFrameScheduler + 完整事件转换
    └── hit_test.rs         # 多点触摸 + 事件冒泡路径
```

### 7.2 WebWindow — 扩展

```rust
// === crates/runtime/src/window.rs ===

pub struct WebWindow {
    // Phase 0 已有字段
    title: String,
    size: (u32, u32),
    event_loop: Option<winit::event_loop::EventLoop<()>>,
    document: Rc<RefCell<Document>>,
    renderer: Option<WgpuBackend>,
    layout_engine: LayoutEngine,
    text_measurer: TextMeasurer,
    needs_redraw: bool,

    // Phase 1 新增
    /// 外部样式表缓存
    stylesheets: Vec<StyleSheet>,
    /// 计算后样式缓存（key = Rc::as_ptr(node) as usize）
    computed_styles: HashMap<usize, ComputedStyle>,
    /// 选择器引擎
    selector_engine: SelectorEngine,
    /// 动画帧调度器
    animation_scheduler: AnimationFrameScheduler,
}

impl WebWindow {
    // ===== Phase 0 已有 =====
    // new / document / run / request_redraw

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 加载外部 CSS 样式表，返回样式表索引
    pub fn load_stylesheet(&mut self, css_text: &str) -> usize;

    /// 移除指定索引的样式表
    pub fn remove_stylesheet(&mut self, index: usize);

    /// 使指定节点的计算样式缓存失效
    pub fn invalidate_style_cache(&mut self, node_ptr: usize);

    /// 注册 requestAnimationFrame 回调
    /// 返回回调 ID（用于取消）
    pub fn request_animation_frame(
        &mut self,
        callback: Box<dyn FnMut(f64)>,
    ) -> u32;

    /// 取消 requestAnimationFrame 回调
    pub fn cancel_animation_frame(&mut self, id: u32);

    // ============================================================
    //  内部方法（Phase 1 增强）
    // ============================================================

    /// 完整渲染管线（含样式缓存检查）
    fn render_pipeline(&mut self);

    /// 增量渲染：仅重绘脏区域
    fn incremental_render(&mut self, dirty_rects: &[Rect<f32>]);
}
```

### 7.3 EventLoopRunner — 事件处理器 + AnimationFrameScheduler

```rust
// === crates/runtime/src/event_loop.rs ===

/// requestAnimationFrame 调度器（Phase 1 实现）
pub struct AnimationFrameScheduler {
    /// 帧回调队列 (回调 ID, 回调函数)
    frame_callbacks: Vec<(u32, Box<dyn FnMut(f64)>)>,
    /// 下一帧 ID
    next_frame_id: u32,
    /// 已取消的回调 ID 集合
    cancelled_ids: HashSet<u32>,
}

impl AnimationFrameScheduler {
    /// 创建调度器
    pub fn new() -> Self;

    /// 注册帧回调
    /// 对应 window.requestAnimationFrame(callback)
    pub fn request_animation_frame(&mut self, callback: Box<dyn FnMut(f64)>) -> u32;

    /// 取消帧回调
    /// 对应 window.cancelAnimationFrame(id)
    pub fn cancel_animation_frame(&mut self, id: u32);

    /// 执行所有待处理的帧回调
    /// timestamp: 高精度时间戳（毫秒）
    pub fn tick(&mut self, timestamp: f64);

    /// 是否有待处理的回调
    pub fn has_pending(&self) -> bool;
}

// ============================================================
//  事件转换映射表（Phase 1 完善）
// ============================================================

impl EventLoopRunner {
    // winit 事件 → DOM 事件：
    //
    // winit::WindowEvent::CursorMoved       → MouseEvent("mousemove")
    // winit::WindowEvent::MouseInput         → MouseEvent("mousedown"/"mouseup"/"click")
    // winit::WindowEvent::KeyboardInput      → KeyboardEvent("keydown"/"keyup")  [Phase 1 新增]
    // winit::WindowEvent::MouseWheel         → WheelEvent("wheel")               [Phase 1 新增]
    // winit::WindowEvent::Focused(true)      → FocusEvent("focus")               [Phase 1 新增]
    // winit::WindowEvent::Focused(false)     → FocusEvent("blur")                [Phase 1 新增]
    // winit::WindowEvent::Touch(touch)       → TouchEvent（Phase 2+）

    /// 将 winit KeyboardInput 转为 DOM KeyboardEvent
    fn convert_keyboard_event(
        window: &WebWindow,
        event: &winit::event::KeyEvent,
    );

    /// 将 winit MouseWheel 转为 DOM WheelEvent
    fn convert_wheel_event(
        window: &WebWindow,
        delta: &winit::event::MouseScrollDelta,
    );
}
```

### 7.4 HitTester — 增强

```rust
// === crates/runtime/src/hit_test.rs ===

impl HitTester {
    // ===== Phase 0 已有 =====
    // hit_test / collect_bubble_path

    // ============================================================
    //  Phase 1 新增
    // ============================================================

    /// 多点触摸命中检测
    pub fn multi_hit_test<'a>(
        root: &'a LayoutBox,
        points: &[(f32, f32)],
    ) -> Vec<Option<&'a LayoutBox>>;

    /// 计算从根到目标的完整事件传播路径
    /// 返回：从 document_element 到 target 的路径（含中间所有祖先）
    pub fn compute_event_path<'a>(
        root: &'a LayoutBox,
        target: &'a LayoutBox,
    ) -> Vec<&'a LayoutBox>;

    /// 获取指定坐标下的光标样式
    pub fn get_cursor_style(
        root: &LayoutBox,
        x: f32,
        y: f32,
    ) -> Option<String>;

    // Phase 2+:
    // pub fn find_scroll_container<'a>(root: &'a LayoutBox, x: f32, y: f32) -> Option<&'a LayoutBox>;
}
```

### 7.5 runtime 公开导出更新

```rust
// === crates/runtime/src/lib.rs ===

pub use window::WebWindow;
pub use event_loop::AnimationFrameScheduler;   // Phase 1 新增
pub use hit_test::HitTester;                    // Phase 1 提升为公开
```

---

## 八、Phase 1 完整调用链路

### 8.1 HTML 文件加载到渲染

```
WebWindow::load_stylesheet(css_text)
  ├── parse_stylesheet(css_text) → StyleSheet
  ├── stylesheets.push(stylesheet)
  └── invalidate_style_cache(ALL)
        │
        ▼
render_pipeline()
  ├── 遍历 DOM 树（仅脏节点）
  │     ├── compute_element_style(element, parent_style, &stylesheets, inline_style)
  │     │     ├── match_selectors(element, &stylesheets) → Vec<MatchedDeclaration>
  │     │     ├── cascade_sort(&mut declarations)
  │     │     ├── merge into ComputedStyle
  │     │     └── apply_inheritance(parent, &mut computed)
  │     └── computed_styles.insert(node_ptr, computed)
  ├── build_layout_tree(&dom_root, &computed_styles)
  │     └── 根据 display 确定 BoxType (Block|Inline|InlineBlock|FlexContainer|Table|etc)
  ├── LayoutEngine::layout(&mut layout_root, viewport)
  │     ├── FlexLayout::layout() → taffy (含 flex-wrap + min/max 约束)
  │     ├── BlockLayout::layout() → margin collapse
  │     ├── InlineLayout::layout() → 行内排列 + 文本换行
  │     ├── TableLayout::layout() → 4-pass 表格算法
  │     └── PositionedLayout::layout() → fixed/sticky
  ├── DisplayListBuilder::build(&layout_root) → DisplayList
  │     ├── FillRect (含 border_radius)
  │     ├── Border (含 style)
  │     ├── Text (含 text_decoration)
  │     ├── BoxShadow, Image, Clip, Opacity
  │     └── extract from ComputedStyle (background-color/background-image/border/*/etc)
  ├── BatchOptimizer::optimize(&display_list) → Vec<RenderBatch>
  └── WgpuBackend::render(&display_list)
        ├── for each PaintCommand:
        │     ├── FillRect → rect_pipeline → encode_rect()
        │     ├── Text → text_renderer.render_text() → SDF or bitmap
        │     ├── Border → border_pipeline → encode_border()
        │     ├── BoxShadow → shadow_pipeline → encode_shadow()
        │     ├── Image → image_pipeline → encode_image()
        │     ├── Clip → push_clip() → render children → pop_clip()
        │     └── Opacity → push_opacity() → render children → pop_opacity()
        └── queue.submit()
```

### 8.2 事件冒泡完整流程

```
winit event → EventLoopRunner::convert_winit_event() → DOM Event
  │
  ▼
EventDispatcher::dispatch(target, event)
  ├── 1. build_path(target) → [document_element, ..., parent, target]
  │       │
  ├── 2. Capture Phase (自上而下, event.phase = CapturingPhase)
  │       for node in path[0..path.len()-1] (root to parent):
  │           for listener in node.events[event_type]:
  │               if listener.options.capture:
  │                   listener.callback(event)
  │                   if event.propagation_stopped() → break
  │       │
  ├── 3. Target Phase (event.phase = AtTarget)
  │       for listener in target.events[event_type]:
  │           listener.callback(event)
  │           if event.immediate_propagation_stopped() → break
  │       │
  └── 4. Bubble Phase (自下而上, event.phase = BubblingPhase)
          if event.bubbles && !event.propagation_stopped():
              for node in path[0..path.len()-1].reverse() (parent to root):
                  for listener in node.events[event_type]:
                      if !listener.options.capture:
                          listener.callback(event)
                          if event.propagation_stopped() → break
```

### 8.3 增量渲染帧

```
用户交互 → DOM 变更
  ├── node.set_text_content("1")
  │     └── MutationObserver.queue_record(MutationRecord { type: CharacterData, ... })
  ├── node.borrow_mut().mark_dirty(true)
  │
  ▼
WebWindow::request_redraw()
  │
  ▼
incremental_render(dirty_rects)
  ├── LayoutEngine::partial_layout(&mut root, &dirty_paths, viewport)
  │     └── 仅重排脏子树，返回受影响区域
  ├── DisplayListBuilder::build_dirty(&mut root, &dirty_nodes)
  │     └── 仅重建脏区域的 PaintCommand
  └── WgpuBackend::render_dirty(&display_list, &dirty_rects)
        └── GPU 裁剪同步 → 仅重绘受影响像素
```

### 8.4 requestAnimationFrame 动画帧

```
WebWindow::request_animation_frame(callback)
  └── animation_scheduler.request_animation_frame(callback)
        │  返回 callback_id
        │
        ▼ (下一帧)
  animation_scheduler.tick(timestamp)
        │
        ▼
  for (id, callback) in pending_callbacks:
      callback(timestamp)       // 用户代码中修改 DOM
      │
      ▼
  request_redraw() → incremental_render()
```

---

## 九、web2rust crate — 编译器重构

### 9.1 模块结构

```
web2rust/
├── Cargo.toml              # 新增: html5ever, cssparser, selectors, swc 依赖
└── src/
    ├── lib.rs              # compile() / compile_body() 入口 + 重构后的代码生成
    ├── parser.rs           # 新文件：统一解析调度（html5ever + cssparser + swc）
    ├── html.rs             # 重构为 html5ever 解析 → DOM 构建代码
    ├── css.rs              # 重构为 cssparser + selectors 解析 → StyleSheet 注册代码
    ├── js.rs               # 重构为 swc 完整 AST 编译 → Rust 代码
    ├── analyzer.rs         # 新文件：JS 语义分析（作用域/类型推导/DOM 绑定）
    ├── codegen.rs          # 新文件：Rust 代码生成器
    └── builtins.rs         # 新文件：内置对象映射表
```

### 9.2 parser.rs — 统一解析

```rust
// === crates/web2rust/src/parser.rs ===

/// 三位一体的解析结果
pub struct ParsedDocument {
    /// HTML 解析结果
    pub html_elements: Vec<HtmlElement>,
    /// CSS 解析结果
    pub css_rules: Vec<CssRule>,
    /// JS AST
    pub js_ast: Option<swc_ecma_ast::Module>,
}

/// 统一解析入口：同时解析 HTML + CSS + JS
pub fn parse_all(
    html: &str,
    css: &str,
    js: &str,
) -> Result<ParsedDocument, ParseError>;

/// 使用 html5ever 解析 HTML，返回元素树
pub fn parse_html(html: &str) -> Result<Vec<HtmlElement>, ParseError>;

/// 使用 cssparser + selectors 解析 CSS
pub fn parse_css(css: &str) -> Result<Vec<CssRule>, ParseError>;

/// 使用 swc 解析 JS 为 AST
pub fn parse_js(js: &str) -> Result<swc_ecma_ast::Module, ParseError>;

/// 解析错误
pub enum ParseError {
    HtmlError(String),
    CssError(String),
    JsError(String),
    IoError(std::io::Error),
}
```

### 9.3 js.rs — swc 完整 AST 编译

```rust
// === crates/web2rust/src/js.rs ===

/// JS→Rust 编译结果
pub struct CompileJsResult {
    /// 事件处理器列表
    pub event_handlers: Vec<EventHandler>,
    /// 状态变量（需要声明为 let mut）
    pub state_variables: Vec<StateVariable>,
    /// 辅助函数
    pub functions: Vec<RustFunction>,
    /// DOM 绑定信息
    pub dom_bindings: Vec<DomBinding>,
}

/// 状态变量
pub struct StateVariable {
    pub name: String,
    pub init_value: String,
    pub is_mutable: bool,
}

/// Rust 函数生成
pub struct RustFunction {
    pub name: String,
    pub params: Vec<(String, String)>,  // (name, type)
    pub return_type: Option<String>,
    pub body: String,
}

/// DOM 绑定
pub struct DomBinding {
    pub element_var: String,
    pub dom_api: DomApiCall,
}

/// DOM API 调用类型
pub enum DomApiCall {
    CreateElement(String),
    SetAttribute(String, String),
    SetTextContent(String),
    SetStyle(String),
    AddEventListener(String, Vec<String>),  // event_type, callback_body
    AppendChild(String),                    // parent_var
}

/// 使用 swc 编译 JS 模块
pub fn compile_js_ast(
    module: &swc_ecma_ast::Module,
    element_vars: &[(String, HtmlElement)],
) -> CompileJsResult;

/// Phase 1 JS 编译能力表：
pub fn get_phase1_js_capabilities() -> JsCompileCapabilities;
```

### 9.4 Phase 1 JS 编译支持表

| JS 语法 | Rust 映射 | 说明 |
|----------|-----------|------|
| `let x = val` | `let x = val;` | 不可变绑定 |
| `const x = val` | `let x = val;` | Rust 无 const 变量概念 |
| `var x = val` | `let mut x = val;` | var 总是可变的 |
| `function f(a, b) { stmts }` | `fn f(a: JsValue, b: JsValue) -> JsValue { stmts }` | 箭头函数同理 |
| `if/else` | `if { } else { }` | 直接映射 |
| `for (let x of arr)` | `for x in arr.into_iter()` | 迭代器映射 |
| `while (cond) { }` | `while cond { }` | 直接映射 |
| `switch (expr) { case: ... }` | `match expr { _ => {} }` | 近似映射 |
| `{ key: val }` | `JsValue::Object(HashMap::from([...]))` | 运行时对象 |
| `[1, 2, 3]` | `JsValue::Array(vec![...])` | 运行时数组 |
| `"hello"` | `"hello".to_string()` | Rust String |
| `42` | `JsValue::Number(42.0)` | 所有数字为 f64 |
| `true / false` | `JsValue::Bool(true)` | 布尔值 |
| `null / undefined` | `JsValue::Null / JsValue::Undefined` | 空值 |
| `console.log(x)` | `println!("{}", x)` | 调试输出 |
| `console.error(x)` | `eprintln!("{}", x)` | 错误输出 |
| `setTimeout(fn, ms)` | `std::thread::spawn(move || { sleep; fn(); })` | 延迟执行 |
| `Promise.resolve(x)` | Phase 1: 同步立即返回 / Phase 2+: Future | 基础支持 |
| `class MyClass { ... }` | Phase 1: struct + impl / Phase 2+: 完整 | 基础支持 |
| `Symbol('desc')` | `JsValue::Symbol(String)` | 符号值 |
| `for...of iterable` | Rust for loop over iterator | 迭代 |
| `import { x } from './mod'` | `mod mod; use mod::x;` | 模块系统 |
| `parseInt(x)` | `x.parse::<i32>().unwrap_or(0)` | 类型转换 |
| `x.toString()` | `x.to_string()` | 字符串化 |

### 9.5 analyzer.rs — JS 语义分析

```rust
// === crates/web2rust/src/analyzer.rs ===

/// JS 语义分析器
pub struct JsAnalyzer;

impl JsAnalyzer {
    /// 分析整个 JS 模块
    /// 推导变量类型、构建作用域树、识别 DOM API 调用
    pub fn analyze(module: &swc_ecma_ast::Module) -> AnalysisResult;

    /// 构建作用域树（变量声明 → 作用域块 → 父子关系）
    fn build_scope_tree(module: &swc_ecma_ast::Module) -> ScopeTree;

    /// 识别所有 DOM API 调用
    fn identify_dom_operations(module: &swc_ecma_ast::Module) -> Vec<DomOperation>;

    /// 推导变量类型（名称 → JsType）
    fn infer_types(module: &swc_ecma_ast::Module) -> HashMap<String, JsType>;
}

/// 分析结果
pub struct AnalysisResult {
    /// 作用域树
    pub scopes: ScopeTree,
    /// DOM 操作列表
    pub dom_ops: Vec<DomOperation>,
    /// 变量类型映射
    pub variable_types: HashMap<String, JsType>,
}

/// 作用域树
pub struct ScopeTree {
    pub root: Scope,
}

pub struct Scope {
    pub variables: Vec<String>,
    pub children: Vec<Scope>,
}

/// DOM 操作
pub struct DomOperation {
    /// 操作类型
    pub operation: DomOperationType,
    /// 目标元素变量名
    pub target_var: String,
    /// 操作参数
    pub params: Vec<String>,
}

pub enum DomOperationType {
    CreateElement,
    SetAttribute,
    SetTextContent,
    AddEventListener,
    AppendChild,
    QuerySelector,
    GetElementById,
    SetStyle,
    RemoveChild,
    ReplaceChild,
}

/// JS 类型（Phase 1 推断）
pub enum JsType {
    String,
    Number,
    Boolean,
    Object,
    Array,
    Function,
    Null,
    Undefined,
    DomElement,
    Unknown,
}
```

### 9.6 codegen.rs — Rust 代码生成器

```rust
// === crates/web2rust/src/codegen.rs ===

/// Rust 代码生成器
pub struct RustCodegen {
    /// 生成的代码缓冲区
    output: String,
    /// 缩进级别
    indent_level: usize,
}

impl RustCodegen {
    /// 创建生成器
    pub fn new() -> Self;

    /// 从分析结果生成完整 main() 函数体
    pub fn generate(
        &mut self,
        html_elements: &[HtmlElement],
        css_matches: &[(String, String)],
        analysis: &AnalysisResult,
    ) -> String;

    /// 生成创建元素的代码
    fn emit_create_element(&mut self, element: &HtmlElement, var_name: &str);

    /// 生成样式应用代码
    fn emit_style_application(&mut self, var_name: &str, style_str: &str);

    /// 生成事件监听器闭包
    fn emit_event_handler(
        &mut self,
        target_var: &str,
        event_type: &str,
        handler: &EventHandler,
    );

    /// 生成函数定义
    fn emit_function(&mut self, func: &RustFunction);

    /// 增加缩进
    fn indent(&mut self);
    /// 减少缩进
    fn dedent(&mut self);
    /// 写入带缩进的一行
    fn emit_line(&mut self, line: &str);
}
```

### 9.7 builtins.rs — 内置对象映射

```rust
// === crates/web2rust/src/builtins.rs ===

/// 内置对象/API 映射表
pub struct BuiltinRegistry;

impl BuiltinRegistry {
    /// 获取内置对象的 Rust 代理信息
    pub fn get_builtin(name: &str) -> Option<BuiltinInfo>;
}

pub struct BuiltinInfo {
    /// Rust 中的替代类型
    pub rust_type: String,
    /// Rust 中的替代函数
    pub rust_function: Option<String>,
    /// 是否需要额外的 use 语句
    pub required_use: Option<String>,
}

// ============================================================
//  内置映射表（Phase 1）
// ============================================================
//
// console.log(x)       → println!("{}", x)
// console.error(x)     → eprintln!("{}", x)
// console.warn(x)      → eprintln!("[WARN] {}", x)
//
// setTimeout(fn, ms)   → std::thread::spawn(move || {
//                             std::thread::sleep(Duration::from_millis(ms));
//                             fn();
//                         })
//
// setInterval(fn, ms)  → loop { fn(); std::thread::sleep(...); }
//                         Phase 2+: 使用 tokio 间隔定时器
//
// JSON.parse(s)        → serde_json::from_str(&s).unwrap()
// JSON.stringify(v)    → serde_json::to_string(&v).unwrap()
//
// Math.random()        → rand::random::<f64>()
// Math.floor(x)        → (x as f64).floor()
// Math.ceil(x)         → (x as f64).ceil()
// Math.round(x)        → (x as f64).round()
// Math.abs(x)          → (x as f64).abs()
// Math.max(a, b)       → a.max(b)  (for f64)
// Math.min(a, b)       → a.min(b)  (for f64)
// Math.PI              → std::f64::consts::PI
//
// Date.now()           → std::time::SystemTime::now()
//                             .duration_since(UNIX_EPOCH)
//                             .unwrap().as_millis() as f64
//
// parseInt(s)          → s.parse::<i32>().unwrap_or(0)
// parseFloat(s)        → s.parse::<f64>().unwrap_or(0.0)
// isNaN(x)             → x.is_nan()  (for f64)
```

---

## 十、Phase 1 → Phase 2 扩展点汇总

| 文件位置 | `// Phase 2+` 标记 | 即将新增 |
|----------|-------------------|----------|
| `dom/src/node.rs` | DocumentFragment/Comment 完善 | lookup_prefix / lookup_namespace_uri |
| `dom/src/element.rs` | inner_html 完整序列化 | scroll_into_view / scroll_to / scroll_by |
| `dom/src/document.rs` | create_element_ns / import_node | adopt_node / create_element_ns |
| `dom/src/event.rs` | AnimationEvent / TransitionEvent / InputEvent | 动画/过渡事件 |
| `dom/src/mutation_observer.rs` | 批量异步回调（微任务队列） | queue_microtask |
| `dom/src/html/` | 新目录 | HTMLAnchorElement / HTMLImageElement / HTMLInputElement / HTMLCanvasElement 等 |
| `css/properties.toml` | ~200 属性 | animation-* / transition-* / grid-* / filter / box-shadow / 自定义属性 |
| `css/src/selector.rs` | 完整伪类 | :hover / :focus / :nth-child(n) / :not() / :checked 等 |
| `css/src/cascade.rs` | 用户样式层 | 样式失效检测 / revert 关键字 |
| `css/src/values.rs` | Transform 3D / Filter | Matrix3d / RotateX/Y/Z / Blur / Brightness |
| `css/src/animations.rs` | 新文件 | CSS 动画引擎（关键帧插值+时间线） |
| `css/src/transitions.rs` | 新文件 | CSS 过渡引擎 |
| `css/src/media.rs` | 新文件 | 媒体查询求值引擎 |
| `css/src/custom_props.rs` | 新文件 | CSS 自定义属性（--*）处理 |
| `layout/src/layout_box.rs` | GridContainer / GridItem / Float | BoxType 新变体 |
| `layout/src/grid.rs` | 新文件 | GridLayout（taffy Grid 集成） |
| `layout/src/float.rs` | 新文件 | FloatLayout（浮动布局 + clear） |
| `layout/src/table.rs` | colspan/rowspan | 跨单元格支持 / border-collapse 合并 |
| `paint/src/command.rs` | Transform / Filter 命令 | 变换 + 滤镜命令 |
| `paint/src/builder.rs` | Gradient / Filter 处理 | process_gradient / process_filter |
| `render/src/wgpu_backend.rs` | TextureError 公开 | 图像解码管线完善 |
| `render/src/text_renderer.rs` | SDF 完善 | 多字体回退 / 图集动态增长 |
| `runtime/src/window.rs` | scroll 支持 | IntersectionObserver / ResizeObserver |
| `runtime/src/event_loop.rs` | TouchEvent / scroll | 触摸事件 + 滚动事件 |
| `runtime/src/hit_test.rs` | scroll container | 可滚动区域检测 |
| `net/` | 全新 crate | fetch API + WebSocket 基础 |
| `storage/` | 全新 crate | localStorage / sessionStorage |
| `web2rust/src/js.rs` | Promise 完整 / Proxy / TypedArray | 完整 async/Future 编译 |
| `web2rust/src/analyzer.rs` | 复杂类型推导 | 泛型 + union type 推断 |

---

## 统计

| 指标 | Phase 0 | Phase 1 | 增量 |
|------|---------|---------|------|
| 代码量（预估） | ~9,000 行 | ~17,000 行 | +8,000 行 |
| CSS 属性 | 30 个 | 80 个 | +50 个 |
| 公开 API 数量 | ~80 个 | ~200 个 | +120 个 |
| crate 模块文件数 | 27 个 | 37 个 | +10 个 |
| 测试预估 | 156 个 | ~300 个 | +~150 个 |
| JS 编译能力 | 5 种模式 | 20+ 种语法 | - |
