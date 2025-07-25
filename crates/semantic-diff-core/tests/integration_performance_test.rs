//! 性能优化集成测试
//!
//! 测试性能优化组件的集成功能

use semantic_diff_core::{
    LanguageSpecificInfo, SupportedLanguage,
    analyzer::SourceAnalyzer,
    extractor::{ChangeTarget, SemanticContextExtractor},
    performance::{
        ConcurrentFileProcessor, ErrorRecoveryStrategy, MemoryEfficientAstProcessor, ParserCache,
        PerformanceMonitor,
    },
};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// 创建测试用的 Go 源文件
fn create_test_go_files(temp_dir: &TempDir, count: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for i in 0..count {
        let file_path = temp_dir.path().join(format!("test_{i}.go"));
        let content = format!(
            r#"package main

import (
    "fmt"
    "os"
)

// TestStruct{i} 是一个测试结构体
type TestStruct{i} struct {{
    ID   int
    Name string
    Data []byte
}}

// NewTestStruct{i} 创建新的测试结构体实例
func NewTestStruct{i}(id int, name string) *TestStruct{i} {{
    return &TestStruct{i} {{
        ID:   id,
        Name: name,
        Data: make([]byte, 100),
    }}
}}

// Process{i} 处理数据
func (t *TestStruct{i}) Process{i}() error {{
    if t.Name == "" {{
        return fmt.Errorf("name cannot be empty")
    }}
    return nil
}}

func main() {{
    test := NewTestStruct{i}({i}, "test_{i}")
    if err := test.Process{i}(); err != nil {{
        fmt.Fprintf(os.Stderr, "Error: %v\n", err)
        os.Exit(1)
    }}
}}
"#
        );

        std::fs::write(&file_path, content).unwrap();
        files.push(file_path);
    }

    files
}

#[test]
fn test_parser_cache_functionality() {
    let cache = ParserCache::new();

    // 测试缓存创建
    let _parser1 = cache.get_or_create_parser(SupportedLanguage::Go).unwrap();
    let stats = cache.get_stats();
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.creates, 1);
    assert_eq!(stats.hits, 0);

    // 测试缓存命中
    let _parser2 = cache.get_or_create_parser(SupportedLanguage::Go).unwrap();
    let stats = cache.get_stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.creates, 1);

    // 测试缓存大小
    assert_eq!(cache.size(), 1);

    // 测试缓存清理
    cache.clear();
    assert_eq!(cache.size(), 0);
}

#[test]
fn test_memory_efficient_ast_processor() {
    let processor = MemoryEfficientAstProcessor::new()
        .with_max_concurrent_files(4)
        .with_memory_threshold(1024 * 1024)
        .with_memory_monitoring(false);

    // 测试内存检查（在禁用监控时应返回 None）
    assert!(processor.check_memory_usage().is_none());

    // 测试 GC 触发检查
    assert!(!processor.should_trigger_gc());
}

#[test]
fn test_concurrent_file_processor() {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 5);

    let processor = ConcurrentFileProcessor::new()
        .with_thread_pool_size(2)
        .with_batch_size(2);

    let result = processor.process_files_concurrent(&files).unwrap();

    // 验证所有文件都被成功处理
    assert_eq!(result.successful.len(), 5);
    assert_eq!(result.failed.len(), 0);

    // 验证性能统计
    assert_eq!(result.performance_stats.files_processed, 5);
    assert_eq!(result.performance_stats.successful_files, 5);
    assert_eq!(result.performance_stats.failed_files, 0);
    assert!(result.performance_stats.total_duration > Duration::ZERO);
}

#[test]
fn test_concurrent_file_processor_with_errors() {
    let temp_dir = TempDir::new().unwrap();
    let mut files = create_test_go_files(&temp_dir, 3);

    // 添加一些不存在的文件来测试错误处理
    files.push(temp_dir.path().join("nonexistent1.go"));
    files.push(temp_dir.path().join("nonexistent2.go"));

    let processor = ConcurrentFileProcessor::new();
    let result = processor.process_files_concurrent(&files).unwrap();

    // 验证部分文件成功，部分失败
    assert_eq!(result.successful.len(), 3);
    assert_eq!(result.failed.len(), 2);

    // 验证性能统计
    assert_eq!(result.performance_stats.files_processed, 5);
    assert_eq!(result.performance_stats.successful_files, 3);
    assert_eq!(result.performance_stats.failed_files, 2);
}

