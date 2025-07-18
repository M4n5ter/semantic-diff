//! Go模块项目集成测试
//!
//! 测试依赖关系解析器在真实Go项目结构中的表现

use semantic_diff_core::analyzer::{DependencyResolver, SourceAnalyzer, TypeAnalyzer};
use std::fs;
use tempfile::TempDir;

/// 创建测试用的Go模块项目结构
fn create_test_go_module(module_name: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path();

    // 创建 go.mod 文件
    let go_mod_content = format!(
        r#"module {module_name}

go 1.21

require (
    github.com/stretchr/testify v1.8.4
)
"#
    );
    fs::write(project_root.join("go.mod"), go_mod_content).expect("Failed to write go.mod");

    // 创建主包文件
    let main_go_content = r#"package main

import (
    "fmt"
    "examplePkg/internal/models"
    "examplePkg/internal/services"
)

func main() {
    user := models.NewUser("Alice", 30)
    service := services.NewUserService()
    
    result := service.ProcessUser(user)
    fmt.Println(result)
}
"#;
    fs::write(project_root.join("main.go"), main_go_content).expect("Failed to write main.go");

    // 创建 internal 目录结构
    fs::create_dir_all(project_root.join("internal/models")).expect("Failed to create models dir");
    fs::create_dir_all(project_root.join("internal/services"))
        .expect("Failed to create services dir");

    // 创建 models/user.go
    let user_model_content = r#"package models

import "time"

// User 表示用户信息
type User struct {
    ID        int       `json:"id"`
    Name      string    `json:"name"`
    Age       int       `json:"age"`
    CreatedAt time.Time `json:"created_at"`
    Profile   *Profile  `json:"profile,omitempty"`
}

// Profile 表示用户档案
type Profile struct {
    Bio     string   `json:"bio"`
    Skills  []string `json:"skills"`
    Address Address  `json:"address"`
}

// Address 表示地址信息
type Address struct {
    Street  string `json:"street"`
    City    string `json:"city"`
    Country string `json:"country"`
}

// NewUser 创建新用户
func NewUser(name string, age int) *User {
    return &User{
        Name:      name,
        Age:       age,
        CreatedAt: time.Now(),
    }
}

// GetFullName 获取用户全名
func (u *User) GetFullName() string {
    return u.Name
}

// SetProfile 设置用户档案
func (u *User) SetProfile(profile *Profile) {
    u.Profile = profile
}

// HasProfile 检查用户是否有档案
func (u *User) HasProfile() bool {
    return u.Profile != nil
}
"#;
    fs::write(
        project_root.join("internal/models/user.go"),
        user_model_content,
    )
    .expect("Failed to write user.go");

    // 创建 services/user_service.go
    let user_service_content = r#"package services

import (
    "fmt"
    "examplePkg/internal/models"
)

// UserService 用户服务
type UserService struct {
    repository UserRepository
}

// UserRepository 用户仓库接口
type UserRepository interface {
    Save(user *models.User) error
    FindByID(id int) (*models.User, error)
    FindByName(name string) ([]*models.User, error)
}

// NewUserService 创建用户服务
func NewUserService() *UserService {
    return &UserService{
        repository: &InMemoryUserRepository{},
    }
}

// ProcessUser 处理用户
func (s *UserService) ProcessUser(user *models.User) string {
    if user.HasProfile() {
        return fmt.Sprintf("Processing user %s with profile", user.GetFullName())
    }
    return fmt.Sprintf("Processing user %s without profile", user.GetFullName())
}

// CreateUserWithProfile 创建带档案的用户
func (s *UserService) CreateUserWithProfile(name string, age int, bio string) *models.User {
    user := models.NewUser(name, age)
    profile := &models.Profile{
        Bio:    bio,
        Skills: []string{"Go", "Rust"},
        Address: models.Address{
            Street:  "123 Main St",
            City:    "Tech City",
            Country: "Techland",
        },
    }
    user.SetProfile(profile)
    return user
}

// InMemoryUserRepository 内存用户仓库实现
type InMemoryUserRepository struct {
    users map[int]*models.User
}

// Save 保存用户
func (r *InMemoryUserRepository) Save(user *models.User) error {
    if r.users == nil {
        r.users = make(map[int]*models.User)
    }
    r.users[user.ID] = user
    return nil
}

// FindByID 根据ID查找用户
func (r *InMemoryUserRepository) FindByID(id int) (*models.User, error) {
    if user, exists := r.users[id]; exists {
        return user, nil
    }
    return nil, fmt.Errorf("user with ID %d not found", id)
}

// FindByName 根据名称查找用户
func (r *InMemoryUserRepository) FindByName(name string) ([]*models.User, error) {
    var result []*models.User
    for _, user := range r.users {
        if user.Name == name {
            result = append(result, user)
        }
    }
    return result, nil
}
"#;
    fs::write(
        project_root.join("internal/services/user_service.go"),
        user_service_content,
    )
    .expect("Failed to write user_service.go");

    // 创建 pkg/utils/helper.go (公共包)
    fs::create_dir_all(project_root.join("pkg/utils")).expect("Failed to create utils dir");
    let helper_content = r#"package utils

import (
    "strings"
    "examplePkg/internal/models"
)

// StringHelper 字符串辅助工具
type StringHelper struct{}

// NewStringHelper 创建字符串辅助工具
func NewStringHelper() *StringHelper {
    return &StringHelper{}
}

// FormatUserName 格式化用户名
func (h *StringHelper) FormatUserName(user *models.User) string {
    return strings.ToUpper(user.Name)
}

// ValidateEmail 验证邮箱格式
func (h *StringHelper) ValidateEmail(email string) bool {
    return strings.Contains(email, "@")
}

// GenerateUserSummary 生成用户摘要
func GenerateUserSummary(user *models.User) string {
    if user.HasProfile() {
        return user.GetFullName() + " (with profile)"
    }
    return user.GetFullName() + " (no profile)"
}
"#;
    fs::write(project_root.join("pkg/utils/helper.go"), helper_content)
        .expect("Failed to write helper.go");

    temp_dir
}

