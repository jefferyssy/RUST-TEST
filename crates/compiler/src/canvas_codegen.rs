//! Canvas 2D API 编译映射 (Phase 3)
//!
//! 将 JS Canvas 2D API 调用编译为 Rust CanvasRenderingContext2D 调用。
//!
//! 共 18 个 API: fillRect/strokeRect/clearRect/beginPath/moveTo/lineTo/
//! rect/arc/fill/stroke/save/restore/translate/rotate/scale/
//! setTransform/fillText/drawImage/toDataURL

use crate::builtins::BuiltinMapping;

/// 获取 Canvas 2D API 映射
pub fn canvas_mappings() -> Vec<BuiltinMapping> {
    vec![
        // ===== 上下文获取 =====
        BuiltinMapping {
            js_api: "canvas.getContext",
            rust_impl: "{0}.get_context_2d()",
        },

        // ===== 样式属性 =====
        BuiltinMapping {
            js_api: "ctx.fillStyle",
            rust_impl: "{0}.set_fill_style({1})",
        },
        BuiltinMapping {
            js_api: "ctx.strokeStyle",
            rust_impl: "{0}.set_stroke_style({1})",
        },
        BuiltinMapping {
            js_api: "ctx.lineWidth",
            rust_impl: "{0}.set_line_width({1})",
        },
        BuiltinMapping {
            js_api: "ctx.globalAlpha",
            rust_impl: "{0}.set_global_alpha({1})",
        },
        BuiltinMapping {
            js_api: "ctx.font",
            rust_impl: "{0}.set_font({1})",
        },
        BuiltinMapping {
            js_api: "ctx.textAlign",
            rust_impl: "{0}.set_text_align({1})",
        },

        // ===== 矩形绘制 =====
        BuiltinMapping {
            js_api: "ctx.fillRect",
            rust_impl: "{0}.fill_rect({1}, {2}, {3}, {4})",
        },
        BuiltinMapping {
            js_api: "ctx.strokeRect",
            rust_impl: "{0}.stroke_rect({1}, {2}, {3}, {4})",
        },
        BuiltinMapping {
            js_api: "ctx.clearRect",
            rust_impl: "{0}.clear_rect({1}, {2}, {3}, {4})",
        },

        // ===== 路径 API =====
        BuiltinMapping {
            js_api: "ctx.beginPath",
            rust_impl: "{0}.begin_path()",
        },
        BuiltinMapping {
            js_api: "ctx.moveTo",
            rust_impl: "{0}.move_to({1}, {2})",
        },
        BuiltinMapping {
            js_api: "ctx.lineTo",
            rust_impl: "{0}.line_to({1}, {2})",
        },
        BuiltinMapping {
            js_api: "ctx.rect",
            rust_impl: "{0}.rect({1}, {2}, {3}, {4})",
        },
        BuiltinMapping {
            js_api: "ctx.arc",
            rust_impl: "{0}.arc({1}, {2}, {3}, {4}, {5})",
        },
        BuiltinMapping {
            js_api: "ctx.fill",
            rust_impl: "{0}.fill()",
        },
        BuiltinMapping {
            js_api: "ctx.stroke",
            rust_impl: "{0}.stroke()",
        },

        // ===== 状态管理 =====
        BuiltinMapping {
            js_api: "ctx.save",
            rust_impl: "{0}.save()",
        },
        BuiltinMapping {
            js_api: "ctx.restore",
            rust_impl: "{0}.restore()",
        },

        // ===== 变换 =====
        BuiltinMapping {
            js_api: "ctx.translate",
            rust_impl: "{0}.translate({1}, {2})",
        },
        BuiltinMapping {
            js_api: "ctx.rotate",
            rust_impl: "{0}.rotate({1})",
        },
        BuiltinMapping {
            js_api: "ctx.scale",
            rust_impl: "{0}.scale({1}, {2})",
        },
        BuiltinMapping {
            js_api: "ctx.setTransform",
            rust_impl: "{0}.set_transform({1}, {2}, {3}, {4}, {5}, {6})",
        },

        // ===== 文本 =====
        BuiltinMapping {
            js_api: "ctx.fillText",
            rust_impl: "{0}.fill_text({1}, {2}, {3})",
        },
        BuiltinMapping {
            js_api: "ctx.measureText",
            rust_impl: "{0}.measure_text({1})",
        },

        // ===== 图像 =====
        BuiltinMapping {
            js_api: "ctx.drawImage",
            rust_impl: "{0}.draw_image(&{1}, {2}, {3})",
        },

        // ===== 导出 =====
        BuiltinMapping {
            js_api: "canvas.toDataURL",
            rust_impl: "{0}.to_data_url()",
        },
    ]
}

/// 将 Canvas 2D 方法调用编译为 Rust 代码
///
/// JS: ctx.fillRect(10, 20, 100, 50)
/// →  Rust: ctx.fill_rect(10.0, 20.0, 100.0, 50.0);
pub fn compile_canvas_call(method: &str, args: &[String], ctx_var: &str) -> Option<String> {
    let mappings = canvas_mappings();
    for m in &mappings {
        if m.js_api == format!("ctx.{}", method) {
            let mut rust = m.rust_impl.to_string();
            rust = rust.replace("{0}", ctx_var);
            for (i, arg) in args.iter().enumerate() {
                rust = rust.replace(&format!("{{{}}}", i + 1), arg);
            }
            return Some(format!("{};", rust));
        }
    }
    None
}

/// 判断是否为 Canvas 2D API 调用
pub fn is_canvas_call(method: &str) -> bool {
    let canvas_methods: &[&str] = &[
        "fillRect", "strokeRect", "clearRect",
        "beginPath", "moveTo", "lineTo", "rect", "arc", "fill", "stroke",
        "save", "restore",
        "translate", "rotate", "scale", "setTransform",
        "fillText", "measureText",
        "drawImage",
    ];
    canvas_methods.contains(&method)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_mappings_have_all_apis() {
        let mappings = canvas_mappings();
        assert!(mappings.len() >= 27);
    }

    #[test]
    fn test_compile_fill_rect() {
        let result = compile_canvas_call("fillRect", &["10".into(), "20".into(), "100".into(), "50".into()], "ctx");
        assert!(result.is_some());
        assert!(result.unwrap().contains("fill_rect"));
    }

    #[test]
    fn test_is_canvas_call() {
        assert!(is_canvas_call("fillRect"));
        assert!(is_canvas_call("beginPath"));
        assert!(is_canvas_call("drawImage"));
        assert!(!is_canvas_call("addEventListener"));
    }
}
