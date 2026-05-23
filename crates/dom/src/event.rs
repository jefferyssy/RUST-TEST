//! Event 事件系统
//!
//! 对应 W3C Event / EventTarget 接口

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::Node;

/// 全局递增监听器 ID
pub(crate) fn next_listener_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// 事件监听器
pub struct EventListener {
    pub callback: Box<dyn Fn(&Event)>,
    pub id: usize,
    /// Phase 1: 监听器选项
    pub options: EventListenerOptions,
}

/// 事件监听器选项 —— W3C AddEventListenerOptions
#[derive(Debug, Clone)]
pub struct EventListenerOptions {
    /// 是否在捕获阶段触发（默认 false = 冒泡阶段）
    pub capture: bool,
    /// 是否只触发一次后自动移除
    pub once: bool,
    /// 是否永不调用 preventDefault（滚动优化）
    pub passive: bool,
}

impl Default for EventListenerOptions {
    fn default() -> Self {
        Self {
            capture: false,
            once: false,
            passive: false,
        }
    }
}

// ============================================================
//  Event
// ============================================================

/// W3C Event 接口
pub struct Event {
    /// 事件类型："click", "mousedown" 等
    pub event_type: String,
    /// 原始触发目标
    pub target: RefCell<Option<Rc<RefCell<Node>>>>,
    /// 当前正在处理该事件的元素
    pub current_target: RefCell<Option<Rc<RefCell<Node>>>>,
    /// 是否冒泡
    pub bubbles: bool,
    /// 是否可取消
    pub cancelable: bool,
    default_prevented: Cell<bool>,
    propagation_stopped: Cell<bool>,
    immediate_propagation_stopped: Cell<bool>,
    /// 事件时间戳
    pub time_stamp: f64,
    /// Phase 1: 事件传播阶段
    pub event_phase: Cell<EventPhase>,
}

/// 事件传播阶段
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventPhase {
    /// 未派发
    None = 0,
    /// 捕获阶段：从 document_element 向下到 target 的父元素
    CapturingPhase = 1,
    /// 目标阶段：在事件目标元素上触发
    AtTarget = 2,
    /// 冒泡阶段：从 target 的父元素向上到 document_element
    BubblingPhase = 3,
}

impl Event {
    /// 创建新事件
    pub fn new(event_type: &str) -> Self {
        Self {
            event_type: event_type.to_string(),
            target: RefCell::new(None),
            current_target: RefCell::new(None),
            bubbles: true,
            cancelable: true,
            default_prevented: Cell::new(false),
            propagation_stopped: Cell::new(false),
            immediate_propagation_stopped: Cell::new(false),
            time_stamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as f64,
            event_phase: Cell::new(EventPhase::None),
        }
    }

    /// 阻止默认行为
    pub fn prevent_default(&self) {
        self.default_prevented.set(true);
    }

    /// 是否已阻止默认行为
    pub fn default_prevented(&self) -> bool {
        self.default_prevented.get()
    }

    /// 停止冒泡
    pub fn stop_propagation(&self) {
        self.propagation_stopped.set(true);
    }

    /// 是否已停止冒泡
    pub fn propagation_stopped(&self) -> bool {
        self.propagation_stopped.get()
    }

    /// 阻止同级其他监听器触发（比 stop_propagation 更强）
    pub fn stop_immediate_propagation(&self) {
        self.immediate_propagation_stopped.set(true);
        self.propagation_stopped.set(true);
    }

    /// 是否已停止立即传播
    pub fn immediate_propagation_stopped(&self) -> bool {
        self.immediate_propagation_stopped.get()
    }
}

// ============================================================
//  EventDispatcher —— 事件传播引擎 (Phase 1 核心新增)
// ============================================================

/// 事件派发器 —— 负责完整的事件传播流程
///
/// 传播流程：
/// 1. 构建事件路径（从 target 到 document_element 的祖先链）
/// 2. 捕获阶段：按路径自上而下触发 capture=true 的监听器
/// 3. 目标阶段：在 target 上触发所有监听器（无论 capture 值）
/// 4. 冒泡阶段：按路径自下而上触发 capture=false 的监听器（仅 bubbles=true）
/// 5. 任一阶段调用 stop_propagation 中止后续传播
pub struct EventDispatcher;

