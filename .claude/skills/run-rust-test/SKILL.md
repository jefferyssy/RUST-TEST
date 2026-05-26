---
name: run-rust-test
description: Build, test, and run this Rust browser engine's compiled demos. Use when asked to run the app, start a demo, test the project, or take a screenshot.
---

# Rust 浏览器引擎 — 运行与测试

一个基于 Rust 的自研浏览器渲染管线项目，通过 `toolchain` CLI 将 HTML+CSS+JS 编译为原生桌面应用（winit + wgpu）。

## 驱动脚本

所有构建/测试/运行操作通过 `.claude/skills/run-rust-test/smoke.sh` 执行。

## 前置条件

无额外系统依赖 —— Rust 工具链 (cargo, rustc) 即可。GUI 运行时需要 GPU 或兼容的图形后端。

## 构建

```bash
# 仅构建 toolchain CLI
bash .claude/skills/run-rust-test/smoke.sh build
```

## 运行 (agent 路径)

### 冒烟测试 — 最快的端到端验证

```bash
bash .claude/skills/run-rust-test/smoke.sh smoke
```

流程：toolchain 编译 todo_app → cargo build → 启动运行 → 验证 wgpu 初始化 → 8 秒后自动终止。

### 完整测试套件

```bash
bash .claude/skills/run-rust-test/smoke.sh test
```

等效于 `cargo test --workspace`，覆盖 DOM、Style、Layout、RenderTree、Renderer、Toolchain 等全部 268 个测试。

### 运行指定 demo

```bash
# 列出可用 demo
bash .claude/skills/run-rust-test/smoke.sh demo

# 运行 todo_app
bash .claude/skills/run-rust-test/smoke.sh demo todo_app

# 运行 counter
bash .claude/skills/run-rust-test/smoke.sh demo counter
```

可用 demo: `todo_app`, `counter`, `two-counters`, `flex-nav`, `dashboard`

### 全部检查

```bash
bash .claude/skills/run-rust-test/smoke.sh all
```

运行完整测试套件 + 所有可用 demo 的构建和运行。

## 直接调用（库/内部函数）

对于仅涉及单个 crate 内部逻辑的改动，直接运行该 crate 的测试即可：

```bash
cargo test -p dom         # DOM 核心
cargo test -p style       # CSS 引擎
cargo test -p layout      # 布局引擎
cargo test -p render_tree # DisplayList 构建
cargo test -p renderer    # wgpu 渲染 + 窗口
cargo test -p toolchain   # 编译器
```

## 运行 (人类路径)

在有显示器的桌面环境上直接运行 demo：

```bash
cargo run -p toolchain -- run examples/todo_app
```

会弹出原生窗口，交互完成后关闭窗口即退出。

## 环境变量

| 变量          | 默认值 | 说明                 |
| ------------- | ------ | -------------------- |
| `TIMEOUT_SEC` | `8`    | GUI 应用运行等待秒数 |

## 运行日志验证

成功运行的标志性日志行：

```
[diag] wgpu init: physical=600x750, logical=400x500, scale=1.5, format=Bgra8UnormSrgb
```

`wgpu init` 和 `resize` 日志表示渲染后端已正确初始化并开始绘制帧。

## Gotchas

- **toolchain 不支持 `--quiet`**：传递给 toolchain 的参数只能是其支持的（compile/run、-o、--name、--title、--width、--height）。
- **GUI 应用不会自动退出**：必须用超时或发送 SIGTERM 终止。`smoke.sh` 使用 `sleep + kill` 模式处理。
- **生成的代码在 `target/generated/<name>/`**：toolchain 的 `compile` 命令生成完整 Cargo 项目（含 Cargo.toml + src/main.rs），构建产物独立于 workspace。
- **Windows 上 taskkill 路径**：`smoke.sh` 中包含 Windows 兼容的进程终止逻辑（`taskkill //F //PID`）。
- **wgpu 在 headless 环境可能失败**：如果没有 GPU，wgpu 可能无法创建 adapter。在 CI 上建议仅运行 `cargo test`。

## Troubleshooting

| 症状                     | 修复                                                                       |
| ------------------------ | -------------------------------------------------------------------------- |
| `toolchain compile` 失败 | 确认 `examples/<name>/` 目录存在且包含 `index.html`, `style.css`, `app.js` |
| demo build 失败          | 检查 workspace crates 编译: `cargo check --workspace`                      |
| wgpu init 日志未出现     | 确认有 GPU/wgpu 后端支持；在 headless 环境可能无输出                       |

## 强制行为准则

1. **语言约束**：所有内部思考、推理过程、分析、输出内容，**一律使用简体中文**，禁止出现英文思考步骤。

# 执行规则

1. 需求、细节、逻辑存在模糊/缺失时，立即停止操作，不要脑补、试错、反复改代码，逐条列出疑问确认后再继续。
2. 大幅减少额外推理、分析和冗余解释，只保留必要内容。
3. 仅按明确指令执行，不主动拓展功能、不额外推演方案。
4. 输出以代码、结论、简答为主，文字尽量简练。
