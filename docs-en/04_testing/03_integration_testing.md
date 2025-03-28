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

- **Command Builder API Usage**:
  - Test proper usage of the borrowing-based API
  - Validate command reuse patterns across modules
  - Test interoperability of different modules using the command builder
  - Verify string lifetime management in cross-module scenarios

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

### 5. Subtitle Processing Integration

These tests validate the subtitle processing capabilities:

- **Format Conversion Integration**:
  - Test conversion between different subtitle formats
  - Validate character encoding handling
  - Test style preservation during conversion

- **Editor Integration with Timeline**:
  - Test subtitle editor integration with video timeline
  - Validate synchronization between subtitle and video
  - Test offset and timing adjustment with video reference

- **Subtitle Rendering Pipeline**:
  - Test subtitle burning into video
  - Validate style application in rendered output
  - Test export to different subtitle formats

- **Complex Ownership Scenarios**:
  - Test safe collection iteration in integrated scenarios
  - Validate borrowing patterns when editing multiple subtitle tracks
  - Test complex operations spanning multiple modules and components

## Rust-Specific Integration Testing Considerations

When writing integration tests for a Rust application, there are specific considerations related to ownership and borrowing:

### 1. Cross-Module Ownership Testing

Integration tests should verify that modules correctly interact with each other's ownership models:

```rust
#[test]
fn test_subtitle_editor_integration_with_pipeline() {
    // Setup components from different modules
    let config = AppConfig::load_default().unwrap();
    let mut editor = SubtitleEditor::new();
    let pipeline = ProcessingPipeline::new(config.clone()).unwrap();
    
    // Test cross-module interaction
    editor.load_file("test_fixtures/subtitles.srt").unwrap();
    editor.shift_subtitles(ShiftBuilder::new(1.0));
    
    // Create a composite operation using both modules
    let subtitle_track = editor.get_track().clone();
    let burn_op = SubtitleBurnOperation::new(
        "test_fixtures/video.mp4", 
        "output.mp4",
        subtitle_track
    );
    
    // Execute the operation
    pipeline.execute(Box::new(burn_op), None).unwrap();
    
    // Verify the result
    assert!(Path::new("output.mp4").exists());
}
```

### 2. Clone vs. Reference Strategy Testing

Test scenarios where decisions need to be made between cloning data or passing references:

```rust
#[test]
fn test_asset_reference_vs_clone() {
    // Setup
    let asset_manager = AssetManager::new();
    let asset_id = asset_manager.import_asset("test_fixtures/video.mp4").unwrap();
    
    // Test using asset by reference (more efficient)
    let timeline = Timeline::new();
    let track_id = timeline.add_track(TrackType::Video).unwrap();
    timeline.add_clip_by_reference(track_id, asset_id, 0.0, 5.0).unwrap();
    
    // Test using asset by clone (less coupled)
    let asset = asset_manager.get_asset(asset_id).unwrap().clone();
    let mut standalone_track = Track::new(TrackType::Video);
    standalone_track.add_clip(Clip::new_with_asset(asset, 0.0, 5.0));
    
    // Verify both approaches work correctly
    assert_eq!(timeline.get_tracks().len(), 1);
    assert_eq!(timeline.get_track(track_id).unwrap().get_clips().len(), 1);
    assert_eq!(standalone_track.get_clips().len(), 1);
}
```

### 3. API Pattern Consistency Testing

Integration tests should verify that ownership patterns are consistent across the API:

```rust
#[test]
fn test_api_consistency() {
    // Test with FFmpeg command builder
    let mut ffmpeg_cmd = FFmpegCommand::new(ffmpeg.clone());
    ffmpeg_cmd.input("input.mp4").output("output.mp4");
    
    // Test with subtitle editor
    let mut editor = SubtitleEditor::new();
    editor.add_subtitle(subtitle1).add_subtitle(subtitle2);
    
    // Test with audio processor
    let mut audio_proc = AudioProcessor::new(ffmpeg.clone());
    audio_proc.input("input.mp3").normalize(true).output("output.mp3");
    
    // All these APIs should follow the same pattern:
    // 1. Methods should use &mut self rather than self
    // 2. Methods should return &mut Self for chaining
    // 3. Immutable operations should use &self
}
```

### 4. Temporary Resource Management Testing

Integration tests should verify proper handling of temporary resources:

```rust
#[test]
fn test_temporary_resource_management() {
    // Create a scope to test resource cleanup
    {
        // Create temporary resources
        let temp_dir = TempDir::new("edv_test").unwrap();
        let temp_path = temp_dir.path().join("temp_output.mp4");
        
        // Use temporary resources in operations
        let mut cmd = FFmpegCommand::new(ffmpeg.clone());
        cmd.input("input.mp4")
           .output(&temp_path)
           .execute()
           .unwrap();
        
        // Verify temporary files were created
        assert!(temp_path.exists());
        
        // Resources should be automatically cleaned up when temp_dir goes out of scope
    }
    
    // Verify resources were cleaned up properly
    assert!(!Path::new("edv_test").exists());
}
```

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

