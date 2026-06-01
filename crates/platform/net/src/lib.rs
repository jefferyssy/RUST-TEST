//! # net crate — 网络层 (Phase 3)
//!
//! 提供 fetch API 和 WebSocket 实现。
//! Phase 3: Headers/Request 独立可构造 + fetch_async + WebSocket 重连/心跳。

pub mod fetch;
pub mod websocket;

pub use fetch::{
    FetchRequest, FetchResponse, FetchError, FetchMethod, fetch, fetch_async,
    FetchOptions, Headers, Request, RequestMode, RequestCredentials,
};
pub use websocket::{
    WebSocket, WebSocketState, WebSocketEvent,
    WebSocketConfig, ConnectionState, WebSocketError,
};
