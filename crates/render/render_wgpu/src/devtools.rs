//! Chrome DevTools Protocol (CDP) 服务器
//!
//! 实现 CDP 最小子集，允许 Chrome DevTools 连接到运行中的应用检查 DOM 树和计算样式。
//!
//! 使用方式：
//! 1. 运行 demo 应用
//! 2. 打开 Chrome，地址栏输入 `chrome://inspect`
//! 3. 点击 "Configure..." 添加 `localhost:9222`
//! 4. 在 "Remote Target" 列表中看到目标，点击 "inspect"

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use dom::{Node, NodeType};
use layout::layout_box::LayoutBox;
use serde_json::{json, Value};
use style::{CSSValue, ComputedStyle};
use tungstenite::protocol::Role;
use tungstenite::WebSocket;

// ============================================================
//  公共接口
// ============================================================

/// 渲染线程持有的 DevTools 句柄
pub struct DevToolsHandle {
    /// CDP 页面快照：渲染线程写入，DevTools 线程读取
    pub snapshot: Arc<RwLock<Option<PageSnapshot>>>,
    /// CDP 事件队列：渲染线程 push，DevTools 线程 drain
    events: Arc<Mutex<Vec<String>>>,
    /// DOM 脏标志：relayout() 设置，about_to_wait() 消费后清除
    dirty: Arc<AtomicBool>,
}

/// 启动 DevTools CDP 服务器（独立线程），返回渲染线程用的句柄
pub fn start(port: u16) -> DevToolsHandle {
    let snapshot = Arc::new(RwLock::new(None));
    let events = Arc::new(Mutex::new(Vec::new()));
    let dirty = Arc::new(AtomicBool::new(true)); // 初始为 true，首帧推送

    let snap = snapshot.clone();
    let evts = events.clone();
    thread::Builder::new()
        .name("devtools-cdp".into())
        .spawn(move || run_server(snap, evts, port))
        .expect("DevTools CDP 线程启动失败");

    DevToolsHandle { snapshot, events, dirty }
}

impl DevToolsHandle {
    /// 推送 CDP 事件到所有 WebSocket 连接（由渲染线程调用）
    pub fn push_event(&self, event_json: &str) {
        if let Ok(mut queue) = self.events.lock() {
            queue.push(event_json.to_string());
            // 限制队列长度防止积压
            if queue.len() > 16 {
                queue.drain(0..8);
            }
        }
    }

    /// 标记 DOM 已变更（relayout 实际做了工作时调用）
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    /// 读取并清除脏标志（about_to_wait 中调用，决定是否推送事件）
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }
}

// ============================================================
//  页面快照（渲染线程每帧构建）
// ============================================================

/// 一帧的完整页面快照，供 CDP 查询使用
#[derive(Clone)]
pub struct PageSnapshot {
    /// CDP DOM.getDocument 返回的 root 节点（含所有子节点）
    dom_root: Value,
    /// nodeId → computed style 数组
    styles: HashMap<u32, Value>,
    /// nodeId → box model
    box_models: HashMap<u32, Value>,
    /// nodeId → DOM node 指针（用于 DOM.getNodeForLocation 的坐标系）
    node_layouts: HashMap<u32, dom::Rect<f32>>,
}

/// 渲染线程调用：遍历 DOM 树 + 布局树，构建 CDP 快照
pub fn build_snapshot(
    doc_element: &Rc<RefCell<Node>>,
    styles: &HashMap<usize, ComputedStyle>,
    layout_root: &LayoutBox,
) -> PageSnapshot {
    let mut id_counter: u32 = 1;
    let mut style_map: HashMap<u32, Value> = HashMap::new();
    let mut box_map: HashMap<u32, Value> = HashMap::new();
    let mut layout_map: HashMap<u32, dom::Rect<f32>> = HashMap::new();

    let dom_root = build_cdp_node(
        doc_element,
        styles,
        layout_root,
        &mut id_counter,
        &mut style_map,
        &mut box_map,
        &mut layout_map,
    );

    PageSnapshot {
        dom_root,
        styles: style_map,
        box_models: box_map,
        node_layouts: layout_map,
    }
}

