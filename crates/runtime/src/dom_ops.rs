//! dom_ops — NodeId 上的 DOM 树操作方法。
//!
//! 对标浏览器 `element.appendChild()` / `element.removeChild()` 等。
//! 所有方法第一个参数为 `&mut DomRegistry`（即 `&mut document`）。

use crate::{DomRegistry, NodeId};

/// NodeId 扩展：DOM 树操作（对标浏览器 element.xxx）。
pub trait DomNodeOps {
    /// 追加子节点（对标 `element.appendChild(child)`）。
    fn appendChild(self, document: &mut DomRegistry, child: NodeId);

    /// 移除子节点（对标 `element.removeChild(child)`）。
    fn removeChild(self, document: &mut DomRegistry, child: NodeId);

    /// 在参考节点前插入（对标 `element.insertBefore(newChild, refChild)`）。
    fn insertBefore(self, document: &mut DomRegistry, newChild: NodeId, refChild: NodeId);

    /// 设置属性（对标 `element.setAttribute(name, value)`）。
    fn setAttribute(self, document: &mut DomRegistry, name: &str, value: &str);

    /// 获取属性（对标 `element.getAttribute(name)`）。
    fn getAttribute(self, document: &DomRegistry, name: &str) -> Option<String>;

    /// 设置文本内容（对标 `element.textContent = value`）。
    fn setTextContent(self, document: &mut DomRegistry, text: &str);

    /// 获取文本内容（对标 `element.textContent`）。
    fn getTextContent(self, document: &DomRegistry) -> String;

    /// 添加事件监听器（对标 `element.addEventListener(type, handler)`）。
    fn addEventListener<F>(self, document: &DomRegistry, eventType: &str, _handler: F)
    where
        F: FnMut(&crate::Event) + 'static;

    /// 设置内联样式（对标 `element.style.setProperty(name, value)`）。
    fn setStyleProperty(self, document: &mut DomRegistry, name: &str, value: &str);
}

impl DomNodeOps for NodeId {
    fn appendChild(self, document: &mut DomRegistry, child: NodeId) {
        document.appendChild(self, child);
    }

    fn removeChild(self, document: &mut DomRegistry, child: NodeId) {
        document.removeChild(self, child);
    }

    fn insertBefore(self, document: &mut DomRegistry, newChild: NodeId, refChild: NodeId) {
        document.insertBefore(self, newChild, refChild);
    }

    fn setAttribute(self, document: &mut DomRegistry, name: &str, value: &str) {
        document.setAttribute(self, name, value);
    }

    fn getAttribute(self, document: &DomRegistry, name: &str) -> Option<String> {
        document.getAttribute(self, name).map(|s| s.to_string())
    }

    fn setTextContent(self, document: &mut DomRegistry, text: &str) {
        document.setTextContent(self, text);
    }

    fn getTextContent(self, document: &DomRegistry) -> String {
        document.getTextContent(self)
    }

    fn addEventListener<F>(self, _document: &DomRegistry, _eventType: &str, _handler: F)
    where
        F: FnMut(&crate::Event) + 'static,
    {
        // 一期：事件监听器占位，后续阶段实现完整的 EventTarget 机制
        // 对标浏览器 element.addEventListener(type, handler)
    }

    fn setStyleProperty(self, document: &mut DomRegistry, name: &str, value: &str) {
        // 通过 setAttribute("style", ...) 实现
        let existing = document.getAttribute(self, "style").unwrap_or_default();
        let new_style = format!("{}: {}; {}", name, value, existing);
        document.setAttribute(self, "style", &new_style);
    }
}
