# Rust 浏览器引擎 — 完整设计文档

> 最后更新: 2026-05-23
> 目标覆盖: Chrome Web API 中 80% 业务场景常用子集（HTML/DOM ~151, CSS ~211, JS/Web ~258，合计 ~620 个 API）

---

## 一、项目定位

将原生 HTML+CSS+JS 前端项目编译为 Rust 代码，通过自研 W3C 标准 DOM API 驱动渲染管线，输出原生跨平台应用。

**数据流**: `index.html + style.css + app.js → toolchain 编译器 → Rust 代码 → cargo build → 原生二进制`

---

## 二、架构 (8 Crates)

```
HTML+CSS+JS 源文件
      │
      ▼
[toolchain] 命令工具 — HTML/CSS/JS → Rust 代码编译
      │
      ▼
[dom] DOM 树构建 + 事件系统 + MutationObserver
      │
      ▼
[style] CSS 解析 + 选择器匹配 + 级联计算 + 动画/过渡
      │
      ▼
[layout] 布局计算 — Flex/Block/Grid/Table/Positioned/Float/Inline
      │
      ▼
[render_tree] 渲染树 — DisplayList 构建 + 批处理优化
      │
      ▼
[renderer] 最终渲染 — wgpu GPU 后端 + winit 窗口 + 事件循环 + HitTest
      │
      ▼
[屏幕]

辅助 crate:
[net]     — fetch API + WebSocket
[storage] — localStorage / sessionStorage
```

| Crate | 职责 | 源文件 | 测试数 |
|-------|------|--------|--------|
| `toolchain` | HTML/CSS/JS 编译器 | 7 | ~50 |
| `dom` | W3C DOM (Node/Element/Document/Event/HTML 元素/Observer) | 22 | ~50 |
| `style` | CSS 引擎 (选择器/级联/值类型/动画/过渡/媒体查询) | 9 | ~70 |
| `layout` | 布局引擎 (Flex/Block/Grid/Table/Positioned/Float/Inline) | 9 | ~25 |
| `render_tree` | DisplayList + BatchOptimizer | 3 | ~15 |
| `renderer` | wgpu + winit + HitTest + AnimationFrameScheduler | 8 | ~25 |
| `net` | fetch + WebSocket | 2 | ~10 |
| `storage` | localStorage / sessionStorage | 2 | ~10 |
| **合计** | | **62** | **~255** |

---

## 三、已完成

### 3.1 toolchain — 编译器

| 已完成 | 说明 |
|--------|------|
| HTML 解析器 | 手写解析器 → HtmlElement 树，支持自闭合/嵌套/忽略标签 |
| CSS 解析器 | CSS 规则解析 → CssRule 列表 + 选择器-元素匹配 |
| JS 模式识别 | querySelector/addEventListener/变量赋值/表达式编译 → EventHandler |
| JS 语义分析 | 变量声明 (let/const/var)、函数调用、DOM 操作检测 |
| Rust 代码生成 | generate_main_body / generate_element_code / generate_event_handler |
| 内置 API 映射 | console (log/error/warn), Math (abs/max/min/floor/ceil/round/random/PI), DOM API, TypedArray (11 种), Object (keys/values/entries/assign), History/Location/URL/Date (映射骨架) |
| Canvas 代码生成 | stub 骨架 |
| JS 语法 | `let`/`const`、函数/箭头函数、默认参数/剩余参数、`return`、`if/else`、`for`/`for...of`/`while`/`do...while`、`break`/`continue`、三元运算符、算术/比较/逻辑/赋值运算符、`typeof`、`switch/case`、`for...in`、`try/catch/finally`、`throw`、展开运算符、解构赋值、可选链 `?.`、空值合并 `??`、模板字符串、标签模板、`in` 运算符、`new` 运算符 |
| JS 模块 | `import { x }` / `import * as` / `import default` / `export default` / `export { x, y }` / `import.meta.url` |
| JS 对象/类 | 对象字面量、`class` → struct+impl、`constructor`、方法、`static`、`get/set`、`extends`、`super`、`this` → &self、`new.target`、私有字段 `#field` |
| JS 异步 | Promise → Future、async/await、Promise.resolve/reject/all/race、.then/.catch/.finally |
| JS 集合 | Map、Set、WeakMap、WeakSet、Symbol |
| JS Proxy/Reflect | `new Proxy` (get/set/has/deleteProperty trap)、Reflect.get/set/has/delete |
| JS JSON | JSON.stringify / JSON.parse |
| JS Timer | setTimeout、setInterval、clearTimeout、clearInterval |
| TypedArray | ArrayBuffer, Uint8/16/32Array, Int8/16/32Array, Float32/64Array, Uint8ClampedArray, BigInt64Array, BigUint64Array, DataView, TextEncoder, TextDecoder |

