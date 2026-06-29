# Ruft 架构文档 — 编译流程与运行时

## 总览

```
  index.html                 编译阶段                   运行时
  style.css   ──►  compiler ──►  Rust 代码  ──►  runtime
  app.js                       (VNode + Select)       (扁平 DOM + CSS 匹配)
```

项目将 **HTML + CSS + JS** 编译为 **纯 Rust 代码**，运行时通过统一的 `runtime` crate（扁平 DOM 注册表 + CSS 样式引擎）驱动渲染管线。

---

## 第一部分：编译阶段（Compile Time）

### 1.1 CLI 入口 → 配置解析

```
用户命令:  cargo run -p cli -- compile examples/counter -o target/generated/counter
                │
                ▼
          cli/src/main.rs
          Args { input_dir, output_dir, name, title, width, height }
                │
                ▼
          compiler::config::resolve()
                │
                ▼
          ResolvedConfig  (补全默认值：title/width/height/name)
```

### 1.2 HTML 解析 → HtmlResources

```
  index.html (字符串)
        │
        ▼
  html::parse_html_document()
        │
        ├─ strip_comments()        去除 HTML 注释
        ├─ extract_html_content()  截取 <html>...</html> 内容区
        ├─ tokenize()              词法分析 → Vec<Token>
        └─ build_document()        一次遍历构建 DOM 树 + 收集资源
                │
                ▼
  HtmlResources {
      elements: Vec<HtmlElement>,     // 完整 DOM 树 (html→head/body→...)
      css_sources: Vec<CssSource>,    // 每个 <link> / <style> / style=""
      js_sources: Vec<JsSource>,      // 每个 <script src> / <script>
  }
```

**`HtmlElement` 数据结构：**
```rust
pub struct HtmlElement {
    pub tag: String,                        // "div", "h1", "button" ...
    pub attributes: HashMap<String, String>, // id, class, style ...
    pub text_content: String,               // 文本内容
    pub children: Vec<HtmlElement>,         // 子元素
}
```

**`CssSource` — CSS 来源三种形式：**
```rust
pub enum CssSource {
    File(PathBuf),                              // <link href="style.css">
    Inline(String),                             // <style>...</style>
    InlineAttr { var_name: String, content: String },  // style="color: red"
}
```

### 1.3 CSS 解析 → CssRule 列表

```
  style.css / <style> 源码字符串
        │
        ▼
  css::parse_css()
        │
        ├─ strip_css_comments()        去除 /* ... */
        ├─ split_selector_list()       按逗号拆分并列选择器 (如 ".av, .ab")
        ├─ normalize_whitespace()      规范化空白字符
        ├─ parse_selector()            解析选择器为结构化段
        │     │
        │     ▼
        │  Vec<SelectorSegment>
        │  · combinator: child(>) / descendant(空格) / sibling(~)
        │  · tag: Option<String>
        │  · id: Option<String>
        │  · classes: Vec<String>
        │  · attrs / pseudo_classes / pseudo_elements
        │
        └─ parse_declarations()        解析 { prop: val; ... }
                │
                ▼
  Vec<CssRule>  (编译器内部类型)
  · selector: String           原始选择器文本
  · declarations: Vec<(String, String)>   声明列表
```

### 1.4 编译管线 → 代码生成

```
  HtmlResources
        │
        ▼
  pipeline::compile_resources()
        │
        ├─ Phase 1: 整理 CSS 模块
        │     每个 CssSource → CssModuleSpec {
        │         module_name:  "style" | "__style_0" | "__attr_0",
        │         href:         Some("style.css") | None,
        │         rules:        Vec<CssRule>
        │     }
        │     · 外部文件 → href=Some, module_name=文件名stem
        │     · <style>   → href=None, module_name=__style_N
        │     · style=""  → href=None, module_name=__attr_N
        │
        ├─ Phase 2: 元素变量分配
        │     variable::build_all_element_vars()
        │     为每个 HtmlElement 分配唯一变量名
        │
        ├─ Phase 3: CSS 规则解析
        │     对每个 CssModuleSpec 调用 css::parse_css()
        │
        ├─ Phase 4: JS 编译
        │     对每个 JsSource 调用 js::compile_js()
        │     识别 DOM API 模式 → 生成事件处理器
        │
        └─ Phase 5: 代码生成
              codegen::generate_multi_file() → MultiFileOutput
```

### 1.5 代码生成器 — 核心转换逻辑

```
  Vec<HtmlElement>  ──┐
  Vec<CssModuleSpec> ─┼──►  codegen::generate_multi_file()
  Vec<JsHandlers>    ─┘
                              │
                              ▼
  MultiFileOutput {
      main_rs:    String,     // 入口：DomRegistry + StyleEngine
      html_rs:    String,     // VNode 树 + 内联 <style>
      style_files: Vec<(stem, content)>,   // 外部 CSS
      handler_files: Vec<(stem, content)>, // JS 事件
  }
```

