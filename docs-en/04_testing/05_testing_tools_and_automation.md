# edv - Testing Tools and Automation

This document outlines the testing tools, frameworks, and automation approaches planned for the edv project to ensure consistent, reliable testing.

## Testing Tools and Frameworks

### Rust Testing Tools

The edv project leverages Rust's built-in testing capabilities along with several specialized testing tools:

- **Built-in Test Framework**: 
  - Use Rust's built-in `#[test]` attribute for unit and integration tests
  - Utilize test organization features like `#[cfg(test)]` modules
  - Employ standard assertion macros (`assert!`, `assert_eq!`, etc.)

- **Test Runners**: 
  - Use `cargo test` for running unit and integration tests
  - Employ `cargo test -- --nocapture` for debugging test output
  - Use test filters to run specific tests or test groups

- **Mocking**: 
  - Use `mockall` for creating mock objects
  - Create mock implementations of external dependencies
  - Use mock expectations to verify interaction patterns

- **Assertions**: 
  - Use standard Rust assertions for basic checks
  - Employ custom assertion macros for domain-specific validations
  - Implement helper functions for common assertion patterns

- **Code Coverage**: 
  - Use `tarpaulin` for measuring code coverage on Linux
  - Employ `grcov` for coverage on all platforms
  - Set coverage targets and track progress

### Performance Testing Tools

For performance testing, the edv project plans to use specialized benchmarking and profiling tools:

- **Benchmarking**: 
  - Use `criterion` for reliable, statistical benchmarking
  - Create benchmark groups for related operations
  - Implement parametric benchmarks for testing scaling characteristics

- **Profiling**: 
  - Use `flamegraph` for CPU profiling and hot spot identification
  - Employ `perf` on Linux for low-level performance analysis
  - Use sampling profilers to identify performance bottlenecks

- **Memory Analysis**: 
  - Use `valgrind` and `DHAT` for memory profiling on Linux
  - Employ custom memory tracking for allocation patterns
  - Implement memory usage metrics and tracking

- **Continuous Monitoring**: 
  - Track performance metrics across builds in CI
  - Implement regression detection for performance changes
  - Create performance dashboards for trend analysis

### Test Data Management

Effective management of test data is crucial for reliable testing:

- **Test Fixtures**: 
  - Maintain a collection of test video files with known characteristics
  - Store fixtures in a version-controlled location
  - Include metadata about fixtures for targeted selection

- **Generated Content**: 
  - Create synthetic video content for specific test cases
  - Generate standardized test files with controlled properties
  - Use deterministically generated content for reproducible tests

- **Reference Outputs**: 
  - Store reference outputs for comparison
  - Use checksums or other validation mechanisms
  - Include reference outputs in version control

- **Large File Testing**: 
  - Include a subset of larger files for stress testing
  - Store large files outside of version control
  - Implement automated download of test assets when needed

## Test Automation Plan

### Continuous Integration

The planned test automation strategy will integrate with CI/CD pipelines to ensure continuous validation:

- **PR Validation**: 
  - Run unit and fast integration tests for every PR
  - Execute linting and style checks
  - Generate and publish test reports

- **Nightly Builds**: 
  - Run the full test suite including performance tests
  - Generate comprehensive coverage reports
  - Execute system tests across platforms

- **Cross-Platform Testing**: 
  - Test on Linux, macOS, and Windows
  - Validate on different CPU architectures
  - Test with different FFmpeg versions

- **Results Reporting**: 
  - Generate test reports and trend analysis
  - Track coverage over time
  - Monitor performance metrics across builds

### Planned CI Configuration

The following GitHub Actions workflow is planned for implementation:

```yaml
name: Test Suite

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable]

    steps:
    - uses: actions/checkout@v2
    
    - name: Install FFmpeg
      uses: FedericoCarboni/setup-ffmpeg@v2
      
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        profile: minimal
        toolchain: ${{ matrix.rust }}
        override: true
        components: rustfmt, clippy
        
    - name: Check formatting
      uses: actions-rs/cargo@v1
      with:
        command: fmt
        args: --all -- --check
        
    - name: Clippy
      uses: actions-rs/cargo@v1
      with:
        command: clippy
        args: -- -D warnings
        
    - name: Build
      uses: actions-rs/cargo@v1
      with:
        command: build
        
    - name: Run unit tests
      uses: actions-rs/cargo@v1
      with:
        command: test
        args: --lib
        
    - name: Run integration tests
      uses: actions-rs/cargo@v1
      with:
        command: test
        args: --test '*'
```

