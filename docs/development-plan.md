# Rust 浏览器引擎 — 开发计划表

## 一、项目总览

### 阶段总图

```
Phase 0 ────────── Phase 1 ─────────── Phase 2 ──────────── Phase 3 ─────────
最小渲染原型       核心引擎             功能完善              跨平台 + 完整
                   
2026 Q2-Q3         2026 Q3-Q4          2027 Q1-Q2           2027 Q3-Q4
2-3个月            4-6个月             6-9个月               6-12个月

DOM 核心           标准 HTML 渲染       CSS 完整               WASM 编译
CSS 30属性         CSS 80属性           JS Promise             iOS 原生
Flexbox + Block    html5ever            动画/过渡              Android 原生
wgpu 渲染          鼠标/键盘/滚动       Table / Float          Canvas 2D
click 事件          JS 编译器(基础)      fetch API              WebGL
                   文本布局完整          localStorage           Worker
                   图像渲染                                    性能优化
```

### 项目规模估算

| 指标 | Phase 0 | Phase 1 | Phase 2 | Phase 3 | 累计 |
|------|---------|---------|---------|---------|------|
| 代码量 | ~9,000 | ~17,000 | ~21,500 | ~14,000 | **~61,500** 行 |
| 工时 | ~3 人月 | ~6 人月 | ~9 人月 | ~12 人月 | **~30 人月** |
| 里程碑 | 5 个 | 6 个 | 6 个 | 5 个 | **22 个** |
| 依赖 crate | ~10 | ~15 | ~20 | ~25 | **~25** |

---

## 二、Phase 0 详细计划 — 最小渲染原型（2-3 个月）

### 目标
走通 HTML+CSS+JS 源文件 → 编译 → Rust 执行 → GPU 渲染的完整链路。包括 Rust 运行时 DOM → CSS → Layout → wgpu 渲染管线，以及 web2rust 编译器（Phase 0 简化版）。

### 里程碑

```
M0: 项目骨架搭建                                       第1周
  ↓
M1: DOM 核心 (Node/Element/Text/Document/Event)      第2-3周
  ↓
M2: CSS 引擎 (cssparser + 选择器 + 级联 + ~30属性)   第4-6周
  ↓
M3: 布局引擎 (taffy Flexbox + Block + 文本测量)       第7-9周
  ↓
M4: 渲染引擎 (winit + wgpu + DisplayList + 文本)     第10-12周
  ↓
M5: web2rust 编译器 + 集成 Demo                      第13-15周
```

### M0：项目骨架（第1周）

| 任务 | 工时 | 说明 |
|------|------|------|
| 创建 workspace Cargo.toml | 0.5d | crates.io 依赖配置 |
| 创建 crate 骨架 (dom/css/layout/paint/render/runtime) | 0.5d | 每个 crate 的 mod.rs |
| 定义 crate 间接口 (trait + struct) | 1d | 跨 crate 类型定义 |
| 验证依赖版本兼容性 | 1d | wgpu + winit + taffy + cssparser + selectors + rustybuzz |
| 配置 rustfmt + clippy + .gitignore | 0.5d | 代码规范 |

**关键交付**：`cargo build` 成功编译整个 workspace

**依赖版本锁定**：
```toml
wgpu = "23"          # 跨平台 GPU
winit = "0.30"       # 跨平台窗口
taffy = "0.5"        # Flexbox/Grid 布局
cssparser = "0.34"   # CSS 解析
selectors = "0.26"   # CSS 选择器
rustybuzz = "0.18"   # 字体 shaping
fontdb = "0.22"      # 字体管理
image = "0.25"       # 图像解码
```

### M1：DOM 核心（第2-3周）

| 任务 | 工时 | 产出 |
|------|------|------|
| NodeType + Node 定义 | 1d | 树节点类型 (Element/Text/Document/Comment/DocumentFragment) |
| 树指针 (parent/child/sibling) | 1d | parent 用 Weak，children 用 Vec<Rc<>> |
| appendChild / removeChild / insertBefore | 1d | 树操作 API |
| textContent / nodeType / nodeName | 0.5d | 属性访问器 |
| contains / cloneNode / isEqualNode | 0.5d | 辅助 API |
| Element 类型 (tag/attrs/classList/style/events) | 2d | Element 数据 + API |
| Document 类型 (createElement/querySelector/body) | 1d | Document API |
| DOMTokenList (classList) | 0.5d | add/remove/contains/toggle |
| CSSStyleDeclaration (style) | 1d | CSS 属性 getter/setter |
| 事件系统基础 (Event/EventTarget/MouseEvent) | 2d | addEventListener/dispatchEvent/冒泡 |
| 单元测试 | 1d | 基本功能覆盖 |

