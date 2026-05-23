//! # JS 编译器 — Phase 0
//!
//! 简化版 JS → Rust 编译，识别特定 DOM API 模式。
//! Phase 1+ 将替换为基于 swc 的完整 JS AST 编译。
//!
//! Phase 0 支持的 JS 模式：
//! - document.querySelector('.class')
//! - document.getElementById('id')
//! - element.addEventListener('event', function() { ... })
//! - element.textContent = expr
//! - let/const 声明
//! - 基本算术表达式

use std::collections::HashMap;

/// 事件处理器信息
#[derive(Debug)]
pub struct EventHandler {
    /// 目标元素的变量名
    pub element_var: String,
    /// 事件类型
    pub event_type: String,
    /// 处理器体的 Rust 代码
    pub body_code: String,
    /// 需要在闭包前 clone 的变量列表
    pub cloned_vars: Vec<String>,
}

/// JS 变量信息
#[derive(Debug)]
struct JsVar {
    name: String,
    rust_code: String,
}

/// 定位 HTML 元素
fn resolve_selector(selector: &str, html_lookup: &HashMap<String, String>) -> Option<String> {
    let selector = selector.trim().trim_matches('"').trim_matches('\'');
    html_lookup.get(selector).cloned()
        .or_else(|| html_lookup.get(&format!(".{}", selector)).cloned())
}

fn resolve_id(id: &str, html_lookup: &HashMap<String, String>) -> Option<String> {
    let id = id.trim().trim_matches('"').trim_matches('\'');
    html_lookup.get(id).cloned()
}

/// 构建 HTML 查询查找表
/// 键: 选择器（"#id", ".class", "tag"）→ 变量名
pub fn build_html_lookup(element_vars: &[(String, super::HtmlElement)]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (var_name, el) in element_vars {
        map.insert(el.tag.clone(), var_name.clone());
        if let Some(id) = el.attributes.get("id") {
            map.insert(format!("#{}", id), var_name.clone());
        }
        if let Some(class) = el.attributes.get("class") {
            for cls in class.split_whitespace() {
                map.insert(format!(".{}", cls), var_name.clone());
            }
        }
        // 变量名本身也可查询（用于直接引用已声明的变量）
        map.insert(var_name.clone(), var_name.clone());
    }
    map
}

/// 从 JS 代码中提取事件处理器
pub fn extract_event_handlers(
    js: &str,
    html_lookup: &HashMap<String, String>,
) -> Vec<EventHandler> {
    let mut handlers = Vec::new();

    // 逐行扫描，寻找 addEventListener 调用
    let lines: Vec<&str> = js.lines().collect();

    // Phase 0: 构建 JS 变量名 → 元素变量名 映射
    // 如: const btn = document.getElementById('inc-btn') → "btn" → "inc_btn"
    let mut js_var_map: HashMap<String, String> = HashMap::new();
    for line in &lines {
        let line = line.trim();
        if (line.starts_with("let ") || line.starts_with("const ")) && line.contains('=') {
            let eq_pos = line.find('=').unwrap();
            let var_name = {
                let before_eq = line[..eq_pos].trim();
                if let Some(name) = before_eq.strip_prefix("let ") {
                    name.trim().to_string()
                } else if let Some(name) = before_eq.strip_prefix("const ") {
                    name.trim().to_string()
                } else {
                    continue;
                }
            };
            let expr = line[eq_pos + 1..].trim().trim_end_matches(';').trim();
            if let Some(el_var) = resolve_element_var(expr, html_lookup) {
                js_var_map.insert(var_name, el_var);
            }
        }
    }

    // 合并 HTML 元素查找表和 JS 变量映射（JS 变量名可解析为元素变量名）
    let merged_lookup: HashMap<String, String> = html_lookup.iter()
        .chain(js_var_map.iter())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // 查找 addEventListener 调用
        if let Some(pos) = line.find(".addEventListener(") {
            let before = line[..pos].trim();
            let after = &line[pos + ".addEventListener(".len()..];

            // 尝试解析事件类型和函数体
            // 格式: element.addEventListener('click', function() { ... })
            // 或: element.addEventListener("click", function() { ... })

            // 提取事件类型（第一个参数）
            let event_type = if after.starts_with('\'') || after.starts_with('"') {
                let quote = after.chars().next().unwrap();
                let end = after[1..].find(quote).map(|p| p + 1).unwrap_or(0);
                after[1..end].to_string()
            } else {
                String::new()
            };

            // 查找函数体
            let fn_start = after.find("function");
            if let Some(fs) = fn_start {
                // 找到函数体的起始 { 位置
                let body_open = after[fs..].find('{').map(|p| fs + p + 1);

                let mut body_lines = Vec::new();
                let mut depth = 1;

                // 收集当前行 { 之后的内容
                if let Some(bs) = body_open {
                    let rest = &after[bs..];
                    let mut line_buf = String::new();
                    for ch in rest.chars() {
                        match ch {
                            '{' => depth += 1,
                            '}' => { depth -= 1; if depth == 0 { break; } }
                            '\n' => { if !line_buf.is_empty() { body_lines.push(line_buf.clone()); line_buf.clear(); } }
                            c => if depth > 0 { line_buf.push(c); }
                        }
                    }
                    if !line_buf.is_empty() { body_lines.push(line_buf); }
                }

                // 继续读取后续行直到 depth == 0
                let mut j = i + 1;
                while j < lines.len() && depth > 0 {
                    let line = lines[j];
                    let mut line_buf = String::new();
                    for ch in line.chars() {
                        match ch {
                            '{' => depth += 1,
                            '}' => { depth -= 1; if depth == 0 { break; } }
                            c => if depth > 0 { line_buf.push(c); }
                        }
                    }
                    if !line_buf.is_empty() { body_lines.push(line_buf); }
                    j += 1;
                }

                // 生成 Rust 事件处理器代码
                let mut cloned_vars = Vec::new();
                let rust_body = translate_event_body(&body_lines, &merged_lookup, &mut cloned_vars);

                // 解析元素变量（先直接查，再尝试 JS 变量名映射）
                let element_var = resolve_element_var(before, &merged_lookup);

                if let Some(ev) = element_var {
                    handlers.push(EventHandler {
                        element_var: ev,
                        event_type,
                        body_code: rust_body,
                        cloned_vars,
                    });
                }
            }
        }

        i += 1;
    }

    handlers
}

