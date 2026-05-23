//! # paint crate — DisplayList 绘制命令
//!
//! 定义渲染引擎消费的绘制命令类型。
//! 负责将 LayoutTree 转换为 PaintCommand 列表。
//! 独立模块，不依赖具体渲染后端（wgpu/webgpu）。

pub mod command;
pub mod builder;
pub mod optimizer;

pub use command::{
    PaintCommand, DisplayList,
    BorderStyle, TextDecoration, ObjectFit,
};
pub use builder::DisplayListBuilder;
pub use optimizer::BatchOptimizer;