**各生成函数的转换规则：**

| 生成函数 | 输入 | 输出格式 |
|---------|------|---------|
| `html_element_to_vnode()` | `HtmlElement` | `VElementVNode::new("div").with_classes([...]).with_children([...])` |
| `selector_to_builder_expr()` | CSS 选择器字符串 | `Select::tag("div").and_class("foo").child().tag("h1")` |
| `generate_html_module()` | elements + 内联 CSS 规则 | `build_vnode()` 函数 + `create_style_sheets()` 函数 |
| `generate_style_module()` | 外部 CSS 规则 | `create_style_sheet() -> CssStyleSheet` (含 `CssRule::new(Select::...).decl(...)`) |
| `generate_main_module()` | 所有模块信息 | `main()` 入口 (初始化 + 注册 + 启动) |

---

## 第二部分：生成的 Rust 代码结构

以 `counter` 示例为例，编译后生成的文件：

```
target/generated/counter/
├── Cargo.toml        依赖 runtime
└── src/
    ├── main.rs       入口
    ├── html.rs       build_vnode() + create_style_sheets()
    ├── style.rs      create_style_sheet() (外部 CSS)
    └── app.js        setup_handlers() (JS 事件)
```

### 2.1 `html.rs` — VNode 树 + 内联样式

```rust
// 由 index.html 编译生成 — VNode 树 + 内联 <style>
use runtime::*;

pub fn build_vnode() -> VNode {
    VElementVNode::new("html")
        .with_children(vec![
            VElementVNode::new("head")
                .with_children(vec![
                    VElementVNode::new("title")
                        .with_children(vec![VNode::text("Counter App")]),
                ]),
            VElementVNode::new("body")
                .with_children(vec![
                    VElementVNode::new("div")
                        .with_classes(vec!["container"])
                        .with_children(vec![
                            VElementVNode::new("h1")
                                .with_children(vec![VNode::text("Counter")]),
                            VElementVNode::new("div")
                                .with_classes(vec!["display"])
                                .with_children(vec![VNode::text("0")]),
                            VElementVNode::new("button")
                                .with_id("inc-btn")
                                .with_children(vec![VNode::text("+")]),
                        ]),
                ]),
        ]).into()
}

pub fn create_style_sheets() -> Vec<CssStyleSheet> {
    vec![
        CssStyleSheet {
            sheetType: "text/css".into(),
            href: None,                         // None = 内联样式
            cssRules: vec![
                CssRule::new(Select::tag("h1")) // Select builder 链式构建
                    .decl("color", "red"),
            ],
            ..Default::default()
        },
    ]
}
```

### 2.2 `style.rs` — 外部 CSS 文件

```rust
// 由 style.css 编译生成
use runtime::{CssStyleSheet, CssRule, Select};

pub fn create_style_sheet() -> CssStyleSheet {
    let rules = vec![
        CssRule::new(Select::tag("body"))
            .decl("font-family", "sans-serif")
            .decl("margin", "0"),
        CssRule::new(Select::class("container").child().and_tag("h1"))
            .decl("background", "#e8e8e8")
            .decl("font-size", "24px"),
        CssRule::new(Select::class("container").descendant().and_tag("button"))
            .decl("background", "#007bff")
            .decl("color", "white"),
    ];

    CssStyleSheet {
        sheetType: "text/css".into(),
        href: Some("examples/counter/style.css".into()),  // 来源文件
        cssRules: rules,
        ..Default::default()
    }
}
```

### 2.3 `main.rs` — 运行时入口

```rust
use runtime::*;
use runtime::StyleEngine;

mod html;
mod style;
mod app;

fn main() {
    let mut document = DomRegistry::new();
    let mut style_engine = StyleEngine::new();

    // 1. 加载 DOM 树
    document.loadVnode(html::build_vnode());

    // 2. 注册样式表
    for sheet in html::create_style_sheets() {
        document.addStyleSheet(sheet);
    }
    document.addStyleSheet(style::create_style_sheet());

    // 3. 启动样式引擎
    style_engine.run(&mut document, |doc| {
        app::setup_handlers(doc);
    });
}
```

---

## 第三部分：运行时（Runtime）

### 3.1 核心类型关系

