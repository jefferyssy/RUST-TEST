# Rust 浏览器引擎 — Phase 0 API 设计文档

> 本文档定义 Phase 0 所有公开类型、函数签名及作用。
> 中文注解标注功能说明，`// Phase N+` 标记为后续阶段预留的扩展点。

---

## 一、模块总览

```
┌──────────────────────────────────────────────────────────────────┐
│  Phase 0 模块依赖关系                                             │
│                                                                  │
│  examples/counter                                                │
│       │  (build.rs 调用 web2rust 编译源文件)                       │
│       ▼                                                          │
│  ┌───────────┐                                                   │
│  │ web2rust  │  ← 编译器（build-dependency, 无外部依赖）           │
│  │ (构建时)   │     HTML+CSS+JS → Rust 代码                       │
│  └───────────┘                                                   │
│       │  生成 main() 函数体                                       │
│       ▼                                                          │
│  runtime (主循环 + 事件)                                          │
│       │                                                          │
│       ├── render (wgpu 渲染后端)                                  │
│       │       │                                                   │
│       │       ▼                                                   │
│       │   paint (DisplayList 绘制命令)                             │
│       │                                                          │
│       ├── layout (布局引擎)                                        │
│       │       │                                                   │
│       │       ├── css (CSS 引擎)                                  │
│       │       │       │                                           │
│       │       │       ▼                                           │
│       │       └── dom (DOM 树)                                    │
│       │                                                          │
│       └── dom (DOM 树 - 同上引用)                                 │
└──────────────────────────────────────────────────────────────────┘
```

### Phase 0 实现原则

1. **最小可用**：只实现计数器 Demo 必需的 API
2. **预留扩展点**：用 `// Phase 1+` 标记后续阶段位置
3. **W3C 命名**：函数名与浏览器标准 DOM API 保持一致
4. **中文注解**：所有文档注释使用中文

---

## 二、dom crate — W3C DOM 标准实现

### 2.1 模块结构

```
dom/
├── Cargo.toml
└── src/
    ├── lib.rs              # 模块导出 + 公开类型重导出
    ├── node.rs             # Node + NodeType 定义
    ├── element.rs          # Element
    ├── document.rs         # Document
    ├── text.rs             # Text 节点
    ├── event.rs            # Event / EventTarget / MouseEvent
    └── dom_token_list.rs   # DOMTokenList (classList)
```

### 2.2 NodeType — 节点类型枚举

```rust
// === crates/dom/src/node.rs ===

/// 节点类型枚举 —— 对应 W3C DOM 标准
pub enum NodeType {
    /// 元素节点：<div>, <h1>, <button> 等
    Element(Element),
    /// 文本节点：元素的文本内容
    Text(Text),
    /// 文档根节点
    Document,
    /// 文档片段（Phase 1+ 实现批量插入）
    // DocumentFragment,  // Phase 1+: 暂不实现
}

/// W3C 标准节点类型常量：与 browser API 保持一致
pub mod node_type_constants {
    pub const ELEMENT_NODE: u16 = 1;
    pub const TEXT_NODE: u16 = 3;
    pub const DOCUMENT_NODE: u16 = 9;
    // Phase 1+:
    // pub const COMMENT_NODE: u16 = 8;
    // pub const DOCUMENT_FRAGMENT_NODE: u16 = 11;
    // pub const DOCUMENT_TYPE_NODE: u16 = 10;
}
```

### 2.3 Node — 节点核心

```rust
// === crates/dom/src/node.rs ===

use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// DOM 节点 —— 树结构核心类型
///
/// 使用 Rc<RefCell<Node>> 管理共享所有权
/// parent 使用 Weak 避免循环引用
/// 兄弟节点通过 prev_sibling(Weak) + next_sibling(Rc) 构成双向链表
pub struct Node {
    /// 节点类型：Element / Text / Document
    pub(crate) node_type: NodeType,
    /// 父节点（Weak 防循环引用）
    parent: Option<Weak<RefCell<Node>>>,
    /// 子节点列表（有序）
    children: Vec<Rc<RefCell<Node>>>,
    /// 前一个兄弟节点（Weak 防循环引用）
    prev_sibling: Option<Weak<RefCell<Node>>>,
    /// 后一个兄弟节点
    next_sibling: Option<Rc<RefCell<Node>>>,
    /// 变更回调 —— 节点修改时通知 layout 引擎重排
    /// Phase 1+: 改为 MutationObserver 机制
    pub(crate) dirty: Cell<bool>,
}

impl Node {
    // ============================================================
    //  构造函数
    // ============================================================

    /// 创建新节点（内部使用，通过 Document 创建对外）
    pub fn new(node_type: NodeType) -> Rc<RefCell<Self>>;

    // ============================================================
    //  树操作 —— W3C DOM 标准
    // ============================================================

    /// 追加子节点到末尾
    /// 返回被追加的子节点（链式调用）
    /// 如果 child 已有父节点，自动从原位置移除
    pub fn append_child(&mut self, child: Rc<RefCell<Node>>) -> Rc<RefCell<Node>>;

    /// 移除指定子节点
    /// 返回被移除的节点
    /// Panics: child 不是直接子节点
    pub fn remove_child(&mut self, child: &Node) -> Rc<RefCell<Node>>;

    /// 在参考节点之前插入新节点
    /// reference_node = None 等价于 append_child
    pub fn insert_before(
        &mut self,
        new_node: Rc<RefCell<Node>>,
        reference_node: Option<&Node>,
    ) -> Rc<RefCell<Node>>;

    /// 用新节点替换旧节点
    pub fn replace_child(
        &mut self,
        new_child: Rc<RefCell<Node>>,
        old_child: &Node,
    ) -> Rc<RefCell<Node>>;

    /// 判断 other 是否是本节点的后代（包含自身）
    pub fn contains(&self, other: &Node) -> bool;

    /// 克隆节点
    /// deep=true: 递归克隆子树; deep=false: 只克隆自身
    /// Phase 1+: 实现完整克隆
    pub fn clone_node(&self, deep: bool) -> Rc<RefCell<Node>>;

    // ============================================================
    //  属性访问 —— 对应 W3C Node 属性
    // ============================================================

    /// 获取节点所有子元素的文本内容拼接
    pub fn text_content(&self) -> String;

    /// 设置文本内容（替换所有子节点为单个 Text 节点）
    pub fn set_text_content(&mut self, text: &str);

    /// 父节点（仅 Node 和 Element 有父节点）
    pub fn parent_node(&self) -> Option<Rc<RefCell<Node>>>;

    /// 子节点列表的拷贝
    pub fn child_nodes(&self) -> Vec<Rc<RefCell<Node>>>;

    /// 第一个子节点
    pub fn first_child(&self) -> Option<Rc<RefCell<Node>>>;

    /// 最后一个子节点
    pub fn last_child(&self) -> Option<Rc<RefCell<Node>>>;

    /// 前一个兄弟节点
    pub fn previous_sibling(&self) -> Option<Rc<RefCell<Node>>>;

    /// 后一个兄弟节点
    pub fn next_sibling(&self) -> Option<Rc<RefCell<Node>>>;

    /// 节点类型数字常量（对应 node_type_constants）
    pub fn node_type(&self) -> u16;

    /// 节点名称：
    ///   Element → 大写标签名 "DIV"
    ///   Text → "#text"
    ///   Document → "#document"
    pub fn node_name(&self) -> String;

    /// 子节点数量
    pub fn child_element_count(&self) -> usize;

    /// 标记节点为脏（需要重排）
    /// Phase 1+: 用 MutationObserver 替代
    pub(crate) fn mark_dirty(&self, dirty: bool);

    /// 是否脏节点
    pub(crate) fn is_dirty(&self) -> bool;

    // Phase 1+:
    // pub fn owner_document(&self) -> Option<Rc<RefCell<Document>>>;
    // pub fn is_equal_node(&self, other: &Node) -> bool;
    // pub fn compare_document_position(&self, other: &Node) -> u16;
    // pub fn normalize(&mut self);  // 合并相邻 Text 节点
}

/// 格式化输出 DOM 树（调试用）
impl std::fmt::Display for Node;
```

### 2.4 Element — 元素节点

