//! dev_server — 运行时开发工具：临时 HTTP 服务 + 交互式仪表盘。
//!
//! 使用纯标准库 `std::net::TcpListener`，零额外依赖。
//! 内嵌完整的 HTML/CSS/JS 仪表盘页面。

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

/// 启动临时 HTTP 仪表盘服务器。
///
/// - `tree_json`: 由 `serialize_tree()` 生成的 JSON 数据
/// - `port`: 监听端口（默认 9876）
///
/// 返回线程 handle，服务器在独立线程中运行。
/// 启动后自动尝试打开系统默认浏览器。
pub fn start_dev_server(tree_json: &str, port: u16) -> JoinHandle<()> {
    let json_data = Arc::new(tree_json.to_string());

    let listener = match TcpListener::bind(format!("127.0.0.1:{port}")) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[DevServer] 无法绑定端口 {port}: {e}");
            return thread::spawn(|| {});
        }
    };

    listener.set_nonblocking(true).ok();

    println!("\n╔══════════════════════════════════════════╗");
    println!("║   Ruft DevTools 仪表盘                    ║");
    println!("║   地址: http://localhost:{port}              ║",);
    println!("║   按 Ctrl+C 退出                           ║");
    println!("╚══════════════════════════════════════════╝\n");

    let start_time = SystemTime::now();

    thread::spawn(move || {
        let timeout = Duration::from_secs(300); // 5 分钟无请求自动退出

        loop {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let _ = addr;
                    handle_connection(stream, &json_data);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 无连接，检查超时
                    if start_time.elapsed().unwrap_or_default() > timeout {
                        println!("[DevServer] 超时，自动关闭。");
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(_) => {
                    break;
                }
            }
        }
        println!("[DevServer] 已关闭。");
    })
}

fn handle_connection(stream: TcpStream, json_data: &str) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }

    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/");

    match path {
        "/api/tree" => {
            let body = json_data;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = reader.get_mut().write_all(response.as_bytes());
        }
        _ => {
            let html = DASHBOARD_HTML;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            let _ = reader.get_mut().write_all(response.as_bytes());
        }
    }
}

/// 尝试打开系统默认浏览器。
pub fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()
            .ok();
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .ok();
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .ok();
    }
}

