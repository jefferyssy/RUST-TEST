# Rust 浏览器引擎 — 需求架构文档

## 一、项目概述

### 1.1 项目定位
用 Rust 构建一套符合 W3C 标准的浏览器渲染引擎，支持 HTML + CSS + JavaScript 的标准 Web 内容渲染。通过"前端源码 → Rust 原生执行"的编译路径，实现跨平台应用交付。

### 1.2 核心特征

| 属性 | 方案 |
|------|------|
| 渲染引擎 | **自研 Rust**（DOM → CSS → Layout → Paint → GPU） |
| JS 执行 | **编译时转换**（JS → Rust 编译器），无嵌入式 JS 引擎 |
| API 标准 | **W3C 规范全量**（分期实现） |
| 跨平台 | **winit + wgpu**（PC / Web WASM / iOS / Android）|
| 输入 | **标准 Web 内容**（.html / .css / .js 文件）|

### 1.3 核心理念

本项目不是嵌入式 WebView，也不是又一个前端框架。它是一套**从零实现的浏览器渲染管线**，核心创新在于：

1. **JS 编译而非解释**：将 JavaScript 代码在构建期编译为 Rust 代码，无运行时 JS 引擎
2. **标准 API 全量覆盖**：API 设计遵循 W3C 规范，分期实现但不做专属扩展
3. **原生性能**：全程 Rust 原生执行，无桥接开销，无 GC 暂停
4. **输入层浏览器兼容**：同一份 HTML+CSS+JS 源码可直接在标准浏览器中打开预览，也可在我们的引擎中渲染，开发体验无缝衔接

### 1.4 目标运行环境

