//! 内置对象映射
//!
//! Phase 1: 将 JS 标准库 API 映射为 Rust 等价实现。
//! Phase 2+: 完整 Web API 映射（fetch, localStorage, etc.）

/// 内置 API 映射条目
pub struct BuiltinMapping {
    /// JS API 名称（如 "console.log"）
    pub js_api: &'static str,
    /// Rust 实现模板（{0}, {1}... 为参数占位符）
    pub rust_impl: &'static str,
}

/// 获取 Console API 映射
pub fn console_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "console.log",
            rust_impl: "println!(\"[console.log] {}\", {0})",
        },
        BuiltinMapping {
            js_api: "console.error",
            rust_impl: "eprintln!(\"[console.error] {}\", {0})",
        },
        BuiltinMapping {
            js_api: "console.warn",
            rust_impl: "eprintln!(\"[console.warn] {}\", {0})",
        },
    ]
}

/// 获取 Timer API 映射
pub fn timer_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "setTimeout",
            rust_impl: "// setTimeout(callback, delay_ms) — Phase 2+",
        },
        BuiltinMapping {
            js_api: "setInterval",
            rust_impl: "// setInterval(callback, interval_ms) — Phase 2+",
        },
        BuiltinMapping {
            js_api: "clearTimeout",
            rust_impl: "// clearTimeout(id) — Phase 2+",
        },
    ]
}

/// 获取 DOM API 映射（运行时直接调用 Rust DOM API）
pub fn dom_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "document.createElement",
            rust_impl: "doc.borrow().create_element({0})",
        },
        BuiltinMapping {
            js_api: "element.appendChild",
            rust_impl: "{0}.borrow_mut().append_child({1}.clone())",
        },
        BuiltinMapping {
            js_api: "element.textContent",
            rust_impl: "{0}.borrow().text_content()",
        },
        BuiltinMapping {
            js_api: "element.setAttribute",
            rust_impl: "{0}.borrow_mut().set_attribute({1}, {2})",
        },
        BuiltinMapping {
            js_api: "element.getAttribute",
            rust_impl: "{0}.borrow().get_attribute({1})",
        },
        BuiltinMapping {
            js_api: "element.addEventListener",
            rust_impl: "{0}.borrow_mut().add_event_listener({1}, Box::new(move |_: &dom::Event| {{ {2} }}))",
        },
        BuiltinMapping {
            js_api: "element.classList.add",
            rust_impl: "{0}.borrow_mut().class_list().add({1})",
        },
        BuiltinMapping {
            js_api: "element.querySelector",
            rust_impl: "// querySelector on element — Phase 2+",
        },
    ]
}

/// 获取 Math API 映射
pub fn math_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "Math.abs",
            rust_impl: "({0}).abs()",
        },
        BuiltinMapping {
            js_api: "Math.max",
            rust_impl: "({0}).max({1})",
        },
        BuiltinMapping {
            js_api: "Math.min",
            rust_impl: "({0}).min({1})",
        },
        BuiltinMapping {
            js_api: "Math.floor",
            rust_impl: "({0}).floor()",
        },
        BuiltinMapping {
            js_api: "Math.ceil",
            rust_impl: "({0}).ceil()",
        },
        BuiltinMapping {
            js_api: "Math.round",
            rust_impl: "({0}).round()",
        },
        BuiltinMapping {
            js_api: "Math.random",
            rust_impl: "rand::random::<f64>()",
        },
    ]
}

/// 获取所有内置 API 映射
pub fn all_builtins() -> Vec<BuiltinMapping> {
    let mut all = Vec::new();
    all.extend(console_mappings());
    all.extend(timer_mappings());
    all.extend(dom_mappings());
    all.extend(math_mappings());
    all
}

/// 查找 JS API 的 Rust 实现
pub fn lookup_builtin(js_api: &str) -> Option<&'static str> {
    for mapping in all_builtins() {
        if mapping.js_api == js_api {
            return Some(mapping.rust_impl);
        }
    }
    None
}

// ============================================================
//  Phase 2: TypedArray / ArrayBuffer 映射
// ============================================================

