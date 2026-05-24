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

use crate::css::{CssRule, selector_matches};

/// 为动态创建的元素查找匹配的 CSS 样式字符串
fn lookup_styles_for_element(tag: &str, class: Option<&str>, css_rules: &[CssRule]) -> Option<String> {
    let classes: Vec<String> = class
        .map(|c| c.split_whitespace().map(String::from).collect())
        .unwrap_or_default();

    let mut matched_decls: Vec<(String, String)> = Vec::new();
    for rule in css_rules {
        // 跳过无法验证父上下文的复合后代选择器
        // 如 .todo-item.completed span — 需要父元素同时具有两 class，仅靠元素自身信息无法判断
        if class.is_none() && has_compound_ancestor(&rule.selector) {
            continue;
        }
        if selector_matches(&rule.selector, tag, &classes, None, false) {
            for decl in &rule.declarations {
                matched_decls.push(decl.clone());
            }
        }
    }

    if matched_decls.is_empty() {
        return None;
    }

    let style_str = matched_decls.iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join("; ");
    Some(style_str)
}

/// 从 CSS 规则中提取 :last-child 覆盖信息
/// 返回 (base_selector, last_child_decls, base_decls) 列表
fn extract_last_child_overrides(css_rules: &[CssRule]) -> Vec<(String, Vec<(String, String)>, Vec<(String, String)>)> {
    let mut result = Vec::new();
    for rule in css_rules {
        let (base_sel, has_last_child) = strip_last_child_selector(&rule.selector);
        if !has_last_child {
            continue;
        }
        // 查找对应的基础规则（相同 base_selector 但无 :last-child）
        let base_decls: Vec<(String, String)> = css_rules.iter()
            .filter(|r| {
                let (_sel, has_lc) = strip_last_child_selector(&r.selector);
                !has_lc && _sel.trim() == base_sel.trim()
            })
            .flat_map(|r| r.declarations.clone())
            .collect();
        result.push((base_sel.to_string(), rule.declarations.clone(), base_decls));
    }
    result
}

/// 从选择器剥离 :last-child（独立版本，返回 String）
fn strip_last_child_selector(selector: &str) -> (String, bool) {
    if let Some(stripped) = selector.trim().strip_suffix(":last-child") {
        (stripped.trim().to_string(), true)
    } else {
        (selector.trim().to_string(), false)
    }
}

/// 从选择器提取用于运行时匹配的条件表达式（如 class.contains("todo-item")）
fn build_match_condition(selector: &str) -> String {
    let (base, _) = strip_last_child_selector(selector);
    let mut conditions = Vec::new();
    // 提取类名
    for part in base.split('.') {
        let cls = part.trim();
        if cls.is_empty() || cls.contains('#') || cls.contains(':') {
            continue;
        }
        // 如果有 #，我们的类名实际上在 # 之前
        let cls = cls.split('#').next().unwrap_or(cls);
        if !cls.is_empty() {
            conditions.push(format!("class.contains(\"{}\")", cls));
        }
    }
    conditions.join(" && ")
}

/// 检查后代选择器的祖先部分是否包含复合条件（多类选择器）
/// 例如 ".todo-item.completed span" → true（父 .todo-item.completed 有多条件）
/// ".todo-item span" → false（父 .todo-item 只有单条件）
fn has_compound_ancestor(selector: &str) -> bool {
    let parts: Vec<&str> = selector.split_whitespace().collect();
    if parts.len() <= 1 {
        return false;
    }
    for ancestor in &parts[..parts.len() - 1] {
        let dots = ancestor.matches('.').count();
        let hashes = ancestor.matches('#').count();
        let has_tag = !ancestor.starts_with('.') && !ancestor.starts_with('#');
        if dots + hashes + (has_tag as usize) > 1 {
            return true;
        }
    }
    false
}

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
    /// 共享数值变量名列表（如 count），闭包前需 clone Rc
    pub shared_vars: Vec<String>,
}