```
┌─────────────────────────────────────────────────────────────────────┐
│  同一份源码 (.html / .css / .js)                                     │
│                                                                     │
│  ├──→ 标准浏览器 (Chrome/Safari/Firefox)  — 直接用浏览器打开          │
│  │      开发阶段预览、调试                                            │
│  │                                                                  │
│  ├──→ Rust 原生引擎 (winit + wgpu) — PC 桌面端                       │
│  │      Windows / macOS / Linux，最大性能                            │
│  │                                                                  │
│  ├──→ WASM + WebGPU — Web 端                                         │
│  │      引擎编译为 WASM，在浏览器中用 WebGPU 绘制                      │
│  │      目标：常规 Web 站点、在线演示                                  │
│  │                                                                  │
│  └──→ WASM + WebGPU — 小程序端                                       │
│        引擎编译为 WASM，在小程序 WebView 中通过 WebGPU 绘制            │
│        目标：微信小程序 / 支付宝小程序等                                │
│        优势：突破小程序 DSL 限制，渲染标准 Web 内容                    │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 二、整体架构

### 2.1 系统层级

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         输入层（直接给浏览器使用）                                │
│                                                                              │
│     .html 文件               .css 文件               .js 文件                  │
│     (W3C HTML5)             (CSS 标准)             (ECMAScript)               │
│                                                                              │
│     ┌── 标准浏览器直接打开 ──→ 开发预览 / 调试                                  │
│     │                                                                        │
└─────┼────────────────────────────────────────────────────────────────────────┘
      │                         │                         │
      ▼                         ▼                         ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                          编译时 (Build Time)                                    │
│                                                                              │
│  ┌──────────────────────┐    ┌──────────────────────────────────────────────┐ │
│  │ HTML 解析器           │    │ JS→Rust 编译器                               │ │
│  │ (html5ever)          │    │ (基于 swc)                                   │ │
│  │ DOM 树输出            │    │ JS AST → Rust AST                            │ │
│  └──────────┬───────────┘    │ ECMAScript 特性映射                           │ │
│             │                │ 含所有前端框架编译产物                          │ │
│             │                └──────────────────┬───────────────────────────┘ │
│             ▼                                   ▼                            │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                      Rust 代码生成器                                     │ │
│  │  ├─ DOM 构建代码 (createElement, appendChild 等)                        │ │
│  │  ├─ 样式代码 (class → style 映射)                                       │ │
│  │  ├─ 事件绑定代码 (addEventListener)                                      │ │
│  │  └─ 业务逻辑代码 (Signal, Effect, 请求等)                                │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                        cargo build (rustc)                              │ │
│  │   编译目标: x86_64-pc-windows-msvc / aarch64-apple-darwin / wasm32-... │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────────┐
                    ▼               ▼                    ▼
┌─────────────────────────┐ ┌─────────────────┐ ┌──────────────────────────┐
│ 原生运行时 (Native)      │ │ WASM 运行时      │ │ WASM 运行时 (小程序)     │
│                         │ │ (Web 端)        │ │                          │
│ winit 窗口              │ │ 浏览器环境       │ │ 小程序 WebView            │
│ wgpu (DX12/Vulkan/Metal)│ │ WebGPU API       │ │ WebGPU API               │
│ PC / iOS / Android      │ │ 在线应用 / 演示   │ │ 微信/支付宝/抖音小程序    │
└──────────┬──────────────┘ └────────┬────────┘ └────────────┬─────────────┘
           │                        │                        │
           └────────────┬───────────┴────────────┬───────────┘
                        ▼                        ▼
              ┌────────────────────────────────────────────────────────┐
              │                    核心运行时 (共享)                      │
              │                                                        │
              │  ┌──────────────────────────────────────────────────┐  │
              │  │  DOM 实现层 (W3C DOM 标准)                        │  │
              │  │  ├─ Core: Node, Element, Text, Document...       │  │
              │  │  ├─ Events: Event, EventTarget, MouseEvent...    │  │
              │  │  ├─ CSSOM: CSSStyleDeclaration, StyleSheet...    │  │
              │  │  └─ HTML: HTMLDivElement, HTMLInputElement...     │  │
              │  └──────────────────┬───────────────────────────────┘  │
              │                     │ DOM 变更通知                      │
              │  ┌──────────────────▼───────────────────────────────┐  │
              │  │  CSS 引擎 (W3C CSS 标准)                          │  │
              │  │  ├─ 解析: cssparser → StyleSheet                 │  │
              │  │  ├─ 选择器: selectors crate → 匹配                │  │
              │  │  ├─ 级联: 特异性 + 来源权重 + !important          │  │
              │  │  ├─ 计算: 相对值 → 绝对值                         │  │
              │  │  ├─ 继承: inherited properties                    │  │
              │  │  └─ 动画: CSS Animations + Transitions           │  │
              │  └──────────────────┬───────────────────────────────┘  │
              │                     │ 每个节点: computed style          │
              │  ┌──────────────────▼───────────────────────────────┐  │
              │  │  布局引擎 (Layout)  ←── 独立模块                    │  │
              │  │  ├─ Normal Flow: Block / Inline / Relative        │  │
              │  │  ├─ Flexbox: W3C Flexbox 规范                     │  │
              │  │  ├─ Grid: W3C Grid 规范                           │  │
              │  │  ├─ Tables: W3C Table 规范                        │  │
              │  │  ├─ Positioned: Absolute / Fixed / Sticky         │  │
              │  │  ├─ Float: 浮动布局                                │  │
              │  │  └─ Text: 文本布局 (rustybuzz + fontdb)           │  │
              │  └──────────────────┬───────────────────────────────┘  │
              │                     │ LayoutTree (每个节点有 Rect)       │
              │  ┌──────────────────▼───────────────────────────────┐  │
              │  │  DisplayList (paint 模块)  ←── 独立模块            │  │
              │  │  ├─ PaintRect / PaintText / PaintBorder...        │  │
              │  │  └─ 按 z-order 排序 + 合批                         │  │
              │  └──────────────────┬───────────────────────────────┘  │
              │                     │ DisplayList (绘制命令数组)        │
              │  ┌──────────────────▼───────────────────────────────┐  │
              │  │  渲染后端 (Render Backend)  ←── 可替换              │  │
              │  │                                                   │  │
              │  │  ┌──────────────┐  ┌──────────────────────────┐   │  │
              │  │  │ wgpu 后端     │  │ WebGPU WASM 后端          │   │  │
              │  │  │ DX12/Vulkan  │  │ 浏览器 / 小程序 WebView   │   │  │
              │  │  │ Metal        │  │ 通过 web-sys 调用 WebGPU  │   │  │
              │  │  │ PC/iOS/Android│  │                          │   │  │
              │  │  └──────────────┘  └──────────────────────────┘   │  │
              │  └──────────────────┬───────────────────────────────┘  │
              │                     │ GPU 渲染 → 屏幕                   │
              │  ┌──────────────────▼───────────────────────────────┐  │
              │  │  交互系统                                          │  │
              │  │  ├─ 输入: 鼠标/键盘/触摸/滚轮                       │  │
              │  │  ├─ 命中检测: 坐标 → DOM 节点                      │  │
              │  │  ├─ 事件分发: 捕获 → 目标 → 冒泡                   │  │
              │  │  └─ 焦点管理                                      │  │
              │  └──────────────────────────────────────────────────┘  │
              └────────────────────────────────────────────────────────┘
```

