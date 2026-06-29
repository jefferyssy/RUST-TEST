//! devtools — 可视化调试工具。
//!
//! 提供 DOM 树打印、JSON 序列化、DevTools HTTP 仪表盘。
//! 推荐使用 [`DevToolsPlugin`] 通过 `AppRegistry::register_plugin()` 注册。

#![allow(non_snake_case)]

pub mod printer;
pub mod serialize;
pub mod dev_server;
pub mod dev_config;
pub mod app_ext;
pub mod plugin;

pub use printer::print_dom_tree;
pub use serialize::serialize_tree;
pub use dev_server::{start_dev_server, open_browser};
pub use dev_config::DevConfig;
pub use plugin::DevToolsPlugin;

#[deprecated(note = "请使用 DevToolsPlugin + AppRegistry::register_plugin()")]
pub use app_ext::AppRegistryExt;