/// 共享状态变量信息
#[derive(Debug)]
pub struct SharedStateVar {
    pub name: String,
    pub initial_value: String,
    /// 哪些 handler 引用了此变量（按 handler 索引）
    pub used_by_handlers: Vec<usize>,
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

/// 检测 JS 中的共享状态变量（如 let count = 0;）
fn detect_shared_state_vars(lines: &[&str]) -> Vec<SharedStateVar> {
    let mut vars = Vec::new();
    for line in lines {
        let line = line.trim();
        if (line.starts_with("let ") || line.starts_with("const "))
            && line.contains('=')
            && !line.contains("document.")
        {
            let eq_pos = line.find('=').unwrap();
            let before = line[..eq_pos].trim();
            let var_name = if let Some(n) = before.strip_prefix("let ") {
                n.trim().to_string()
            } else if let Some(n) = before.strip_prefix("const ") {
                n.trim().to_string()
            } else {
                continue;
            };
            let rhs = line[eq_pos + 1..].trim().trim_end_matches(';').trim();
            // 仅当初始值是纯数字时视为共享状态
            if rhs.parse::<i32>().is_ok() {
                vars.push(SharedStateVar {
                    name: var_name,
                    initial_value: rhs.to_string(),
                    used_by_handlers: Vec::new(),
                });
            }
        }
    }
    vars
}

/// 从 JS 代码中提取事件处理器
pub fn extract_event_handlers(
    js: &str,
    html_lookup: &HashMap<String, String>,
    css_rules: &[CssRule],
) -> (Vec<EventHandler>, Vec<SharedStateVar>) {
    let mut handlers = Vec::new();

    // 逐行扫描
    let lines: Vec<&str> = js.lines().collect();

    // 检测共享状态变量
    let mut shared_state = detect_shared_state_vars(&lines);

    // 提取函数定义（用于内联）
    let function_defs = extract_function_defs(&lines);

    // Phase 0: 构建 JS 变量名 → 元素变量名 映射
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

    // 合并 HTML 元素查找表和 JS 变量映射
    let merged_lookup: HashMap<String, String> = html_lookup.iter()
        .chain(js_var_map.iter())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // 构建共享变量名集合（用于快速查找）
    let shared_var_names: Vec<String> = shared_state.iter().map(|s| s.name.clone()).collect();

    // 计算函数定义覆盖的行范围（这些行内的 addEventListener 应被跳过）
    let func_line_ranges: std::collections::HashSet<usize> = compute_function_line_ranges(&lines);

    let mut i = 0;

    while i < lines.len() {
        // 跳过函数定义内部的行
        if func_line_ranges.contains(&i) {
            i += 1;
            continue;
        }

        let line = lines[i].trim();

        if let Some(pos) = line.find(".addEventListener(") {
            let before = line[..pos].trim();
            let after = &line[pos + ".addEventListener(".len()..];

            let event_type = if after.starts_with('\'') || after.starts_with('"') {
                let quote = after.chars().next().unwrap();
                let end = after[1..].find(quote).map(|p| p + 1).unwrap_or(0);
                after[1..end].to_string()
            } else {
                String::new()
            };

            let fn_start = after.find("function");
            // 记录处理器体覆盖的行范围，用于后续跳过嵌套处理器
            let mut handler_end = i + 1;
            if let Some(fs) = fn_start {
                let body_open = after[fs..].find('{').map(|p| fs + p + 1);

                let mut body_lines = Vec::new();
                let mut depth = 1;

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
                handler_end = j;

                // 内联函数调用
                let inlined_body = inline_function_calls(&body_lines, &function_defs);

                let handler_idx = handlers.len();
                let mut cloned_vars = Vec::new();
                let mut handler_shared_vars = Vec::new();
                let rust_body = translate_event_body(
                    &inlined_body,
                    &merged_lookup,
                    &mut cloned_vars,
                    &shared_var_names,
                    &mut handler_shared_vars,
                    css_rules,
                );

                // 记录哪些 handler 使用了共享变量
                for sv_name in &handler_shared_vars {
                    for sv in &mut shared_state {
                        if &sv.name == sv_name {
                            sv.used_by_handlers.push(handler_idx);
                        }
                    }
                }

                let element_var = resolve_element_var(before, &merged_lookup);

                if let Some(ev) = element_var {
                    handlers.push(EventHandler {
                        element_var: ev,
                        event_type,
                        body_code: rust_body,
                        cloned_vars,
                        shared_vars: handler_shared_vars,
                    });
                }
            }
            // 跳过整个处理器体
            i = handler_end;
            continue;
        }

        i += 1;
    }

    (handlers, shared_state)
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

/// 函数定义信息（用于 Phase 0 内联）
#[derive(Debug, Clone)]
struct FunctionDef {
    name: String,
    params: Vec<String>,
    body_lines: Vec<String>,
}

/// 从 JS 源码中提取函数定义
fn extract_function_defs(lines: &[&str]) -> HashMap<String, FunctionDef> {
    let mut funcs = HashMap::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("function ") {
            let rest = line["function ".len()..].trim();
            if let Some(paren) = rest.find('(') {
                let name = rest[..paren].trim().to_string();
                let close_paren = rest[paren..].find(')').map(|p| paren + p).unwrap_or(rest.len());
                let params_str = &rest[paren + 1..close_paren];
                let params: Vec<String> = params_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                let body_start = rest[close_paren..].find('{').map(|p| close_paren + p + 1);
                if let Some(bs) = body_start {
                    let mut body = Vec::new();
                    let mut depth = 1;
                    let mut line_buf = String::new();
                    for ch in rest[bs..].chars() {
                        match ch {
                            '{' => depth += 1,
                            '}' => { depth -= 1; if depth == 0 { break; } }
                            '\n' => { if !line_buf.is_empty() { body.push(line_buf.clone()); line_buf.clear(); } }
                            c => if depth > 0 { line_buf.push(c); }
                        }
                    }
                    if !line_buf.is_empty() { body.push(line_buf); }
                    // 也检查后续行
                    let mut j = i + 1;
                    while j < lines.len() && depth > 0 {
                        let nl = lines[j];
                        let mut buf = String::new();
                        for ch in nl.chars() {
                            match ch {
                                '{' => depth += 1,
                                '}' => { depth -= 1; if depth == 0 { break; } }
                                c => if depth > 0 { buf.push(c); }
                            }
                        }
                        if !buf.is_empty() { body.push(buf); }
                        j += 1;
                    }
                    funcs.insert(name.clone(), FunctionDef { name, params, body_lines: body });
                }
            }
        }
        i += 1;
    }
    funcs
}

/// 计算函数定义覆盖的行范围
/// 返回所有在函数定义内部的行的索引集合
fn compute_function_line_ranges(lines: &[&str]) -> std::collections::HashSet<usize> {
    let mut ranges = std::collections::HashSet::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("function ") {
            // 查找函数体的 {
            let mut depth = 0;
            let mut started = false;
            let mut j = i;
            loop {
                if j >= lines.len() { break; }
                let content = if j == i { line } else { lines[j] };
                for ch in content.chars() {
                    match ch {
                        '{' => {
                            depth += 1;
                            started = true;
                        }
                        '}' => {
                            depth -= 1;
                            if depth == 0 && started {
                                // 函数体结束，记录 j 但不包括后续行
                                // 需要从循环中跳出
                            }
                        }
                        _ => {}
                    }
                }
                if started && depth == 0 {
                    // 函数体在同一行结束或跨多行结束
                    ranges.insert(j);
                    break;
                }
                if started {
                    ranges.insert(j);
                }
                j += 1;
            }
            i = j;
        }
        i += 1;
    }
    ranges
}

/// 翻译 JS if 条件为 Rust 条件
fn translate_if_condition(cond: &str) -> String {
    let cond = cond.trim();
    // text !== '' → !text.is_empty()
    if let Some(rest) = cond.strip_suffix("!== ''") {
        return format!("!{}.is_empty()", rest.trim());
    }
    if let Some(rest) = cond.strip_suffix("!== \"\"") {
        return format!("!{}.is_empty()", rest.trim());
    }
    if let Some(rest) = cond.strip_suffix("!= ''") {
        return format!("{} != \"\"", rest.trim());
    }
    if let Some(rest) = cond.strip_suffix("=== ''") {
        return format!("{}.is_empty()", rest.trim());
    }
    if let Some(rest) = cond.strip_suffix("== ''") {
        return format!("{} == \"\"", rest.trim());
    }
    if cond.contains("!==") {
        return cond.replace("!==", "!=");
    }
    if cond.contains("===") {
        return cond.replace("===", "==");
    }
    cond.to_string()
}

