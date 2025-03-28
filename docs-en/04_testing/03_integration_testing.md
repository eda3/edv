# edv - Integration Testing Strategy

This document details the integration testing approach for the edv project, which validates the interaction between multiple components and modules.

## Integration Testing Overview

Integration tests verify that different parts of the system work together correctly:

- **Scope**: Multiple interacting modules and subsystems
- **Real Dependencies**: Use actual dependencies where possible
- **Location**: Separate `tests/` directory organized by feature area
- **Data**: Test with real video files of various formats and characteristics

## Implementation Structure

### Directory Organization

Integration tests are organized in a dedicated `tests/` directory:

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

Integration tests follow a consistent pattern:

```rust
// In tests/processing_tests.rs
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

- Maintain a collection of test video files with diverse characteristics
- Include files of different formats, codecs, resolutions, and durations
- Store test fixtures in a version-controlled location separate from test code

## Key Integration Testing Areas

### 1. Command Execution Flow

These tests validate the end-to-end command execution flow:

- **Command Line to Execution**:
  - Test parsing arguments into commands
  - Validate command initialization with arguments
  - Test end-to-end execution flow

- **Pipeline Integration**:
  - Test interaction between CLI and processing pipeline
  - Validate context creation and passing
  - Test progress reporting from operations to CLI

- **Output Handling**:
  - Verify correct file generation
  - Test output formatting and display
  - Validate error reporting to users

### 2. FFmpeg Integration

These tests focus on the interaction with FFmpeg:

- **Command Generation**:
  - Test generation of complex FFmpeg commands
  - Validate parameter handling and escaping
  - Test filter graph generation

- **Output Parsing**:
  - Test parsing FFmpeg output for progress
  - Validate error detection in FFmpeg output
  - Test media information extraction

- **Error Handling**:
  - Test handling of FFmpeg executable not found
  - Validate response to FFmpeg crashes
  - Test handling of unsupported operations

### 3. Project Workflows

These tests validate project management functionality:

- **Project Creation and Editing**:
  - Test creating new projects
  - Validate adding and editing project elements
  - Test project modification workflows

- **Timeline Operations**:
  - Test adding tracks and clips
  - Validate clip manipulation operations
  - Test timeline state consistency after operations

- **Undo/Redo Functionality**:
  - Test undo/redo for various operations
  - Validate history state management
  - Test history limits and pruning

### 4. Batch Processing

These tests focus on handling multiple files:

- **Batch Operations**:
  - Test processing multiple files with the same operation
  - Validate batch job specification and parsing
  - Test batch operation progress tracking

- **Parallel Execution**:
  - Test concurrent processing of multiple files
  - Validate resource management during parallel execution
  - Test error handling in parallel operations

- **Progress Tracking**:
  - Test overall progress reporting for batch operations
  - Validate individual operation progress
  - Test cancellation of batch operations

## Integration Testing Guidelines

1. **Focus on Interactions**: Test the interfaces between components
2. **Use Real Dependencies**: Minimize mocking to test actual behavior
3. **Cover Critical Paths**: Prioritize testing of common user workflows
4. **Test Error Handling**: Validate behavior when components fail
5. **Realistic Test Data**: Use representative video files and scenarios
6. **Minimize Setup Code**: Use helper functions for common setup
7. **Clean Up Resources**: Ensure tests clean up temporary files

## Example Integration Test

```rust
#[test]
fn test_project_save_load_workflow() {
    // Setup
    let temp_dir = tempfile::tempdir().unwrap();
    let project_path = temp_dir.path().join("test_project.edv");
    
    // Create a new project
    let mut project = Project::new("Test Project");
    
    // Add a video track
    let track_id = project.timeline.add_track(TrackType::Video).unwrap();
    
    // Import an asset
    let asset_manager = AssetManager::new(Default::default());
    let asset_id = asset_manager.import_asset(Path::new("test_fixtures/sample.mp4")).unwrap();
    
    // Add a clip to the track
    project.timeline.add_clip(
        track_id,
        Clip::new(
            asset_id,
            TimePosition::from_seconds(0.0),
            Duration::from_seconds(5.0),
            TimePosition::from_seconds(0.0),
            TimePosition::from_seconds(5.0),
        )
    ).unwrap();
    
    // Save the project
    let project_manager = ProjectManager::new(Default::default());
    project_manager.save_project(&project, &project_path).unwrap();
    
    // Load the project
    let loaded_project = project_manager.load_project(&project_path).unwrap();
    
    // Verify project was loaded correctly
    assert_eq!(loaded_project.metadata.name, "Test Project");
    assert_eq!(loaded_project.timeline.tracks.len(), 1);
    
    let loaded_track = &loaded_project.timeline.tracks[0];
    assert_eq!(loaded_track.kind, TrackType::Video);
    assert_eq!(loaded_track.clips.len(), 1);
    
    let loaded_clip = &loaded_track.clips[0];
    assert_eq!(loaded_clip.asset_id, asset_id);
    assert_eq!(loaded_clip.duration.to_seconds(), 5.0);
}
```

This comprehensive integration testing strategy ensures that the different components of the edv project work together correctly, validating common workflows and interactions between modules. 