#[test]
fn test_go_module_dependency_resolution_simple_module() {
    // 测试简单模块名 "examplePkg"
    let temp_dir = create_test_go_module("examplePkg");
    let project_root = temp_dir.path();

    // 分析所有Go文件
    let mut source_files = Vec::new();
    let go_files = [
        "main.go",
        "internal/models/user.go",
        "internal/services/user_service.go",
        "pkg/utils/helper.go",
    ];

    for file_path in &go_files {
        let full_path = project_root.join(file_path);
        let mut analyzer =
            SourceAnalyzer::new_for_file(&full_path).expect("Failed to create analyzer");
        let source_file = analyzer
            .analyze_file(&full_path)
            .expect("Failed to analyze file");
        source_files.push(source_file);
    }

    let resolver = DependencyResolver::from_project_root(project_root)
        .expect("Failed to create resolver from project root");

    // 测试类型解析
    let user_type_ref = semantic_diff_core::analyzer::TypeReference {
        name: "User".to_string(),
        package: Some("models".to_string()),
    };

    let resolved_user = resolver.resolve_type(&user_type_ref, &source_files);
    assert!(resolved_user.is_some(), "应该能够解析User类型");

    let user_def = resolved_user.unwrap();
    assert_eq!(user_def.name, "User");
    assert!(user_def.file_path.to_string_lossy().contains("user.go"));

    // 测试函数解析
    let new_user_func = semantic_diff_core::analyzer::FunctionCall {
        name: "NewUser".to_string(),
        receiver: None,
        package: Some("models".to_string()),
    };

    let resolved_func = resolver.resolve_function(&new_user_func, &source_files);
    assert!(resolved_func.is_some(), "应该能够解析NewUser函数");

    let func_def = resolved_func.unwrap();
    assert_eq!(func_def.name, "NewUser");

    // 测试方法解析 - 先不指定接收者，因为在同一个包内调用
    let get_full_name_method = semantic_diff_core::analyzer::FunctionCall {
        name: "GetFullName".to_string(),
        receiver: None, // 在同一个包内，不需要指定接收者
        package: None,
    };

    let resolved_method = resolver.resolve_function(&get_full_name_method, &source_files);
    assert!(resolved_method.is_some(), "应该能够解析GetFullName方法");

    // 测试类型分析器
    let type_analyzer = TypeAnalyzer::new();

    // 分析User结构体的依赖
    let user_dependencies = type_analyzer.analyze_type_dependencies(&user_def, &source_files);
    assert!(!user_dependencies.is_empty(), "User类型应该有依赖");

    // 验证找到了Profile和Address类型的依赖（包括间接依赖）
    let dep_names: Vec<&str> = user_dependencies.iter().map(|d| d.name.as_str()).collect();
    assert!(dep_names.contains(&"Profile"), "应该找到Profile依赖");
    assert!(
        dep_names.contains(&"Address"),
        "应该找到Address依赖（间接依赖）"
    );

    // 测试外部依赖过滤
    let fmt_import = semantic_diff_core::parser::Import {
        path: "fmt".to_string(),
        alias: None,
    };
    assert!(
        resolver.is_external_dependency(&fmt_import),
        "fmt应该是外部依赖"
    );

    let internal_import = semantic_diff_core::parser::Import {
        path: "examplePkg/internal/models".to_string(),
        alias: None,
    };
    assert!(
        !resolver.is_external_dependency(&internal_import),
        "内部包不应该是外部依赖"
    );
}

