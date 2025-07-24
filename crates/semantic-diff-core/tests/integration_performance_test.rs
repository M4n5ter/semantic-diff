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
