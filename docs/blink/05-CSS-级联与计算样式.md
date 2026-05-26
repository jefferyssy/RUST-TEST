# 05 — CSS 级联与计算样式：HashMap 全量克隆 vs 分组 CoW ComputedStyle

## 问题/目标

CSS 级联（Cascade）的核心任务是：从多个来源（UA 默认、作者样式表、内联样式）的声明中，按优先级选择最终生效的属性值，输出每个 DOM 元素的 `ComputedStyle`。

关键约束：
- 页面可能有数千个 DOM 节点，每个节点有 200+ CSS 属性
- 大多数元素之间样式高度相似（如继承、默认值）
- DOM 变更频繁，样式需要快速重新计算

## 我们的实现

### ComputedStyle：HashMap 设计

来源：[crates/style/src/cascade.rs](../../crates/style/src/cascade.rs)

```rust
/// 计算后的样式集合
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    /// 属性名 → 计算后值 的映射
    pub properties: HashMap<String, CSSValue>,
}

impl ComputedStyle {
    pub fn new() -> Self {
        Self { properties: HashMap::new() }
    }

    pub fn get(&self, name: &str) -> Option<&CSSValue> {
        self.properties.get(name)
    }

    pub fn set(&mut self, name: &str, value: CSSValue) {
        self.properties.insert(name.to_string(), value);
    }

    /// 合并另一个样式（仅添加不存在的属性）
    pub fn merge(&mut self, other: &ComputedStyle) {
        for (prop, val) in &other.properties {
            self.properties.entry(prop.clone())
                .or_insert_with(|| val.clone());
        }
    }
}
```

### 级联计算流程

```rust
pub fn compute_element_style(
    element: &ElementData,
    parent_style: Option<&ComputedStyle>,
    stylesheets: &[StyleSheet],
    inline_style: &[Declaration],
) -> ComputedStyle {
    // 1. 收集所有匹配的声明（选择器匹配）
    let mut matched = match_selectors(element, stylesheets);

    // 2. 内联声明以 u32::MAX 特异性加入（最高优先级）
    for decl in inline_style {
        matched.push(MatchedDeclaration {
            specificity: (u32::MAX, u32::MAX, u32::MAX),
            declaration: decl.clone(),
        });
    }

    // 3. 按特异性排序（低→高），!important 规则跳过排序优先
    matched.sort_by(|a, b| {
        if a.declaration.important && !b.declaration.important {
            return std::cmp::Ordering::Greater;
        }
        if !a.declaration.important && b.declaration.important {
            return std::cmp::Ordering::Less;
        }
        a.specificity.cmp(&b.specificity)  // (id, class, tag) 元组比较
    });

    // 4. 依次应用（后覆盖前）
    let mut style = ComputedStyle::new();
    for m in &matched {
        let value = parse_css_value(&m.declaration.property, &m.declaration.value);
        style.set(&m.declaration.property, value);  // 直接插入 HashMap
    }

    // 5. 继承父元素的可继承属性
    if let Some(parent) = parent_style {
        apply_inherited(parent, &mut style);
    }

    // 6. 应用 UA 默认样式（最低优先级，仅补足缺失属性）
    apply_user_agent_defaults(element, &mut style);

    style
}
```

### 继承机制

```rust
/// 从父元素继承可继承属性（color、font-*、text-align 等）
fn apply_inherited(parent: &ComputedStyle, child: &mut ComputedStyle) {
    const INHERITED_PROPS: &[&str] = &[
        "color", "font-size", "font-family", "font-weight", "font-style",
        "text-align", "line-height", "visibility", "cursor",
        "white-space", "word-spacing", "letter-spacing",
    ];
    for prop in INHERITED_PROPS {
        if !child.properties.contains_key(*prop) {
            if let Some(val) = parent.get(*prop) {
                child.set(*prop, val.clone());  // ★ 每个继承属性独立克隆
            }
        }
    }
}
```

### 内存开销分析

```
每个元素 ComputedStyle 的内存开销：

HashMap 元数据:    ~64 bytes (capacity, len, hasher)
每个属性条目:      ~48 bytes (String key + CSSValue enum + hash)
典型属性数:        15~40 个
每元素总大小:      ~800~2000 bytes

1000 个元素:       ~0.8~2 MB
10000 个元素:      ~8~20 MB

每次 Clone():      全量拷贝 HashMap → 第二个元素即使只多一个属性，
                   也要复制前一个元素的所有条目
```

## Blink 的实现

### ComputedStyle：不可变 + 分组 Copy-on-Write

```cpp
// Blink 的 ComputedStyle 是按属性组组织的不可变对象
class ComputedStyle : public RefCounted<ComputedStyle> {
public:
    // 每组由一个独立的 scoped_refptr 持有
    const StyleBoxData& BoxData() const;              // width, height, margin, padding, ...
    const StyleSurroundData& SurroundData() const;    // border, border-radius, position insets
    const StyleBackgroundData& BackgroundLayers() const; // background-color + 图层
    const StyleInheritedData& InheritedData() const;  // color, font-*, cursor, line-height, ...
    const StyleRareInheritedData& RareInheritedData() const; // text-align, white-space, ...
    const StyleRareNonInheritedData& RareNonInheritedData() const; // flex, grid, transform, filter, ...

private:
    // 每组是指向不可变数据的 scoped_refptr
    scoped_refptr<const StyleBoxData> box_;              // Box 模型组
    scoped_refptr<const StyleInheritedData> inherited_;  // 继承属性组
    scoped_refptr<const StyleRareInheritedData> rare_inherited_;
    scoped_refptr<const StyleRareNonInheritedData> rare_non_inherited_;
    scoped_refptr<const StyleBackgroundData> background_;
    // ...更多组
};
```

