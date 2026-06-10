//! CLI 参数解析和默认值测试。

use super::*;
use clap::Parser;

// ── Args::from_build ──

#[test]
fn args_default_name_from_input_dir() {
    let a = BuildArgs {
        input_dir: PathBuf::from("examples/counter"),
        output_dir: None,
        name: None,
        title: "Demo".into(),
        width: 800,
        height: 600,
    };
    let args = Args::from_build(a);
    assert_eq!(args.name, "counter");
    assert_eq!(args.output_dir, PathBuf::from("target/generated/counter"));
}

#[test]
fn args_explicit_name_overrides_input_dir() {
    let a = BuildArgs {
        input_dir: PathBuf::from("examples/counter"),
        output_dir: None,
        name: Some("my-app".into()),
        title: "Demo".into(),
        width: 800,
        height: 600,
    };
    let args = Args::from_build(a);
    assert_eq!(args.name, "my-app");
    assert_eq!(args.output_dir, PathBuf::from("target/generated/my-app"));
}

#[test]
fn args_explicit_output_dir() {
    let a = BuildArgs {
        input_dir: PathBuf::from("examples/counter"),
        output_dir: Some(PathBuf::from("/tmp/out")),
        name: None,
        title: "Demo".into(),
        width: 800,
        height: 600,
    };
    let args = Args::from_build(a);
    assert_eq!(args.output_dir, PathBuf::from("/tmp/out"));
}

#[test]
fn args_preserves_title_width_height() {
    let a = BuildArgs {
        input_dir: PathBuf::from("examples/foo"),
        output_dir: None,
        name: None,
        title: "My App".into(),
        width: 1024,
        height: 768,
    };
    let args = Args::from_build(a);
    assert_eq!(args.title, "My App");
    assert_eq!(args.width, 1024);
    assert_eq!(args.height, 768);
}

// ── clap 参数解析 ──

#[test]
fn parse_compile_minimal() {
    let cli = Cli::try_parse_from(["cli", "compile", "examples/counter"]).unwrap();
    match cli.command {
        Command::Compile(a) => {
            assert_eq!(a.input_dir, PathBuf::from("examples/counter"));
            assert_eq!(a.title, "Demo");
            assert_eq!(a.width, 800);
            assert_eq!(a.height, 600);
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
            assert_eq!(a.title, "Hello");
            assert_eq!(a.width, 640);
            assert_eq!(a.height, 480);
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
            assert_eq!(a.title, "Demo");
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
            assert_eq!(a.width, 1280);
            assert_eq!(a.height, 720);
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
