//! 大型代码库基准测试
//!
//! 专门测试处理大型代码库时的性能表现

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use semantic_diff_core::{
    analyzer::SourceAnalyzer,
    extractor::SemanticContextExtractor,
    generator::CodeSliceGenerator,
    parser::{LanguageSpecificInfo, SupportedLanguage},
    performance::{ConcurrentFileProcessor, MemoryEfficientAstProcessor, ParserCache},
};
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// 创建大型复杂的 Go 源文件
fn create_large_go_files(temp_dir: &TempDir, count: usize, complexity: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for file_index in 0..count {
        let file_path = temp_dir.path().join(format!("large_file_{file_index}.go"));
        let content = generate_complex_go_content(file_index, complexity);

        std::fs::write(&file_path, content).unwrap();
        files.push(file_path);
    }

    files
}

/// 生成复杂的 Go 文件内容
fn generate_complex_go_content(file_index: usize, complexity: usize) -> String {
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
    "errors"
    "strings"
    "strconv"
    "io"
    "os"
    "path/filepath"
)

// File{file_index}Config 配置结构
type File{file_index}Config struct {{
    DatabaseURL    string        `json:"database_url" yaml:"database_url"`
    RedisURL       string        `json:"redis_url" yaml:"redis_url"`
    MaxConnections int           `json:"max_connections" yaml:"max_connections"`
    Timeout        time.Duration `json:"timeout" yaml:"timeout"`
    RetryCount     int           `json:"retry_count" yaml:"retry_count"`
    BatchSize      int           `json:"batch_size" yaml:"batch_size"`
    WorkerCount    int           `json:"worker_count" yaml:"worker_count"`
    EnableMetrics  bool          `json:"enable_metrics" yaml:"enable_metrics"`
    LogLevel       string        `json:"log_level" yaml:"log_level"`
    Features       map[string]bool `json:"features" yaml:"features"`
}}

// File{file_index}Data 复杂数据结构
type File{file_index}Data struct {{
    ID          int64                  `json:"id" db:"id"`
    Name        string                 `json:"name" db:"name"`
    Description string                 `json:"description" db:"description"`
    Value       float64                `json:"value" db:"value"`
    Timestamp   time.Time              `json:"timestamp" db:"timestamp"`
    CreatedAt   time.Time              `json:"created_at" db:"created_at"`
    UpdatedAt   time.Time              `json:"updated_at" db:"updated_at"`
    Metadata    map[string]interface{{}} `json:"metadata" db:"metadata"`
    Tags        []string               `json:"tags" db:"tags"`
    Status      File{file_index}Status `json:"status" db:"status"`
    Priority    int                    `json:"priority" db:"priority"`
    Config      *File{file_index}Config `json:"config,omitempty" db:"config"`
    Children    []*File{file_index}Data `json:"children,omitempty"`
    Parent      *File{file_index}Data   `json:"parent,omitempty"`
    Relations   map[string]*File{file_index}Data `json:"relations,omitempty"`
    Attributes  map[string]string      `json:"attributes" db:"attributes"`
}}

// File{file_index}Status 状态枚举
type File{file_index}Status int

const (
    File{file_index}StatusPending File{file_index}Status = iota
    File{file_index}StatusProcessing
    File{file_index}StatusCompleted
    File{file_index}StatusFailed
    File{file_index}StatusCancelled
    File{file_index}StatusRetrying
)

// String 实现 Stringer 接口
func (s File{file_index}Status) String() string {{
    switch s {{
    case File{file_index}StatusPending:
        return "pending"
    case File{file_index}StatusProcessing:
        return "processing"
    case File{file_index}StatusCompleted:
        return "completed"
    case File{file_index}StatusFailed:
        return "failed"
    case File{file_index}StatusCancelled:
        return "cancelled"
    case File{file_index}StatusRetrying:
        return "retrying"
    default:
        return "unknown"
    }}
}}

