---
description: Rust 单元测试规范
globs: ["crates/**/*.rs"]
---

## Rust 单元测试规范

当用户要求添加单元测试时，**必须**遵守以下规则：

### 文件位置

- 测试内容**不能**放在被测文件中
- 测试文件**必须**放在被测文件所在的同级的`test/` 目录下

### 文件命名

- 文件名格式：`<被测模块名>.test.rs`
- 例如：被测文件是 `**/calculator.rs`，测试文件应为 `**/test/calculator.test.rs`