## Implementation Status Update (2024)

As of March 2024, integration testing has been implemented for many of the key module interactions in the edv project, with some areas still under development:

### Integration Testing Status

| Integration Area | Test Coverage | Status | Key Test Scenarios |
|------------------|---------------|--------|-------------------|
| CLI → Processing | 85% | ✅ Complete | Command execution flow, parameter passing |
| Processing → FFmpeg | 90% | ✅ Complete | Command generation, output parsing |
| Core → Processing | 80% | ✅ Complete | Configuration application, context passing |
| Audio Processing | 85% | ✅ Complete | Volume adjustment, extraction, fading |
| Subtitle Handling | 80% | ✅ Complete | Format conversion, editing, burning |
| Project → Timeline | 50% | 🔄 In Progress | Clip management, track operations |
| Asset → Project | 40% | 🔄 In Progress | Asset references, metadata access |
| Multi-Module Workflows | 60% | 🔄 In Progress | End-to-end operation chains |

### Major Integration Test Achievements

1. **Command Execution Pipeline**
   - Successfully tested the entire flow from CLI parsing to execution
   - Validated parameter transformation across module boundaries
   - Tested progress reporting from deep operations to CLI display
   - Verified error propagation through multiple layers

   Example of a successful cross-module test:
   ```rust
   #[test]
   fn test_cli_to_processing_integration() {
       // Parse command line arguments
       let args = vec!["edv", "trim", "--input", "test.mp4", "--output", "out.mp4", 
                       "--start", "00:00:10", "--end", "00:00:30"];
       let parsed_command = parse_command_line(&args).unwrap();
       
       // Create execution context
       let context = ExecutionContext::new(config);
       
       // Execute command through the processing pipeline
       let result = execute_command(parsed_command, &context);
       
       // Verify output was created correctly
       assert!(result.is_ok());
       assert!(Path::new("out.mp4").exists());
       
       // Verify output duration
       let output_info = get_media_info("out.mp4");
       assert_eq!(output_info.duration.unwrap().as_seconds(), 20.0);
   }
   ```

2. **Processing-FFmpeg Integration**
   - Established robust tests for FFmpeg command generation
   - Verified handling of various FFmpeg versions and configurations
   - Tested complex filter graph generation and validation
   - Created tests for output parsing and error detection

3. **Audio Module Integration**
   - Tested audio extraction integrated with the processing pipeline
   - Validated volume adjustment operations across modules
   - Tested audio replacement with video synchronization
   - Verified audio fading with multiple curve types

### Focus Areas for Integration Testing Improvement

1. **Project-Timeline Integration**
   - The integration between Project and Timeline modules requires additional tests for:
     - Multi-track timeline operations
     - Timeline rendering to video
     - Project state synchronization during complex operations
     - Timeline-based editing workflows

2. **Asset-Project Integration**
   - The Asset to Project module integration requires more tests for:
     - Asset reference management in projects
     - Asset metadata access patterns
     - Proxy handling in project contexts
     - Asset lifecycle across project operations

3. **End-to-End Workflows**
   - More comprehensive tests are needed for complex, multi-step workflows:
     - Project creation → editing → rendering flows
     - Asset import → processing → timeline integration
     - Batch processing workflows across modules

### Integration Testing Challenges and Solutions

1. **Test Data Management**
   - **Challenge**: Managing test media across integration tests
   - **Solution**: Implemented a centralized test fixture system with standard test media

2. **Consistent Environment**
   - **Challenge**: Ensuring consistent test environments across platforms
   - **Solution**: Created containerized test environments with standardized FFmpeg installations

3. **Cross-Module Mocking**
   - **Challenge**: Creating cohesive mocks across module boundaries
   - **Solution**: Developed a mock registry for coordinating mock behaviors across modules

### Upcoming Integration Testing Initiatives

1. **Expanded Cross-Module Test Fixtures**
   - Creating standardized test scenarios for cross-module testing
   - Implementing fixtures that represent real-world user workflows
   - Building parameterized test generators for varied scenarios

2. **Timeline Rendering Tests**
   - Developing tests for rendering timeline projects to video
   - Implementing validation for rendered outputs
   - Creating fixtures for complex timeline scenarios

3. **Asset Pipeline Integration**
   - Expanding tests for the full asset lifecycle in projects
   - Testing proxy generation and usage in editing contexts
   - Validating metadata extraction and utilization across modules

The integration testing strategy continues to mature alongside the evolving implementation, with particular focus on the modules still under active development. 