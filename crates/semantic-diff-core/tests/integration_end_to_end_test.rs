//! 端到端集成测试
//!
//! 使用真实的 Git 仓库测试完整的工作流程

use semantic_diff_core::{
    Result, analyzer::SourceAnalyzer, extractor::SemanticContextExtractor,
    generator::CodeSliceGenerator, git::GitDiffParser, parser::SupportedLanguage,
};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// 端到端测试套件
struct EndToEndTestSuite {
    repo_path: PathBuf,
}

impl EndToEndTestSuite {
    /// 创建测试套件
    fn new() -> Result<Self> {
        let temp_dir =
            TempDir::new().map_err(semantic_diff_core::error::SemanticDiffError::IoError)?;
        let repo_path = temp_dir.path().join("test_repo");

        Ok(Self { repo_path })
    }

    /// 初始化 Git 仓库
    fn init_git_repo(&self) -> Result<()> {
        std::fs::create_dir_all(&self.repo_path)
            .map_err(semantic_diff_core::error::SemanticDiffError::IoError)?;

        // 初始化 Git 仓库
        let output = Command::new("git")
            .args(["init"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                semantic_diff_core::error::SemanticDiffError::GitError(format!(
                    "Failed to init git repo: {e}"
                ))
            })?;

        if !output.status.success() {
            return Err(semantic_diff_core::error::SemanticDiffError::GitError(
                format!(
                    "Git init failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        // 配置 Git 用户
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                semantic_diff_core::error::SemanticDiffError::GitError(format!(
                    "Failed to config git user: {e}"
                ))
            })?;

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                semantic_diff_core::error::SemanticDiffError::GitError(format!(
                    "Failed to config git email: {e}"
                ))
            })?;

        Ok(())
    }

