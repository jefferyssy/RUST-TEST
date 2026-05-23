# rust-browser-engine 项目目录树

## 目录结构

```
rust-test/
├── Cargo.toml                  # workspace 根配置（8 个 crate + 5 个 example）
│
├── crates/
│   ├── dom/                    # W3C DOM 标准实现
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs           # 模块导出 + Color/Rect/Size 辅助类型
│   │       ├── node.rs          # Node + 树操作（append/remove/insert/replace/clone/contains）
│   │       ├── element.rs       # ElementData（属性/classList/style/事件管理/样式解析）
│   │       ├── document.rs      # Document（createElement/createTextNode/body/querySelector）
│   │       ├── text.rs          # Text 文本节点
│   │       ├── event.rs         # Event/MouseEvent 事件系统
│   │       ├── dom_token_list.rs # DOMTokenList（classList 操作）
│   │       ├── mutation_observer.rs # MutationObserver（DOM 变更观察）
│   │       ├── html/
│   │       │   ├── mod.rs       # HTML 元素模块入口
│   │       │   ├── anchor.rs    # HTMLAnchorElement（<a> 链接）
│   │       │   ├── audio.rs     # HTMLAudioElement（<audio> 音频）
│   │       │   ├── canvas.rs    # HTMLCanvasElement（<canvas> 画布 + 2D 上下文）
│   │       │   ├── form.rs      # HTMLFormElement（<form> 表单）
│   │       │   ├── image.rs     # HTMLImageElement（<img> 图片）
│   │       │   ├── input.rs     # HTMLInputElement（<input> 输入框）
│   │       │   ├── link.rs      # HTMLLinkElement（<link> 外部资源）
│   │       │   ├── meta.rs      # HTMLMetaElement（<meta> 元数据）
│   │       │   ├── select.rs    # HTMLSelectElement（<select> 下拉）
│   │       │   ├── text_area.rs # HTMLTextAreaElement（<textarea> 多行输入）
│   │       │   └── video.rs     # HTMLVideoElement（<video> 视频）
│   │       └── observer/
│   │           ├── mod.rs       # 观察者模块入口
│   │           ├── resize_observer.rs      # ResizeObserver（元素尺寸变化观察）
│   │           └── intersection_observer.rs # IntersectionObserver（可见性交叉观察）
│   │
│   ├── style/                   # 样式系统（CSS 引擎 + 动画 + 过渡）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs           # 模块导出
│   │       ├── stylesheet.rs    # StyleSheet/Rule/Declaration/Keyframes/MediaQuery
│   │       ├── selector.rs      # 复合选择器 + 特异性计算 + match_selectors
│   │       ├── cascade.rs       # ComputedStyle + compute_element_style + 继承
│   │       ├── values.rs        # CSSValue/CSSUnit/颜色/长度/函数解析
│   │       ├── properties.rs    # Phase 3 新增属性解析（aspect-ratio/transform-style 等）
│   │       ├── media.rs         # 媒体查询求值引擎（@media 规则条件评估）
│   │       ├── animations.rs    # CSS 动画引擎（@keyframes 插值/时间线管理）
│   │       ├── transitions.rs   # CSS 过渡引擎（属性变更平滑过渡/缓动函数）
│   │       └── custom_props.rs  # CSS 自定义属性（--var 变量系统）
│   │
│   ├── layout/                  # 布局引擎
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs           # LayoutEngine + build_layout_tree（DOM→Layout 树构建）
│   │       ├── layout_box.rs    # LayoutBox/BoxType/EdgeSizes + 遍历方法
│   │       ├── flex.rs          # Flexbox 布局（taffy TaffyTree 集成）
│   │       ├── block.rs         # Block 块级垂直堆叠布局
│   │       ├── inline.rs        # Inline 行内元素水平排列布局
│   │       ├── positioned.rs    # relative/absolute/fixed/sticky 定位
│   │       ├── grid.rs          # Grid 网格布局（grid-template-columns/rows）
│   │       ├── float.rs         # Float 浮动布局（left/right + clear 清除）
│   │       ├── table.rs         # Table 表格布局（TableRow/TableCell + colspan/rowspan）
│   │       └── text.rs          # TextMeasurer 文本测量（字符宽度/换行计算）
│   │
│   ├── render_tree/             # 渲染树（DisplayList 构建 + 批处理优化）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs           # 模块导出
│   │       ├── command.rs       # PaintCommand（FillRect/Text/Border/Image）+ DisplayList
│   │       ├── builder.rs       # DisplayListBuilder（LayoutBox→PaintCommand 转换）
│   │       └── optimizer.rs     # BatchOptimizer（合批 + 遮挡剔除优化）
│   │
│   ├── renderer/                # 渲染后端 + 运行时整合
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs           # RenderBackend trait + 模块导出
│   │       ├── wgpu_backend.rs  # wgpu 桌面端渲染（WGSL 着色器 + 矩形/边框/圆角绘制）
│   │       ├── text_renderer.rs # 文本渲染（rustybuzz 字形排版 + fontdb 字体加载）
│   │       ├── window.rs        # WebWindow 应用主入口（DOM→CSS→Layout→DisplayList→GPU 全管线）
│   │       ├── window_trait.rs  # Window trait + ViewportInfo（窗口抽象接口）
│   │       ├── event_loop.rs    # AnimationFrameScheduler（requestAnimationFrame 调度）
│   │       ├── hit_test.rs      # HitTester（坐标命中检测 + 事件冒泡路径）
│   │       └── observer_manager.rs # ObserverManager（Resize/Intersection/Mutation 观察者调度）
│   │
│   ├── toolchain/               # 命令工具（HTML+CSS+JS → Rust 代码编译器）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs           # compile()/compile_body() 入口 + 代码生成
│   │       ├── html.rs          # HTML 标签解析器 → 元素树
│   │       ├── css.rs           # CSS 规则解析 + 元素匹配（tag/.class/#id）
│   │       ├── js.rs            # JS 模式识别 → 事件处理器/变量/表达式编译
│   │       ├── parser.rs        # 统一解析调度器（CompilerUnit 中间表示）
│   │       ├── analyzer.rs      # JS 语义分析器（变量/函数调用/DOM 操作检测）
│   │       ├── codegen.rs       # Rust 代码生成器（DOM 构建 + 样式 + 事件代码）
│   │       ├── builtins.rs      # 内置 API 映射（console/Math/DOM/URL/Date/History 等）
│   │       └── canvas_codegen.rs # Canvas 2D API 编译映射
│   │
│   ├── net/                     # 网络层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs           # 模块导出
│   │       ├── fetch.rs         # fetch API（HTTP 请求/响应处理）
│   │       └── websocket.rs     # WebSocket（连接/收发/重连/心跳）
│   │
│   └── storage/                 # 存储层
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs           # 模块导出 + Storage trait
│           └── local_storage.rs # localStorage/sessionStorage 持久化存储
│
├── examples/
│   ├── counter/                 # 计数器 Demo — 源文件驱动模式
│   │   ├── Cargo.toml
│   │   ├── build.rs             # 构建时调用 toolchain 编译源文件
│   │   ├── .gitignore           # 忽略 src/（自动生成）
│   │   ├── index.html           # 源文件：HTML 结构
│   │   ├── style.css            # 源文件：CSS 样式
│   │   ├── app.js               # 源文件：JS 交互逻辑
│   │   └── src/
│   │       └── main.rs          # 入口：include! 引入生成代码
│   ├── two-counters/            # 双计数器 Demo — 多个独立状态
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── index.html
│   │   ├── style.css
│   │   ├── app.js
│   │   └── src/
│   │       └── main.rs
│   ├── flex-nav/                # Flex 导航栏 Demo — flexbox 布局验证
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── index.html
│   │   ├── style.css
│   │   ├── app.js
│   │   └── src/
│   │       └── main.rs
│   ├── dashboard/               # 仪表盘 Demo — 复杂布局
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── index.html
│   │   ├── style.css
│   │   ├── app.js
│   │   └── src/
│   │       └── main.rs
│   └── todo_app/                # 待办事项 Demo — 完整 CRUD 交互
│       ├── Cargo.toml
│       ├── build.rs
│       ├── index.html
│       ├── style.css
│       ├── app.js
│       └── src/
│           └── main.rs
│
└── docs/                        # 设计文档
    ├── requirements.md          # 需求文档
    ├── feasibility.md           # 可行性报告
    ├── development-plan.md      # 开发计划表
    ├── project-tree.md          # 本文档 — 目录结构与文件功能说明
    ├── phase/
    │   ├── api-design-phase0.md # Phase 0 API 详细设计
    │   ├── api-design-phase1.md # Phase 1 API 设计
    │   ├── api-design-phase2.md # Phase 2 API 设计
    │   └── api-design-phase3.md # Phase 3 API 设计
    └── coverage/
        ├── html-api-coverage.md # HTML API 覆盖率
        ├── css-api-coverage.md  # CSS API 覆盖率
        └── js-api-coverage.md   # JS API 覆盖率
```

