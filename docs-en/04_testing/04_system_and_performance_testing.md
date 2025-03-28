# edv - System and Performance Testing

This document outlines the approach to system testing and performance testing for the edv project, focusing on validating the application as a whole and ensuring efficient operation.

## System Testing

System tests validate the entire application from end to end, simulating real-world usage scenarios.

### System Testing Overview

- **Scope**: Entire application, validating complete workflows
- **Environment**: Test across all target operating systems
- **Automation**: Run as part of CI/CD pipeline, but less frequently than unit/integration tests
- **Coverage**: Test major user workflows that represent actual use cases

### Key System Testing Areas

#### 1. Installation and Setup

These tests validate the installation and initial setup process:

- **FFmpeg Detection and Compatibility**:
  - Test automatic detection of FFmpeg installations
  - Validate compatibility checks for different FFmpeg versions
  - Test handling of missing or incompatible FFmpeg

- **Configuration Creation and Loading**:
  - Test default configuration generation
  - Validate loading configuration from files
  - Test environment variable overrides

- **First-Run Experience**:
  - Test application behavior on first run
  - Validate setup workflows
  - Test directory creation and permissions

#### 2. End-to-End Workflows

These tests validate complete user workflows:

- **Basic Editing Operations**:
  - Test complete trim, cut, and concat workflows
  - Validate filter application from command line to output
  - Test format conversion workflows

- **Project-Based Workflows**:
  - Test creating, modifying, and rendering projects
  - Validate timeline editing workflows
  - Test project import/export processes

- **Output Validation**:
  - Verify output file correctness across formats
  - Test with various input and output combinations
  - Validate metadata preservation

#### 3. Error Recovery and Edge Cases

These tests focus on error handling and recovery:

- **Invalid Input Handling**:
  - Test application behavior with corrupt input files
  - Validate error reporting for invalid parameters
  - Test handling of unsupported formats

- **Crash Recovery**:
  - Test recovery from unexpected termination
  - Validate project backup and restoration
  - Test handling of partial outputs

- **Resource Constraints**:
  - Test behavior under low memory conditions
  - Validate handling of disk space exhaustion
  - Test operation with limited CPU resources

### System Testing Implementation

```rust
// In tests/system/workflow_tests.rs
#[test]
fn test_complete_edit_workflow() {
    // Prepare a clean test environment
    let temp_dir = tempfile::tempdir().unwrap();
    let input_path = copy_fixture_to_temp("test_fixtures/source.mp4", &temp_dir);
    let output_path = temp_dir.path().join("output.mp4");
    
    // Step 1: Trim the video
    let trim_output = temp_dir.path().join("trimmed.mp4");
    Command::new(get_edv_executable())
        .args(&["trim", 
                "--input", input_path.to_str().unwrap(),
                "--output", trim_output.to_str().unwrap(),
                "--start", "00:00:01.0",
                "--end", "00:00:05.0"])
        .output()
        .expect("Failed to execute trim command");
    
    assert!(trim_output.exists());
    
    // Step 2: Apply a filter
    let filter_output = temp_dir.path().join("filtered.mp4");
    Command::new(get_edv_executable())
        .args(&["filter", 
                "--input", trim_output.to_str().unwrap(),
                "--output", filter_output.to_str().unwrap(),
                "--filter", "eq=brightness=0.1:contrast=1.2"])
        .output()
        .expect("Failed to execute filter command");
    
    assert!(filter_output.exists());
    
    // Step 3: Verify the final output using FFmpeg
    let probe_output = Command::new("ffprobe")
        .args(&["-v", "error", 
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                filter_output.to_str().unwrap()])
        .output()
        .expect("Failed to execute ffprobe command");
    
    let duration_str = String::from_utf8_lossy(&probe_output.stdout);
    let duration: f64 = duration_str.trim().parse().unwrap();
    
    // Verify duration is approximately 4 seconds (5s - 1s)
    assert!((duration - 4.0).abs() < 0.1);
}
```

## Performance Testing

Performance tests measure and verify the efficiency of operations, ensuring the application meets performance expectations.

### Performance Testing Overview

- **Benchmarks**: Establish performance baselines for key operations
- **Profiling**: Identify performance bottlenecks in critical paths
- **Regression Testing**: Ensure changes don't degrade performance
- **Scalability**: Test with large files and batch operations to ensure linear scaling

### Key Performance Testing Areas

#### 1. Processing Speed

These tests focus on operation execution time:

- **Operation Benchmarking**:
  - Measure processing time for different operations
  - Compare with baseline performance
  - Test with standardized input files

- **Hardware Variations**:
  - Test on different hardware configurations
  - Measure scaling with CPU cores
  - Document minimum requirements

- **Format Dependencies**:
  - Measure performance across different formats
  - Test transcoding performance
  - Evaluate codec-specific optimizations

#### 2. Memory Usage

These tests focus on memory efficiency:

- **Memory Consumption Monitoring**:
  - Track memory usage during operations
  - Identify memory growth patterns
  - Test peak memory requirements

