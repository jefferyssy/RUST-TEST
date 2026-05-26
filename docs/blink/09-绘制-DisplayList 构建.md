# 09 — 绘制 DisplayList 构建：扁平枚举列表 vs DisplayItem + PaintChunk 分层

## 问题/目标

绘制阶段的任务是将布局树（每个节点有精确的 `Rect` 和 `ComputedStyle`）转换为渲染器可消费的绘制命令列表。

关键约束：
- 绘制顺序遵循 CSS 层叠规则（背景 → 边框 → 内容 → 效果）
- 需要支持透明度和裁剪分组
- 需要支持增量绘制（只重绘变更区域）
- 命令列表的效率直接影响 GPU 渲染性能（合批、实例化）

## 我们的实现

### PaintCommand：单一枚举类型

来源：[crates/render_tree/src/command.rs](../../crates/render_tree/src/command.rs)

```rust
pub enum PaintCommand {
    FillRect {
        rect: Rect<f32>,
        color: Color,
        radius: f32,               // 圆角半径
    },
    Text {
        text: String,
        font_size: f32,
        font_family: String,
        font_weight: u16,
        x: f32, y: f32,
        color: Color,
        decoration: TextDecoration,
    },
    Border {
        rect: Rect<f32>,
        widths: [f32; 4],         // 四边可能不同宽度
        colors: [Color; 4],       // 四边可能不同颜色
        radius: f32,
        style: BorderStyle,
    },
    // Phase 1 新增
    BoxShadow { rect, offset_x, offset_y, blur_radius, ... },
    Image { rect, data: Vec<u8>, width, height, fit: ObjectFit },
    Clip { rect: Rect<f32>, commands: Vec<PaintCommand> },
    Opacity { alpha: f32, commands: Vec<PaintCommand> },
}
```

**特点**：
- 扁平枚举，所有绘制类型在一个 `enum` 中
- `Clip` 和 `Opacity` 是嵌套分组（`commands: Vec<PaintCommand>`），递归处理
- 8 种变体覆盖主要绘制需求
- `BorderStyle` 9 种变体（仅 Solid 实际使用）
- `Vec<PaintCommand>` 扁平列表存储

### DisplayList：扁平命令列表 + Z 排序

```rust
pub struct DisplayList {
    commands: Vec<PaintCommand>,
}

impl DisplayList {
    /// 按 z-order 排序：FillRect(0) → Image(1) → Border(2) → Text(3) → BoxShadow(-1) → Clip(5) → Opacity(6)
    pub fn sort_by_z_order(&mut self) {
        self.commands.sort_by(|a, b| layer(a).cmp(&layer(b)));
    }
}
```

### DisplayListBuilder：布局树遍历 → 命令生成

来源：[crates/render_tree/src/builder.rs](../../crates/render_tree/src/builder.rs)

```rust
impl DisplayListBuilder {
    pub fn build(&mut self, layout_root: &LayoutBox) -> DisplayList {
        self.display_list.clear();
        self.process_node(layout_root);
        self.display_list.sort_by_z_order();     // ★ 按 z-order 排序所有命令
        std::mem::take(&mut self.display_list)
    }

    fn process_node(&mut self, node: &LayoutBox) {
        // 0. BoxShadow（渲染在背景之下，layer=-1）
        if let Some(shadow) = Self::extract_box_shadow(node) {
            self.display_list.push(shadow);
        }

        // 1. background-color → FillRect (layer=0)
        if let Some(color) = Self::extract_bg_color(node) {
            self.display_list.push(PaintCommand::FillRect { rect: node.rect, color, radius });
        }

        // 2. border → Border (layer=2)
        if let Some(border) = Self::extract_border(node) { ... }
        // 2b. 单边边框 (border-top/right/bottom/left)
        for side in &["border-top", "border-right", "border-bottom", "border-left"] { ... }

        // 3. text → Text (layer=3)
        if let Some(text_cmd) = Self::extract_text(node) { ... }

        // 4. 递归处理子节点 —— 不产生 Clip/Opacity 分组
        for child in &node.children {
            self.process_node(child);
        }
    }
}
```

**构建流程**：
1. DFS 遍历布局树，每个节点生成 0~4 条 PaintCommand
2. 所有命令扁平插入同一个 `Vec<PaintCommand>`
3. `sort_by_z_order()` 全局排序（所有节点的背景先于所有节点的文本）
4. 无裁剪优化、无合批去重

