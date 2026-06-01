//! JS 语义分析器
//!
//! Phase 1: 分析 JS 代码中的变量声明、函数引用、API 调用。
//! Phase 2+: 基于 swc AST 的完整语义分析。

// use std::collections::HashMap; // Phase 2+

/// 语义分析结果
#[derive(Debug)]
pub struct AnalysisResult {
    pub variables: Vec<VariableInfo>,
    pub function_calls: Vec<FunctionCall>,
    pub dom_operations: Vec<DomOperation>,
}

/// 变量信息
#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub kind: VariableKind,
    pub line: usize,
}

/// 变量类型
#[derive(Debug, Clone, PartialEq)]
pub enum VariableKind {
    Let,
    Const,
    Var,
    Function,
}

/// 函数调用信息
#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Vec<String>,
    pub line: usize,
}

/// DOM 操作类型
#[derive(Debug, Clone)]
pub enum DomOperation {
    QuerySelector(String),
    GetElementById(String),
    CreateElement(String),
    AppendChild,
    SetTextContent(String),
    SetAttribute(String, String),
    AddEventListener(String, String),
    SetStyle(String, String),
}

/// JS 语义分析器
pub struct Analyzer;

impl Analyzer {
    /// 分析 JS 源代码
    pub fn analyze(js_source: &str) -> AnalysisResult {
        let mut variables = Vec::new();
        let mut function_calls = Vec::new();
        let mut dom_operations = Vec::new();

        for (line_num, line) in js_source.lines().enumerate() {
            let line = line.trim();

            // 检测变量声明
            if line.starts_with("let ") || line.starts_with("const ") || line.starts_with("var ") {
                let (kind, rest) = if let Some(r) = line.strip_prefix("let ") {
                    (VariableKind::Let, r)
                } else if let Some(r) = line.strip_prefix("const ") {
                    (VariableKind::Const, r)
                } else if let Some(r) = line.strip_prefix("var ") {
                    (VariableKind::Var, r)
                } else {
                    continue;
                };

                if let Some(name) = rest.split(&['=', ';', ' '][..]).next() {
                    let name = name.trim();
                    if !name.is_empty() && is_valid_identifier(name) {
                        variables.push(VariableInfo {
                            name: name.to_string(),
                            kind,
                            line: line_num + 1,
                        });
                    }
                }
            }

            // Phase 1: 检测 DOM API 调用
            if line.contains("document.querySelector") {
                if let Some(sel) = extract_string_arg(line, "document.querySelector") {
                    dom_operations.push(DomOperation::QuerySelector(sel));
                }
            }
            if line.contains("document.getElementById") {
                if let Some(id) = extract_string_arg(line, "document.getElementById") {
                    dom_operations.push(DomOperation::GetElementById(id));
                }
            }
            if line.contains("document.createElement") {
                if let Some(tag) = extract_string_arg(line, "document.createElement") {
                    dom_operations.push(DomOperation::CreateElement(tag));
                }
            }
            if line.contains(".addEventListener") {
                let args = extract_two_string_args(line, ".addEventListener");
                if let Some((event, _handler)) = args {
                    dom_operations.push(DomOperation::AddEventListener(event, String::new()));
                }
            }
            if line.contains(".textContent") && line.contains('=') {
                dom_operations.push(DomOperation::SetTextContent(String::new()));
            }
            if line.contains(".setAttribute") {
                let args = extract_two_string_args(line, ".setAttribute");
                if let Some((name, value)) = args {
                    dom_operations.push(DomOperation::SetAttribute(name, value));
                }
            }
            if line.contains(".style.") {
                dom_operations.push(DomOperation::SetStyle(String::new(), String::new()));
            }

            // Phase 1: 检测函数调用
            if let Some((name, args)) = extract_function_call(line) {
                function_calls.push(FunctionCall {
                    name,
                    arguments: args,
                    line: line_num + 1,
                });
            }
        }

        AnalysisResult {
            variables,
            function_calls,
            dom_operations,
        }
    }
}