impl EventDispatcher {
    /// 派发事件到目标元素，执行完整传播过程
    /// 返回是否被 preventDefault
    pub fn dispatch(
        target: &Rc<RefCell<Node>>,
        event: &Event,
    ) -> bool {
        // 设置 target
        *event.target.borrow_mut() = Some(target.clone());

        // 构建祖先路径
        let path = Self::build_path(target);

        // 1. 捕获阶段 (自上而下)
        event.event_phase.set(EventPhase::CapturingPhase);
        for node_rc in path.iter().rev() {
            if event.propagation_stopped() {
                return event.default_prevented();
            }
            *event.current_target.borrow_mut() = Some(node_rc.clone());
            Self::fire_listeners_on_node(node_rc, event, true);
        }

        if event.propagation_stopped() {
            return event.default_prevented();
        }

        // 2. 目标阶段
        event.event_phase.set(EventPhase::AtTarget);
        *event.current_target.borrow_mut() = Some(target.clone());
        Self::fire_listeners_on_node(target, event, false);

        if event.propagation_stopped() {
            return event.default_prevented();
        }

        // 3. 冒泡阶段 (自下而上)
        if event.bubbles {
            event.event_phase.set(EventPhase::BubblingPhase);
            for node_rc in path.iter() {
                if event.propagation_stopped() {
                    break;
                }
                // 跳过 target（在目标阶段已处理）
                if Rc::as_ptr(node_rc) == Rc::as_ptr(target) {
                    continue;
                }
                *event.current_target.borrow_mut() = Some(node_rc.clone());
                Self::fire_listeners_on_node(node_rc, event, false);
            }
        }

        event.event_phase.set(EventPhase::None);
        event.default_prevented()
    }

    /// 构建从 target 到根（document_element）的祖先路径
    /// 返回: [target, parent, grandparent, ..., document_element]
    fn build_path(target: &Rc<RefCell<Node>>) -> Vec<Rc<RefCell<Node>>> {
        let mut path = vec![target.clone()];
        let mut current = target.borrow().parent_node();
        while let Some(parent) = current {
            path.push(parent.clone());
            current = parent.borrow().parent_node();
        }
        path
    }