### Planned Advanced CI Features

In addition to basic testing, the CI system will implement several advanced features:

- **Test Matrix**: 
  - Test multiple OS and Rust versions
  - Test different FFmpeg versions
  - Validate with different compiler configurations

- **Parallelization**: 
  - Run independent tests in parallel
  - Distribute tests across runners
  - Optimize test execution order

- **Caching**: 
  - Cache Rust dependencies between runs
  - Cache build artifacts for faster builds
  - Store test fixtures to avoid repeated downloads

- **Scheduled Testing**: 
  - Run full test suite on a schedule
  - Execute performance benchmarks regularly
  - Test against latest dependencies

### Local Test Automation

For local development, several scripts and tools are planned:

- **Test Helpers**: 
  - Scripts for running common test scenarios
  - Helpers for setting up test environments
  - Tools for analyzing test results

- **Pre-commit Hooks**: 
  - Run unit tests before commits
  - Execute linting and formatting checks
  - Validate commit messages

- **Development Workflow**: 
  - Integration with IDE test runners
  - Watch mode for continuous testing during development
  - Fast feedback loops for TDD workflow

## Test Documentation

### Test Plans

For each major feature, develop a test plan that includes:

- **Test Objectives**: 
  - Define what needs to be validated
  - Set acceptance criteria
  - Establish test scope and boundaries

- **Test Cases**: 
  - Document test inputs and expected outputs
  - Create step-by-step test procedures
  - Identify test data requirements

- **Edge Cases and Error Conditions**: 
  - List boundary conditions to test
  - Document expected error behaviors
  - Identify potential failure modes

- **Performance Expectations**: 
  - Set performance targets
  - Define acceptable performance ranges
  - Document resource requirements

### Test Reports

Generate test reports that include:

- **Test Coverage Metrics**: 
  - Line and branch coverage statistics
  - Uncovered code areas
  - Coverage trends over time

- **Test Results Summary**: 
  - Pass/fail statistics
  - Test execution times
  - Critical issues identified

- **Performance Benchmark Results**: 
  - Performance measurements
  - Comparison to baselines
  - Performance trends

- **Issues and Recommendations**: 
  - Document discovered issues
  - Suggest improvements
  - Prioritize follow-up actions

## Automated Test Execution

The following diagram illustrates the planned automated test execution flow:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│             │     │             │     │             │
│  Unit Tests │────►│ Integration │────►│   System    │
│             │     │    Tests    │     │    Tests    │
│             │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
       │                  │                   │
       │                  │                   │
       ▼                  ▼                   ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│             │     │             │     │             │
│  Coverage   │     │ Performance │     │  Platform   │
│   Reports   │     │ Benchmarks  │     │   Testing   │
│             │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
       │                  │                   │
       │                  │                   │
       └──────────────────┼───────────────────┘
                          │
                          ▼
                    ┌─────────────┐
                    │             │
                    │  Test Report│
                    │ Generation  │
                    │             │
                    └─────────────┘
                          │
                          ▼
                    ┌─────────────┐
                    │             │
                    │  Notification│
                    │   & Alerts  │
                    │             │
                    └─────────────┘