### 风格提取：从 ComputedStyle 按需取值

```rust
// 提取背景色
fn extract_bg_color(node: &LayoutBox) -> Option<Color> {
    let style = node.computed_style.as_ref()?;
    // 检查 background-color → 检查 background 简写
    if let Some(bg) = style.get("background-color") { return parse_css_value_color(bg); }
    if let Some(bg) = style.get("background") { return parse_css_value_color(bg); }
    None
}

// 提取边框：解析 "1px solid #ddd" 字符串
fn extract_border(node: &LayoutBox) -> Option<PaintCommand> {
    let raw = style.get("border")?.as_keyword()?;
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    // 手动解析 width / color
    let width = tokens.iter().find_map(|t| t.strip_suffix("px")?.parse().ok()).unwrap_or(1.0);
    let color = tokens.iter().find_map(|t| Some(parse_color(t))).unwrap_or(BLACK);
    Some(PaintCommand::Border { rect, widths: [width; 4], colors: [color; 4], ... })
}

// 提取文本
fn extract_text(node: &LayoutBox) -> Option<PaintCommand> {
    // font-size → x, y, 颜色, placeholder 支持
    let half_leading = (node.rect.height - font_size).max(0.0) / 2.0;
    Some(PaintCommand::Text { text, font_size, font_family, font_weight,
                              x: node.rect.x, y: node.rect.y + half_leading, color, ... })
}
```

## Blink 的实现

### DisplayItem：类型层次体系

```cpp
// Blink 使用 DisplayItem 类层次（而非 enum）
// 每个绘制类型是一个子类，具有自己的参数和序列化方法

class DisplayItem {
public:
    DisplayItemType GetType() const;  // 类型枚举（50+ 种类）
    const PaintChunk& GetPaintChunk() const;

    // 虚方法
    virtual bool Equals(const DisplayItem&) const;
    virtual sk_sp<SkPicture> GetPicture() const;  // 录制为 SkPicture

protected:
    // 与旧 DisplayItem 比较（增量绘制时检查"是否与上一帧相同"）
    bool EqualsForInvalidation(const DisplayItem& other) const;
};

// 具体子类别
class DrawingDisplayItem : public DisplayItem {
    PaintRecord paint_record_;  // SkPicture 录制
};

class BoxDecorationBreakDisplayItem : public DrawingDisplayItem { ... };
class CaretDisplayItem : public DrawingDisplayItem { ... };
class ScrollbarDisplayItem : public DrawingDisplayItem { ... };
class SVGImageDisplayItem : public DrawingDisplayItem { ... };
class ForeignLayerDisplayItem : public DisplayItem {
    scoped_refptr<cc::Layer> layer_;  // 委托给 cc::Layer
};

// Type 枚举（50+ 种）
enum class DisplayItemType : uint8_t {
    kBoxDecorationBackground,
    kCapsLockIndicator,
    kCaret,
    kClipPaintPhase,
    kDragImage,
    kLinkHighlight,
    kScrollCorner,
    kSVGImage,
    kText,
    kTextEmphasisMark,
    kTextShadow,
    kVideo,
    kWebPlugin,
    kBoxShadow,
    kFilter,
    kOpacity,
    kTransform,
    // ... 30+ 更多类型
};
```

### PaintChunk：绘制分块

```cpp
// PaintChunk 是一组共享相同属性的 DisplayItem 序列
// 对应一个"绘制层"或一个 stacking context
struct PaintChunk {
    // 此块中的 DisplayItem 索引范围 [begin_index, end_index)
    wtf_size_t begin_index;
    wtf_size_t end_index;

    // 关联的 PropertyTree 节点 ID
    PropertyTreeState properties;  // { transform, clip, effect }

    // 命中测试信息
    const TransformPaintPropertyNode* hit_test_odata;

    // 已知绘制边界（用于增量绘制 culling）
    gfx::Rect bounds;        // 联合边界
    gfx::Rect drawable_bounds; // 可绘制子集
    gfx::Rect rect_known_to_be_opaque;

    // 是否为缓存的包裹（记录 DrawingDisplayItem 被录制为 cc::Layer paint op）
    bool is_cached = false;
};
```