```rust
// === crates/dom/src/element.rs ===

/// HTML 元素节点数据 —— 对应 W3C Element 接口
///
/// 存放标签名、属性、样式、事件监听器等
pub struct ElementData {
    /// 标签名（小写）："div", "h1", "button"
    pub(crate) tag_name: String,
    /// 属性键值对：<div class="foo" id="bar">
    pub(crate) attributes: HashMap<String, String>,
    /// CSS 类名列表
    pub(crate) class_list: Vec<String>,
    /// 内联样式键值对："color" → "red"
    pub(crate) style: HashMap<String, String>,
    /// 事件监听器：事件类型 → 回调函数列表
    pub(crate) events: HashMap<String, Vec<EventListener>>,
    /// 元素 ID
    pub(crate) id: Option<String>,
}

/// 事件监听器结构体
/// Phase 1+: 支持 options (capture/once/passive)
pub struct EventListener {
    /// 回调函数
    pub callback: Box<dyn Fn(&Event)>,
    /// 监听器唯一标识（用于 removeEventListener）
    pub id: usize,
}

// 原子递增 ID 生成器
static NEXT_LISTENER_ID: AtomicUsize = AtomicUsize::new(1);

impl ElementData {
    /// 创建新元素数据
    pub fn new(tag_name: &str) -> Self;

    // ============================================================
    //  属性操作 —— W3C Element API
    // ============================================================

    /// 获取属性值：element.getAttribute("class")
    pub fn get_attribute(&self, name: &str) -> Option<String>;

    /// 设置属性：element.setAttribute("class", "foo")
    /// 会自动同步 classList（name="class" 时）和 style（Phase 1+）
    pub fn set_attribute(&mut self, name: &str, value: &str);

    /// 移除属性：element.removeAttribute("class")
    pub fn remove_attribute(&mut self, name: &str);

    /// 判断属性是否存在：element.hasAttribute("class")
    pub fn has_attribute(&self, name: &str) -> bool;

    // ============================================================
    //  classList —— W3C DOMTokenList
    // ============================================================

    /// 获取类名列表引用
    pub fn class_list(&self) -> &[String];

    /// 获取类名列表可变引用
    pub fn class_list_mut(&mut self) -> &mut Vec<String>;

    /// 添加类名（classList.add）
    pub fn add_class(&mut self, class: &str);

    /// 移除类名（classList.remove）
    pub fn remove_class(&mut self, class: &str);

    /// 切换类名（classList.toggle）
    pub fn toggle_class(&mut self, class: &str) -> bool;

    /// 是否包含类名（classList.contains）
    pub fn has_class(&self, class: &str) -> bool;

    // ============================================================
    //  style —— 内联样式
    // ============================================================

    /// 获取样式属性值：element.style.getPropertyValue("color")
    pub fn get_style_value(&self, property: &str) -> Option<&String>;

    /// 设置样式属性：element.style.setProperty("color", "red")
    pub fn set_style_value(&mut self, property: &str, value: &str);

    /// 移除样式属性：element.style.removeProperty("color")
    pub fn remove_style_value(&mut self, property: &str) -> Option<String>;

    /// 获取全部样式映射（只读）
    pub fn style_map(&self) -> &HashMap<String, String>;

    /// 设置样式字符串：element.setAttribute("style", "color: red; font-size: 16px")
    /// Phase 1+: 完整 CSS 值解析
    pub fn parse_and_set_style(&mut self, style_str: &str);

    // ============================================================
    //  事件管理 —— W3C EventTarget
    // ============================================================

    /// 添加事件监听器：element.addEventListener("click", callback)
    /// 返回监听器 ID（用于 removeEventListener）
    pub fn add_event_listener(
        &mut self,
        event_type: &str,
        callback: Box<dyn Fn(&Event)>,
    ) -> usize;

    /// 移除事件监听器：element.removeEventListener("click", id)
    pub fn remove_event_listener(&mut self, event_type: &str, id: usize);

    /// 派发事件：element.dispatchEvent(event)
    /// Phase 1+: 实现冒泡
    pub fn dispatch_event(&mut self, event: &Event) -> bool;

    /// 获取指定类型的事件监听器列表
    pub fn get_event_listeners(&self, event_type: &str) -> &[EventListener];

    // ============================================================
    //  固有属性
    // ============================================================

    /// 元素 ID
    pub fn id(&self) -> Option<&String>;

    /// 设置 ID
    pub fn set_id(&mut self, id: &str);

    /// class 属性字符串（多个类名空格分隔）
    pub fn class_name(&self) -> String;

    /// 从字符串设置 className（解析空格分隔的类名）
    pub fn set_class_name(&mut self, class: &str);

    /// 标签名称（小写）
    pub fn tag_name(&self) -> &str;

    // Phase 1+:
    // pub fn inner_html(&self) -> String;
    // pub fn set_inner_html(&mut self, html: &str);
    // pub fn get_bounding_client_rect(&self) -> Rect<f32>;
    // pub fn query_selector(&self, selector: &str) -> Option<Rc<RefCell<Node>>>;
    // pub fn closest(&self, selector: &str) -> Option<Rc<RefCell<Node>>>;
    // pub fn matches(&self, selector: &str) -> bool;
    // pub fn focus(&mut self);
    // pub fn blur(&mut self);
}
```

### 2.5 Document — 文档对象

```rust
// === crates/dom/src/document.rs ===

/// Document 对象 —— W3C document 接口
///
/// 提供节点创建和查询 API
/// Phase 0 只实现创建 API, 查询 API 在 Phase 1+
pub struct Document {
    /// 文档元素 (<html>)
    document_element: Rc<RefCell<Node>>,
    /// body 元素
    body: Rc<RefCell<Node>>,
    /// Phase 1+: 根据 ID 索引元素的 HashMap
    // element_id_map: RefCell<HashMap<String, Weak<RefCell<Node>>>>,
}

impl Document {
    /// 创建新文档（含默认的 html > head + body 结构）
    pub fn new() -> Rc<RefCell<Self>>;

    // ============================================================
    //  节点创建 —— W3C Document API
    // ============================================================

    /// 创建元素节点：document.createElement("div")
    pub fn create_element(&self, tag_name: &str) -> Rc<RefCell<Node>>;

    /// 创建文本节点：document.createTextNode("hello")
    pub fn create_text_node(&self, data: &str) -> Rc<RefCell<Node>>;

    /// 创建文档片段：document.createDocumentFragment()
    /// Phase 1+: 实现
    // pub fn create_document_fragment(&self) -> Rc<RefCell<Node>>;

    // ============================================================
    //  查询 —— W3C Document API
    // ============================================================

    /// Phase 1+: 通过 ID 查找元素
    // pub fn get_element_by_id(&self, id: &str) -> Option<Rc<RefCell<Node>>>;

    /// Phase 1+: CSS 选择器查询
    // pub fn query_selector(&self, selector: &str) -> Option<Rc<RefCell<Node>>>;

    // ============================================================
    //  文档属性
    // ============================================================

    /// 获取文档元素 (<html>)
    pub fn document_element(&self) -> Rc<RefCell<Node>>;

    /// 获取 body 元素
    pub fn body(&self) -> Rc<RefCell<Node>>;

    /// Phase 1+:
    // pub fn create_comment(&self, data: &str) -> Rc<RefCell<Node>>;
    // pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<Rc<RefCell<Node>>>;
    // pub fn get_elements_by_class_name(&self, class: &str) -> Vec<Rc<RefCell<Node>>>;
    // pub fn title(&self) -> String;
    // pub fn set_title(&self, title: &str);
}
```

### 2.6 Text — 文本节点

```rust
// === crates/dom/src/text.rs ===

/// Text 节点 —— 存储文本内容
pub struct Text {
    /// 文本数据
    data: String,
}

impl Text {
    /// 创建文本节点
    pub fn new(data: &str) -> Self;

    /// 获取文本内容（W3C: textNode.data / textNode.textContent）
    pub fn data(&self) -> &str;

    /// 设置文本内容
    pub fn set_data(&mut self, data: &str);

    /// 文本长度
    pub fn length(&self) -> usize;

    /// Phase 1+:
    // pub fn split_text(&mut self, offset: usize) -> Rc<RefCell<Node>>;
    // pub fn append_data(&mut self, data: &str);
    // pub fn delete_data(&mut self, offset: usize, count: usize);
    // pub fn insert_data(&mut self, offset: usize, data: &str);
    // pub fn replace_data(&mut self, offset: usize, count: usize, data: &str);
    // pub fn substring_data(&self, offset: usize, count: usize) -> String;
}
```

### 2.7 Event — 事件系统

```rust
// === crates/dom/src/event.rs ===

/// W3C Event 接口 —— 基础事件类型
///
/// Phase 0 实现最小子集：type + target + 冒泡控制
pub struct Event {
    /// 事件类型："click", "mousedown", "keydown" 等
    pub event_type: String,
    /// 原始触发目标元素
    pub target: Option<Rc<RefCell<Node>>>,
    /// 当前正在处理该事件的元素
    pub current_target: Option<Rc<RefCell<Node>>>,
    /// 是否冒泡
    pub bubbles: bool,
    /// 是否可取消默认行为
    pub cancelable: bool,
    /// 是否已调用 preventDefault
    default_prevented: Cell<bool>,
    /// 是否已调用 stopPropagation
    propagation_stopped: Cell<bool>,
    /// Phase 1+: 是否已调用 stopImmediatePropagation
    // immediate_propagation_stopped: Cell<bool>,
    /// 事件时间戳（毫秒）
    pub time_stamp: f64,
}

impl Event {
    /// 创建新事件
    /// bubbles: 默认 true（冒泡）
    /// cancelable: 默认 true（可取消）
    pub fn new(event_type: &str) -> Self;

    // ============================================================
    //  W3C Event 方法
    // ============================================================

    /// 阻止默认行为（如阻止链接跳转）
    pub fn prevent_default(&self);

    /// 是否已阻止默认行为
    pub fn default_prevented(&self) -> bool;

    /// 停止冒泡：事件不再向父元素传播
    pub fn stop_propagation(&self);

    /// 是否已停止冒泡
    pub fn propagation_stopped(&self) -> bool;

    /// Phase 1+:
    // pub fn stop_immediate_propagation(&self);
    // pub fn composed(&self) -> bool;
}

// ============================================================
//  MouseEvent —— 鼠标事件
// ============================================================

/// 鼠标事件 —— 继承 Event
pub struct MouseEvent {
    /// 基础事件
    pub event: Event,
    /// 鼠标在视口中的 X 坐标
    pub client_x: f64,
    /// 鼠标在视口中的 Y 坐标
    pub client_y: f64,
    /// 按下的鼠标键：0=左键 1=中键 2=右键
    pub button: i16,
    /// 是否按下 Alt 键
    pub alt_key: bool,
    /// 是否按下 Ctrl 键
    pub ctrl_key: bool,
    /// 是否按下 Shift 键
    pub shift_key: bool,
    /// 是否按下 Meta 键（Win/Cmd）
    pub meta_key: bool,
}

impl MouseEvent {
    /// 创建鼠标事件
    pub fn new(event_type: &str, x: f64, y: f64, button: i16) -> Self;

    // Phase 1+:
    // pub fn screen_x / screen_y(页面坐标);
    // pub fn movement_x / movement_y(相对移动);
    // pub fn related_target(相关元素);
}

// ============================================================
//  KeyboardEvent —— 键盘事件 (Phase 1+)
// ============================================================

// Phase 1+:
// pub struct KeyboardEvent {
//     pub event: Event,
//     pub key: String,         // 按键值: "Enter", "a", "1"
//     pub code: String,        // 物理键码: "KeyA", "Digit1"
//     pub alt_key: bool,
//     pub ctrl_key: bool,
//     pub shift_key: bool,
//     pub meta_key: bool,
//     pub repeat: bool,        // 是否长按重复触发
// }

// ============================================================
//  TouchEvent —— 触摸事件 (Phase 3)
// ============================================================

// Phase 3:
// pub struct TouchEvent { ... }
// pub struct Touch { ... }

// ============================================================
//  FocusEvent —— 焦点事件 (Phase 1+)
// ============================================================

// Phase 1+:
// pub struct FocusEvent { ... }

// ============================================================
//  WheelEvent —— 滚轮事件 (Phase 1+)
// ============================================================

// Phase 1+:
// pub struct WheelEvent {
//     pub event: Event,
//     pub delta_x: f64,
//     pub delta_y: f64,
//     pub delta_z: f64,
//     pub delta_mode: WheelDeltaMode,
// }
```

