//! 测试数据集模块
//!
//! 提供各种 Go 语言结构的测试示例

use std::path::PathBuf;
use tempfile::TempDir;

/// 测试数据集结构
pub struct TestDataSet {
    pub temp_dir: TempDir,
    pub files: Vec<TestFile>,
}

/// 测试文件信息
pub struct TestFile {
    pub path: PathBuf,
    pub content: String,
    pub description: String,
}

impl TestDataSet {
    /// 创建包含各种 Go 语言结构的测试数据集
    pub fn create_comprehensive_go_dataset() -> std::io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let mut files = Vec::new();

        // 1. 基础结构体和方法
        files.push(TestFile {
            path: temp_dir.path().join("basic_struct.go"),
            content: r#"package main

import (
    "fmt"
    "time"
)

// User 表示系统用户
type User struct {
    ID        int       `json:"id"`
    Name      string    `json:"name"`
    Email     string    `json:"email"`
    CreatedAt time.Time `json:"created_at"`
}

// NewUser 创建新用户
func NewUser(name, email string) *User {
    return &User{
        Name:      name,
        Email:     email,
        CreatedAt: time.Now(),
    }
}

// GetDisplayName 获取显示名称
func (u *User) GetDisplayName() string {
    if u.Name != "" {
        return u.Name
    }
    return u.Email
}

// Validate 验证用户数据
func (u *User) Validate() error {
    if u.Name == "" {
        return fmt.Errorf("name cannot be empty")
    }
    if u.Email == "" {
        return fmt.Errorf("email cannot be empty")
    }
    return nil
}
"#
            .to_string(),
            description: "基础结构体定义和方法".to_string(),
        });

        // 2. 接口和实现
        files.push(TestFile {
            path: temp_dir.path().join("interface_impl.go"),
            content: r#"package main

import "fmt"

// Storage 存储接口
type Storage interface {
    Save(key string, value []byte) error
    Load(key string) ([]byte, error)
    Delete(key string) error
}

// MemoryStorage 内存存储实现
type MemoryStorage struct {
    data map[string][]byte
}

// NewMemoryStorage 创建内存存储
func NewMemoryStorage() *MemoryStorage {
    return &MemoryStorage{
        data: make(map[string][]byte),
    }
}

// Save 保存数据
func (m *MemoryStorage) Save(key string, value []byte) error {
    if key == "" {
        return fmt.Errorf("key cannot be empty")
    }
    m.data[key] = value
    return nil
}

// Load 加载数据
func (m *MemoryStorage) Load(key string) ([]byte, error) {
    value, exists := m.data[key]
    if !exists {
        return nil, fmt.Errorf("key not found: %s", key)
    }
    return value, nil
}

// Delete 删除数据
func (m *MemoryStorage) Delete(key string) error {
    delete(m.data, key)
    return nil
}

// StorageManager 存储管理器
type StorageManager struct {
    storage Storage
}

// NewStorageManager 创建存储管理器
func NewStorageManager(storage Storage) *StorageManager {
    return &StorageManager{storage: storage}
}

// ProcessData 处理数据
func (sm *StorageManager) ProcessData(key string, data []byte) error {
    return sm.storage.Save(key, data)
}
"#
            .to_string(),
            description: "接口定义和实现".to_string(),
        });