    /// 在指定节点上触发匹配的监听器
    fn fire_listeners_on_node(
        node: &Rc<RefCell<Node>>,
        event: &Event,
        capture_phase: bool,
    ) {
        let node_ref = node.borrow();
        if let super::NodeType::Element(elem) = &node_ref.node_type {
            if let Some(listeners) = elem.events.get(&event.event_type) {
                // 克隆监听器列表以避免借用冲突
                let listeners: Vec<_> = listeners.iter().map(|l| l.id).collect();
                for id in listeners {
                    if event.immediate_propagation_stopped() {
                        break;
                    }
                    // 根据阶段过滤监听器
                    if let Some(listener) = elem.events.get(&event.event_type)
                        .and_then(|v| v.iter().find(|l| l.id == id))
                    {
                        let should_fire = if capture_phase {
                            listener.options.capture
                        } else {
                            !listener.options.capture
                        };
                        if should_fire {
                            (listener.callback)(event);
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
//  MouseEvent
// ============================================================

/// 鼠标事件 —— 继承 Event
pub struct MouseEvent {
    pub event: Event,
    pub client_x: f64,
    pub client_y: f64,
    pub button: i16,
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub meta_key: bool,
}

impl MouseEvent {
    /// 创建鼠标事件
    pub fn new(event_type: &str, x: f64, y: f64, button: i16) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = true;
        Self {
            event,
            client_x: x,
            client_y: y,
            button,
            alt_key: false,
            ctrl_key: false,
            shift_key: false,
            meta_key: false,
        }
    }
}

// ============================================================
//  KeyboardEvent (Phase 1)
// ============================================================

/// 键盘事件 —— 对应 W3C KeyboardEvent 接口
pub struct KeyboardEvent {
    /// 基础事件
    pub event: Event,
    /// 按键值："Enter", "a", "ArrowUp", "Escape"
    pub key: String,
    /// 物理键码："KeyA", "Digit1", "ArrowUp"
    pub code: String,
    /// 是否按下 Alt 键
    pub alt_key: bool,
    /// 是否按下 Ctrl 键
    pub ctrl_key: bool,
    /// 是否按下 Shift 键
    pub shift_key: bool,
    /// 是否按下 Meta 键（Win/Cmd）
    pub meta_key: bool,
    /// 是否长按重复触发
    pub repeat: bool,
}

impl KeyboardEvent {
    /// 创建键盘事件
    pub fn new(event_type: &str, key: &str, code: &str) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = true;
        Self {
            event,
            key: key.to_string(),
            code: code.to_string(),
            alt_key: false,
            ctrl_key: false,
            shift_key: false,
            meta_key: false,
            repeat: false,
        }
    }
}

// ============================================================
//  FocusEvent (Phase 1)
// ============================================================

/// 焦点事件 —— 对应 W3C FocusEvent 接口
pub struct FocusEvent {
    /// 基础事件
    pub event: Event,
    /// 关联的焦点转移目标
    pub related_target: Option<Rc<RefCell<Node>>>,
}

impl FocusEvent {
    pub fn new(event_type: &str) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = false;
        Self {
            event,
            related_target: None,
        }
    }
}

// ============================================================
//  WheelEvent (Phase 1)
// ============================================================

/// 滚轮事件 —— 对应 W3C WheelEvent 接口
pub struct WheelEvent {
    /// 基础事件
    pub event: Event,
    /// X 轴滚动量
    pub delta_x: f64,
    /// Y 轴滚动量
    pub delta_y: f64,
    /// Z 轴滚动量
    pub delta_z: f64,
    /// 滚动量的计算模式
    pub delta_mode: WheelDeltaMode,
}

/// 滚轮增量模式
#[derive(Debug, Clone, Copy)]
pub enum WheelDeltaMode {
    /// 像素模式
    Pixel = 0,
    /// 行模式
    Line = 1,
    /// 页模式
    Page = 2,
}

impl WheelEvent {
    pub fn new(
        event_type: &str,
        delta_x: f64,
        delta_y: f64,
        delta_mode: WheelDeltaMode,
    ) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = true;
        Self {
            event,
            delta_x,
            delta_y,
            delta_z: 0.0,
            delta_mode,
        }
    }
}

// ============================================================
//  AnimationEvent (Phase 2)
// ============================================================

/// 动画事件 —— 对应 W3C AnimationEvent 接口
pub struct AnimationEvent {
    pub event: Event,
    /// 动画名称
    pub animation_name: String,
    /// 动画已运行时间（秒）
    pub elapsed_time: f32,
}

impl AnimationEvent {
    pub fn new(event_type: &str, animation_name: &str, elapsed_time: f32) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = true;
        Self {
            event,
            animation_name: animation_name.to_string(),
            elapsed_time,
        }
    }
}

// ============================================================
//  TransitionEvent (Phase 2)
// ============================================================

/// 过渡事件 —— 对应 W3C TransitionEvent 接口
pub struct TransitionEvent {
    pub event: Event,
    /// 过渡属性名
    pub property_name: String,
    /// 过渡已运行时间（秒）
    pub elapsed_time: f32,
}

impl TransitionEvent {
    pub fn new(event_type: &str, property_name: &str, elapsed_time: f32) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = true;
        Self {
            event,
            property_name: property_name.to_string(),
            elapsed_time,
        }
    }
}

// ============================================================
//  InputEvent (Phase 2)
// ============================================================

/// 输入事件 —— 对应 W3C InputEvent 接口
pub struct InputEvent {
    pub event: Event,
    /// 插入的文本数据
    pub data: Option<String>,
    /// 输入类型："insertText", "insertFromPaste", "deleteContent" 等
    pub input_type: String,
    /// 是否是组合输入的一部分
    pub is_composing: bool,
}

impl InputEvent {
    pub fn new(event_type: &str, data: Option<&str>, input_type: &str) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = true;
        Self {
            event,
            data: data.map(|s| s.to_string()),
            input_type: input_type.to_string(),
            is_composing: false,
        }
    }
}

// ============================================================
//  Touch / TouchList / TouchEvent — 多点触控 (Phase 3)
// ============================================================

/// 单个触控点
#[derive(Debug, Clone)]
pub struct Touch {
    pub identifier: i32,
    pub client_x: f64,
    pub client_y: f64,
    pub page_x: f64,
    pub page_y: f64,
    pub screen_x: f64,
    pub screen_y: f64,
    pub force: f32,
    pub radius_x: f32,
    pub radius_y: f32,
}

impl Touch {
    pub fn new(identifier: i32, client_x: f64, client_y: f64) -> Self {
        Self {
            identifier,
            client_x,
            client_y,
            page_x: client_x,
            page_y: client_y,
            screen_x: client_x,
            screen_y: client_y,
            force: 1.0,
            radius_x: 10.0,
            radius_y: 10.0,
        }
    }
}

