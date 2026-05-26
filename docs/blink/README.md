# Blink 参考实现对照文档

## 目的

本目录记录我们的 Rust 浏览器引擎与 Chrome Blink 渲染引擎的对照分析。通过对比 Blink 的生产级实现，系统性地暴露当前设计的差距，为后续架构演进提供参考。

## 为什么研究 Blink

- Blink 是 Chromium 的渲染引擎，经过 15+ 年打磨，支撑全球数十亿用户的日常浏览
- Blink 在 DOM、样式、布局、绘制、合成的每个环节都有成熟的工程实践
- 对照 Blink 可以帮助我们避免重蹈早期设计错误，直接借鉴验证过的模式

## 阅读指南

### 按渲染管线顺序

```
01-总览 → 02-DOM内存 → 05-CSS级联 → 06-布局树 → 07-布局算法 → 08-增量更新 → 09-绘制 → 10-合成 → 12-全管线
                                       ↑                ↑
                                   04-选择器          03-事件
```

### 按关注点跳读

- **内存模型**：02
- **样式计算**：04, 05
- **布局性能**：06, 07, 08
- **渲染优化**：09, 10, 11
- **全局视角**：01, 12, 13

## 文档结构约定

每篇文档遵循统一结构：

1. **问题/目标** — 这一层要解决什么核心问题
2. **我们的实现** — Rust 代码示例 + 架构说明（标注源文件路径）
3. **Blink 的实现** — C++ 代码示例 + 架构说明（标注 Blink 源文件路径）
4. **优劣势分析** — 双方的取舍与权衡
5. **改进方向** — 短期可行的优化建议

## 术语对照

| 我们的模块 | Blink 对应组件 | 说明 |
|-----------|---------------|------|
| `crates/dom/` | `third_party/blink/renderer/core/dom/` | DOM 树与事件 |
| `crates/style/` | `core/css/` + `core/style/` | CSS 解析与级联 |
| `crates/layout/` | `core/layout/` (LayoutNG) | 布局引擎 |
| `crates/render_tree/` | `platform/graphics/paint/` | DisplayList |
| `crates/renderer/` | `cc/` + `viz/` | 光栅化与合成 |
| `Rc<RefCell<Node>>` | `Member<Node>` (Oilpan GC) | 内存管理 |
| `LayoutBox` (enum) | `LayoutObject` (继承体系) | 布局树节点 |
| `PaintCommand` (enum) | `DisplayItem` (类体系) | 绘制命令 |
| `wgpu` | Skia + GL | 图形后端 |

## 关键发现速览

| 方面 | 我们 | Blink | 差距等级 |
|------|------|-------|---------|
| 内存管理 | Rc+RefCell 引用计数 | Oilpan 追踪式 GC | 中等 |
| 样式存储 | HashMap 全量克隆 | 分组 CoW 不可变对象 | **高** |
| 样式更新 | 全量重建 | StyleRecalcChange 增量标记 | **高** |
| 布局更新 | 全量 relayout() | 脏标记 + 局部重排 | **高** |
| 绘制组织 | 单层 PaintCommand 列表 | PaintChunk + PropertyTrees | 中等 |
| GPU 渲染 | 单 render pass 实例化 | 多进程合成 + tile 化 | 中等 |
| 选择器匹配 | 全树遍历 | bloom filter + 从右向左 | 低 |