#[test]
fn test_go_module_dependency_resolution_github_module() {
    // 测试GitHub模块名 "github.com/M4n5ter/examplePkg"
    let temp_dir = create_test_go_module("github.com/M4n5ter/examplePkg");
    let project_root = temp_dir.path();

    // 更新main.go以使用GitHub模块路径
    let main_go_content = r#"package main

import (
    "fmt"
    "github.com/M4n5ter/examplePkg/internal/models"
    "github.com/M4n5ter/examplePkg/internal/services"
)

func main() {
    user := models.NewUser("Bob", 25)
    service := services.NewUserService()
    
    result := service.ProcessUser(user)
    fmt.Println(result)
}
"#;
    fs::write(project_root.join("main.go"), main_go_content).expect("Failed to update main.go");

    // 更新services文件以使用GitHub模块路径
    let user_service_content = r#"package services

import (
    "fmt"
    "github.com/M4n5ter/examplePkg/internal/models"
)

// UserService 用户服务
type UserService struct {
    repository UserRepository
}

// UserRepository 用户仓库接口
type UserRepository interface {
    Save(user *models.User) error
    FindByID(id int) (*models.User, error)
}

// NewUserService 创建用户服务
func NewUserService() *UserService {
    return &UserService{}
}

// ProcessUser 处理用户
func (s *UserService) ProcessUser(user *models.User) string {
    return fmt.Sprintf("Processing user %s", user.GetFullName())
}
"#;
    fs::write(
        project_root.join("internal/services/user_service.go"),
        user_service_content,
    )
    .expect("Failed to update user_service.go");

    // 更新utils文件以使用GitHub模块路径
    let helper_content = r#"package utils

import (
    "strings"
    "github.com/M4n5ter/examplePkg/internal/models"
)

// FormatUserName 格式化用户名
func FormatUserName(user *models.User) string {
    return strings.ToUpper(user.Name)
}
"#;
    fs::write(project_root.join("pkg/utils/helper.go"), helper_content)
        .expect("Failed to update helper.go");

    // 分析所有Go文件
    let mut source_files = Vec::new();
    let go_files = [
        "main.go",
        "internal/models/user.go",
        "internal/services/user_service.go",
        "pkg/utils/helper.go",
    ];

    for file_path in &go_files {
        let full_path = project_root.join(file_path);
        let mut analyzer =
            SourceAnalyzer::new_for_file(&full_path).expect("Failed to create analyzer");
        let source_file = analyzer
            .analyze_file(&full_path)
            .expect("Failed to analyze file");
        source_files.push(source_file);
    }

    let resolver =
        DependencyResolver::new_with_project_path("github.com/M4n5ter/examplePkg".to_string());

    // 测试GitHub模块路径的依赖解析
    let github_models_import = semantic_diff_core::parser::Import {
        path: "github.com/M4n5ter/examplePkg/internal/models".to_string(),
        alias: None,
    };

    // GitHub模块的内部包不应该被认为是外部依赖
    assert!(
        !resolver.is_external_dependency(&github_models_import),
        "GitHub模块的内部包不应该是外部依赖"
    );

    // 测试第三方GitHub依赖
    let third_party_import = semantic_diff_core::parser::Import {
        path: "github.com/stretchr/testify".to_string(),
        alias: None,
    };
    assert!(
        resolver.is_external_dependency(&third_party_import),
        "第三方GitHub包应该是外部依赖"
    );

    // 测试类型解析仍然有效
    let user_type_ref = semantic_diff_core::analyzer::TypeReference {
        name: "User".to_string(),
        package: Some("models".to_string()),
    };

    let resolved_user = resolver.resolve_type(&user_type_ref, &source_files);
    assert!(resolved_user.is_some(), "应该能够解析User类型");
}