**关键设计决策**：
- 节点使用 `Rc<RefCell<Node>>` 引用计数
- parent 使用 `Weak` 避免循环引用
- 事件回调使用 `Box<dyn Fn(&Event)>` 或 `Rc<dyn Fn(&Event)>`
- 节点变更时通过回调通知 layout 引擎

### M2：CSS 引擎（第4-6周）

| 任务 | 工时 | 产出 |
|------|------|------|
| cssparser 集成：CSS 文本 → StyleSheet | 2d | 样式表解析 |
| selectors 集成：选择器 → 元素匹配 | 2d | 选择器匹配引擎 |
| CSS 属性定义 (properties.toml) | 2d | 30 个核心属性的元数据 |
| 代码生成：properties.toml → Rust struct | 1d | build.rs 代码生成 |
| 级联算法：特异性 + 来源权重 | 2d | cascade 模块 |
| 值计算：em→px, %→父容器 | 2d | computed values |
| 简写属性展开 (margin/padding/border) | 1d | shorthand 展开 |
| 继承属性处理 | 1d | inherited flag + 父传子 |
| 单元测试 | 1d | CSS 解析 + 级联测试 |

**Phase 0 CSS 属性清单**（30 个核心）：
```
盒模型(11): display, width, height, margin(all sides), padding(all sides),
            box-sizing, overflow
弹性布局(8): flex-direction, flex-wrap, justify-content, align-items,
             align-content, gap, flex-grow, flex-shrink
定位(4):     position, top, right, bottom, left
文字(7):     color, font-size, font-weight, font-family, line-height,
             text-align, white-space
背景(2):     background-color, background
其他(2):     opacity, border (简写)
```

**属性元数据格式 (properties.toml)**：
```toml
[display]
initial = "inline"
inherited = false
values = ["block", "inline", "flex", "inline-flex", "none", "grid", ...]

[color]
initial = "#000000"
inherited = true
type = "color"

[font-size]
initial = "16px"
inherited = true
type = "length"
```

### M3：布局引擎（第7-9周）

| 任务 | 工时 | 产出 |
|------|------|------|
| taffy 集成 + 基础布局树 | 2d | LayoutBox 类型 + taffy 调用 |
| Flexbox 布局 (通过 taffy) | 2d | display:flex 的完整支持 |
| Block 布局 (自研) | 3d | 从上到下布局 + margin collapse |
| Positioned 布局 (自研) | 1d | absolute/relative/fixed 定位 |
| 文本尺寸测量 (rustybuzz + fontdb) | 3d | 文本宽高计算 |
| LayoutBox → Rect 输出 | 1d | 每个节点的位置+尺寸 |
| LayoutTree 遍历 API | 1d | 子→父、兄弟遍历 |
| 脏标记 (dirty flag) 基础 | 1d | 节点变更标记 + 重排触发 |
| 单元测试 | 1d | 布局测试 |

**布局引擎核心逻辑**：
```rust
fn compute_layout(dom_root: &Node, viewport: Size) -> LayoutTree {
    // 1. 遍历 DOM，构建 LayoutBox 树
    let layout_root = build_layout_tree(dom_root);
    
    // 2. 第一 pass: 自底向上计算 intrinsic 尺寸
    compute_intrinsic_sizes(&layout_root);
    
    // 3. 第二 pass: 自顶向下分配最终位置
    assign_positions(&layout_root, viewport);
    
    // 4. 输出 LayoutTree (每个节点有绝对坐标)
    LayoutTree { root: layout_root }
}
```

