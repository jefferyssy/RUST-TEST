//! # storage crate — 存储层 (Phase 2)
//!
//! 提供 Web Storage API 实现（localStorage / sessionStorage）。
//! Phase 2: 内存存储 + 文件持久化存根。
//! Phase 3+: 完整 IndexedDB 支持。

pub mod local_storage;

pub use local_storage::{StorageBackend, LocalStorage, SessionStorage};