/// 递归构建 CDP DOM 节点
fn build_cdp_node(
    node: &Rc<RefCell<Node>>,
    styles: &HashMap<usize, ComputedStyle>,
    layout_root: &LayoutBox,
    id_counter: &mut u32,
    style_map: &mut HashMap<u32, Value>,
    box_map: &mut HashMap<u32, Value>,
    layout_map: &mut HashMap<u32, dom::Rect<f32>>,
) -> Value {
    let node_id = *id_counter;
    *id_counter += 1;

    let node_ref = node.borrow();
    let dom_ptr = Rc::as_ptr(node) as usize;

    match &node_ref.node_type {
        NodeType::Document | NodeType::DocumentFragment => {
            let children: Vec<Value> = node_ref
                .child_nodes()
                .iter()
                .map(|child| {
                    build_cdp_node(child, styles, layout_root, id_counter, style_map, box_map, layout_map)
                })
                .collect();
            drop(node_ref);

            if children.len() == 1 {
                return children.into_iter().next().unwrap();
            }
            json!({
                "nodeId": node_id,
                "backendNodeId": node_id,
                "nodeType": 1,
                "nodeName": "HTML",
                "localName": "html",
                "nodeValue": "",
                "childNodeCount": children.len(),
                "children": children,
                "attributes": []
            })
        }
        NodeType::Element(elem) => {
            let tag = elem.tag_name().to_lowercase();
            let mut attrs: Vec<String> = Vec::new();
            for key in elem.attribute_names() {
                if let Some(val) = elem.get_attribute(&key) {
                    attrs.push(key);
                    attrs.push(val);
                }
            }
            let children: Vec<Value> = node_ref
                .child_nodes()
                .iter()
                .map(|child| {
                    build_cdp_node(child, styles, layout_root, id_counter, style_map, box_map, layout_map)
                })
                .collect();
            let count = children.len();
            drop(node_ref);

            // 计算样式
            if let Some(cs) = styles.get(&dom_ptr) {
                let css_arr = computed_style_to_cdp(cs);
                if !css_arr.is_empty() {
                    style_map.insert(node_id, json!(css_arr));
                }
            }

            // 盒模型
            if let Some(lb) = layout_root.find_layout_node(node) {
                let x = lb.rect.x;
                let y = lb.rect.y;
                let w = lb.rect.width;
                let h = lb.rect.height;
                let pl = lb.padding.left;
                let pt = lb.padding.top;
                let pr = lb.padding.right;
                let pb = lb.padding.bottom;
                let bl = lb.border.left;
                let bt = lb.border.top;
                let br = lb.border.right;
                let bb = lb.border.bottom;
                let ml = lb.margin.left;
                let mt = lb.margin.top;
                let mr = lb.margin.right;
                let mb = lb.margin.bottom;

                box_map.insert(node_id, json!({
                    "content":  [x+bl+pl, y+bt+pt, x+bl+pl+w, y+bt+pt, x+bl+pl+w, y+bt+pt+h, x+bl+pl, y+bt+pt+h],
                    "padding":  [x+bl, y+bt, x+bl+w+pl+pr, y+bt, x+bl+w+pl+pr, y+bt+pt+pb+h, x+bl, y+bt+pt+pb+h],
                    "border":   [x, y, x+bl+pl+pr+w+br, y, x+bl+pl+pr+w+br, y+bt+pt+pb+h+bb, x, y+bt+pt+pb+h+bb],
                    "margin":   [x-ml, y-mt, x+bl+pl+pr+w+br+mr, y-mt, x+bl+pl+pr+w+br+mr, y+bt+pt+pb+h+bb+mb, x-ml, y+bt+pt+pb+h+bb+mb],
                    "width":    w + pl + pr + bl + br,
                    "height":   h + pt + pb + bt + bb
                }));
                layout_map.insert(node_id, lb.rect);
            }

            json!({
                "nodeId": node_id,
                "backendNodeId": node_id,
                "nodeType": 1,
                "nodeName": tag.to_uppercase(),
                "localName": tag,
                "nodeValue": "",
                "childNodeCount": count,
                "children": children,
                "attributes": attrs
            })
        }
        NodeType::Text(text) => {
            let content = text.data().to_string();
            drop(node_ref);
            json!({
                "nodeId": node_id,
                "backendNodeId": node_id,
                "nodeType": 3,
                "nodeName": "#text",
                "localName": "",
                "nodeValue": content,
                "childNodeCount": 0,
                "children": []
            })
        }
        NodeType::Comment(comment) => {
            let content = comment.clone();
            drop(node_ref);
            json!({
                "nodeId": node_id,
                "backendNodeId": node_id,
                "nodeType": 8,
                "nodeName": "#comment",
                "localName": "",
                "nodeValue": content,
                "childNodeCount": 0,
                "children": []
            })
        }
    }
}