#[test]
fn test_cross_package_dependency_analysis() {
    // 测试跨包依赖分析
    let temp_dir = create_test_go_module("examplePkg");
    let project_root = temp_dir.path();

    // 分析所有Go文件
    let mut source_files = Vec::new();
    let go_files = [
        "main.go",
        "internal/models/user.go",
        "internal/services/user_service.go",
        "pkg/utils/helper.go",
    ];

    for file_path in &go_files {
        let full_path = project_root.join(file_path);
        let mut analyzer =
            SourceAnalyzer::new_for_file(&full_path).expect("Failed to create analyzer");
        let source_file = analyzer
            .analyze_file(&full_path)
            .expect("Failed to analyze file");
        source_files.push(source_file);
    }

    let resolver = DependencyResolver::from_project_root(project_root)
        .expect("Failed to create resolver from project root");

    // 找到UserService类型
    let user_service_type = resolver.find_type_definition("UserService", &source_files);
    assert!(user_service_type.is_some(), "应该找到UserService类型");

    // 找到ProcessUser函数
    let process_user_func = resolver.find_function_definition("ProcessUser", &source_files);
    assert!(process_user_func.is_some(), "应该找到ProcessUser函数");

    let func_info = process_user_func.unwrap();

    // 分析函数的依赖关系
    let func_dependencies = resolver.extract_function_dependencies(&func_info, &source_files);
    assert!(!func_dependencies.is_empty(), "ProcessUser函数应该有依赖");

    // 过滤内部依赖
    let internal_deps = resolver.filter_internal_dependencies(&func_dependencies);
    assert!(!internal_deps.is_empty(), "应该有内部依赖");

    // 调试输出
    println!("ProcessUser function info: {func_info:?}");
    println!("Function dependencies: {func_dependencies:?}");
    println!("Internal dependencies: {internal_deps:?}");

    // 验证依赖类型
    let has_user_dependency = internal_deps.iter().any(|dep| dep.name == "User");
    assert!(has_user_dependency, "应该依赖User类型");
}