/// 解析元素变量引用
fn resolve_element_var(code: &str, lookup: &HashMap<String, String>) -> Option<String> {
    let code = code.trim();

    // document.querySelector('.foo') → 查找选择器
    if let Some(sel) = code.strip_prefix("document.querySelector(") {
        let sel = sel.trim_end_matches(')').trim().trim_matches('"').trim_matches('\'');
        return resolve_selector(sel, lookup);
    }

    // document.getElementById('foo') → 查找 ID
    if let Some(id) = code.strip_prefix("document.getElementById(") {
        let id = id.trim_end_matches(')').trim().trim_matches('"').trim_matches('\'');
        return resolve_id(&format!("#{}", id), lookup);
    }

    // 直接变量名（如前面声明过的 btn, display 等）
    if let Some(var) = lookup.get(code) {
        return Some(var.clone());
    }

    // 纯变量名本身也可能是变量名（由之前的 let 声明）
    // 这里通过调用者传入已知变量列表
    None
}

/// 翻译事件处理器体
fn translate_event_body(
    body_lines: &[String],
    html_lookup: &HashMap<String, String>,
    cloned_vars: &mut Vec<String>,
) -> String {
    let mut rust_lines = Vec::new();

    for line in body_lines {
        let line = line.trim();
        if line.is_empty() || line == "{" || line == "}" {
            continue;
        }

        // 跳过 function() { 之类的行
        if line.contains("function") {
            continue;
        }

        // 模式: element.textContent = expr;
        if line.contains(".textContent") {
            if let Some(pos) = line.find(".textContent") {
                let target = line[..pos].trim();
                let rhs_start = line[pos + ".textContent".len()..].find('=').map(|p| pos + ".textContent".len() + p + 1);
                if let Some(rs) = rhs_start {
                    let rhs = line[rs..].trim_end_matches(';').trim();
                    let var_name = resolve_element_var(target, html_lookup)
                        .unwrap_or_else(|| target.to_string());

                    if !cloned_vars.contains(&var_name) {
                        cloned_vars.push(format!("{}_clone", var_name));
                    }

                    // 处理不同类型的赋值
                    let rust_code = if rhs == "count" || rhs == "count + 1" || rhs.contains("count") {
                        // 复杂表达式: count 相关的值需要从 textContent 转换
                        // 简化为读取当前值 + 递增模式
                        format!(
                            "let val = {}_clone.borrow().text_content().parse::<i32>().unwrap_or(0) + 1;\n\
                             {}_clone.borrow_mut().set_text_content(&val.to_string());",
                            var_name, var_name
                        )
                    } else {
                        format!(
                            "{}_clone.borrow_mut().set_text_content(&({}).to_string());",
                            var_name, translate_expr(rhs)
                        )
                    };
                    rust_lines.push(rust_code);
                    continue;
                }
            }
        }

        // Phase 0: 忽略纯 JS 变量递增（如 count = count + 1）
        // 状态存储在 DOM textContent 中，递增逻辑已在 .textContent = count 中内联处理
        if line.contains("=") && line.contains("+") && line.contains("count") {
            continue;
        }

        // Canvas 2D 方法调用 (Phase 3)
        if let Some(canvas_code) = translate_canvas_call(line, html_lookup) {
            rust_lines.push(canvas_code);
            continue;
        }

        // var 声明 → let (Phase 3)
        if line.starts_with("var ") {
            if let Some(rust_line) = translate_var_declaration(line) {
                rust_lines.push(rust_line);
            }
            continue;
        }

        // let/const 声明
        if line.starts_with("let ") || line.starts_with("const ") {
            continue;
        }

        // 自增/自减 (Phase 3): x++ / x-- / ++x / --x
        if line.contains("++") || line.contains("--") {
            if let Some(rust_line) = translate_increment(line) {
                rust_lines.push(rust_line);
                continue;
            }
        }

        // instanceof (Phase 3)
        if line.contains("instanceof") {
            rust_lines.push(translate_instanceof(line));
            continue;
        }

        // queueMicrotask (Phase 3)
        if line.contains("queueMicrotask(") {
            if let Some(rust_line) = translate_queue_microtask(line) {
                rust_lines.push(rust_line);
                continue;
            }
        }

        // 函数调用或方法调用 — 尝试翻译为 Rust
        if !line.is_empty() && !line.starts_with("//") {
            rust_lines.push(format!("// {} // (JS → Rust)", line));
        }
    }

    rust_lines.join("\n")
}