/// ComputedStyle → CDP CSS 属性数组
fn computed_style_to_cdp(cs: &ComputedStyle) -> Vec<Value> {
    let mut props: Vec<(&style::PropertyId, &CSSValue)> = cs.iter().collect();
    // 按属性名排序
    props.sort_by(|(a, _), (b, _)| a.name().cmp(b.name()));
    props
        .iter()
        .map(|(prop, val)| {
            json!({
                "name": prop.name(),
                "value": css_value_to_string(val)
            })
        })
        .collect()
}

/// CSSValue → CSS 字符串表示
fn css_value_to_string(val: &CSSValue) -> String {
    match val {
        CSSValue::Keyword(s) => s.clone(),
        CSSValue::Length(v, unit) => format!("{}{}", format_f32(*v), unit_str(*unit)),
        CSSValue::Percentage(v) => format!("{}%", format_f32(*v)),
        CSSValue::Color(c) => {
            if c.a == 255 {
                format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
            } else {
                format!("rgba({}, {}, {}, {:.2})", c.r, c.g, c.b, c.a as f32 / 255.0)
            }
        }
        CSSValue::Number(v) => format_f32(*v),
        CSSValue::String(s) => s.clone(),
        CSSValue::Composite(vals) => vals
            .iter()
            .map(|v| css_value_to_string(v))
            .collect::<Vec<_>>()
            .join(" "),
        CSSValue::Initial => "initial".to_string(),
        _ => format!("{:?}", val),
    }
}

fn format_f32(v: f32) -> String {
    if v.fract().abs() < 0.05 {
        format!("{}", v as i32)
    } else {
        format!("{:.1}", v)
    }
}

fn unit_str(u: style::CSSUnit) -> &'static str {
    use style::CSSUnit::*;
    match u {
        Px => "px", Em => "em", Rem => "rem", Vw => "vw", Vh => "vh",
        Vmin => "vmin", Vmax => "vmax", Percent => "%", Deg => "deg",
        Rad => "rad", Grad => "grad", Turn => "turn", S => "s", Ms => "ms",
        Dpi => "dpi", Dpcm => "dpcm", None => "",
    }
}

// ============================================================
//  HTTP + WebSocket 服务器
// ============================================================

fn run_server(snapshot: Arc<RwLock<Option<PageSnapshot>>>, events: Arc<Mutex<Vec<String>>>, port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[DevTools] 无法绑定 {}: {}", addr, e);
            return;
        }
    };
    println!("[DevTools] 调试服务已启动");
    println!("[DevTools] 1. 打开 Chrome → chrome://inspect");
    println!("[DevTools] 2. 点击 Configure... 添加 localhost:{}", port);
    println!("[DevTools] 3. 在 Remote Target 中找到目标，点击 inspect");

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let snap = snapshot.clone();
                let evts = events.clone();
                thread::spawn(move || handle_connection(s, &snap, &evts));
            }
            Err(_) => break,
        }
    }
}

fn handle_connection(mut stream: TcpStream, snapshot: &Arc<RwLock<Option<PageSnapshot>>>, events: &Arc<Mutex<Vec<String>>>) {
    // 读取 HTTP 请求首行
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }

    // 读取其余请求头
    let mut headers = String::new();
    let mut sec_websocket_key = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return;
        }
        if line == "\r\n" || line == "\n" || line.is_empty() {
            break;
        }
        if line.to_lowercase().starts_with("sec-websocket-key:") {
            sec_websocket_key = line
                .split(':')
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();
        }
        headers.push_str(&line);
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let path = parts[1];

    // Drop BufReader so we can use the raw stream again
    drop(reader);

    if path.starts_with("/json") {
        handle_http(&mut stream, path, snapshot);
    } else {
        handle_websocket(&mut stream, &sec_websocket_key, snapshot, events);
    }
}

// ============================================================
//  HTTP 端点
// ============================================================