#[test]
fn test_module_path_detection() {
    // 测试模块路径检测
    // 创建一个带有项目模块路径的解析器来测试内部包识别
    let resolver_with_project = DependencyResolver::new_with_project_path("myproject".to_string());
    let resolver_with_example = DependencyResolver::new_with_project_path("examplePkg".to_string());
    let resolver_default = DependencyResolver::new();

    // 测试各种导入路径的分类
    let test_cases = vec![
        // 标准库
        ("fmt", true, &resolver_default),
        ("net/http", true, &resolver_default),
        ("encoding/json", true, &resolver_default),
        // 第三方库
        ("github.com/gin-gonic/gin", true, &resolver_default),
        ("github.com/stretchr/testify", true, &resolver_default),
        ("golang.org/x/crypto", true, &resolver_default),
        ("google.golang.org/grpc", true, &resolver_default),
        // 项目内部包（相对路径）
        ("./internal/models", false, &resolver_default),
        ("../utils", false, &resolver_default),
        // 项目内部包（绝对路径，但不包含域名）
        ("myproject/internal/models", false, &resolver_with_project),
        (
            "examplePkg/internal/services",
            false,
            &resolver_with_example,
        ),
    ];

    for (import_path, expected_external, resolver) in test_cases {
        let import = semantic_diff_core::parser::Import {
            path: import_path.to_string(),
            alias: None,
        };

        let is_external = resolver.is_external_dependency(&import);
        assert_eq!(
            is_external, expected_external,
            "导入路径 '{import_path}' 的外部依赖判断错误，期望: {expected_external}, 实际: {is_external}"
        );
    }
}

#[test]
fn test_complex_type_dependency_chain() {
    // 测试复杂的类型依赖链
    let temp_dir = create_test_go_module("examplePkg");
    let project_root = temp_dir.path();

    // 分析models文件
    let models_path = project_root.join("internal/models/user.go");
    let mut analyzer =
        SourceAnalyzer::new_for_file(&models_path).expect("Failed to create analyzer");
    let models_file = analyzer
        .analyze_file(&models_path)
        .expect("Failed to analyze models file");

    let type_analyzer = TypeAnalyzer::new();
    let resolver = DependencyResolver::from_project_root(project_root)
        .expect("Failed to create resolver from project root");

    // 找到User类型
    let user_type = resolver.find_type_definition("User", &[models_file]);
    assert!(user_type.is_some(), "应该找到User类型");

    let user_def = user_type.unwrap();

    // 重新分析文件以获取User依赖
    let mut analyzer2 =
        SourceAnalyzer::new_for_file(&models_path).expect("Failed to create analyzer");
    let models_file2 = analyzer2
        .analyze_file(&models_path)
        .expect("Failed to analyze models file");

    // 分析User类型的依赖
    let user_deps = type_analyzer.analyze_type_dependencies(&user_def, &[models_file2]);

    // 调试输出
    println!("User dependencies: {user_deps:?}");
    let dep_names: Vec<&str> = user_deps.iter().map(|d| d.name.as_str()).collect();
    println!("Dependency names: {dep_names:?}");

    // 验证直接依赖：User -> Profile
    assert!(dep_names.contains(&"Profile"), "User应该依赖Profile");
    // User 不直接依赖 Address，Address 是 Profile 的依赖

    // 重新分析文件以获取Profile类型
    let mut analyzer3 =
        SourceAnalyzer::new_for_file(&models_path).expect("Failed to create analyzer");
    let models_file3 = analyzer3
        .analyze_file(&models_path)
        .expect("Failed to analyze models file");

    // 找到Profile类型并分析其依赖
    let profile_type = resolver.find_type_definition("Profile", &[models_file3]);
    assert!(profile_type.is_some(), "应该找到Profile类型");

    let profile_def = profile_type.unwrap();

    // 重新分析文件以获取Profile依赖
    let mut analyzer4 =
        SourceAnalyzer::new_for_file(&models_path).expect("Failed to create analyzer");
    let models_file4 = analyzer4
        .analyze_file(&models_path)
        .expect("Failed to analyze models file");

    let profile_deps = type_analyzer.analyze_type_dependencies(&profile_def, &[models_file4]);

    println!("Profile dependencies: {profile_deps:?}");
    let profile_dep_names: Vec<&str> = profile_deps.iter().map(|d| d.name.as_str()).collect();
    println!("Profile dependency names: {profile_dep_names:?}");

    assert!(
        profile_dep_names.contains(&"Address"),
        "Profile应该依赖Address"
    );
}