### 2.8 DOMTokenList — classList 实现

```rust
// === crates/dom/src/dom_token_list.rs ===

/// W3C DOMTokenList —— classList 操作接口
///
/// 内部维护一个有序去重的类名列表
/// 修改时自动同步到父元素的 attributes
pub struct DOMTokenList {
    tokens: Vec<String>,
}

impl DOMTokenList {
    /// 从空格分隔的字符串创建
    pub fn from_string(class_str: &str) -> Self;

    /// Phase 1+: 返回类名个数
    // pub fn length(&self) -> usize;

    /// Phase 1+: 通过索引获取类名
    // pub fn item(&self, index: usize) -> Option<&str>;

    /// 是否包含指定类名
    pub fn contains(&self, token: &str) -> bool;

    /// 添加类名（已存在不重复添加）
    pub fn add(&mut self, token: &str);

    /// 移除类名
    pub fn remove(&mut self, token: &str);

    /// 切换类名：存在则移除，不存在则添加
    /// force=true: 强制添加; force=false: 强制移除
    /// 返回切换后的状态
    pub fn toggle(&mut self, token: &str) -> bool;

    /// Phase 1+:
    // pub fn replace(&mut self, old_token: &str, new_token: &str);
    // pub fn supports(&self, token: &str) -> bool;

    /// 转为空格分隔的字符串
    pub fn to_string(&self) -> String;
}
```

### 2.9 dom 公开导出

```rust
// === crates/dom/src/lib.rs ===

//! # DOM crate — W3C DOM 标准实现
//!
//! 符合 W3C DOM Living Standard 规范的 Rust 实现。
//! Phase 0 实现核心节点类型 + 树操作 + 基础事件。
//!
//! 使用方式：
//! ```rust
//! use dom::*;
//! let doc = Document::new();
//! let div = doc.create_element("div");
//! let text = doc.create_text_node("hello");
//! div.append_child(text);
//! ```

// Phase 0 公开类型
pub use node::Node;
pub use node::NodeType;
pub use node::node_type_constants;
pub use element::ElementData;
pub use document::Document;
pub use text::Text;
pub use event::Event;
pub use event::MouseEvent;
pub use dom_token_list::DOMTokenList;

// Phase 1+ 将增加：
// pub use event::KeyboardEvent;
// pub use event::FocusEvent;
// pub use event::WheelEvent;

// ============================================================
//  辅助类型
// ============================================================

/// 颜色类型（RGBA）
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// 矩形类型
#[derive(Debug, Clone, Copy)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

/// 尺寸类型
#[derive(Debug, Clone, Copy)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

// Phase 1+:
// pub struct Point<T> { pub x: T, pub y: T }
// pub struct EdgeInsets { pub top, right, bottom, left: f32 }
```

---

## 三、css crate — CSS 引擎

### 3.1 模块结构

```
css/
├── Cargo.toml
└── src/
    ├── lib.rs              # 模块导出
    ├── stylesheet.rs       # 样式表解析
    ├── selector.rs         # 选择器匹配
    ├── cascade.rs          # 级联计算
    ├── values.rs           # CSS 值类型
    └── properties.toml     # CSS 属性元数据（编译期代码生成）
```

### 3.2 StyleSheet — 样式表

```rust
// === crates/css/src/stylesheet.rs ===

/// 解析后的 CSS 样式表
///
/// 内部使用 cssparser crate 解析 CSS 文本
pub struct StyleSheet {
    /// 样式规则列表
    pub rules: Vec<Rule>,
}

/// 单条 CSS 规则：选择器 + 声明块
pub struct Rule {
    /// 选择器（如 "div.container"）
    pub selectors: Vec<Selector>,
    /// 声明列表（如 color: red; font-size: 16px）
    pub declarations: Vec<Declaration>,
}

/// 单条 CSS 声明：属性 + 值
pub struct Declaration {
    /// 属性名："color", "font-size"
    pub property: String,
    /// 属性值："red", "16px"
    pub value: String,
    /// 是否标记了 !important
    pub important: bool,
}

/// 选择器（通过 selectors crate 解析）
/// Phase 0: 只支持简单选择器
pub type Selector = String;
// Phase 1+: 改为 selectors::Selector<SimpleSelector>

/// 解析 CSS 文本为样式表
///
/// ```rust
/// let ss = parse_stylesheet("div { color: red; }");
/// ```
pub fn parse_stylesheet(css_text: &str) -> StyleSheet;

/// 解析 style 属性值字符串为声明列表
///
/// ```rust
/// let decls = parse_inline_style("color: red; font-size: 16px");
/// ```
pub fn parse_inline_style(style_str: &str) -> Vec<Declaration>;

// Phase 1+:
// pub fn parse_media_query(query: &str) -> MediaQuery;
// pub fn parse_keyframes(keyframes_text: &str) -> Vec<Keyframe>;
// pub fn parse_font_face(font_face_text: &str) -> FontFace;
```

### 3.3 Selector — 选择器匹配

```rust
// === crates/css/src/selector.rs ===

/// 选择器匹配结果：特异性 + 声明
pub struct MatchedDeclaration {
    /// 特异性值 (ID, Class, Tag 三元组)
    pub specificity: (u32, u32, u32),
    /// 匹配的声明
    pub declaration: Declaration,
}

/// 针对单个元素的匹配：找出所有匹配的规则声明
///
/// element: 目标元素
/// stylesheets: 全局样式表列表
/// 返回所有匹配的声明（含特异性权重）
pub fn match_selectors(
    element: &ElementData,
    stylesheets: &[StyleSheet],
) -> Vec<MatchedDeclaration>;

/// 判断元素是否匹配选择器字符串
///
/// Phase 0: 支持 tag, .class, #id
/// Phase 1+: 支持 组合器 + 伪类
pub fn element_matches_selector(element: &ElementData, selector: &str) -> bool;

/// 计算选择器特异性 (ID, Class, Tag)
///
/// 规则：ID 选择器 > Class/属性/伪类 > Tag/伪元素
fn compute_specificity(selector: &str) -> (u32, u32, u32);

// Phase 1+: selectors crate 完整集成
// pub struct SelectorEngine { ... }
// Phase 2+: 伪类支持 (:hover, :nth-child)
// pub enum PseudoClass { Hover, NthChild(usize), ... }

// Phase 1+ 预留：
// pub fn match_selectors_with_pseudo(
//     element: &ElementData,
//     pseudo: Option<PseudoClass>,
//     stylesheets: &[StyleSheet],
// ) -> Vec<MatchedDeclaration>;
```

### 3.4 Cascade — 级联计算

```rust
// === crates/css/src/cascade.rs ===

use crate::stylesheet::MatchedDeclaration;
use crate::values::CSSValue;

/// 计算元素的最终样式
///
/// 参数：
///   - element: 目标元素
///   - parent_style: 父元素计算后样式（用于继承属性）
///   - stylesheets: 所有样式表
///   - inline_style: 内联样式声明（style 属性值）
///
/// 返回：属性名 → CSSValue 的映射
pub fn compute_element_style(
    element: &ElementData,
    parent_style: Option<&ComputedStyle>,
    stylesheets: &[StyleSheet],
    inline_style: &[Declaration],
) -> ComputedStyle;

/// 级联排序：特异性 + 来源 + 顺序
///
/// 排序规则（从低到高）：
/// 1. 用户代理样式（浏览器默认）
/// 2. 用户样式（Phase 2+）
/// 3. 作者样式（开发者定义的 CSS）
/// 4. 内联样式（style 属性）
/// 5. !important 规则
fn cascade_sort(declarations: &mut [MatchedDeclaration]);

/// 应用继承属性：从父元素继承
fn apply_inherited(parent: &ComputedStyle, child: &mut ComputedStyle);

/// 计算后的样式集合
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    /// 属性映射
    pub properties: HashMap<String, CSSValue>,
}

impl ComputedStyle {
    /// 创建空的 ComputedStyle（全部使用初始值）
    pub fn empty() -> Self;

    /// 获取属性值，不存在的属性返回 None
    pub fn get(&self, name: &str) -> Option<&CSSValue>;

    /// 获取属性值，不存在的属性返回 CSSValue::Keyword("initial")
    pub fn get_or_initial(&self, name: &str) -> CSSValue;
}
```

### 3.5 CSSValue — CSS 值类型

```rust
// === crates/css/src/values.rs ===

/// CSS 属性值 —— 计算后的值类型
#[derive(Debug, Clone, PartialEq)]
pub enum CSSValue {
    /// 长度值：16px, 1.5em
    Length(f32, CSSUnit),
    /// 百分比：50%
    Percentage(f32),
    /// 颜色：#ff0000, rgba(255,0,0,0.5)
    Color(Color),
    /// 关键字：auto, none, block, flex
    Keyword(String),
    /// 数值：opacity: 0.5
    Number(f32),
    /// 字符串：content: "hello"
    String(String),
    /// 长度+百分比组合：calc(100% - 20px) (Phase 1+)
    // Calc(Box<CalcValue>),
    /// 初始值（未设置）
    Initial,
}