// File{file_index}Service 复杂服务结构
type File{file_index}Service struct {{
    data           map[int64]*File{file_index}Data
    mutex          sync.RWMutex
    config         *File{file_index}Config
    db             *sql.DB
    httpClient     *http.Client
    workerPool     chan struct{{}}
    ctx            context.Context
    cancel         context.CancelFunc
    wg             sync.WaitGroup
    stats          *File{file_index}Stats
    cache          map[string]interface{{}}
    cacheMutex     sync.RWMutex
    eventHandlers  map[string][]File{file_index}EventHandler
    handlerMutex   sync.RWMutex
    metrics        *File{file_index}Metrics
    logger         *log.Logger
}}

// File{file_index}Stats 统计信息
type File{file_index}Stats struct {{
    ProcessedCount    int64         `json:"processed_count"`
    ErrorCount        int64         `json:"error_count"`
    LastProcessed     time.Time     `json:"last_processed"`
    AverageTime       time.Duration `json:"average_time"`
    TotalTime         time.Duration `json:"total_time"`
    CacheHits         int64         `json:"cache_hits"`
    CacheMisses       int64         `json:"cache_misses"`
    ActiveWorkers     int32         `json:"active_workers"`
    QueueSize         int32         `json:"queue_size"`
    ThroughputPerSec  float64       `json:"throughput_per_sec"`
}}

// File{file_index}EventHandler 事件处理器类型
type File{file_index}EventHandler func(event *File{file_index}Event) error

// File{file_index}Event 事件结构
type File{file_index}Event struct {{
    Type      string                 `json:"type"`
    Data      *File{file_index}Data  `json:"data"`
    Timestamp time.Time              `json:"timestamp"`
    Metadata  map[string]interface{{}} `json:"metadata"`
}}

// File{file_index}Metrics 指标结构
type File{file_index}Metrics struct {{
    Counters   map[string]int64   `json:"counters"`
    Gauges     map[string]float64 `json:"gauges"`
    Histograms map[string][]float64 `json:"histograms"`
    mutex      sync.RWMutex
}}