fn is_valid_identifier(s: &str) -> bool {
    s.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_' || c == '$').unwrap_or(false)
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn extract_string_arg(line: &str, func: &str) -> Option<String> {
    let after = line.split(func).nth(1)?;
    let after_paren = after.trim_start().strip_prefix('(')?;
    let inner = after_paren.trim();
    if inner.starts_with('\'') || inner.starts_with('"') {
        let quote = inner.chars().next().unwrap();
        let end = inner[1..].find(quote)?;
        Some(inner[1..end + 1].to_string())
    } else {
        None
    }
}

fn extract_two_string_args(line: &str, func: &str) -> Option<(String, String)> {
    let after = line.split(func).nth(1)?;
    let inner = after.trim_start().strip_prefix('(')?;
    let inner = inner.trim();
    let first = if inner.starts_with('\'') || inner.starts_with('"') {
        let quote = inner.chars().next().unwrap();
        let end = inner[1..].find(quote)?;
        inner[1..end + 1].to_string()
    } else {
        return None;
    };
    let after_comma = inner[1 + first.len() + 1..].trim();
    let after_comma = after_comma.strip_prefix(',')?.trim();
    let second = if after_comma.starts_with('\'') || after_comma.starts_with('"') {
        let quote = after_comma.chars().next().unwrap();
        let end = after_comma[1..].find(quote)?;
        after_comma[1..end + 1].to_string()
    } else {
        return None;
    };
    Some((first, second))
}

fn extract_function_call(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim();
    if let Some(paren) = line.find('(') {
        let name = line[..paren].trim().to_string();
        if name.is_empty() || name.contains(' ') || name.contains('.') {
            return None;
        }
        let after_paren = &line[paren + 1..];
        let end = after_paren.rfind(')')?;
        let args_str = &after_paren[..end];
        let args: Vec<String> = args_str.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Some((name, args))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_detects_let_variable() {
        let result = Analyzer::analyze("let count = 0;\n");
        assert_eq!(result.variables.len(), 1);
        assert_eq!(result.variables[0].name, "count");
        assert_eq!(result.variables[0].kind, VariableKind::Let);
    }

    #[test]
    fn test_analyze_detects_const_variable() {
        let result = Analyzer::analyze("const MAX = 100;\n");
        assert_eq!(result.variables.len(), 1);
        assert_eq!(result.variables[0].kind, VariableKind::Const);
    }

    #[test]
    fn test_analyze_detects_var_variable() {
        let result = Analyzer::analyze("var x = 1;\n");
        assert_eq!(result.variables.len(), 1);
        assert_eq!(result.variables[0].kind, VariableKind::Var);
    }

    #[test]
    fn test_analyze_detects_query_selector() {
        let result = Analyzer::analyze("let el = document.querySelector('.btn');\n");
        assert_eq!(result.dom_operations.len(), 1);
        match &result.dom_operations[0] {
            DomOperation::QuerySelector(sel) => assert_eq!(sel, ".btn"),
            other => panic!("Expected QuerySelector, got {:?}", other),
        }
    }

    #[test]
    fn test_analyze_detects_get_element_by_id() {
        let result = Analyzer::analyze("let x = document.getElementById('my-id');\n");
        assert_eq!(result.dom_operations.len(), 1);
        match &result.dom_operations[0] {
            DomOperation::GetElementById(id) => assert_eq!(id, "my-id"),
            other => panic!("Expected GetElementById, got {:?}", other),
        }
    }

    #[test]
    fn test_analyze_detects_create_element() {
        let result = Analyzer::analyze("let d = document.createElement('div');\n");
        assert_eq!(result.dom_operations.len(), 1);
        match &result.dom_operations[0] {
            DomOperation::CreateElement(tag) => assert_eq!(tag, "div"),
            other => panic!("Expected CreateElement, got {:?}", other),
        }
    }

    #[test]
    fn test_analyze_detects_add_event_listener() {
        let result = Analyzer::analyze("btn.addEventListener('click', 'handler');\n");
        assert_eq!(result.dom_operations.len(), 1);
        match &result.dom_operations[0] {
            DomOperation::AddEventListener(event, _) => assert_eq!(event, "click"),
            other => panic!("Expected AddEventListener, got {:?}", other),
        }
    }

    #[test]
    fn test_analyze_detects_text_content_set() {
        let result = Analyzer::analyze("el.textContent = 'hello';\n");
        assert_eq!(result.dom_operations.len(), 1);
        assert!(matches!(result.dom_operations[0], DomOperation::SetTextContent(_)));
    }

    #[test]
    fn test_analyze_detects_set_attribute() {
        let result = Analyzer::analyze("el.setAttribute('class', 'active');\n");
        assert_eq!(result.dom_operations.len(), 1);
        match &result.dom_operations[0] {
            DomOperation::SetAttribute(name, value) => {
                assert_eq!(name, "class");
                assert_eq!(value, "active");
            }
            other => panic!("Expected SetAttribute, got {:?}", other),
        }
    }

    #[test]
    fn test_analyze_detects_style_set() {
        let result = Analyzer::analyze("el.style.color = 'red';\n");
        assert_eq!(result.dom_operations.len(), 1);
        assert!(matches!(result.dom_operations[0], DomOperation::SetStyle(_, _)));
    }

    #[test]
    fn test_analyze_detects_function_calls() {
        let result = Analyzer::analyze("update(42, 'hello');\n");
        assert_eq!(result.function_calls.len(), 1);
        assert_eq!(result.function_calls[0].name, "update");
        assert_eq!(result.function_calls[0].arguments, vec!["42", "'hello'"]);
    }

    #[test]
    fn test_extract_string_arg_single_quotes() {
        let line = "document.getElementById('my-id');";
        let result = extract_string_arg(line, "document.getElementById");
        assert_eq!(result, Some("my-id".to_string()));
    }

    #[test]
    fn test_extract_string_arg_double_quotes() {
        let line = "document.querySelector(\".my-class\");";
        let result = extract_string_arg(line, "document.querySelector");
        assert_eq!(result, Some(".my-class".to_string()));
    }

    #[test]
    fn test_extract_two_string_args() {
        let line = "el.setAttribute('id', 'main');";
        let (first, second) = extract_two_string_args(line, ".setAttribute").unwrap();
        assert_eq!(first, "id");
        assert_eq!(second, "main");
    }

    #[test]
    fn test_is_valid_identifier_true() {
        assert!(is_valid_identifier("count"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("$jquery"));
        assert!(is_valid_identifier("x1"));
    }

    #[test]
    fn test_is_valid_identifier_false() {
        assert!(!is_valid_identifier("1count"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("has space"));
    }

    #[test]
    fn test_analyze_empty_js() {
        let result = Analyzer::analyze("");
        assert!(result.variables.is_empty());
        assert!(result.dom_operations.is_empty());
        assert!(result.function_calls.is_empty());
    }
}