/// CSS 长度单位
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CSSUnit {
    /// 像素 px
    Px,
    /// 相对父元素字体大小 em
    Em,
    /// 相对根元素字体大小 rem
    Rem,
    /// 视口宽度百分比 vw (Phase 1+)
    // Vw,
    /// 视口高度百分比 vh (Phase 1+)
    // Vh,
    /// 百分比 %
    Percent,
}

/// 将 CSS 值字符串解析为 CSSValue（Phase 0 基础版本）
pub fn parse_css_value(property: &str, value: &str) -> CSSValue;

/// 解析颜色值：#ff0000, red, rgb(255,0,0)
pub fn parse_color(value: &str) -> Option<Color>;

/// 解析长度值：16px, 1.5em, 0
pub fn parse_length(value: &str) -> Option<(f32, CSSUnit)>;

/// Phase 1+:
// pub fn parse_transform(value: &str) -> Vec<Transform>;
// pub fn parse_animation(value: &str) -> Animation;
// pub fn parse_gradient(value: &str) -> Gradient;
```

### 3.6 properties.toml — CSS 属性定义

```toml
# === crates/css/properties.toml ===
#
# Phase 0 CSS 属性定义（30 个核心属性）
# 通过 build.rs 代码生成 Rust 初始化代码

# ===== 盒模型 =====

[width]
initial = "auto"
inherited = false

[height]
initial = "auto"
inherited = false

[margin-top]
initial = "0px"
inherited = false

[margin-right]
initial = "0px"
inherited = false

[margin-bottom]
initial = "0px"
inherited = false

[margin-left]
initial = "0px"
inherited = false

[padding-top]
initial = "0px"
inherited = false

[padding-right]
initial = "0px"
inherited = false

[padding-bottom]
initial = "0px"
inherited = false

[padding-left]
initial = "0px"
inherited = false

[display]
initial = "inline"
inherited = false

[box-sizing]
initial = "content-box"
inherited = false

[overflow]
initial = "visible"
inherited = false

# ===== 弹性布局 =====

[flex-direction]
initial = "row"
inherited = false

[flex-wrap]
initial = "nowrap"
inherited = false

[justify-content]
initial = "flex-start"
inherited = false

[align-items]
initial = "stretch"
inherited = false

[align-content]
initial = "stretch"
inherited = false

[gap]
initial = "0px"
inherited = false

[flex-grow]
initial = "0"
inherited = false

[flex-shrink]
initial = "1"
inherited = false

[flex-basis]
initial = "auto"
inherited = false

# ===== 定位 =====

[position]
initial = "static"
inherited = false

[top]
initial = "auto"
inherited = false

[right]
initial = "auto"
inherited = false

[bottom]
initial = "auto"
inherited = false

[left]
initial = "auto"
inherited = false

# ===== 文字 =====

[color]
initial = "#000000"
inherited = true

[font-size]
initial = "16px"
inherited = true

[font-weight]
initial = "400"
inherited = true

[font-family]
initial = "sans-serif"
inherited = true

[line-height]
initial = "normal"
inherited = true

[text-align]
initial = "start"
inherited = true

[white-space]
initial = "normal"
inherited = true

# ===== 背景 =====

[background-color]
initial = "transparent"
inherited = false

# ===== 其他 =====

[opacity]
initial = "1"
inherited = false

[border]
initial = "none"
inherited = false

# ============================================================
# Phase 1+ 属性预留
# ============================================================

# Phase 1:
# [min-width] [min-height] [max-width] [max-height]
# [border-width] [border-style] [border-color]
# [border-radius] [visibility]
# [font-style] [text-decoration] [word-break]
# [background-image] [background-size] [background-position]
# [z-index] [cursor]
# [flex] [order] [align-self]
# [list-style] [table-layout] [border-collapse]

# Phase 2:
# [transform] [transition] [animation]
# [box-shadow] [text-shadow] [filter]
# [grid] [grid-template] [grid-column] [grid-row]
# [float] [clear]
# [media queries]
# [clip-path] [mask]

# Phase 3:
# [scroll-behavior] [scrollbar-width]
# [mix-blend-mode] [backdrop-filter]
# [writing-mode] [direction]
# [caret-color] [accent-color]
# [contain] [content-visibility]
# [container] [container-type]
```

### 3.7 css 公开导出

```rust
// === crates/css/src/lib.rs ===

//! # CSS crate — W3C CSS 引擎
//!
//! 提供 CSS 解析、选择器匹配、级联计算的完整功能。
//! Phase 0 支持 ~30 个核心属性。

pub use stylesheet::{parse_stylesheet, parse_inline_style, StyleSheet, Rule, Declaration};
pub use selector::{match_selectors, element_matches_selector, MatchedDeclaration};
pub use cascade::{compute_element_style, ComputedStyle};
pub use values::{CSSValue, CSSUnit, parse_css_value, parse_color, parse_length};

// Phase 1+:
// pub use stylesheet::parse_media_query;
// pub use selector::SelectorEngine;
// pub use values::parse_transform;
```

---

## 四、layout crate — 布局引擎

### 4.1 模块结构

```
layout/
├── Cargo.toml
└── src/
    ├── lib.rs              # 模块导出 + 公开 API
    ├── layout_box.rs       # LayoutBox 类型
    ├── flex.rs             # Flexbox 布局（taffy 集成）
    ├── block.rs            # Block 布局
    ├── positioned.rs       # 定位布局
    └── text.rs             # 文本测量
```

### 4.2 LayoutBox — 布局节点

```rust
// === crates/layout/src/layout_box.rs ===

use dom::{Node, Rect, Size};

/// 布局框类型 —— 参与布局计算的基本单元
#[derive(Debug, Clone)]
pub enum BoxType {
    /// 块级框(block)
    Block,
    /// 行内框(inline)
    Inline,
    /// Flex 容器 (display: flex)
    FlexContainer,
    /// Flex 子项 (Flex 容器的直接子元素)
    FlexItem,
    /// 文本行框
    Text,
    /// 匿名框（包裹行内元素的不可见框）
    Anonymous,
    // Phase 1+:
    // InlineBlock,
    // Table, TableRow, TableCell, TableCaption,
    // GridContainer, GridItem,
    // Absolute, Fixed, Sticky,
    // Float,
}

/// 布局树节点 —— 每个 DOM 节点对应一个或多个 LayoutBox
///
/// 布局引擎的输入：DOM 树 + ComputedStyle
/// 布局引擎的输出：每个 LayoutBox 的 rect 被填充
#[derive(Debug)]
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
}

/// 四边尺寸
#[derive(Debug, Clone, Copy, Default)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl LayoutBox {
    /// 创建新布局框
    pub fn new(box_type: BoxType, node: Option<Rc<RefCell<Node>>>) -> Self;

    /// 添加子布局框
    pub fn append_child(&mut self, child: LayoutBox);

    /// 获取内容区域尺寸（去除 padding + border）
    pub fn content_area(&self) -> Size<f32>;

    /// 设置内容区域尺寸（加上 padding + border 得到总尺寸）
    pub fn set_content_area(&mut self, size: Size<f32>);

    /// 遍历所有后代（深度优先）
    pub fn traverse<F: FnMut(&LayoutBox)>(&self, f: &mut F);

    /// 遍历所有后代（可变引用，深度优先）
    pub fn traverse_mut<F: FnMut(&mut LayoutBox)>(&mut self, f: &mut F);

    /// 收集所有匹配条件的布局框
    pub fn find<F: Fn(&LayoutBox) -> bool>(&self, f: &F) -> Vec<&LayoutBox>;
}
```

### 4.3 LayoutEngine — 布局引擎主入口

```rust
// === crates/layout/src/lib.rs ===

/// 布局引擎 —— 负责计算每个节点的位置和尺寸
///
/// 使用流程：
/// 1. 构建布局树：build_layout_tree(&dom_root, &styles)
/// 2. 执行布局：LayoutEngine::new().layout(&mut layout_root, viewport)
/// 3. 读取结果：每个 LayoutBox.rect 为最终位置
pub struct LayoutEngine {
    /// taffy 布局实例（用于 Flexbox）
    taffy: Option<taffy::Taffy>,
    /// Phase 1+: 布局缓存
    // cache: LayoutCache,
}

impl LayoutEngine {
    /// 创建新布局引擎
    pub fn new() -> Self;

    /// 主入口：执行完整布局计算
    ///
    /// 参数：
    ///   - root: 布局树根节点
    ///   - viewport: 视口尺寸（通常为窗口尺寸）
    ///
    /// 输出：root 及其所有后代的 rect 被填充
    pub fn layout(&mut self, root: &mut LayoutBox, viewport: Size<f32>);

    /// Phase 1+：局部重排（只重排脏子树）
    // pub fn partial_layout(&mut self, root: &mut LayoutBox, dirty: &[NodePath]);
}

/// 从 DOM 树 + 样式映射构建布局树
///
/// computed_styles: 从 css::cascade::compute_element_style 获取
pub fn build_layout_tree(
    dom_root: &Rc<RefCell<Node>>,
    computed_styles: &HashMap<usize, ComputedStyle>,  // key = Rc pointer addr
) -> LayoutBox;

/// Phase 1+: 更新布局树（DOM 变更后增量更新）
// pub fn update_layout_tree(
//     layout: &mut LayoutBox,
//     dom_changes: &[DomMutation],
//     computed_styles: &HashMap<usize, ComputedStyle>,
// ) -> bool;  // 返回是否有尺寸变化

/// Phase 1+: 通过节点地址在布局树中查找
// pub fn find_layout_node<'a>(root: &'a LayoutBox, node_ptr: usize) -> Option<&'a LayoutBox>;

// ============================================================
//  辅助类型
// ============================================================

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
    /// Phase 2+: 浮动
    // Float,
    /// Phase 1+: 表格
    // Table,
}
```

### 4.4 Flex — Flexbox 布局

```rust
// === crates/layout/src/flex.rs ===

use crate::layout_box::LayoutBox;
use dom::Size;