"#
    );

    // 生成多个复杂函数
    for func_index in 0..complexity {
        let function_content = format!(
            r#"
// ProcessData{func_index} 复杂数据处理方法 {func_index}
func (s *File{file_index}Service) ProcessData{func_index}(ctx context.Context, data *File{file_index}Data) error {{
    start := time.Now()
    defer func() {{
        duration := time.Since(start)
        s.updateStats(duration)
        s.emitEvent("data_processed", data, map[string]interface{{}}{{
            "function": "ProcessData{func_index}",
            "duration": duration.String(),
        }})
    }}()

    // 增加活跃工作者计数
    atomic.AddInt32(&s.stats.ActiveWorkers, 1)
    defer atomic.AddInt32(&s.stats.ActiveWorkers, -1)

    s.mutex.Lock()
    defer s.mutex.Unlock()
    
    if data == nil {{
        s.stats.ErrorCount++
        return fmt.Errorf("data cannot be nil")
    }}
    
    // 复杂的验证逻辑
    if err := s.validateDataComplex{func_index}(data); err != nil {{
        s.stats.ErrorCount++
        s.logError("validation_failed", err, data)
        return fmt.Errorf("validation failed: %w", err)
    }}
    
    // 检查容量限制
    if len(s.data) >= s.config.MaxConnections {{
        s.stats.ErrorCount++
        return fmt.Errorf("maximum items reached: %d", s.config.MaxConnections)
    }}
    
    // 多级缓存检查
    cacheKey := fmt.Sprintf("data_%d_%d_%d", {file_index}, {func_index}, data.ID)
    if cached := s.getFromMultiLevelCache(cacheKey); cached != nil {{
        s.stats.CacheHits++
        s.metrics.IncrementCounter("cache_hits")
        return nil
    }}
    s.stats.CacheMisses++
    s.metrics.IncrementCounter("cache_misses")
    
    // 数据库事务处理
    tx, err := s.db.BeginTx(ctx, &sql.TxOptions{{Isolation: sql.LevelReadCommitted}})
    if err != nil {{
        s.stats.ErrorCount++
        return fmt.Errorf("failed to begin transaction: %w", err)
    }}
    defer tx.Rollback()
    
    // 复杂的数据库操作
    if err := s.saveToDatabase{func_index}(ctx, tx, data); err != nil {{
        s.stats.ErrorCount++
        s.logError("database_save_failed", err, data)
        return fmt.Errorf("database save failed: %w", err)
    }}
    
    // 提交事务
    if err := tx.Commit(); err != nil {{
        s.stats.ErrorCount++
        return fmt.Errorf("failed to commit transaction: %w", err)
    }}
    
    // 设置默认值和元数据
    s.enrichDataWithDefaults{func_index}(data)
    
    // 状态更新
    data.Status = File{file_index}StatusProcessing
    data.UpdatedAt = time.Now()
    
    // 异步处理相关数据
    if len(data.Children) > 0 {{
        go s.processChildrenAsync{func_index}(ctx, data)
    }}
    
    if len(data.Relations) > 0 {{
        go s.processRelationsAsync{func_index}(ctx, data)
    }}
    
    // 存储数据
    s.data[data.ID] = data
    
    // 多级缓存存储
    s.setToMultiLevelCache(cacheKey, data)
    
    // 更新指标
    s.metrics.IncrementCounter("data_processed")
    s.metrics.UpdateGauge("data_count", float64(len(s.data)))
    s.metrics.AddToHistogram("processing_time", float64(time.Since(start).Milliseconds()))
    
    return nil
}}

// validateDataComplex{func_index} 复杂验证方法 {func_index}
func (s *File{file_index}Service) validateDataComplex{func_index}(data *File{file_index}Data) error {{
    if data.Name == "" {{
        return fmt.Errorf("name cannot be empty")
    }}
    
    if len(data.Name) > 255 {{
        return fmt.Errorf("name too long: %d characters", len(data.Name))
    }}
    
    if data.Value < 0 {{
        return fmt.Errorf("value cannot be negative: %f", data.Value)
    }}
    
    if data.ID <= 0 {{
        return fmt.Errorf("invalid ID: %d", data.ID)
    }}
    
    // 验证标签
    if len(data.Tags) > 100 {{
        return fmt.Errorf("too many tags: %d", len(data.Tags))
    }}
    
    for i, tag := range data.Tags {{
        if tag == "" {{
            return fmt.Errorf("tag at index %d cannot be empty", i)
        }}
        if len(tag) > 50 {{
            return fmt.Errorf("tag at index %d too long: %s", i, tag)
        }}
        if strings.Contains(tag, " ") {{
            return fmt.Errorf("tag at index %d cannot contain spaces: %s", i, tag)
        }}
    }}
    
    // 验证状态
    if data.Status < File{file_index}StatusPending || data.Status > File{file_index}StatusRetrying {{
        return fmt.Errorf("invalid status: %d", data.Status)
    }}
    
    // 验证优先级
    if data.Priority < 0 || data.Priority > 10 {{
        return fmt.Errorf("invalid priority: %d", data.Priority)
    }}
    
    // 验证元数据
    if len(data.Metadata) > 1000 {{
        return fmt.Errorf("too many metadata entries: %d", len(data.Metadata))
    }}
    
    for key, value := range data.Metadata {{
        if key == "" {{
            return fmt.Errorf("metadata key cannot be empty")
        }}
        if len(key) > 100 {{
            return fmt.Errorf("metadata key too long: %s", key)
        }}
        if value == nil {{
            return fmt.Errorf("metadata value cannot be nil for key: %s", key)
        }}
    }}
    
    // 验证属性
    for key, value := range data.Attributes {{
        if key == "" {{
            return fmt.Errorf("attribute key cannot be empty")
        }}
        if value == "" {{
            return fmt.Errorf("attribute value cannot be empty for key: %s", key)
        }}
    }}
    
    return nil
}}

// saveToDatabase{func_index} 保存到数据库方法 {func_index}
func (s *File{file_index}Service) saveToDatabase{func_index}(ctx context.Context, tx *sql.Tx, data *File{file_index}Data) error {{
    if tx == nil {{
        return fmt.Errorf("transaction cannot be nil")
    }}
    
    // 主表插入
    mainQuery := `
        INSERT INTO file_{file_index}_data 
        (id, name, description, value, timestamp, created_at, updated_at, status, priority) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
        name = VALUES(name),
        description = VALUES(description),
        value = VALUES(value),
        timestamp = VALUES(timestamp),
        updated_at = VALUES(updated_at),
        status = VALUES(status),
        priority = VALUES(priority)
    `
    
    ctxWithTimeout, cancel := context.WithTimeout(ctx, s.config.Timeout)
    defer cancel()
    
    _, err := tx.ExecContext(ctxWithTimeout, mainQuery, 
        data.ID, data.Name, data.Description, data.Value, 
        data.Timestamp, data.CreatedAt, data.UpdatedAt, 
        data.Status, data.Priority)
    if err != nil {{
        return fmt.Errorf("failed to execute main query: %w", err)
    }}
    
    // 标签表插入
    if len(data.Tags) > 0 {{
        // 先删除旧标签
        _, err = tx.ExecContext(ctxWithTimeout, 
            "DELETE FROM file_{file_index}_tags WHERE data_id = ?", data.ID)
        if err != nil {{
            return fmt.Errorf("failed to delete old tags: %w", err)
        }}
        
        // 插入新标签
        tagQuery := "INSERT INTO file_{file_index}_tags (data_id, tag) VALUES (?, ?)"
        for _, tag := range data.Tags {{
            _, err = tx.ExecContext(ctxWithTimeout, tagQuery, data.ID, tag)
            if err != nil {{
                return fmt.Errorf("failed to insert tag %s: %w", tag, err)
            }}
        }}
    }}
    
    // 元数据表插入
    if len(data.Metadata) > 0 {{
        metadataJSON, err := json.Marshal(data.Metadata)
        if err != nil {{
            return fmt.Errorf("failed to marshal metadata: %w", err)
        }}
        
        metadataQuery := `
            INSERT INTO file_{file_index}_metadata (data_id, metadata_json) 
            VALUES (?, ?)
            ON DUPLICATE KEY UPDATE metadata_json = VALUES(metadata_json)
        `
        _, err = tx.ExecContext(ctxWithTimeout, metadataQuery, data.ID, metadataJSON)
        if err != nil {{
            return fmt.Errorf("failed to insert metadata: %w", err)
        }}
    }}
    
    // 属性表插入
    if len(data.Attributes) > 0 {{
        // 先删除旧属性
        _, err = tx.ExecContext(ctxWithTimeout, 
            "DELETE FROM file_{file_index}_attributes WHERE data_id = ?", data.ID)
        if err != nil {{
            return fmt.Errorf("failed to delete old attributes: %w", err)
        }}
        
        // 插入新属性
        attrQuery := "INSERT INTO file_{file_index}_attributes (data_id, attr_key, attr_value) VALUES (?, ?, ?)"
        for key, value := range data.Attributes {{
            _, err = tx.ExecContext(ctxWithTimeout, attrQuery, data.ID, key, value)
            if err != nil {{
                return fmt.Errorf("failed to insert attribute %s: %w", key, err)
            }}
        }}
    }}
    
    return nil
}}

// enrichDataWithDefaults{func_index} 丰富数据默认值方法 {func_index}
func (s *File{file_index}Service) enrichDataWithDefaults{func_index}(data *File{file_index}Data) {{
    if data.Timestamp.IsZero() {{
        data.Timestamp = time.Now()
    }}
    
    if data.CreatedAt.IsZero() {{
        data.CreatedAt = time.Now()
    }}
    
    data.UpdatedAt = time.Now()
    
    if data.Metadata == nil {{
        data.Metadata = make(map[string]interface{{}})
    }}
    
    if data.Attributes == nil {{
        data.Attributes = make(map[string]string)
    }}
    
    if data.Relations == nil {{
        data.Relations = make(map[string]*File{file_index}Data)
    }}
    
    // 添加处理元数据
    data.Metadata["processed_by"] = fmt.Sprintf("File{file_index}Service.ProcessData{func_index}")
    data.Metadata["processed_at"] = time.Now().Unix()
    data.Metadata["file_index"] = {file_index}
    data.Metadata["func_index"] = {func_index}
    data.Metadata["version"] = "2.0.0"
    data.Metadata["processor_id"] = fmt.Sprintf("proc_%d_%d", {file_index}, {func_index})
    
    // 添加默认属性
    data.Attributes["source"] = "File{file_index}Service"
    data.Attributes["environment"] = "production"
    data.Attributes["region"] = "us-west-2"
    data.Attributes["instance_id"] = fmt.Sprintf("inst_%d", {file_index})
}}
"#
        );
        content.push_str(&function_content);
    }

    // 添加辅助方法
    content.push_str(&format!(
        r#"
// 辅助方法实现

// updateStats 更新统计信息
func (s *File{file_index}Service) updateStats(duration time.Duration) {{
    s.stats.TotalTime += duration
    s.stats.ProcessedCount++
    s.stats.LastProcessed = time.Now()
    if s.stats.ProcessedCount > 0 {{
        s.stats.AverageTime = s.stats.TotalTime / time.Duration(s.stats.ProcessedCount)
    }}
    
    // 计算吞吐量
    if s.stats.TotalTime > 0 {{
        s.stats.ThroughputPerSec = float64(s.stats.ProcessedCount) / s.stats.TotalTime.Seconds()
    }}
}}

// emitEvent 发送事件
func (s *File{file_index}Service) emitEvent(eventType string, data *File{file_index}Data, metadata map[string]interface{{}}) {{
    event := &File{file_index}Event{{
        Type:      eventType,
        Data:      data,
        Timestamp: time.Now(),
        Metadata:  metadata,
    }}
    
    s.handlerMutex.RLock()
    handlers := s.eventHandlers[eventType]
    s.handlerMutex.RUnlock()
    
    for _, handler := range handlers {{
        go func(h File{file_index}EventHandler) {{
            if err := h(event); err != nil {{
                s.logError("event_handler_failed", err, data)
            }}
        }}(handler)
    }}
}}

// logError 记录错误
func (s *File{file_index}Service) logError(errorType string, err error, data *File{file_index}Data) {{
    s.logger.Printf("[ERROR] %s: %v, data_id: %d", errorType, err, data.ID)
    s.metrics.IncrementCounter("errors_" + errorType)
}}

// getFromMultiLevelCache 从多级缓存获取数据
func (s *File{file_index}Service) getFromMultiLevelCache(key string) interface{{}} {{
    s.cacheMutex.RLock()
    defer s.cacheMutex.RUnlock()
    
    // L1 缓存检查
    if value, exists := s.cache[key]; exists {{
        s.metrics.IncrementCounter("l1_cache_hits")
        return value
    }}
    
    // L2 缓存检查（模拟 Redis）
    if s.config.RedisURL != "" {{
        // 这里应该是实际的 Redis 调用
        s.metrics.IncrementCounter("l2_cache_misses")
    }}
    
    return nil
}}

// setToMultiLevelCache 设置多级缓存
func (s *File{file_index}Service) setToMultiLevelCache(key string, value interface{{}}) {{
    s.cacheMutex.Lock()
    defer s.cacheMutex.Unlock()
    
    // L1 缓存设置
    s.cache[key] = value
    
    // L2 缓存设置（模拟 Redis）
    if s.config.RedisURL != "" {{
        // 这里应该是实际的 Redis 调用
        s.metrics.IncrementCounter("l2_cache_sets")
    }}
}}

// processChildrenAsync 异步处理子项
func (s *File{file_index}Service) processChildrenAsync(ctx context.Context, parent *File{file_index}Data) {{
    s.wg.Add(1)
    defer s.wg.Done()
    
    for _, child := range parent.Children {{
        select {{
        case <-ctx.Done():
            return
        case s.workerPool <- struct{{}}{{}}:
            go func(c *File{file_index}Data) {{
                defer func() {{ <-s.workerPool }}()
                
                if err := s.ProcessData0(ctx, c); err != nil {{
                    s.logError("child_processing_failed", err, c)
                }}
            }}(child)
        }}
    }}
}}

// processRelationsAsync 异步处理关联数据
func (s *File{file_index}Service) processRelationsAsync(ctx context.Context, data *File{file_index}Data) {{
    s.wg.Add(1)
    defer s.wg.Done()
    
    for relationType, relatedData := range data.Relations {{
        select {{
        case <-ctx.Done():
            return
        case s.workerPool <- struct{{}}{{}}:
            go func(rt string, rd *File{file_index}Data) {{
                defer func() {{ <-s.workerPool }}()
                
                if err := s.ProcessData0(ctx, rd); err != nil {{
                    s.logError("relation_processing_failed", err, rd)
                    s.logger.Printf("[ERROR] Failed to process relation %s: %v", rt, err)
                }}
            }}(relationType, relatedData)
        }}
    }}
}}

// IncrementCounter 增加计数器
func (m *File{file_index}Metrics) IncrementCounter(name string) {{
    m.mutex.Lock()
    defer m.mutex.Unlock()
    
    if m.Counters == nil {{
        m.Counters = make(map[string]int64)
    }}
    m.Counters[name]++
}}

// UpdateGauge 更新仪表
func (m *File{file_index}Metrics) UpdateGauge(name string, value float64) {{
    m.mutex.Lock()
    defer m.mutex.Unlock()
    
    if m.Gauges == nil {{
        m.Gauges = make(map[string]float64)
    }}
    m.Gauges[name] = value
}}

// AddToHistogram 添加到直方图
func (m *File{file_index}Metrics) AddToHistogram(name string, value float64) {{
    m.mutex.Lock()
    defer m.mutex.Unlock()
    
    if m.Histograms == nil {{
        m.Histograms = make(map[string][]float64)
    }}
    m.Histograms[name] = append(m.Histograms[name], value)
    
    // 保持直方图大小限制
    if len(m.Histograms[name]) > 1000 {{
        m.Histograms[name] = m.Histograms[name][1:]
    }}
}}

// NewFile{file_index}Service 创建服务实例
func NewFile{file_index}Service(config *File{file_index}Config) *File{file_index}Service {{
    if config == nil {{
        config = &File{file_index}Config{{
            MaxConnections: 1000,
            Timeout:        30 * time.Second,
            RetryCount:     3,
            BatchSize:      20,
            WorkerCount:    8,
            EnableMetrics:  true,
            LogLevel:       "info",
            Features:       make(map[string]bool),
        }}
    }}
    
    ctx, cancel := context.WithCancel(context.Background())
    
    service := &File{file_index}Service{{
        data:          make(map[int64]*File{file_index}Data),
        config:        config,
        httpClient:    &http.Client{{Timeout: config.Timeout}},
        workerPool:    make(chan struct{{}}, config.WorkerCount),
        ctx:           ctx,
        cancel:        cancel,
        stats:         &File{file_index}Stats{{}},
        cache:         make(map[string]interface{{}}),
        eventHandlers: make(map[string][]File{file_index}EventHandler),
        metrics:       &File{file_index}Metrics{{}},
        logger:        log.New(os.Stdout, fmt.Sprintf("[File{file_index}Service] "), log.LstdFlags),
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

// GetMetrics 获取指标
func (s *File{file_index}Service) GetMetrics() *File{file_index}Metrics {{
    return s.metrics
}}
"#
    ));

    content
}

/// 基准测试：大型文件解析性能
fn bench_large_file_parsing(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_large_go_files(&temp_dir, 1, 20); // 1个文件，20个复杂函数

    let mut group = c.benchmark_group("large_file_parsing");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("single_large_file", |b| {
        b.iter(|| {
            let mut analyzer = SourceAnalyzer::new_for_language(SupportedLanguage::Go).unwrap();
            let result = analyzer.analyze_file(black_box(&files[0])).unwrap();
            black_box(result);
        })
    });

    group.finish();
}

/// 基准测试：大量文件并发处理
fn bench_massive_file_processing(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    let mut group = c.benchmark_group("massive_file_processing");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    // 测试不同规模的文件处理
    for file_count in [50, 100, 200, 500].iter() {
        let files = create_large_go_files(&temp_dir, *file_count, 5);

        group.throughput(Throughput::Elements(*file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrent_processing", file_count),
            &files,
            |b, files| {
                let processor = ConcurrentFileProcessor::new()
                    .with_thread_pool_size(num_cpus::get())
                    .with_batch_size(25);

                b.iter(|| {
                    let result = processor
                        .process_files_concurrent(black_box(files))
                        .unwrap();
                    black_box(result);
                })
            },
        );
    }

    group.finish();
}

/// 基准测试：内存高效处理
fn bench_memory_efficient_processing(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_large_go_files(&temp_dir, 100, 8);

    let mut group = c.benchmark_group("memory_efficient_processing");
    group.sample_size(10);

    // 测试不同内存阈值的影响
    for threshold_mb in [128, 256, 512, 1024].iter() {
        let threshold = threshold_mb * 1024 * 1024;

        group.bench_with_input(
            BenchmarkId::new("memory_threshold_mb", threshold_mb),
            &threshold,
            |b, &threshold| {
                let ast_processor = MemoryEfficientAstProcessor::new()
                    .with_memory_threshold(threshold)
                    .with_memory_monitoring(true);

                let processor = ConcurrentFileProcessor::new().with_ast_processor(ast_processor);

                b.iter(|| {
                    let result = processor
                        .process_files_concurrent(black_box(&files))
                        .unwrap();
                    black_box(result);
                })
            },
        );
    }

    group.finish();
}

/// 基准测试：解析器缓存在大规模处理中的效果
fn bench_parser_cache_large_scale(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_large_go_files(&temp_dir, 200, 5);

    let mut group = c.benchmark_group("parser_cache_large_scale");
    group.sample_size(10);

    // 测试有缓存和无缓存的性能差异
    group.bench_function("with_cache", |b| {
        let cache = ParserCache::new();
        b.iter(|| {
            for _file in &files {
                let _parser = cache
                    .get_or_create_parser(black_box(SupportedLanguage::Go))
                    .unwrap();
                // 模拟使用解析器
                std::thread::sleep(Duration::from_micros(10));
            }
        })
    });

    group.bench_function("without_cache", |b| {
        b.iter(|| {
            for _file in &files {
                let _parser = SourceAnalyzer::new_for_language(SupportedLanguage::Go).unwrap();
                // 模拟使用解析器
                std::thread::sleep(Duration::from_micros(10));
            }
        })
    });

    group.finish();
}

/// 基准测试：语义上下文提取在大型项目中的性能
fn bench_semantic_extraction_large_project(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_large_go_files(&temp_dir, 50, 10);

    // 预处理：分析所有文件
    let source_files: Vec<_> = files
        .iter()
        .map(|file| {
            let mut analyzer = SourceAnalyzer::new_for_language(SupportedLanguage::Go).unwrap();
            analyzer.analyze_file(file).unwrap()
        })
        .collect();

    let mut group = c.benchmark_group("semantic_extraction_large_project");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(45));

    group.bench_function("extract_contexts", |b| {
        let extractor = SemanticContextExtractor::new();

        b.iter(|| {
            for source_file in &source_files {
                // 从语言特定信息中获取函数
                if let Some(go_info) = source_file
                    .language_specific
                    .as_any()
                    .downcast_ref::<semantic_diff_core::parser::GoLanguageInfo>()
                {
                    for declaration in go_info.declarations() {
                        if let Some(semantic_diff_core::parser::GoDeclaration::Function(
                            func_info,
                        )) = declaration
                            .as_any()
                            .downcast_ref::<semantic_diff_core::parser::GoDeclaration>()
                        {
                            let change_target =
                                semantic_diff_core::extractor::ChangeTarget::Function(
                                    func_info.clone(),
                                );
                            let context = extractor
                                .extract_context_for_target(
                                    black_box(change_target),
                                    black_box(&source_files),
                                )
                                .unwrap();
                            black_box(context);
                        }
                    }
                }
            }
        })
    });

    group.finish();
}

/// 基准测试：代码切片生成在大型上下文中的性能
fn bench_code_generation_large_context(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_large_go_files(&temp_dir, 30, 15);

    // 预处理：分析文件并提取上下文
    let mut contexts = Vec::new();
    let source_files: Vec<_> = files
        .iter()
        .map(|file| {
            let mut analyzer = SourceAnalyzer::new_for_language(SupportedLanguage::Go).unwrap();
            analyzer.analyze_file(file).unwrap()
        })
        .collect();

    let extractor = SemanticContextExtractor::new();
    for source_file in &source_files {
        // 从语言特定信息中获取函数
        if let Some(go_info) = source_file
            .language_specific
            .as_any()
            .downcast_ref::<semantic_diff_core::parser::GoLanguageInfo>()
        {
            let mut count = 0;
            for declaration in go_info.declarations() {
                if count >= 3 {
                    break;
                } // 只取前3个函数避免测试时间过长
                if let Some(semantic_diff_core::parser::GoDeclaration::Function(func_info)) =
                    declaration
                        .as_any()
                        .downcast_ref::<semantic_diff_core::parser::GoDeclaration>()
                {
                    let change_target =
                        semantic_diff_core::extractor::ChangeTarget::Function(func_info.clone());
                    if let Ok(context) =
                        extractor.extract_context_for_target(change_target, &source_files)
                    {
                        contexts.push(context);
                        count += 1;
                    }
                }
            }
        }
    }

    let mut group = c.benchmark_group("code_generation_large_context");
    group.sample_size(10);

    group.bench_function("generate_code_slices", |b| {
        let generator = CodeSliceGenerator::new();

        b.iter(|| {
            for context in &contexts {
                let code_slice = generator
                    .generate_slice(black_box(context), black_box(&[]))
                    .unwrap();
                black_box(code_slice);
            }
        })
    });

    group.finish();
}

/// 基准测试：极限压力测试
fn bench_stress_test(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_large_go_files(&temp_dir, 1000, 3); // 1000个文件，每个3个函数

    let mut group = c.benchmark_group("stress_test");
    group.sample_size(5); // 减少样本数量
    group.measurement_time(Duration::from_secs(120)); // 增加测量时间

    group.bench_function("extreme_load", |b| {
        let processor = ConcurrentFileProcessor::new()
            .with_thread_pool_size(num_cpus::get() * 2)
            .with_batch_size(50);

        b.iter(|| {
            let result = processor
                .process_files_concurrent(black_box(&files))
                .unwrap();
            black_box(result);
        })
    });

    group.finish();
}

criterion_group!(
    large_codebase_benches,
    bench_large_file_parsing,
    bench_massive_file_processing,
    bench_memory_efficient_processing,
    bench_parser_cache_large_scale,
    bench_semantic_extraction_large_project,
    bench_code_generation_large_context,
    bench_stress_test
);

criterion_main!(large_codebase_benches);