```

## Testing Schedule

The testing activities will be integrated throughout the development lifecycle:

- **Design Phase**: 
  - Develop initial test plans
  - Create testability requirements
  - Design for testing

- **Implementation Phase**: 
  - Write unit tests alongside code (TDD)
  - Execute continuous testing
  - Track test coverage

- **Feature Completion**: 
  - Add integration tests
  - Execute full test suite
  - Validate against requirements

- **Pre-Release**: 
  - Conduct system testing
  - Execute performance benchmarking
  - Perform regression testing

- **Maintenance**: 
  - Continue regression testing
  - Update tests for new features
  - Maintain and improve test suite

This comprehensive approach to testing tools and automation will ensure that the edv project maintains high quality through continuous validation and efficient testing practices. 

## Implementation Status Update (March 2024)

As of March 2024, the implementation of testing tools and automation is in the early stages:

### Testing Tools Implementation Status

| Tool Category | Status | Implementation Level | Tools Deployed |
|---------------|--------|----------------------|----------------|
| Test Framework | 🔄 In Progress | 60% | Rust test, cargo-test |
| Mocking | 🔄 In Progress | 40% | Basic mock implementations |
| Code Coverage | 🟠 Planned | 15% | Initial exploration of tarpaulin |
| Benchmarking | 🟠 Planned | 10% | Planned for criterion implementation |
| Profiling | 🟠 Planned | 5% | Research phase |
| Fuzz Testing | 🟠 Planned | 0% | Not started |
| Test Data Management | 🔄 In Progress | 30% | Basic test fixtures created |

### CI/CD Pipeline Status

| Pipeline Component | Status | Implementation Level | Details |
|-------------------|--------|----------------------|---------|
| Build Verification | 🟠 Planned | 15% | Local build verification only |
| Unit Test Automation | 🟠 Planned | 20% | Manual test runs, planned for CI |
| Integration Test Automation | 🟠 Planned | 10% | Basic structure defined |
| System Test Automation | 🟠 Planned | 5% | Planning phase |
| Performance Regression | 🟠 Planned | 0% | Not started |
| Test Reporting | 🟠 Planned | 5% | Basic concept defined |
| Cross-Platform Testing | 🟠 Planned | 10% | Manual testing on different platforms |

### Current Testing Approach

1. **Manual Testing Process**
   - Currently using manual test runs with `cargo test`
   - Basic unit tests written for core components
   - Manual verification on different platforms
   - Ad-hoc performance testing

2. **Test Data Management**
   - Small collection of test media files
   - Manual verification of outputs
   - Basic test input/output validation

3. **Testing Documentation**
   - Test plans for key components
   - Documentation of testing approach
   - Guidelines for writing tests

### Testing Tool Current Usage

1. **Unit Testing**
   - Using Rust's built-in testing framework
   - Basic assertions for functionality verification
   - Test organization using `#[cfg(test)]` modules
   - Example unit test:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_time_conversion() {
           let tc = Timecode::from_seconds(3600.5);
           assert_eq!(tc.to_string(), "01:00:00.500");
           assert_eq!(tc.to_seconds(), 3600.5);
       }
   }
   ```

2. **Basic Integration Testing**
   - Limited integration tests between modules
   - Manual verification of module interactions
   - Example integration test:

   ```rust
   #[cfg(test)]
   mod integration_tests {
       use crate::cli::commands::TrimCommand;
       use crate::processing::trim::TrimOperation;
       
       #[test]
       fn test_trim_command_to_operation() {
           let cmd = TrimCommand::new();
           cmd.set_input("input.mp4");
           cmd.set_output("output.mp4");
           cmd.set_start("00:00:10");
           cmd.set_end("00:01:00");
           
           let op = cmd.create_operation().unwrap();
           assert_eq!(op.input_path, "input.mp4");
           assert_eq!(op.start_time.to_seconds(), 10.0);
           assert_eq!(op.end_time.to_seconds(), 60.0);
       }
   }
   ```

### Challenges in Testing Implementation

1. **Test Data Management**
   - **Challenge**: Creating and managing test media files
   - **Current Approach**: Small set of manually created test files
   - **Plan**: Develop comprehensive test data management system

2. **Cross-Platform Testing**
   - **Challenge**: Ensuring consistent behavior across platforms
   - **Current Approach**: Manual testing on available platforms
   - **Plan**: Implement CI with platform matrix

3. **FFmpeg Integration Testing**
   - **Challenge**: Testing FFmpeg interactions reliably
   - **Current Approach**: Basic command validation
   - **Plan**: Develop comprehensive FFmpeg mock system

### Testing Automation Roadmap (2024-2025)

1. **Q2 2024: Basic CI Setup**
   - Implement GitHub Actions for basic build verification
   - Set up automated unit test execution
   - Implement code style checking (rustfmt, clippy)

2. **Q3 2024: Enhanced Testing Tools**
   - Implement code coverage tracking
   - Set up mocking framework for external dependencies
   - Develop basic integration test automation

3. **Q4 2024: Performance Testing**
   - Implement benchmark framework
   - Set up performance regression testing
   - Create performance test reporting

4. **Q1 2025: Full Test Automation**
   - Complete CI/CD pipeline implementation
   - Implement cross-platform test matrices
   - Develop comprehensive test reporting

The testing tools and automation infrastructure will continue to evolve, with particular focus on supporting the modules under active development and establishing a solid foundation for future testing needs. 