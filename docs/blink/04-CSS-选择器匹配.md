# 04 — CSS 选择器匹配：从左向右全树遍历 vs SelectorChecker + BloomFilter 右向左

## 问题/目标

CSS 选择器匹配是样式计算的前序步骤。给定一组样式表（每个规则包含选择器 + 声明块），为每个 DOM 元素找出所有匹配的规则声明。

关键约束：
- 样式规则数可能很大（数千条规则的 CSS 框架很常见）
- 每个规则需要检查页面中所有元素（Naive 算法 O(Rules × Elements)）
- 大多数规则只匹配极少数元素（如 `#header` 只匹配 1 个元素）
- 匹配必须快速，因为它在每次样式重算时运行

## 我们的实现

### 匹配策略：从左向右 + 全表遍历

来源：[crates/style/src/selector.rs](../../crates/style/src/selector.rs)

```rust
/// 针对单个元素的匹配：找出所有匹配的规则声明
pub fn match_selectors(
    element: &ElementData,
    stylesheets: &[StyleSheet],
) -> Vec<MatchedDeclaration> {
    let mut results = Vec::new();
    // ★ 外循环：遍历样式表
    for sheet in stylesheets {
        // ★ 中层：遍历规则
        for rule in &sheet.rules {
            // ★ 内循环：遍历选择器
            for selector in &rule.selectors {
                if element_matches_selector(element, selector) {
                    let specificity = compute_specificity(selector);
                    for decl in &rule.declarations {
                        results.push(MatchedDeclaration { specificity, declaration: decl.clone() });
                    }
                }
            }
        }
    }
    results
}
```

**复杂度**：`O(Elements × Rules × SelectorsPerRule)` — 500 元素 × 50 规则 × 2 选择器 = 50,000 次匹配检查。

### 选择器解析：手写字符级解析器

```rust
#[derive(PartialEq)]
enum Mode { Tag, Class, Id }

pub fn parse_selector_parts(selector: &str) -> Vec<SelectorPart> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut mode = Mode::Tag;
    let mut in_pseudo = false;
    let mut in_attr = false;

    for ch in selector.chars() {
        if in_attr {
            if ch == ']' { /* 结束属性 */ }
            else { attr_buffer.push(ch); }
            continue;
        }
        match ch {
            '.' if !in_pseudo => { flush_part_mode(&mut current, &mode, &mut parts); mode = Mode::Class; }
            '#' if !in_pseudo => { flush_part_mode(&mut current, &mode, &mut parts); mode = Mode::Id; }
            '[' if !in_pseudo => { flush_part_mode(...); in_attr = true; }
            ':' if !in_pseudo => { flush_part_mode(...); in_pseudo = true; }
            '(' if in_pseudo => { pseudo_depth += 1; current.push(ch); }
            ')' if in_pseudo => { /* 结束伪类 */ }
            _ => { current.push(ch); }
        }
    }
    // 处理剩余缓冲
    parts
}
```

### 选择器匹配：按部分逐项检查

```rust
/// 判断元素是否匹配选择器
pub fn element_matches_selector(element: &ElementData, selector: &str) -> bool {
    let selector = selector.trim();
    if selector == "*" { return true; }
    let parts = parse_selector_parts(selector);
    for part in parts {
        if !element_matches_selector_part(element, &part) { return false; }
    }
    true
}

fn element_matches_selector_part(element: &ElementData, part: &SelectorPart) -> bool {
    match part {
        SelectorPart::Tag(tag)     => element.tag_name() == tag.as_str(),
        SelectorPart::Class(class) => element.has_class(class),
        SelectorPart::Id(id)       => element.id() == Some(id),
        SelectorPart::Attribute { name, op } => match_attribute(element, name, op),
        SelectorPart::PseudoClass(pc) => matches_simple_pseudo_class(element, pc),
    }
}
```

### 复杂选择器匹配（Phase 3）：从左向右遍历

```rust
/// Phase 3: 检查节点是否匹配复杂选择器（含组合器）
pub fn matches_complex_selector(node: &Rc<RefCell<dom::Node>>, selector: &ParsedSelector) -> bool {
    let segments = &selector.segments;
    // ★ 最后一段（目标元素）先匹配
    if !matches_segment_parts(node, &segments.last().unwrap().parts) {
        return false;
    }
    // ★ 从右向左检查组合器
    let mut current = node.clone();
    for i in (0..segments.len() - 1).rev() {
        match segments[i + 1].combinator {
            Combinator::Descendant =>      current = find_ancestor_matching(&current, ...)?,
            Combinator::Child =>           current = find_parent_matching(&current, ...)?,
            Combinator::AdjacentSibling => current = find_prev_sibling_matching(&current, ...)?,
            Combinator::GeneralSibling =>  current = find_prev_sibling_general(&current, ...)?,
        }
    }
    true
}

// 后代组合器：沿祖先链向上查找
fn find_ancestor_matching(node: &Rc<RefCell<dom::Node>>, parts: &[SelectorPart]) -> Option<...> {
    let mut current = node.borrow().parent_node()?;
    loop {
        if matches_segment_parts(&current, parts) { return Some(current.clone()); }
        let parent = current.borrow().parent_node()?;
        current = parent;
    }
}
```