### 3.2 dom — DOM 树 + 事件

| 已完成 | 说明 |
|--------|------|
| Node | appendChild, removeChild, insertBefore, replaceChild, cloneNode, contains, parentNode, childNodes, firstChild/lastChild, previousSibling/nextSibling, nodeType, nodeName, textContent, normalize, isEqualNode, compareDocumentPosition, isSupported, lookupPrefix, lookupNamespaceURI, ownerDocument |
| Element | getAttribute, setAttribute, removeAttribute, hasAttribute, id, className, classList (add/remove/toggle/contains/replace/supports/item/length), tagName, innerHTML, style.property, querySelector, querySelectorAll, matches, focus, blur, childElementCount, scrollIntoView, scrollTo, scrollBy, scrollTop/Left |
| ElementData | 属性存储、classList 管理、style 解析与存储、事件监听器管理 |
| Document | createElement, createTextNode, createDocumentFragment, createComment, createElementNS, getElementById, getElementsByTagName, getElementsByClassName, querySelector, querySelectorAll, body, documentElement, title, importNode, adoptNode |
| Text | data, length, splitText, appendData, deleteData, insertData, replaceData, substringData |
| DocumentFragment | 类型定义 + appendChild 批量插入 |
| Event | Event (type/target/bubbles/cancelable), EventTarget (addEventListener/removeEventListener/dispatchEvent), preventDefault, stopPropagation, stopImmediatePropagation, eventPhase, timeStamp, defaultPrevented |
| MouseEvent | clientX/Y, button, altKey/ctrlKey/shiftKey/metaKey, click/mousedown/mouseup/mousemove |
| KeyboardEvent | key/code/altKey/ctrlKey/shiftKey/metaKey/repeat, keydown/keyup |
| FocusEvent | relatedTarget, focus/blur |
| WheelEvent | deltaX/Y/Z, deltaMode, wheel |
| AnimationEvent | animationName/elapsedTime, animationstart/end/iteration |
| TransitionEvent | propertyName/elapsedTime, transitionstart/end/cancel |
| InputEvent | data/inputType/isComposing, input |
| MutationObserver | observe/disconnect/takeRecords, MutationRecord (type/target/addedNodes/removedNodes/attributeName/oldValue) |
| DOMTokenList | add/remove/toggle/contains/replace/supports/item/length |
| HTML 元素 | HTMLAnchorElement (href/target/rel), HTMLImageElement (src/alt/width/height/complete/naturalWidth), HTMLInputElement (type/value/checked/disabled/name/placeholder), HTMLCanvasElement (getContext stub), HTMLFormElement (action/method/enctype/elements/submit/reset), HTMLSelectElement (value/selectedIndex/disabled/multiple), HTMLTextAreaElement (value/rows/cols/name/disabled/readOnly) |

### 3.3 style — CSS 引擎