/// 触控点列表
#[derive(Debug, Clone)]
pub struct TouchList {
    touches: Vec<Touch>,
}

impl TouchList {
    pub fn new() -> Self {
        Self { touches: Vec::new() }
    }

    pub fn from(touches: Vec<Touch>) -> Self {
        Self { touches }
    }

    pub fn len(&self) -> usize {
        self.touches.len()
    }

    pub fn is_empty(&self) -> bool {
        self.touches.is_empty()
    }

    pub fn item(&self, index: usize) -> Option<&Touch> {
        self.touches.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Touch> {
        self.touches.iter()
    }
}

/// 触控事件
pub struct TouchEvent {
    pub event: Event,
    pub touches: TouchList,
    pub target_touches: TouchList,
    pub changed_touches: TouchList,
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub meta_key: bool,
}

impl TouchEvent {
    pub fn new(
        event_type: &str,
        touches: TouchList,
        target_touches: TouchList,
        changed_touches: TouchList,
    ) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = true;
        Self {
            event,
            touches,
            target_touches,
            changed_touches,
            alt_key: false,
            ctrl_key: false,
            shift_key: false,
            meta_key: false,
        }
    }
}

// ============================================================
//  PointerEvent — 统一鼠标+触控+笔 (Phase 3)
// ============================================================

/// 指针类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerType {
    Mouse,
    Touch,
    Pen,
}

/// 指针事件 —— 对应 W3C PointerEvent 接口
pub struct PointerEvent {
    pub event: Event,
    pub pointer_id: i32,
    pub pointer_type: PointerType,
    pub client_x: f64,
    pub client_y: f64,
    pub page_x: f64,
    pub page_y: f64,
    pub screen_x: f64,
    pub screen_y: f64,
    pub pressure: f32,
    pub width: f32,
    pub height: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub meta_key: bool,
    pub button: i16,
    pub buttons: u16,
    pub is_primary: bool,
}

impl PointerEvent {
    pub fn new(
        event_type: &str,
        pointer_id: i32,
        pointer_type: PointerType,
        client_x: f64,
        client_y: f64,
    ) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = true;
        Self {
            event,
            pointer_id,
            pointer_type,
            client_x,
            client_y,
            page_x: client_x,
            page_y: client_y,
            screen_x: client_x,
            screen_y: client_y,
            pressure: if matches!(pointer_type, PointerType::Mouse) { 0.5 } else { 1.0 },
            width: 1.0,
            height: 1.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            alt_key: false,
            ctrl_key: false,
            shift_key: false,
            meta_key: false,
            button: 0,
            buttons: 0,
            is_primary: true,
        }
    }

    /// 从 MouseEvent 转换
    pub fn from_mouse_event(event: &MouseEvent, pointer_id: i32) -> Self {
        let mut base = Event::new(&event.event.event_type);
        base.bubbles = event.event.bubbles;
        Self {
            event: base,
            pointer_id,
            pointer_type: PointerType::Mouse,
            client_x: event.client_x,
            client_y: event.client_y,
            page_x: event.client_x,
            page_y: event.client_y,
            screen_x: event.client_x,
            screen_y: event.client_y,
            pressure: 0.5,
            width: 1.0,
            height: 1.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            alt_key: event.alt_key,
            ctrl_key: event.ctrl_key,
            shift_key: event.shift_key,
            meta_key: event.meta_key,
            button: event.button,
            buttons: 0,
            is_primary: true,
        }
    }

    /// 从 Touch 转换
    pub fn from_touch(touch: &Touch, pointer_id: i32, event_type: &str) -> Self {
        Self::new(
            event_type,
            pointer_id,
            PointerType::Touch,
            touch.client_x,
            touch.client_y,
        )
    }
}

// ============================================================
//  CustomEvent — 自定义事件 (Phase 3)
// ============================================================

/// 自定义事件 —— 携带任意 detail 数据
pub struct CustomEvent {
    pub event: Event,
    pub detail: Option<String>,
}

impl CustomEvent {
    pub fn new(event_type: &str, detail: Option<&str>) -> Self {
        let mut event = Event::new(event_type);
        event.bubbles = true;
        Self {
            event,
            detail: detail.map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
#[path = "event.test.rs"]
mod tests;