---

## 渲染管线映射

```
HTML+CSS+JS 源文件
      │
      ▼
[toolchain] 命令工具 — 编译为 Rust 代码（WebWindow + DOM API 调用）
      │
      ▼
[dom] DOM 树构建 + 事件系统 + Mutation/Resize/Intersection 观察者
      │
      ▼
[style] CSS 解析 + 选择器匹配 + 级联计算 + 动画/过渡/媒体查询/自定义属性
      │
      ▼
[layout] 布局计算 — Flex/Block/Inline/Grid/Table/Positioned/Float
      │
      ▼
[render_tree] 渲染树生成 — DisplayList 构建 + 批处理/遮挡剔除优化
      │
      ▼
[renderer] 最终渲染 — wgpu GPU 后端 + winit 窗口 + 事件循环 + HitTest + 观察者调度
      │
      ▼
[屏幕]
```

---

## Crate 职责一览

| Crate | 管线阶段 | 核心功能 |
|-------|---------|---------|
| `toolchain` | 命令工具 | HTML+CSS+JS → Rust 编译器，含解析/分析/代码生成/内置 API 映射 |
| `dom` | 解析 | W3C DOM 标准实现（Node/Element/Document/Event/HTML 元素/Observer） |
| `style` | 样式系统 | CSS 引擎（选择器/级联/值类型/动画/过渡/媒体查询/自定义属性） |
| `layout` | 布局 | 7 种布局模式（Flex/Block/Inline/Grid/Table/Positioned/Float） |
| `render_tree` | 渲染树 | DisplayList 构建 + 批处理优化器（合批/遮挡剔除） |
| `renderer` | 最终渲染 | wgpu 渲染 + winit 窗口 + 事件循环 + HitTest + ObserverManager |
| `net` | 网络 | fetch API + WebSocket（含重连/心跳） |
| `storage` | 存储 | localStorage / sessionStorage |