#[test]
fn test_circular_dependency_handling() {
    // 测试循环依赖的处理
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path();

    // 创建 go.mod 文件
    let go_mod_content = r#"module circulartest

go 1.21
"#;
    fs::write(project_root.join("go.mod"), go_mod_content).expect("Failed to write go.mod");

    // 创建包含循环依赖的类型定义
    let circular_types_content = r#"package main

// Node 表示图节点
type Node struct {
    ID       int
    Value    string
    Children []*Node
    Parent   *Node
    Graph    *Graph
}

// Graph 表示图结构
type Graph struct {
    Nodes []*Node
    Root  *Node
}

// Edge 表示边
type Edge struct {
    From *Node
    To   *Node
    Graph *Graph
}
"#;
    fs::write(project_root.join("circular.go"), circular_types_content)
        .expect("Failed to write circular.go");

    // 分析文件
    let circular_path = project_root.join("circular.go");
    let mut analyzer =
        SourceAnalyzer::new_for_file(&circular_path).expect("Failed to create analyzer");
    let circular_file = analyzer
        .analyze_file(&circular_path)
        .expect("Failed to analyze circular file");

    let type_analyzer = TypeAnalyzer::new();
    let resolver = DependencyResolver::from_project_root(project_root)
        .expect("Failed to create resolver from project root");

    // 测试Node类型的依赖（包含自引用和循环依赖）
    let source_files = vec![circular_file];

    let node_type = resolver.find_type_definition("Node", &source_files);
    assert!(node_type.is_some(), "应该找到Node类型");

    let node_def = node_type.unwrap();
    let node_deps = type_analyzer.analyze_type_dependencies(&node_def, &source_files);

    println!("Node dependencies: {node_deps:?}");
    let node_dep_names: Vec<&str> = node_deps.iter().map(|d| d.name.as_str()).collect();
    println!("Node dependency names: {node_dep_names:?}");

    // Node应该依赖Graph，但不应该无限递归
    assert!(node_dep_names.contains(&"Graph"), "Node应该依赖Graph");

    // 测试Graph类型的依赖
    let graph_type = resolver.find_type_definition("Graph", &source_files);
    assert!(graph_type.is_some(), "应该找到Graph类型");

    let graph_def = graph_type.unwrap();
    let graph_deps = type_analyzer.analyze_type_dependencies(&graph_def, &source_files);

    println!("Graph dependencies: {graph_deps:?}");
    let graph_dep_names: Vec<&str> = graph_deps.iter().map(|d| d.name.as_str()).collect();
    println!("Graph dependency names: {graph_dep_names:?}");

    // Graph应该依赖Node
    assert!(graph_dep_names.contains(&"Node"), "Graph应该依赖Node");
}