    /// 创建初始提交
    fn create_initial_commit(&self) -> Result<String> {
        // 创建初始文件
        let initial_content = r#"package main

import "fmt"

// User 表示用户
type User struct {
    ID   int
    Name string
}

// NewUser 创建用户
func NewUser(id int, name string) *User {
    return &User{
        ID:   id,
        Name: name,
    }
}

// GetName 获取用户名
func (u *User) GetName() string {
    return u.Name
}

func main() {
    user := NewUser(1, "Alice")
    fmt.Println(user.GetName())
}
"#;

        let file_path = self.repo_path.join("main.go");
        std::fs::write(&file_path, initial_content)
            .map_err(semantic_diff_core::error::SemanticDiffError::IoError)?;

        // 添加文件到 Git
        Command::new("git")
            .args(["add", "main.go"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                semantic_diff_core::error::SemanticDiffError::GitError(format!(
                    "Failed to add file: {e}"
                ))
            })?;

        // 创建提交
        let output = Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                semantic_diff_core::error::SemanticDiffError::GitError(format!(
                    "Failed to commit: {e}"
                ))
            })?;

        if !output.status.success() {
            return Err(semantic_diff_core::error::SemanticDiffError::GitError(
                format!(
                    "Git commit failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        // 获取提交哈希
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                semantic_diff_core::error::SemanticDiffError::GitError(format!(
                    "Failed to get commit hash: {e}"
                ))
            })?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// 创建修改提交
    fn create_modification_commit(&self) -> Result<String> {
        // 修改文件内容
        let modified_content = r#"package main

import (
    "fmt"
    "time"
)

// User 表示用户
type User struct {
    ID        int
    Name      string
    Email     string    // 新增字段
    CreatedAt time.Time // 新增字段
}

// NewUser 创建用户
func NewUser(id int, name string) *User {
    return &User{
        ID:        id,
        Name:      name,
        CreatedAt: time.Now(), // 新增逻辑
    }
}

// GetName 获取用户名
func (u *User) GetName() string {
    return u.Name
}

// GetEmail 获取用户邮箱 - 新增方法
func (u *User) GetEmail() string {
    return u.Email
}

// SetEmail 设置用户邮箱 - 新增方法
func (u *User) SetEmail(email string) {
    u.Email = email
}

// Validate 验证用户数据 - 新增方法
func (u *User) Validate() error {
    if u.Name == "" {
        return fmt.Errorf("name cannot be empty")
    }
    return nil
}

func main() {
    user := NewUser(1, "Alice")
    user.SetEmail("alice@example.com") // 新增逻辑
    
    if err := user.Validate(); err != nil { // 新增逻辑
        fmt.Printf("Validation error: %v\n", err)
        return
    }
    
    fmt.Printf("User: %s <%s>\n", user.GetName(), user.GetEmail())
}
"#;

        let file_path = self.repo_path.join("main.go");
        std::fs::write(&file_path, modified_content)
            .map_err(semantic_diff_core::error::SemanticDiffError::IoError)?;

        // 添加修改到 Git
        Command::new("git")
            .args(["add", "main.go"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                semantic_diff_core::error::SemanticDiffError::GitError(format!(
                    "Failed to add modified file: {e}"
                ))
            })?;

        // 创建提交
        let output = Command::new("git")
            .args(["commit", "-m", "Add email field and validation methods"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                semantic_diff_core::error::SemanticDiffError::GitError(format!(
                    "Failed to commit modification: {e}"
                ))
            })?;

        if !output.status.success() {
            return Err(semantic_diff_core::error::SemanticDiffError::GitError(
                format!(
                    "Git commit failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        // 获取提交哈希
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| {
                semantic_diff_core::error::SemanticDiffError::GitError(format!(
                    "Failed to get commit hash: {e}"
                ))
            })?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[test]
fn test_end_to_end_workflow() -> Result<()> {
    // 创建测试套件
    let suite = EndToEndTestSuite::new()?;

    // 初始化 Git 仓库
    suite.init_git_repo()?;

    // 创建初始提交
    let _initial_commit = suite.create_initial_commit()?;

    // 创建修改提交
    let modification_commit = suite.create_modification_commit()?;

    // 1. 解析 Git 差异
    let git_parser = GitDiffParser::new(suite.repo_path.clone())?;
    let file_changes = git_parser.parse_commit(&modification_commit)?;

    assert!(!file_changes.is_empty(), "应该有文件变更");

    // 2. 分析变更的文件
    let mut analyzer = SourceAnalyzer::new_for_language(SupportedLanguage::Go)?;

    for file_change in &file_changes {
        if file_change.file_path.extension().and_then(|s| s.to_str()) == Some("go") {
            // 分析文件 - 使用完整路径
            let full_path = suite.repo_path.join(&file_change.file_path);
            let source_file = analyzer.analyze_file(&full_path)?;

            // 查找变更的函数
            let changed_functions =
                analyzer.find_changed_functions(&source_file, &file_change.hunks)?;

            assert!(!changed_functions.is_empty(), "应该找到变更的函数");

            // 3. 提取语义上下文
            let extractor = SemanticContextExtractor::new();

            for function in &changed_functions {
                let context = extractor.extract_context(function, &[source_file.clone()])?;

                // 验证上下文包含必要信息
                assert!(!context.change_target.name().is_empty(), "函数名不应为空");

                // 4. 生成代码切片
                let generator = CodeSliceGenerator::new();
                let code_slice = generator.generate_slice(&context, &file_change.hunks)?;

                // 验证生成的代码切片
                assert!(
                    !code_slice.function_definitions.is_empty(),
                    "应该包含函数定义"
                );

                println!("成功生成代码切片，函数: {}", function.name);
                println!("代码切片长度: {} 行", code_slice.function_definitions.len());
            }
        }
    }

    Ok(())
}

#[test]
fn test_multiple_file_changes() -> Result<()> {
    let suite = EndToEndTestSuite::new()?;
    suite.init_git_repo()?;

    // 创建多个文件的初始版本
    let files = [
        (
            "user.go",
            r#"package main

type User struct {
    Name string
}

func (u *User) GetName() string {
    return u.Name
}
"#,
        ),
        (
            "storage.go",
            r#"package main

import "fmt"

type Storage struct {
    data map[string]string
}

func NewStorage() *Storage {
    return &Storage{
        data: make(map[string]string),
    }
}

func (s *Storage) Save(key, value string) error {
    s.data[key] = value
    return nil
}
"#,
        ),
    ];

    // 创建初始文件
    for (filename, content) in &files {
        let file_path = suite.repo_path.join(filename);
        std::fs::write(&file_path, content)
            .map_err(semantic_diff_core::error::SemanticDiffError::IoError)?;
    }

    // 提交初始版本
    Command::new("git")
        .args(["add", "."])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!(
                "Failed to add files: {e}"
            ))
        })?;

    Command::new("git")
        .args(["commit", "-m", "Initial multi-file commit"])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!("Failed to commit: {e}"))
        })?;

    // 修改多个文件
    let modified_files = [
        (
            "user.go",
            r#"package main

import "fmt"

type User struct {
    Name  string
    Email string // 新增字段
}

func (u *User) GetName() string {
    return u.Name
}

// 新增方法
func (u *User) GetEmail() string {
    return u.Email
}

// 新增方法
func (u *User) Validate() error {
    if u.Name == "" {
        return fmt.Errorf("name cannot be empty")
    }
    return nil
}
"#,
        ),
        (
            "storage.go",
            r#"package main

import "fmt"

type Storage struct {
    data map[string]string
}

func NewStorage() *Storage {
    return &Storage{
        data: make(map[string]string),
    }
}

func (s *Storage) Save(key, value string) error {
    if key == "" { // 新增验证
        return fmt.Errorf("key cannot be empty")
    }
    s.data[key] = value
    return nil
}

// 新增方法
func (s *Storage) Load(key string) (string, error) {
    value, exists := s.data[key]
    if !exists {
        return "", fmt.Errorf("key not found: %s", key)
    }
    return value, nil
}
"#,
        ),
    ];

    // 写入修改后的文件
    for (filename, content) in &modified_files {
        let file_path = suite.repo_path.join(filename);
        std::fs::write(&file_path, content)
            .map_err(semantic_diff_core::error::SemanticDiffError::IoError)?;
    }

    // 提交修改
    Command::new("git")
        .args(["add", "."])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!(
                "Failed to add modified files: {e}"
            ))
        })?;

    let output = Command::new("git")
        .args(["commit", "-m", "Modify multiple files"])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!(
                "Failed to commit modifications: {e}"
            ))
        })?;

    if !output.status.success() {
        return Err(semantic_diff_core::error::SemanticDiffError::GitError(
            format!(
                "Git commit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    // 获取提交哈希
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!(
                "Failed to get commit hash: {e}"
            ))
        })?;

    let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // 测试完整工作流程
    let git_parser = GitDiffParser::new(suite.repo_path.clone())?;
    let file_changes = git_parser.parse_commit(&commit_hash)?;

    // 应该有两个文件的变更
    assert_eq!(file_changes.len(), 2, "应该有两个文件的变更");

    let mut total_functions_found = 0;
    let mut analyzer = SourceAnalyzer::new_for_language(SupportedLanguage::Go)?;

    for file_change in &file_changes {
        if file_change.file_path.extension().and_then(|s| s.to_str()) == Some("go") {
            let full_path = suite.repo_path.join(&file_change.file_path);
            let source_file = analyzer.analyze_file(&full_path)?;
            let changed_functions =
                analyzer.find_changed_functions(&source_file, &file_change.hunks)?;

            total_functions_found += changed_functions.len();

            println!(
                "文件 {:?} 中找到 {} 个变更的函数",
                file_change.file_path.file_name(),
                changed_functions.len()
            );
        }
    }

    assert!(total_functions_found > 0, "应该找到变更的函数");

    Ok(())
}