/// Flexbox 布局 —— 通过 taffy crate 实现
///
/// 支持的 flex 属性：
///   flex-direction, flex-wrap, justify-content,
///   align-items, align-content, gap,
///   flex-grow, flex-shrink, flex-basis
pub struct FlexLayout;

impl FlexLayout {
    /// 对 Flex 容器执行布局
    ///
    /// container: 必须是 FlexContainer 类型
    ///          其子节点会被视为 FlexItem
    /// viewport: 当前视口尺寸（用于 max-width 约束）
    pub fn layout(
        &mut self,
        taffy: &mut taffy::Taffy,
        container: &mut LayoutBox,
        viewport: Size<f32>,
    );

    /// 将 LayoutBox 的属性转换为 taffy 的 Style
    fn convert_style(box_node: &LayoutBox) -> taffy::Style;

    /// 将 taffy 的计算结果写回 LayoutBox.rect
    fn apply_result(container: &mut LayoutBox, taffy: &taffy::Taffy, node_id: taffy::NodeId);

    /// Phase 1+: 处理 min-width / max-width 约束
    // fn apply_min_max_constraints(...)
}
```

### 4.5 Block — Block 布局

```rust
// === crates/layout/src/block.rs ===

use crate::layout_box::LayoutBox;
use dom::Size;

/// Block 布局 —— 从顶到底逐行排列
///
/// 实现 W3C CSS 2.1 Block 布局规范
/// Phase 0 实现基础版本，Phase 1+ 实现 margin collapse
pub struct BlockLayout;

impl BlockLayout {
    /// 对 Block 容器执行布局
    ///
    /// container.children 按顺序从上到下排列
    /// 每个子节点的宽度 = container.content_area.width
    pub fn layout(
        &self,
        container: &mut LayoutBox,
        viewport: Size<f32>,
    );

    /// 计算块级子节点的纵坐标
    fn compute_y_position(
        children: &[LayoutBox],
        start_y: f32,
    ) -> Vec<f32>;

    // Phase 1+: margin collapse（相邻块级元素 margin 合并）
    // fn compute_collapsed_margin(
    //     top: &LayoutBox,
    //     bottom: &LayoutBox,
    // ) -> f32;
}
```

### 4.6 Positioned — 定位布局

```rust
// === crates/layout/src/positioned.rs ===

use crate::layout_box::LayoutBox;
use dom::Size;

/// 定位布局 —— position: absolute / fixed / relative / sticky
///
/// Phase 0 实现：
///   - position: absolute（相对已定位祖先）
///   - position: relative（相对自身偏移）
///
/// Phase 1+:
///   - position: fixed（相对视口）
///   - position: sticky（粘性定位）
pub struct PositionedLayout;

impl PositionedLayout {
    /// 执行定位布局
    ///
    /// 需要在 main layout 之后执行，因为定位依赖包含块的尺寸
    pub fn layout(
        &self,
        root: &mut LayoutBox,
        viewport: Size<f32>,
    );

    /// 查找最近的已定位祖先
    /// position: relative/absolute/fixed/sticky → 已定位
    fn find_positioned_ancestor<'a>(
        node: &LayoutBox,
        root: &'a LayoutBox,
    ) -> Option<&'a LayoutBox>;

    // Phase 1+:
    // /// 处理 position: fixed（相对于视口）
    // fn layout_fixed(&self, root: &mut LayoutBox, viewport: Size<f32>);
}

// Phase 1+: Sticky 定位
// pub struct StickyLayout { ... }
```

### 4.7 Text — 文本测量

```rust
// === crates/layout/src/text.rs ===

use dom::Size;

/// 文本测量工具 —— 计算文本在屏幕上的实际宽高
///
/// 基于 rustybuzz (HarfBuzz) + fontdb
/// Phase 0: 英文 + 中文（不处理复杂文本布局）
pub struct TextMeasurer {
    /// fontdb 字体数据库
    font_db: fontdb::Database,
    /// 字体缓存（已加载的字体数据）
    font_cache: HashMap<String, rustybuzz::Face>,
}

impl TextMeasurer {
    /// 创建文本测量器（初始化 fontdb）
    pub fn new() -> Self;

    /// 测量文本在指定字体下的尺寸
    ///
    /// text: 要测量的文本
    /// font_size: 字号（像素）
    /// font_family: 字体系列名（逗号分隔的回退列表）
    /// weight: 字重（400=normal, 700=bold）
    ///
    /// 返回文本的像素宽高
    pub fn measure(
        &mut self,
        text: &str,
        font_size: f32,
        font_family: &str,
        weight: u16,
    ) -> Size<f32>;

    /// 从 fontdb 加载字体文件
    fn load_font(&mut self, family: &str, weight: u16) -> Option<&rustybuzz::Face>;

    /// 使用 rustybuzz 执行字形 shaping
    fn shape_text(
        face: &rustybuzz::Face,
        text: &str,
        font_size: f32,
    ) -> Vec<GlyphInfo>;

    // Phase 1+:
    // /// 断字换行（处理长单词）
    // pub fn break_text(&self, text: &str, max_width: f32) -> Vec<&str>;
    // /// 字体回退链（中英文混排）
    // fn font_fallback(&mut self, text: &str, families: &[&str]) -> Vec<FontSpan>;
    // /// 行高计算
    // pub fn line_height(font_size: f32, line_height: CSSValue) -> f32;
}

/// 字形信息 —— 文本 shaping 结果
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// 字形 ID
    pub glyph_id: u16,
    /// 水平偏移
    pub x_offset: f32,
    /// 垂直偏移
    pub y_offset: f32,
    /// 水平步进（字形左边缘到下个字形左边缘）
    pub x_advance: f32,
    /// 垂直步进
    pub y_advance: f32,
    /// 字形宽度
    pub width: f32,
    /// 字形高度
    pub height: f32,
}
```

### 4.8 layout 公开导出

```rust
// === crates/layout/src/lib.rs ===

//! # layout crate — 布局引擎
//!
//! 负责将 DOM 树 + CSS 计算样式转换为每个节点的位置和尺寸。
//! 输出 LayoutTree（每个节点有精确的 Rect）。
//!
//! 独立模块设计，不依赖 render/runtime。

pub use layout_box::{LayoutBox, BoxType, EdgeSizes};
pub use flex::FlexLayout;
pub use block::BlockLayout;
pub use positioned::PositionedLayout;
pub use text::TextMeasurer;

// Phase 1+:
// pub use inline::InlineLayout;
// pub use layout_box::LayoutMode;
// pub use text::GlyphInfo;

// Phase 2+:
// pub use table::TableLayout;
// pub use float::FloatLayout;
```

---

## 五、paint crate — DisplayList 绘制命令

### 5.1 模块结构

```
paint/
├── Cargo.toml
└── src/
    ├── lib.rs              # 模块导出
    ├── command.rs           # PaintCommand 类型
    └── builder.rs           # LayoutTree → DisplayList
```

### 5.2 PaintCommand — 绘制命令

```rust
// === crates/paint/src/command.rs ===

use dom::Color;
use dom::Rect;

/// 像素坐标 —— 整数值避免 sub-pixel 渲染不一致
pub type Pixel = f32;

/// 绘制命令 —— 渲染引擎处理的原子操作
///
/// 由 paint::builder 从 LayoutTree 生成
/// 被 render crate 消费并转换为 GPU 绘制调用
#[derive(Debug, Clone)]
pub enum PaintCommand {
    /// 填充矩形（背景色填充）
    FillRect {
        rect: Rect<Pixel>,
        color: Color,
        /// Phase 1+: 圆角
        // border_radius: [f32; 4],
    },
    /// 绘制文本
    Text {
        text: String,
        font_size: f32,
        font_family: String,
        font_weight: u16,
        position: (Pixel, Pixel),
        color: Color,
        /// Phase 1+: 行内格式
        // text_decoration: TextDecoration,
    },
    /// 绘制边框
    Border {
        rect: Rect<Pixel>,
        /// 各边宽度
        widths: [f32; 4], // top, right, bottom, left
        /// 各边颜色
        colors: [Color; 4],
        /// 圆角半径
        radius: f32,
        /// Phase 1+: 边框样式（solid/dashed/dotted）
        // style: BorderStyle,
    },
    // Phase 1+:
    // /// 绘制阴影
    // BoxShadow {
    //     rect: Rect<Pixel>,
    //     offset: (Pixel, Pixel),
    //     blur: f32,
    //     color: Color,
    //     spread: f32,
    // },
    // /// 绘制图像
    // Image {
    //     rect: Rect<Pixel>,
    //     image_handle: ImageHandle,
    //     repeat: RepeatMode,
    // },
    // /// 裁剪区域
    // Clip {
    //     rect: Rect<Pixel>,
    //     children: Vec<PaintCommand>,
    // },
    // /// 透明度组
    // Opacity {
    //     opacity: f32,
    //     children: Vec<PaintCommand>,
    // },
}
```

### 5.3 DisplayList — 绘制命令集合

```rust
// === crates/paint/src/command.rs ===

/// 绘制命令列表 —— 按渲染顺序排列
#[derive(Debug)]
pub struct DisplayList {
    /// 绘制命令（按 z-order 排序后）
    commands: Vec<PaintCommand>,
}

impl DisplayList {
    /// 创建空 DisplayList
    pub fn new() -> Self;

    /// 添加绘制命令
    pub fn push(&mut self, cmd: PaintCommand);

    /// 按 z-order 排序
    ///
    /// 排序规则：
    /// 1. 先绘制背景（z-index 低）
    /// 2. 再绘制内容（正常流）
    /// 3. 最后绘制前景/浮动（z-index 高）
    pub fn sort_by_z_order(&mut self);

    /// 获取命令列表
    pub fn commands(&self) -> &[PaintCommand];

    /// 清空 DisplayList
    pub fn clear(&mut self);

    /// 命令数量
    pub fn len(&self) -> usize;

    /// 是否为空
    pub fn is_empty(&self) -> bool;
}
```

### 5.4 Builder — LayoutTree → DisplayList

```rust
// === crates/paint/src/builder.rs ===