---

## 模块依赖关系

```
examples     → renderer + toolchain (build-dep)
renderer     → style + layout + render_tree + dom
render_tree  → dom + style + layout
layout       → dom + style + taffy + rustybuzz + fontdb
style        → dom + cssparser + selectors
dom          → （无外部依赖）
toolchain    → （无外部依赖，纯 Rust 标准库 + 文件系统）
net          → dom
storage      → dom
```

---

## Crate 文件功能详表

### dom/ — W3C DOM 标准实现（17 个源文件 + 3 个测试文件 + 1 个目录）

| 文件 | 功能 |
|------|------|
| `lib.rs` | Color/Rect/Size 类型定义 + 模块导出 |
| `node.rs` | Node 核心：树操作（appendChild/removeChild/insertBefore/replaceChild/cloneNode/contains） |
| `element.rs` | ElementData：属性管理（getAttribute/setAttribute/removeAttribute）、classList 操作、事件委托、内联样式解析 |
| `document.rs` | Document：createElement/createTextNode/getElementById/querySelector/body/getElementsByClassName |
| `text.rs` | Text 文本节点（字符数据 + 分割方法） |
| `event.rs` | Event/MouseEvent/KeyboardEvent/FocusEvent/InputEvent + 事件监听器管理 + 冒泡/捕获 |
| `dom_token_list.rs` | DOMTokenList（classList.add/remove/toggle/contains/replace） |
| `mutation_observer.rs` | MutationObserver：监听 DOM 属性/子节点/字符数据变更 |
| `html/mod.rs` | HTML 元素模块入口 + HTMLAnchorElement/HTMLImageElement 等接口定义 |
| `html/anchor.rs` | `<a>` 链接元素（href/target/download/rel） |
| `html/audio.rs` | `<audio>` 音频元素（src/controls/autoplay/loop/muted/volume/playbackRate） |
| `html/canvas.rs` | `<canvas>` 画布元素 + CanvasRenderingContext2D（2D 绘制 API） |
| `html/form.rs` | `<form>` 表单元素（action/method/enctype + submit/reset） |
| `html/image.rs` | `<img>` 图片元素（src/alt/width/height/loading/decoding） |
| `html/input.rs` | `<input>` 输入元素（type/value/placeholder/checked/required/disabled） |
| `html/link.rs` | `<link>` 外部资源元素（rel/href/type/media） |
| `html/meta.rs` | `<meta>` 元数据元素（name/content/charset/http-equiv） |
| `html/select.rs` | `<select>` 下拉选择 + `<option>` 选项元素 |
| `html/text_area.rs` | `<textarea>` 多行文本输入元素 |
| `html/video.rs` | `<video>` 视频元素（src/controls/autoplay/loop/muted/volume/poster） |
| `observer/mod.rs` | 观察者模块入口 |
| `observer/resize_observer.rs` | ResizeObserver：监听元素尺寸变化 |
| `observer/intersection_observer.rs` | IntersectionObserver：监听元素可见性交叉 |

