//! CLI 参数解析测试。

use super::*;
use clap::Parser;

// ── clap 参数解析 ──

#[test]
fn parse_compile_minimal() {
    let cli = Cli::try_parse_from(["cli", "compile", "examples/counter"]).unwrap();
    match cli.command {
        Command::Compile(a) => {
            assert_eq!(a.input_dir, PathBuf::from("examples/counter"));
            // 未传参数时为 None，由 compiler 兜底
            assert_eq!(a.title, None);
            assert_eq!(a.width, None);
            assert_eq!(a.height, None);
        }
        _ => panic!("expected Compile"),
    }
}

#[test]
fn parse_compile_all_flags() {
    let cli = Cli::try_parse_from([
        "cli", "compile", "my-app",
        "-o", "build/out",
        "--name", "app",
        "--title", "Hello",
        "--width", "640",
        "--height", "480",
    ])
    .unwrap();
    match cli.command {
        Command::Compile(a) => {
            assert_eq!(a.input_dir, PathBuf::from("my-app"));
            assert_eq!(a.output_dir, Some(PathBuf::from("build/out")));
            assert_eq!(a.name, Some("app".into()));
            assert_eq!(a.title, Some("Hello".into()));
            assert_eq!(a.width, Some(640));
            assert_eq!(a.height, Some(480));
        }
        _ => panic!("expected Compile"),
    }
}

#[test]
fn parse_run_minimal() {
    let cli = Cli::try_parse_from(["cli", "run", "examples/todo_app"]).unwrap();
    match cli.command {
        Command::Run(a) => {
            assert_eq!(a.input_dir, PathBuf::from("examples/todo_app"));
            assert_eq!(a.title, None);
        }
        _ => panic!("expected Run"),
    }
}

#[test]
fn parse_run_with_flags() {
    let cli = Cli::try_parse_from([
        "cli", "run", "examples/flex-nav",
        "-o", "out/flex",
        "--name", "flex",
        "--title", "Flex Demo",
        "--width", "1280",
        "--height", "720",
    ])
    .unwrap();
    match cli.command {
        Command::Run(a) => {
            assert_eq!(a.name, Some("flex".into()));
            assert_eq!(a.width, Some(1280));
            assert_eq!(a.height, Some(720));
        }
        _ => panic!("expected Run"),
    }
}

#[test]
fn parse_missing_input_dir() {
    let err = Cli::try_parse_from(["cli", "compile"]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("INPUT_DIR") || msg.contains("input"), "unexpected error: {msg}");
}

#[test]
fn parse_unknown_subcommand() {
    let err = Cli::try_parse_from(["cli", "build", "foo"]).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unrecognized") || msg.contains("build") || msg.contains("subcommand"),
        "unexpected error: {msg}"
    );
}

#[test]
fn parse_invalid_width() {
    let err =
        Cli::try_parse_from(["cli", "compile", "foo", "--width", "abc"]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("width") || msg.contains("invalid"), "unexpected error: {msg}");
}

#[test]
fn parse_help_flag() {
    let err = Cli::try_parse_from(["cli", "--help"]).unwrap_err();
    assert!(err.to_string().contains("前端项目编译"));
}
