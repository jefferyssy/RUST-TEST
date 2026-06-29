//! DevTools 仪表盘插件。
//!
//! 实现 `runtime::Plugin`，在应用启动时开启 HTTP 仪表盘服务。

use runtime::{DomRegistry, Plugin};

use crate::dev_config::DevConfig;
use crate::dev_server::{start_dev_server, open_browser};
use crate::serialize::serialize_tree;

/// DevTools 仪表盘插件。
///
/// # 使用
///
/// ```rust,ignore
/// use runtime::AppRegistry;
/// use devtools::DevToolsPlugin;
///
/// let mut app = AppRegistry::new();
/// app.loadPlugins(DevToolsPlugin::default());
/// // init() 末尾会自动启动插件
/// app.init(|doc| { /* ... */ });
/// ```
pub struct DevToolsPlugin {
    pub config: DevConfig,
}

impl Default for DevToolsPlugin {
    fn default() -> Self {
        Self {
            config: DevConfig::default(),
        }
    }
}

impl Plugin for DevToolsPlugin {
    fn name(&self) -> &'static str {
        "devtools"
    }

    fn start(&self, document: &DomRegistry) {
        let json = serialize_tree(document);
        start_dev_server(&json, self.config.port);
        if self.config.auto_open {
            let url = format!("http://localhost:{}", self.config.port);
            open_browser(&url);
        }
        println!("\n按 Enter 退出...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
    }
}