#[test]
fn test_complex_go_structures() -> Result<()> {
    let suite = EndToEndTestSuite::new()?;
    suite.init_git_repo()?;

    // 创建包含复杂 Go 结构的文件
    let complex_content = r#"package main

import (
    "context"
    "fmt"
    "sync"
)

// Service 接口
type Service interface {
    Process(ctx context.Context, data []byte) error
    Close() error
}

// Config 配置结构
type Config struct {
    Database struct {
        Host string
        Port int
    }
    Features map[string]bool
}

// Manager 管理器
type Manager struct {
    services []Service
    config   *Config
    mutex    sync.RWMutex
}

// NewManager 创建管理器
func NewManager(config *Config) *Manager {
    return &Manager{
        config: config,
        services: make([]Service, 0),
    }
}

// AddService 添加服务
func (m *Manager) AddService(service Service) {
    m.mutex.Lock()
    defer m.mutex.Unlock()
    m.services = append(m.services, service)
}
"#;

    let file_path = suite.repo_path.join("complex.go");
    std::fs::write(&file_path, complex_content)
        .map_err(semantic_diff_core::error::SemanticDiffError::IoError)?;

    // 提交初始版本
    Command::new("git")
        .args(["add", "complex.go"])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!(
                "Failed to add file: {e}"
            ))
        })?;

    Command::new("git")
        .args(["commit", "-m", "Add complex structures"])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!("Failed to commit: {e}"))
        })?;

    // 修改文件，添加更多复杂功能
    let modified_complex_content = r#"package main