### style/ — CSS 引擎 + 动画系统（9 个源文件 + 3 个测试文件）

| 文件 | 功能 |
|------|------|
| `lib.rs` | 模块导出 |
| `stylesheet.rs` | StyleSheet/StyleRule/Declaration/KeyframesRule/MediaQuery/FontFaceRule + CSS 文本解析 |
| `selector.rs` | 复合选择器 + 特异性计算 (id, class, tag) + match_selectors + 伪类 |
| `cascade.rs` | ComputedStyle 类型 + compute_element_style（特异性排序 + !important + 继承） |
| `values.rs` | CSSValue 枚举（Keyword/Length/Color/Percentage/Number/Composite）+ 颜色/长度/函数解析 |
| `properties.rs` | Phase 3 新增属性解析器（aspect-ratio/contain/word-break/transform-style/perspective/touch-action 等 14 个） |
| `media.rs` | 媒体查询求值（MediaEvaluator + ViewportInfo，评估 @media 规则条件） |
| `animations.rs` | CSS 动画引擎（AnimationEngine + AnimationState + @keyframes 插值 + 时间线管理） |
| `transitions.rs` | CSS 过渡引擎（TransitionEngine + 缓动函数 + 属性变更平滑过渡） |
| `custom_props.rs` | CSS 自定义属性变量（`--var` 声明与替换） |

### layout/ — 布局引擎（9 个源文件 + 3 个测试文件）

| 文件 | 功能 |
|------|------|
| `lib.rs` | LayoutEngine + build_layout_tree（DOM 树 → 布局树转换 + 布局调度） |
| `layout_box.rs` | LayoutBox 核心类型（BoxType/EdgeSizes/Overflow/BorderRadius/Visibility）+ 遍历方法 |
| `flex.rs` | Flexbox 布局（taffy 集成：flex-direction/justify-content/align-items/gap/flex-grow 等） |
| `block.rs` | Block 垂直堆叠布局（margin 合并 + 高度自动计算） |
| `inline.rs` | Inline 行内布局（水平排列 + 自动换行 + line-height） |
| `positioned.rs` | 定位布局（relative/absolute/fixed/sticky + z-index 层级） |
| `grid.rs` | Grid 网格布局（grid-template-columns/rows 解析 + fr/px/auto 轨道 + GridItem 排列） |
| `float.rs` | Float 浮动布局（FloatDirection/ClearMode + 文字环绕） |
| `table.rs` | Table 表格布局（TableRow/TableCell + colspan/rowspan + border-spacing） |
| `text.rs` | TextMeasurer：字符宽度估算 + 文本换行计算 |

### render_tree/ — 渲染树（3 个源文件 + 2 个测试文件）

