# CLI 命令参考

## Demo 工作流

`examples/<name>/` 仅包含源文件（`index.html` + `style.css` + `app.js`），通过 `cli` 生成完整 Rust 项目后运行。

### 编译并运行单个 Demo

```bash
# 一步完成：生成 + 运行
cargo run -p cli -- run examples/counter -o generated/counter

# 分步操作：
# 1. 从源文件生成 Rust 项目
cargo run -p cli -- compile examples/counter -o generated/counter

# 2. 运行生成的项目
cargo run --manifest-path generated/counter/Cargo.toml
```

### cli 命令

```
cli compile <input-dir> [-o <output-dir>] [--name <name>] [--title <title>] [--width <w>] [--height <h>]
cli run <input-dir> [-o <output-dir>] [--name <name>] [--title <title>]
```

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `<input-dir>` | 源文件目录（含 index.html / style.css / app.js） | 必填 |
| `-o, --output` | 输出目录 | `target/generated/<name>` |
| `--name` | 项目名 | 输入目录名 |
| `--title` | 窗口标题 | `"Demo"` |
| `--width` | 窗口宽度 | `800` |
| `--height` | 窗口高度 | `600` |

### 所有 Demo

```bash
cargo run -p cli -- run examples/counter -o generated/counter
cargo run -p cli -- run examples/two-counters -o generated/two-counters
cargo run -p cli -- run examples/flex-nav -o generated/flex-nav
cargo run -p cli -- run examples/dashboard -o generated/dashboard
cargo run -p cli -- run examples/todo_app -o generated/todo_app
```

生成的项目位于 `generated/<name>/`，可通过 `git clean -fd generated/` 清理。

## 构建

```bash
cargo build --workspace             # 全量构建（8 crate）
cargo build -p cli            # 仅构建 CLI 工具
```

## 测试

```bash
cargo test                          # 全部 258 个测试
cargo test -p dom                   # 单 crate 测试
cargo test -p style
cargo test -p layout
cargo test -p paint
cargo test -p render_wgpu
cargo test -p compiler
cargo test -p net
cargo test -p storage
```

## 编译器 API

`compiler` 同时提供库 API 和 CLI 二进制入口。

### CLI 二进制

```bash
cargo run -p cli -- compile <input-dir> -o <output-dir>
cargo run -p cli -- run <input-dir> -o <output-dir>
```

### 库函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `compile` | `(html, css, js) -> String` | 生成完整 `main.rs` |
| `compile_with_options` | `(html, css, js, opts) -> String` | 同上，可配置标题/宽高 |
| `compile_body` | `(html, css, js) -> String` | 生成 `main()` 函数体 |
| `compile_body_with_options` | `(html, css, js, opts) -> String` | 同上，可配置窗口参数 |
| `compile_to_file` | `(html, css, js, output)` | 同 `compile`，写入文件 |
| `compile_body_to_file` | `(html, css, js, output)` | 同 `compile_body`，写入文件 |

### CompileOptions

```rust
compiler::CompileOptions {
    title: "Counter".into(),  // 窗口标题
    width: 800,               // 窗口宽度（逻辑像素）
    height: 600,              // 窗口高度（逻辑像素）
}
```

## 诊断输出

| 路径 | 说明 |
|------|------|
| `target/paint_debug.txt` | 渲染管线完整调试输出（DOM → 样式 → 布局 → DisplayList） |
| `target/wgpu_diag.txt` | wgpu 渲染帧诊断日志 |

## 源码目录速查

```
crates/
├── dom/          # DOM 树 + W3C API
├── style/        # CSS 引擎（选择器、级联、值解析）
├── layout/       # 布局引擎（block / flex / grid / table / float）
├── paint/  # DisplayList 构建（LayoutTree → PaintCommand）
├── render_wgpu/     # wgpu 渲染后端 + winit 窗口 + 事件循环
├── compiler/    # HTML+CSS+JS → Rust 编译器（库 + CLI）
├── net/          # 网络层（fetch stub）
└── storage/      # 存储层（localStorage / sessionStorage）

examples/
├── counter/       # 计数器（源码：index.html + style.css + app.js）
├── two-counters/  # 双计数器
├── flex-nav/      # Flex 导航栏
├── dashboard/     # 仪表盘
└── todo_app/      # 待办事项
```