import (
    "context"
    "fmt"
    "sync"
    "time"
)

// Service 接口
type Service interface {
    Process(ctx context.Context, data []byte) error
    Close() error
    Status() string // 新增方法
}

// Config 配置结构
type Config struct {
    Database struct {
        Host     string
        Port     int
        Timeout  time.Duration // 新增字段
        MaxConns int           // 新增字段
    }
    Features map[string]bool
    Logging  struct { // 新增嵌套结构
        Level  string
        Output string
    }
}

// Manager 管理器
type Manager struct {
    services []Service
    config   *Config
    mutex    sync.RWMutex
    stats    *Stats // 新增字段
}

// Stats 统计信息 - 新增结构
type Stats struct {
    ProcessedCount int64
    ErrorCount     int64
    LastProcessed  time.Time
}

// NewManager 创建管理器
func NewManager(config *Config) *Manager {
    return &Manager{
        config:   config,
        services: make([]Service, 0),
        stats:    &Stats{}, // 新增初始化
    }
}

// AddService 添加服务
func (m *Manager) AddService(service Service) {
    m.mutex.Lock()
    defer m.mutex.Unlock()
    m.services = append(m.services, service)
}

// ProcessAll 处理所有服务 - 新增方法
func (m *Manager) ProcessAll(ctx context.Context, data []byte) error {
    m.mutex.RLock()
    services := make([]Service, len(m.services))
    copy(services, m.services)
    m.mutex.RUnlock()
    
    for _, service := range services {
        if err := service.Process(ctx, data); err != nil {
            m.stats.ErrorCount++
            return fmt.Errorf("service processing failed: %w", err)
        }
    }
    
    m.stats.ProcessedCount++
    m.stats.LastProcessed = time.Now()
    return nil
}

// GetStats 获取统计信息 - 新增方法
func (m *Manager) GetStats() Stats {
    m.mutex.RLock()
    defer m.mutex.RUnlock()
    return *m.stats
}
"#;

    std::fs::write(&file_path, modified_complex_content)
        .map_err(semantic_diff_core::error::SemanticDiffError::IoError)?;

    // 提交修改
    Command::new("git")
        .args(["add", "complex.go"])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!(
                "Failed to add modified file: {e}"
            ))
        })?;

    let output = Command::new("git")
        .args(["commit", "-m", "Add complex features and stats"])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!("Failed to commit: {e}"))
        })?;

    if !output.status.success() {
        return Err(semantic_diff_core::error::SemanticDiffError::GitError(
            format!(
                "Git commit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    // 获取提交哈希并测试
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&suite.repo_path)
        .output()
        .map_err(|e| {
            semantic_diff_core::error::SemanticDiffError::GitError(format!(
                "Failed to get commit hash: {e}"
            ))
        })?;

    let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // 测试复杂结构的解析
    let git_parser = GitDiffParser::new(suite.repo_path.clone())?;
    let file_changes = git_parser.parse_commit(&commit_hash)?;

    assert!(!file_changes.is_empty(), "应该有文件变更");

    let mut analyzer = SourceAnalyzer::new_for_language(SupportedLanguage::Go)?;
    let extractor = SemanticContextExtractor::new();
    let generator = CodeSliceGenerator::new();

    for file_change in &file_changes {
        if file_change.file_path.extension().and_then(|s| s.to_str()) == Some("go") {
            let full_path = suite.repo_path.join(&file_change.file_path);
            let source_file = analyzer.analyze_file(&full_path)?;
            let changed_functions =
                analyzer.find_changed_functions(&source_file, &file_change.hunks)?;

            for function in &changed_functions {
                let context = extractor.extract_context(function, &[source_file.clone()])?;
                let code_slice = generator.generate_slice(&context, &file_change.hunks)?;

                // 验证复杂结构的处理
                // 注意：不是所有函数都会有相关类型，这取决于函数的具体实现
                println!("相关类型数量: {}", context.related_types.len());
                println!("类型定义数量: {}", code_slice.type_definitions.len());

                // 至少应该有一些内容
                assert!(!code_slice.content.is_empty(), "代码切片内容不应为空");

                println!("成功处理复杂结构，函数: {}", function.name);
            }
        }
    }

    Ok(())
}