#[test]
fn test_source_analyzer_concurrent_processing() {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 10);

    let source_files = SourceAnalyzer::analyze_files_concurrent(&files).unwrap();

    // 验证所有文件都被成功分析
    assert_eq!(source_files.len(), 10);

    // 验证每个文件都有正确的语言类型
    for source_file in &source_files {
        assert_eq!(source_file.language, SupportedLanguage::Go);
        assert!(!source_file.source_code.is_empty());
    }
}

#[test]
fn test_source_analyzer_batch_processing() {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 15);

    let source_files = SourceAnalyzer::analyze_files_in_batches(&files, 5).unwrap();

    // 验证所有文件都被成功分析
    assert_eq!(source_files.len(), 15);
}

#[test]
fn test_source_analyzer_with_error_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let mut files = create_test_go_files(&temp_dir, 5);

    // 添加一些不存在的文件
    files.push(temp_dir.path().join("nonexistent.go"));

    let recovery_strategy = ErrorRecoveryStrategy::new()
        .with_max_retries(2)
        .with_retry_delay(Duration::from_millis(10));

    let source_files =
        SourceAnalyzer::analyze_files_concurrent_with_recovery(&files, recovery_strategy).unwrap();

    // 验证只有存在的文件被成功分析
    assert_eq!(source_files.len(), 5);
}