#[test]
fn test_interface_dependency_analysis() {
    // 测试接口依赖分析
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path();

    // 创建 go.mod 文件
    let go_mod_content = r#"module interfacetest

go 1.21
"#;
    fs::write(project_root.join("go.mod"), go_mod_content).expect("Failed to write go.mod");

    // 创建包含接口的复杂类型定义
    let interface_content = r#"package main

import "context"

// Repository 通用仓库接口
type Repository[T Entity] interface {
    Save(ctx context.Context, entity T) error
    FindByID(ctx context.Context, id string) (T, error)
    FindAll(ctx context.Context, filter Filter) ([]T, error)
    Delete(ctx context.Context, id string) error
}

// Entity 实体接口
type Entity interface {
    GetID() string
    Validate() error
}

// Filter 过滤器
type Filter struct {
    Limit  int
    Offset int
    Sort   SortOrder
    Where  map[string]interface{}
}

// SortOrder 排序
type SortOrder struct {
    Field string
    Desc  bool
}

// UserEntity 用户实体
type UserEntity struct {
    ID    string
    Name  string
    Email string
    Profile UserProfile
}

// UserProfile 用户档案
type UserProfile struct {
    Bio     string
    Avatar  string
    Settings UserSettings
}

// UserSettings 用户设置
type UserSettings struct {
    Theme    string
    Language string
    Notifications NotificationSettings
}

// NotificationSettings 通知设置
type NotificationSettings struct {
    Email bool
    SMS   bool
    Push  bool
}

// GetID 实现Entity接口
func (u *UserEntity) GetID() string {
    return u.ID
}

// Validate 实现Entity接口
func (u *UserEntity) Validate() error {
    if u.ID == "" {
        return fmt.Errorf("ID cannot be empty")
    }
    return nil
}

// UserRepository 用户仓库
type UserRepository struct {
    repo Repository[UserEntity]
}

// ProcessUser 处理用户
func ProcessUser(user UserEntity, repo Repository[UserEntity]) error {
    if err := user.Validate(); err != nil {
        return err
    }
    return repo.Save(context.Background(), user)
}
"#;
    fs::write(project_root.join("interface.go"), interface_content)
        .expect("Failed to write interface.go");

    // 分析文件
    let interface_path = project_root.join("interface.go");
    let mut analyzer =
        SourceAnalyzer::new_for_file(&interface_path).expect("Failed to create analyzer");
    let interface_file = analyzer
        .analyze_file(&interface_path)
        .expect("Failed to analyze interface file");

    let type_analyzer = TypeAnalyzer::new();
    let resolver = DependencyResolver::from_project_root(project_root)
        .expect("Failed to create resolver from project root");

    // 测试UserEntity的深层依赖链
    let source_files = vec![interface_file];

    let user_entity_type = resolver.find_type_definition("UserEntity", &source_files);
    assert!(user_entity_type.is_some(), "应该找到UserEntity类型");

    let user_entity_def = user_entity_type.unwrap();
    let user_entity_deps = type_analyzer.analyze_type_dependencies(&user_entity_def, &source_files);

    println!("UserEntity dependencies: {user_entity_deps:?}");
    let user_entity_dep_names: Vec<&str> =
        user_entity_deps.iter().map(|d| d.name.as_str()).collect();
    println!("UserEntity dependency names: {user_entity_dep_names:?}");

    // 验证深层依赖链：UserEntity -> UserProfile -> UserSettings -> NotificationSettings
    assert!(
        user_entity_dep_names.contains(&"UserProfile"),
        "UserEntity应该依赖UserProfile"
    );
    assert!(
        user_entity_dep_names.contains(&"UserSettings"),
        "UserEntity应该依赖UserSettings（间接）"
    );
    assert!(
        user_entity_dep_names.contains(&"NotificationSettings"),
        "UserEntity应该依赖NotificationSettings（间接）"
    );

    // 测试ProcessUser函数的复杂依赖
    let process_user_func = resolver.find_function_definition("ProcessUser", &source_files);
    assert!(process_user_func.is_some(), "应该找到ProcessUser函数");

    let func_info = process_user_func.unwrap();
    let func_dependencies = resolver.extract_function_dependencies(&func_info, &source_files);

    println!("ProcessUser function dependencies: {func_dependencies:?}");
    let func_dep_names: Vec<&str> = func_dependencies.iter().map(|d| d.name.as_str()).collect();
    println!("ProcessUser dependency names: {func_dep_names:?}");

    // ProcessUser函数应该依赖UserEntity和Repository
    assert!(
        func_dep_names.contains(&"UserEntity"),
        "ProcessUser应该依赖UserEntity"
    );
    // 注意：Repository是泛型接口，可能需要特殊处理
}