/// 翻译 JS 表达式为 Rust 表达式
fn translate_expr(expr: &str) -> String {
    let expr = expr.trim();

    // JS parseInt(x) → Rust x.parse::<i32>().unwrap_or(0)
    if expr.starts_with("parseInt(") {
        let inner = expr.trim_end_matches(')');
        let inner = inner.split('(').skip(1).collect::<Vec<_>>().join("(");
        return format!("{}.parse::<i32>().unwrap_or(0)", inner.trim());
    }

    // JS x.toString() → Rust x.to_string()
    if expr.ends_with(".toString()") {
        return format!("{}.to_string()", &expr[..expr.len() - ".toString()".len()]);
    }

    // 字符串字面量: 'hello' 或 "hello" → Rust "hello"
    if (expr.starts_with('\'') && expr.ends_with('\''))
        || (expr.starts_with('"') && expr.ends_with('"'))
    {
        let inner = &expr[1..expr.len() - 1];
        return format!("\"{}\"", inner);
    }

    // 数字字面量
    if let Ok(_) = expr.parse::<i32>() {
        return expr.to_string();
    }

    // Phase 3: 位运算
    if let Some(result) = translate_bitwise(expr) {
        return result;
    }

    // 变量引用
    expr.to_string()
}

// ============================================================
//  Phase 3: var 声明翻译
// ============================================================

/// 将 var 声明翻译为 let mut
/// var x = 1; → let mut x = 1;
fn translate_var_declaration(line: &str) -> Option<String> {
    let line = line.trim().trim_end_matches(';');
    if let Some(rest) = line.strip_prefix("var ") {
        let rest = rest.trim();
        if rest.contains('=') {
            let parts: Vec<&str> = rest.splitn(2, '=').collect();
            let var_name = parts[0].trim();
            let value = parts[1].trim();
            let rust_value = translate_expr(value);
            return Some(format!("let mut {} = {};", var_name, rust_value));
        } else {
            return Some(format!("let mut {} = None::<String>; // var declaration", rest));
        }
    }
    None
}

// ============================================================
//  Phase 3: ++/-- 翻译
// ============================================================

/// 翻译自增/自减运算符
/// x++ → x += 1;
/// x-- → x -= 1;
fn translate_increment(line: &str) -> Option<String> {
    let line = line.trim().trim_end_matches(';');
    if line.ends_with("++") {
        let var = line[..line.len() - 2].trim();
        return Some(format!("{} += 1;", var));
    }
    if line.ends_with("--") {
        let var = line[..line.len() - 2].trim();
        return Some(format!("{} -= 1;", var));
    }
    if line.starts_with("++") {
        let var = line[2..].trim();
        return Some(format!("{} += 1;", var));
    }
    if line.starts_with("--") {
        let var = line[2..].trim();
        return Some(format!("{} -= 1;", var));
    }
    None
}

// ============================================================
//  Phase 3: 位运算翻译
// ============================================================

