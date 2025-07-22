//! CLI 集成测试
//!
//! 测试命令行接口的各种功能和参数组合

use std::process::Command;
use tempfile::TempDir;

/// 获取编译后的二进制文件路径
fn get_binary_path() -> String {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // 移除测试可执行文件名
    if path.ends_with("deps") {
        path.pop(); // 移除 deps 目录
    }
    path.push("semantic-diff");
    path.to_string_lossy().to_string()
}

/// 创建测试用的临时 Git 仓库
fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // 初始化 Git 仓库
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to initialize git repo");

    // 配置 Git 用户
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to configure git user");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to configure git email");

    // 创建一个简单的 Go 文件
    let go_content = r#"package main

import "fmt"

func main() {
    fmt.Println("Hello, World!")
}
"#;

    std::fs::write(temp_dir.path().join("main.go"), go_content).expect("Failed to write Go file");

    // 添加并提交文件
    Command::new("git")
        .args(["add", "."])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to add files");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to commit files");

    temp_dir
}

#[test]
fn test_help_output() {
    let output = Command::new(get_binary_path())
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("semantic-diff"));
    assert!(stdout.contains("COMMIT_HASH"));
    assert!(stdout.contains("--format"));
    assert!(stdout.contains("--highlight"));
}

#[test]
fn test_version_output() {
    let output = Command::new(get_binary_path())
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_missing_commit_hash() {
    let output = Command::new(get_binary_path())
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("required") || stderr.contains("COMMIT_HASH"));
}

#[test]
fn test_invalid_commit_hash() {
    let output = Command::new(get_binary_path())
        .args(["invalid-hash-with-special-chars!@#"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Invalid commit hash format"));
}

#[test]
fn test_short_commit_hash() {
    let output = Command::new(get_binary_path())
        .args(["abc123"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Commit hash length must be between 7 and 40 characters"));
}

#[test]
fn test_valid_commit_hash_format() {
    let _temp_repo = create_test_repo();

    let output = Command::new(get_binary_path())
        .args(["abcdef1", "--repo", _temp_repo.path().to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    // 即使提交不存在，哈希格式验证应该通过
    // 错误应该来自 Git 操作，而不是参数验证
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("Invalid commit hash format"));
    assert!(!stderr.contains("Commit hash length"));
}

#[test]
fn test_output_format_options() {
    let formats = ["text", "markdown", "html"];

    for format in &formats {
        let output = Command::new(get_binary_path())
            .args(["abcdef1234", "--format", format])
            .output()
            .expect("Failed to execute command");

        // 格式参数应该被接受
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("invalid value"));
        assert!(!stderr.contains(&format!("'{format}' isn't a valid value")));
    }
}

#[test]
fn test_highlight_style_options() {
    let styles = ["none", "inline", "separate"];

    for style in &styles {
        let output = Command::new(get_binary_path())
            .args(["abcdef1234", "--highlight", style])
            .output()
            .expect("Failed to execute command");

        // 高亮样式参数应该被接受
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("invalid value"));
        assert!(!stderr.contains(&format!("'{style}' isn't a valid value")));
    }
}

#[test]
fn test_max_depth_validation() {
    // 测试有效范围
    let output = Command::new(get_binary_path())
        .args(["abcdef1234", "--max-depth", "5"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("isn't in range"));

    // 测试超出范围的值
    let output = Command::new(get_binary_path())
        .args(["abcdef1234", "--max-depth", "15"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("isn't in range") || stderr.contains("invalid value"));
}

#[test]
fn test_nonexistent_repo_path() {
    let output = Command::new(get_binary_path())
        .args(["abcdef1234", "--repo", "/nonexistent/path"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Repository path does not exist"));
}

#[test]
fn test_verbose_flag() {
    let temp_repo = create_test_repo();

    let output = Command::new(get_binary_path())
        .args([
            "abcdef1234",
            "--repo",
            temp_repo.path().to_str().unwrap(),
            "--verbose",
        ])
        .output()
        .expect("Failed to execute command");

    // verbose 标志应该被接受，不应该有参数错误
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("unexpected argument"));
    assert!(!stderr.contains("invalid value"));
}

#[test]
fn test_output_file_option() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let output_file = temp_dir.path().join("output.txt");

    let output = Command::new(get_binary_path())
        .args(["abcdef1234", "--output", output_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    // 输出文件参数应该被接受
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("unexpected argument"));
}

#[test]
fn test_boolean_flags() {
    let flags = ["--exclude-tests", "--functions-only", "--include-comments"];

    for flag in &flags {
        let output = Command::new(get_binary_path())
            .args(["abcdef1234", flag])
            .output()
            .expect("Failed to execute command");

        // 布尔标志应该被接受
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("unexpected argument"));
        assert!(!stderr.contains(&format!("'{flag}' isn't a valid value")));
    }
}

#[test]
fn test_complex_argument_combination() {
    let temp_repo = create_test_repo();
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let output_file = temp_dir.path().join("result.md");

    let output = Command::new(get_binary_path())
        .args([
            "abcdef1234567",
            "--repo",
            temp_repo.path().to_str().unwrap(),
            "--format",
            "markdown",
            "--highlight",
            "separate",
            "--max-depth",
            "2",
            "--output",
            output_file.to_str().unwrap(),
            "--exclude-tests",
            "--functions-only",
            "--verbose",
        ])
        .output()
        .expect("Failed to execute command");

    // 复杂的参数组合应该被正确解析
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("unexpected argument"));
    assert!(!stderr.contains("invalid value"));
    assert!(!stderr.contains("Invalid commit hash format"));
}