/// 获取 TypedArray 映射
pub fn typed_array_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "Uint8Array",
            rust_impl: "Vec<u8>",
        },
        BuiltinMapping {
            js_api: "Uint16Array",
            rust_impl: "Vec<u16>",
        },
        BuiltinMapping {
            js_api: "Uint32Array",
            rust_impl: "Vec<u32>",
        },
        BuiltinMapping {
            js_api: "Int8Array",
            rust_impl: "Vec<i8>",
        },
        BuiltinMapping {
            js_api: "Int16Array",
            rust_impl: "Vec<i16>",
        },
        BuiltinMapping {
            js_api: "Int32Array",
            rust_impl: "Vec<i32>",
        },
        BuiltinMapping {
            js_api: "Float32Array",
            rust_impl: "Vec<f32>",
        },
        BuiltinMapping {
            js_api: "Float64Array",
            rust_impl: "Vec<f64>",
        },
        BuiltinMapping {
            js_api: "ArrayBuffer",
            rust_impl: "Vec<u8>",
        },
        BuiltinMapping {
            js_api: "TextEncoder",
            rust_impl: "// TextEncoder → String::into_bytes() (Phase 2)",
        },
        BuiltinMapping {
            js_api: "TextDecoder",
            rust_impl: "// TextDecoder → String::from_utf8() (Phase 2)",
        },
    ]
}

// ============================================================
//  Phase 2: Promise / async 映射（编译时标记）
// ============================================================

/// 获取 Async/Promise 映射
pub fn async_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "Promise",
            rust_impl: "// Promise<T> → impl Future<Output = T> (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "async",
            rust_impl: "// async fn → async fn (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "await",
            rust_impl: "// await → .await (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "Promise.resolve",
            rust_impl: "// Promise.resolve(v) → std::future::ready(v) (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "Promise.all",
            rust_impl: "// Promise.all([...]) → futures::join!(...) (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "new Promise",
            rust_impl: "// new Promise(executor) — Phase 2+: Future adapter",
        },
    ]
}

/// 获取 Class 系统映射
pub fn class_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "class",
            rust_impl: "// class Foo → struct Foo + impl Foo (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "constructor",
            rust_impl: "// constructor() → fn new() (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "extends",
            rust_impl: "// extends → trait inheritance (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "super",
            rust_impl: "// super.method() → Trait::method(self) (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "this",
            rust_impl: "// this → &self / &mut self (Phase 2+)",
        },
        BuiltinMapping {
            js_api: "Proxy",
            rust_impl: "// Proxy → compile-time accessor rewrite (Phase 2+)",
        },
    ]
}

/// 获取所有 Phase 2 内置映射
pub fn phase2_builtins() -> Vec<BuiltinMapping> {
    let mut all = Vec::new();
    all.extend(typed_array_mappings());
    all.extend(async_mappings());
    all.extend(class_mappings());
    all
}

// ============================================================
//  Phase 3: Object 静态方法映射 (10 个)
// ============================================================

/// 获取 Object 静态方法映射
pub fn object_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "Object.create",
            rust_impl: "Object::create({0})",
        },
        BuiltinMapping {
            js_api: "Object.defineProperty",
            rust_impl: "Object::define_property(&mut {0}, {1}, {2})",
        },
        BuiltinMapping {
            js_api: "Object.defineProperties",
            rust_impl: "Object::define_properties(&mut {0}, {1})",
        },
        BuiltinMapping {
            js_api: "Object.freeze",
            rust_impl: "Object::freeze(&mut {0})",
        },
        BuiltinMapping {
            js_api: "Object.seal",
            rust_impl: "Object::seal(&mut {0})",
        },
        BuiltinMapping {
            js_api: "Object.is",
            rust_impl: "Object::is({0}, {1})",
        },
        BuiltinMapping {
            js_api: "Object.hasOwn",
            rust_impl: "Object::has_own(&{0}, {1})",
        },
        BuiltinMapping {
            js_api: "Object.fromEntries",
            rust_impl: "Object::from_entries({0})",
        },
        BuiltinMapping {
            js_api: "Object.getPrototypeOf",
            rust_impl: "Object::get_prototype_of(&{0})",
        },
        BuiltinMapping {
            js_api: "Object.setPrototypeOf",
            rust_impl: "Object::set_prototype_of(&mut {0}, {1})",
        },
    ]
}