**文本测量流程**：
```rust
fn measure_text(text: &str, font_size: f32, font_family: &str) -> Size {
    // 1. fontdb 选择字体
    let font = fontdb.query(&FontQuery { family: font_family, .. });
    
    // 2. rustybuzz shape
    let buffer = rustybuzz::shape(&font, text, ...);
    let glyphs: Vec<GlyphPosition> = buffer.glyph_positions();
    
    // 3. 计算总宽高
    let width: f32 = glyphs.iter().map(|g| g.x_advance).sum();
    let height: f32 = font_size; // 近似
    
    Size { width, height }
}
```

### M4：渲染引擎（第10-12周）

| 任务 | 工时 | 产出 |
|------|------|------|
| winit 窗口创建 | 1d | 窗口 + 事件循环 |
| wgpu 设备/队列/表面初始化 | 2d | GPU 上下文 (swapchain) |
| 着色器编写 (WGSL) | 1d | 顶点 + 片元着色器 |
| DisplayList 类型定义 | 1d | Paint 命令集合 |
| LayoutTree → DisplayList | 2d | 遍历布局树 → 绘制命令 |
| 矩形 + 背景色渲染 | 1d | wgpu draw rect |
| 边框渲染 | 1d | wgpu draw border |
| 文本渲染管线 | 3d | glyph 缓存 + 纹理渲染 |
| 帧循环 (requestAnimationFrame) | 1d | 持续渲染 |
| resize 处理 | 1d | 窗口大小 → 重新布局 |
| 单元测试 | 1d | DisplayList 构建测试 |

**渲染管线核心逻辑**：
```rust
fn render_frame(&mut self, layout_tree: &LayoutTree) {
    // 1. 构建 DisplayList
    let mut dl = DisplayList::new();
    build_display_list(&layout_tree.root, &mut dl);
    
    // 2. 按 z-order 排序
    dl.sort_by_z_order();
    
    // 3. 获取 swapchain 帧
    let frame = self.surface.get_current_texture()?;
    let view = frame.texture.create_view(&Default::default());
    
    // 4. 编码渲染命令
    let mut encoder = self.device.create_command_encoder(...);
    let mut pass = encoder.begin_render_pass(&view, ...);
    
    for cmd in dl.commands() {
        match cmd {
            PaintCommand::Rect { rect, color } => {
                // 绘制矩形
                self.draw_rect(&mut pass, rect, color);
            }
            PaintCommand::Text { text, font, pos, size, color } => {
                // 绘制文本
                self.draw_text(&mut pass, text, font, pos, size, color);
            }
            PaintCommand::Border { rect, widths, colors } => {
                // 绘制边框
                self.draw_border(&mut pass, rect, widths, colors);
            }
        }
    }
    
    // 5. 提交
    pass.end();
    self.queue.submit([encoder.finish()]);
    frame.present();
}
```

### M5：web2rust 编译器 + 集成 Demo（第13-15周）

| 任务 | 工时 | 产出 |
|------|------|------|
| web2rust HTML 解析器 | 2d | 手写标签解析 → 元素树 |
| web2rust CSS 解析器 | 1d | CSS 解析 + 元素匹配 |
| web2rust JS 模式识别 | 2d | querySelector/getElementById/addEventListener/textContent |
| 代码生成器 (lib.rs) | 2d | compile() + compile_body() 生成 main.rs |
| build.rs 集成 | 1d | 构建脚本编译源文件 |
| 计数器 Demo 源文件 | 1d | index.html + style.css + app.js |
| 调试 + Bug 修复 | 2d | 各模块联调 |
| 文档 | 1d | 项目说明 |

**Phase 0 编译器数据流**：
```text
index.html ──→ html.rs 解析器 ──→ 元素树 → 变量名分配 → create_element 代码
style.css  ──→ css.rs 解析器  ──→ 规则表 → 元素匹配 → set_style 代码
app.js     ──→ js.rs 模式识别  ──→ 事件处理器提取 → add_event_listener 闭包
                                        ⇓
                                counter_generated.rs → cargo build → 原生二进制
```

**Demo 源文件（index.html）**：
```html
<!DOCTYPE html>
<html>
<head><title>Counter App</title></head>
<body>
  <div class="container">
    <h1>Counter</h1>
    <div class="display">0</div>
    <button id="inc-btn">+</button>
  </div>
  <script src="app.js"></script>
</body>
</html>
```

