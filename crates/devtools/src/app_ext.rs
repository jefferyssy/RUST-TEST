//! AppRegistry 调试扩展 trait。

use runtime::{AppRegistry};

use crate::{serialize::serialize_tree, dev_server::{start_dev_server, open_browser}, DevConfig};

/// AppRegistry 的 DevTools 扩展方法。
pub trait AppRegistryExt {
    /// 启动 DevTools 仪表盘 HTTP 服务。
    ///
    /// 会阻塞直到用户输入 Enter 或超时。
    fn open_dev_server(&self, config: DevConfig);
}

impl AppRegistryExt for AppRegistry {
    fn open_dev_server(&self, config: DevConfig) {
        let json = serialize_tree(self.document());
        start_dev_server(&json, config.port);
        if config.auto_open {
            let url = format!("http://localhost:{}", config.port);
            open_browser(&url);
        }
        println!("\n按 Enter 退出...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
    }
}
