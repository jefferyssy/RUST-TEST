//! app_registry — 统一应用入口。
//!
//! 包装 DomRegistry + StyleEngine，提供链式 API：
//!
//! ```rust,ignore
//! let mut app = AppRegistry::new();
//! app.loadNodes(html::build_vnode());
//! app.loadStyles(html::create_style_sheets());
//! app.init(|doc| { app::setup_handlers(doc); });
//! ```
//!
//! 插件在 `init()` 末尾自动启动，无需手动调用。

use crate::{CssStyleSheet, DomRegistry, Plugin, VNode};
use crate::style_manager::StyleEngine;

/// 应用入口：统一管理 DOM 注册表 + 样式引擎 + 插件系统。
///
/// ```rust,ignore
/// let mut app = AppRegistry::new();
/// app.loadPlugins(DevToolsPlugin::default());
/// app.loadNodes(vnode);
/// app.loadStyles(sheets);
/// app.init(|doc| { /* JS 事件绑定 + 插件自动启动 */ });
/// ```
pub struct AppRegistry {
    document: DomRegistry,
    style_engine: StyleEngine,
    plugins: Vec<Box<dyn Plugin>>,
}

impl AppRegistry {
    /// 创建空应用。
    pub fn new() -> Self {
        Self {
            document: DomRegistry::new(),
            style_engine: StyleEngine::new(),
            plugins: Vec::new(),
        }
    }

    /// 加载 VNode 树到 DOM。
    ///
    /// 返回 `&mut Self` 支持链式调用。
    pub fn loadNodes(&mut self, vnode: VNode) -> &mut Self {
        self.document.loadVnode(vnode);
        self
    }

    /// 加载样式表到 document.styleSheets。
    ///
    /// 接受 `Vec<CssStyleSheet>` 或 `[CssStyleSheet; N]` 数组。
    pub fn loadStyles(
        &mut self,
        sheets: impl IntoIterator<Item = CssStyleSheet>,
    ) -> &mut Self {
        for sheet in sheets {
            self.document.addStyleSheet(sheet);
        }
        self
    }

    /// 初始化：重建样式索引 → 计算样式 → 执行回调 → 启动插件。
    ///
    /// `on_ready` 通常在此时注册 JS 事件处理器。
    /// 所有已注册的插件在回调执行完毕后自动启动。
    pub fn init(&mut self, on_ready: impl FnOnce(&mut DomRegistry)) {
        self.style_engine.rebuildIndices(&self.document.styleSheets);
        if let Some(body_id) = self.document.bodyId {
            self.document.markAllDirty(body_id);
        }
        self.style_engine.flushStyleDirty(&mut self.document);
        on_ready(&mut self.document);
        // 自动启动插件
        for plugin in &self.plugins {
            plugin.start(&self.document);
        }
    }

    /// 加载插件。
    ///
    /// 插件在 `init()` 末尾按注册顺序自动启动。
    pub fn loadPlugins(&mut self, plugin: impl Plugin + 'static) -> &mut Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// 获取内部 DomRegistry 的不可变引用（供 DevTools 等外部工具使用）。
    pub fn document(&self) -> &DomRegistry {
        &self.document
    }
}

impl Default for AppRegistry {
    fn default() -> Self {
        Self::new()
    }
}