**Demo 源文件（app.js）**：
```javascript
let count = 0;
const display = document.querySelector(".display");
const btn = document.getElementById("inc-btn");

btn.addEventListener("click", function() {
  count = count + 1;
  display.textContent = count;
});
```

**验证方式**：
```bash
RUST_LOG=wgpu=warn cargo run -p counter
# → build.rs 调用 web2rust 编译源文件
# → 弹出 800x600 窗口
# → 显示标题 "Counter"
# → 显示数字 (初值 0)
# → 点击按钮 → 数字 +1
```

### Phase 0 甘特图

```
任务                    W1  W2  W3  W4  W5  W6  W7  W8  W9  W10 W11 W12 W13 W14 W15
项目骨架                ██
DOM Node/Element        ██  ██
Document + API             ██  ██
事件系统基础                   ██
CSS 解析                            ██  ██
级联 + 属性计算                         ██  ██
Flexbox (taffy)                                    ██  ██
Block 布局                                              ██  ██
文本测量                                                    ██
winit + wgpu 初始化                                              ██  ██
DisplayList + 渲染                                                  ██  ██
文本渲染                                                                ██
web2rust 编译器                                                              ██  ██
集成 Demo                                                                           ██
文档                                                                                 ██
```

---

## 三、Phase 1 计划 — 核心引擎（4-6 个月）

### 目标
可渲染标准 HTML 页面，支持鼠标/键盘交互，JS→Rust 编译器可运行基础 JavaScript。

### 里程碑

```
M6: 标准 HTML 解析 (html5ever)        第1-3周
M7: CSS 扩展到 80 属性 + inline 布局  第4-6周
M8: 文本布局完整 + 图像渲染           第7-9周
M9: 事件系统完善 (键盘/鼠标/滚动)     第10-11周
M10: JS→Rust 编译器 (基础版)         第12-18周
M11: 集成 Demo (HTML 页面渲染)       第19-20周
```

### M10：JS→Rust 编译器核心

这是整个项目中最大的单个组件。分步子实现：

**Step 1：基础 JS 子集（4周）**
```
支持的 JS 输入：              → Rust 输出：
let x = 42;                  let x = 42;
function add(a, b) {         fn add(a: i32, b: i32) -> i32 {
  return a + b;                  a + b
}                            }
const obj = {               let obj = HashMap::from([
  name: "hello",               ("name", JsValue::String("hello")),
  count: 1                     ("count", JsValue::Number(1)),
];                           ]);
```

**Step 2：闭包 + DOM 操作（4周）**
```javascript
// JS 输入
document.createElement('div');
element.setAttribute('class', 'container');
element.addEventListener('click', function(e) {
    console.log('clicked');
});
```

```rust
// Rust 输出
Document::create_element("div");
element.set_attribute("class", "container");
element.add_event_listener("click", |e: &Event| {
    console_log("clicked");
});
```

**Step 3：SolidJS/React 模式支持（4周）**
- Signal → Rust Signal
- Effect → Rust Effect
- 条件渲染 (ternary) → Rust if
- 列表渲染 (map) → Rust iterator

### Phase 1 关键交付

```bash
# 1. 标准 HTML 渲染
cargo run -- example.html
# → 窗口显示 HTML 页面

# 2. JS 编译器
node-solid2rust build app.js -o generated.rs
```

---

## 四、Phase 2 计划 — 功能完善（6-9 个月）

### 目标
支持动态 Web 应用，CSS 完整，JS Promise/async，网络 API。

### 主要任务

| 模块 | 内容 | 工时 |
|------|------|------|
| CSS 动画/过渡 | 动画时间线 + 关键帧 + 插值器 | 6w |
| Grid 布局 | taffy Grid 集成 | 2w |
| Table 布局 | 自研 table 算法 | 4w |
| Float 布局 | 浮动布局 | 2w |
| JS Promise/async | Rust Future 映射 | 4w |
| JS Class 系统 | struct + trait 映射 | 3w |
| fetch API | HTTP 请求 + 响应处理 | 3w |
| localStorage | 持久化存储 | 2w |
| MutationObserver | DOM 变更观察 | 2w |
| CSS 媒体查询 | 响应式设计支持 | 3w |
| CSS 自定义属性 | CSS 变量 | 2w |
| 表单控件完整 | input/select/textarea 全部类型 | 4w |

---