| 文件 | 功能 |
|------|------|
| `lib.rs` | 模块导出 |
| `command.rs` | PaintCommand 枚举（FillRect/Border/Text/Image/Clip）+ DisplayList 容器 |
| `builder.rs` | DisplayListBuilder：LayoutBox 树 → PaintCommand 列表（背景/边框/文本绘制命令） |
| `optimizer.rs` | BatchOptimizer：合批（同色矩形合并）+ 遮挡剔除（不透明区域覆盖移除） |

### renderer/ — 最终渲染 + 运行时整合（8 个源文件 + 1 个测试文件）

| 文件 | 功能 |
|------|------|
| `lib.rs` | RenderBackend trait（render/resize/present/size）+ 模块导出 |
| `wgpu_backend.rs` | wgpu 桌面端 GPU 渲染（WGSL 着色器 + 矩形/边框/圆角/图片绘制 + TextureAtlas） |
| `text_renderer.rs` | 文本渲染（rustybuzz 字形排版 + fontdb 字体加载 + 缓存） |
| `window.rs` | WebWindow 应用主入口（create_window + DOM→CSS→Layout→DisplayList→GPU 全管线整合 + 事件处理） |
| `window_trait.rs` | Window trait 窗口抽象 + ViewportInfo（尺寸/设备像素比/方向/配色方案） |
| `event_loop.rs` | AnimationFrameScheduler（requestAnimationFrame/cancelAnimationFrame/callback 调度） |
| `hit_test.rs` | HitTester（坐标命中检测 + 事件冒泡路径收集 + 可交互路径构建） |
| `observer_manager.rs` | ObserverManager（ResizeObserver/IntersectionObserver/MutationObserver 统一调度） |

### toolchain/ — 命令工具编译器（8 个源文件 + 4 个测试文件 + 1 个目录）

| 文件 | 功能 |
|------|------|
| `lib.rs` | compile()/compile_body() 顶层入口 + 代码生成逻辑（元素构建/样式应用/事件处理器/窗口启动） |
| `html.rs` | HTML 标签解析器 → HtmlElement 树（自闭合/忽略标签/嵌套结构） |
| `css.rs` | CSS 规则解析 → CssRule 列表 + 选择器匹配元素 |
| `js.rs` | JS 模式识别 → EventHandler 提取（querySelector/addEventListener/变量赋值/表达式编译） |
| `parser.rs` | 统一解析调度器（Parser + CompilationUnit 中间表示 + SourceFile） |
| `analyzer.rs` | JS 语义分析器（变量声明/函数调用/DOM 操作检测） |
| `codegen.rs` | CodeGenerator Rust 代码生成（generate_main_body/generate_element_code） |
| `builtins.rs` | 内置 API 映射（console/Math/DOM/TypedArray/Object/History/Location/URL/Date） |
| `canvas_codegen.rs` | Canvas 2D API 编译映射（CanvasRenderingContext2D 方法 → Rust 绘制调用） |

### net/ — 网络层（2 个源文件）

| 文件 | 功能 |
|------|------|
| `lib.rs` | 模块导出 |
| `fetch.rs` | fetch() API（HTTP GET/POST/PUT/DELETE + Headers + Response + 超时处理） |
| `websocket.rs` | WebSocket（连接/发送/接收 + 自动重连 + 心跳保活） |

### storage/ — 存储层（2 个源文件）

| 文件 | 功能 |
|------|------|
| `lib.rs` | 模块导出 + Storage trait |
| `local_storage.rs` | localStorage / sessionStorage（getItem/setItem/removeItem/clear + 内存存储） |

---

## 测试文件清单