### 2.2 数据流（完整链路 + 多输出路径）

```
.html 文件  .css 文件  .js 文件
    │         │        │
    │         │        ├──→ 标准浏览器直接打开（开发预览）
    │         │        │
    ▼         ▼        ▼
html5ever  cssparser  swc (JS→Rust 编译器)
    │         │        │
    ▼         ▼        ▼
  DOM 树  StyleSheet  Rust 代码（DOM API + 逻辑）
    │         │        │
    └────┬────┘        │
         │              │
         ▼              │
   CSS 选择器匹配        │
   + 级联计算            │
         │              │
         ▼              │
   Styled Tree          │
         │              │
         ▼              │
┌───────┴───────────────┴────┐
│  布局引擎 (layout crate)    │  ←── 独立模块
│  taffy + 自研              │
└───────┬────────────────────┘
        │ Layout Tree
        ▼
┌───────┴────────────────────┐
│  DisplayList (paint crate) │  ←── 独立模块
│  绘制命令集合               │
└───────┬────────────────────┘
        │ DisplayList
        ▼
┌───────────────────────────┐
│ 渲染后端 (可替换)          │
├───────────────────────────┤
│  ┌──→ wgpu 后端            │ ──→ GPU → 屏幕 (PC/iOS/Android)
│  │    wgpu crate           │
│  │    (DX12/Vulkan/Metal)  │
│  │                         │
│  └──→ WebGPU WASM 后端     │ ──→ 浏览器 Canvas / 小程序
│       web-sys WebGPU       │
│                            │
└───────────────────────────┘
        │
        ▼
   事件循环 ←────────── JS 回调
   (鼠标/键盘/触摸 → DOM 事件)

---

## 三、完整的浏览器标准覆盖范围

### 3.1 HTML 标准

| 模块 | 标准 | 实现策略 |
|------|------|----------|
| HTML 解析 | W3C HTML5 / WHATWG | html5ever crate（完整支持） |
| HTML 元素 | 全部 ~120 个标签 | 按优先级分期实现 |
| DOM 树构建 | W3C DOM Parsing | html5ever 产出 DOM 树 |

**HTML 元素优先级**：

| 层级 | 标签 | 数量 |
|------|------|------|
| P0 (核心) | div, span, p, h1-h6, a, img, button, input, textarea, form, ul, ol, li, table, header, footer, section, nav, main, aside, br, hr, strong, em, code, pre, blockquote | ~30 |
| P1 (常用) | select, option, label, fieldset, legend, details, summary, figure, figcaption, article, time, mark, sub, sup, video, audio (无解码) | ~20 |
| P2 (扩展) | canvas, svg, iframe, script (编译用), style, link, meta, title, base | ~15 |
| P3 (完整) | 剩余所有 HTML 元素 | ~55 |

### 3.2 CSS 标准

| CSS 模块 | 标准 | 一期 | 二期 | 三期 |
|----------|------|------|------|------|
| CSS 2.1 盒模型 | ✅ | ✅ | ✅ | ✅ |
| CSS Display | ✅ | ✅ | ✅ | ✅ |
| CSS 定位 | ✅ | ✅ | ✅ | ✅ |
| CSS Flexbox | ✅ | ✅ | ✅ | ✅ |
| CSS Grid | ✅ | ⬜ | ✅ | ✅ |
| CSS 选择器 Level 3 | ✅ | 基础 | 全部 | ✅ |
| CSS 选择器 Level 4 | ✅ | ⬜ | ⬜ | 部分 |
| CSS 文本 | ✅ | 基础 | 全部 | ✅ |
| CSS 字体 | ✅ | ✅ | ✅ | ✅ |
| CSS 背景/边框 | ✅ | 基础 | 全部 | ✅ |
| CSS 颜色 | ✅ | ✅ | ✅ | ✅ |
| CSS 动画 | ✅ | ⬜ | 基础 | ✅ |
| CSS 过渡 | ✅ | ⬜ | 基础 | ✅ |
| CSS 变换 (transform) | ✅ | ⬜ | 2D | 3D |
| CSS 媒体查询 | ✅ | ⬜ | 基础 | ✅ |
| CSS 自定义属性 | ✅ | ⬜ | ✅ | ✅ |
| CSS 滚动条 | ✅ | ⬜ | ⬜ | ✅ |
| CSS 列表 | ✅ | ⬜ | ✅ | ✅ |
| CSS 表格 | ✅ | ⬜ | ✅ | ✅ |
| CSS 滤镜 | ✅ | ⬜ | ⬜ | 部分 |

**CSS 属性总量**：W3C 标准定义约 500+ 个属性。分期覆盖：

| 阶段 | 属性数量 | 覆盖率 |
|------|----------|--------|
| Phase 0 | ~30 个 | 核心布局 + 文字 |
| Phase 1 | ~80 个 | 日常开发 90%+ |
| Phase 2 | ~200 个 | 全面覆盖 |
| Phase 3 | ~400+ 个 | 接近完整 |

### 3.3 DOM 标准

| DOM 模块 | 规格 | 一期 | 二期 | 三期 |
|----------|------|------|------|------|
| Core (Node, Element, Text) | DOM Living Standard | ✅ | ✅ | ✅ |
| Document | DOM Living Standard | ✅ | ✅ | ✅ |
| Events (Event, EventTarget) | DOM Living Standard | 基础 | 全部 | ✅ |
| CSSOM (style, classList) | CSSOM | ✅ | ✅ | ✅ |
| HTML 元素扩展 | HTML Living Standard | 基础 | 全部 | ✅ |
| MutationObserver | DOM Living Standard | ⬜ | ✅ | ✅ |
| IntersectionObserver | W3C | ⬜ | ⬜ | ✅ |
| ResizeObserver | W3C | ⬜ | ⬜ | ✅ |

### 3.4 ECMAScript 标准（通过 JS→Rust 编译器覆盖）

| 特性 | 阶段 | 说明 |
|------|------|------|
| 变量声明 (let/const/var) | P0 | 作用域 + 提升 |
| 函数 (function, arrow) | P0 | 闭包 + 捕获 |
| 基本类型 (string, number, boolean, null, undefined) | P0 | Rust 类型映射 |
| 对象 (Object, Array, Map, Set) | P0 | 对应 Rust 集合 |
| 流程控制 (if/for/while/switch) | P0 | 对应 Rust 流程控制 |
| Promise / async / await | P1 | Rust Future 映射 |
| Proxy / Reflect | P2 | 运行时元编程（复杂） |
| Symbol | P1 | 基本支持 |
| 迭代器 / Generator | P1 | Rust Iterator 映射 |
| 模块 (import/export) | P1 | Rust module 映射 |
| Class | P1 | Rust struct + impl |
| TypedArray / ArrayBuffer | P2 | 二进制数据处理 |
| ES Next 特性 | P3 | 按需 |

### 3.5 Web API（非 DOM 部分）

| API | 标准 | 一期 | 二期 | 三期 |
|-----|------|------|------|------|
| console | Console 标准 | ✅ | ✅ | ✅ |
| setTimeout / setInterval | HTML Timers | ✅ | ✅ | ✅ |
| requestAnimationFrame | HTML | ✅ | ✅ | ✅ |
| fetch / XMLHttpRequest | Fetch / XHR | ⬜ | ✅ | ✅ |
| localStorage / sessionStorage | Web Storage | ⬜ | ✅ | ✅ |
| URL / URLSearchParams | URL 标准 | ⬜ | ✅ | ✅ |
| WebSocket | WebSocket | ⬜ | ⬜ | ✅ |
| History API | HTML History | ⬜ | ✅ | ✅ |
| Canvas 2D | Canvas 标准 | ⬜ | ⬜ | ✅ |
| WebGL | WebGL 标准 | ⬜ | ⬜ | ✅ |
| Web Worker | Worker 标准 | ⬜ | ⬜ | ✅ |
| SVG | SVG 标准 | ⬜ | ⬜ | 部分 |

---

## 四、分期规划

### 4.1 Phase 0 — 最小渲染原型

**目标**：走通 DOM → CSS → Layout → wgpu 完整管线

**范围**：
- DOM: 基础 Node/Element/Text/Document，树操作 API
- CSS: ~30 个核心属性，cssparser + selectors 集成
- 布局: Flexbox + Block + Positioned
- 渲染: wgpu 矩形/边框/文本
- 事件: click
- JS 编译: Phase 0 简化版（模式识别：querySelector、getElementById、addEventListener、textContent）
- 编译器: web2rust crate，将 HTML+CSS+JS 源文件编译为 Rust DOM API 代码

**可演示**：从 index.html + style.css + app.js 编译为原生二进制，弹出窗口显示带交互的计数器 UI

### 4.2 Phase 1 — 核心引擎

**目标**：可渲染标准 HTML 页面，支持基础交互

**新增**：
- CSS: ~80 个属性，完整级联
- 布局: Inline + Grid + 文本布局完整
- 事件: 鼠标/键盘/滚动完整
- JS 编译: SolidJS / Vue / React 编译产物运行
- HTML: html5ever 解析 + 基础 HTML 元素
- 图像: PNG 解码 + 显示
- 表单: input / button / textarea

**可演示**：打开一个 HTML 文件，正常渲染和交互

### 4.3 Phase 2 — 功能完善

**目标**：支持动态 Web 应用

**新增**：
- CSS: ~200 个属性，动画/过渡/变换/媒体查询
- JS 编译: Promise/async, Class, Iterator
- 布局: Table, Float
- 网络: fetch API
- 存储: localStorage
- 完整表单控件
- MutationObserver

**可演示**：TODO App, 简单数据可视化

### 4.4 Phase 3 — 跨平台 + 完整

**目标**：覆盖所有目标平台，接近完整浏览器标准

**新增**：
- WASM 编译 → 浏览器内以 WebGPU 后端运行
- 小程序支持 → 引擎编译为 WASM，在小程序 WebView 中通过 WebGPU 绘制
- iOS / Android 原生
- Canvas 2D / WebGL
- Web Worker / WebSocket
- 增量重排 / GPU 合批优化 / 内存优化
- SVG 基础支持

---

## 五、项目结构（完整）

```
/rust-browser-engine/
├── Cargo.toml                     # workspace 根
├── crates/
│   ├── dom/                       # W3C DOM 实现
│   │   ├── Cargo.toml
│   │   ├── properties.toml        # DOM 属性元数据
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── node.rs            # Node 核心
│   │       ├── element.rs         # Element + HTML 元素
│   │       ├── document.rs        # Document
│   │       ├── text.rs            # Text
│   │       ├── event.rs           # Event / EventTarget
│   │       ├── cssom.rs           # CSSStyleDeclaration, etc.
│   │       ├── html/              # HTML 元素特化
│   │       └── mutation_observer.rs # MutationObserver (Phase 2+)
│   │
│   ├── css/                       # CSS 引擎
│   │   ├── Cargo.toml
│   │   ├── properties.toml        # CSS 属性定义
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── stylesheet.rs      # cssparser → StyleSheet
│   │       ├── selector.rs        # selectors 包装
│   │       ├── cascade.rs         # 级联计算
│   │       ├── computed.rs        # 计算值
│   │       └── values/            # CSS 值类型
│   │
│   ├── layout/                    # 布局引擎 (独立模块)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── layout_box.rs      # LayoutBox 类型
│   │       ├── flex.rs            # taffy Flexbox
│   │       ├── grid.rs            # taffy Grid
│   │       ├── block.rs           # Block 布局
│   │       ├── inline.rs          # Inline 布局
│   │       ├── positioned.rs      # 定位布局
│   │       ├── table.rs           # 表格布局
│   │       ├── float.rs           # 浮动布局
│   │       └── text.rs            # 文本布局
│   │
│   ├── paint/                     # DisplayList (独立模块)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── display_list.rs    # 绘制命令集合
│   │       ├── command.rs         # PaintRect, PaintText, PaintBorder...
│   │       └── batching.rs        # 按 z-order + 纹理合批
│   │
│   ├── render/                    # 渲染后端 (可替换)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # RenderBackend trait
│   │       ├── wgpu/              # wgpu 后端 (DX12/Vulkan/Metal)
│   │       │   ├── mod.rs
│   │       │   ├── device.rs      # GPU 设备初始化
│   │       │   ├── pipeline.rs    # 渲染管线
│   │       │   ├── shader.wgsl    # 着色器
│   │       │   └── glyph_cache.rs # 字形缓存
│   │       └── webgpu/            # WebGPU WASM 后端 (浏览器/小程序)
│   │           ├── mod.rs
│   │           └── canvas.rs      # web-sys Canvas 绑定
│   │
│   ├── web2rust/                  # JS→Rust 编译器（Phase 0: 模式识别 / Phase 1+: swc 完整编译）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser.rs          # swc 解析
│   │       ├── analyzer.rs        # JS 语义分析
│   │       ├── codegen.rs         # Rust 代码生成
│   │       └── builtins.rs        # 内置对象映射
│   │
│   ├── net/                       # 网络层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── fetch.rs           # fetch API
│   │       └── websocket.rs       # WebSocket
│   │
│   ├── storage/                   # 存储层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── local_storage.rs
│   │
│   └── runtime/                   # 运行时整合
│       ├── Cargo.toml
│       ├── Cargo.toml.wasm        # WASM 构建配置
│       └── src/
│           ├── lib.rs
│           ├── native.rs          # winit 窗口 + 原生事件循环
│           ├── wasm.rs            # WASM 事件循环 (web-sys)
│           ├── hit_test.rs        # 命中检测
│           ├── event_loop.rs      # 主循环
│           └── app.rs             # 应用入口
│
└── examples/
    ├── counter/                   # Phase 0 计数器（源文件驱动模式：build.rs + index.html + style.css + app.js）
    ├── simple_page/               # Phase 1 HTML 页面
    └── todo_app/                  # Phase 2 TODO 应用
