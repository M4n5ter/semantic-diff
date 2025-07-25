# semantic-diff 测试指南

## 概述

本文档介绍了 semantic-diff 项目的测试策略、测试类型和运行方法。项目采用多层次的测试方法，确保代码质量和性能表现。

## 测试架构

### 测试类型

1. **单元测试** - 测试单个组件的功能
2. **集成测试** - 测试组件间的交互
3. **端到端测试** - 测试完整的工作流程
4. **性能测试** - 测试系统性能和可扩展性
5. **基准测试** - 使用 criterion 进行精确的性能测量

### 测试结构

```
crates/semantic-diff-core/
├── src/
│   ├── **/*.rs          # 单元测试（#[cfg(test)] 模块）
│   └── */tests.rs       # 组件特定的测试
├── tests/               # 集成测试
│   ├── integration_*.rs # 各种集成测试
│   └── test_data/       # 测试数据集
├── benches/             # 基准测试
│   ├── performance_benchmarks.rs
│   └── large_codebase_benchmarks.rs
└── Cargo.toml          # 测试依赖配置
```

## 运行测试

### 基本测试命令

```bash
# 运行所有测试
cargo test

# 运行特定包的测试
cargo test --package semantic-diff-core

# 运行特定测试
cargo test test_parser_cache_functionality

# 运行集成测试
cargo test --test integration_performance_test

# 运行发布模式测试（更快）
cargo test --release
```

### 基准测试命令

```bash
# 运行所有基准测试
cargo bench

# 运行特定基准测试
cargo bench --bench performance_benchmarks

# 运行大型代码库基准测试
cargo bench --bench large_codebase_benchmarks

# 运行特定的基准测试函数
cargo bench bench_parser_cache
```

### 测试过滤

```bash
# 运行包含特定名称的测试
cargo test parser

# 运行性能相关的测试
cargo test performance

# 忽略慢速测试
cargo test -- --skip slow
```

## 测试数据集

### 测试数据生成

项目包含自动生成的测试数据集，用于测试各种 Go 语言结构：

- **基础结构体和方法**
- **接口和实现**
- **复杂的嵌套结构和泛型**
- **错误处理和自定义错误类型**
- **并发和 goroutine**
- **包级别函数和常量**

### 使用测试数据

```rust
use crate::test_data::TestDataSet;

// 创建综合测试数据集
let dataset = TestDataSet::create_comprehensive_go_dataset()?;

// 创建 Git 仓库测试数据
let git_dataset = TestDataSet::create_git_repo_dataset()?;

// 获取文件路径
let file_paths = dataset.get_file_paths();
```

## 性能测试

### 性能测试配置

```rust
struct PerformanceTestConfig {
    pub file_count: usize,
    pub functions_per_file: usize,
    pub max_processing_time: Duration,
    pub max_memory_usage_mb: usize,
}
```

### 性能测试类型

1. **单文件处理性能**
   - 测试单个文件的解析时间
   - 目标：< 100ms per file

2. **批量处理性能**
   - 测试大量文件的并发处理
   - 目标：< 30s for 100 files

3. **内存效率测试**
   - 测试内存使用和泄漏
   - 目标：< 512MB for large projects

4. **可扩展性测试**
   - 测试不同规模下的性能表现
   - 验证吞吐量不显著下降

### 运行性能测试

```bash
# 小规模性能测试
cargo test test_small_dataset_performance --release

# 中等规模性能测试
cargo test test_medium_dataset_performance --release

# 大规模性能测试
cargo test test_large_dataset_performance --release

# 压力测试
cargo test test_stress_testing --release
```

## 基准测试

### 基准测试指标

使用 criterion crate 进行精确的性能测量：

1. **解析器缓存性能**
   - 缓存命中率和创建时间
   - 内存使用效率

2. **并发文件处理**
   - 不同线程数的性能对比
   - 批处理大小的影响

3. **语义上下文提取**
   - 复杂项目中的提取性能
   - 依赖解析效率

4. **代码生成性能**
   - 大型上下文的代码切片生成
   - 格式化和高亮性能

### 基准测试示例

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_parser_performance(c: &mut Criterion) {
    c.bench_function("parse_large_file", |b| {
        b.iter(|| {
            // 基准测试代码
        })
    });
}

criterion_group!(benches, bench_parser_performance);
criterion_main!(benches);
```

## 集成测试

### 端到端测试

测试完整的工作流程：

1. **Git 差异解析**
2. **语言检测和解析器创建**
3. **源文件分析**
4. **语义上下文提取**
5. **代码切片生成**
6. **输出格式化**

### 多语言测试

验证多语言架构的正确性：

```rust
#[test]
fn test_multi_language_workflow() {
    let files = vec![
        "src/main.go",
        "src/lib.rs",      // 未来支持
        "src/app.ts",      // 未来支持
    ];
    
    for file_path in files {
        // 自动检测语言并处理
        if let Some(language) = ParserFactory::detect_language(path) {
            let mut analyzer = SourceAnalyzer::new_for_language(language)?;
            let source_file = analyzer.analyze_file(path)?;
            // 验证处理结果
        }
    }
}
```

## 测试最佳实践

### 1. 测试命名

- 使用描述性的测试名称
- 遵循 `test_<功能>_<场景>` 模式
- 例如：`test_parser_cache_hit_performance`

### 2. 测试组织

- 将相关测试分组到同一个文件
- 使用模块组织复杂的测试套件
- 提供清晰的测试文档

### 3. 测试数据

- 使用真实的代码示例
- 覆盖边界情况和错误条件
- 提供可重现的测试环境

### 4. 性能测试

- 设置合理的性能目标
- 使用统计学方法验证结果
- 监控性能回归

### 5. 错误处理测试

- 测试各种错误条件
- 验证错误恢复机制
- 确保优雅降级

## 持续集成

### CI 配置

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: cargo test --all
      - name: Run benchmarks
        run: cargo bench --all
```

### 测试覆盖率

```bash
# 安装 tarpaulin
cargo install cargo-tarpaulin

# 生成覆盖率报告
cargo tarpaulin --out Html
```

## 调试测试

### 测试调试技巧

1. **使用 println! 调试**
   ```rust
   #[test]
   fn test_debug_example() {
       println!("Debug info: {:?}", value);
       // 使用 cargo test -- --nocapture 查看输出
   }
   ```

2. **使用 dbg! 宏**
   ```rust
   let result = dbg!(complex_calculation());
   ```

3. **条件编译调试代码**
   ```rust
   #[cfg(test)]
   fn debug_helper() {
       // 仅在测试时编译
   }
   ```

### 测试失败分析

- 检查测试输出和错误信息
- 使用 `--verbose` 标志获取详细信息
- 分析性能测试的统计数据
- 检查内存使用和泄漏

## 总结

semantic-diff 的测试策略确保了：

1. **功能正确性** - 通过单元测试和集成测试
2. **性能表现** - 通过性能测试和基准测试
3. **可扩展性** - 通过大规模测试和压力测试
4. **代码质量** - 通过全面的测试覆盖
5. **回归预防** - 通过持续集成和自动化测试

这种多层次的测试方法确保了系统的稳定性和可靠性，为未来的功能扩展提供了坚实的基础。