use crate::command::{DisplayList, PaintCommand};
use layout::LayoutBox;
use dom::Color;

/// Paint 命令构建器 —— 将布局树转换为绘制命令
pub struct DisplayListBuilder {
    /// 输出列表
    display_list: DisplayList,
}

impl DisplayListBuilder {
    /// 创建构建器
    pub fn new() -> Self;

    /// 主入口：从布局树构建 DisplayList
    ///
    /// 遍历 LayoutTree，为每个节点生成对应的 PaintCommand：
    /// 1. background-color → FillRect
    /// 2. border → Border
    /// 3. text content → Text
    ///
    /// 返回构建完成的 DisplayList
    pub fn build(&mut self, layout_root: &LayoutBox) -> DisplayList;

    /// 处理单个布局节点
    fn process_node(&mut self, node: &LayoutBox);

    /// 递归处理子树
    fn process_children(&mut self, node: &LayoutBox);

    /// 从 computed style 中提取颜色
    fn extract_bg_color(node: &LayoutBox) -> Option<Color>;

    /// 从 computed style 中提取 border 属性
    fn extract_border(node: &LayoutBox) -> Option<(...)>;

    /// 从 DOM 节点提取文本内容
    fn extract_text(node: &LayoutBox) -> Option<String>;

    // Phase 1+:
    // fn process_box_shadow(node: &LayoutBox) -> Option<PaintCommand>;
    // fn process_image(node: &LayoutBox) -> Option<PaintCommand>;
    // fn process_clip(node: &LayoutBox) -> Option<PaintCommand>;
}

/// Phase 1+: 合批优化
// pub struct BatchOptimizer {
//     /// 按纹理分组后的绘制命令
//     batches: Vec<Vec<PaintCommand>>,
// }

// impl BatchOptimizer {
//     pub fn optimize(display_list: &DisplayList) -> Self;
//     pub fn batches(&self) -> &[Vec<PaintCommand>];
// }
```

### 5.5 paint 公开导出

```rust
// === crates/paint/src/lib.rs ===

//! # paint crate — DisplayList 绘制命令
//!
//! 定义渲染引擎消费的绘制命令类型。
//! 负责将 LayoutTree 转换为 PaintCommand 列表。
//!
//! 独立模块，不依赖具体渲染后端（wgpu/webgpu）。

pub use command::{PaintCommand, DisplayList};
pub use builder::DisplayListBuilder;

// Phase 1+:
// pub use command::BorderStyle;
// pub use builder::BatchOptimizer;
```

---

## 六、render crate — wgpu 渲染后端

### 6.1 模块结构

```
render/
├── Cargo.toml
└── src/
    ├── lib.rs              # 模块导出 + RenderBackend trait
    ├── wgpu_backend.rs     # wgpu 渲染后端
    └── text_renderer.rs    # 文本纹理渲染
```

### 6.2 RenderBackend — 渲染后端接口

```rust
// === crates/render/src/lib.rs ===

//! # render crate — 渲染后端
//!
//! 定义 RenderBackend trait，支持多种渲染后端实现。
//! Phase 0: wgpu 后端（原生桌面）
//! Phase 3: WebGPU WASM 后端（浏览器/小程序）

use paint::DisplayList;

/// 渲染后端接口 —— 可插拔
///
/// 所有渲染后端都实现这个 trait
pub trait RenderBackend {
    /// 初始化渲染后端
    /// surface: 平台特定的窗口表面
    fn initialize(surface: impl Into<RawWindowHandle>) -> Self;

    /// 渲染一帧
    /// display_list: 待渲染的绘制命令列表
    fn render(&mut self, display_list: &DisplayList);

    /// 窗口尺寸变更通知
    fn resize(&mut self, width: u32, height: u32);

    /// 交换缓冲区（显示帧）
    fn present(&mut self);

    /// 获取当前渲染尺寸
    fn size(&self) -> (u32, u32);
}

/// 原始窗口句柄（平台无关）
/// 从 winit 窗口获取
pub enum RawWindowHandle {
    Windows(isize),     // HWND
    Mac(isize),         // NSView
    Xlib(isize),        // X11 Window
    Wayland(isize),     // Wayland Surface
    // Phase 3:
    // Web(web_sys::Canvas),  // WASM Canvas
}

// Phase 3: WebGPU WASM 后端
// pub mod webgpu;
// pub struct WebGpuBackend;
// impl RenderBackend for WebGpuBackend { ... }
```

### 6.3 WgpuBackend — wgpu 渲染实现

```rust
// === crates/render/src/wgpu_backend.rs ===

use crate::RenderBackend;
use paint::{DisplayList, PaintCommand};
use dom::Color;

/// wgpu 渲染后端 —— 支持 DX12/Vulkan/Metal
///
/// wgpu 是 Rust 原生跨平台图形 API 抽象层
pub struct WgpuBackend {
    /// wgpu 表面（来自 winit 窗口）
    surface: wgpu::Surface,
    /// wgpu 设备（GPU 逻辑设备）
    device: wgpu::Device,
    /// wgpu 命令队列
    queue: wgpu::Queue,
    /// 交换链配置
    config: wgpu::SurfaceConfiguration,
    /// 当前渲染尺寸
    size: (u32, u32),
    // ===== 渲染管线状态 =====
    /// 矩形渲染管线（填充色）
    rect_pipeline: wgpu::RenderPipeline,
    /// 文本渲染器
    text_renderer: TextRenderer,
    // Phase 1+:
    // border_pipeline: wgpu::RenderPipeline,
    // image_pipeline: wgpu::RenderPipeline,
}

impl WgpuBackend {
    /// 创建 wgpu 后端
    ///
    /// window: winit 窗口引用
    pub async fn new(window: &winit::window::Window) -> Self;

    /// 初始化 wgpu 设备和渲染管线
    async fn init_device(window: &winit::window::Window) -> (wgpu::Device, wgpu::Queue, wgpu::Surface, wgpu::SurfaceConfiguration);

    /// 创建矩形填充渲染管线
    fn create_rect_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline;

    /// 编码矩形绘制命令
    fn encode_rect(&self, pass: &mut wgpu::RenderPass, rect: &Rect<f32>, color: &Color);

    /// 编码文本绘制命令
    fn encode_text(&mut self, pass: &mut wgpu::RenderPass, cmd: &PaintCommand);

    // Phase 1+: 编码边框绘制命令
    // fn encode_border(&self, pass: &mut wgpu::RenderPass, cmd: &PaintCommand);
}

impl RenderBackend for WgpuBackend {
    fn initialize(surface: impl Into<RawWindowHandle>) -> Self { /* 见上 new */ }
    fn render(&mut self, display_list: &DisplayList) { /* 遍历命令并绘制 */ }
    fn resize(&mut self, width: u32, height: u32) { /* 重建 swapchain */ }
    fn present(&mut self) { /* wgpu::Surface::present */ }
    fn size(&self) -> (u32, u32) { self.size }
}
```

### 6.4 TextRenderer — 文本渲染

```rust
// === crates/render/src/text_renderer.rs ===

/// 文本渲染器 —— 将文本转为 GPU 纹理并绘制
///
/// 工作原理：
/// 1. rustybuzz 对文本做 shaping → 获取字形 ID 和位置
/// 2. 从字体文件中提取字形轮廓（或位图）
/// 3. 将字形渲染到纹理缓存
/// 4. 使用 wgpu 纹理采样绘制
pub struct TextRenderer {
    /// 字形纹理缓存：font_size + glyph_id → 纹理坐标
    glyph_cache: HashMap<(u16, u16), GlyphTexture>,
    /// 字体数据缓存：font_family + weight → Face
    font_cache: HashMap<String, rustybuzz::Face>,
    /// fontdb 字体数据库
    font_db: fontdb::Database,
    /// wgpu 纹理资源
    atlass_texture: wgpu::Texture,
    /// 当前图集布局
    atlass_position: (u16, u16),
    /// Phase 1+: SDF 渲染管线
    // sdf_pipeline: Option<wgpu::RenderPipeline>,
}

/// 字形纹理信息
struct GlyphTexture {
    /// 纹理坐标区域
    uv_rect: (f32, f32, f32, f32),
    /// 字形尺寸
    size: (f32, f32),
}

impl TextRenderer {
    /// 创建文本渲染器
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self;

    /// 渲染一行文本到 RenderPass
    ///
    /// text: 文本内容
    /// x, y: 左上角坐标
    /// font_size: 字号
    /// color: 文字颜色
    pub fn render_text(
        &mut self,
        pass: &mut wgpu::RenderPass,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: &Color,
    );

    /// 获取或生成字形纹理
    fn get_or_cache_glyph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        face: &mut rustybuzz::Face,
        glyph_id: u16,
        font_size: f32,
    ) -> &GlyphTexture;

    // Phase 1+:
    // /// 切换到 SDF（Signed Distance Field）渲染模式
    // /// SDF 在缩放时更清晰
    // pub fn set_sdf_mode(&mut self, enabled: bool);
}
```

### 6.5 render 公开导出

```rust
// === crates/render/src/lib.rs ===

//! # render crate — 渲染后端 + wgpu 实现
//!
//! 通过 RenderBackend trait 支持可插拔渲染后端。
//! Phase 0 提供 wgpu 后端（桌面原生）。
//! Phase 3 增加 WebGPU WASM 后端（浏览器/小程序）。

pub use wgpu_backend::WgpuBackend;
pub use text_renderer::TextRenderer;

// Phase 3:
// pub mod webgpu;
// pub use webgpu::WebGpuBackend;
```

---

## 七、runtime crate — 运行时整合

### 7.1 模块结构

```
runtime/
├── Cargo.toml
└── src/
    ├── lib.rs              # 模块导出
    ├── window.rs           # WebWindow —— 对外入口
    ├── event_loop.rs       # 主事件循环
    └── hit_test.rs         # 命中检测
```

### 7.2 WebWindow — 应用程序入口

```rust
// === crates/runtime/src/window.rs ===