        // 3. 复杂的嵌套结构和泛型
        files.push(TestFile {
            path: temp_dir.path().join("complex_types.go"),
            content: r#"package main

import (
    "encoding/json"
    "fmt"
    "sync"
)

// Result 泛型结果类型
type Result[T any] struct {
    Value T
    Error error
}

// NewResult 创建结果
func NewResult[T any](value T, err error) Result[T] {
    return Result[T]{Value: value, Error: err}
}

// IsOk 检查是否成功
func (r Result[T]) IsOk() bool {
    return r.Error == nil
}

// Config 配置结构
type Config struct {
    Database struct {
        Host     string `json:"host"`
        Port     int    `json:"port"`
        Username string `json:"username"`
        Password string `json:"password"`
    } `json:"database"`
    Redis struct {
        Addr     string `json:"addr"`
        Password string `json:"password"`
        DB       int    `json:"db"`
    } `json:"redis"`
    Features map[string]bool `json:"features"`
}

// ConfigManager 配置管理器
type ConfigManager struct {
    config *Config
    mutex  sync.RWMutex
}

// NewConfigManager 创建配置管理器
func NewConfigManager() *ConfigManager {
    return &ConfigManager{
        config: &Config{
            Features: make(map[string]bool),
        },
    }
}

// LoadFromJSON 从 JSON 加载配置
func (cm *ConfigManager) LoadFromJSON(data []byte) error {
    cm.mutex.Lock()
    defer cm.mutex.Unlock()
    
    return json.Unmarshal(data, cm.config)
}

// GetFeature 获取功能开关
func (cm *ConfigManager) GetFeature(name string) bool {
    cm.mutex.RLock()
    defer cm.mutex.RUnlock()
    
    return cm.config.Features[name]
}

// SetFeature 设置功能开关
func (cm *ConfigManager) SetFeature(name string, enabled bool) {
    cm.mutex.Lock()
    defer cm.mutex.Unlock()
    
    cm.config.Features[name] = enabled
}
"#
            .to_string(),
            description: "复杂嵌套结构和泛型".to_string(),
        });

        // 4. 错误处理和自定义错误类型
        files.push(TestFile {
            path: temp_dir.path().join("error_handling.go"),
            content: r#"package main

import (
    "fmt"
    "strings"
)

// ErrorCode 错误代码类型
type ErrorCode int

const (
    ErrCodeUnknown ErrorCode = iota
    ErrCodeValidation
    ErrCodeNotFound
    ErrCodePermission
    ErrCodeInternal
)

// String 实现 Stringer 接口
func (e ErrorCode) String() string {
    switch e {
    case ErrCodeValidation:
        return "VALIDATION_ERROR"
    case ErrCodeNotFound:
        return "NOT_FOUND"
    case ErrCodePermission:
        return "PERMISSION_DENIED"
    case ErrCodeInternal:
        return "INTERNAL_ERROR"
    default:
        return "UNKNOWN_ERROR"
    }
}

// AppError 应用错误类型
type AppError struct {
    Code    ErrorCode
    Message string
    Cause   error
}

// Error 实现 error 接口
func (e *AppError) Error() string {
    if e.Cause != nil {
        return fmt.Sprintf("[%s] %s: %v", e.Code, e.Message, e.Cause)
    }
    return fmt.Sprintf("[%s] %s", e.Code, e.Message)
}

// Unwrap 支持错误链
func (e *AppError) Unwrap() error {
    return e.Cause
}

// NewAppError 创建应用错误
func NewAppError(code ErrorCode, message string, cause error) *AppError {
    return &AppError{
        Code:    code,
        Message: message,
        Cause:   cause,
    }
}

// ValidationError 创建验证错误
func ValidationError(message string) *AppError {
    return NewAppError(ErrCodeValidation, message, nil)
}

// NotFoundError 创建未找到错误
func NotFoundError(resource string) *AppError {
    return NewAppError(ErrCodeNotFound, fmt.Sprintf("%s not found", resource), nil)
}

// Validator 验证器接口
type Validator interface {
    Validate() error
}

// ValidateStruct 验证结构体
func ValidateStruct(v Validator) error {
    if err := v.Validate(); err != nil {
        return ValidationError(err.Error())
    }
    return nil
}

// EmailValidator 邮箱验证器
type EmailValidator struct {
    Email string
}

// Validate 验证邮箱
func (ev EmailValidator) Validate() error {
    if ev.Email == "" {
        return fmt.Errorf("email cannot be empty")
    }
    if !strings.Contains(ev.Email, "@") {
        return fmt.Errorf("invalid email format")
    }
    return nil
}
"#
            .to_string(),
            description: "错误处理和自定义错误类型".to_string(),
        });

