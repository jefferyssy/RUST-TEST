# 02 — DOM 树内存管理：Rc<RefCell<Node>> vs Oilpan GC

## 问题/目标

DOM 树是浏览器的核心数据结构。它需要支持：
- 树结构的频繁增删改（appendChild、removeChild、insertBefore）
- 多个持有者同时引用同一个节点（JS 变量、事件回调、布局树、样式引擎）
- 内存安全（悬垂指针、循环引用、use-after-free）

## 我们的实现

### 核心模式：Rc<RefCell<Node>>

来源：[crates/dom/src/node.rs](../../crates/dom/src/node.rs)

```rust
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub struct Node {
    pub node_type: NodeType,
    /// 父节点（Weak 防循环引用）
    parent: Option<Weak<RefCell<Node>>>,
    /// 子节点列表（有序）
    pub(crate) children: Vec<Rc<RefCell<Node>>>,
    /// 前一个兄弟节点（Weak 防循环引用）
    prev_sibling: Option<Weak<RefCell<Node>>>,
    /// 后一个兄弟节点
    next_sibling: Option<Rc<RefCell<Node>>>,
    /// 变更标记
    pub(crate) dirty: Cell<bool>,
    /// 指向自身的 Weak 指针
    weak_self: Weak<RefCell<Node>>,
}

impl Node {
    /// 使用 Rc::new_cyclic 创建自引用节点
    pub fn new(node_type: NodeType) -> Rc<RefCell<Self>> {
        Rc::new_cyclic(|weak| RefCell::new(Self {
            node_type,
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            dirty: Cell::new(false),
            weak_self: weak.clone(),
        }))
    }
}
```

### 引用关系图

```
         Rc（强引用）           Weak（弱引用）
         ═══════════           -------------
         
  Document
    │ (Rc)
    ▼
  <html> ◄──────── Weak ──────┐
    │ (Rc)                     │
    ▼                          │
  <body> ◄─────── Weak ───────┤
    │ (Rc)                     │
    ├───────────┐              │
    ▼           ▼              │
  <div> ◄─── <span>           │
    │ Rc        │ Rc           │
    │           │              │
    │ parent (Weak) ──────────┘  ← 子→父永远是 Weak
    │
    │ prev_sibling (Weak) ──→ null
    │ next_sibling (Rc) ────→ <span>
    │
    ▼
  children: Vec<Rc<RefCell<Node>>>  ← 父→子永远是 Rc
```

### 树操作中的借用管理

```rust
// append_child —— 来自 crates/dom/src/node.rs
pub fn append_child(&mut self, child: Rc<RefCell<Node>>) {
    // 从原父节点移除
    {
        let child_ref = child.borrow();
        if let Some(parent_weak) = &child_ref.parent {
            if let Some(parent) = parent_weak.upgrade() {
                drop(child_ref);  // ★ 释放借用后再操作父节点
                parent.borrow_mut().remove_child_by_ptr(&child);
            }
        }
    }  // ★ 借用必须在此释放

    // 设置双向链接
    child.borrow_mut().parent = Some(self.weak_self.clone());
    // ... prev_sibling / next_sibling 维护 ...
    self.children.push(child);
    self.mark_dirty(true);
}
```

**关键约束**：
- `borrow()` 和 `borrow_mut()` 不能同时存在 → 运行时 panic
- 操作前必须手动 `drop(child_ref)` 释放借用
- 不能持有 `borrow()` 的同时调用同一节点的 `borrow_mut()`

## Blink 的实现

### Oilpan GC 基础

Oilpan 是 Blink 的**线程局部增量标记-清除垃圾收集器**，所有 DOM 对象继承 `GarbageCollected<T>`。

```cpp
// core/dom/node.h — Blink 的 Node 定义（简化）
class Node : public GarbageCollected<Node> {
public:
    Node* parentNode() const { return parent_; }
    Node* firstChild() const { return first_child_; }
    Node* lastChild() const { return last_child_; }
    Node* previousSibling() const { return previous_; }
    Node* nextSibling() const { return next_; }

protected:
    // 所有指针由 GC 管理
    Member<Node> parent_;        // 强引用，GC 可循环检测
    Member<Node> previous_;      // 强引用
    Member<Node> next_;          // 强引用
    Member<Node> first_child_;   // 强引用
    Member<Node> last_child_;    // 强引用
};

// GC Trace 方法 —— 必须手写，告诉 GC 哪些指针需要追踪
void Node::Trace(Visitor* visitor) const {
    visitor->Trace(parent_);
    visitor->Trace(previous_);
    visitor->Trace(next_);
    visitor->Trace(first_child_);
    visitor->Trace(last_child_);
}
```

### Oilpan 智能指针类型