```

---

## 六、技术栈

| 层 | 技术 | 用途 |
|----|------|------|
| HTML 解析 | **html5ever** | W3C HTML5 解析 |
| CSS 解析 | **cssparser** | W3C CSS 解析 |
| CSS 选择器 | **selectors** | W3C 选择器匹配 |
| 布局引擎 (独立) | **taffy** (Flexbox/Grid) + **自研** (Block/Inline/Table/Float) | 布局计算 |
| 文本布局 | **rustybuzz + fontdb + unicode-linebreak** | 字体 shaping + 换行 |
| 渲染后端 (独立, 可替换) | **wgpu** (原生: DX12/Vulkan/Metal) + **web-sys WebGPU** (WASM) | GPU 渲染 |
| 窗口/输入 | **winit** (原生) / **web-sys** (WASM) | 跨平台窗口/事件 |
| 小程序适配 | **wasm-bindgen + web-sys** | WASM 运行时 |
| JS 编译 | **swc (parser)** | JS 解析 + AST |
| 图像解码 | **image** crate | PNG/JPEG/WebP 解码 |
| 编码 | **encoding_rs** | 字符编码 |

---

## 七、验收标准

### 7.1 Phase 0 验收
- [ ] Rust DOM API 可构建 UI 树并输出 DOM 结构
- [ ] CSS 解析 + 选择器匹配 + 级联计算正确
- [ ] Flexbox 布局计算正确（taffy 集成）
- [ ] wgpu 窗口弹出并显示渲染内容
- [ ] 点击事件触发重绘
- [ ] 窗口 resize 触发布局重排

### 7.2 Phase 1 验收（待细化）
- [ ] html5ever 解析 HTML 生成 DOM 树
- [ ] HTML 文档渲染到窗口
- [ ] 鼠标/键盘事件正常工作
- [ ] JS→Rust 编译器可转换基础 JS 代码

### 7.3 Phase 2-3 验收（待细化）
- [ ] 完整的 CSS 布局模式支持
- [ ] 网络 API 正常工作
- [ ] 跨平台构建验证

---

## 八、项目约束

### 8.1 不做（各阶段均不涉及）
- 嵌入式 JS 引擎（V8 / Boa / QuickJS）
- 插件系统 / 扩展
- 开发者工具（DevTools）
- 打印 / PDF 导出
- 无障碍 (ARIA)
- 加密 API (WebCrypto)
- 支付 API
- 蓝牙 / USB / 串口 API
- WebAssembly 执行

### 8.2 架构原则
1. **编译时转换**：所有 JS 逻辑在构建期转换为 Rust，不在运行时执行 JS
2. **标准遵循**：API 命名和行为遵循 W3C 规范，不创造私有 API
3. **渐进增强**：从最小子集逐步扩展到完整规范
4. **输入层浏览器兼容**：源码为标准 HTML+CSS+JS，可直接在浏览器中打开预览
5. **布局渲染分离**：`layout` crate 只计算位置尺寸，`paint` crate 生成绘制命令，`render` crate 负责 GPU 输出，三者独立可替换
6. **渲染后端可插拔**：通过 `RenderBackend` trait 抽象，支持 wgpu（原生）和 WebGPU（WASM/小程序）多种后端
7. **模块解耦**：每个 crate 独立，可单独测试和替换
8. **平台抽象**：窗口和事件层通过 trait 抽象，便于移植