// ============================================================
//  Phase 3: History API 映射
// ============================================================

/// 获取 History API 映射
pub fn history_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "history.pushState",
            rust_impl: "history.push_state({0}, {1}, {2})",
        },
        BuiltinMapping {
            js_api: "history.replaceState",
            rust_impl: "history.replace_state({0}, {1}, {2})",
        },
        BuiltinMapping {
            js_api: "history.back",
            rust_impl: "history.back()",
        },
        BuiltinMapping {
            js_api: "history.forward",
            rust_impl: "history.forward()",
        },
        BuiltinMapping {
            js_api: "history.go",
            rust_impl: "history.go({0})",
        },
        BuiltinMapping {
            js_api: "history.length",
            rust_impl: "history.length()",
        },
        BuiltinMapping {
            js_api: "history.state",
            rust_impl: "history.state()",
        },
    ]
}

// ============================================================
//  Phase 3: Location API 映射
// ============================================================

/// 获取 Location API 映射
pub fn location_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "location.href",
            rust_impl: "location.href()",
        },
        BuiltinMapping {
            js_api: "location.host",
            rust_impl: "location.host()",
        },
        BuiltinMapping {
            js_api: "location.pathname",
            rust_impl: "location.pathname()",
        },
        BuiltinMapping {
            js_api: "location.search",
            rust_impl: "location.search()",
        },
        BuiltinMapping {
            js_api: "location.hash",
            rust_impl: "location.hash()",
        },
        BuiltinMapping {
            js_api: "location.protocol",
            rust_impl: "location.protocol()",
        },
        BuiltinMapping {
            js_api: "location.assign",
            rust_impl: "location.assign({0})",
        },
        BuiltinMapping {
            js_api: "location.replace",
            rust_impl: "location.replace({0})",
        },
        BuiltinMapping {
            js_api: "location.reload",
            rust_impl: "location.reload()",
        },
    ]
}

// ============================================================
//  Phase 3: URL / URLSearchParams 映射
// ============================================================

/// 获取 URL/URLSearchParams API 映射
pub fn url_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "new URL",
            rust_impl: "URL::new({0}, {1})",
        },
        BuiltinMapping {
            js_api: "url.href",
            rust_impl: "{0}.href()",
        },
        BuiltinMapping {
            js_api: "url.host",
            rust_impl: "{0}.host()",
        },
        BuiltinMapping {
            js_api: "url.pathname",
            rust_impl: "{0}.pathname()",
        },
        BuiltinMapping {
            js_api: "url.search",
            rust_impl: "{0}.search()",
        },
        BuiltinMapping {
            js_api: "url.searchParams",
            rust_impl: "{0}.search_params()",
        },
        BuiltinMapping {
            js_api: "new URLSearchParams",
            rust_impl: "URLSearchParams::new({0})",
        },
        BuiltinMapping {
            js_api: "params.get",
            rust_impl: "{0}.get({1})",
        },
        BuiltinMapping {
            js_api: "params.set",
            rust_impl: "{0}.set({1}, {2})",
        },
        BuiltinMapping {
            js_api: "params.has",
            rust_impl: "{0}.has({1})",
        },
        BuiltinMapping {
            js_api: "params.delete",
            rust_impl: "{0}.delete({1})",
        },
        BuiltinMapping {
            js_api: "params.toString",
            rust_impl: "{0}.to_string()",
        },
        BuiltinMapping {
            js_api: "params.forEach",
            rust_impl: "{0}.for_each(|k, v| {{ {1} }})",
        },
    ]
}

// ============================================================
//  Phase 3: Date 编译映射
// ============================================================

