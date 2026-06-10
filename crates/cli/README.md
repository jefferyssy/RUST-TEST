# cli — 浏览器引擎命令行工具

将 HTML+CSS+JS 前端项目编译为 Rust 原生应用，并通过 `cargo run` 一键运行。

## 功能

- **`compile`** — 读取输入目录中的 HTML/CSS/JS 源文件，生成完整的 Cargo 项目（含 `main.rs` + `Cargo.toml`）
- **`run`** — 先执行 compile，然后自动 `cargo run` 运行生成的二进制

## 安装

```bash
cargo install --path crates/cli
```

## 用法

```bash
# 仅生成 Rust 项目代码，不运行
cli compile <input-dir> [选项]

# 生成代码后自动编译并运行
cli run <input-dir> [选项]
```

## 参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `<input-dir>` | 输入目录路径，应包含 `index.html`、`style.css`、`app.js` | **必填** |
| `-o, --output-dir` | 输出目录路径 | `target/generated/<项目名>` |
| `--name` | 项目名称，用作 Cargo package name | 取输入目录文件夹名 |
| `--title` | 窗口标题 | `"Demo"` |
| `--width` | 窗口宽度（像素） | `800` |
| `--height` | 窗口高度（像素） | `600` |

## 示例

```bash
# 编译 examples/counter 并运行
cargo run -p cli -- run examples/counter

# 仅编译，输出到自定义目录
cargo run -p cli -- compile examples/counter -o my_app --name "my-app" --title "My App"

# 指定窗口尺寸
cargo run -p cli -- run examples/flex-nav --width 1024 --height 768
```

## 数据流

```
命令行参数 → clap 解析 → BuildArgs → Args（默认值解析）
                                    ↓
                       compiler::compile_project_to_dir
                                    ↓
                        Cargo.toml + src/main.rs
                                    ↓
                              cargo run
```

## 依赖

- [clap](https://crates.io/crates/clap) — 命令行参数解析
- [compiler](../compiler) — 编译逻辑（HTML/CSS/JS → Rust 代码生成）