### PropertyTrees：属性分离

```cpp
// PropertyTrees 将 transform/clip/effect 从 DisplayItem 中分离
// 每个 DisplayItem 通过 ID 引用属性节点，而非内嵌属性值

class PropertyTrees {
    TransformTree transform_tree;   // 变换树（旋转、缩放、平移）
    ClipTree clip_tree;             // 裁剪树（overflow: hidden, border-radius clip）
    EffectTree effect_tree;         // 效果树（opacity, filter, mask）
    ScrollTree scroll_tree;         // 滚动偏移
};

// 示例：一个带 transform 和 opacity 的元素
// 不需要为每个命令复制 transform 和 opacity 值
// DisplayItem 仅存储 property_state_ 的三个节点 ID
// GPU 合成时按节点 ID 查表获取实际值
```

### PaintArtifact：完整绘制产物

```cpp
class PaintArtifact {
public:
    // DisplayItem 序列（扁平数组）
    const Vector<DisplayItem>& GetDisplayItemList() const;

    // PaintChunk 序列（分块索引 + 属性）
    const Vector<PaintChunk>& GetPaintChunks() const;

    // 是否为空的绘制产物
    bool IsEmpty() const;

    // 比较两个 PaintArtifact 的差异（增量更新用）
    void UpdateExistingChunks(const PaintArtifact& old, const PaintChunkSubset&);

private:
    Vector<std::unique_ptr<DisplayItem>> display_items_;
    Vector<PaintChunk> chunks_;
};
```

### BoxPainter：绘制命令生成

```cpp
// 每个 LayoutBox 的绘制由 BoxPainter 处理
class BoxPainter {
public:
    void Paint(const PaintInfo& paint_info) {
        // 1. 背景（含 background-color, background-image, background-clip）
        PaintBackground(paint_info);

        // 2. 盒阴影（在背景之下）
        PaintBoxShadow(paint_info, kBackgroundBleedAvoidance);

        // 3. 边框
        PaintBorder(paint_info);

        // 4. 前景（文本、子元素、轮廓）
        PaintForeground(paint_info);

        // 5. 盒阴影（在前景之上）
        PaintBoxShadow(paint_info, kDrawInForeground);
    }
};

// PaintInfo 携带当前绘制上下文
struct PaintInfo {
    GraphicsContext& context;         // Skia 绘制上下文
    const CullRect& cull_rect;       // 剔除矩形（视口外的不绘制）
    PaintPhase phase;                // 绘制阶段（Background/Foreground/Outline 等）
    const PhysicalOffset& paint_offset;
};
```

### 增量绘制：DisplayItem 比较

```cpp
// Blink 通过比较上一帧的 DisplayItem 决定哪些需要重绘
// 未变化的 DisplayItem 直接复用（从 SkPicture 缓存取）
bool DisplayItem::EqualsForInvalidation(const DisplayItem& other) const {
    if (GetType() != other.GetType())
        return false;

    // 比较几何信息
    if (VisualRect() != other.VisualRect())
        return false;

    // 子类特定比较
    return PropertiesEqual(other);
}

// 绘制时跳过未变更的 item
void PaintController::UpdateCurrentPaintChunkProperties(...) {
    // 仅当 DisplayItem 属性变更时才创建新的 PaintChunk
    // 相同属性的连续 DisplayItem 合并到同一个 PaintChunk
}
```

## 优劣势分析

| 维度 | 我们 (PaintCommand enum + DisplayList) | Blink (DisplayItem + PaintChunk + PropertyTrees) |
|------|---------------------------------------|------------------------------------------------|
| **类型定义** | 单一 enum，8 种变体 | 类层次 + 50+ 种 Type 枚举 |
| **绘制命令** | `Vec<PaintCommand>` 扁平列表 | `DisplayItem` + `PaintChunk` 双层结构 |
| **属性存储** | 属性嵌入命令（rect, color, font_size） | 属性分离到 PropertyTrees（命令存 ID） |
| **层叠排序** | 全局 `sort_by_z_order()` | 深度优先遍历自然有序（不排序） |
| **增量绘制** | 无：每帧重建全部命令 | DisplayItem 比较 + SkPicture 缓存复用 |
| **变换/裁剪/透明度** | 嵌套命令（`Clip { commands: Vec<_> }`） | PropertyTrees 节点引用 |
| **视口剔除** | 无：所有命令发送到 GPU | CullRect 视口外跳过 |
| **绘制阶段** | 不分阶段 | BackgroundBorder / Foreground / Outline / Caret 等阶段 |
| **扩展性** | 加 enum 变体 + 所有 match 分支 | 加子类 + 虚方法重写 |
| **内存占用** | ~72B/cmd（字段嵌入） | ~120B+/item（虚表指针 + 字段） |