        // 5. 并发和 goroutine
        files.push(TestFile {
            path: temp_dir.path().join("concurrency.go"),
            content: r#"package main

import (
    "context"
    "fmt"
    "sync"
    "time"
)

// Worker 工作器接口
type Worker interface {
    Process(ctx context.Context, data interface{}) error
}

// WorkerPool 工作池
type WorkerPool struct {
    workers   []Worker
    taskChan  chan Task
    resultChan chan Result
    wg        sync.WaitGroup
    ctx       context.Context
    cancel    context.CancelFunc
}

// Task 任务
type Task struct {
    ID   string
    Data interface{}
}

// Result 结果
type Result struct {
    TaskID string
    Error  error
}

// NewWorkerPool 创建工作池
func NewWorkerPool(workers []Worker, bufferSize int) *WorkerPool {
    ctx, cancel := context.WithCancel(context.Background())
    return &WorkerPool{
        workers:    workers,
        taskChan:   make(chan Task, bufferSize),
        resultChan: make(chan Result, bufferSize),
        ctx:        ctx,
        cancel:     cancel,
    }
}

// Start 启动工作池
func (wp *WorkerPool) Start() {
    for i, worker := range wp.workers {
        wp.wg.Add(1)
        go wp.workerRoutine(fmt.Sprintf("worker-%d", i), worker)
    }
}

// Stop 停止工作池
func (wp *WorkerPool) Stop() {
    wp.cancel()
    close(wp.taskChan)
    wp.wg.Wait()
    close(wp.resultChan)
}

// Submit 提交任务
func (wp *WorkerPool) Submit(task Task) {
    select {
    case wp.taskChan <- task:
    case <-wp.ctx.Done():
    }
}

// Results 获取结果通道
func (wp *WorkerPool) Results() <-chan Result {
    return wp.resultChan
}

// workerRoutine 工作器协程
func (wp *WorkerPool) workerRoutine(name string, worker Worker) {
    defer wp.wg.Done()
    
    for {
        select {
        case task, ok := <-wp.taskChan:
            if !ok {
                return
            }
            
            err := worker.Process(wp.ctx, task.Data)
            wp.resultChan <- Result{
                TaskID: task.ID,
                Error:  err,
            }
            
        case <-wp.ctx.Done():
            return
        }
    }
}

// SimpleWorker 简单工作器实现
type SimpleWorker struct {
    name string
}

// NewSimpleWorker 创建简单工作器
func NewSimpleWorker(name string) *SimpleWorker {
    return &SimpleWorker{name: name}
}

// Process 处理任务
func (sw *SimpleWorker) Process(ctx context.Context, data interface{}) error {
    // 模拟处理时间
    select {
    case <-time.After(100 * time.Millisecond):
        return nil
    case <-ctx.Done():
        return ctx.Err()
    }
}
"#
            .to_string(),
            description: "并发和 goroutine 处理".to_string(),
        });

        // 6. 包级别函数和常量
        files.push(TestFile {
            path: temp_dir.path().join("package_level.go"),
            content: r#"package main

import (
    "fmt"
    "os"
    "strconv"
)

// 包级别常量
const (
    DefaultTimeout = 30
    MaxRetries     = 3
    BufferSize     = 1024
)

// 包级别变量
var (
    GlobalConfig *Config
    Logger       *SimpleLogger
)

// 包级别类型别名
type (
    ID       = int64
    UserID   = ID
    OrderID  = ID
    Callback = func(error)
)

// init 初始化函数
func init() {
    GlobalConfig = &Config{}
    Logger = NewSimpleLogger()
}

// GetEnvInt 获取环境变量整数值
func GetEnvInt(key string, defaultValue int) int {
    if value := os.Getenv(key); value != "" {
        if intValue, err := strconv.Atoi(value); err == nil {
            return intValue
        }
    }
    return defaultValue
}

// GetEnvString 获取环境变量字符串值
func GetEnvString(key, defaultValue string) string {
    if value := os.Getenv(key); value != "" {
        return value
    }
    return defaultValue
}

// Retry 重试函数
func Retry(fn func() error, maxRetries int) error {
    var lastErr error
    for i := 0; i < maxRetries; i++ {
        if err := fn(); err == nil {
            return nil
        } else {
            lastErr = err
        }
    }
    return fmt.Errorf("failed after %d retries: %w", maxRetries, lastErr)
}

// SimpleLogger 简单日志器
type SimpleLogger struct {
    prefix string
}

// NewSimpleLogger 创建简单日志器
func NewSimpleLogger() *SimpleLogger {
    return &SimpleLogger{prefix: "[LOG]"}
}

// Info 记录信息日志
func (l *SimpleLogger) Info(msg string) {
    fmt.Printf("%s INFO: %s\n", l.prefix, msg)
}

// Error 记录错误日志
func (l *SimpleLogger) Error(msg string) {
    fmt.Printf("%s ERROR: %s\n", l.prefix, msg)
}

// WithPrefix 设置前缀
func (l *SimpleLogger) WithPrefix(prefix string) *SimpleLogger {
    return &SimpleLogger{prefix: prefix}
}
"#
            .to_string(),
            description: "包级别函数、常量和变量".to_string(),
        });