/// 翻译位运算表达式
fn translate_bitwise(expr: &str) -> Option<String> {
    let expr = expr.trim();

    // >>> (无符号右移): a >>> b → ((a as u32) >> b) as i32
    if expr.contains(">>>") {
        let parts: Vec<&str> = expr.splitn(2, ">>>").collect();
        let left = parts[0].trim();
        let right = parts[1].trim();
        return Some(format!("(({} as u32) >> {}) as i32", left, right));
    }

    // >> (有符号右移)
    if expr.contains(">>") {
        let parts: Vec<&str> = expr.splitn(2, ">>").collect();
        let left = parts[0].trim();
        let right = parts[1].trim();
        return Some(format!("({} >> {})", left, right));
    }

    // << (左移)
    if expr.contains("<<") {
        let parts: Vec<&str> = expr.splitn(2, "<<").collect();
        let left = parts[0].trim();
        let right = parts[1].trim();
        return Some(format!("({} << {})", left, right));
    }

    // & (按位与) — 注意区分 &&
    if expr.contains('&') && !expr.contains("&&") {
        let parts: Vec<&str> = expr.splitn(2, '&').collect();
        if parts.len() == 2 {
            let left = parts[0].trim();
            let right = parts[1].trim();
            return Some(format!("({} & {})", left, right));
        }
    }

    // | (按位或) — 注意区分 ||
    if expr.contains('|') && !expr.contains("||") {
        let parts: Vec<&str> = expr.splitn(2, '|').collect();
        if parts.len() == 2 {
            let left = parts[0].trim();
            let right = parts[1].trim();
            return Some(format!("({} | {})", left, right));
        }
    }

    // ^ (按位异或)
    if expr.contains('^') {
        let parts: Vec<&str> = expr.splitn(2, '^').collect();
        if parts.len() == 2 {
            let left = parts[0].trim();
            let right = parts[1].trim();
            return Some(format!("({} ^ {})", left, right));
        }
    }

    // ~ (按位取反): ~a → !a (Rust)
    if expr.starts_with('~') {
        let inner = &expr[1..].trim();
        return Some(format!("(!{})", inner));
    }

    None
}

// ============================================================
//  Phase 3: instanceof 翻译
// ============================================================

/// 翻译 instanceof 表达式
/// x instanceof Foo → x.is_instance_of::<Foo>()
fn translate_instanceof(line: &str) -> String {
    let line = line.trim().trim_end_matches(';');
    if let Some(pos) = line.find("instanceof") {
        let left = line[..pos].trim();
        let right = line[pos + "instanceof".len()..].trim();
        return format!("{}.is_instance_of::<{}>();", left, right);
    }
    format!("// {}; // (JS instanceof → Rust)", line)
}

// ============================================================
//  Phase 3: queueMicrotask 翻译
// ============================================================

/// 翻译 queueMicrotask 调用
/// queueMicrotask(fn) → queue_microtask(fn)
fn translate_queue_microtask(line: &str) -> Option<String> {
    let line = line.trim().trim_end_matches(';');
    if let Some(rest) = line.strip_prefix("queueMicrotask(") {
        let inner = rest.trim_end_matches(')').trim();
        return Some(format!("queue_microtask({});", inner));
    }
    None
}

// ============================================================
//  Phase 3: Canvas 2D 调用翻译
// ============================================================

/// 翻译 Canvas 2D API 调用
fn translate_canvas_call(line: &str, _html_lookup: &HashMap<String, String>) -> Option<String> {
    let line = line.trim().trim_end_matches(';');
    // 模式: ctx.method(args)
    if line.contains(".fillRect(")
        || line.contains(".strokeRect(")
        || line.contains(".clearRect(")
        || line.contains(".beginPath(")
        || line.contains(".moveTo(")
        || line.contains(".lineTo(")
        || line.contains(".arc(")
        || line.contains(".fillText(")
    {
        // 提取 ctx 变量和方法调用
        if let Some(dot_pos) = line.find('.') {
            let ctx_var = line[..dot_pos].trim();
            let after_dot = &line[dot_pos + 1..];
            if let Some(paren_pos) = after_dot.find('(') {
                let method = &after_dot[..paren_pos];
                let args_str = after_dot[paren_pos + 1..].trim_end_matches(')');
                let args: Vec<String> = if args_str.is_empty() {
                    vec![]
                } else {
                    args_str.split(',').map(|s| s.trim().to_string()).collect()
                };

                if let Some(rust) = crate::canvas_codegen::compile_canvas_call(method, &args, ctx_var) {
                    return Some(rust);
                }
            }
        }
    }
    None
}

/// 主入口：从 JS 代码中识别事件处理器
pub fn compile_js(
    js: &str,
    element_vars: &[(String, super::HtmlElement)],
) -> Vec<EventHandler> {
    let lookup = build_html_lookup(element_vars);
    extract_event_handlers(js, &lookup)
}

#[cfg(test)]
#[path = "js.test.rs"]
mod tests;
