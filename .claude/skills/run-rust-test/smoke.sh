#!/usr/bin/env bash
# smoke.sh — Rust 浏览器引擎 Demo 构建与运行驱动
#
# 用法:
#   .claude/skills/run-rust-test/smoke.sh build          # 仅构建 toolchain
#   .claude/skills/run-rust-test/smoke.sh test           # 运行完整测试套件
#   .claude/skills/run-rust-test/smoke.sh demo <name>    # 编译并运行指定 demo
#   .claude/skills/run-rust-test/smoke.sh smoke          # 快速冒烟：toolchain + todo_app
#   .claude/skills/run-rust-test/smoke.sh all            # 全部：test + smoke + 所有 demo
#
# 环境变量:
#   TIMEOUT_SEC=10    运行 GUI 应用的等待秒数（默认 8）

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TIMEOUT_SEC="${TIMEOUT_SEC:-8}"

# 可用 demo 列表
DEMOS=("todo_app" "counter" "two-counters" "flex-nav" "dashboard")

red()  { echo -e "\033[31m$1\033[0m"; }
green(){ echo -e "\033[32m$1\033[0m"; }
cyan() { echo -e "\033[36m$1\033[0m"; }

# ── 构建 toolchain ──
build_toolchain() {
    cyan ">>> 构建 toolchain CLI..."
    cd "$PROJECT_ROOT"
    cargo build -p toolchain --quiet 2>&1 || {
        red "toolchain 构建失败"
        return 1
    }
    green "   toolchain 构建成功"
}

# ── 运行测试套件 ──
run_tests() {
    cyan ">>> 运行完整测试套件..."
    cd "$PROJECT_ROOT"
    cargo test --workspace 2>&1 | tail -5
    green "   测试通过"
}

# ── 编译并运行单个 demo ──
# 参数: demo_name
run_demo() {
    local name="$1"
    local input_dir="$PROJECT_ROOT/examples/$name"
    local output_dir="$PROJECT_ROOT/target/generated/$name"

    if [ ! -f "$input_dir/index.html" ]; then
        red "Demo '$name' 不存在: $input_dir"
        return 1
    fi

    cyan ">>> 编译 demo: $name"

    # Step 1: toolchain 生成 Rust 项目
    cd "$PROJECT_ROOT"
    cargo run -p toolchain -- compile "$input_dir" \
        -o "$output_dir" \
        --name "$name" \
        --title "Demo: $name" \
        --width 400 --height 500 2>&1 || { red "toolchain compile 失败"; return 1; }

    # Step 2: cargo build
    cargo build --manifest-path "$output_dir/Cargo.toml" --quiet 2>&1 || {
        red "demo build 失败"
        return 1
    }

    green "   $name 构建成功 → $output_dir"

    # Step 3: 运行并验证 wgpu 初始化
    cyan ">>> 运行 demo: $name (${TIMEOUT_SEC}s 后自动终止)"
    local logfile
    logfile="$(mktemp)"
    cd "$PROJECT_ROOT"

    # 后台运行 demo，捕获日志
    cargo run --manifest-path "$output_dir/Cargo.toml" --quiet > "$logfile" 2>&1 &
    local pid=$!

    sleep "$TIMEOUT_SEC"

    # 终止进程
    kill "$pid" 2>/dev/null || taskkill //F //PID "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true

    # 验证日志中包含 wgpu 初始化
    if grep -q "wgpu init" "$logfile" || grep -q "resize" "$logfile"; then
        green "   $name 运行正常 (wgpu 已初始化)"
        cat "$logfile"
    else
        red "   $name 可能未正确初始化，日志:"
        cat "$logfile"
        rm -f "$logfile"
        return 1
    fi

    rm -f "$logfile"
}

# ── 冒烟测试：toolchain + todo_app ──
smoke() {
    build_toolchain
    run_demo "todo_app"
    green ">>> 冒烟测试完成"
}

# ── 全部：test + 所有 demo ──
all() {
    run_tests
    build_toolchain
    for demo in "${DEMOS[@]}"; do
        if [ -d "$PROJECT_ROOT/examples/$demo" ]; then
            run_demo "$demo" || echo "   (demo '$demo' 失败，继续下一个)"
        fi
    done
    green ">>> 全部检查完成"
}

# ── main ──
case "${1:-}" in
    build)
        build_toolchain
        ;;
    test)
        run_tests
        ;;
    demo)
        if [ -z "${2:-}" ]; then
            echo "用法: smoke.sh demo <name>"
            echo "可用 demo: ${DEMOS[*]}"
            exit 1
        fi
        build_toolchain
        run_demo "$2"
        ;;
    smoke)
        smoke
        ;;
    all)
        all
        ;;
    *)
        echo "用法: smoke.sh {build|test|demo <name>|smoke|all}"
        echo ""
        echo "  build        构建 toolchain CLI"
        echo "  test         运行 cargo test --workspace"
        echo "  demo <name>  编译并运行指定 demo (todo_app, counter, etc.)"
        echo "  smoke        快速冒烟: toolchain + todo_app"
        echo "  all          全部: test + 所有 demo"
        echo ""
        echo "可用 demo: ${DEMOS[*]}"
        exit 1
        ;;
esac