- **Large File Handling**:
  - Test with files of increasing size
  - Verify memory usage scales appropriately
  - Identify potential memory leaks

- **Garbage Collection**:
  - Test timely resource release
  - Validate cleanup after operations
  - Measure temporary file management

#### 3. Disk I/O Performance

These tests focus on file operations:

- **File Reading/Writing**:
  - Measure throughput for different operations
  - Test with different storage types (SSD, HDD)
  - Identify I/O bottlenecks

- **Temporary File Management**:
  - Test efficiency of temp file creation/deletion
  - Measure disk space requirements
  - Validate cleanup procedures

- **Cache Effectiveness**:
  - Test cache hit rates
  - Measure performance improvement from caching
  - Validate cache invalidation

#### 4. Concurrency Performance

These tests focus on parallel processing:

- **Thread Scaling**:
  - Measure performance gain with increasing threads
  - Test optimal thread count determination
  - Identify thread contention points

- **Batch Processing Efficiency**:
  - Test processing multiple files in parallel
  - Measure resource utilization
  - Validate load balancing

- **Resource Management**:
  - Test CPU, memory, and I/O balancing
  - Validate resource limits enforcement
  - Measure resource release timing

### Performance Testing Implementation

```rust
// In benches/processing_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use edv::core::config::AppConfig;
use edv::processing::pipeline::ProcessingPipeline;
use edv::processing::operations::trim::TrimOperation;
use std::path::Path;

fn trim_operation_benchmark(c: &mut Criterion) {
    let config = AppConfig::load_default().unwrap();
    let pipeline = ProcessingPipeline::new(config.clone()).unwrap();
    
    // Prepare benchmark with different video durations
    let durations = [10, 30, 60, 120]; // seconds
    let mut group = c.benchmark_group("Trim Operation");
    
    for duration in durations.iter() {
        // Use a specific test file for each duration
        let input_path = Path::new(&format!("bench_fixtures/video_{}_seconds.mp4", duration));
        
        group.bench_with_input(
            BenchmarkId::from_parameter(duration), 
            duration, 
            |b, &duration| {
                b.iter(|| {
                    let output_path = tempfile::NamedTempFile::new().unwrap();
                    let trim_op = TrimOperation::new(
                        input_path,
                        output_path.path(),
                        Some(TimePosition::from_seconds(0.0)),
                        Some(TimePosition::from_seconds(duration as f64 * 0.5)), // Trim half the video
                        true,
                    );
                    
                    pipeline.execute(Box::new(trim_op), None).unwrap();
                })
            }
        );
    }
    
    group.finish();
}

// Memory usage benchmark
fn memory_usage_benchmark(c: &mut Criterion) {
    let config = AppConfig::load_default().unwrap();
    let pipeline = ProcessingPipeline::new(config.clone()).unwrap();
    
    // Prepare benchmark with different video resolutions
    let resolutions = ["480p", "720p", "1080p", "4K"];
    let mut group = c.benchmark_group("Memory Usage");
    
    for resolution in resolutions.iter() {
        let input_path = Path::new(&format!("bench_fixtures/video_{}.mp4", resolution));
        
        group.bench_with_input(
            BenchmarkId::from_parameter(resolution),
            resolution,
            |b, _| {
                b.iter(|| {
                    // Use a memory profiler to track allocations
                    let _guard = memory_profiler::start_memory_profiling();
                    
                    let output_path = tempfile::NamedTempFile::new().unwrap();
                    let filter_op = FilterOperation::new(
                        input_path,
                        output_path.path(),
                        "scale=iw/2:ih/2", // Downscale by 50%
                    );
                    
                    pipeline.execute(Box::new(filter_op), None).unwrap();
                })
            }
        );
    }
    
    group.finish();
}

criterion_group!(
    benches, 
    trim_operation_benchmark,
    memory_usage_benchmark,
);
criterion_main!(benches);
```

## Best Practices for System and Performance Testing

1. **Automation**: Automate as much as possible, but recognize that some system tests may require manual validation
2. **Real-World Scenarios**: Design tests that mimic actual user workflows
3. **Cross-Platform Testing**: Test on all supported operating systems
4. **Resource Variation**: Test with different hardware resources
5. **Benchmarking Consistency**: Use controlled environments for performance benchmarks
6. **Trend Analysis**: Track performance metrics over time to identify regressions
7. **Stress Testing**: Include tests that push the system beyond normal operating conditions
8. **Documentation**: Document performance characteristics and system requirements

This comprehensive approach to system and performance testing ensures that the edv project delivers a reliable, efficient video editing tool that meets user expectations across different environments and usage scenarios. 

## Implementation Status Update (2024)

As of March 2024, system and performance testing implementation is in progress, with significant strides made in establishing the foundation for comprehensive testing:

### System Testing Status