use dom::Document;
use dom::Node;
use layout::LayoutBox;
use layout::LayoutEngine;
use css::ComputedStyle;
use paint::DisplayList;

/// WebWindow —— 应用主入口
///
/// 封装窗口创建、渲染循环、事件处理。
///
/// 使用方式：
/// ```rust
/// use runtime::WebWindow;
///
/// let window = WebWindow::new("My App", 800, 600);
/// let doc = window.document();
/// // ... 使用 DOM API 构建 UI ...
/// window.run(); // 启动事件循环
/// ```
pub struct WebWindow {
    /// winit 窗口
    window: winit::window::Window,
    /// 事件循环
    event_loop: winit::event_loop::EventLoop<()>,
    /// DOM Document
    document: Rc<RefCell<Document>>,
    /// 渲染后端
    renderer: Option<WgpuBackend>,
    /// 布局引擎
    layout_engine: LayoutEngine,
    /// 文本测量器（用于布局阶段的尺寸计算）
    text_measurer: layout::TextMeasurer,
    /// 标记是否需要重绘
    needs_redraw: bool,
    // Phase 1+:
    // stylesheets: Vec<StyleSheet>,
    // computed_styles: HashMap<usize, ComputedStyle>,
}

impl WebWindow {
    /// 创建应用窗口
    ///
    /// title: 窗口标题
    /// width: 窗口宽度（CSS 像素）
    /// height: 窗口高度（CSS 像素）
    pub fn new(title: &str, width: u32, height: u32) -> Self;

    /// 获取 Document 对象
    pub fn document(&self) -> Rc<RefCell<Document>>;

    /// 启动主事件循环（阻塞，直到窗口关闭）
    pub fn run(self);

    /// 标记需要重绘
    pub fn request_redraw(&mut self);

    // ============================================================
    //  内部方法
    // ============================================================

    /// 主渲染循环（每帧调用）
    fn render_frame(&mut self);

    /// 完整渲染管线执行
    ///
    /// 1. DOM → ComputedStyle（css 引擎）
    /// 2. LayoutTree 构建 + 布局计算
    /// 3. DisplayList 构建
    /// 4. wgpu 渲染
    fn render_pipeline(&mut self);

    /// 处理 winit 输入事件 → DOM 事件
    fn handle_event(&mut self, event: &winit::event::WindowEvent);

    // Phase 1+:
    // /// 加载外部 CSS 样式表
    // pub fn load_stylesheet(&mut self, css_text: &str) -> StyleSheetId;
    // /// 监听 DOM 变更（MutationObserver）
    // fn observe_dom_changes(&self);
}
```

### 7.3 EventLoop — 事件循环

```rust
// === crates/runtime/src/event_loop.rs ===

use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

/// 事件循环运行器
///
/// 负责：
/// 1. 处理 winit 窗口事件
/// 2. 调用渲染管线的 render_frame
/// 3. 将 winit 事件转为 DOM 事件
pub(crate) struct EventLoopRunner {
    /// 窗口引用
    window: Rc<RefCell<WebWindow>>,
}

impl EventLoopRunner {
    /// 创建事件循环运行器
    pub fn new(window: Rc<RefCell<WebWindow>>) -> Self;

    /// 事件循环主函数
    ///
    /// 处理 RedrawRequested / WindowEvent / MainEventsCleared
    pub fn run(
        self,
        event_loop: winit::event_loop::EventLoop<()>,
    );

    /// 将 winit WindowEvent 转换为 DOM Event
    ///
    /// 映射表：
    ///   winit CursorMoved → DOM mousemove + MouseEvent
    ///   winit MouseInput → DOM click / mousedown / mouseup
    ///   winit KeyboardInput → DOM keydown / keyup (Phase 1+)
    ///   winit CloseRequested → 关闭窗口
    fn convert_winit_event(
        window: &WebWindow,
        event: &WindowEvent,
    );

    // Phase 1+:
    // fn handle_touch_event(...)
    // fn handle_wheel_event(...)
    // fn handle_focus_event(...)
}

// Phase 1+: requestAnimationFrame 调度器
// pub struct AnimationFrameScheduler {
//     frame_callbacks: Vec<Box<dyn FnMut(f64)>>,
//     frame_id: u32,
// }
// impl AnimationFrameScheduler {
//     pub fn request_animation_frame(&mut self, callback: Box<dyn FnMut(f64)>) -> u32;
//     pub fn cancel_animation_frame(&mut self, id: u32);
//     pub fn tick(&mut self, timestamp: f64);
// }
```

### 7.4 HitTest — 命中检测

```rust
// === crates/runtime/src/hit_test.rs ===

use layout::LayoutBox;
use dom::Node;

/// 命中检测 —— 根据屏幕坐标找到对应的 DOM 节点
///
/// 用于将鼠标点击事件分发到正确的元素
pub struct HitTester;

impl HitTester {
    /// 从布局树中查找坐标 (x, y) 对应的最深层节点
    ///
    /// 从根节点开始，逆序遍历子节点（z-order 高的优先）
    /// 返回第一个包含该坐标的叶子节点
    pub fn hit_test<'a>(
        root: &'a LayoutBox,
        x: f32,
        y: f32,
    ) -> Option<&'a LayoutBox>;

    /// 检查矩形是否包含坐标点
    fn rect_contains(rect: &Rect<f32>, x: f32, y: f32) -> bool;

    /// 收集从根到目标节点的路径（用于事件冒泡）
    pub fn collect_bubble_path<'a>(
        root: &'a LayoutBox,
        target: &'a LayoutBox,
    ) -> Vec<&'a LayoutBox>;

    // Phase 1+:
    // /// 多点触摸命中检测
    // pub fn multi_hit_test(
    //     root: &LayoutBox,
    //     points: &[(f32, f32)],
    // ) -> Vec<Option<&LayoutBox>>;
}
```

### 7.5 runtime 公开导出

```rust
// === crates/runtime/src/lib.rs ===

//! # runtime crate — 浏览器引擎运行时整合
//!
//! 整合所有底层模块，提供应用入口。
//! WebWindow 是唯一对外公开的 API。

pub use window::WebWindow;

// Phase 1+:
// pub use event_loop::AnimationFrameScheduler;
// pub use hit_test::HitTester;
```

---

## 八、Phase 0 完整调用链路

以下是计数器 Demo 从初始化到点击按钮的完整调用链：

### 初始化阶段

```
WebWindow::new("Counter", 400, 300)
  ├── winit::window::Window::new(&event_loop)
  ├── WgpuBackend::new(&window)
  │     ├── wgpu::Instance::new()
  │     ├── instance.create_surface(&window)
  │     ├── adapter.request_device()
  │     ├── device.create_render_pipeline(rect, ...)
  │     └── TextRenderer::new(&device, format)
  ├── Document::new()
  │     ├── Node::new(Document)
  │     ├── Node::new(Element(html)).append_child → document
  │     ├── Node::new(Element(head)).append_child → html
  │     └── Node::new(Element(body)).append_child → html
  └── LayoutEngine::new()
        └── taffy::Taffy::new()
```

### DOM 构建阶段（用户代码）

```
doc.create_element("div")        → Node { NodeType::Element(ElementData{tag:"div"}) }
div.set_attribute("style", ...)  → ElementData::parse_and_set_style("display:flex;...")
doc.create_element("h1")         → Node { NodeType::Element(ElementData{tag:"h1"}) }
h1.set_text_content("Counter")   → Node { children: [Text("Counter")] }
div.append_child(h1)             → h1.parent = div; div.children = [h1]
... (其他节点相同模式)
doc.body().append_child(div)     → div 挂到 body 下
```

### 渲染帧（window.run → render_frame）

```
render_pipeline()
  ├── css::compute_element_style() → ComputedStyle (每个节点)
  ├── layout::build_layout_tree()  → LayoutBox 树
  ├── LayoutEngine::layout()       → 填充 rect
  │     ├── FlexLayout::layout()   → taffy 计算
  │     ├── BlockLayout::layout()  → 从上到下排列
  │     └── PositionedLayout::layout() → 绝对定位
  ├── DisplayListBuilder::build()  → DisplayList
  │     ├── process_node(div)      → FillRect{background}
  │     ├── process_node(h1)       → Text{"Counter"}
  │     └── process_node(button)   → FillRect + Text{"+"}
  ├── display_list.sort_by_z_order()
  └── WgpuBackend::render()
        ├── for each PaintCommand:
        │     FillRect → encode_rect()
        │     Text     → encode_text()
        └── pass.present()
```

### 事件处理（用户点击按钮）

```
winit::event::WindowEvent::MouseInput { state: Pressed }
  ├── EventLoopRunner::convert_winit_event()
  │     ├── HitTester::hit_test(root, x, y)  → button LayoutBox
  │     └── MouseEvent::new("click", x, y, 0)
  ├── ElementData::dispatch_event(&event)
  │     ├── button.event["click"] iter
  │     └── callback(&event)  → 用户代码
  ├── 用户回调: display.set_text_content("1")
  │     └── Node::mark_dirty(true) → 标记重排
  └── window.request_redraw() → render_frame()
```

---

## 十、web2rust crate — HTML+CSS+JS → Rust 编译器

### 10.1 模块结构

```
web2rust/
├── Cargo.toml              # 无外部依赖，纯 Rust 标准库
└── src/
    ├── lib.rs              # compile() / compile_body() 入口 + 代码生成
    ├── html.rs             # 手写标签解析器 → 元素树
    ├── css.rs              # CSS 规则解析 + 元素匹配
    └── js.rs               # JS 模式识别 → 事件处理器
```

### 10.2 核心 API

```rust
// === crates/web2rust/src/lib.rs ===

use std::collections::HashMap;
use std::fs;

/// HTML 元素节点（HTML 解析器的输出）
pub use html::HtmlElement;

/// 编译 HTML+CSS+JS 为完整可编译的 main.rs
///
/// ```rust,no_run
/// let code = web2rust::compile("index.html", "style.css", "app.js");
/// std::fs::write("src/main.rs", code).unwrap();
/// ```
pub fn compile(html_path: &str, css_path: &str, js_path: &str) -> String;