| Crate | 测试文件 | 测试内容 |
|-------|---------|---------|
| dom | `node.test.rs` | Node 树操作 |
| dom | `element.test.rs` | Element 属性/classList/style |
| dom | `document.test.rs` | Document 工厂方法 |
| dom | `text.test.rs` | Text 节点 |
| dom | `event.test.rs` | Event 系统 |
| dom | `dom_token_list.test.rs` | classList 操作 |
| dom | `lib.test.rs` | 辅助类型 |
| style | `values.test.rs` | CSS 值解析 |
| style | `selector.test.rs` | 选择器匹配 |
| style | `stylesheet.test.rs` | 样式表解析 |
| style | `cascade.test.rs` | 级联计算 |
| layout | `layout_box.test.rs` | 布局框测试 |
| layout | `block.test.rs` | Block 布局 |
| layout | `text.test.rs` | 文本测量 |
| layout | `positioned.test.rs` | 定位布局 |
| render_tree | `command.test.rs` | 绘制命令 |
| render_tree | `builder.test.rs` | DisplayList 构建 |
| renderer | `lib.test.rs` | 渲染后端测试 |
| toolchain | `lib.test.rs` | 代码生成 |
| toolchain | `html.test.rs` | HTML 解析 |
| toolchain | `css.test.rs` | CSS 解析 |
| toolchain | `js.test.rs` | JS 编译 |

**内联测试模块（文件内 `#[cfg(test)]`）:**

| Crate | 文件 | 测试数 |
|-------|------|--------|
| style | `animations.rs` | 9 个（引擎/插值/迭代/取消） |
| style | `transitions.rs` | 14 个（缓动/过渡/延迟/完成/插值） |
| style | `media.rs` | 4 个（视口/媒体查询匹配） |
| style | `properties.rs` | 4 个（属性解析） |
| toolchain | `builtins.rs` | 17 个（API 映射查找） |
| toolchain | `analyzer.rs` | 16 个（语义分析/字符串提取） |
| render_tree | `optimizer.rs` | 6 个（合批/遮挡剔除） |
| renderer | `hit_test.rs` | 10 个（命中检测/矩形包含/冒泡路径） |
| renderer | `event_loop.rs` | 8 个（动画帧调度） |
| net | `fetch.rs` | 内联测试 |
| net | `websocket.rs` | 内联测试 |
| storage | `local_storage.rs` | 内联测试 |

---

## 统计

| Crate | 源文件 | 测试文件 | 总测试数（约） | 当前阶段 |
|-------|--------|---------|--------------|---------|
| dom | 23 | 7 | 56 | Phase 2 完成，Phase 3 8 项待实现 |
| style | 9 | 4 | 73 | Phase 2 完成，Phase 3 47 项待实现 |
| layout | 9 | 4 | 23 | Phase 2 — 7 种布局模式 |
| render_tree | 3 | 2 | 16 | Phase 1 — 合批优化 |
| renderer | 8 | 1 | 25 | Phase 1 — HitTest/事件循环/观察者管理 |
| toolchain | 8 | 4 | 60 | Phase 2 完成，Phase 3 116 项待实现 |
| net | 2 | 0（内联） | 2 | Phase 2 完成，Phase 3 3 项待实现 |
| storage | 2 | 0（内联） | 3 | Phase 2 完成 |
| **合计** | **64** | **22（+ 内联）** | **258+** | **所有模块可工作** |

---

## API 覆盖率总览

> 详细分析见 `docs/consolidated-design.md`

| 维度 | 目标 API | ✅ 已实现 | 🔲 待实现 | 覆盖率 |
|------|----------|-----------|-----------|--------|
| HTML / DOM | 151 | 115 | 36 | 76% |
| CSS | 211 | 171 | 47 | 81% |
| JS / Web API | 258 | 142 | 116 | 55% |
| **合计** | **620** | **428** | **199** | **69%** |

### Phase 3 待完成核心项 (P0)

1. **CSS 选择器**: 属性选择器 (6) + 组合器 (4) + `:has()`/`:is()`/`:where()` + `::before`/`::after`
2. **JS String**: `replace`/`replaceAll`/`toUpperCase`/`toLowerCase`/`indexOf`
3. **JS Array**: `sort`/`reverse`/`forEach`/`some`/`every`/`concat`/`findIndex`
4. **JS RegExp**: 正则表达式基础 (`/pattern/flags`, `.test()`, `.exec()`)
5. **JS Date**: Date 对象全套 (构造/格式化/计算)
6. **JS Object**: `create`/`defineProperty`/`freeze`/`seal`/`is`/`hasOwn`/`fromEntries`
7. **SPA 路由**: History API (9) + Location API (12) + URL/SearchParams (12)
8. **Canvas 2D**: 核心绘制 API (18 项)