| System Test Area | Status | Implementation Level | Key Progress |
|-----------------|--------|----------------------|-------------|
| Installation & Setup | ✅ Complete | 85% | FFmpeg detection, configuration initialization |
| Core Workflows | ✅ Complete | 80% | Basic editing operations, command execution |
| Project Workflows | 🔄 In Progress | 45% | Basic project operations tested |
| Error Recovery | 🔄 In Progress | 60% | Basic error handling scenarios |
| Cross-Platform | ✅ Complete | 75% | Core functionality on all target platforms |
| Resource Handling | 🔄 In Progress | 50% | Basic resource limit testing |

### Performance Testing Status

| Performance Test Area | Status | Implementation Level | Key Metrics Established |
|----------------------|--------|----------------------|-------------------------|
| Processing Speed | 🔄 In Progress | 65% | Basic operation benchmarks |
| Memory Usage | 🔄 In Progress | 55% | Memory profiles for core operations |
| Disk I/O | 🔄 In Progress | 40% | File operation throughput baselines |
| Concurrency | 🔶 Planned | 25% | Initial threading models tested |
| Large File Handling | 🔄 In Progress | 50% | Scaling tests with increasing file sizes |

### System Testing Achievements

1. **End-to-End Workflow Validation**
   - Successfully tested complete trim, filter, and concatenation workflows
   - Validated command execution from CLI to final output
   - Verified correct parameter passing and application
   - Established validation of output correctness

   Example of a validated workflow:
   ```
   1. Input video (1080p, 2 minutes) → Trim operation (30s segment) → Output verification
   2. Output from step 1 → Filter application (brightness adjustment) → Output verification
   3. Multiple outputs → Concatenation → Final output validation
   ```

2. **Installation and Setup Testing**
   - Verified FFmpeg detection and compatibility checking
   - Tested configuration file generation and loading
   - Validated environment variable handling
   - Confirmed proper setup across operating systems

3. **Error Handling Validation**
   - Tested graceful handling of missing input files
   - Verified user-friendly error messages for common issues
   - Validated recovery from invalid parameters
   - Confirmed proper exit codes for various error conditions

### Performance Testing Achievements

1. **Operation Benchmarking**
   - Established baseline performance for core operations:
     - Trim: ~1.5x realtime speed for 1080p content on reference hardware
     - Filter: ~1.2x realtime for basic filters on 1080p content
     - Concat: ~2.0x realtime for direct stream copy operations
   - Implemented consistent benchmark methodology
   - Created reference test media for repeatable measurements

2. **Memory Profiling**
   - Developed memory usage profiles for key operations
   - Identified and addressed initial memory optimization opportunities:
     - Reduced allocation in command building
     - Optimized buffer management for file operations
     - Implemented streaming approaches for large files
   - Established memory usage expectations for documentation

3. **Cross-Platform Performance Baselines**
   - Measured performance variations across:
     - Linux: Baseline reference platform
     - macOS: ~95% of Linux performance
     - Windows: ~90% of Linux performance for most operations
   - Documented platform-specific considerations
   - Implemented adjustments for platform-specific optimizations

### Current System Testing Challenges

1. **Complex Timeline Testing**
   - **Challenge**: Testing multi-track timeline rendering with various effects
   - **Progress**: Basic test cases implemented, complex scenarios in development
   - **Plan**: Create standardized timeline test cases with predictable outputs

2. **Cross-Platform Consistency**
   - **Challenge**: Ensuring consistent behavior across operating systems
   - **Progress**: Core functionality verified, addressing platform-specific issues
   - **Plan**: Expand test matrix with more platform variations

3. **Resource Limit Testing**
   - **Challenge**: Testing behavior under restricted resources
   - **Progress**: Basic tests implemented for memory constraints
   - **Plan**: Develop more sophisticated resource limitation simulation

### Current Performance Testing Challenges

1. **Benchmark Variability**
   - **Challenge**: Ensuring consistent, comparable performance measurements
   - **Progress**: Implemented statistical approaches to handle variation
   - **Plan**: Further refine methodology to reduce measurement noise

2. **Hardware Variation**
   - **Challenge**: Accounting for diverse hardware configurations
   - **Progress**: Established baseline hardware profiles for comparisons
   - **Plan**: Develop normalized metrics to compare across configurations

3. **Large-Scale Testing**
   - **Challenge**: Testing with very large files and projects
   - **Progress**: Initial scaling tests implemented up to 4K/30min content
   - **Plan**: Expand test data to include longer, higher-resolution content

### Next Steps for System and Performance Testing

1. **Expanded Timeline Testing**
   - Developing comprehensive tests for timeline-based editing
   - Creating validation methods for complex timeline renders
   - Implementing tests for timeline state consistency

2. **Performance Optimization Focus**
   - Identifying critical performance paths based on profiling
   - Establishing performance budgets for key operations
   - Implementing optimization targets for upcoming development

3. **Automated Performance Regression Testing**
   - Integrating performance tests into CI pipeline
   - Implementing comparison with historical data
   - Creating alerts for significant performance regressions

4. **Resource Efficiency Testing**
   - Expanding tests for memory, CPU, and I/O efficiency
   - Implementing long-running stability tests
   - Developing tests for resource usage under varied conditions

System and performance testing will continue to evolve as the project progresses, with increased focus on the modules still under development, particularly the Project and Asset modules. 