/// 编译为 main() 函数体（用于 include! 方式，无 fn main() 包裹）
pub fn compile_body(html_path: &str, css_path: &str, js_path: &str) -> String;

/// 编译为完整 main.rs 并写入输出文件
pub fn compile_to_file(html_path: &str, css_path: &str, js_path: &str, output_path: &str);

/// 编译为 main() 函数体并写入输出文件
pub fn compile_body_to_file(html_path: &str, css_path: &str, js_path: &str, output_path: &str);
```

### 10.3 编译器数据流

```text
index.html
    │
    ▼
html::parse_html() → 元素树 (Vec<HtmlElement>)
    │
    ├── assign_variable_names() → Vec<VarAssignment>
    │       命名规则: id > class > tag（重复加数字后缀）
    │
    ├── css::parse_css() → Vec<CssRule>
    │       │
    │       ▼
    │   css::match_css_to_elements() → Vec<(var_name, style_string)>
    │
    ├── js::compile_js() → Vec<EventHandler>
    │       │
    │       ▼
    │   extract_event_handlers() + translate_event_body()
    │
    ▼
generate_main_body() → String
    ├── 创建窗口 + Document
    ├── 生成 create_element / set_attribute 代码
    ├── 挂载元素到 body
    ├── 应用 CSS 样式（set_style 调用）
    ├── 生成事件监听器（add_event_listener 闭包）
    └── 调用 window.run()
```

### 10.4 HTML 解析器 (html.rs)

```rust
// === crates/web2rust/src/html.rs ===

use std::collections::HashMap;

/// HTML 元素节点
#[derive(Debug, Clone)]
pub struct HtmlElement {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub text_content: String,
    pub children: Vec<HtmlElement>,
}

/// 主入口：解析 HTML 字符串，返回 body 内元素树
///
/// 支持：
/// - 开闭标签：<div>...</div>
/// - 属性：class="foo" id="bar"
/// - 自闭合标签：<br>, <link>, <input>
/// - 文本内容提取
/// - 过滤 <head>/<script>/<link>/<title>/<!DOCTYPE>
/// - 只处理 <body> 内元素
pub fn parse_html(html: &str) -> Vec<HtmlElement>;
```

### 10.5 CSS 处理器 (css.rs)

```rust
// === crates/web2rust/src/css.rs ===

/// CSS 规则
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selector: String,
    pub declarations: Vec<(String, String)>,
}

/// 解析 CSS 文本为规则列表（去除注释、处理嵌套括号）
pub fn parse_css(css: &str) -> Vec<CssRule>;

/// 检查选择器是否匹配元素
/// 支持：tag, .class, #id, tag.class, tag#id, tag.class#id
pub fn selector_matches(
    selector: &str,
    tag: &str,
    classes: &[String],
    id: Option<&str>,
) -> bool;

/// 将 CSS 规则匹配到元素树，返回 (variable_name, style_string) 列表
pub fn match_css_to_elements(
    css_rules: &[CssRule],
    elements: &[HtmlElement],
    element_vars: &[(String, HtmlElement)],
) -> Vec<(String, String)>;
```

### 10.6 JS 模式识别 (js.rs)

Phase 0 不做完整 JS 编译，而是通过模式匹配识别特定 DOM API 调用。

```rust
// === crates/web2rust/src/js.rs ===

use std::collections::HashMap;

/// 事件处理器信息
#[derive(Debug)]
pub struct EventHandler {
    /// 目标元素的变量名
    pub element_var: String,
    /// 事件类型："click", "mousedown" 等
    pub event_type: String,
    /// 处理器体的 Rust 代码
    pub body_code: String,
    /// 需要在闭包前 clone 的变量列表
    pub cloned_vars: Vec<String>,
}

/// 构建 HTML 查询查找表
/// 键：选择器（"id", ".class", "tag"）→ 变量名
pub fn build_html_lookup(
    element_vars: &[(String, HtmlElement)]
) -> HashMap<String, String>;

/// 从 JS 代码中提取事件处理器
pub fn extract_event_handlers(
    js: &str,
    html_lookup: &HashMap<String, String>,
) -> Vec<EventHandler>;

/// 主入口：从 JS 代码中识别事件处理器
pub fn compile_js(
    js: &str,
    element_vars: &[(String, HtmlElement)],
) -> Vec<EventHandler>;
```

**支持的 JS 模式**：

| JS 模式 | Rust 生成代码 |
|---------|--------------|
| `document.querySelector('.class')` | 解析为元素的 Rust 变量名 |
| `document.getElementById('id')` | 解析为元素的 Rust 变量名 |
| `element.addEventListener('click', function() { ... })` | `element.borrow_mut().add_event_listener("click", Box::new(move \|_\| { ... }))` |
| `element.textContent = expr` | `element_clone.borrow_mut().set_text_content(&expr.to_string())` |
| `let x = val` / `const x = val` | 识别但不生成代码（编译时已推导）|
| `x = x + 1` | 映射 Rust 算术（仅在 textContent 递增上下文）|
| `parseInt(x)` | `x.parse::<i32>().unwrap_or(0)` |
| `x.toString()` | `x.to_string()` |

### 10.7 编译输出示例

以下是从 counter demo 源文件编译生成的 Rust 代码概览：

```rust
pub fn run() {
    let mut window = WebWindow::new("Demo", 800, 600);
    let doc = window.document();

    // ====== Compiled from index.html ======
    let container = doc.borrow().create_element("div");
    container.borrow_mut().set_attribute("class", "container");
    let h1 = doc.borrow().create_element("h1");
    h1.borrow_mut().set_text_content("Counter");
    container.borrow_mut().append_child(h1.clone());
    let display = doc.borrow().create_element("div");
    display.borrow_mut().set_text_content("0");
    container.borrow_mut().append_child(display.clone());
    let inc_btn = doc.borrow().create_element("button");
    inc_btn.borrow_mut().set_attribute("id", "inc-btn");
    inc_btn.borrow_mut().set_text_content("+");
    container.borrow_mut().append_child(inc_btn.clone());

    // ====== Mount to body ======
    doc.borrow().body().borrow_mut().append_child(container.clone());

    // ====== Compiled from style.css ======
    container.borrow_mut().set_style("background: #f5f5f5; padding: 20px");
    // ... (其他样式)

    // ====== Compiled from app.js ======
    let display_clone = display.clone();
    inc_btn.borrow_mut().add_event_listener("click", Box::new(move |_: &dom::Event| {
        let val = display_clone.borrow().text_content().parse::<i32>().unwrap_or(0) + 1;
        display_clone.borrow_mut().set_text_content(&val.to_string());
    }));

    // ====== Start rendering ======
    window.run();
}
```

---

## 十一、Phase 0 → Phase 1 扩展点汇总

| 位置 | 预留标记 | 即将新增 |
|------|----------|----------|
| `node.rs` | `// Phase 1+` | `owner_document()`, `normalize()`, `is_equal_node()` |
| `element.rs` | `// Phase 1+` | `inner_html`, `query_selector`, `closest()`, `matches()`, `focus()` |
| `document.rs` | `// Phase 1+` | `getElementById`, `createDocumentFragment`, `createComment`, `querySelector` |
| `text.rs` | `// Phase 1+` | `splitText()`, `appendData()`, `deleteData()` 等文本操作方法 |
| `event.rs` | `// Phase 1+` | `KeyboardEvent`, `FocusEvent`, `WheelEvent`, `stopImmediatePropagation` |
| `dom_token_list.rs` | `// Phase 1+` | `replace()`, `supports()`, `length`, `item()` |
| `lib.rs (dom)` | `// Phase 1+` | `KeyboardEvent`, `FocusEvent`, `WheelEvent` 导出 |
| `stylesheet.rs` | `// Phase 1+` | `parseMediaQuery`, `parseKeyframes`, `parseFontFace` |
| `selector.rs` | `// Phase 1+` | `SelectorEngine` 结构体, 完整选择器支持 |
| `values.rs` | `// Phase 1+` | `parseTransform`, `parseGradient`, `Vw/Vh` 单位 |
| `properties.toml` | `// Phase 1+ 属性预留` | `min-width`, `visibility`, `font-style`, `z-index` 等 50+ 属性 |
| `layout_box.rs` | `// Phase 1+` | `InlineBlock`, `Table*`, `Grid*` 等布局类型 |
| `flex.rs` | `// Phase 1+` | `min-width/max-width` 约束处理 |
| `block.rs` | `// Phase 1+` | `margin collapse`（外边距合并）|
| `positioned.rs` | `// Phase 1+` | `position: fixed/sticky` |
| `text.rs (layout)` | `// Phase 1+` | 断字换行, 字体回退链, 行高计算 |
| `command.rs` | `// Phase 1+` | `BoxShadow`, `Image`, `Clip`, `Opacity` 命令 |
| `builder.rs` | `// Phase 1+` | `BatchOptimizer` 合批优化 |
| `wgpu_backend.rs` | `// Phase 1+` | `border_pipeline`, `shadow_pipeline`, `image_pipeline` |
| `text_renderer.rs` | `// Phase 1+` | SDF 渲染模式, 更复杂的字体回退 |
| `window.rs` | `// Phase 1+` | `loadStylesheet`, 外部 CSS 加载 |
| `event_loop.rs` | `// Phase 1+` | `AnimationFrameScheduler`, 触摸事件 |
| `render/src/lib.rs` | `// Phase 3` | `WebGpuBackend` WASM 后端 |
| `lib.rs (dom)` | `// Phase 3` | `TouchEvent` 导出 |
| `web2rust/html.rs` | `// Phase 1+` | 替换为 html5ever 完整解析 |
| `web2rust/css.rs` | `// Phase 1+` | 替换为 cssparser + selectors crate |
| `web2rust/js.rs` | `// Phase 1+` | 替换为 swc 完整 AST 编译 |