/// 内联函数调用：将 body_lines 中的函数调用替换为函数体
///
/// 支持两种模式：
/// 1. bare call: `funcName(args);` — return 值被丢弃
/// 2. assignment: `(const|let) varName = funcName(args);` — return 值赋给 varName
fn inline_function_calls(
    body_lines: &[String],
    function_defs: &HashMap<String, FunctionDef>,
) -> Vec<String> {
    let mut result = Vec::new();
    for line in body_lines {
        let trimmed = line.trim();
        let indent = line[..line.len() - line.trim_start().len()].to_string();
        let mut inlined = false;

        // 跳过控制流语句
        if trimmed.starts_with("if") || trimmed.starts_with("for") || trimmed.starts_with("while") {
            result.push(line.clone());
            continue;
        }

        // 检测函数调用
        if trimmed.ends_with(';') && !trimmed.starts_with("//") {
            if let Some(paren_pos) = trimmed.find('(') {
                let before_paren = trimmed[..paren_pos].trim();

                // 模式 1: bare call — funcName(args);
                if !before_paren.contains(' ') && !before_paren.contains('.') {
                    if let Some(expanded) = try_inline_call(
                        before_paren, None,
                        trimmed, paren_pos, function_defs, &indent,
                    ) {
                        result.extend(expanded);
                        inlined = true;
                    }
                }

                // 模式 2: assignment — (const|let) var = funcName(args);
                if !inlined && before_paren.contains('=') {
                    let eq_pos = before_paren.rfind('=').unwrap();
                    let lhs = before_paren[..eq_pos].trim(); // "const item" or "let item"
                    let rhs_func = before_paren[eq_pos + 1..].trim(); // "createTodoItem"

                    let assign_var = lhs.strip_prefix("const ")
                        .or_else(|| lhs.strip_prefix("let "))
                        .map(|s| s.trim().to_string());

                    if let Some(var_name) = assign_var {
                        if !rhs_func.contains('.') {
                            if let Some(expanded) = try_inline_call(
                                rhs_func, Some(&var_name),
                                trimmed, paren_pos, function_defs, &indent,
                            ) {
                                result.extend(expanded);
                                inlined = true;
                            }
                        }
                    }
                }
            }
        }

        if !inlined {
            result.push(line.clone());
        }
    }
    result
}

/// 尝试内联一个函数调用，返回展开后的行列表
fn try_inline_call(
    func_name: &str,
    assign_to: Option<&str>, // 如果 Some(var)，return expr 转为 let var = expr;
    line: &str,
    paren_pos: usize,
    function_defs: &HashMap<String, FunctionDef>,
    base_indent: &str,
) -> Option<Vec<String>> {
    let func_def = function_defs.get(func_name)?;
    let close_paren = line[paren_pos..].find(')')?;
    let args_str = &line[paren_pos + 1..paren_pos + close_paren];
    let args: Vec<String> = if args_str.trim().is_empty() {
        vec![]
    } else {
        args_str.split(',').map(|s| s.trim().to_string()).collect()
    };

    let mut expanded = Vec::new();
    for body_line in &func_def.body_lines {
        let mut substituted = body_line.trim().to_string();
        for (i, param) in func_def.params.iter().enumerate() {
            if i < args.len() {
                substituted = substituted.replace(param, &args[i]);
            }
        }

        // 处理 return 语句：return expr; → let assign_to = expr;
        let trimmed_sub = substituted.trim();
        if let Some(assigned) = assign_to {
            if trimmed_sub.starts_with("return ") {
                let ret_expr = trimmed_sub["return ".len()..].trim_end_matches(';').trim();
                expanded.push(format!("{}let {} = {};", base_indent, assigned, ret_expr));
                continue;
            }
        } else {
            // bare call: 移除 return 语句
            if trimmed_sub.starts_with("return ") {
                continue;
            }
        }

        // 递归内联嵌套函数调用（保持 base_indent）
        let indented = format!("{}{}", base_indent, substituted);
        let nested = inline_function_calls(&[indented], function_defs);
        expanded.extend(nested);
    }
    Some(expanded)
}

/// 翻译事件处理器体（增强版：支持 if/else、.value、createElement、appendChild 等）
fn translate_event_body(
    body_lines: &[String],
    html_lookup: &HashMap<String, String>,
    cloned_vars: &mut Vec<String>,
    shared_var_names: &[String],
    handler_shared_vars: &mut Vec<String>,
    css_rules: &[CssRule],
) -> String {
    let mut local_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut scope_locals: std::collections::HashSet<String> = std::collections::HashSet::new();
    translate_event_body_inner(body_lines, html_lookup, cloned_vars, shared_var_names, handler_shared_vars, &mut local_vars, &mut scope_locals, css_rules)
}

/// 解析变量引用，优先检查局部变量
fn resolve_with_locals(
    name: &str,
    html_lookup: &HashMap<String, String>,
    local_vars: &std::collections::HashSet<String>,
) -> String {
    if local_vars.contains(name) {
        return name.to_string();
    }
    resolve_element_var(name, html_lookup).unwrap_or_else(|| name.to_string())
}

/// 检查变量名是否对应局部变量（无需 clone）
fn is_local_var(name: &str, local_vars: &std::collections::HashSet<String>) -> bool {
    local_vars.contains(name)
}

/// 获取用于 borrow_mut 的变量名：当前作用域局部变量直接用原名，外部变量用 _clone
fn borrow_var_name(var_name: &str, scope_locals: &std::collections::HashSet<String>) -> String {
    if scope_locals.contains(var_name) {
        var_name.to_string()
    } else {
        format!("{}_clone", var_name)
    }
}

