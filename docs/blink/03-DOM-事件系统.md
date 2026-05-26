# 03 — DOM 事件系统：内联 HashMap 存储 vs 全局 EventTargetDataMap

## 问题/目标

DOM 事件系统需要支持 W3C EventTarget 接口：`addEventListener`、`removeEventListener`、`dispatchEvent`，以及事件传播模型（捕获 → 目标 → 冒泡）。

关键约束：
- 每个元素可以有任意数量的事件监听器，按事件类型分组
- 事件传播沿 DOM 树祖先链进行（捕获自顶向下，冒泡自底向上）
- 监听器可以在传播过程中被移除（一旦触发自动删除）或阻止后续传播
- 大多数元素没有事件监听器，空元素的存储开销应尽量小

## 我们的实现

### 事件存储：内联 HashMap

来源：[crates/dom/src/element.rs](../../crates/dom/src/element.rs#L15)

```rust
pub struct ElementData {
    pub(crate) tag_name: String,
    pub(crate) attributes: HashMap<String, String>,
    /// ★ 事件监听器直接存储在 ElementData 中
    pub(crate) events: HashMap<String, Vec<EventListener>>,
    pub(crate) focused: Cell<bool>,
    // ...
}

impl ElementData {
    pub fn new(tag_name: &str) -> Self {
        Self {
            tag_name: tag_name.to_lowercase(),
            attributes: HashMap::new(),
            events: HashMap::new(),      // ★ 每个元素都分配空 HashMap
            focused: Cell::new(false),
            // ...
        }
    }
}
```

**内存开销**：每个 `ElementData` 的 `events` HashMap 即使为空也占用 ~48 bytes（capacity + len + hasher）。

### 监听器注册

```rust
impl ElementData {
    pub fn add_event_listener_with_options(
        &mut self,
        event_type: &str,
        callback: Box<dyn Fn(&Event)>,
        options: EventListenerOptions,
    ) -> usize {
        let id = next_listener_id();   // ★ 全局原子计数器
        let listener = EventListener {
            callback,
            id,
            options,
        };
        self.events
            .entry(event_type.to_string())
            .or_default()
            .push(listener);
        id                               // ★ 返回 ID 用于 removeEventListener
    }

    pub fn remove_event_listener(&mut self, event_type: &str, id: usize) {
        if let Some(listeners) = self.events.get_mut(event_type) {
            listeners.retain(|l| l.id != id);
        }
    }
}
```

### 事件派发（三阶段传播）

来源：[crates/dom/src/event.rs](../../crates/dom/src/event.rs#L153)

```rust
impl EventDispatcher {
    /// 派发事件到目标元素，执行三阶段传播
    pub fn dispatch(target: &Rc<RefCell<Node>>, event: &Event) -> bool {
        *event.target.borrow_mut() = Some(target.clone());

        // 1. 构建祖先路径 [target, parent, grandparent, ..., document_element]
        let path = Self::build_path(target);

        // 2. 捕获阶段：自顶向下（👑 头 → 尾），只触发 capture=true 的监听器
        event.event_phase.set(EventPhase::CapturingPhase);
        for node_rc in path.iter().rev() {
            if event.propagation_stopped() { return event.default_prevented(); }
            *event.current_target.borrow_mut() = Some(node_rc.clone());
            Self::fire_listeners_on_node(node_rc, event, true);  // capture=true
        }

        // 3. 目标阶段：在 target 上触发所有监听器（忽略 capture 标志）
        event.event_phase.set(EventPhase::AtTarget);
        *event.current_target.borrow_mut() = Some(target.clone());
        Self::fire_listeners_on_node(target, event, false);

        // 4. 冒泡阶段：自底向上，跳过 target（已在目标阶段处理）
        if event.bubbles {
            event.event_phase.set(EventPhase::BubblingPhase);
            for node_rc in path.iter() {
                if event.propagation_stopped() { break; }
                if Rc::as_ptr(node_rc) == Rc::as_ptr(target) { continue; }  // ★ 跳过 target
                *event.current_target.borrow_mut() = Some(node_rc.clone());
                Self::fire_listeners_on_node(node_rc, event, false);  // capture=false
            }
        }

        event.default_prevented()
    }

    fn build_path(target: &Rc<RefCell<Node>>) -> Vec<Rc<RefCell<Node>>> {
        let mut path = vec![target.clone()];
        let mut current = target.borrow().parent_node();
        while let Some(parent) = current {
            path.push(parent.clone());
            current = parent.borrow().parent_node();
        }
        path  // [target, parent, grandparent, ..., document_element]
    }
}
```

### 事件类型体系

```rust
// ★ 每种事件类型是独立 struct，包含基础 Event
pub struct MouseEvent {
    pub event: Event,           // 组合而非继承
    pub client_x: f64,
    pub client_y: f64,
    pub button: i16,
    pub alt_key: bool, pub ctrl_key: bool, pub shift_key: bool, pub meta_key: bool,
}

pub struct KeyboardEvent { pub event: Event, pub key: String, pub code: String, ... }
pub struct WheelEvent { pub event: Event, pub delta_x: f64, pub delta_y: f64, ... }
pub struct TouchEvent { pub event: Event, pub touches: TouchList, ... }
pub struct PointerEvent { pub event: Event, pub pointer_id: i32, pub pointer_type: PointerType, ... }
pub struct CustomEvent { pub event: Event, pub detail: Option<String> }
pub struct AnimationEvent { pub event: Event, pub animation_name: String, pub elapsed_time: f32 }
pub struct TransitionEvent { pub event: Event, pub property_name: String, pub elapsed_time: f32 }
pub struct InputEvent { pub event: Event, pub data: Option<String>, pub input_type: String, ... }
pub struct FocusEvent { pub event: Event, pub related_target: Option<Rc<RefCell<Node>>> }
```

**事件传播控制**：
- `stop_propagation()` → 停止冒泡和捕获
- `stop_immediate_propagation()` → 停止冒泡 + 跳过同元素其余监听器
- `prevent_default()` → 阻止浏览器默认行为

### 使用方式：app.rs 中的事件处理

```rust
fn handle_click(&mut self) {
    // 1. Hit test — 找到点击的布局节点
    let hit = HitTester::hit_test(&self.layout_root, x, y);

    // 2. 获取对应 DOM 节点（若是文本节点向上查到元素）
    let target_node = hit.node;
    let dom_node = if is_text { target_node.parent_node() } else { target_node };

    // 3. 焦点管理
    if element.tag_name() == "input" { self.focused_element = Some(dom_node.clone()); }

    // 4. 派发事件（事件回调可能修改 DOM）
    let mouse_event = MouseEvent::new("click", x, y, 0);
    dom_node.borrow_mut().dispatch_event(&mouse_event.event);

    // 5. ★ 全量重建渲染管线
    self.relayout();
}
```

## Blink 的实现

### 事件存储：全局 EventTargetDataMap

```cpp
// Blink 的关键设计：大多数元素没有事件监听器
// 因此不直接在 Element 中存储事件数据，而是使用全局映射表

// ===== 惰性存储：有监听器时才分配 =====
class EventTarget {
public:
    bool AddEventListener(const AtomicString& type, EventListener* listener, ...);
    bool RemoveEventListener(const AtomicString& type, EventListener* listener, ...);
    bool DispatchEvent(Event& event);

    // ★ 懒加载：只有首次 addEventListener 时才分配 EventTargetData
    EventTargetData* GetEventTargetData();
    void EnsureEventTargetData();

private:
    // ★ 注意：不带事件数据成员
    // EventTargetData 存储在全局映射表或惰性分配
};

// ===== EventTargetData：仅在需要时存在 =====
class EventTargetData {
public:
    // 按事件类型分组的监听器列表
    using EventListenerMap = HeapHashMap<AtomicString, Member<RegisteredEventListenerVector>>;

    EventListenerMap& EventListeners() { return event_listeners_; }

private:
    // ★ 只有注册了监听器的元素才有此数据
    EventListenerMap event_listeners_;
};

// ===== 全局映射表（优化：避免每个 EventTarget 都分配 EventTargetData）=====
// Blink 实际使用 Member<EventTargetData> event_target_data_ 存储在
// EventTarget 子类中，但关键优化是惰性分配。
// 部分版本确实使用全局映射表存储"不常有的数据"。
```

### EventPath：路径预构建

```cpp
// Blink 的事件路径比我们更细粒度
// 包含 Shadow DOM 跨域、slot 分配、以及关闭影子树处理
class EventPath {
public:
    // 路径中的每一步
    struct EventContext {
        Node* node;
        EventTarget* current_target;
        bool is_target;
        // Shadow DOM 相关
        ShadowRoot* shadow_root;
        TreeScope* tree_scope;
    };

    void BuildPath(Node* target, Event& event) {
        // 1. 构建从 target 到根的元素路径
        HeapVector<Member<Node>> node_path;
        for (Node* node = target; node; node = node->parentNode()) {
            node_path.push_back(node);
        }

        // 2. 处理 Shadow DOM 边界（跨 shadow root）
        // 处理 slot 分配（替换 slot 为实际分配的元素）

        // 3. Touch target 调整（touch 事件的 target 不变）
    }

    const HeapVector<EventContext>& GetPath() const { return path_; }
};
```

### EventDispatcher：事件跨线程安全

```cpp
// Blink 的事件派发器比我们复杂得多
class EventDispatcher {
public:
    static DispatchEventResult DispatchEvent(Node& node, Event& event) {
        // 1. 如果不是 Window/Worker 则进入常规派发
        EventDispatcher dispatcher(node, event);
        return dispatcher.Dispatch();
    }

    DispatchEventResult Dispatch() {
        // 1. 构建 EventPath（含 Shadow DOM 处理）
        event_path_.BuildPath(node_, event_);

        // 2. 捕获阶段
        event_->SetEventPhase(Event::kCapturingPhase);
        for (auto& context : event_path_.GetPath().reversed()) {
            if (event_->PropagationStopped()) break;
            FireListenersAtTarget(context, Event::kCapturingPhase);
        }

        // 3. 目标阶段
        event_->SetEventPhase(Event::kAtTarget);
        FireListenersAtTarget(event_path_.GetTargetContext(), Event::kAtTarget);

        // 4. 冒泡阶段
        if (event_->bubbles()) {
            event_->SetEventPhase(Event::kBubblingPhase);
            for (auto& context : event_path_.GetPath()) {
                if (event_->PropagationStopped()) break;
                if (context.is_target) continue;
                FireListenersAtTarget(context, Event::kBubblingPhase);
            }
        }

        return event_->defaultPrevented() ? kCanceled : kNotCanceled;
    }
};
```

### 事件类型：继承体系而非组合

```cpp
// Blink 使用真正的类继承
class Event {
    bool bubbles_;
    bool cancelable_;
    Member<EventTarget> target_;
    Member<EventTarget> current_target_;
    // ...
};

class MouseEvent : public UIEvent {  // UIEvent → Event
    double client_x_;
    double client_y_;
    short button_;
    // ...
};

class KeyboardEvent : public UIEvent {
    String key_;
    String code_;
    // ...
};

class WheelEvent : public MouseEvent {  // 继承 MouseEvent
    double delta_x_;
    double delta_y_;
    // ...
};

// ★ 接口派发：通过虚函数
Event* Event::Create(const AtomicString& type, ...) {
    if (type == event_names::kClick) return MouseEvent::Create();
    if (type == event_names::kKeydown) return KeyboardEvent::Create();
    // ... 工厂模式
}
```

### 被动事件监听器（滚动性能优化）

```cpp
// Blink 的一个重要特性：passive event listeners
// 标记为 passive 的监听器保证不调用 preventDefault()
// 浏览器可以立即开始滚动而无需等待 JS 执行完毕
void EventTarget::AddEventListener(const AtomicString& type, EventListener* listener,
                                    AddEventListenerOptions* options) {
    if (options->passive()) {
        listener->SetPassive(true);   // ★ 标记为被动
        // 如果是 touch/wheel 事件 + passive → 合成器线程可立即滚动
    }
    EnsureEventTargetData()->AddEventListener(type, listener, options);
}

// 合成器线程检查
void CompositorThread::HandleInputEvent() {
    if (AllListenersArePassive(event)) {
        // ★ 无需等待主线程 JS 执行 → 直接开始滚动
        ScrollImmediately();
        MainThread::QueueEvent(event);
    } else {
        // 必须等待主线程确认
        MainThread::WaitForEventResult(event);
    }
}
```

## 优劣势分析

| 维度 | 我们 | Blink |
|------|------|-------|
| **事件存储** | 每个 ElementData 内联 `HashMap<事件类型, Vec<监听器>>` | 惰性分配 EventTargetData（无监听器 = 零开销） |
| **空元素开销** | ~48 bytes/element (空 HashMap) | ~0 bytes（无 EventTargetData 分配） |
| **事件类型** | Rust struct + 组合（`MouseEvent { event: Event }`） | C++ 类继承（`MouseEvent : UIEvent : Event`） |
| **事件传播** | EventDispatcher（完整三阶段） | EventDispatcher（三阶段 + ShadowDOM + 被动检测） |
| **监听器 ID** | 全局 AtomicUsize | 无（通过指针对比移除） |
| **passive 支持** | `EventListenerOptions.passive: bool`（已定义，编译用） | 完整运行时代码（编译用 + 合成器线程利用） |
| **跨线程安全** | 单线程，无需 | 多线程（主线程 + 合成器线程） |
| **事件类型数量** | 11 种事件 struct | 50+ 种事件类 |
| **Shadow DOM** | 不支持 | EventPath 支持跨影子树 |
| **Touch/Scroll 优化** | 无 | passive + 合成器线程快速路径 |

### 我们的优势

1. **代码极简**：`Event` ~130 行 + `EventDispatcher` ~100 行 + 事件类型 ~80 行/每种
2. **无继承的复杂度**：`MouseEvent { event: Event }` 组合式架构清晰
3. **监听器 ID 精确移除**：返回 `usize` 给调用方，与 JS 标准 `removeEventListener(name, fn)` 一致
4. **三阶段传播完整**：捕获 → 目标 → 冒泡，与 W3C 标准一致
5. **Rust 类型安全**：`Box<dyn Fn(&Event)>` 不会出现虚函数错误

### 我们的劣势

1. **每个元素都分配 events HashMap**：即使 99% 的元素没有任何事件监听器，仍消耗 ~48 bytes
2. **事件类型是组合而非继承**：无法做 `event as MouseEvent` 的多态派发
3. **无 passive 运行时优化**：`passive` 标记已存在但未被渲染引擎利用
4. **无 Shadow DOM 事件路径**：事件不能穿透 Shadow DOM 边界
5. **Clone 监听器列表导致额外分配**：`fire_listeners_on_node` 先收集 ID Vec，再逐个查找（两个间接层级）
6. **无事件类型工厂**：每次创建事件需要手动选择具体 struct

### Blink 的优势

1. **零开销惰性存储**：无监听器 = 不分配 EventTargetData
2. **passive 滚动优化**：合成器线程可跳过主线程直接滚动
3. **完善的事件类型层次**：继承链支持多态事件处理
4. **Shadow DOM 事件重定向**：事件路径自动处理影子边界
5. **合成器线程安全**：`postTask` 和消息传递机制

### Blink 的劣势

1. **代码量巨大**：EventDispatcher 1000+ 行，EventPath 600+ 行
2. **GC 耦合**：EventTargetData 必须是 GC 对象
3. **继承体系复杂**：`WheelEvent → MouseEvent → UIEvent → Event` — 4 层继承
4. **被动监听器检测开销**：每次 addEventListener 都做属性检查

## 改进方向

### 短期

1. **惰性分配 events HashMap**：将 `HashMap::new()` 改为 `Option<HashMap>`，首次 `addEventListener` 时才分配
   ```rust
   pub(crate) events: Option<HashMap<String, Vec<EventListener>>>,
   // 原本 48 bytes → 8 bytes (Option pointer) 对于 99% 无事件的元素
   ```
2. **避免 fire_listeners_on_node 中的二次分配**：直接在循环内处理而非先收集 ID

### 中期

3. **被动监听器实践应用**：在 scroll/touch 处理时检查 passive 标记，跳过主线程回调
4. **事件委托优化**：对于大量兄弟元素的相同事件，使用事件委托而非逐元素注册

### 长期

5. **事件类型 trait 抽象**：`trait DomEvent { fn event(&self) -> &Event; fn event_type() -> &'static str; }`
6. **合成器线程输入处理**：将 scroll/wheel 事件直接发送到合成器线程，满足 passive 条件时跳过主线程
