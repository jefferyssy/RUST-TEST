# Rust 浏览器引擎 — CLAUDE.md

## 项目概述

**将原生 HTML+CSS+JS 前端项目编译为 Rust 代码**，通过 Rust 实现的 W3C 标准 DOM API 驱动自研渲染管线，输出原生跨平台应用。

数据流：`index.html + style.css + app.js → 编译器 → Rust 代码 → cargo build → 原生二进制`

Phase 0 目标：Rust 运行时（DOM API + CSS 引擎 + 布局 + wgpu 渲染）+ web2rust 编译器均可工作，demo 展示"从 HTML+CSS+JS 源文件编译为 Rust"的完整流程。

## 构建与运行

```bash
# 运行所有测试
cargo test

# 运行 counter demo
RUST_LOG=wgpu=warn cargo run -p counter

# 运行特定 crate 测试
cargo test -p dom
cargo test -p css
cargo test -p layout
cargo test -p paint
cargo test -p web2rust
```

## 工作区结构

```
crates/
├── dom/        # 核心 DOM 树 + W3C 标准 API（无依赖）
├── css/        # CSS 引擎（cssparser + 选择器 + 级联）
├── layout/     # 布局引擎（taffy flexbox + 自研 block）
├── paint/      # DisplayList 构建（LayoutTree → 绘制命令）
├── render/     # wgpu 渲染后端
├── runtime/    # 整合入口 + winit 事件循环
├── web2rust/   # 编译器：HTML+CSS+JS → Rust DOM API 代码
examples/
├── counter/       # 计数器 Demo — 源文件驱动
├── two-counters/  # 双计数器 Demo — 多独立状态
├── flex-nav/      # Flex 导航栏 Demo — flexbox 布局验证
└── dashboard/     # 仪表盘 Demo — 复杂布局
```

Crate 依赖链：`dom → css → layout → paint → render → runtime`（web2rust 是独立编译器，仅作 build-dependency）

## 编译器管线

### Phase 0（当前实现 — 无外部依赖）

```
index.html ─→ 手写 HTML 解析器 ─→ 元素树 → 变量名分配 → create_element 代码
style.css  ─→ 手写 CSS 解析器  ─→ 规则表 → 元素匹配 → set_style 代码
app.js     ─→ 模式识别          ─→ 识别 DOM API → 生成事件处理器代码
                                        ↓
                               generated.rs → cargo build → 原生二进制
```

### Phase 1+（计划）

```
index.html ─→ html5ever ─→ DOM 树构建代码 (create_element / append_child)
style.css  ─→ cssparser  ─→ StyleSheet 注册代码 (add_rule)
app.js     ─→ swc        ─→ Rust DOM API 调用 + 事件监听器
                            ↓
                       generated.rs → cargo build → 原生二进制
```

### Phase 0 web2rust 支持的 JS 模式

- `document.querySelector('.class')` / `document.querySelector('#id')` — 在 HTML 元素树中查找
- `document.getElementById('id')` — 同上
- `element.addEventListener('event', function() { ... })` — 生成闭包
- `element.textContent = expr` — 生成 set_text_content 调用
- `let/const` 声明 — 识别但不生成额外 Rust 代码（由编译时推导）
- 基本算术: `x = x + 1` — 映射 Rust 表达式
- `parseInt(x)`, `x.toString()` — 映射 Rust 类型转换

Phase 1+ 将替换为基于 swc 的完整 JS AST 编译。

### Phase 1+ 计划支持的 JS 子集

- DOM API: `document.createElement`, `.appendChild`, `.textContent`, `.setAttribute`, `.getAttribute`, `.addEventListener`, `.querySelector`, `.getElementById`, `.classList.add`, `.style.prop = val`
- 语法: `let/const`, `function`, `if/else`, 算术/字符串运算
- **不支持**: `eval`, `this`, `prototype`, `Promise/async/await`, `Proxy`, `Symbol`

## Demo 结构说明

counter demo 通过 `build.rs` 在构建时调用 web2rust 编译器，将源文件编译为 Rust 代码：

- **index.html** — 标准 HTML5 文档结构（`<div>`, `<h1>`, `<button>`）
- **style.css** — 标准 CSS 规则（背景色、字体、边距、边框）
- **app.js** — 标准 JS 交互（`querySelector`, `getElementById`, `addEventListener`）
- **build.rs** — 构建脚本，调用 `web2rust::compile_body()` 生成 Rust 代码
- **src/main.rs** — 仅通过 `include!` 引入构建产物，调用 `generated::run()`

构建流程：

1. cargo 编译 web2rust（build-dependency）
2. `build.rs` 执行 → web2rust 读取 `index.html` + `style.css` + `app.js` → 生成 `$OUT_DIR/counter_generated.rs`
3. `src/main.rs` 通过 `mod generated { include!(...) }` 引入生成的代码
4. cargo 编译成品 → 原生二进制

## 关键约定

### 命名

- 函数/方法：`snake_case`，镜像 W3C 标准名（`append_child`, `set_attribute`）
- 类型/枚举：`CamelCase`
- 常量：`SCREAMING_SNAKE_CASE`

### DOM 树 — Rc<RefCell<Node>>

- `Node::new()` 永远返回 `Rc<RefCell<Node>>`
- 所有 DOM API 接收/返回 `Rc<RefCell<Node>>`
- 子节点：`Vec<Rc<RefCell<Node>>>`
- 父节点：`Option<Weak<RefCell<Node>>>`（防止循环引用）
- 兄弟节点：`prev_sibling: Option<Weak<...>>` / `next_sibling: Option<Rc<...>>`