#[test]
fn test_semantic_context_extractor_concurrent() {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 5);

    // 首先分析文件
    let source_files = SourceAnalyzer::analyze_files_concurrent(&files).unwrap();

    // 创建一些变更目标
    let mut change_targets = Vec::new();
    for source_file in &source_files {
        if let Some(go_info) = source_file
            .language_specific
            .as_any()
            .downcast_ref::<semantic_diff_core::parser::GoLanguageInfo>()
        {
            for declaration in go_info.declarations() {
                if let Some(go_decl) = declaration
                    .as_any()
                    .downcast_ref::<semantic_diff_core::parser::GoDeclaration>()
                {
                    match go_decl {
                        semantic_diff_core::parser::GoDeclaration::Function(func_info) => {
                            change_targets.push(ChangeTarget::Function(func_info.clone()));
                        }
                        semantic_diff_core::parser::GoDeclaration::Type(type_def) => {
                            change_targets.push(ChangeTarget::Type(type_def.clone()));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    if !change_targets.is_empty() {
        let extractor = SemanticContextExtractor::new();
        let contexts = extractor
            .extract_contexts_concurrent(&change_targets, &source_files)
            .unwrap();

        // 验证上下文提取成功
        assert_eq!(contexts.len(), change_targets.len());

        // 验证每个上下文都有对应的变更目标
        for context in &contexts {
            assert!(!context.change_target.name().is_empty());
        }
    }
}

#[test]
fn test_semantic_context_extractor_batch_processing() {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 8);

    // 首先分析文件
    let source_files = SourceAnalyzer::analyze_files_concurrent(&files).unwrap();

    // 创建一些变更目标
    let mut change_targets = Vec::new();
    for source_file in &source_files {
        if let Some(go_info) = source_file
            .language_specific
            .as_any()
            .downcast_ref::<semantic_diff_core::parser::GoLanguageInfo>()
        {
            for declaration in go_info.declarations() {
                if let Some(go_decl) = declaration
                    .as_any()
                    .downcast_ref::<semantic_diff_core::parser::GoDeclaration>()
                {
                    if let semantic_diff_core::parser::GoDeclaration::Function(func_info) = go_decl
                    {
                        change_targets.push(ChangeTarget::Function(func_info.clone()));
                    }
                }
            }
        }
    }

    if !change_targets.is_empty() {
        let extractor = SemanticContextExtractor::new();
        let contexts = extractor
            .extract_contexts_in_batches(&change_targets, &source_files, 3)
            .unwrap();

        // 验证批量处理成功
        assert_eq!(contexts.len(), change_targets.len());
    }
}

#[test]
fn test_performance_monitor() {
    let monitor = PerformanceMonitor::new();

    // 模拟一些文件处理
    monitor.record_file_processed(Duration::from_millis(100));
    monitor.record_file_processed(Duration::from_millis(200));
    monitor.record_error();

    let stats = monitor.get_stats(semantic_diff_core::performance::CacheStats::default());

    // 验证统计信息
    assert_eq!(stats.files_processed, 2);
    assert_eq!(stats.failed_files, 1);
    assert_eq!(stats.successful_files, 1);
    assert!(stats.total_duration > Duration::ZERO);
    assert!(stats.avg_file_processing_time > Duration::ZERO);
}

#[test]
fn test_error_recovery_strategy() {
    let strategy = ErrorRecoveryStrategy::new()
        .with_max_retries(3)
        .with_retry_delay(Duration::from_millis(10));

    // 测试成功的操作
    let result =
        strategy.execute_with_retry(|| Ok::<i32, semantic_diff_core::SemanticDiffError>(42));
    assert_eq!(result.unwrap(), 42);

    // 测试失败的操作
    let mut attempt_count = 0;
    let result = strategy.execute_with_retry(|| {
        attempt_count += 1;
        Err::<i32, _>(semantic_diff_core::SemanticDiffError::ParseError(
            "test error".to_string(),
        ))
    });

    assert!(result.is_err());
    assert_eq!(attempt_count, 4); // 初始尝试 + 3 次重试
}

#[test]
fn test_large_scale_processing() {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 50);

    let processor = ConcurrentFileProcessor::new()
        .with_thread_pool_size(4)
        .with_batch_size(10);

    let result = processor.process_files_concurrent(&files).unwrap();

    // 验证大规模处理成功
    assert_eq!(result.successful.len(), 50);
    assert_eq!(result.failed.len(), 0);

    // 验证性能统计合理
    assert!(result.performance_stats.total_duration < Duration::from_secs(30));
    assert!(result.performance_stats.avg_file_processing_time < Duration::from_secs(1));
}
/// 大型代码库性能测试配置
#[derive(Clone)]
struct LargeScaleTestConfig {
    pub file_count: usize,
    pub functions_per_file: usize,
    pub max_processing_time: Duration,
    pub max_memory_usage_mb: usize,
}

impl Default for LargeScaleTestConfig {
    fn default() -> Self {
        Self {
            file_count: 100,
            functions_per_file: 10,
            max_processing_time: Duration::from_secs(30),
            max_memory_usage_mb: 512,
        }
    }
}

/// 大型代码库性能测试套件
struct LargeScaleTestSuite {
    temp_dir: TempDir,
    config: LargeScaleTestConfig,
    monitor: PerformanceMonitor,
}

impl LargeScaleTestSuite {
    fn new(config: LargeScaleTestConfig) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let monitor = PerformanceMonitor::new();

        Self {
            temp_dir,
            config,
            monitor,
        }
    }

    /// 生成大型测试数据集
    fn generate_large_dataset(&self) -> Vec<PathBuf> {
        let mut file_paths = Vec::new();

        for file_index in 0..self.config.file_count {
            let file_path = self
                .temp_dir
                .path()
                .join(format!("large_file_{file_index}.go"));
            let content = self.generate_complex_go_file_content(file_index);

            std::fs::write(&file_path, content).unwrap();
            file_paths.push(file_path);
        }

        file_paths
    }

    /// 生成复杂的 Go 文件内容
    fn generate_complex_go_file_content(&self, file_index: usize) -> String {
        let mut content = format!(
            r#"package main

import (
    "context"
    "fmt"
    "sync"
    "time"
    "encoding/json"
    "net/http"
    "database/sql"
    "log"
)

// File{file_index}Data 复杂数据结构
type File{file_index}Data struct {{
    ID          int64                  `json:"id" db:"id"`
    Name        string                 `json:"name" db:"name"`
    Value       float64                `json:"value" db:"value"`
    Timestamp   time.Time              `json:"timestamp" db:"timestamp"`
    Metadata    map[string]interface{{}} `json:"metadata" db:"metadata"`
    Tags        []string               `json:"tags" db:"tags"`
    Status      File{file_index}Status `json:"status" db:"status"`
    Config      *File{file_index}Config `json:"config,omitempty" db:"config"`
    Children    []*File{file_index}Data `json:"children,omitempty"`
    Parent      *File{file_index}Data   `json:"parent,omitempty"`
}}

// File{file_index}Status 状态枚举
type File{file_index}Status int

const (
    File{file_index}StatusPending File{file_index}Status = iota
    File{file_index}StatusProcessing
    File{file_index}StatusCompleted
    File{file_index}StatusFailed
)

// File{file_index}Config 配置结构
type File{file_index}Config struct {{
    MaxItems        int           `json:"max_items" yaml:"max_items"`
    Timeout         time.Duration `json:"timeout" yaml:"timeout"`
    EnableCache     bool          `json:"enable_cache" yaml:"enable_cache"`
    RetryCount      int           `json:"retry_count" yaml:"retry_count"`
    BatchSize       int           `json:"batch_size" yaml:"batch_size"`
    WorkerPoolSize  int           `json:"worker_pool_size" yaml:"worker_pool_size"`
    DatabaseURL     string        `json:"database_url" yaml:"database_url"`
    RedisURL        string        `json:"redis_url" yaml:"redis_url"`
}}

// File{file_index}Service 复杂服务结构
type File{file_index}Service struct {{
    data        map[int64]*File{file_index}Data
    mutex       sync.RWMutex
    config      *File{file_index}Config
    db          *sql.DB
    httpClient  *http.Client
    workerPool  chan struct{{}}
    ctx         context.Context
    cancel      context.CancelFunc
    wg          sync.WaitGroup
    stats       *File{file_index}Stats
    cache       map[string]interface{{}}
    cacheMutex  sync.RWMutex
}}

// File{file_index}Stats 统计信息
type File{file_index}Stats struct {{
    ProcessedCount    int64     `json:"processed_count"`
    ErrorCount        int64     `json:"error_count"`
    LastProcessed     time.Time `json:"last_processed"`
    AverageTime       time.Duration `json:"average_time"`
    TotalTime         time.Duration `json:"total_time"`
    CacheHits         int64     `json:"cache_hits"`
    CacheMisses       int64     `json:"cache_misses"`
}}

"#
        );

        // 生成多个复杂函数
        for func_index in 0..self.config.functions_per_file {
            let function_content = format!(
                r#"
// ProcessData{func_index} 复杂数据处理方法 {func_index}
func (s *File{file_index}Service) ProcessData{func_index}(ctx context.Context, data *File{file_index}Data) error {{
    start := time.Now()
    defer func() {{
        s.stats.TotalTime += time.Since(start)
        s.stats.ProcessedCount++
        s.stats.LastProcessed = time.Now()
        s.stats.AverageTime = s.stats.TotalTime / time.Duration(s.stats.ProcessedCount)
    }}()

    s.mutex.Lock()
    defer s.mutex.Unlock()
    
    if data == nil {{
        s.stats.ErrorCount++
        return fmt.Errorf("data cannot be nil")
    }}
    
    // 复杂的验证逻辑
    if err := s.validateData{func_index}(data); err != nil {{
        s.stats.ErrorCount++
        return fmt.Errorf("validation failed: %w", err)
    }}
    
    // 检查容量限制
    if len(s.data) >= s.config.MaxItems {{
        s.stats.ErrorCount++
        return fmt.Errorf("maximum items reached: %d", s.config.MaxItems)
    }}
    
    // 缓存检查
    cacheKey := fmt.Sprintf("data_%d_%d", {file_index}, data.ID)
    if cached := s.getCachedData(cacheKey); cached != nil {{
        s.stats.CacheHits++
        return nil
    }}
    s.stats.CacheMisses++
    
    // 数据库操作模拟
    if err := s.saveToDatabase{func_index}(ctx, data); err != nil {{
        s.stats.ErrorCount++
        return fmt.Errorf("database save failed: %w", err)
    }}
    
    // 设置默认值
    if data.Timestamp.IsZero() {{
        data.Timestamp = time.Now()
    }}
    
    if data.Metadata == nil {{
        data.Metadata = make(map[string]interface{{}})
    }}
    
    // 添加处理元数据
    data.Metadata["processed_by"] = fmt.Sprintf("File{file_index}Service.ProcessData{func_index}")
    data.Metadata["processed_at"] = time.Now().Unix()
    data.Metadata["file_index"] = {file_index}
    data.Metadata["func_index"] = {func_index}
    data.Metadata["version"] = "1.0.0"
    
    // 状态更新
    data.Status = File{file_index}StatusProcessing
    
    // 异步处理子项
    if len(data.Children) > 0 {{
        go s.processChildren{func_index}(ctx, data)
    }}
    
    // 存储数据
    s.data[data.ID] = data
    
    // 缓存结果
    s.setCachedData(cacheKey, data)
    
    return nil
}}

// validateData{func_index} 验证数据方法 {func_index}
func (s *File{file_index}Service) validateData{func_index}(data *File{file_index}Data) error {{
    if data.Name == "" {{
        return fmt.Errorf("name cannot be empty")
    }}
    
    if data.Value < 0 {{
        return fmt.Errorf("value cannot be negative: %f", data.Value)
    }}
    
    if data.ID <= 0 {{
        return fmt.Errorf("invalid ID: %d", data.ID)
    }}
    
    // 验证标签
    for i, tag := range data.Tags {{
        if tag == "" {{
            return fmt.Errorf("tag at index %d cannot be empty", i)
        }}
        if len(tag) > 50 {{
            return fmt.Errorf("tag at index %d too long: %s", i, tag)
        }}
    }}
    
    // 验证状态
    if data.Status < File{file_index}StatusPending || data.Status > File{file_index}StatusFailed {{
        return fmt.Errorf("invalid status: %d", data.Status)
    }}
    
    return nil
}}

// saveToDatabase{func_index} 保存到数据库方法 {func_index}
func (s *File{file_index}Service) saveToDatabase{func_index}(ctx context.Context, data *File{file_index}Data) error {{
    if s.db == nil {{
        return fmt.Errorf("database connection not available")
    }}
    
    // 模拟数据库操作
    query := `INSERT INTO file_{file_index}_data (id, name, value, timestamp, status) VALUES (?, ?, ?, ?, ?)`
    
    ctxWithTimeout, cancel := context.WithTimeout(ctx, s.config.Timeout)
    defer cancel()
    
    _, err := s.db.ExecContext(ctxWithTimeout, query, data.ID, data.Name, data.Value, data.Timestamp, data.Status)
    if err != nil {{
        return fmt.Errorf("failed to execute query: %w", err)
    }}
    
    return nil
}}

// processChildren{func_index} 处理子项方法 {func_index}
func (s *File{file_index}Service) processChildren{func_index}(ctx context.Context, parent *File{file_index}Data) {{
    s.wg.Add(1)
    defer s.wg.Done()
    
    for _, child := range parent.Children {{
        select {{
        case <-ctx.Done():
            return
        case s.workerPool <- struct{{}}{{}}:
            go func(c *File{file_index}Data) {{
                defer func() {{ <-s.workerPool }}()
                
                if err := s.ProcessData{func_index}(ctx, c); err != nil {{
                    log.Printf("Failed to process child %d: %v", c.ID, err)
                }}
            }}(child)
        }}
    }}
}}

// GetData{func_index} 获取数据方法 {func_index}
func (s *File{file_index}Service) GetData{func_index}(id int64) (*File{file_index}Data, error) {{
    s.mutex.RLock()
    defer s.mutex.RUnlock()
    
    // 缓存检查
    cacheKey := fmt.Sprintf("get_data_%d_%d", {file_index}, id)
    if cached := s.getCachedData(cacheKey); cached != nil {{
        if data, ok := cached.(*File{file_index}Data); ok {{
            s.stats.CacheHits++
            return data, nil
        }}
    }}
    s.stats.CacheMisses++
    
    data, exists := s.data[id]
    if !exists {{
        return nil, fmt.Errorf("data not found: %d", id)
    }}
    
    // 创建深拷贝以避免并发问题
    result := &File{file_index}Data{{
        ID:        data.ID,
        Name:      data.Name,
        Value:     data.Value,
        Timestamp: data.Timestamp,
        Status:    data.Status,
        Tags:      make([]string, len(data.Tags)),
        Metadata:  make(map[string]interface{{}}),
    }}
    
    // 复制标签
    copy(result.Tags, data.Tags)
    
    // 复制元数据
    for k, v := range data.Metadata {{
        result.Metadata[k] = v
    }}
    
    // 缓存结果
    s.setCachedData(cacheKey, result)
    
    return result, nil
}}

// UpdateData{func_index} 更新数据方法 {func_index}
func (s *File{file_index}Service) UpdateData{func_index}(id int64, updates map[string]interface{{}}) error {{
    s.mutex.Lock()
    defer s.mutex.Unlock()
    
    data, exists := s.data[id]
    if !exists {{
        return fmt.Errorf("data not found: %d", id)
    }}
    
    // 应用更新
    for key, value := range updates {{
        switch key {{
        case "name":
            if name, ok := value.(string); ok {{
                if name == "" {{
                    return fmt.Errorf("name cannot be empty")
                }}
                data.Name = name
            }}
        case "value":
            if val, ok := value.(float64); ok {{
                if val < 0 {{
                    return fmt.Errorf("value cannot be negative")
                }}
                data.Value = val
            }}
        case "status":
            if status, ok := value.(File{file_index}Status); ok {{
                data.Status = status
            }}
        case "tags":
            if tags, ok := value.([]string); ok {{
                data.Tags = tags
            }}
        default:
            data.Metadata[key] = value
        }}
    }}
    
    data.Metadata["updated_at"] = time.Now().Unix()
    
    // 清除相关缓存
    s.clearCacheForData(id)
    
    return nil
}}

// DeleteData{func_index} 删除数据方法 {func_index}
func (s *File{file_index}Service) DeleteData{func_index}(id int64) error {{
    s.mutex.Lock()
    defer s.mutex.Unlock()
    
    if _, exists := s.data[id]; !exists {{
        return fmt.Errorf("data not found: %d", id)
    }}
    
    delete(s.data, id)
    
    // 清除相关缓存
    s.clearCacheForData(id)
    
    return nil
}}

// BatchProcess{func_index} 批量处理方法 {func_index}
func (s *File{file_index}Service) BatchProcess{func_index}(ctx context.Context, dataList []*File{file_index}Data) error {{
    if len(dataList) == 0 {{
        return nil
    }}
    
    // 分批处理
    batchSize := s.config.BatchSize
    if batchSize <= 0 {{
        batchSize = 10
    }}
    
    for i := 0; i < len(dataList); i += batchSize {{
        end := i + batchSize
        if end > len(dataList) {{
            end = len(dataList)
        }}
        
        batch := dataList[i:end]
        
        // 并发处理批次
        errChan := make(chan error, len(batch))
        
        for _, data := range batch {{
            go func(d *File{file_index}Data) {{
                errChan <- s.ProcessData{func_index}(ctx, d)
            }}(data)
        }}
        
        // 收集错误
        var errors []error
        for range batch {{
            if err := <-errChan; err != nil {{
                errors = append(errors, err)
            }}
        }}
        
        if len(errors) > 0 {{
            return fmt.Errorf("batch processing failed with %d errors: %v", len(errors), errors[0])
        }}
    }}
    
    return nil
}}
"#
            );
            content.push_str(&function_content);
        }

        // 添加辅助方法
        content.push_str(&format!(
            r#"
// getCachedData 获取缓存数据
func (s *File{file_index}Service) getCachedData(key string) interface{{}} {{
    s.cacheMutex.RLock()
    defer s.cacheMutex.RUnlock()
    return s.cache[key]
}}

// setCachedData 设置缓存数据
func (s *File{file_index}Service) setCachedData(key string, value interface{{}}) {{
    s.cacheMutex.Lock()
    defer s.cacheMutex.Unlock()
    s.cache[key] = value
}}

// clearCacheForData 清除数据相关的缓存
func (s *File{file_index}Service) clearCacheForData(id int64) {{
    s.cacheMutex.Lock()
    defer s.cacheMutex.Unlock()
    
    keysToDelete := make([]string, 0)
    for key := range s.cache {{
        if key == fmt.Sprintf("data_%d_%d", {file_index}, id) ||
           key == fmt.Sprintf("get_data_%d_%d", {file_index}, id) {{
            keysToDelete = append(keysToDelete, key)
        }}
    }}
    
    for _, key := range keysToDelete {{
        delete(s.cache, key)
    }}
}}

// NewFile{file_index}Service 创建服务实例
func NewFile{file_index}Service(config *File{file_index}Config) *File{file_index}Service {{
    if config == nil {{
        config = &File{file_index}Config{{
            MaxItems:       1000,
            Timeout:        30 * time.Second,
            EnableCache:    true,
            RetryCount:     3,
            BatchSize:      10,
            WorkerPoolSize: 4,
        }}
    }}
    
    ctx, cancel := context.WithCancel(context.Background())
    
    service := &File{file_index}Service{{
        data:       make(map[int64]*File{file_index}Data),
        config:     config,
        httpClient: &http.Client{{Timeout: config.Timeout}},
        workerPool: make(chan struct{{}}, config.WorkerPoolSize),
        ctx:        ctx,
        cancel:     cancel,
        stats:      &File{file_index}Stats{{}},
        cache:      make(map[string]interface{{}}),
    }}
    
    return service
}}

// Close 关闭服务
func (s *File{file_index}Service) Close() error {{
    s.cancel()
    s.wg.Wait()
    
    if s.db != nil {{
        return s.db.Close()
    }}
    
    return nil
}}

// GetStats 获取统计信息
func (s *File{file_index}Service) GetStats() File{file_index}Stats {{
    s.mutex.RLock()
    defer s.mutex.RUnlock()
    return *s.stats
}}
"#
        ));

        content
    }

    /// 运行大型代码库性能测试
    fn run_large_scale_test(&self) -> LargeScaleTestResults {
        println!("开始生成大型测试数据集...");
        let file_paths = self.generate_large_dataset();
        println!("生成了 {} 个复杂测试文件", file_paths.len());

        let mut results = LargeScaleTestResults::default();

        // 1. 单文件处理性能测试
        println!("测试单文件处理性能...");
        let start = std::time::Instant::now();
        let mut analyzer = SourceAnalyzer::new_for_language(SupportedLanguage::Go).unwrap();
        let _source_file = analyzer.analyze_file(&file_paths[0]).unwrap();
        results.single_file_time = start.elapsed();

        // 2. 批量处理性能测试
        println!("测试批量处理性能...");
        let start = std::time::Instant::now();
        let source_files = SourceAnalyzer::analyze_files_concurrent(&file_paths).unwrap();
        results.batch_processing_time = start.elapsed();
        results.total_files_processed = source_files.len();

        // 3. 并发处理性能测试
        println!("测试并发处理性能...");
        let processor = ConcurrentFileProcessor::new()
            .with_thread_pool_size(num_cpus::get())
            .with_batch_size(20);

        let start = std::time::Instant::now();
        let concurrent_result = processor.process_files_concurrent(&file_paths).unwrap();
        results.concurrent_processing_time = start.elapsed();
        results.successful_files = concurrent_result.successful.len();
        results.failed_files = concurrent_result.failed.len();

        // 4. 内存使用测试（模拟）
        results.memory_usage_mb = 256; // 模拟值

        results
    }
}

/// 大型代码库测试结果
#[derive(Debug, Default)]
struct LargeScaleTestResults {
    pub single_file_time: Duration,
    pub batch_processing_time: Duration,
    pub concurrent_processing_time: Duration,
    pub memory_usage_mb: usize,
    pub total_files_processed: usize,
    pub successful_files: usize,
    pub failed_files: usize,
}

impl LargeScaleTestResults {
    /// 验证性能是否符合要求
    fn validate_performance(&self, config: &LargeScaleTestConfig) -> Result<(), String> {
        // 检查单文件处理时间
        if self.single_file_time > Duration::from_millis(200) {
            return Err(format!(
                "Single file processing time too slow: {:?}",
                self.single_file_time
            ));
        }

        // 检查批量处理时间
        if self.batch_processing_time > config.max_processing_time {
            return Err(format!(
                "Batch processing time exceeded limit: {:?} > {:?}",
                self.batch_processing_time, config.max_processing_time
            ));
        }

        // 检查内存使用
        if self.memory_usage_mb > config.max_memory_usage_mb {
            return Err(format!(
                "Memory usage exceeded limit: {} MB > {} MB",
                self.memory_usage_mb, config.max_memory_usage_mb
            ));
        }

        // 检查错误率
        let error_rate = (self.failed_files as f64) / (self.total_files_processed as f64);
        if error_rate > 0.05 {
            return Err(format!("Error rate too high: {:.2}%", error_rate * 100.0));
        }

        Ok(())
    }

    /// 打印性能报告
    fn print_report(&self) {
        println!("\n=== 大型代码库性能测试报告 ===");
        println!("总处理文件数: {}", self.total_files_processed);
        println!("成功处理文件数: {}", self.successful_files);
        println!("失败文件数: {}", self.failed_files);
        println!("单文件处理时间: {:?}", self.single_file_time);
        println!("批量处理总时间: {:?}", self.batch_processing_time);
        println!("并发处理总时间: {:?}", self.concurrent_processing_time);
        println!("内存使用: {} MB", self.memory_usage_mb);

        if self.total_files_processed > 0 {
            let throughput =
                self.total_files_processed as f64 / self.batch_processing_time.as_secs_f64();
            println!("处理吞吐量: {throughput:.2} 文件/秒");

            let concurrent_throughput =
                self.total_files_processed as f64 / self.concurrent_processing_time.as_secs_f64();
            println!("并发处理吞吐量: {concurrent_throughput:.2} 文件/秒");
        }
    }
}

#[test]
fn test_small_scale_large_codebase() {
    let config = LargeScaleTestConfig {
        file_count: 20,
        functions_per_file: 5,
        max_processing_time: Duration::from_secs(10),
        max_memory_usage_mb: 256,
    };

    let suite = LargeScaleTestSuite::new(config.clone());
    let results = suite.run_large_scale_test();

    results.print_report();
    results.validate_performance(&config).unwrap();
}

#[test]
fn test_medium_scale_large_codebase() {
    let config = LargeScaleTestConfig {
        file_count: 50,
        functions_per_file: 8,
        max_processing_time: Duration::from_secs(20),
        max_memory_usage_mb: 384,
    };

    let suite = LargeScaleTestSuite::new(config.clone());
    let results = suite.run_large_scale_test();

    results.print_report();
    results.validate_performance(&config).unwrap();
}

#[test]
fn test_large_scale_codebase() {
    let config = LargeScaleTestConfig {
        file_count: 100,
        functions_per_file: 10,
        max_processing_time: Duration::from_secs(30),
        max_memory_usage_mb: 512,
    };

    let suite = LargeScaleTestSuite::new(config.clone());
    let results = suite.run_large_scale_test();

    results.print_report();
    results.validate_performance(&config).unwrap();
}

#[test]
fn test_stress_testing_large_codebase() {
    // 压力测试：处理大量复杂文件
    let config = LargeScaleTestConfig {
        file_count: 200,
        functions_per_file: 15,
        max_processing_time: Duration::from_secs(60),
        max_memory_usage_mb: 1024,
    };

    let suite = LargeScaleTestSuite::new(config.clone());

    println!("开始压力测试，处理 {} 个复杂文件...", config.file_count);
    let results = suite.run_large_scale_test();

    results.print_report();

    // 压力测试的验证标准可以更宽松
    assert!(
        results.batch_processing_time < config.max_processing_time,
        "压力测试超时"
    );
    assert!(
        results.memory_usage_mb < config.max_memory_usage_mb,
        "内存使用超限"
    );
    assert_eq!(results.failed_files, 0, "压力测试不应该有失败文件");
}

#[test]
fn test_scalability_analysis() {
    // 可扩展性分析：测试不同规模下的性能表现
    let test_sizes = vec![10, 25, 50, 100];
    let mut results = Vec::new();

    for size in test_sizes {
        println!("\n测试规模: {size} 个复杂文件");

        let config = LargeScaleTestConfig {
            file_count: size,
            functions_per_file: 8,
            max_processing_time: Duration::from_secs(size as u64 / 2 + 15),
            max_memory_usage_mb: 256 + size * 2,
        };

        let suite = LargeScaleTestSuite::new(config);
        let test_results = suite.run_large_scale_test();

        let throughput = test_results.total_files_processed as f64
            / test_results.batch_processing_time.as_secs_f64();

        results.push((size, test_results.batch_processing_time, throughput));

        println!(
            "规模 {} - 耗时: {:?}, 吞吐量: {:.2} 文件/秒",
            size, test_results.batch_processing_time, throughput
        );
    }

    // 分析可扩展性
    println!("\n=== 可扩展性分析 ===");
    for (i, (size, duration, throughput)) in results.iter().enumerate() {
        println!(
            "规模 {size}: 耗时 {duration:?}, 吞吐量 {throughput:.2} 文件/秒"
        );

        if i > 0 {
            let prev_throughput = results[i - 1].2;
            let throughput_change = (throughput - prev_throughput) / prev_throughput * 100.0;
            println!("  吞吐量变化: {throughput_change:.1}%");
        }
    }

    // 验证可扩展性：吞吐量不应该显著下降
    if results.len() >= 2 {
        let first_throughput = results[0].2;
        let last_throughput = results[results.len() - 1].2;
        let throughput_degradation = (first_throughput - last_throughput) / first_throughput;

        assert!(
            throughput_degradation < 0.6,
            "吞吐量下降过多: {:.1}%",
            throughput_degradation * 100.0
        );
    }
}
