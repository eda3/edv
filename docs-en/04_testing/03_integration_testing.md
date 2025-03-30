# edv - Integration Testing Strategy

This document details the integration testing approach for the edv project, which validates the interaction between multiple components and modules.

## Integration Testing Overview

Integration tests verify that different parts of the system work together correctly:

- **Scope**: Multiple interacting modules and subsystems
- **Real Dependencies**: Use actual dependencies where possible
- **Status**: Currently in planning phase, to be implemented in future iterations
- **Target**: Validate interactions between key modules like CLI, Processing, and FFmpeg

## Planned Implementation Structure

### Directory Organization

Integration tests will be organized in a dedicated `tests/` directory:

```
tests/
├── cli_tests.rs        // CLI integration tests
├── processing_tests.rs // Processing pipeline tests
├── project_tests.rs    // Project management tests
├── asset_tests.rs      // Asset handling tests
└── common/             // Shared test utilities
    ├── mod.rs
    ├── test_utils.rs
    └── test_fixtures.rs
```

### Test Implementation Pattern

Integration tests will follow a consistent pattern:

```rust
// Future example in tests/processing_tests.rs
use edv::core::config::AppConfig;
use edv::processing::pipeline::ProcessingPipeline;
use edv::processing::operations::trim::TrimOperation;
use std::path::Path;

#[test]
fn test_trim_video_integration() {
    // Setup: Initialize components with real implementations
    let config = AppConfig::load_default().unwrap();
    let pipeline = ProcessingPipeline::new(config.clone()).unwrap();
    
    // Prepare test data
    let input_path = Path::new("test_fixtures/sample.mp4");
    let output_path = tempfile::NamedTempFile::new().unwrap();
    
    // Create operation
    let trim_op = TrimOperation::new(
        input_path,
        output_path.path(),
        Some(TimePosition::from_seconds(1.0)),
        Some(TimePosition::from_seconds(3.0)),
        true,
    );
    
    // Execute the operation
    let result = pipeline.execute(Box::new(trim_op), None);
    assert!(result.is_ok());
    
    // Verify results
    let output_info = pipeline.get_media_info(output_path.path()).unwrap();
    assert!(output_info.duration.unwrap().to_seconds() - 2.0 < 0.1);
}
```

### Test Fixtures

- The project will maintain a collection of test video files with diverse characteristics
- Files of different formats, codecs, resolutions, and durations will be included
- Test fixtures will be stored in a version-controlled location separate from test code

## Current Integration Testing Status

Currently, integration testing is performed manually during development. Formal integration tests in a dedicated `tests/` directory are planned but not yet implemented. Some integration tests are implemented as part of the module-level tests in the `#[cfg(test)]` blocks within source files.

### Current Integration Testing Approach

Current integration testing is mainly performed through:

1. **In-module integration tests**: Some modules test interactions with other modules in their unit test blocks
2. **Manual testing**: Developers manually test integration points during development
3. **CLI command execution**: Testing end-to-end functionality through CLI command execution

### Interim Integration Testing Examples

Example of testing integration between modules in unit tests:

```rust
// Example of integration test within a unit test block
#[test]
fn test_subtitle_parsing_integration() {
    // Create a temporary subtitle file
    let file = create_temp_srt_file();
    let path = file.path();
    
    // Parse the subtitle file using the parser
    let track = parse_subtitle_file(path, None).unwrap();
    
    // Verify correct parsing
    assert_eq!(track.len(), 3);
    
    // Verify integration with time utilities
    let first = track.get_subtitle_at_index(0).unwrap();
    assert_eq!(first.get_start_time().as_seconds(), 1.0);
    assert_eq!(first.get_end_time().as_seconds(), 4.0);
}
```

## Key Integration Testing Areas Planned

### 1. Command Execution Flow

These tests will validate the end-to-end command execution flow:

- **Command Line to Execution**:
  - Test parsing arguments into commands
  - Validate command initialization with arguments
  - Test end-to-end execution flow

- **Pipeline Integration**:
  - Test interaction between CLI and processing pipeline
  - Validate context creation and passing
  - Test progress reporting from operations to CLI

### 2. FFmpeg Integration

These tests will focus on the interaction with FFmpeg:

- **Command Generation and Execution**:
  - Test generation and execution of FFmpeg commands
  - Validate parameter handling and escaping
  - Test error handling and status reporting

- **Media Information Extraction**:
  - Test parsing FFmpeg output for media information
  - Validate handling of different file formats
  - Test error conditions with invalid media files

### 3. Subtitle Processing Integration

These tests will validate the subtitle processing capabilities:

- **Format Conversion**:
  - Test conversion between different subtitle formats
  - Validate character encoding handling
  - Test synchronization with audio/video content

## Implementation Plan

The integration testing will be implemented in phases:

### Phase 1: Test Infrastructure (Q2 2024)

- Set up the `tests/` directory structure
- Create test utilities and helper functions
- Establish media file test fixtures
- Implement basic test runner configuration

### Phase 2: Core Integration Tests (Q3 2024)

- Implement tests for CLI and FFmpeg integration
- Create tests for processing pipeline and operations
- Test file handling and utility integration

### Phase 3: Advanced Integration Tests (Q4 2024)

- Implement tests for complex workflows
- Create tests for error handling and recovery
- Test performance characteristics of integrated components

## Best Practices for Integration Testing

1. **Focused Scope**: Test specific interactions between components
2. **Real Dependencies**: Use actual dependencies where practical
3. **Isolated Test Cases**: Ensure tests don't interfere with each other
4. **Realistic Data**: Use realistic media files and inputs
5. **Error Handling**: Test error cases and recovery scenarios
6. **Performance Considerations**: Consider resource usage and test duration

## Conclusion

While formal integration testing is not yet fully implemented, the edv project recognizes the importance of validating component interactions. The planned integration testing approach will ensure reliable interactions between modules as the project continues to evolve. 