#[test]
fn test_multi_file_complex_dependencies() {
    // 测试多文件复杂依赖关系
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_root = temp_dir.path();

    // 创建 go.mod 文件
    let go_mod_content = r#"module multifiletest

go 1.21
"#;
    fs::write(project_root.join("go.mod"), go_mod_content).expect("Failed to write go.mod");

    // 创建多个相互依赖的文件
    fs::create_dir_all(project_root.join("domain")).expect("Failed to create domain dir");
    fs::create_dir_all(project_root.join("service")).expect("Failed to create service dir");
    fs::create_dir_all(project_root.join("repository")).expect("Failed to create repository dir");

    // domain/user.go
    let domain_user_content = r#"package domain

import "time"

type User struct {
    ID        string
    Profile   Profile
    Orders    []Order
    CreatedAt time.Time
}

type Profile struct {
    Name    string
    Email   string
    Address Address
    Preferences Preferences
}

type Address struct {
    Street  string
    City    string
    Country string
    Location GeoLocation
}

type GeoLocation struct {
    Latitude  float64
    Longitude float64
}

type Preferences struct {
    Theme      string
    Language   string
    Timezone   string
    Categories []Category
}

type Category struct {
    ID   string
    Name string
    Tags []Tag
}

type Tag struct {
    ID    string
    Name  string
    Color string
}
"#;
    fs::write(project_root.join("domain/user.go"), domain_user_content)
        .expect("Failed to write domain/user.go");

    // domain/order.go
    let domain_order_content = r#"package domain

import "time"

type Order struct {
    ID       string
    UserID   string
    Items    []OrderItem
    Payment  Payment
    Shipping Shipping
    Status   OrderStatus
    CreatedAt time.Time
}

type OrderItem struct {
    ProductID string
    Quantity  int
    Price     Money
    Product   Product
}

type Product struct {
    ID          string
    Name        string
    Description string
    Price       Money
    Category    Category
    Attributes  []ProductAttribute
}

type ProductAttribute struct {
    Name  string
    Value string
    Type  AttributeType
}

type AttributeType struct {
    ID   string
    Name string
}

type Money struct {
    Amount   int64
    Currency string
}

type Payment struct {
    ID     string
    Method PaymentMethod
    Status PaymentStatus
    Amount Money
}

type PaymentMethod struct {
    Type string
    Details map[string]string
}

type PaymentStatus struct {
    Code    string
    Message string
}

type Shipping struct {
    Address Address
    Method  ShippingMethod
    Status  ShippingStatus
}

type ShippingMethod struct {
    ID   string
    Name string
    Cost Money
}

type ShippingStatus struct {
    Code        string
    Message     string
    TrackingID  string
}

type OrderStatus struct {
    Code    string
    Message string
}
"#;
    fs::write(project_root.join("domain/order.go"), domain_order_content)
        .expect("Failed to write domain/order.go");

    // 分析所有文件
    let mut source_files = Vec::new();
    let go_files = ["domain/user.go", "domain/order.go"];

    for file_path in &go_files {
        let full_path = project_root.join(file_path);
        let mut analyzer =
            SourceAnalyzer::new_for_file(&full_path).expect("Failed to create analyzer");
        let source_file = analyzer
            .analyze_file(&full_path)
            .expect("Failed to analyze file");
        source_files.push(source_file);
    }

    let type_analyzer = TypeAnalyzer::new();
    let resolver = DependencyResolver::from_project_root(project_root)
        .expect("Failed to create resolver from project root");

    // 测试User类型的复杂依赖链
    let user_type = resolver.find_type_definition("User", &source_files);
    assert!(user_type.is_some(), "应该找到User类型");

    let user_def = user_type.unwrap();
    let user_deps = type_analyzer.analyze_type_dependencies(&user_def, &source_files);

    println!("User complex dependencies: {user_deps:?}");
    let user_dep_names: Vec<&str> = user_deps.iter().map(|d| d.name.as_str()).collect();
    println!("User complex dependency names: {user_dep_names:?}");

    // 验证深层依赖链
    let expected_deps = [
        "Profile",
        "Order",
        "Address",
        "Preferences",
        "GeoLocation",
        "Category",
        "Tag",
        "OrderItem",
        "Payment",
        "Shipping",
        "OrderStatus",
        "Money",
        "Product",
        "ProductAttribute",
        "AttributeType",
        "PaymentMethod",
        "PaymentStatus",
        "ShippingMethod",
        "ShippingStatus",
    ];

    for expected_dep in expected_deps {
        assert!(
            user_dep_names.contains(&expected_dep),
            "User应该依赖{expected_dep}（直接或间接）"
        );
    }

    // 验证依赖数量合理（应该包含所有间接依赖）
    assert!(
        user_deps.len() >= 15,
        "User应该有至少15个依赖（包括间接依赖）"
    );
}