/// 获取 Date API 映射
pub fn date_mappings() -> Vec<BuiltinMapping> {
    vec![
        BuiltinMapping {
            js_api: "new Date",
            rust_impl: "Date::now()",
        },
        BuiltinMapping {
            js_api: "new Date.timestamp",
            rust_impl: "Date::from_timestamp({0})",
        },
        BuiltinMapping {
            js_api: "new Date.string",
            rust_impl: "Date::parse({0})",
        },
        BuiltinMapping {
            js_api: "date.getTime",
            rust_impl: "{0}.timestamp_ms()",
        },
        BuiltinMapping {
            js_api: "date.getFullYear",
            rust_impl: "{0}.year()",
        },
        BuiltinMapping {
            js_api: "date.getMonth",
            rust_impl: "{0}.month()",
        },
        BuiltinMapping {
            js_api: "date.getDate",
            rust_impl: "{0}.day()",
        },
        BuiltinMapping {
            js_api: "date.getHours",
            rust_impl: "{0}.hours()",
        },
        BuiltinMapping {
            js_api: "date.getMinutes",
            rust_impl: "{0}.minutes()",
        },
        BuiltinMapping {
            js_api: "date.getSeconds",
            rust_impl: "{0}.seconds()",
        },
        BuiltinMapping {
            js_api: "date.getDay",
            rust_impl: "{0}.weekday()",
        },
        BuiltinMapping {
            js_api: "date.toISOString",
            rust_impl: "{0}.to_iso_string()",
        },
        BuiltinMapping {
            js_api: "date.toJSON",
            rust_impl: "{0}.to_iso_string()",
        },
    ]
}

// ============================================================
//  Phase 3: 编译时辅助 — lookup 入口
// ============================================================

/// 获取所有 Phase 3 内置映射
pub fn phase3_builtins() -> Vec<BuiltinMapping> {
    let mut all = Vec::new();
    all.extend(object_mappings());
    all.extend(history_mappings());
    all.extend(location_mappings());
    all.extend(url_mappings());
    all.extend(date_mappings());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_console_log() {
        let result = lookup_builtin("console.log");
        assert!(result.is_some());
        assert!(result.unwrap().contains("println!"));
    }

    #[test]
    fn test_lookup_console_error() {
        let result = lookup_builtin("console.error");
        assert!(result.is_some());
        assert!(result.unwrap().contains("eprintln!"));
    }

    #[test]
    fn test_lookup_unknown_api() {
        let result = lookup_builtin("non.existent");
        assert!(result.is_none());
    }

    #[test]
    fn test_lookup_create_element() {
        let result = lookup_builtin("document.createElement");
        assert!(result.is_some());
        assert!(result.unwrap().contains("create_element"));
    }

    #[test]
    fn test_lookup_add_event_listener() {
        let result = lookup_builtin("element.addEventListener");
        assert!(result.is_some());
        assert!(result.unwrap().contains("add_event_listener"));
    }

    #[test]
    fn test_lookup_math_abs() {
        let result = lookup_builtin("Math.abs");
        assert!(result.is_some());
        assert!(result.unwrap().contains("abs()"));
    }

    #[test]
    fn test_console_mappings_count() {
        assert_eq!(console_mappings().len(), 3);
    }

    #[test]
    fn test_dom_mappings_count() {
        assert_eq!(dom_mappings().len(), 8);
    }

    #[test]
    fn test_math_mappings_count() {
        assert_eq!(math_mappings().len(), 7);
    }

    #[test]
    fn test_all_builtins_non_empty() {
        let all = all_builtins();
        assert!(all.len() >= 20);
    }

    #[test]
    fn test_typed_array_mappings_count() {
        let mappings = typed_array_mappings();
        assert_eq!(mappings.len(), 11);
    }

    #[test]
    fn test_object_mappings_count() {
        assert_eq!(object_mappings().len(), 10);
    }

    #[test]
    fn test_history_mappings_count() {
        assert_eq!(history_mappings().len(), 7);
    }

    #[test]
    fn test_location_mappings_count() {
        assert_eq!(location_mappings().len(), 9);
    }

    #[test]
    fn test_date_mappings_count() {
        assert_eq!(date_mappings().len(), 13);
    }

    #[test]
    fn test_phase2_builtins_non_empty() {
        let all = phase2_builtins();
        assert!(all.len() >= 15);
    }

    #[test]
    fn test_phase3_builtins_non_empty() {
        let all = phase3_builtins();
        assert!(all.len() >= 40);
    }
}