### StyleBuilder：构造器模式

```cpp
// StyleBuilder 用于逐步构建 ComputedStyle
// 与 StyleResolver 协作完成级联过程
class StyleBuilder {
public:
    void ApplyProperty(CSSPropertyID id, const CSSValue& value) {
        // 按属性 ID 分发到对应的数据组
        switch (id) {
            case CSSPropertyID::kWidth:
                MutableBoxData().SetWidth(value);    // 只修改 box 组
                break;
            case CSSPropertyID::kColor:
                MutableInheritedData().SetColor(value);  // 只修改 inherited 组
                break;
            case CSSPropertyID::kDisplay:
                MutableRareNonInheritedData().SetDisplay(value);  // 只修改 rare 组
                break;
        }
    }

    ComputedStyle* TakeStyle() {
        // 将各组"冻结"为不可变版本
        return MakeGarbageCollected<ComputedStyle>(
            box_->Clone(),          // unique 则移动，shared 则复制
            inherited_->Clone(),
            rare_inherited_->Clone(),
            // ...
        );
    }
};
```

### Copy-on-Write 原理

```cpp
// 场景：两个 <div> 共享同一个 ComputedStyle
 div1 -> ComputedStyle { box_ → StyleBoxData#1 (refcount=2) }
 div2 -> ComputedStyle { box_ → StyleBoxData#1 (refcount=2) }

// 为 div1 设置 width: 100px
 div1.ApplyProperty(kWidth, 100px);
   → MutableBoxData() 内部检测到 refcount > 1
   → 创建 StyleBoxData#2（复制 #1，修改 width_）
   → div1.box_ → StyleBoxData#2 (refcount=1)
   → div2.box_ → StyleBoxData#1 (refcount=1)  // ★ div2 不受影响

// 内存开销：仅分配 1 个新 StyleBoxData（~200 bytes），而非整个 ComputedStyle
```

### 级联排序：CascadePriority

```cpp
// Blink 的优先级排序比我们更精细
struct CascadePriority {
    // 比较顺序（从高到低）：
    // 1. IsImportant()     — !important 覆盖普通规则
    // 2. GetOrigin()       — Transition > Animation > Author > User > UA
    // 3. TreeOrder()       — @scope/@layer 声明顺序
    // 4. Specificity()     — 选择器特异性
    // 5. Appearance()      — 同优先级内出现顺序
};

// 匹配结果：所有匹配的声明按 CascadePriority 排序后逐一应用
void StyleResolver::CascadeAndApply(MatchedProperties& matched) {
    std::sort(matched.begin(), matched.end(),
              [](const auto& a, const auto& b) {
                  return a.cascade_priority < b.cascade_priority;
              });
    for (const auto& m : matched) {
        builder.ApplyProperty(m.property_id, m.value);
    }
}
```

### 延迟分配（Rare Data）

```cpp
// 普通 <div> 不设置为 flex 容器时，RareNonInheritedData 为 nullptr
class ComputedStyle {
    const StyleRareNonInheritedData* RareNonInheritedData() const {
        return rare_non_inherited_.get();  // 可能返回 nullptr
    }

    EDisplay Display() const {
        if (!rare_non_inherited_)
            return EDisplay::kInline;  // ★ 默认值，零内存开销
        return rare_non_inherited_->Display();
    }
};
```

## 优劣势分析

| 维度 | 我们 (HashMap) | Blink (分组 CoW) |
|------|---------------|-------------------|
| **克隆成本** | O(属性数) ≈ O(40) 全量复制 | O(1) 共享引用，仅修改时 O(修改组大小) |
| **内存占用** | 每元素 ~1KB (独立 HashMap) | 相似元素共享组，每元素 ~200B |
| **属性访问** | O(1) HashMap 查找 | O(1) 直接字段访问 |
| **序列化** | 天然支持 `serde` | 需手写序列化 |
| **代码复杂度** | 极简 (~120行) | 高（多组定义 + StyleBuilder + Mutable/Immutable 版本）|
| **类型安全** | 字符串 key，无编译期检查 | 枚举 ID，编译期检查 |
| **扩展属性** | 添加字符串即可 | 需在对应组添加字段 |

## 改进方向

### 短期

1. **属性分组**：将 `HashMap<String, CSSValue>` 拆分为 `BoxModel` / `Typography` / `Visual` 三个子 HashMap，克隆时按需拷贝
2. **常量属性名**：用 `&'static str` 或枚举替代 `String` key，减少分配

### 中期

3. **CoW ComputedStyle**：引入 `Arc<BoxModelData>` + `Arc::make_mut` 实现写时复制
   ```rust
   struct ComputedStyle {
       box_: Arc<BoxModelData>,
       typography: Arc<TypographyData>,
       rare: Arc<RareData>,
   }
   // 继承时全部共享 Arc，修改时 Arc::make_mut 才复制
   ```
4. **延迟分配 RareData**：flex/grid/transform 等罕见属性放在 `Option<Arc<RareData>>` 中

### 长期

5. **样式缓存**：`(Element, ComputedStyle)` 缓存键，相同选择器+相同继承链命中时跳过级联
6. **增量样式更新**：不重算整个 ComputedStyle，只更新变更的属性（配合 `StyleRecalcChange`）