/// 内部实现：带局部变量跟踪
/// `scope_locals`: 仅包含当前作用域内创建的变量（不含继承的），用于判断是否需要 _clone 后缀
fn translate_event_body_inner(
    body_lines: &[String],
    html_lookup: &HashMap<String, String>,
    cloned_vars: &mut Vec<String>,
    shared_var_names: &[String],
    handler_shared_vars: &mut Vec<String>,
    local_vars: &mut std::collections::HashSet<String>,
    scope_locals: &mut std::collections::HashSet<String>,
    css_rules: &[CssRule],
) -> String {
    let mut rust_lines = Vec::new();
    let mut idx: usize = 0;

    // 跟踪 doc 是否需要 clone
    let mut _need_doc_clone = false;

    while idx < body_lines.len() {
        let line = body_lines[idx].trim();
        idx += 1;

        if line.is_empty() || line == "{" || line == "}" {
            continue;
        }
        if line.contains("function") && !line.contains(".addEventListener(") {
            continue;
        }

        // ============================================================
        // 模式: if (condition) { ... }
        // 注意：body_lines 中 { 和 } 已被剥离，用缩进判断 if 体边界
        // ============================================================
        if line.starts_with("if (") || line.starts_with("if(") {
            let cond_start = line.find('(').unwrap() + 1;
            let cond_end = line.rfind(')').unwrap_or(line.len());
            let js_cond = &line[cond_start..cond_end];
            let rust_cond = translate_if_condition(js_cond);
            // 用缩进确定 if 体范围：if 体行缩进 > if 行缩进
            let if_indent = body_lines[idx - 1].chars().take_while(|c| c.is_whitespace()).count();
            let mut body_end = idx;
            while body_end < body_lines.len() {
                if body_lines[body_end].trim().is_empty() {
                    body_end += 1;
                    continue;
                }
                let next_indent = body_lines[body_end].chars().take_while(|c| c.is_whitespace()).count();
                if next_indent <= if_indent {
                    break;
                }
                body_end += 1;
            }
            let inner_lines: Vec<String> = body_lines[idx..body_end].to_vec();
            let inner_body = translate_event_body_inner(&inner_lines, html_lookup, cloned_vars, shared_var_names, handler_shared_vars, local_vars, scope_locals, css_rules);
            rust_lines.push(format!("if {} {{", rust_cond));
            for inner_line in inner_body.lines() {
                rust_lines.push(format!("    {}", inner_line));
            }
            rust_lines.push("}".to_string());
            idx = body_end;
            continue;
        }

        // ============================================================
        // 模式: const/let var = document.createElement('tag');
        // ============================================================
        if (line.starts_with("const ") || line.starts_with("let ")) && line.contains("document.createElement(") {
            if let Some(eq_pos) = line.find('=') {
                let var_name = {
                    let before = line[..eq_pos].trim();
                    before.strip_prefix("const ").or_else(|| before.strip_prefix("let ")).map(|s| s.trim().to_string())
                };
                if let Some(vn) = var_name {
                    let after_eq = line[eq_pos + 1..].trim().trim_end_matches(';').trim();
                    if let Some(tag) = after_eq.strip_prefix("document.createElement(").and_then(|s| s.trim_end_matches(')').trim().trim_matches('\'').to_string().into()).or_else(|| {
                        after_eq.strip_prefix("document.createElement(").map(|s| s.trim_end_matches(')').trim().trim_matches('"').to_string())
                    }) {
                        _need_doc_clone = true;
                        if !cloned_vars.contains(&"doc_clone".to_string()) {
                            cloned_vars.push("doc_clone".to_string());
                        }
                        local_vars.insert(vn.clone());
                        scope_locals.insert(vn.clone());
                        rust_lines.push(format!("let {} = doc_clone.borrow().create_element(\"{}\");", vn, tag));
                        // 为动态创建的元素查找并应用 CSS 样式（仅标签匹配，如 .todo-item span → tag="span"）
                        if let Some(style_str) = lookup_styles_for_element(&tag, None, css_rules) {
                            let escaped = style_str.replace('\\', "\\\\").replace('"', "\\\"");
                            rust_lines.push(format!("{}.borrow_mut().set_style(\"{}\");", vn, escaped));
                        }
                        continue;
                    }
                }
            }
        }

        // ============================================================
        // 模式: element.className = 'name';
        // ============================================================
        if line.contains(".className") && line.contains('=') {
            if let Some(pos) = line.find(".className") {
                let target = line[..pos].trim();
                let rhs_start = line[pos + ".className".len()..].find('=').map(|p| pos + ".className".len() + p + 1);
                if let Some(rs) = rhs_start {
                    let rhs = line[rs..].trim_end_matches(';').trim().trim_matches('\'').trim_matches('"');
                    let var_name = resolve_with_locals(target, html_lookup, local_vars);
                    rust_lines.push(format!("{}.borrow_mut().set_attribute(\"class\", \"{}\");", var_name, rhs));
                    // 为动态创建的元素查找并应用 CSS 样式（类名匹配）
                    if let Some(style_str) = lookup_styles_for_element("*", Some(rhs), css_rules) {
                        let escaped = style_str.replace('\\', "\\\\").replace('"', "\\\"");
                        rust_lines.push(format!("{}.borrow_mut().set_style(\"{}\");", var_name, escaped));
                    }
                    continue;
                }
            }
        }

        // ============================================================
        // 模式: element.value (getter) — 读取输入值
        // ============================================================
        if line.contains(".value") && !line.contains(".value =") && !line.contains(".value=") {
            // 这个模式主要用于表达式右值，如: const text = input.value.trim();
            // 转换为: let text = ...text_content().trim()...
            if (line.starts_with("const ") || line.starts_with("let ")) && line.contains('=') {
                if let Some(eq_pos) = line.find('=') {
                    let var_name = {
                        let before = line[..eq_pos].trim();
                        before.strip_prefix("const ").or_else(|| before.strip_prefix("let ")).map(|s| s.trim().to_string())
                    };
                    let rhs = line[eq_pos + 1..].trim().trim_end_matches(';').trim();
                    if let Some(vn) = var_name {
                        let translated = translate_rhs_expr(rhs, html_lookup, cloned_vars, shared_var_names, handler_shared_vars, local_vars, scope_locals);
                        rust_lines.push(format!("let {} = {};", vn, translated));
                        continue;
                    }
                }
            }
        }

        // ============================================================
        // 模式: element.value = expr (setter) — 设置输入值
        // ============================================================
        if (line.contains(".value =") || line.contains(".value=")) && line.ends_with(';') {
            let op = if line.contains(".value =") { ".value =" } else { ".value=" };
            if let Some(pos) = line.find(op) {
                let target = line[..pos].trim();
                let rhs = line[pos + op.len()..].trim_end_matches(';').trim();
                let var_name = resolve_with_locals(target, html_lookup, local_vars);
                let use_name = borrow_var_name(&var_name, scope_locals);
                if !is_local_var(&var_name, scope_locals) {
                    let var_clone = format!("{}_clone", var_name);
                    if !cloned_vars.contains(&var_clone) {
                        cloned_vars.push(var_clone);
                    }
                }
                if rhs == "''" || rhs == "\"\"" {
                    rust_lines.push(format!("{}.borrow_mut().set_text_content(\"\");", use_name));
                } else {
                    rust_lines.push(format!("{}.borrow_mut().set_text_content(&({}).to_string());", use_name, translate_expr(rhs)));
                }
                continue;
            }
        }

        // ============================================================
        // 模式: element.textContent = expr;
        // ============================================================
        if line.contains(".textContent") {
            if let Some(pos) = line.find(".textContent") {
                let target = line[..pos].trim();
                let rhs_start = line[pos + ".textContent".len()..].find('=').map(|p| pos + ".textContent".len() + p + 1);
                if let Some(rs) = rhs_start {
                    let rhs = line[rs..].trim_end_matches(';').trim();
                    let var_name = resolve_with_locals(target, html_lookup, local_vars);
                    let use_name = borrow_var_name(&var_name, scope_locals);
                    if !is_local_var(&var_name, scope_locals) {
                        let var_clone = format!("{}_clone", var_name);
                        if !cloned_vars.contains(&var_clone) {
                            cloned_vars.push(var_clone);
                        }
                    }

                    // 检查 RHS 是否引用了共享变量
                    let mut uses_shared = false;
                    let mut matched_sv = String::new();
                    for sv in shared_var_names {
                        if rhs == sv.as_str() || rhs.contains(&format!("{} ", sv)) || rhs.starts_with(sv) {
                            uses_shared = true;
                            matched_sv = sv.clone();
                            if !handler_shared_vars.contains(sv) {
                                handler_shared_vars.push(sv.clone());
                            }
                            break;
                        }
                    }

                    if uses_shared {
                        // 支持 todoCount + ' items' 模式 → format!("{} items", ...)
                        if rhs.contains('+') {
                            // 找到共享变量部分和字符串部分 → format!("...{}...", var)
                            let parts: Vec<&str> = rhs.split('+').map(|s| s.trim()).collect();
                            let mut fmt_lit = String::new();
                            for p in &parts {
                                let clean = p.trim().trim_matches('\'').trim_matches('"');
                                if clean == matched_sv {
                                    fmt_lit.push_str("{}");
                                } else {
                                    fmt_lit.push_str(clean);
                                }
                            }
                            rust_lines.push(format!(
                                "{}.borrow_mut().set_text_content(&format!(\"{}\", {}_clone.borrow()));",
                                use_name, fmt_lit, matched_sv
                            ));
                        } else {
                            rust_lines.push(format!(
                                "{}.borrow_mut().set_text_content(&{}_clone.borrow().to_string());",
                                use_name, matched_sv
                            ));
                        }
                    } else {
                        // 字符串字面量：直接设置
                        if (rhs.starts_with('\'') && rhs.ends_with('\'')) || (rhs.starts_with('"') && rhs.ends_with('"')) {
                            let inner = &rhs[1..rhs.len()-1];
                            rust_lines.push(format!(
                                "{}.borrow_mut().set_text_content(\"{}\");",
                                use_name, inner
                            ));
                        } else if rhs.parse::<i32>().is_ok() {
                            rust_lines.push(format!(
                                "{}.borrow_mut().set_text_content(\"{}\");",
                                use_name, rhs
                            ));
                        } else {
                            rust_lines.push(format!(
                                "{}.borrow_mut().set_text_content(&({}).to_string());",
                                use_name, translate_expr(rhs)
                            ));
                        }
                    }
                    continue;
                }
            }
        }

        // ============================================================
        // 模式: element.appendChild(child);
        // ============================================================
        if line.contains(".appendChild(") {
            if let Some(pos) = line.find(".appendChild(") {
                let target = line[..pos].trim();
                let child = line[pos + ".appendChild(".len()..].trim().trim_end_matches(';').trim_end_matches(')').trim();
                let var_name = resolve_with_locals(target, html_lookup, local_vars);
                let use_name = borrow_var_name(&var_name, scope_locals);
                if !is_local_var(&var_name, scope_locals) {
                    let var_clone = format!("{}_clone", var_name);
                    if !cloned_vars.contains(&var_clone) {
                        cloned_vars.push(var_clone);
                    }
                }
                let child_var = resolve_with_locals(child, html_lookup, local_vars);
                let child_use = borrow_var_name(&child_var, scope_locals);
                if !is_local_var(&child_var, scope_locals) {
                    let child_clone = format!("{}_clone", child_var);
                    if !cloned_vars.contains(&child_clone) {
                        cloned_vars.push(child_clone);
                    }
                }
                rust_lines.push(format!("{}.borrow_mut().append_child({}.clone());", use_name, child_use));
                // 为 :last-child 伪类生成运行时样式修复代码
                let last_child_overrides = extract_last_child_overrides(css_rules);
                if !last_child_overrides.is_empty() {
                    rust_lines.push(format!("{{"));
                    rust_lines.push(format!("    let parent_ref = {}.borrow();", use_name));
                    rust_lines.push(format!("    let children = parent_ref.child_nodes();"));
                    rust_lines.push(format!("    let len = children.len();"));
                    rust_lines.push(format!("    drop(parent_ref);"));
                    rust_lines.push(format!("    for (i, child_rc) in children.iter().enumerate() {{"));
                    rust_lines.push(format!("        let mut child_ref = child_rc.borrow_mut();"));
                    rust_lines.push(format!("        let class = child_ref.get_attribute(\"class\").unwrap_or_default();"));
                    for (base_sel, last_decls, base_decls) in &last_child_overrides {
                        let condition = build_match_condition(base_sel);
                        let overridden_props: Vec<&str> = last_decls.iter().map(|(k, _)| k.as_str()).collect();
                        let base_overrides: Vec<&(String, String)> = base_decls.iter()
                            .filter(|(k, _)| overridden_props.contains(&k.as_str()))
                            .collect();
                        if !condition.is_empty() {
                            rust_lines.push(format!("        if {} {{", condition));
                        }
                        if !last_decls.is_empty() && !base_overrides.is_empty() {
                            rust_lines.push(format!("            if i == len - 1 {{"));
                            for (prop, value) in last_decls {
                                rust_lines.push(format!("                child_ref.set_style_property(\"{}\", \"{}\");", prop, value));
                            }
                            rust_lines.push(format!("            }} else {{"));
                            for (prop, value) in &base_overrides {
                                rust_lines.push(format!("                child_ref.set_style_property(\"{}\", \"{}\");", prop, value));
                            }
                            rust_lines.push(format!("            }}"));
                        }
                        if !condition.is_empty() {
                            rust_lines.push(format!("        }}"));
                        }
                    }
                    rust_lines.push(format!("    }}"));
                    rust_lines.push(format!("}}"));
                }
                continue;
            }
        }

        // ============================================================
        // 模式: element.removeChild(child);
        // ============================================================
        if line.contains(".removeChild(") {
            if let Some(pos) = line.find(".removeChild(") {
                let target = line[..pos].trim();
                let child = line[pos + ".removeChild(".len()..].trim().trim_end_matches(';').trim_end_matches(')').trim();
                let var_name = resolve_with_locals(target, html_lookup, local_vars);
                let use_name = borrow_var_name(&var_name, scope_locals);
                if !is_local_var(&var_name, scope_locals) {
                    let var_clone = format!("{}_clone", var_name);
                    if !cloned_vars.contains(&var_clone) {
                        cloned_vars.push(var_clone);
                    }
                }
                let child_var = resolve_with_locals(child, html_lookup, local_vars);
                let child_use = borrow_var_name(&child_var, scope_locals);
                if !is_local_var(&child_var, scope_locals) {
                    let child_clone = format!("{}_clone", child_var);
                    if !cloned_vars.contains(&child_clone) {
                        cloned_vars.push(child_clone);
                    }
                }
                rust_lines.push(format!("{}.borrow_mut().remove_child_by_ptr(&{});", use_name, child_use));
                continue;
            }
        }

        // ============================================================
        // 模式: return expr;
        // ============================================================
        if line.starts_with("return ") {
            let expr = line["return ".len()..].trim_end_matches(';').trim();
            let child_var = resolve_with_locals(expr, html_lookup, local_vars);
            rust_lines.push(format!("return {};", child_var));
            continue;
        }

        // ============================================================
        // 共享变量递增/递减/赋值
        // ============================================================
        let mut handled_shared = false;
        for sv in shared_var_names {
            let cleaned = line.replace(' ', "").trim_end_matches(';').to_string();
            // count = count + 1
            let pat_inc1 = format!("{}={}+1", sv, sv);
            let pat_inc2 = format!("{0}={0}+1", sv);
            // count = count - 1
            let pat_dec1 = format!("{}={}-1", sv, sv);
            let pat_dec2 = format!("{0}={0}-1", sv);
            // count = 0 / count = N
            let pat_set = format!("{}=", sv);

            if cleaned == pat_inc1 || cleaned == pat_inc2 {
                if !handler_shared_vars.contains(sv) {
                    handler_shared_vars.push(sv.clone());
                }
                rust_lines.push(format!("*{}_clone.borrow_mut() += 1;", sv));
                handled_shared = true;
                break;
            }
            if cleaned == pat_dec1 || cleaned == pat_dec2 {
                if !handler_shared_vars.contains(sv) {
                    handler_shared_vars.push(sv.clone());
                }
                rust_lines.push(format!("*{}_clone.borrow_mut() -= 1;", sv));
                handled_shared = true;
                break;
            }
            if cleaned.starts_with(&pat_set) && !cleaned.contains("++") && !cleaned.contains("--") {
                let rhs = cleaned[pat_set.len()..].to_string();
                if let Ok(_n) = rhs.parse::<i32>() {
                    if !handler_shared_vars.contains(sv) {
                        handler_shared_vars.push(sv.clone());
                    }
                    rust_lines.push(format!("*{}_clone.borrow_mut() = {};", sv, rhs));
                    handled_shared = true;
                    break;
                }
            }
        }
        if handled_shared {
            continue;
        }

        // var 声明 → let (Phase 3)
        if line.starts_with("var ") {
            if let Some(rust_line) = translate_var_declaration(line) {
                rust_lines.push(rust_line);
            }
            continue;
        }

        // let/const 声明（无 document.createElement）
        // 如 let item = li; → Rust: let item = li.clone();
        if (line.starts_with("let ") || line.starts_with("const ")) && line.contains('=') {
            if let Some(eq_pos) = line.find('=') {
                let var_name = {
                    let before = line[..eq_pos].trim();
                    before.strip_prefix("const ").or_else(|| before.strip_prefix("let ")).map(|s| s.trim().to_string())
                };
                let rhs = line[eq_pos + 1..].trim().trim_end_matches(';').trim();
                if let Some(vn) = var_name {
                    local_vars.insert(vn.clone());
                    scope_locals.insert(vn.clone());
                    // 如果 RHS 是简单变量引用，生成 clone 赋值
                    let rhs_var = resolve_with_locals(rhs, html_lookup, local_vars);
                    rust_lines.push(format!("let {} = {}.clone();", vn, rhs_var));
                }
            }
            continue;
        }

        // ============================================================
        // 模式: element.addEventListener('event', function()  body );
        // 用于处理内联函数体中的嵌套事件监听器
        // body 中的 { } 已被函数提取器消费，只剩内容文本
        // 注意：此检查必须放在 .classList.toggle 之前，因为内联后的行可能同时包含两者
        // ============================================================
        if line.contains(".addEventListener(") {
            if let Some(pos) = line.find(".addEventListener(") {
                let target = line[..pos].trim();
                let after = &line[pos + ".addEventListener(".len()..];

                let event_type = if after.starts_with('\'') || after.starts_with('"') {
                    let quote = after.chars().next().unwrap();
                    let end = after[1..].find(quote).map(|p| p + 1).unwrap_or(0);
                    after[1..end].to_string()
                } else {
                    String::new()
                };

                // 提取内联函数体（{ } 已被剥离）
                let fn_start = after.find("function");
                if let Some(fs) = fn_start {
                    let after_fn = &after[fs + "function".len()..];
                    let paren_end = after_fn.find(')').unwrap_or(0);
                    // 函数体在两个空格之后（原来 { 和 } 的位置被消费了）
                    let body_start = paren_end + 1;
                    let inner_body = after_fn[body_start..].trim().trim_end_matches(';').trim_end_matches(')').trim();
                    // inner_body 可能有残留的 ; 和 )
                    let inner_body = inner_body.trim_end_matches(')').trim();

                    let var_name = resolve_with_locals(target, html_lookup, local_vars);
                    let use_name = borrow_var_name(&var_name, scope_locals);
                    if !is_local_var(&var_name, scope_locals) {
                        let var_clone = format!("{}_clone", var_name);
                        if !cloned_vars.contains(&var_clone) {
                            cloned_vars.push(var_clone);
                        }
                    }

                    if inner_body.is_empty() {
                        // 跨行情况：函数体的 { 和 } 已被消耗，内容在后续行
                        // 收集后续行直到 ); 为止
                        let mut collected_body = String::new();
                        while idx < body_lines.len() {
                            let next_line = body_lines[idx].trim();
                            idx += 1;
                            if next_line == ");" || next_line == ")" {
                                break;
                            }
                            if !collected_body.is_empty() {
                                collected_body.push(' ');
                            }
                            collected_body.push_str(next_line);
                        }
                        if !collected_body.is_empty() {
                            let collected_body = collected_body.trim().trim_end_matches(';').to_string();
                            if !collected_body.is_empty() {
                                // 将收集到的内容作为 inner_body 处理
                                let inner_body_lines: Vec<String> = collected_body.split(';').filter(|s| !s.trim().is_empty()).map(|s| format!("{};", s.trim())).collect();

                                let mut inner_cloned: Vec<String> = Vec::new();
                                let mut inner_shared: Vec<String> = Vec::new();
                                let mut inner_local: std::collections::HashSet<String> = std::collections::HashSet::new();
                                for lv in local_vars.iter() {
                                    inner_local.insert(lv.clone());
                                }

                                let mut inner_scope_locals: std::collections::HashSet<String> = std::collections::HashSet::new();
                                let inner_rust = translate_event_body_inner(
                                    &inner_body_lines,
                                    html_lookup,
                                    &mut inner_cloned,
                                    shared_var_names,
                                    &mut inner_shared,
                                    &mut inner_local,
                                    &mut inner_scope_locals,
                                    css_rules,
                                );

                                // 为嵌套闭包生成 clone 变量。
                                // 对于外部变量（不在 local_vars 中的），使用外层 _clone 作为源并重命名避免遮蔽。
                                let mut rename_map: Vec<(String, String)> = Vec::new();
                                for ic in &inner_cloned {
                                    let base = ic.trim_end_matches("_clone").to_string();
                                    if base != "doc" {
                                        if !local_vars.contains(&base) {
                                            let new_name = format!("{}_2", ic);
                                            rename_map.push((ic.clone(), new_name.clone()));
                                            rust_lines.push(format!("let {} = {}.clone();", new_name, ic));
                                        } else {
                                            rust_lines.push(format!("let {} = {}.clone();", ic, base));
                                        }
                                    }
                                }
                                for sv_name in &inner_shared {
                                    if !local_vars.contains(sv_name) {
                                        let old_name = format!("{}_clone", sv_name);
                                        let new_name = format!("{}_clone_2", sv_name);
                                        rename_map.push((old_name.clone(), new_name.clone()));
                                        rust_lines.push(format!("let {} = {}.clone();", new_name, old_name));
                                    } else {
                                        rust_lines.push(format!("let {}_clone = {}.clone();", sv_name, sv_name));
                                    }
                                }
                                let mut inner_rust = inner_rust;
                                for (old_name, new_name) in &rename_map {
                                    inner_rust = inner_rust.replace(old_name, new_name);
                                }
                                rust_lines.push(format!(
                                    "{0}.borrow_mut().add_event_listener(\"{1}\", Box::new(move |_: &dom::Event| {{",
                                    use_name, event_type
                                ));
                                for inner_line in inner_rust.lines() {
                                    rust_lines.push(format!("    {}", inner_line));
                                }
                                rust_lines.push("}));".to_string());
                                continue;
                            }
                        }
                    } else if !inner_body.is_empty() {
                        // 构建内部事件体的 Rust 代码
                        let inner_body_lines: Vec<String> = inner_body.split(';').filter(|s| !s.trim().is_empty()).map(|s| format!("{};", s.trim())).collect();

                        // 为内部闭包准备变量（需要 clone 的）
                        // 简化处理：inner event 使用自己独立的 cloned_vars 列表
                        let mut inner_cloned: Vec<String> = Vec::new();
                        let mut inner_shared: Vec<String> = Vec::new();
                        let mut inner_local: std::collections::HashSet<String> = std::collections::HashSet::new();
                        // 继承外部 local_vars
                        for lv in local_vars.iter() {
                            inner_local.insert(lv.clone());
                        }

                        let mut inner_scope_locals: std::collections::HashSet<String> = std::collections::HashSet::new();
                        let inner_rust = translate_event_body_inner(
                            &inner_body_lines,
                            html_lookup,
                            &mut inner_cloned,
                            shared_var_names,
                            &mut inner_shared,
                            &mut inner_local,
                            &mut inner_scope_locals,
                            css_rules,
                        );

                        // 生成嵌套闭包代码。对于外部变量（不在 local_vars 中的），使用外层 _clone 作为源并重命名避免遮蔽。
                        let mut rename_map: Vec<(String, String)> = Vec::new();
                        for ic in &inner_cloned {
                            let base = ic.trim_end_matches("_clone").to_string();
                            if base != "doc" {
                                if !local_vars.contains(&base) {
                                    let new_name = format!("{}_2", ic);
                                    rename_map.push((ic.clone(), new_name.clone()));
                                    rust_lines.push(format!("let {} = {}.clone();", new_name, ic));
                                } else {
                                    rust_lines.push(format!("let {} = {}.clone();", ic, base));
                                }
                            }
                        }
                        for sv_name in &inner_shared {
                            if !local_vars.contains(sv_name) {
                                let old_name = format!("{}_clone", sv_name);
                                let new_name = format!("{}_clone_2", sv_name);
                                rename_map.push((old_name.clone(), new_name.clone()));
                                rust_lines.push(format!("let {} = {}.clone();", new_name, old_name));
                            } else {
                                rust_lines.push(format!("let {}_clone = {}.clone();", sv_name, sv_name));
                            }
                        }
                        let mut inner_rust = inner_rust;
                        for (old_name, new_name) in &rename_map {
                            inner_rust = inner_rust.replace(old_name, new_name);
                        }
                        rust_lines.push(format!(
                            "{0}.borrow_mut().add_event_listener(\"{1}\", Box::new(move |_: &dom::Event| {{",
                            use_name, event_type
                        ));
                        for inner_line in inner_rust.lines() {
                            rust_lines.push(format!("    {}", inner_line));
                        }
                        rust_lines.push("}));".to_string());
                        continue;
                    }
                }
            }
        }

        // ============================================================
        // 模式: element.classList.toggle('class')
        // → get_attribute / set_attribute / set_style
        // ============================================================
        if line.contains(".classList.toggle(") {
            if let Some(pos) = line.find(".classList.toggle(") {
                let target = line[..pos].trim();
                let class_val = line[pos + ".classList.toggle(".len()..].trim().trim_end_matches(';').trim_end_matches(')').trim().trim_matches('\'').trim_matches('"');
                let var_name = resolve_with_locals(target, html_lookup, local_vars);
                let use_name = borrow_var_name(&var_name, scope_locals);
                if !is_local_var(&var_name, scope_locals) {
                    let var_clone = format!("{}_clone", var_name);
                    if !cloned_vars.contains(&var_clone) {
                        cloned_vars.push(var_clone);
                    }
                }
                rust_lines.push(format!("let mut {0}_ref = {1}.borrow_mut();", var_name, use_name));
                rust_lines.push(format!("let current = {0}_ref.get_attribute(\"class\").unwrap_or_default();", var_name));
                rust_lines.push(format!("if current.contains(\"{}\") {{", class_val));
                rust_lines.push(format!("    {0}_ref.set_attribute(\"class\", &current.replace(\" {1}\", \"\").replace(\"{1}\", \"\"));", var_name, class_val));
                rust_lines.push(format!("    {0}_ref.set_style(\"text-decoration: none; color: #333\");", var_name));
                rust_lines.push("} else {".to_string());
                rust_lines.push(format!("    {0}_ref.set_attribute(\"class\", &format!(\"{{}} {1}\", current.trim()));", var_name, class_val));
                rust_lines.push(format!("    {0}_ref.set_style(\"text-decoration: line-through; color: #999\");", var_name));
                rust_lines.push("}".to_string());
                continue;
            }
        }

        // 自增/自减 (Phase 3)
        if line.contains("++") || line.contains("--") {
            if let Some(rust_line) = translate_increment(line) {
                rust_lines.push(rust_line);
                continue;
            }
        }

        // Canvas 2D 方法调用 (Phase 3)
        if let Some(canvas_code) = translate_canvas_call(line, html_lookup) {
            rust_lines.push(canvas_code);
            continue;
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

        // 函数调用或方法调用
        if !line.is_empty() && !line.starts_with("//") {
            rust_lines.push(format!("// {} // (JS → Rust)", line));
        }
    }

    rust_lines.join("\n")
}

/// 翻译右值表达式（含 .value、.trim() 等）
fn translate_rhs_expr(
    expr: &str,
    html_lookup: &HashMap<String, String>,
    cloned_vars: &mut Vec<String>,
    _shared_var_names: &[String],
    _handler_shared_vars: &mut Vec<String>,
    local_vars: &std::collections::HashSet<String>,
    scope_locals: &std::collections::HashSet<String>,
) -> String {
    let expr = expr.trim();
    // 处理 .value.trim() 或 .value
    if expr.contains(".value") {
        if let Some(dot_pos) = expr.rfind(".value") {
            let target = &expr[..dot_pos];
            let var_name = resolve_with_locals(target, html_lookup, local_vars);
            let use_name = borrow_var_name(&var_name, scope_locals);
            if !is_local_var(&var_name, scope_locals) {
                if !cloned_vars.contains(&var_name) {
                    cloned_vars.push(format!("{}_clone", var_name));
                }
            }
            let rest = &expr[dot_pos + ".value".len()..];
            if rest.starts_with(".trim()") {
                return format!("{}.borrow().text_content().trim().to_string()", use_name);
            }
            return format!("{}.borrow().text_content()", use_name);
        }
    }
    translate_expr(expr)
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

/// 主入口：从 JS 代码中识别事件处理器和共享状态
pub fn compile_js(
    js: &str,
    element_vars: &[(String, super::HtmlElement)],
    css_rules: &[CssRule],
) -> (Vec<EventHandler>, Vec<SharedStateVar>) {
    let lookup = build_html_lookup(element_vars);
    extract_event_handlers(js, &lookup, css_rules)
}

#[cfg(test)]
#[path = "js.test.rs"]
mod tests;