```
  VNode 树 (编译产物)
      │ loadVnode()
      ▼
  DomRegistry (运行时 document)
  ├── allNodes: HashMap<NodeId, DomNode>   ← 扁平存储，避免 Rust 借用难题
  ├── idMap / classMap / tagMap            ← 反向索引，O(1) 查询
  ├── styleSheets: Vec<CssStyleSheet>      ← document.styleSheets
  └── styleDirtySet: HashSet<NodeId>       ← 脏标记追踪
      ▲
      │ computeStyle / flushStyleDirty
      │
  StyleEngine
  ├── RuleIndex                           ← 规则候选加速索引
  └── cascade()                           ← CSS 层叠计算
      │
      ▼
  DomNode.computedStyle: ComputedStyle     ← 最终样式结果
  DomNode.layoutDirty: bool               ← 触发布局重算
```

### 3.2 VNode → DomNode 转换

```
  VNode::Element(VElementVNode {
      tagName: "div",
      classList: ["container"],
      id: Some("app"),
      style: "color: red",
      childNodes: [VNode::Text("hello")],
  })
        │  DomRegistry::vnodeToDom()
        ▼
  DomNode {
      nodeType: Element,
      tagName: "div",
      classList: ["container"],
      id: Some("app"),
      attributes: {"style": "color: red"},
      childNodes: [NodeId(3)],      // 纯 ID 引用
      parentNode: Some(NodeId(1)),
      computedStyle: ComputedStyle::default(),  // 待计算
      styleDirty: true,
      layoutDirty: false,
  }
```

**关键：** 运行时 `DomNode` 不使用 `Rc<RefCell<>>`，而是通过 `NodeId` (u64) 扁平索引 `HashMap<NodeId, DomNode>`，彻底避免了 Rust 的所有权/借用冲突。

### 3.3 CSS 规则匹配流程

```
  StyleEngine::computeStyle(node_id)
        │
        ├─ 1. RuleIndex::find_candidates(node_id)
        │       基于 tagName / id / classList 索引粗筛
        │       排除不可能匹配的规则
        │
        ├─ 2. 对候选规则逐一调用 ComplexSelector::matches()
        │       从右向左匹配选择器段（如 ".container > h1" 先匹配 h1，再检查父 .container）
        │
        ├─ 3. 收集命中规则并排序
        │       排序键：specificity (id, class, tag) → sheet 顺序 → 规则内位置
        │       !important 提升优先级
        │       内联 style 优先级最高
        │
        ├─ 4. cascade() 层叠计算
        │       按优先级顺序合并所有命中规则的声明
        │       处理继承属性（如 font-size, color）
        │       输出 ComputedStyle
        │
        ▼
  ComputedStyle {
      fontSize: 24.0,
      textColor: 0xFF333333,
      bgColor: 0xFFE8E8E8,
      padding: 20.0,
      margin: 0.0,
      ...
  }
```

### 3.4 Select Builder 运行时语义

编译器生成的 `Select` builder 链式调用在运行时不涉及字符串解析：

```rust
// 编译期：CSS ".container > h1" → 代码生成 → 运行时：零解析
Select::class("container")   // 起始段：.container
      .child()               // 组合器：>
      .and_tag("h1")         // 第二段：h1
//    .done()                // CssRule::new() 内部调用

// 等价 ComplexSelector:
// segments: [
//   SelectSegment { combinator: None,   classes: ["container"] },
//   SelectSegment { combinator: Child,  tag: Some("h1") },
// ]
```

`Select` builder 构建过程中实时累加特异性 `(id_count, class_count, tag_count)`，`.done()` 时零额外计算即可产出 `ComplexSelector`。

### 3.5 样式重新计算（Style Invalidation）

```
  JS 事件触发
      │ doc.setTextContent(node_id, "42")
      │ doc.setAttribute(node_id, "class", "active")
      ▼
  DomRegistry.styleDirtySet  ←  标记受影响的节点
      │
      ▼
  StyleEngine::flushStyleDirty()
      │
      ├─ 批量计算所有 dirty 节点的 ComputedStyle
      │
      ├─ 比较新旧样式，若变化：
      │     node.layoutDirty = true   ← 向上传播到父节点
      │     node.paintDirty  = true
      │
      └─ 清空 styleDirtySet

  后续管线消费 layoutDirty / paintDirty 标记进行布局和绘制
```

---

## 第四部分：对照表 — 前端概念到 Rust 实现

