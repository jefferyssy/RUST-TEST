//! 应用插件接口。
//!
//! 第三方 crate 实现 [`Plugin`] trait，通过
//! [`AppRegistry::loadPlugins`](crate::AppRegistry::loadPlugins) 注册，
//! 在 `init()` 末尾自动启动。

use crate::DomRegistry;

/// 应用插件接口。
///
/// # 使用
///
/// ```rust,ignore
/// use runtime::{AppRegistry, Plugin, DomRegistry};
///
/// struct MyTool;
/// impl Plugin for MyTool {
///     fn name(&self) -> &'static str { "my-tool" }
///     fn start(&self, doc: &DomRegistry) {
///         // 启动逻辑...
///     }
/// }
///
/// let mut app = AppRegistry::new();
/// app.loadPlugins(MyTool);
/// // init() 末尾会自动调用 MyTool.start()
/// app.init(|doc| { /* ... */ });
/// ```
pub trait Plugin {
    /// 插件名称（用于日志/调试）。
    fn name(&self) -> &'static str;

    /// 插件启动回调。
    ///
    /// 由 `AppRegistry::init()` 在 DOM 加载、样式计算、
    /// JS 事件处理器注册完毕后自动调用。
    fn start(&self, _document: &DomRegistry) {}
}
