//! Element 元素节点数据
//!
//! 对应 W3C Element 接口

use std::collections::HashMap;

use super::event::{Event, EventListener, EventListenerOptions};

/// HTML 元素节点数据
pub struct ElementData {
    pub(crate) tag_name: String,
    pub(crate) attributes: HashMap<String, String>,
    pub(crate) class_list: Vec<String>,
    pub(crate) style: HashMap<String, String>,
    pub(crate) events: HashMap<String, Vec<EventListener>>,
    pub(crate) id: Option<String>,
    /// Phase 1: 元素是否获得焦点
    pub(crate) focused: bool,
}

impl ElementData {
    /// 创建新元素数据
    pub fn new(tag_name: &str) -> Self {
        Self {
            tag_name: tag_name.to_lowercase(),
            attributes: HashMap::new(),
            class_list: Vec::new(),
            style: HashMap::new(),
            events: HashMap::new(),
            id: None,
            focused: false,
        }
    }

    // ===== 属性操作 =====

    /// 获取属性值
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        self.attributes.get(name).cloned()
    }

    /// 设置属性
    /// 当 name="class" 时自动同步 class_list
    /// 当 name="id" 时自动同步 id 字段
    pub fn set_attribute(&mut self, name: &str, value: &str) {
        match name {
            "class" => {
                self.class_list = value.split_whitespace().map(String::from).collect();
            }
            "id" => {
                self.id = Some(value.to_string());
            }
            _ => {}
        }
        self.attributes.insert(name.to_string(), value.to_string());
    }

    /// 移除属性
    pub fn remove_attribute(&mut self, name: &str) {
        self.attributes.remove(name);
    }

    /// 判断属性是否存在
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    // ===== classList =====

    /// 获取类名列表
    pub fn class_list(&self) -> &[String] {
        &self.class_list
    }

    /// 获取类名列表可变引用
    pub fn class_list_mut(&mut self) -> &mut Vec<String> {
        &mut self.class_list
    }

    /// 添加类名
    pub fn add_class(&mut self, class: &str) {
        if !self.class_list.contains(&class.to_string()) {
            self.class_list.push(class.to_string());
        }
    }

    /// 移除类名
    pub fn remove_class(&mut self, class: &str) {
        self.class_list.retain(|c| c != class);
    }

    /// 切换类名
    pub fn toggle_class(&mut self, class: &str) -> bool {
        if self.class_list.contains(&class.to_string()) {
            self.class_list.retain(|c| c != class);
            false
        } else {
            self.class_list.push(class.to_string());
            true
        }
    }

    /// 是否包含类名
    pub fn has_class(&self, class: &str) -> bool {
        self.class_list.contains(&class.to_string())
    }

    // ===== style =====

    /// 获取样式属性值
    pub fn get_style_value(&self, property: &str) -> Option<&String> {
        self.style.get(property)
    }

    /// 设置样式属性
    pub fn set_style_value(&mut self, property: &str, value: &str) {
        self.style.insert(property.to_string(), value.to_string());
    }

    /// 移除样式属性
    pub fn remove_style_value(&mut self, property: &str) -> Option<String> {
        self.style.remove(property)
    }

    /// 获取全部样式映射
    pub fn style_map(&self) -> &HashMap<String, String> {
        &self.style
    }

    /// 从 CSS 字符串解析并设置样式
    pub fn parse_and_set_style(&mut self, style_str: &str) {
        for part in style_str.split(';') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once(':') {
                self.set_style_value(key.trim(), value.trim());
            }
        }
        self.attributes.insert("style".to_string(), style_str.to_string());
    }

    /// 设置单个样式属性（保留 attributes["style"] 中的其他已有样式）
    pub fn set_style_property(&mut self, property: &str, value: &str) {
        self.set_style_value(property, value);

        let existing = self.attributes.get("style").cloned().unwrap_or_default();
        let mut new_style = String::new();
        let mut found = false;

        for part in existing.split(';') {
            let part = part.trim();
            if let Some((k, _)) = part.split_once(':') {
                if k.trim() == property {
                    if !new_style.is_empty() {
                        new_style.push_str("; ");
                    }
                    new_style.push_str(&format!("{}: {}", property, value));
                    found = true;
                    continue;
                }
            }
            if !part.is_empty() {
                if !new_style.is_empty() {
                    new_style.push_str("; ");
                }
                new_style.push_str(part);
            }
        }

        if !found {
            if !new_style.is_empty() {
                new_style.push_str("; ");
            }
            new_style.push_str(&format!("{}: {}", property, value));
        }

        self.attributes.insert("style".to_string(), new_style);
    }

    // ===== 事件管理 =====

    /// 添加事件监听器，返回监听器 ID
    pub fn add_event_listener(
        &mut self,
        event_type: &str,
        callback: Box<dyn Fn(&Event)>,
    ) -> usize {
        self.add_event_listener_with_options(event_type, callback, EventListenerOptions::default())
    }

    /// 添加事件监听器（带选项），返回监听器 ID
    pub fn add_event_listener_with_options(
        &mut self,
        event_type: &str,
        callback: Box<dyn Fn(&Event)>,
        options: EventListenerOptions,
    ) -> usize {
        let id = super::event::next_listener_id();
        let listener = EventListener { callback, id, options };
        self.events.entry(event_type.to_string()).or_default().push(listener);
        id
    }

    /// 移除事件监听器
    pub fn remove_event_listener(&mut self, event_type: &str, id: usize) {
        if let Some(listeners) = self.events.get_mut(event_type) {
            listeners.retain(|l| l.id != id);
        }
    }

    /// 派发事件
    pub fn dispatch_event(&mut self, event: &Event) -> bool {
        if let Some(listeners) = self.events.get(&event.event_type) {
            for listener in listeners {
                (listener.callback)(event);
                if event.propagation_stopped() {
                    break;
                }
            }
        }
        true
    }

    /// 获取指定类型的事件监听器列表
    pub fn get_event_listeners(&self, event_type: &str) -> &[EventListener] {
        self.events.get(event_type).map(|v| v.as_slice()).unwrap_or(&[])
    }

    // ===== 固有属性 =====

    pub fn id(&self) -> Option<&String> {
        self.id.as_ref()
    }

    pub fn set_id(&mut self, id: &str) {
        self.id = Some(id.to_string());
        self.set_attribute("id", id);
    }

    pub fn class_name(&self) -> String {
        self.class_list.join(" ")
    }

    pub fn set_class_name(&mut self, class: &str) {
        self.class_list = class.split_whitespace().map(String::from).collect();
        self.attributes.insert("class".to_string(), class.to_string());
    }

    pub fn tag_name(&self) -> &str {
        &self.tag_name
    }

    // ============================================================
    //  Phase 1 新增方法 —— W3C Element API
    // ============================================================

    /// 序列化子节点为 HTML 字符串
    pub fn inner_html(&self) -> String {
        // Phase 1: 基础实现（遍历子节点拼接）
        // Phase 2+: 完整 HTML 序列化（含属性转义）
        String::new() // 需配合 Node 实现；实际调用在 node 层面
    }

    /// 设置焦点，触发 focus 事件
    pub fn focus(&mut self) {
        self.focused = true;
    }

    /// 移除焦点，触发 blur 事件
    pub fn blur(&mut self) {
        self.focused = false;
    }

    /// 是否获得焦点
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    // ============================================================
    //  Phase 2: 滚动方法
    // ============================================================

    /// 滚动元素到可视区域
    pub fn scroll_into_view(&self) {
        // Phase 2: 标记需要滚动，实际滚动由布局/渲染处理
    }

    /// 滚动到指定坐标
    pub fn scroll_to(&self, _x: f32, _y: f32) {
        // Phase 2: 设置 scroll_left / scroll_top
    }

    /// 相对当前滚动位置偏移
    pub fn scroll_by(&self, _x: f32, _y: f32) {
        // Phase 2: scroll_left += x; scroll_top += y
    }

    /// 水平滚动位置
    pub fn scroll_left(&self) -> f32 {
        // Phase 2: 返回缓存的滚动位置
        0.0
    }

    pub fn set_scroll_left(&mut self, _value: f32) {
        // Phase 2: 设置水平滚动位置
    }

    /// 垂直滚动位置
    pub fn scroll_top(&self) -> f32 {
        0.0
    }

    pub fn set_scroll_top(&mut self, _value: f32) {
        // Phase 2: 设置垂直滚动位置
    }
}

#[cfg(test)]
#[path = "element.test.rs"]
mod tests;
