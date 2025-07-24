//! 性能基准测试
//!
//! 使用 criterion 测试各种性能关键组件的性能

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use semantic_diff_core::{
    ConcurrentFileProcessor, MemoryEfficientAstProcessor, ParserCache, PerformanceMonitor,
    SupportedLanguage,
};
use std::hint::black_box;
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
    "strings"
)

// TestStruct{i} 是一个测试结构体
type TestStruct{i} struct {{
    ID   int    `json:"id"`
    Name string `json:"name"`
    Data []byte `json:"data"`
}}

// NewTestStruct{i} 创建新的测试结构体实例
func NewTestStruct{i}(id int, name string) *TestStruct{i} {{
    return &TestStruct{i} {{
        ID:   id,
        Name: name,
        Data: make([]byte, 1024),
    }}
}}

// Process{i} 处理数据
func (t *TestStruct{i}) Process{i}() error {{
    if t.Name == "" {{
        return fmt.Errorf("name cannot be empty")
    }}
    
    // 模拟一些处理逻辑
    for i := 0; i < len(t.Data); i++ {{
        t.Data[i] = byte(i % 256)
    }}
    
    return nil
}}

// Helper{i} 是一个辅助函数
func Helper{i}(input string) string {{
    return strings.ToUpper(input)
}}

func main() {{
    test := NewTestStruct{i}({i}, "test_{i}")
    if err := test.Process{i}(); err != nil {{
        fmt.Fprintf(os.Stderr, "Error: %v\n", err)
        os.Exit(1)
    }}
    
    result := Helper{i}("hello world")
    fmt.Println(result)
}}
"#
        );

        std::fs::write(&file_path, content).unwrap();
        files.push(file_path);
    }

    files
}

/// 基准测试：解析器缓存性能
fn bench_parser_cache(c: &mut Criterion) {
    let cache = ParserCache::new();

    c.bench_function("parser_cache_get_or_create", |b| {
        b.iter(|| {
            let _parser = cache
                .get_or_create_parser(black_box(SupportedLanguage::Go))
                .unwrap();
        })
    });

    // 测试缓存命中性能
    let _initial_parser = cache.get_or_create_parser(SupportedLanguage::Go).unwrap();

    c.bench_function("parser_cache_hit", |b| {
        b.iter(|| {
            let _parser = cache
                .get_or_create_parser(black_box(SupportedLanguage::Go))
                .unwrap();
        })
    });
}

/// 基准测试：单文件解析性能
fn bench_single_file_parsing(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 1);
    let processor = ConcurrentFileProcessor::new();

    c.bench_function("single_file_parsing", |b| {
        b.iter(|| {
            let result = processor
                .process_files_concurrent(black_box(&files))
                .unwrap();
            black_box(result);
        })
    });
}

/// 基准测试：并发文件处理性能
fn bench_concurrent_file_processing(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    let mut group = c.benchmark_group("concurrent_file_processing");

    // 测试不同文件数量的处理性能
    for file_count in [1, 5, 10, 20, 50].iter() {
        let files = create_test_go_files(&temp_dir, *file_count);

        group.throughput(Throughput::Elements(*file_count as u64));
        group.bench_with_input(BenchmarkId::new("files", file_count), &files, |b, files| {
            let processor = ConcurrentFileProcessor::new();
            b.iter(|| {
                let result = processor
                    .process_files_concurrent(black_box(files))
                    .unwrap();
                black_box(result);
            })
        });
    }

    group.finish();
}

/// 基准测试：不同线程数的并发性能
fn bench_thread_pool_sizes(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 20);

    let mut group = c.benchmark_group("thread_pool_sizes");

    // 测试不同线程池大小的性能
    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            thread_count,
            |b, &thread_count| {
                let processor = ConcurrentFileProcessor::new().with_thread_pool_size(thread_count);
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

/// 基准测试：批处理大小对性能的影响
fn bench_batch_sizes(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 50);

    let mut group = c.benchmark_group("batch_sizes");

    // 测试不同批处理大小的性能
    for batch_size in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            batch_size,
            |b, &batch_size| {
                let processor = ConcurrentFileProcessor::new().with_batch_size(batch_size);
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

/// 基准测试：内存高效 AST 处理器
fn bench_memory_efficient_processor(c: &mut Criterion) {
    let processor = MemoryEfficientAstProcessor::new().with_memory_monitoring(true);

    c.bench_function("memory_check", |b| {
        b.iter(|| {
            let _usage = processor.check_memory_usage();
        })
    });

    c.bench_function("gc_trigger_check", |b| {
        b.iter(|| {
            let _should_gc = processor.should_trigger_gc();
        })
    });
}

/// 基准测试：性能监控器
fn bench_performance_monitor(c: &mut Criterion) {
    let monitor = PerformanceMonitor::new();

    c.bench_function("record_file_processed", |b| {
        b.iter(|| {
            monitor.record_file_processed(black_box(Duration::from_millis(100)));
        })
    });

    c.bench_function("record_error", |b| {
        b.iter(|| {
            monitor.record_error();
        })
    });
}

/// 基准测试：大型项目模拟
fn bench_large_project_simulation(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    let mut group = c.benchmark_group("large_project_simulation");
    group.sample_size(10); // 减少样本数量，因为这是一个耗时的测试
    group.measurement_time(Duration::from_secs(30)); // 增加测量时间

    // 模拟不同规模的项目
    for file_count in [100, 200, 500].iter() {
        let files = create_test_go_files(&temp_dir, *file_count);

        group.throughput(Throughput::Elements(*file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("large_project", file_count),
            &files,
            |b, files| {
                let processor = ConcurrentFileProcessor::new()
                    .with_thread_pool_size(num_cpus::get())
                    .with_batch_size(20);
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

/// 基准测试：内存使用模式
fn bench_memory_patterns(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let files = create_test_go_files(&temp_dir, 50);

    let mut group = c.benchmark_group("memory_patterns");

    // 测试不同内存阈值的影响
    for threshold_mb in [128, 256, 512, 1024].iter() {
        let threshold = threshold_mb * 1024 * 1024; // 转换为字节

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

/// 基准测试：错误处理性能
fn bench_error_handling(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    // 创建一些正常文件和一些损坏的文件
    let mut files = create_test_go_files(&temp_dir, 10);

    // 添加一些不存在的文件路径来模拟错误
    for i in 0..5 {
        files.push(temp_dir.path().join(format!("nonexistent_{i}.go")));
    }

    c.bench_function("error_handling", |b| {
        let processor = ConcurrentFileProcessor::new();
        b.iter(|| {
            let result = processor
                .process_files_concurrent(black_box(&files))
                .unwrap();
            black_box(result);
        })
    });
}

criterion_group!(
    benches,
    bench_parser_cache,
    bench_single_file_parsing,
    bench_concurrent_file_processing,
    bench_thread_pool_sizes,
    bench_batch_sizes,
    bench_memory_efficient_processor,
    bench_performance_monitor,
    bench_large_project_simulation,
    bench_memory_patterns,
    bench_error_handling
);

criterion_main!(benches);