### weak_self / Rc::new_cyclic 模式

每个 Node 包含 `weak_self: Weak<RefCell<Node>>`，通过 `Rc::new_cyclic` 初始化：

```rust
pub fn new(node_type: NodeType) -> Rc<RefCell<Self>> {
    Rc::new_cyclic(|weak| RefCell::new(Self {
        weak_self: weak.clone(),
        // ...
    }))
}
```

在树操作中通过 `self.weak_self.upgrade().expect("Node was dropped")` 获取自身 Rc。

### Node → ElementData 便捷方法

Node 提供 `set_style()`、`add_event_listener()`、`remove_event_listener()`、`dispatch_event()` 等方法，内部委托给 `ElementData`，避免使用者手动匹配 `NodeType::Element`。

### 测试

- 所有测试均为内联 `#[cfg(test)] mod tests`，写在每个源文件末尾
- 使用 `use super::*` 访问私有项
- helper 函数定义在测试模块内
- doc-test 如果启动窗口需加 `no_run` 避免阻塞

### Phase 标注

代码中用 `// Phase 0:` / `// Phase 1+:` / `// Phase 2+:` 注释标记当前状态和未来计划。

## 渲染管线（runtime::window → App::render）

1. **compute_dom_styles** — 递归遍历 DOM，对每个元素匹配样式表 + 解析 inline style，输出 `HashMap<usize, ComputedStyle>`（key 为 `Rc::as_ptr(node) as usize`）
2. **build_layout_tree** — 从 DOM 树 + ComputedStyle 构建 LayoutBox 树，确定 BoxType（Block/FlexContainer/Inline/Text）
3. **LayoutEngine::layout** — 递归计算每个 LayoutBox 的 position 和 size（Flex → taffy / Block → 手动 Y 堆叠 / Positioned → 偏移）
4. **DisplayListBuilder::build** — 遍历布局树生成 PaintCommand 列表（FillRect/Border/Text）
5. **WgpuBackend::render** — 消费 DisplayList，通过 wgpu shader 绘制到窗口

## 内存模型要点

- **RefCell 双重借用**：不能在持有 `borrow()` 时调用同一节点的 `borrow_mut()`。需要手动 `drop(node)` 释放借用再写
- **Rc 所有权**：`append_child(child)` 消费 child 所有权，需在追加后继续使用时先 `.clone()`
- **布局树所有权**：LayoutBox 通过 `Vec<LayoutBox>` 拥有子节点（与 DOM 的 Rc 不同）。布局树到 DOM 的链接通过 `node: Option<Rc<RefCell<Node>>>` 维护
- **DisplayList 所有权**：`build()` 通过 `std::mem::take` 移交所有权
- **wgpu Surface 生命周期**：使用 `unsafe transmute` 将 `Surface<'_>` 转为 `Surface<'static>`（调用者保证 Window 比 Surface 存活更久）

## CSS 引擎

- **values**: CSSValue 枚举（Keyword/Length/Color/Percentage/Number/String），`parse_css_value(property, value)` 入口
- **selector**: Phase 0 手写实现，支持 `tag`, `.class`, `#id`, `tag.class#id` 匹配；`compute_specificity` 返回 `(id, class, tag)` 元组
- **cascade**: `compute_element_style` 按特异性排序 + `!important` 处理 + 继承属性传播
- **stylesheet**: `parse_stylesheet()` 当前为 stub（Phase 1+ 集成 cssparser）；`parse_inline_style` 可用
- **特异性**: `!important` > inline（`u32::MAX`） > 选择器匹配；元组比较 `(id, class, tag)`

## 常见陷阱

1. `parse_and_set_style` 必须同时写入 `style` HashMap 和 `attributes["style"]`（CSS 级联从后者读取）
2. 布局根节点的 `rect` 必须在 `layout()` 开始时设置为 viewport 尺寸，否则子节点 width=0 不可见
3. 文本节点必须有非零 `rect.height` 否则 wgpu 无法渲染（在 `build_layout_box` 中从 font-size 估算）
4. `"rem"` 必须在 `"em"` 之前检查，否则 `"2rem"` 会被误匹配为 em 单位
5. 事件回调中使用 `clone()` 增加引用计数，防止节点在回调执行时被释放
6. `append_child()` 消费子节点的 Rc 所有权，后续需要使用时务必先 `.clone()`
7. Color 没有 `RED`/`BLUE`/`GREEN` 常量，只有 `BLACK`、`WHITE`、`TRANSPARENT`；使用 `Color::rgb(r, g, b)` 构造

## wgpu 23.0.1 要点

- `PipelineCompilationOptions` 结构体需设置 `cache: None`
- `Surface<'window>` 使用 transmute 为 `'static`
- 着色器嵌入 WGSL 源码为 `const` 字符串
- 混合模式：`wgpu::BlendState::ALPHA_BLENDING`
- 清除色：`(0.95, 0.95, 0.95, 1.0)` 浅灰
- PresentMode：`Fifo`（vsync）

## winit 0.30 要点

- 使用 `ApplicationHandler` trait，非旧版 `EventLoop::run`
- `resumed()` 中创建窗口和 wgpu backend
- `about_to_wait()` 中调用 `window.request_redraw()` 持续刷新
- `EventLoop::run_app(&mut app)` 阻塞直到窗口关闭

## 语言要求

所有思考过程（thinking/reasoning_content）必须使用简体中文。
所有最终回答必须使用简体中文。
思考过程中禁止使用英文，仅代码关键字可用英文。
严格遵守，不得违反。