fn handle_http(stream: &mut TcpStream, path: &str, snapshot: &Arc<RwLock<Option<PageSnapshot>>>) {
    if path == "/json/version" {
        let body = json!({
            "Browser": "RustBrowser/0.1",
            "Protocol-Version": "1.3"
        })
        .to_string();
        respond_http(stream, 200, "application/json", &body);
    } else if path == "/json" || path == "/json/list" {
        let _snap_guard = snapshot.read().unwrap_or_else(|e| e.into_inner());
        let title = "Rust App";
        let body = json!([{
            "id": "0",
            "type": "page",
            "title": title,
            "url": "http://localhost:0",
            "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/0",
            "description": "",
            "devtoolsFrontendUrl": "devtools://devtools/bundled/inspector.html?ws=127.0.0.1:9222/devtools/page/0",
            "faviconUrl": ""
        }]).to_string();
        respond_http(stream, 200, "application/json", &body);
    } else {
        // /json/protocol, /json/new 等 → 空对象的合法响应
        respond_http(stream, 200, "application/json", "{}");
    }
}

fn respond_http(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) {
    let status_text = match status {
        200 => "OK",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status, status_text, content_type, body.len(), body
    );
    let _ = stream.write_all(response.as_bytes());
}

// ============================================================
//  WebSocket / CDP 协议处理
// ============================================================