**特点**：
- 目标元素匹配后，沿 DOM 树向上验证祖先/兄弟
- 后代组合器的匹配沿祖先链线性扫描到根（可能触及大量无关祖先）
- 无快速拒绝机制：即使祖先链上没有匹配，也必须检查每一个父节点

### 选择器支持范围

| 选择器类型 | Phase 0 | Phase 2 | Phase 3 | 说明 |
|-----------|---------|---------|---------|------|
| 类型选择器 `div` | ✓ | ✓ | ✓ | 标签名精确匹配 |
| 类选择器 `.class` | ✓ | ✓ | ✓ | classList 包含检查 |
| ID 选择器 `#id` | ✓ | ✓ | ✓ | id 属性精确匹配 |
| 通配 `*` | ✓ | ✓ | ✓ | 始终匹配 |
| 属性选择器 `[attr=val]` | ❌ | ❌ | ✓ | 6 种操作符 |
| 伪类 `:hover/:focus/:nth-child` | ❌ | ✓ | ✓ | 20+ 伪类 |
| 组合器 ` ` `>` `+` `~` | ❌ | ❌ | ✓ | 4 种组合器 |
| `::before` / `::after` | ❌ | ❌ | ✓ | 伪元素生成 |
| `:has()`, `:is()`, `:where()` | ❌ | ❌ | ✓ | 关系伪类 |
| `:not()` | ❌ | ❌ | ✓ | 否定伪类 |

### 特异性计算

```rust
pub fn compute_specificity(selector: &str) -> (u32, u32, u32) {
    let parts = parse_selector_parts(selector);
    let mut id_count = 0u32;
    let mut class_count = 0u32;
    let mut tag_count = 0u32;
    for part in parts {
        match part {
            SelectorPart::Id(_)             => id_count += 1,
            SelectorPart::Class(_)          => class_count += 1,
            SelectorPart::Attribute { .. }  => class_count += 1,
            SelectorPart::PseudoClass(_)    => class_count += 1,
            SelectorPart::Tag(_)            => tag_count += 1,
        }
    }
    (id_count, class_count, tag_count)
}
```

**级联排序**：`(id_count, class_count, tag_count)` 元组比较，`!important` 最高，内联样式 `(u32::MAX, u32::MAX, u32::MAX)` 次之。

## Blink 的实现

### SelectorChecker：从右向左匹配

```cpp
// Blink 的匹配方向是从右向左（RTL）
// 原因：最后一段（目标元素）最具区分度，先匹配可以有效剪枝
class SelectorChecker {
public:
    enum MatchResult { kMatches, kNotMatches, kMatchesWithVisited };

    MatchResult MatchSelector(
        const CSSSelector& selector,
        const Element* element,
        const SelectorChecker::SelectorCheckingContext& context) {

        // 1. ★ 从最后一个关系段开始匹配目标元素
        const CSSSelector* current = &selector;
        while (current->TagHistory() != nullptr)
            current = current->TagHistory();  // 链表遍历到最右段

        // 2. ★ 匹配目标元素（快速检查 tag/id/class）
        if (!CheckOneSelectorContext(*current, element, context))
            return kNotMatches;

        // 3. ★ 从右向左沿组合器链向上匹配
        while (current != &selector) {
            current = current->PreviousInTagHistory();  // 向左一步

            switch (current->Relation()) {
                case CSSSelector::kSubSelector:
                    // 复合选择器（如 div.class#id）
                    if (!CheckOne(context, element))
                        return kNotMatches;
                    break;

                case CSSSelector::kDescendant:
                    // 后代组合器：向上查找匹配祖先
                    element = FindMatchingAncestor(element, current);
                    if (!element) return kNotMatches;
                    break;

                case CSSSelector::kChild:
                    // 子代组合器：父元素必须匹配
                    element = element->parentElement();
                    if (!element || !CheckOne(context, element))
                        return kNotMatches;
                    break;

                case CSSSelector::kDirectAdjacent:
                    // 相邻兄弟：前一个兄弟必须匹配
                    element = element->previousElementSibling();
                    if (!element || !CheckOne(context, element))
                        return kNotMatches;
                    break;

                case CSSSelector::kIndirectAdjacent:
                    // 通用兄弟：任意前兄弟
                    element = FindPreviousSiblingMatching(element, current);
                    if (!element) return kNotMatches;
                    break;
            }
        }
        return kMatches;
    }
};
```

