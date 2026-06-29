//! DevTools 仪表盘配置。

/// DevTools 仪表盘配置。
#[derive(Debug, Clone)]
pub struct DevConfig {
    /// HTTP 服务端口（默认 9876）
    pub port: u16,
    /// 是否自动打开浏览器（默认 true）
    pub auto_open: bool,
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            port: 9876,
            auto_open: true,
        }
    }
}