fn handle_websocket(stream: &mut TcpStream, sec_key: &str, snapshot: &Arc<RwLock<Option<PageSnapshot>>>, events: &Arc<Mutex<Vec<String>>>) {
    // WebSocket 握手响应
    if sec_key.is_empty() {
        respond_http(stream, 400, "text/plain", "Bad Request: missing Sec-WebSocket-Key");
        return;
    }
    let accept_key = compute_accept_key(sec_key);
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept_key
    );
    if stream.write_all(response.as_bytes()).is_err() {
        return;
    }
    if stream.flush().is_err() {
        return;
    }

    // 创建 WebSocket（握手已完成，进入帧循环）
    let stream_clone = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    stream_clone.set_read_timeout(Some(std::time::Duration::from_millis(250))).ok();
    let mut ws = WebSocket::from_raw_socket(stream_clone, Role::Server, None);

    // CDP 消息循环
    loop {
        // 先发送积压的 CDP 事件
        drain_events(&mut ws, events);

        let msg = match ws.read() {
            Ok(m) => m,
            Err(tungstenite::Error::Io(ref e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => break,
        };

        match msg {
            tungstenite::Message::Text(text) => {
                // 处理前再刷新一次事件
                drain_events(&mut ws, events);
                let response = process_cdp_message(&text, snapshot);
                if let Some(resp_text) = response {
                    if ws.send(tungstenite::Message::Text(resp_text)).is_err() {
                        break;
                    }
                }
            }
            tungstenite::Message::Close(_) => break,
            tungstenite::Message::Ping(data) => {
                drain_events(&mut ws, events);
                let _ = ws.send(tungstenite::Message::Pong(data));
            }
            _ => {}
        }
    }
}

/// 发送积压的 CDP 事件到 WebSocket 连接
fn drain_events(ws: &mut WebSocket<TcpStream>, events: &Arc<Mutex<Vec<String>>>) {
    if let Ok(mut queue) = events.lock() {
        for event in queue.drain(..) {
            let _ = ws.send(tungstenite::Message::Text(event));
        }
    }
}

/// 处理单条 CDP 消息，返回响应 JSON 字符串
fn process_cdp_message(msg: &str, snapshot: &Arc<RwLock<Option<PageSnapshot>>>) -> Option<String> {
    let parsed: Value = match serde_json::from_str(msg) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let id = parsed.get("id").cloned();
    let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = parsed.get("params");

    let result = match method {
        // ── Browser ──
        "Browser.getVersion" => Some(json!({
            "protocolVersion": "1.3",
            "product": "RustBrowser/0.1",
            "revision": "",
            "userAgent": "RustBrowser/0.1",
            "jsVersion": "0.1"
        })),

        // ── Target ──
        "Target.setDiscoverTargets" | "Target.setAutoAttach" => {
            Some(json!({}))
        }

        // ── Page ──
        "Page.enable" => Some(json!({})),
        "Page.getResourceTree" => Some(json!({
            "frameTree": {
                "frame": {
                    "id": "0",
                    "loaderId": "loader-0",
                    "url": "http://localhost:0",
                    "securityOrigin": "http://localhost:0",
                    "mimeType": "text/html"
                },
                "resources": []
            }
        })),
        "Page.startScreencast" | "Page.stopScreencast" |
        "Page.setAdBlockingEnabled" | "Page.getAppManifest" |
        "Page.getInstallabilityErrors" => Some(json!({})),

        // ── Runtime ──
        "Runtime.enable" => Some(json!({})),
        "Runtime.runIfWaitingForDebugger" => Some(json!({})),

        // ── DOM ──
        "DOM.enable" => Some(json!({})),
        "DOM.getDocument" => {
            let _depth = params
                .and_then(|p| p.get("depth"))
                .and_then(|d| d.as_i64())
                .unwrap_or(-1);
            let _pierce = params
                .and_then(|p| p.get("pierce"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            // 忽略 depth/pierce，始终返回完整树
            let guard = snapshot.read().unwrap_or_else(|e| e.into_inner());
            match guard.as_ref() {
                Some(snap) => Some(json!({ "root": snap.dom_root })),
                None => Some(json!({
                    "root": {
                        "nodeId": 0, "backendNodeId": 0, "nodeType": 1,
                        "nodeName": "HTML", "localName": "html", "nodeValue": "",
                        "childNodeCount": 1,
                        "children": [{
                            "nodeId": 1, "backendNodeId": 1, "nodeType": 1,
                            "nodeName": "BODY", "localName": "body", "nodeValue": "",
                            "childNodeCount": 0, "children": [], "attributes": []
                        }],
                        "attributes": []
                    }
                })),
            }
        }
        "DOM.getChildNodes" => {
            let _node_id = params.and_then(|p| p.get("nodeId")).and_then(|v| v.as_u64());
            // 不支持延迟加载，返回空
            Some(json!({ "nodes": [] }))
        }
        "DOM.getBoxModel" => {
            let node_id = params
                .and_then(|p| p.get("nodeId"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            let guard = snapshot.read().unwrap_or_else(|e| e.into_inner());
            match (node_id, guard.as_ref()) {
                (Some(nid), Some(snap)) => {
                    snap.box_models.get(&nid)
                        .cloned()
                        .unwrap_or(json!({
                            "model": {
                                "content":[], "padding":[], "border":[], "margin":[],
                                "width": 0, "height": 0
                            }
                        }))
                        .as_object()
                        .map(|_| json!({ "model": snap.box_models.get(&nid).cloned().unwrap_or(json!({"content":[],"padding":[],"border":[],"margin":[],"width":0,"height":0}))}))
                }
                _ => Some(json!({
                    "model": {
                        "content": [], "padding": [], "border": [], "margin": [],
                        "width": 0, "height": 0
                    }
                })),
            }
        }
        "DOM.getNodeForLocation" => {
            // 不支持点击检查
            Some(json!({ "nodeId": 1 }))
        }
        "DOM.highlightNode" | "DOM.hideHighlight" |
        "DOM.setInspectedNode" | "DOM.getSearchResults" |
        "DOM.discardSearchResults" | "DOM.performSearch" |
        "DOM.querySelector" | "DOM.querySelectorAll" |
        "DOM.setNodeName" | "DOM.setNodeValue" |
        "DOM.setAttributesAsText" | "DOM.removeNode" |
        "DOM.setAttributeValue" | "DOM.removeAttribute" |
        "DOM.setOuterHTML" | "DOM.getOuterHTML" => Some(json!({})),

        // ── CSS ──
        "CSS.enable" => Some(json!({})),
        "CSS.getComputedStyleForNode" => {
            let node_id = params
                .and_then(|p| p.get("nodeId"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            let guard = snapshot.read().unwrap_or_else(|e| e.into_inner());
            match (node_id, guard.as_ref()) {
                (Some(nid), Some(snap)) => {
                    let styles = snap.styles.get(&nid).cloned().unwrap_or(json!([]));
                    Some(json!({ "computedStyle": styles }))
                }
                _ => Some(json!({ "computedStyle": [] })),
            }
        }
        "CSS.getMatchedStylesForNode" => {
            let node_id = params
                .and_then(|p| p.get("nodeId"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            let guard = snapshot.read().unwrap_or_else(|e| e.into_inner());
            let computed = match (node_id, guard.as_ref()) {
                (Some(nid), Some(snap)) => {
                    snap.styles.get(&nid).cloned().unwrap_or(json!([]))
                }
                _ => json!([]),
            };
            Some(json!({
                "matchedCSSRules": [],
                "inlineStyle": null,
                "inherited": [{
                    "matchedCSSRules": [],
                    "inlineStyle": {
                        "cssProperties": computed,
                        "shorthandEntries": [],
                        "cssText": ""
                    }
                }],
                "pseudoElements": []
            }))
        }
        "CSS.getInlineStylesForNode" => Some(json!({
            "inlineStyle": null,
            "attributesStyle": null
        })),
        "CSS.getBackgroundColors" | "CSS.getPlatformFontsForNode" |
        "CSS.startRuleUsageTracking" | "CSS.stopRuleUsageTracking" |
        "CSS.addRule" | "CSS.createStyleSheet" | "CSS.setStyleSheetText" |
        "CSS.setStyleTexts" | "CSS.forcePseudoState" |
        "CSS.setEffectivePropertyValueForNode" => Some(json!({})),

        // ── Overlay ──
        "Overlay.enable" => Some(json!({})),
        "Overlay.setInspectMode" => Some(json!({})),
        "Overlay.setShowAdHighlights" | "Overlay.setShowDebugBorders" |
        "Overlay.setShowFPSCounter" | "Overlay.setShowPaintRects" |
        "Overlay.setShowScrollBottleneckRects" | "Overlay.setShowLayoutShiftRegions" |
        "Overlay.highlightNode" | "Overlay.hideHighlight" |
        "Overlay.setPausedInDebuggerMessage" => Some(json!({})),

        // ── Network ──
        "Network.enable" | "Network.setCacheDisabled" |
        "Network.setAcceptedEncodings" | "Network.canClearBrowserCache" |
        "Network.canClearBrowserCookies" | "Network.clearBrowserCache" |
        "Network.clearBrowserCookies" | "Network.getResponseBody" |
        "Network.emulateNetworkConditions" | "Network.setBypassServiceWorker" => {
            Some(json!({}))
        }

        // ── 其他域 ──
        "Log.enable" | "Log.startViolationsReport" | "Log.stopViolationsReport" => {
            Some(json!({}))
        }
        "Inspector.enable" => Some(json!({})),
        "Performance.enable" | "Performance.getMetrics" => Some(json!({})),
        "Emulation.canEmulate" => Some(json!({ "result": false })),
        "Emulation.setEmulatedMedia" | "Emulation.setFocusEmulationEnabled" |
        "Emulation.setDeviceMetricsOverride" | "Emulation.setEmitTouchEventsForMouse" => {
            Some(json!({}))
        }
        "Debugger.enable" | "Debugger.setAsyncCallStackDepth" |
        "Debugger.setPauseOnExceptions" | "Debugger.setSkipAllPauses" => {
            Some(json!({}))
        }
        "Console.enable" | "Console.clearMessages" => Some(json!({})),
        "Profiler.enable" => Some(json!({})),
        "Rendering.setShowPaintRects" | "Rendering.setShowDebugBorders" |
        "Rendering.setShowFPSCounter" | "Rendering.setShowScrollBottleneckRects" |
        "Rendering.setShowLayoutShiftRegions" => Some(json!({})),
        "HeadlessExperimental.enable" => Some(json!({})),
        "Fetch.enable" | "Fetch.disable" => Some(json!({})),

        // ── 未知方法 ──
        _ => {
            // 返回空结果避免 DevTools 报错
            Some(json!({}))
        }
    };

    result.map(|r| {
        let response = if let Some(i) = id {
            json!({ "id": i, "result": r })
        } else {
            json!({ "result": r })
        };
        response.to_string()
    })
}

// ============================================================
//  WebSocket 握手辅助
// ============================================================

fn compute_accept_key(sec_websocket_key: &str) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(sec_websocket_key.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let hash = hasher.finalize();
    base64_encode(hash.as_slice())
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[(triple >> 18) as usize & 0x3F] as char);
        result.push(CHARS[(triple >> 12) as usize & 0x3F] as char);
        result.push(if chunk.len() > 1 {
            CHARS[(triple >> 6) as usize & 0x3F] as char
        } else {
            b'=' as char
        });
        result.push(if chunk.len() > 2 {
            CHARS[triple as usize & 0x3F] as char
        } else {
            b'=' as char
        });
    }
    result
}