### 我们的优势

1. **概念简单**：165 行 build 代码 + 207 行 command 定义即可工作
2. **序列化自然**：`#[derive(Debug, Clone)]` 一键获得，无需手写序列化
3. **类型安全**：`match` 穷尽检查，添加绘制类型自动获得编译错误提示
4. **嵌套直观**：`Clip { commands: Vec<_> }` 语义清晰
5. **Z 排序直接**：排序函数一行，layer 值明确定义

### 我们的劣势

1. **全局排序 O(N log N)**：每个元素的背景/边框/文本被拆分后全局排序，失去空间局部性
2. **无增量绘制**：每帧重建全部命令，1000 次绘制即使只改 1 个字符也重建 1000 条
3. **无 PropertyTrees**：变换/裁剪/透明度嵌入命令，无法独立缓存变换层
4. **无 CullRect 剔除**：视口外元素仍然生成命令并发送到 GPU
5. **无 DisplayItem 比较**：不能判断"这个命令和上一帧一样"，始终重建
6. **边框解析每次重复**：`extract_border` 每次手动解析字符串，无缓存
7. **placeholder 文本特殊处理**：文本为空时向上查找父级 placeholder，耦合度较高
8. **缺少过滤/遮罩/渐变**：仅 FillRect + Text + Border + BoxShadow，无 CSS filter/backdrop-filter、mask、gradient

### Blink 的优势

1. **PropertyTrees 分离**：变换/裁剪/效果独立存储，属性变更不触及 DisplayItem
2. **增量绘制**：DisplayItem 逐项比较，未变的直接从 SkPicture 缓存取出
3. **PaintChunk 分组**：同一 PropertyTree 状态的连续 DisplayItem 共享一个 PaintChunk
4. **CullRect 剔除**：快速跳过视口外绘制对象
5. **PaintPhase 分阶段**：背景/前景/轮廓分离，方便局部重绘
6. **Skia 集成**：DisplayItem 可录制为 SkPicture，高效复用

### Blink 的劣势

1. **代码量巨大**：DisplayItem 体系 + PaintChunk + PropertyTrees + PaintController 数万行
2. **概念复杂**：DisplayItem、PaintChunk、PropertyTrees、PaintArtifact、PaintController 多层抽象
3. **手写序列化**：每个 DisplayItem 子类需要手写 Equals/Serialize/Trace

## 改进方向

### 短期

1. **避免全局排序**：DFS 遍历时按背景→子元素→前景的自然顺序插入，生成时即有序
2. **缓存边框解析结果**：在 `LayoutBox` 或 `ComputedStyle` 层缓存解析后的边框/背景值
3. **简单 CullRect 剔除**：在 `process_node` 中检查 `node.rect` 是否与视口相交
4. **DisplayItem 指纹比较**：为每个命令计算轻量哈希（`(type, rect, color_hash)`），相同则跳过

### 中期

5. **PaintChunk 分层**：将连续相同属性的命令归组
   ```rust
   struct PaintChunk {
       commands: Vec<PaintCommand>,
       transform: Option<Transform>,
       clip_rect: Option<Rect<f32>>,
       opacity: Option<f32>,
   }
   struct DisplayList {
       chunks: Vec<PaintChunk>,
   }
   ```
6. **PropertyTrees 引用**：变换/裁剪/透明度从命令中分离为独立树，命令只存节点 ID

### 长期

7. **Skia 集成**：用 Skia 的 `SkPicture` / `SkCanvas` 替代自研 PaintCommand，获得硬件加速+增量绘制
8. **DisplayItem 缓存**：对 `<display_item, hash>` 做 L1 缓存，跳过重复生成
9. **多阶段绘制**：Support PaintPhase (Background → Border → Content → Effect → Foreground)