### SelectorFilter：Bloom Filter 快速拒绝

```cpp
// Blink 的核心优化：SelectorFilter
// 为每个 CSS 规则的最右段选择器建立 Bloom Filter
// 匹配时先用 Bloom Filter 快速拒绝"完全不可能匹配"的元素
class SelectorFilter {
public:
    // 收集此元素的所有可能匹配特征
    void CollectFeatures(Element* element) {
        features_.clear();
        // 记录 tag name hash
        features_.push_back(element->TagQName().LocalName().Hash());
        // 记录所有 class hash
        for (const AtomicString& cls : element->ClassNames())
            features_.push_back(cls.Hash());
    }

    // 快速检查：此选择器的最右段是否可能匹配此元素？
    bool FastReject(const CSSSelector& selector) {
        const CSSSelector* rightmost = GetRightmostSelector(selector);
        // 检查 Bloom Filter 中是否存在需要的特征
        for (const auto& feature : rightmost->RequiredFeatures()) {
            if (!features_.contains(feature))
                return true;  // ★ 缺少必要特征 → 肯定不匹配，跳过
        }
        return false;  // 可能匹配 → 进入完整 SelectorChecker
    }
};
```

**Bloom Filter 效果**：在样式重算时，60-80% 的规则在 FastReject 阶段被跳过，只有 20-40% 进入完整的 SelectorChecker。

### CSSSelector 数据结构：链表而非数组

```cpp
// Blink 的选择器使用单向链表（而非数组）
// 每个节点包含：类型 (tag/id/class)、具体值、组合器关系、下一个节点的指针
class CSSSelector {
public:
    MatchType Match() const;               // kTag, kClass, kId, kPseudoClass...
    const AtomicString& Value() const;     // 选择器值
    RelationType Relation() const;         // kSubSelector, kDescendant, kChild...

    // ★ 链表遍历
    const CSSSelector* TagHistory() const;            // 下一个（RTL 方向）
    const CSSSelector* PreviousInTagHistory() const;  // 上一个（LTR 方向）

    // ★ 伪类解析
    CSSSelector::PseudoType GetPseudoType() const;
    const CSSSelector* SelectorList() const;          // :is() / :not() 内嵌列表

private:
    // 紧凑位域存储
    unsigned match_: 4;
    unsigned relation_: 4;
    unsigned pseudo_type_: 8;
    // ...
};
```

**与我们的对比**：
- 我们：`Vec<SelectorPart>` 数组（堆分配，O(1) 随机访问）
- Blink：`TagHistory` 链表（无堆分配，选择器解析时构建）

### 按类型分发：编译期 ID 枚举

```cpp
// Blink 不解析字符串，而是使用编译期枚举
enum CSSPropertyID : uint16_t {
    kColor = 1,
    kFontSize = 2,
    kDisplay = 3,
    // ... 500+ ID
};

// 选择器匹配也类似 —— CSSSelector::Match() 返回枚举而非字符串
switch (selector.Match()) {
    case CSSSelector::kTag:
        return element->TagQName() == selector.TagQName();
    case CSSSelector::kClass:
        return element->HasClass(selector.Value());
    case CSSSelector::kId:
        return element->Id() == selector.Value();
    case CSSSelector::kPseudoClass:
        return CheckPseudoClass(selector.GetPseudoType(), element);
}
```

### 从右向左的优势

```
选择器: "div.container > ul li.active"
RTL 匹配过程:
  Step 1: 检查目标元素是否匹配 "li.active" (class + tag)
    → 不匹配? 立即退出（无须检查祖先）
  Step 2: 检查父/祖先是否存在 "ul"
  Step 3: 检查 "ul" 的父元素是否为 "div.container"

N 个元素中，只有 ~5% 是 <li>，~0.5% 满足 li.active
RTL 从最具区分度的部分开始 → 95% 以上元素在 Step 1 被剪枝
```

## 优劣势分析