        // 写入所有文件
        for file in &files {
            std::fs::write(&file.path, &file.content)?;
        }

        Ok(TestDataSet { temp_dir, files })
    }

    /// 创建模拟 Git 仓库的测试数据
    pub fn create_git_repo_dataset() -> std::io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let mut files = Vec::new();

        // 创建一个模拟的 Go 项目结构
        let project_structure = [
            (
                "main.go",
                r#"package main

import (
    "fmt"
    "log"
    "./internal/user"
    "./internal/storage"
)

func main() {
    userService := user.NewService()
    storageService := storage.NewService()
    
    if err := userService.CreateUser("john", "john@example.com"); err != nil {
        log.Fatal(err)
    }
    
    fmt.Println("Application started successfully")
}
"#,
            ),
            (
                "internal/user/service.go",
                r#"package user

import "fmt"

type Service struct {
    users map[string]*User
}

type User struct {
    Name  string
    Email string
}

func NewService() *Service {
    return &Service{
        users: make(map[string]*User),
    }
}

func (s *Service) CreateUser(name, email string) error {
    if name == "" {
        return fmt.Errorf("name cannot be empty")
    }
    
    s.users[name] = &User{
        Name:  name,
        Email: email,
    }
    
    return nil
}

func (s *Service) GetUser(name string) (*User, error) {
    user, exists := s.users[name]
    if !exists {
        return nil, fmt.Errorf("user not found: %s", name)
    }
    return user, nil
}
"#,
            ),
            (
                "internal/storage/service.go",
                r#"package storage

import (
    "encoding/json"
    "fmt"
    "os"
)

type Service struct {
    filePath string
}

func NewService() *Service {
    return &Service{
        filePath: "data.json",
    }
}

func (s *Service) Save(key string, data interface{}) error {
    file, err := os.OpenFile(s.filePath, os.O_CREATE|os.O_WRONLY, 0644)
    if err != nil {
        return fmt.Errorf("failed to open file: %w", err)
    }
    defer file.Close()
    
    encoder := json.NewEncoder(file)
    return encoder.Encode(map[string]interface{}{key: data})
}

func (s *Service) Load(key string) (interface{}, error) {
    file, err := os.Open(s.filePath)
    if err != nil {
        return nil, fmt.Errorf("failed to open file: %w", err)
    }
    defer file.Close()
    
    var data map[string]interface{}
    decoder := json.NewDecoder(file)
    if err := decoder.Decode(&data); err != nil {
        return nil, fmt.Errorf("failed to decode data: %w", err)
    }
    
    value, exists := data[key]
    if !exists {
        return nil, fmt.Errorf("key not found: %s", key)
    }
    
    return value, nil
}
"#,
            ),
        ];

        for (relative_path, content) in project_structure.iter() {
            let file_path = temp_dir.path().join(relative_path);

            // 创建目录（如果需要）
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&file_path, content)?;

            files.push(TestFile {
                path: file_path,
                content: content.to_string(),
                description: format!("Git 仓库文件: {relative_path}"),
            });
        }

        Ok(TestDataSet { temp_dir, files })
    }

    /// 获取所有测试文件路径
    pub fn get_file_paths(&self) -> Vec<PathBuf> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }

    /// 根据描述查找文件
    pub fn find_file_by_description(&self, description: &str) -> Option<&TestFile> {
        self.files
            .iter()
            .find(|f| f.description.contains(description))
    }
}