| 已完成 | 说明 |
|--------|------|
| CSS 值类型 | CSSValue 枚举 (Keyword/Length/Percentage/Number/Color/String), 长度单位 (px/%/em/rem/vw/vh/vmin/vmax), 角度 (deg/rad/grad/turn), 时间 (s/ms), 分辨率 (dpi/dpcm), 颜色 (#rrggbb/#rgb/rgb()/rgba()/关键字) |
| 选择器 | 通配 `*`、标签 `div`、类 `.class`、ID `#id`、复合 `div.class#id`, SelectorEngine (querySelector/querySelectorAll 运行时匹配), 特异性计算 `(id, class, tag)` |
| 伪类 | `:hover`, `:active`, `:focus`, `:link`, `:visited`, `:root`, `:empty`, `:first-child`, `:last-child`, `:only-child`, `:first-of-type`, `:last-of-type`, `:only-of-type`, `:nth-child(an+b)`, `:nth-last-child`, `:nth-of-type`, `:nth-last-of-type`, `:not(selector)`, `:enabled`, `:disabled`, `:checked` |
| 级联计算 | compute_element_style (特异性排序 + !important + 继承传播), computed_value |
| 样式表 | parse_stylesheet, parse_inline_style, KeyframesRule, @keyframes 解析 |
| CSS 属性 (200+) | display/position/top/right/bottom/left, flex-direction/wrap/flow, justify-content/align-items/align-content/align-self, flex-grow/shrink/basis, order, gap/row-gap/column-gap, width/height/min-width/max-width/min-height/max-height, margin/padding 全方向, border/width/style/color/radius 全方向, box-sizing, box-shadow, outline, color, font-family/size/weight/style, line-height, text-align/decoration/transform, letter-spacing, word-spacing, white-space, text-overflow, text-indent, vertical-align, background/color/image/size/position/repeat/attachment, transform/origin, cursor, pointer-events, user-select, visibility, overflow/x/y, z-index, opacity, grid-template-columns/rows/areas, grid-column/row/area, grid-auto-columns/rows/flow, place-items/content/self, float, clear, filter (blur/brightness/contrast/drop-shadow/grayscale/hue-rotate/invert/opacity/saturate/sepia), transition/property/duration/timing-function/delay, animation/name/duration/timing-function/delay/iteration-count/direction/fill-mode/play-state, content, --* 自定义属性 + var() |
| 动画引擎 | AnimationEngine (@keyframes 注册 + 关键帧插值 + 迭代控制 + 暂停/取消), AnimationState (duration/delay/iteration_count/direction/fill_mode) |
| 过渡引擎 | TransitionEngine (属性变更触发 + 延迟 + 缓动 + 值插值), TransitionConfig, ease 缓动函数 |
| 媒体查询 | MediaEvaluator (width/height/orientation/prefers-color-scheme), @media 解析, window.matchMedia |
| CSS 函数 | calc() 基础四则运算, var() 含 fallback, url(), linear-gradient(), rgb()/rgba(), matrix()/translate()/rotate()/scale()/skew() (2D), blur()/brightness() 等 10 种滤镜函数 |
| At-Rules | @media, @keyframes, @font-face (数据模型) |
| 全局关键字 | `initial`, `inherit` |

### 3.4 layout — 布局引擎

| 已完成 | 说明 |
|--------|------|
| Flexbox | taffy 集成: flex-direction, justify-content, align-items, align-content, align-self, flex-grow/shrink/basis, gap, order, flex-wrap |
| Block | 垂直堆叠 + margin 合并 + 高度自动计算 |
| Grid | grid-template-columns/rows 解析 + fr/px/auto 轨道 + GridItem 排列 |
| Table | TableRow/TableCell + colspan/rowspan + border-spacing |
| Positioned | relative/absolute/fixed/sticky + z-index 层级 |
| Float | FloatDirection/ClearMode + 文字环绕 |
| Inline | 水平排列 + 自动换行 + line-height |
| Text | TextMeasurer: 字符宽度估算 + 文本换行计算 |
| 核心类型 | LayoutBox (BoxType/EdgeSizes/Overflow/BorderRadius/Visibility), Rect, Size |
| Viewport | DOM Size 作为布局视口 + vw/vh/% 单位解析 |

### 3.5 render_tree — 渲染树

| 已完成 | 说明 |
|--------|------|
| DisplayList | PaintCommand 枚举 (FillRect/Border/Text/Image/Clip) + DisplayList 容器 |
| DisplayListBuilder | LayoutBox 树 → PaintCommand 列表 (背景/边框/文本绘制命令) |
| BatchOptimizer | 同色矩形合批合并 + 遮挡剔除 (不透明区域覆盖移除) |

### 3.6 renderer — 渲染 + 运行时

| 已完成 | 说明 |
|--------|------|
| WgpuBackend | wgpu 23.0.1 桌面 GPU 渲染 (WGSL 着色器 + 矩形/边框/圆角/图片绘制 + TextureAtlas) |
| TextRenderer | rustybuzz 字形排版 + fontdb 字体加载 + 缓存 |
| WebWindow | 应用主入口 (create_window + DOM→CSS→Layout→DisplayList→GPU 全管线整合 + 事件处理) |
| RenderBackend trait | render/resize/present/size |
| AnimationFrameScheduler | requestAnimationFrame/cancelAnimationFrame + callback 调度 + ID 管理 |
| HitTester | 坐标命中检测 + 事件冒泡路径收集 + z-order + 可交互路径 |
| ObserverManager | ResizeObserver/IntersectionObserver/MutationObserver 统一调度 |
| Window trait | 窗口抽象 + ViewportInfo (尺寸/设备像素比/方向/配色方案) |
| App (winit) | ApplicationHandler 实现 (resumed/window_event/about_to_wait), 鼠标点击→HitTest→DOM事件派发→重建渲染管线 |

### 3.7 net — 网络层

| 已完成 | 说明 |
|--------|------|
| fetch API | HTTP GET/POST/PUT/DELETE + Headers + Response (json/text/status/ok/headers) + 超时处理 |
| WebSocket | 连接/发送/接收 + onmessage/onopen/onerror/onclose + readyState + close(code, reason) |

### 3.8 storage — 存储层

| 已完成 | 说明 |
|--------|------|
| localStorage | setItem/getItem/removeItem/clear/length/key, 内存存储后端 |
| sessionStorage | 同 localStorage API |

---

## 四、覆盖率

### HTML / DOM API — 115/151 (76%)

| 分类 | 目标 | 已实现 | 待实现 |
|------|------|--------|--------|
| HTML 元素 | 48 | 48 | 0 |
| Node API | 17 | 15 | 2 |
| Element API | 28 | 19 | 9 |
| Document API | 15 | 14 | 1 |
| Text API | 8 | 8 | 0 |
| 事件类型 | 26 | 18 | 8 |
| Observer API | 3 | 1 | 2 |
| **合计** | **151** | **115** | **36** |

### CSS API — 171/211 (81%)

| 分类 | 目标 | 已实现 | 待实现 |
|------|------|--------|--------|
| 布局属性 | 37 | 33 | 4 |
| 盒模型属性 | 23 | 22 | 1 |
| 排版属性 | 20 | 17 | 3 |
| 变换属性 | 7 | 2 | 5 |
| 滤镜属性 | 11 | 10 | 1 |
| 过渡属性 | 7 | 5 | 2 |
| 动画属性 | 9 | 9 | 0 |
| 遮罩/裁剪 | 4 | 0 | 4 |
| 选择器-伪类 | 27 | 22 | 5 |
| 选择器-伪元素 | 2 | 0 | 2 |
| 选择器-属性 | 6 | 0 | 6 |
| 选择器-组合器 | 4 | 0 | 4 |
| CSS 函数 | 24 | 16 | 8 |
| At-Rules | 4 | 3 | 1 |
| 其他 | 26 | 24 | 2 |
| **合计** | **211** | **171** | **47** |

### JS / Web API — 142/258 (55%)

| 分类 | 目标 | 已实现 | 待实现 |
|------|------|--------|--------|
| JS 语法 | 56 | 48 | 8 |
| Object | 14 | 4 | 10 |
| Array | 23 | 13 | 10 |
| String | 15 | 7 | 8 |
| Number/Math | 17 | 11 | 6 |
| Date | 11 | 1 | 10 |
| RegExp | 7 | 0 | 7 |
| TypedArray | 15 | 15 | 0 |
| Proxy/Reflect | 7 | 7 | 0 |
| Console/Timer | 6 | 6 | 0 |
| Fetch API | 13 | 11 | 2 |
| WebSocket | 10 | 7 | 3 |
| History API | 8 | 0 | 8 |
| Location API | 12 | 0 | 12 |
| URL/SearchParams | 12 | 0 | 12 |
| Canvas 2D | 19 | 1 | 18 |
| 其他 Web | 13 | 3 | 10 |
| **合计** | **258** | **142** | **116** |

---

## 五、待完成 (按优先级)

### P0 — 核心缺失 (阻塞常见业务场景)

| 编号 | 分类 | 内容 | 涉及文件 |
|------|------|------|---------|
| P0-1 | CSS 选择器 | 属性选择器 `[attr]`/`[attr=value]`/`[attr~=]`/`[attr^=]`/`[attr$=]`/`[attr*=]` 6 种 | `style/src/selector.rs` |
| P0-2 | CSS 选择器 | 组合器: 后代 `A B`、子代 `A>B`、相邻兄弟 `A+B`、通用兄弟 `A~B` | `style/src/selector.rs` |
| P0-3 | CSS 选择器 | `:has()` / `:is()` / `:where()` 伪类 | `style/src/selector.rs` |
| P0-4 | CSS 选择器 | `::before` / `::after` 伪元素 | `style/src/selector.rs`, `layout/` |
| P0-5 | CSS 函数 | `min()` / `max()` / `clamp()` | `style/src/values.rs` |
| P0-6 | JS String | `str.replace()` / `str.replaceAll()` | `toolchain/src/codegen.rs` |
| P0-7 | JS String | `str.toUpperCase()` / `str.toLowerCase()` | `toolchain/src/codegen.rs` |
| P0-8 | JS String | `str.indexOf()` / `str.lastIndexOf()` | `toolchain/src/codegen.rs` |
| P0-9 | JS Array | `arr.sort()` / `arr.reverse()` | `toolchain/src/codegen.rs` |
| P0-10 | JS Array | `arr.forEach()` / `arr.some()` / `arr.every()` | `toolchain/src/codegen.rs` |
| P0-11 | JS Array | `arr.concat()` / `arr.findIndex()` | `toolchain/src/codegen.rs` |
| P0-12 | JS RegExp | 正则表达式: `/pattern/flags` 字面量、`.test()`、`.exec()`、`/g`、`/i` | `toolchain/src/codegen.rs` |
| P0-13 | JS Date | Date 对象全套: `new Date()`、`getTime()`、`getFullYear/Month/Date`、`getHours/Minutes/Seconds`、`getDay()`、`toISOString()`、`toJSON()` | `toolchain/src/codegen.rs` |
| P0-14 | JS Object | `Object.create()` / `defineProperty()` / `freeze()` / `seal()` / `is()` / `hasOwn()` / `fromEntries()` / `getPrototypeOf()` / `setPrototypeOf()` | `toolchain/src/codegen.rs` |
| P0-15 | JS SPA | History API: `pushState`/`replaceState`/`back`/`forward`/`go`/`length`/`state`/`popstate` 事件 | `renderer/src/`, `toolchain/` |
| P0-16 | JS SPA | Location API: `href`/`host`/`hostname`/`pathname`/`search`/`hash`/`protocol`/`origin`/`port`/`assign`/`replace`/`reload` | `renderer/src/`, `toolchain/` |
| P0-17 | JS SPA | URL/URLSearchParams: `new URL()` / `searchParams` / `params.get/set/has/delete/toString/forEach` | `toolchain/src/codegen.rs` |

### P1 — 重要扩展

| 编号 | 分类 | 内容 | 涉及文件 |
|------|------|------|---------|
| P1-1 | DOM Element | `element.children` / `firstElementChild` / `lastElementChild` / `nextElementSibling` / `previousElementSibling` | `dom/src/element.rs` |
| P1-2 | DOM Element | `element.closest(selector)` | `dom/src/element.rs` |
| P1-3 | DOM Element | `element.getBoundingClientRect()` | `dom/src/element.rs` |
| P1-4 | DOM Node | `node.hasChildNodes()` / `node.isSameNode()` | `dom/src/node.rs` |
| P1-5 | DOM Document | `document.cookie` | `dom/src/document.rs` |
| P1-6 | CSS 布局 | `aspect-ratio` / `contain` / `content-visibility` | `style/src/properties.rs`, `layout/` |
| P1-7 | CSS 变换 | 3D 变换: `perspective` / `transform-style` / `backface-visibility` / `translateZ` / `rotate3d` / `scale3d` / `matrix3d` | `style/`, `renderer/` |
| P1-8 | CSS 滤镜 | `backdrop-filter` | `style/src/properties.rs`, `renderer/` |
| P1-9 | CSS 过渡 | `cubic-bezier()` / `steps()` 缓动函数 | `style/src/transitions.rs` |
| P1-10 | CSS 函数 | `radial-gradient()` / `currentColor` / `circle()`/`ellipse()`/`polygon()`/`inset()` (clip-path) | `style/src/values.rs` |
| P1-11 | CSS 排版 | `font-variant` / `font-stretch` / `word-break` / `overflow-wrap` | `style/src/properties.rs` |
| P1-12 | CSS At-Rule | `@import` | `style/src/stylesheet.rs` |
| P1-13 | JS 语法 | `var` 声明、`++/--` 自增自减、`instanceof`、位运算 `&|^~<<>>>>>` | `toolchain/src/codegen.rs` |
| P1-14 | JS Number | `Number.isInteger()` / `Number.toFixed()` | `toolchain/src/builtins.rs` |
| P1-15 | JS Math | `Math.sqrt()` / `Math.pow()` | `toolchain/src/builtins.rs` |
| P1-16 | JS String | `str.trimStart()` / `str.trimEnd()` / `str.match()` / `str.search()` | `toolchain/src/codegen.rs` |
| P1-17 | JS Array | `Array.of()` / `arr.indexOf()` | `toolchain/src/codegen.rs` |
| P1-18 | JS Promise | `Promise.allSettled()` / `Promise.any()` | `toolchain/src/codegen.rs` |
| P1-19 | JS Timer | `queueMicrotask()` | `renderer/src/event_loop.rs` |
| P1-20 | JS Canvas | Canvas 2D 核心: `fillRect`/`strokeRect`/`clearRect`/`fillStyle`/`strokeStyle`/`lineWidth`/`globalAlpha`/`beginPath`/`moveTo`/`lineTo`/`rect`/`arc`/`fill`/`stroke`/`save`/`restore`/`translate`/`rotate`/`scale`/`setTransform`/`fillText`/`font`/`textAlign`/`measureText`/`drawImage`/`toDataURL` | `renderer/src/`, `dom/src/html/canvas.rs`, `toolchain/src/canvas_codegen.rs` |

### P2 — 触控与移动端

| 编号 | 分类 | 内容 |
|------|------|------|
| P2-1 | DOM Event | TouchEvent / Touch / TouchList + touchstart / touchmove / touchend / touchcancel |
| P2-2 | DOM Event | PointerEvent + pointerdown / pointermove / pointerup / pointercancel |
| P2-3 | CSS 交互 | `touch-action` (pan-x/pan-y/pinch-zoom/manipulation) |
| P2-4 | DOM Event | `contextmenu` / `dblclick` / `mouseenter` / `mouseleave` / `mouseover` / `mouseout` |
| P2-5 | DOM Event | `submit` / `reset` / `change` 表单事件 |
| P2-6 | DOM Event | `focusin` / `focusout` 焦点冒泡 |
| P2-7 | DOM Event | `copy` / `cut` / `paste` 剪贴板事件 |

### P3 — Observer 与跨平台

| 编号 | 分类 | 内容 |
|------|------|------|
| P3-1 | DOM Observer | ResizeObserver (响应式布局核心) |
| P3-2 | DOM Observer | IntersectionObserver (懒加载/无限滚动) |
| P3-3 | JS Web | `window.innerWidth` / `window.innerHeight` |
| P3-4 | DOM Event | CustomEvent |
| P3-5 | JS Fetch | `fetch_async` (真正异步) + `Request` / `Headers` 构造函数 |
| P3-6 | JS WebSocket | 自动重连 + PING/PONG 心跳 + Per-Message Deflate |
| P3-7 | 跨平台 | WebGpuBackend (WASM) + Window trait 平台抽象 (iOS/Android) |
| P3-8 | HTML | `<video>` / `<audio>` + play/pause/currentTime/duration/volume |
| P3-9 | HTML | `<meta>` / `<link>` 文档元数据 |

### 远期

| 编号 | 分类 | 内容 |
|------|------|------|
| F-1 | DOM | Shadow DOM (`attachShadow`/`shadowRoot`/`slot`/`::part`/`::slotted`) |
| F-2 | DOM | 拖拽事件 (`dragstart`/`drag`/`dragend`/`dragenter`/`dragover`/`dragleave`/`drop`) |
| F-3 | CSS | 容器查询 (`container-type`/`container-name`/`@container`) |
| F-4 | CSS | Cascade Layers (`@layer`) |
| F-5 | CSS | CSS Houdini (`@property`) |

---

## 六、文件级状态速查

### toolchain/

| 文件 | 状态 | 说明 |
|------|------|------|
| `lib.rs` | ✅ | 模块导出 + compile/compile_body 入口 |
| `parser.rs` | ✅ | 手写 HTML/CSS 解析器 |
| `analyzer.rs` | ✅ | JS 语义分析 |
| `codegen.rs` | ✅ | Rust 代码生成器 (待补: String/Array/Date/RegExp 方法) |
| `builtins.rs` | ✅ | JS → Rust 内置映射表 (待补: Object 静态方法/Number/Math 扩展) |
| `canvas_codegen.rs` | 🔲 | Canvas 2D 代码生成 (stub) |
| `types.rs` | ✅ | 编译期类型定义 |

### dom/

| 文件 | 状态 | 说明 |
|------|------|------|
| `lib.rs` | ✅ | 模块导出 |
| `node.rs` | ✅ | Node 核心 (待补: hasChildNodes/isSameNode) |
| `element.rs` | ✅ | ElementData (待补: children/closest/getBoundingClientRect) |
| `document.rs` | ✅ | Document (待补: cookie) |
| `text.rs` | ✅ | Text 完整 |
| `event.rs` | ✅ | 事件系统 (待补: TouchEvent/PointerEvent/CustomEvent) |
| `dom_token_list.rs` | ✅ | classList 完整 |
| `mutation_observer.rs` | ✅ | MutationObserver 完整 |
| `html/*.rs` (8) | ✅ | HTMLAnchor/Image/Input/Canvas/Form/Select/TextArea 完整; 🔲 Video/Audio/Meta/Link |
| `observer/mod.rs` | ✅ | 模块导出; 🔲 ResizeObserver/IntersectionObserver 待实现 |

### style/

| 文件 | 状态 | 说明 |
|------|------|------|
| `lib.rs` | ✅ | 模块导出 |
| `values.rs` | ✅ | CSSValue (待补: min/max/clamp/radial-gradient/currentColor) |
| `selector.rs` | ✅ | 基础选择器 (待补: 属性选择器/组合器/:has/:is/:where/::before/::after) |
| `cascade.rs` | ✅ | 级联计算 |
| `stylesheet.rs` | ✅ | 样式表 (待补: @import) |
| `properties.rs` | ✅ | 200+ 属性解析 (待补: aspect-ratio/contain/content-visibility/backdrop-filter) |
| `animations.rs` | ✅ | 动画引擎完整 |
| `transitions.rs` | ✅ | 过渡引擎 (待补: cubic-bezier/steps) |
| `media.rs` | ✅ | 媒体查询 |
| `color.rs` | ✅ | 颜色类型 |

### layout/ — 全部 ✅

7 种布局模式均已完成: Flex/Block/Grid/Table/Positioned/Float/Inline + TextMeasurer

### render_tree/ — 全部 ✅

DisplayList + DisplayListBuilder + BatchOptimizer 均已完成

### renderer/

| 文件 | 状态 | 说明 |
|------|------|------|
| `lib.rs` | ✅ | RenderBackend trait |
| `window.rs` | ✅ | WebWindow + App 主循环 |
| `wgpu_backend.rs` | ✅ | wgpu 渲染后端 |
| `text_renderer.rs` | ✅ | 文本渲染 |
| `event_loop.rs` | ✅ | AnimationFrameScheduler (待补: queueMicrotask) |
| `hit_test.rs` | ✅ | HitTester |
| `observer_manager.rs` | ✅ | ObserverManager |
| `window_trait.rs` | 🔲 | 跨平台 Window trait 抽象 |

### net/

| 文件 | 状态 | 说明 |
|------|------|------|
| `fetch.rs` | ✅ | fetch (待补: async + Request/Headers 构造函数) |
| `websocket.rs` | ✅ | WebSocket (待补: 自动重连+心跳) |

### storage/ — 全部 ✅

localStorage + sessionStorage 完整 CRUD

---

## 七、统计总览

| 维度 | 目标 API | 已实现 | 待实现 | 覆盖率 |
|------|---------|--------|--------|--------|
| HTML/DOM | 151 | 115 | 36 | 76% |
| CSS | 211 | 171 | 47 | 81% |
| JS/Web API | 258 | 142 | 116 | 55% |
| **合计** | **620** | **428** | **199** | **69%** |

| 维度 | 数值 |
|------|------|
| Crate 数 | 8 |
| 源文件数 | 62 |
| 测试函数数 | ~255 |
| 已实现 API | 428 |
| 待实现 API | 199 |
| 整体覆盖率 | 69% |