```cpp
// ===== 堆内（GarbageCollected 对象内部）=====
class MyDOMClass : public GarbageCollected<MyDOMClass> {
    Member<OtherClass> strong_;        // 强引用（类似 Rc），GC 可检测循环
    WeakMember<OtherClass> weak_;      // 弱引用（类似 Weak），GC 后自动 null
    HeapVector<Member<Node>> children_; // 堆上分配的容器

    void Trace(Visitor* v) const {
        v->Trace(strong_);
        v->Trace(weak_);    // ★ WeakMember 也必须 trace（用于 GC 更新 null）
        v->Trace(children_);
    }
};

// ===== 堆外（非 GC 对象引用 GC 对象）=====
class NonGCClass {
    Persistent<SomeDOMClass> strong_;          // 根级强引用（阻止 GC）
    WeakPersistent<SomeDOMClass> weak_;        // 根级弱引用（不阻止 GC）
    CrossThreadPersistent<SomeDOMClass> cross_;// 跨线程持有
};
```

### 惰性属性存储

Blink 的一个关键内存优化：`Element` 不预先分配属性存储。

```cpp
// Element 的属性数据在首次 setAttribute 时才分配
class Element : public ContainerNode {
    // 不直接持有 attribute map，而是惰性分配
    Member<ElementData> element_data_;  // 初始为 nullptr

    void setAttribute(const QualifiedName& name, const AtomicString& value) {
        EnsureElementData();  // ★ 首次调用时分配 ElementData
        element_data_->SetAttribute(name, value);
    }
};

// ElementData 内部结构
class ElementData {
    // 属性很少时用数组（线性扫描比 HashMap 快）
    Vector<Attribute> attribute_vector_;    // 少量属性
    // 属性多时切换到 HashMap
    HashMap<QualifiedName, Attribute> attribute_map_;  // 大量属性
};
```

### GC 三色标记流程

```
根集（栈 + 全局 Persistent）
   │
   ▼
标记阶段（增量，可中断）:
  白色 → 灰色（根可达对象）
  灰色 → 黑色（成员已扫描）→ 其 Member 变为灰色
  重复直到无灰色对象
   │
   ▼
清除阶段（并发）:
  所有白色对象 → 不可达 → 回收
  黑色对象 → 保留
```

## 优劣势分析

### 我们的优势

| 优势 | 说明 |
|------|------|
| **零 GC 停顿** | 引用计数即时回收，无 stop-the-world 暂停 |
| **确定性析构** | `Rc` 计数归零立即执行 `Drop`，适合 RAII |
| **所有权清晰** | 编译期保证内存安全，无悬垂指针 |
| **无 Trace 手写** | 不需要为每个类手动实现 Trace 方法 |
| **单线程安全** | `Rc` 不是 `Send`，编译期杜绝多线程竞态 |

### 我们的劣势

| 劣势 | 说明 | 影响 |
|------|------|------|
| **RefCell 运行时开销** | 每次 `borrow()`/`borrow_mut()` 检查借用状态 | 微秒级 |
| **借用冲突风险** | 同一节点的 `borrow()` 和 `borrow_mut()` 同时存在会 panic | **开发时频繁遇到** |
| **引用计数原子操作** | 每次 clone/drop 触发原子增减 | 纳秒级 |
| **循环引用泄漏** | 如果忘记用 `Weak`，两个 `Rc` 互指 → 内存泄漏 | 需警惕 |
| **Weak 升级代价** | `upgrade()` 需要检查是否已回收 | 每次访问父节点 |
| **批量回收不能合批** | 删除 1000 个节点产生 1000 次析构 | 罕见场景 |

### Blink 的优势

| 优势 | 说明 |
|------|------|
| **循环引用天然安全** | GC 三色标记可检测并回收循环 |
| **无借用检查** | C++ 原生可变性，不用 RefCell |
| **惰性分配** | ElementData 在首次 setAttribute 时才分配 |
| **批量析构** | GC 周期批量回收，分摊成本 |
| **跨堆协作** | 与 V8 JS 堆统一 GC，跨堆引用安全 |

### Blink 的劣势

| 劣势 | 说明 |
|------|------|
| **GC 停顿不可控** | 增量 GC 仍可能触发 world-stop |
| **手写 Trace** | 每个 GC 类必须手写 Trace 方法 |
| **写屏障开销** | 每次 `Member<T>` 赋值触发写屏障 |
| **非确定性析构** | 不确定对象何时被回收 |

## 改进方向

### 短期（可行）

1. **减少 RefCell 借用范围**：当前代码模式 `drop(child_ref)` 已是最佳实践，但可进一步抽取"借用快照"模式
2. **用索引替代 Weak 引用**：兄弟节点链接用数组索引+偏移而非 Weak<Node>，减少 Weak 升级

### 中期（需评估）

3. **Arena 分配器**：为 DOM 节点引入 `typed_arena` crate，批量分配/释放，减少 Rc 数量
4. **Cell 替代 RefCell**：对简单字段（如 `dirty`）已使用 `Cell<bool>`，可扩展到更多字段

### 长期（架构级）

5. **自定义 GC**：参考 Servo 的 `dom::bindings::trace` trait，实现基于标记的 DOM 专用 GC
6. **集成 V8 GC**：如果未来引入 JS 运行时，考虑与 V8 的 Oilpan 集成