## 五、Phase 3 计划 — 跨平台 + 完整（6-12 个月）

### 目标
覆盖 WASM/iOS/Android，实现 Canvas 2D/WebGL，性能优化。

### 主要任务

| 模块 | 内容 | 工时 |
|------|------|------|
| WASM 编译 | wgpu WASM + winit WASM 适配 | 6w |
| iOS 支持 | Metal 渲染 + 触控 + 手势 | 8w |
| Android 支持 | Vulkan 渲染 + 触控 + 键盘 | 8w |
| Canvas 2D API | 完整的 2D 图形上下文 | 12w |
| WebGL (基础) | OpenGL ES 子集 | 8w |
| Web Worker | 多线程 JS 编译 | 6w |
| 性能优化 | 增量重排 / 合批 / 纹理缓存 / layout 缓存 | 8w |
| 内存优化 | arena 分配器 / 节点池化 | 4w |

---

## 六、项目组织结构

### 6.1 仓库结构

```
rust-browser-engine/
├── Cargo.toml                    # workspace
├── docs/                         # 文档
│   ├── requirements.md
│   ├── feasibility.md
│   ├── development-plan.md
│   ├── architecture.md           # Phase 1 产出
│   └── api-reference.md          # Phase 1 产出
├── crates/
│   ├── dom/                      # W3C DOM 实现
│   ├── css/                      # CSS 引擎
│   ├── layout/                   # 布局引擎
│   ├── paint/                    # DisplayList
│   ├── render/                   # wgpu 渲染
│   ├── script/                   # JS→Rust 编译器
│   ├── net/                      # 网络层
│   ├── storage/                  # 存储层
│   └── runtime/                  # 整合
├── examples/                     # 示例
└── tests/                        # 集成测试
```

### 6.2 跨 crate 依赖关系

```
         ┌──────────────────────────────┐
         │  runtime                      │
         │  (入口 + 主循环 + 事件)        │
         └──┬──────┬──────┬──────┬──────┘
            │      │      │      │
    ┌───────┘      │      │      └───────┐
    ▼              ▼      ▼              ▼
┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
│ render │  │ script │  │  net   │  │storage │
│ (wgpu) │  │(JS编译)│  │(fetch) │  │(local) │
└───┬────┘  └────────┘  └────────┘  └────────┘
    │
    ▼
┌────────┐
│ paint  │
│(DL)    │
└───┬────┘
    │
    ▼
┌────────┐  ┌────────┐
│ layout │◄─│  css   │
│(taffy) │  │(解析+) │
└───┬────┘  │ 级联)  │
    │       └────────┘
    ▼
┌────────┐
│  dom   │
│(标准)  │
└────────┘
```

---

## 七、每个阶段的验收标准

### Phase 0 验收

- [ ] `cargo build` 编译通过，无 clippy 警告
- [ ] DOM API 单元测试通过（增删改查节点）
- [ ] CSS 引擎能解析样式表并计算级联值
- [ ] Flexbox 布局正确（taffy 集成验证）
- [ ] wgpu 弹出窗口并显示彩色矩形
- [ ] 文本渲染显示在窗口上
- [ ] 计数器 Demo 运行：点击按钮数字 +1
- [ ] 窗口 resize 触发布局重排

### Phase 1 验收

- [ ] html5ever 解析 HTML 为 DOM 树
- [ ] HTML 页面在窗口中渲染（含文本、图片）
- [ ] 鼠标移动/点击/键盘输入事件工作
- [ ] 页面内容可滚动
- [ ] JS→Rust 编译器可编译基础 JS（函数 + 对象 + DOM 操作）
- [ ] 简单的 HTML+CSS+JS 页面可完整运行

### Phase 2 验收（指标待细化）

- [ ] CSS 动画/过渡触发并正确插值
- [ ] Grid/Table/Float 布局正确
- [ ] Promise/async 编译运行正确
- [ ] fetch API 可发起 HTTP 请求
- [ ] TODO App 可完整运行

### Phase 3 验收（指标待细化）

- [ ] WASM 版本在浏览器中运行
- [ ] iOS 版本在真机上运行
- [ ] Android 版本在真机上运行
- [ ] Canvas 2D 基本图形渲染
- [ ] 页面渲染性能 > 30fps（标准页面）