| 前端概念 | 编译期 (compiler crate) | 运行时 (runtime crate) |
|---------|------------------------|----------------------------------|
| `document.createElement("div")` | `VElementVNode::new("div")` | `DomRegistry::createElement("div")` |
| `.className = "foo"` | `.with_classes(["foo"])` | `classList: Vec<String>` + `classMap` 索引 |
| `.id = "app"` | `.with_id("app")` | `idMap: HashMap<String, NodeId>` |
| `.appendChild(child)` | `.with_children([...])` | `DomRegistry::appendChild(parent, child)` |
| `.textContent = "hi"` | `VNode::text("hi")` | `DomRegistry::setTextContent(node, "hi")` |
| `document.styleSheets` | `CssStyleSheet` 结构体 | `StyleSheetList` → `Vec<CssStyleSheet>` |
| CSS 规则 `h1 { color: red }` | `CssRule::new(Select::tag("h1")).decl("color", "red")` | `ComplexSelector::matches()` 匹配 → `cascade()` 计算 |
| CSS 选择器 `.foo > .bar` | `Select::class("foo").child().and_class("bar")` | `SelectSegment` 链 + combinator 匹配 |
| CSS 特异性 | `Select` builder 构建时累加 `(id, class, tag)` | `Specificity` 元组比较排序 |
| `style.cssText = "..."` | `parse_declarations()` 解析 | `computedStyle` 计算 |
| `<style>...</style>` | `href = None` 内联 CSS | 作为规则源参与匹配 |
| `<link href="...">` | `href = Some(path)` 外部 CSS | 同上，保留来源路径 |
| `element.addEventListener(...)` | `js::compile_js()` 模式识别 | `DomRegistry` + 闭包绑定 |

---

## 第五部分：数据流全景图

```
╔═══════════════════════════════════════════════════════════════════╗
║                         编译阶段 (Build Time)                     ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                                   ║
║  ┌──────────┐    ┌──────────┐    ┌──────────┐                    ║
║  │index.html│    │ style.css│    │  app.js  │                    ║
║  └────┬─────┘    └────┬─────┘    └────┬─────┘                    ║
║       │               │               │                          ║
║       ▼               ▼               ▼                          ║
║  html::parse_    css::parse_     js::compile_                     ║
║  html_document   css()           js()                             ║
║       │               │               │                          ║
║       ▼               ▼               ▼                          ║
║  HtmlElement[]   CssRule[]       EventHandler[]                   ║
║       │               │               │                          ║
║       └───────────────┴───────────────┘                          ║
║                       │                                          ║
║                       ▼                                          ║
║              codegen::generate_multi_file()                       ║
║                       │                                          ║
║                       ▼                                          ║
║  ┌─────────────────────────────────────────────┐                 ║
║  │          生成的 Rust 代码                     │                 ║
║  │  ├─ build_vnode()          VNode 树构造      │                 ║
║  │  ├─ create_style_sheets()  样式表注册        │                 ║
║  │  ├─ setup_handlers()       事件绑定          │                 ║
║  │  └─ main()                 初始化 + 启动     │                 ║
║  └─────────────────────────────────────────────┘                 ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝
                              │
                              │  cargo run
                              ▼
╔═══════════════════════════════════════════════════════════════════╗
║                        运行时 (Runtime)                           ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                                   ║
║  ┌──────────┐    ┌─────────────┐    ┌──────────────┐             ║
║  │build_vnode│    │create_style │    │setup_handlers │             ║
║  │    ()     │    │ _sheets()   │    │     ()        │             ║
║  └────┬──────┘    └──────┬──────┘    └──────┬───────┘             ║
║       │                  │                  │                     ║
║       ▼                  ▼                  │                     ║
║  VNode 树          Vec<CssStyleSheet>        │                     ║
║       │                  │                  │                     ║
║       │    loadVnode()   │  addStyleSheet()  │                     ║
║       ▼                  ▼                  │                     ║
║  ┌──────────────────────────────────────────┐│                    ║
║  │          DomRegistry (document)          ││                    ║
║  │  allNodes:  HashMap<NodeId, DomNode>     │◄┘                   ║
║  │  idMap:     HashMap<String, NodeId>      │                     ║
║  │  classMap:  HashMap<String, Vec<Id>>     │                     ║
║  │  tagMap:    HashMap<String, Vec<Id>>     │                     ║
║  │  styleSheets: Vec<CssStyleSheet>         │                     ║
║  │  styleDirtySet: HashSet<NodeId>          │                     ║
║  └──────────────────┬───────────────────────┘                     ║
║                     │                                             ║
║                     ▼                                             ║
║  ┌──────────────────────────────────────────┐                    ║
║  │           StyleEngine                     │                    ║
║  │  rebuildIndices()  建立规则索引           │                    ║
║  │  computeStyle()    逐节点计算样式         │                    ║
║  │  flushStyleDirty() 批量刷新脏节点         │                    ║
║  └──────────────────┬───────────────────────┘                     ║
║                     │                                             ║
║                     ▼                                             ║
║  ┌──────────────────────────────────────────┐                    ║
║  │  DomNode.computedStyle: ComputedStyle    │                    ║
║  │  DomNode.layoutDirty / paintDirty        │                    ║
║  └──────────────────┬───────────────────────┘                     ║
║                     │                                             ║
║                     ▼                                             ║
║              布局引擎 → 渲染管线 → 原生窗口                       ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝
```
