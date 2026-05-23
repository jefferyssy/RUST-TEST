//! toolchain CLI — HTML+CSS+JS → Rust project generator
//!
//! Usage:
//!   toolchain compile <input-dir> [-o <output-dir>] [--name <name>] [--title <title>] [--width <w>] [--height <h>]
//!   toolchain run <input-dir> [-o <output-dir>] [--name <name>] [--title <title>]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

struct Args {
    command: CommandType,
    input_dir: PathBuf,
    output_dir: PathBuf,
    name: String,
    title: String,
    width: u32,
    height: u32,
}

enum CommandType {
    Compile,
    Run,
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!(
                "usage: toolchain compile <input-dir> [-o <output-dir>] [--name <name>] [--title <title>] [--width <w>] [--height <h>]"
            );
            eprintln!("       toolchain run <input-dir> [-o <output-dir>] [--name <name>] [--title <title>]");
            process::exit(1);
        }
    };

    match args.command {
        CommandType::Compile => cmd_compile(&args),
        CommandType::Run => cmd_run(&args),
    }
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = env::args().collect();
    if raw.len() < 2 {
        return Err("missing command".into());
    }

    let command = match raw[1].as_str() {
        "compile" => CommandType::Compile,
        "run" => CommandType::Run,
        other => return Err(format!("unknown command: {other}")),
    };

    let mut input_dir: Option<PathBuf> = None;
    let mut output_dir: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut title = "Demo".to_string();
    let mut width = 800u32;
    let mut height = 600u32;

    let mut i = 2;
    while i < raw.len() {
        match raw[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                if i >= raw.len() {
                    return Err("missing value for -o".into());
                }
                output_dir = Some(PathBuf::from(&raw[i]));
            }
            "--name" => {
                i += 1;
                if i >= raw.len() {
                    return Err("missing value for --name".into());
                }
                name = Some(raw[i].clone());
            }
            "--title" => {
                i += 1;
                if i >= raw.len() {
                    return Err("missing value for --title".into());
                }
                title = raw[i].clone();
            }
            "--width" => {
                i += 1;
                if i >= raw.len() {
                    return Err("missing value for --width".into());
                }
                width = raw[i].parse().map_err(|_| "invalid --width")?;
            }
            "--height" => {
                i += 1;
                if i >= raw.len() {
                    return Err("missing value for --height".into());
                }
                height = raw[i].parse().map_err(|_| "invalid --height")?;
            }
            arg if !arg.starts_with('-') && input_dir.is_none() => {
                input_dir = Some(PathBuf::from(arg));
            }
            other => return Err(format!("unexpected argument: {other}")),
        }
        i += 1;
    }

    let input_dir = input_dir.ok_or("missing input directory")?;
    let name = name.unwrap_or_else(|| {
        input_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "app".into())
    });
    let output_dir = output_dir.unwrap_or_else(|| PathBuf::from("target/generated").join(&name));

    Ok(Args {
        command,
        input_dir,
        output_dir,
        name,
        title,
        width,
        height,
    })
}

fn cmd_compile(args: &Args) {
    let workspace_root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    eprintln!(
        "Compiling {} from {:?} -> {:?}",
        args.name, args.input_dir, args.output_dir
    );

    // Verify input files
    let html_path = args.input_dir.join("index.html");
    let css_path = args.input_dir.join("style.css");
    let js_path = args.input_dir.join("app.js");
    for (path, label) in &[(&html_path, "HTML"), (&css_path, "CSS"), (&js_path, "JS")] {
        if !path.exists() {
            eprintln!("warning: {label} file not found: {}", path.display());
        }
    }

    // Generate Rust source
    let opts = toolchain::CompileOptions {
        title: args.title.clone(),
        width: args.width,
        height: args.height,
    };
    let rust_code = toolchain::compile_with_options(
        &html_path.to_string_lossy(),
        &css_path.to_string_lossy(),
        &js_path.to_string_lossy(),
        &opts,
    );

    // Create output directories
    let src_dir = args.output_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap_or_else(|e| {
        eprintln!("error: cannot create {}: {e}", src_dir.display());
        process::exit(1);
    });

    // Write src/main.rs
    fs::write(src_dir.join("main.rs"), &rust_code).unwrap_or_else(|e| {
        eprintln!("error: cannot write main.rs: {e}");
        process::exit(1);
    });
    eprintln!("  wrote {}", src_dir.join("main.rs").display());

    // Compute relative paths for Cargo.toml dependencies
    let renderer_rel = rel_path(&args.output_dir, &workspace_root.join("crates").join("renderer"));
    let dom_rel = rel_path(&args.output_dir, &workspace_root.join("crates").join("dom"));

    // Generate Cargo.toml
    let cargo_toml = format!(
        "[workspace]\n\
         \n\
         [package]\n\
         name = \"{}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         \n\
         [dependencies]\n\
         renderer = {{ path = \"{}\" }}\n\
         dom = {{ path = \"{}\" }}\n",
        args.name, renderer_rel, dom_rel,
    );
    fs::write(args.output_dir.join("Cargo.toml"), &cargo_toml).unwrap_or_else(|e| {
        eprintln!("error: cannot write Cargo.toml: {e}");
        process::exit(1);
    });
    eprintln!("  wrote {}", args.output_dir.join("Cargo.toml").display());

    eprintln!(
        "Done. Run with: cargo run --manifest-path {}/Cargo.toml",
        args.output_dir.display()
    );
}

fn cmd_run(args: &Args) {
    cmd_compile(args);
    eprintln!("Running...");
    let status = Command::new("cargo")
        .args(["run", "--manifest-path"])
        .arg(args.output_dir.join("Cargo.toml"))
        .status()
        .unwrap_or_else(|e| {
            eprintln!("error: cargo run failed: {e}");
            process::exit(1);
        });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

/// Compute a relative path from `from` (directory) to `to` (file or directory).
/// Uses canonicalized paths when possible, falls back to lexical path computation.
fn rel_path(from: &Path, to: &Path) -> String {
    // Canonicalize — may fail if paths don't exist (e.g. output dir not yet created for `from`).
    // We handle this by falling back to the raw path.
    let from_abs = from.canonicalize().unwrap_or_else(|_| {
        // If the directory doesn't exist, try canonicalizing the parent and appending the leaf
        if let Some(parent) = from.parent() {
            if let Ok(parent_abs) = parent.canonicalize() {
                return parent_abs.join(from.file_name().unwrap_or_default());
            }
        }
        from.to_path_buf()
    });
    let to_abs = to.canonicalize().unwrap_or_else(|_| to.to_path_buf());

    let from_parts: Vec<_> = from_abs.components().collect();
    let to_parts: Vec<_> = to_abs.components().collect();

    let common = from_parts
        .iter()
        .zip(to_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let up_count = from_parts.len() - common;
    let mut result = String::new();
    for _ in 0..up_count {
        result.push_str("../");
    }
    for comp in &to_parts[common..] {
        result.push_str(&comp.as_os_str().to_string_lossy());
        result.push('/');
    }
    if result.ends_with('/') {
        result.pop();
    }
    // Normalize to forward slashes
    result.replace('\\', "/")
}