/// 内嵌的仪表盘 HTML 页面（单文件，零外部依赖）。
const DASHBOARD_HTML: &str = r###"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Ruft DevTools — 运行时仪表盘</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:'Segoe UI',system-ui,sans-serif;background:#1a1a2e;color:#e0e0e0;height:100vh;overflow:hidden}
#header{background:#16213e;padding:8px 16px;display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid #0f3460}
#header h1{font-size:16px;color:#e94560;font-weight:600}
#header .badge{font-size:11px;background:#0f3460;color:#53d8fb;padding:2px 8px;border-radius:10px;margin-left:8px}
#container{display:grid;grid-template-columns:280px 1fr 320px;height:calc(100vh - 42px);gap:1px;background:#0f3460}
.panel{background:#1a1a2e;overflow:auto;display:flex;flex-direction:column}
.panel-title{padding:10px 12px;font-size:12px;font-weight:600;text-transform:uppercase;letter-spacing:1px;color:#53d8fb;background:#16213e;position:sticky;top:0;z-index:1;border-bottom:1px solid #0f3460}
.tree-node{cursor:pointer;user-select:none;font-size:13px;line-height:1.8}
.tree-node:hover{background:rgba(233,69,96,0.1)}
.tree-node.active{background:rgba(233,69,96,0.25);border-left:3px solid #e94560}
.tree-node.matched{background:rgba(83,216,251,0.15)}
.tree-caret{display:inline-block;width:16px;text-align:center;color:#666;font-size:10px;cursor:pointer}
.tree-caret.open{color:#53d8fb}
.tree-tag{color:#e94560;font-weight:600}
.tree-id{color:#f5c542}
.tree-class{color:#53d8fb}
.tree-text{color:#888;font-style:italic}
.tree-flags{margin-left:4px;font-size:10px}
.tree-flag-dirty{color:#e94560}
.tree-flag-layout{color:#f5c542}
.tree-flag-paint{color:#e78230}
.style-summary{font-size:10px;color:#666;margin-left:4px}
.rule-item{padding:6px 12px;border-bottom:1px solid rgba(15,52,96,0.5);cursor:pointer;font-size:12px;transition:background .15s}
.rule-item:hover{background:rgba(233,69,96,0.08)}
.rule-item.active{background:rgba(233,69,96,0.2);border-left:3px solid #e94560}
.rule-selector{color:#53d8fb;font-weight:600;margin-bottom:3px;font-family:'Fira Code',monospace}
.rule-source{font-size:10px;color:#666;margin-bottom:2px}
.rule-decl{font-size:11px;color:#ccc;margin:1px 0}
.rule-decl-prop{color:#e94560}
.rule-decl-val{color:#f5c542}
.detail-empty{color:#555;padding:20px;text-align:center;font-size:13px}
.detail-section{margin-bottom:12px}
.detail-section-title{font-size:11px;color:#53d8fb;text-transform:uppercase;margin-bottom:6px;padding-bottom:3px;border-bottom:1px solid #0f3460}
.detail-row{display:flex;justify-content:space-between;padding:3px 0;font-size:12px}
.detail-label{color:#888}
.detail-value{color:#e0e0e0;font-family:'Fira Code',monospace;font-size:11px}
.color-swatch{display:inline-block;width:14px;height:14px;border-radius:2px;border:1px solid #333;vertical-align:middle;margin-right:4px}
#detail-content{padding:8px 12px}
.clr{display:inline-flex;align-items:center}
</style>
</head>
<body>

<div id="header">
  <div style="display:flex;align-items:center">
    <h1>◇ Ruft DevTools</h1>
    <span class="badge">运行时仪表盘</span>
  </div>
  <div style="font-size:11px;color:#666">
    端口: <span id="port" style="color:#53d8fb">9876</span>
    &nbsp;|&nbsp; 节点: <span id="node-count" style="color:#e94560">-</span>
    &nbsp;|&nbsp; 规则: <span id="rule-count" style="color:#f5c542">-</span>
  </div>
</div>

<div id="container">
  <!-- 左栏: DOM 树 -->
  <div class="panel">
    <div class="panel-title">🌳 DOM 树</div>
    <div id="dom-tree" style="padding:8px;flex:1;overflow:auto"></div>
  </div>

  <!-- 中栏: CSS 规则 -->
  <div class="panel">
    <div class="panel-title">📋 CSS 规则</div>
    <div id="rules-list" style="flex:1;overflow:auto"></div>
  </div>

  <!-- 右栏: 节点详情 -->
  <div class="panel">
    <div class="panel-title">🔍 节点详情</div>
    <div id="detail-content">
      <div class="detail-empty">点击 DOM 节点或 CSS 规则查看详情</div>
    </div>
  </div>
</div>

<script>
let treeData = null;
let selectedNode = null;
let selectedRuleIdx = null;

async function init() {
  try {
    const res = await fetch('/api/tree');
    treeData = await res.json();
    renderAll();
  } catch(e) {
    document.getElementById('dom-tree').innerHTML = '<div style="color:#e94560;padding:12px">❌ 无法加载数据: ' + e.message + '</div>';
  }
}

function renderAll() {
  if (!treeData) return;
  document.getElementById('dom-tree').innerHTML = '';
  if (treeData.dom) {
    renderDomNode(treeData.dom, document.getElementById('dom-tree'), 0);
  }
  renderRules();
  document.getElementById('node-count').textContent = countNodes(treeData.dom);
  document.getElementById('rule-count').textContent = treeData.rules ? treeData.rules.length : 0;
}

function countNodes(node) {
  if (!node) return 0;
  let c = 1;
  if (node.children) node.children.forEach(ch => c += countNodes(ch));
  return c;
}

// ── DOM 树渲染 ──

function renderDomNode(node, parent, depth) {
  const div = document.createElement('div');
  div.className = 'tree-node';
  div.style.paddingLeft = (depth * 20 + 4) + 'px';
  div.dataset.nodeId = node.nodeId;

  // 折叠/展开
  const hasKids = node.children && node.children.length > 0;
  const caret = document.createElement('span');
  caret.className = 'tree-caret' + (hasKids ? ' open' : '');
  caret.textContent = hasKids ? '▼' : '·';
  caret.onclick = (e) => {
    e.stopPropagation();
    const kids = div.nextElementSibling;
    if (kids && kids.classList.contains('tree-kids')) {
      const hide = kids.style.display !== 'none';
      kids.style.display = hide ? 'none' : '';
      caret.textContent = hide ? '▶' : '▼';
      caret.classList.toggle('open', !hide);
    }
  };
  div.appendChild(caret);

  // 节点标签
  if (node.nodeType === 'text') {
    const span = document.createElement('span');
    span.className = 'tree-text';
    span.textContent = '"' + (node.text || '') + '"';
    div.appendChild(span);
  } else {
    // tag
    const tag = document.createElement('span');
    tag.className = 'tree-tag';
    tag.textContent = node.tagName || '?';
    div.appendChild(tag);
    // id
    if (node.id) {
      const id = document.createElement('span');
      id.className = 'tree-id';
      id.textContent = '#' + node.id;
      div.appendChild(id);
    }
    // classes
    (node.classes || []).forEach(c => {
      const cl = document.createElement('span');
      cl.className = 'tree-class';
      cl.textContent = '.' + c;
      div.appendChild(cl);
    });
  }

  // 脏标记
  const flags = [];
  if (node.styleDirty) flags.push('<span class="tree-flag-dirty">S</span>');
  if (node.layoutDirty) flags.push('<span class="tree-flag-layout">L</span>');
  if (node.paintDirty) flags.push('<span class="tree-flag-paint">P</span>');
  if (flags.length > 0) {
    const fs = document.createElement('span');
    fs.className = 'tree-flags';
    fs.innerHTML = '[' + flags.join('') + ']';
    div.appendChild(fs);
  }

  // 样式摘要
  if (node.computedStyle) {
    const parts = [];
    const cs = node.computedStyle;
    if (cs.bgColor) parts.push('bg=' + cs.bgColor);
    if (cs.textColor) parts.push('fg=' + cs.textColor);
    if (cs.fontSize) parts.push('fs=' + cs.fontSize);
    if (cs.padding) parts.push('pad=' + cs.padding);
    if (parts.length > 0) {
      const ss = document.createElement('span');
      ss.className = 'style-summary';
      ss.textContent = parts.join(' ');
      div.appendChild(ss);
    }
  }

  // 点击事件
  div.onclick = (e) => {
    e.stopPropagation();
    selectNode(node, div);
  };

  parent.appendChild(div);

  // 子节点容器
  if (hasKids) {
    const kidsDiv = document.createElement('div');
    kidsDiv.className = 'tree-kids';
    node.children.forEach(ch => renderDomNode(ch, kidsDiv, depth + 1));
    parent.appendChild(kidsDiv);
  }
}

// ── 节点选中 ──

function selectNode(node, el) {
  document.querySelectorAll('.tree-node.active').forEach(e => e.classList.remove('active'));
  if (el) el.classList.add('active');
  selectedNode = node;
  selectedRuleIdx = null;
  document.querySelectorAll('.rule-item.active').forEach(e => e.classList.remove('active'));
  showNodeDetail(node);
}

function showNodeDetail(node) {
  const area = document.getElementById('detail-content');
  if (!node) { area.innerHTML = '<div class="detail-empty">点击 DOM 节点或 CSS 规则查看详情</div>'; return; }

  let html = '<div class="detail-section">';
  html += '<div class="detail-section-title">节点信息</div>';
  html += row('nodeId', node.nodeId);
  html += row('类型', node.nodeType === 'text' ? '文本节点' : '元素节点');
  if (node.tagName) html += row('标签', '&lt;' + node.tagName + '&gt;');
  if (node.id) html += row('ID', '#' + node.id);
  if (node.classes && node.classes.length > 0) html += row('Class', node.classes.map(c => '.' + c).join(' '));
  if (node.nodeType === 'text') html += row('Text', '"' + (node.text || '') + '"');
  html += '</div>';

  // Flags
  const flags = [];
  if (node.styleDirty) flags.push('styleDirty');
  if (node.layoutDirty) flags.push('layoutDirty');
  if (node.paintDirty) flags.push('paintDirty');
  if (flags.length > 0) {
    html += '<div class="detail-section"><div class="detail-section-title">脏标记</div>';
    html += '<span style="color:#e94560;font-size:12px">' + flags.join(', ') + '</span>';
    html += '</div>';
  }

  // ComputedStyle
  if (node.computedStyle) {
    html += '<div class="detail-section"><div class="detail-section-title">计算样式</div>';
    const cs = node.computedStyle;
    if (cs.bgColor) html += colorRow('bgColor', cs.bgColor);
    if (cs.textColor) html += colorRow('textColor', cs.textColor);
    if (cs.width != null) html += row('width', cs.width + 'px');
    if (cs.height != null) html += row('height', cs.height + 'px');
    if (cs.padding) html += row('padding', cs.padding + 'px');
    if (cs.margin) html += row('margin', cs.margin + 'px');
    if (cs.fontSize) html += row('fontSize', cs.fontSize + 'px');
    if (cs.fontFamily) html += row('fontFamily', cs.fontFamily);
    if (cs.textAlign) html += row('textAlign', cs.textAlign);
    if (cs.textDecoration) html += row('textDecoration', cs.textDecoration);
    html += '</div>';
  }

  area.innerHTML = html;
}

function row(label, value) {
  return '<div class="detail-row"><span class="detail-label">' + label + '</span><span class="detail-value">' + value + '</span></div>';
}

function colorRow(label, hex) {
  return '<div class="detail-row"><span class="detail-label">' + label + '</span><span class="detail-value clr"><span class="color-swatch" style="background:' + hex + '"></span>' + hex + '</span></div>';
}

// ── CSS 规则列表 ──

function renderRules() {
  const area = document.getElementById('rules-list');
  area.innerHTML = '';
  if (!treeData.rules || treeData.rules.length === 0) {
    area.innerHTML = '<div class="detail-empty" style="padding:12px">暂无 CSS 规则</div>';
    return;
  }
  treeData.rules.forEach((rule, idx) => {
    const div = document.createElement('div');
    div.className = 'rule-item';
    div.onclick = () => selectRule(idx, div);

    // 选择器
    const sel = document.createElement('div');
    sel.className = 'rule-selector';
    sel.textContent = rule.selectorText;
    div.appendChild(sel);

    // 来源
    const src = document.createElement('div');
    src.className = 'rule-source';
    src.textContent = '📄 ' + (rule.source || '<unknown>');
    div.appendChild(src);

    // 声明
    (rule.declarations || []).forEach(([prop, val]) => {
      const d = document.createElement('div');
      d.className = 'rule-decl';
      d.innerHTML = '<span class="rule-decl-prop">' + prop + '</span>: <span class="rule-decl-val">' + val + '</span>';
      div.appendChild(d);
    });

    area.appendChild(div);
  });
}

function selectRule(idx, el) {
  document.querySelectorAll('.rule-item.active').forEach(e => e.classList.remove('active'));
  if (el) el.classList.add('active');
  selectedRuleIdx = idx;
  selectedNode = null;
  document.querySelectorAll('.tree-node.active').forEach(e => e.classList.remove('active'));

  // 在详情区显示规则详情
  const rule = treeData.rules[idx];
  const area = document.getElementById('detail-content');
  let html = '<div class="detail-section">';
  html += '<div class="detail-section-title">CSS 规则</div>';
  html += row('选择器', rule.selectorText);
  html += row('来源', rule.source || '<unknown>');
  html += '</div>';
  html += '<div class="detail-section"><div class="detail-section-title">声明</div>';
  (rule.declarations || []).forEach(([prop, val]) => {
    html += row(prop, val);
    // 颜色预览
    if (val.match(/^#[0-9a-fA-F]{3,8}$/)) {
      html = html.replace('</div>', '') + '';
    }
  });
  html += '</div>';

  area.innerHTML = html;

  // 高亮匹配的 DOM 节点
  highlightMatchingNodes(rule.selectorText);
}

function highlightMatchingNodes(selector) {
  document.querySelectorAll('.tree-node.matched').forEach(e => e.classList.remove('matched'));
  // 简单的选择器匹配可视化（基于 tag/id/class）
  const allNodes = document.querySelectorAll('.tree-node');
  allNodes.forEach(el => {
    const nid = el.dataset.nodeId;
    if (!nid) return;
    // 简单启发式：检查选择器文本是否包含该节点的特征
    // 实际匹配逻辑在 Rust 端，这里只做视觉提示
    const nodeData = findNodeById(treeData.dom, parseInt(nid));
    if (nodeData && heuristicMatch(nodeData, selector)) {
      el.classList.add('matched');
    }
  });
}

function findNodeById(node, id) {
  if (!node) return null;
  if (node.nodeId === id) return node;
  if (node.children) {
    for (const ch of node.children) {
      const found = findNodeById(ch, id);
      if (found) return found;
    }
  }
  return null;
}

function heuristicMatch(node, selector) {
  if (!node || node.nodeType === 'text') return false;
  const s = selector.toLowerCase();
  const tag = (node.tagName || '').toLowerCase();
  const id = (node.id || '').toLowerCase();
  const classes = (node.classes || []).map(c => c.toLowerCase());
  if (tag && s.includes(tag)) return true;
  if (id && s.includes('#' + id)) return true;
  if (classes.some(c => s.includes('.' + c))) return true;
  return false;
}

// 启动
init();
</script>
</body>
</html>"###;