| 维度 | 我们 (手写解析 + 全表遍历) | Blink (SelectorChecker + Filter) |
|------|--------------------------|-------------------------------|
| **解析方式** | 字符级手写解析器（~300 行） | CSS 解析器输出 AST → CSSSelector 链表 |
| **匹配方向** | 从左向右（目标元素 → 祖先） | 从右向左（最右段 → 祖先） |
| **快速拒绝** | 无 | Bloom Filter (FastReject 跳过 60-80% 规则) |
| **遍历策略** | 三重循环：sheets → rules → selectors | 双重：rules → SelectorChecker（elements 在外层） |
| **匹配方式** | 选择器字符串 → 每次解析 → 逐部分匹配 | 预编译 CSSSelector 链表 → 按类型 dispatch |
| **组合器匹配** | 祖先链线性扫描（可能到根） | 精确的 N 层向上查找 |
| **伪类支持** | 20+ 伪类（字符串匹配分发） | 50+ 伪类（枚举 ID 分发 + 专用检查器） |
| **选择器存储** | `Vec<SelectorPart>` 堆分配 | CSSSelector 链表（连续内存 + 位域） |
| **特异性计算** | 重新解析选择器字符串 | CSSSelector 节点预计算 |

### 我们的优势

1. **Phase 0 可工作且代码量小**：1200 行实现 tag/class/id + 组合器 + 伪类 + 属性 + `:has/:is/:where`
2. **解析简单直观**：字符逐字节解析，无需外部依赖
3. **`Vec<SelectorPart>` 易调试**：`#[derive(Debug, Clone)]` 一键获得
4. **覆盖度高**：20+ 伪类、6 种属性操作符、4 种组合器，远超 MVP 需求
5. **`:has()` / `:is()` / `:where()` 已实现**：这些现代选择器很多引擎都未完整支持

### 我们的劣势

1. **O(Elements × Rules × Selectors)** 无剪枝：每个元素检查每条规则的每个选择器
2. **选择器字符串每次匹配都解析**：如果同一条规则匹配 100 个元素，`parse_selector_parts` 调用 100 次
3. **从右向左匹配不完整**：组合器解析后的 `matches_complex_selector` 虽然从右向左检查，但祖先查找是线性扫描到根的
4. **无 BloomFilter 快速拒绝**：`.todo-item.completed` 规则对每个元素都进入完整 `element_matches_selector`（包括明显不匹配的 `<h1>`, `<input>`, `<button>`）
5. **选择器未预编译**：每次 `match_selectors` 都对选择器字符串调用 `parse_selector_parts`
6. **无规则分组索引**：所有规则一视同仁，无按最右段 Tag/Class/Id 分组

### Blink 的优势

1. **BloomFilter 快速拒绝**：仅检查必要特征（tag hash + class hash），60-80% 规则跳过
2. **RTL 天然剪枝**：从最具区分度的右段开始，不匹配立即退出
3. **编译期 ID 枚举**：选择器类型和属性名都是枚举，无字符串比较
4. **预编译选择器链**：CSS 规则解析后存储为 CSSSelector 链表，匹配时无需解析
5. **规则索引**：规则按最右段类型（ID / Class / Tag / Universal）分组，先查对应索引

### Blink 的劣势

1. **代码量巨大**：SelectChecker 5000+ 行，SelectorFilter 1000+ 行，CSSSelector 数据结构 800+ 行
2. **概念复杂**：Bloom Filter + TagHistory 链表 + 多级 SelectorCheckingContext
3. **内存布局耦合**：位域打包 + 连续分配的约束使选择器修改困难

## 改进方向

### 短期

1. **预编译选择器**：将 `parse_selector_parts` 结果缓存在 `StyleSheet` 规则中，避免重复解析
2. **从右向左剪枝**：在 `match_selectors` 循环中，先检查最右段是否匹配当前元素的 tag/id/class
3. **规则按类型分组**：建立 `id_rules: HashMap<String, Vec<Rule>>` / `class_rules: HashMap<String, Vec<Rule>>` / `tag_rules: HashMap<String, Vec<Rule>>` 索引

### 中期

4. **简单 Bloom Filter**：为每个元素收集 tag hash + class hashes，为每条规则最右段建立 feature 集合
   ```rust
   struct SelectorFilter {
       element_tag_hash: u64,
       element_class_hashes: HashSet<u64>,
   }
   impl SelectorFilter {
       fn fast_reject(&self, rule: &CssRule) -> bool {
           // 规则最右段要求 .completed? 但元素无此 class? → 拒绝
           if let Some(class) = rule.rightmost_class() {
               if !self.element_class_hashes.contains(&hash(class)) { return true; }
           }
           false
       }
   }
   ```
5. **`:has()` 索引优化**：`:has()` 当前是 O(N²) 的子树遍历，可缓存后代 tag/class 集合

### 长期

6. **cssparser 集成**：用 Mozilla 的 `cssparser` crate 替换手写解析器
7. **swc CSS 解析**：如果引入 JS 编译，同时用 swc 的 CSS 解析（AST 模式）
8. **选择器编译为闭包**：`div.container > ul li.active` → `|elem| { elem.tag == "li" && elem.has_class("active") && ancestor_matches(elem, |a| a.tag == "ul" && parent_matches(...)) }`（JIT 风格）
