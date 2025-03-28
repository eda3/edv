# edv - Unit Testing Strategy

This document outlines the unit testing approach for the edv project, focusing on testing individual components in isolation.

## Unit Testing Overview

Unit tests form the foundation of the testing pyramid and focus on testing individual components in isolation:

- **Scope**: Individual functions, methods, and structs
- **Isolation**: Mock dependencies to test components in isolation
- **Coverage**: Aim for >80% code coverage with unit tests
- **Location**: Co-located with implementation code in `src/` directory

## Implementation Approach

### Testing Structure

Unit tests in the edv project follow Rust's standard testing approach:

```rust
// In the same file as the implementation
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::*;
    
    #[test]
    fn test_function_name() {
        // Test implementation
    }
}
```

### Mocking Strategy

For components with external dependencies, we use `mockall` to create mock implementations:

```rust
// Mock FFmpeg wrapper for testing
mock! {
    FfmpegWrapper {
        fn run_command(&self, command: FfmpegCommand, progress: Option<ProgressBar>) -> Result<()>;
        fn get_media_info(&self, path: &Path) -> Result<MediaInfo>;
    }
}
```

### Test Data Management

- Use small, representative test fixtures when needed
- Prefer in-memory data when possible for faster tests
- Use deterministic test data to ensure consistent results

## Key Unit Testing Areas

### 1. Core Module

The Core module tests focus on the central functionality:

- **Configuration Management**:
  - Test loading configuration from different sources
  - Validate configuration constraints and defaults
  - Test error handling for invalid configurations

- **Error Handling**:
  - Ensure error types correctly propagate information
  - Test error conversion and context enrichment
  - Validate error formatting and user-facing messages

- **Execution Context**:
  - Test context creation and resource management
  - Validate temporary directory handling
  - Test context cleanup and resource disposal

### 2. CLI Module

The CLI module tests focus on user interaction:

- **Command Argument Parsing**:
  - Test parsing of valid command arguments
  - Validate error handling for invalid arguments
  - Test handling of optional and required parameters

- **Help Text Generation**:
  - Ensure help text is correctly formatted
  - Test command-specific help generation
  - Validate global help text formatting

- **Command Registration and Execution**:
  - Test command registration mechanisms
  - Validate command lookup and selection
  - Test command execution workflow

### 3. Processing Module

The Processing module tests focus on video operations:

- **FFmpeg Command Generation**:
  - Test building of FFmpeg command lines
  - Validate parameter escaping and formatting
  - Test complex filter graph generation

- **Ownership and Borrowing**:
  - Test method chaining using mutable references
  - Validate proper memory management in long-lived operations
  - Test string lifetime handling in command arguments
  - Ensure cloneable command structures work correctly
  - Verify that multiple commands can be created from templates

- **Operation Validation**:
  - Test validation of operation parameters
  - Ensure invalid parameters are rejected
  - Test boundary conditions and edge cases

- **Execution Plan Creation**:
  - Test execution plan generation
  - Validate step ordering and dependencies
  - Test plan optimization

### 4. Project Module

The Project module tests focus on project management:

- **Timeline Operations**:
  - Test track and clip management
  - Validate timeline editing operations
  - Test timeline state consistency

- **Edit History Management**:
  - Test undo/redo functionality
  - Validate history state management
  - Test history pruning and cleanup

- **Project Serialization/Deserialization**:
  - Test saving and loading projects
  - Validate format compatibility
  - Test error handling during serialization

### 5. Asset Module

The Asset module tests focus on media asset management:

- **Metadata Extraction**:
  - Test media information parsing
  - Validate handling of different file formats
  - Test error handling for corrupt files

- **Asset Management**:
  - Test asset tracking and lookup
  - Validate asset registration and removal
  - Test asset collection operations

- **Proxy Generation**:
  - Test proxy creation workflows
  - Validate proxy quality and size
  - Test proxy management and cleanup

### 6. Utility Module

The Utility module tests focus on shared utilities:

- **Time Code Parsing and Formatting**:
  - Test parsing of different time formats
  - Validate time code conversions
  - Test boundary conditions (e.g., very large times)

- **Filesystem Operations**:
  - Test file and directory operations
  - Validate path handling and normalization
  - Test error handling for filesystem operations

- **Format Detection and Conversion**:
  - Test format identification from file extensions
  - Validate format compatibility checking
  - Test format conversion logic

### 7. Subtitle Module

The Subtitle module tests focus on subtitle handling:

- **Subtitle Parsing**:
  - Test parsing of different subtitle formats (SRT, WebVTT)
  - Validate handling of different time formats
  - Test error handling for malformed subtitle files

- **Subtitle Track Management**:
  - Test adding, removing, and modifying subtitles
  - Validate track state consistency
  - Test subtitle overlap detection and resolution

- **Ownership and Borrowing**:
  - Test proper collection iteration patterns
  - Validate mutable reference handling
  - Test borrowing conflicts resolution strategies
  - Ensure safe mutable and immutable access to subtitles
  - Verify iterator safety with complex operations

- **Subtitle Rendering**:
  - Test subtitle rendering to different formats
  - Validate style application
  - Test time code conversion between formats

## Testing for Rust-Specific Concerns

### Ownership and Reference Testing

When testing Rust code, particular attention should be paid to ownership and borrowing patterns:

#### 1. Method Chaining Tests

```rust
#[test]
fn test_command_builder_method_chaining() {
    let mut cmd = FFmpegCommand::new(ffmpeg);
    
    // Test that method chaining with mutable references works
    cmd.input("input.mp4")
       .output_options(&["-c:v", "libx264"])
       .output("output.mp4");
       
    // Test that we can still use cmd after chaining
    assert_eq!(cmd.inputs.len(), 1);
    assert_eq!(cmd.output.as_ref().unwrap().to_str().unwrap(), "output.mp4");
}
```

#### 2. Temporary Value Lifetime Tests

```rust
#[test]
fn test_string_argument_lifetimes() {
    let mut cmd = FFmpegCommand::new(ffmpeg);
    
    // Test with owned strings in a collection
    let options = vec![
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "128k".to_string()
    ];
    
    cmd.output_options(&options);
    
    // Verify the options were captured correctly
    assert!(cmd.output_options.contains(&"aac".to_string()));
}
```

#### 3. Mutable Borrowing Tests

```rust
#[test]
fn test_subtitle_track_borrowing() {
    let mut track = SubtitleTrack::new();
    
    // Add some subtitles
    track.add_subtitle(create_test_subtitle("1", 0.0, 2.0));
    track.add_subtitle(create_test_subtitle("2", 3.0, 5.0));
    
    // Test safe iteration patterns
    let ids = track.get_subtitle_ids();
    assert_eq!(ids.len(), 2);
    
    // Test mutable access after collecting IDs
    for id in &ids {
        if let Some(subtitle) = track.get_subtitle_mut(id) {
            subtitle.set_text("Modified");
        }
    }
    
    // Verify modifications
    for id in &ids {
        assert_eq!(track.get_subtitle(id).unwrap().get_text(), "Modified");
    }
}
```

#### 4. Clone and Template Tests

```rust
#[test]
fn test_command_template_pattern() {
    let template = FFmpegCommand::new(ffmpeg)
        .input("input.mp4");
    
    // Create different commands from the same template
    let mut cmd1 = template.clone();
    cmd1.output("output1.mp4");
    
    let mut cmd2 = template.clone();
    cmd2.output("output2.mp4");
    
    // Verify they have the same input but different outputs
    assert_eq!(cmd1.inputs.len(), 1);
    assert_eq!(cmd2.inputs.len(), 1);
    assert_eq!(cmd1.output.as_ref().unwrap().to_str().unwrap(), "output1.mp4");
    assert_eq!(cmd2.output.as_ref().unwrap().to_str().unwrap(), "output2.mp4");
}
```

These tests help ensure that the code correctly handles Rust's ownership and borrowing rules, preventing bugs that might only manifest at runtime in less strict languages.

## Best Practices for Unit Testing

1. **Test One Thing Per Test**: Each test should verify a single behavior
2. **Descriptive Test Names**: Name tests clearly to describe what they're testing
3. **Arrange-Act-Assert**: Structure tests with clear setup, action, and verification
4. **Minimize Test Dependencies**: Tests should not depend on external state
5. **Test Edge Cases**: Include tests for boundary conditions and error paths
6. **Keep Tests Fast**: Unit tests should execute quickly for rapid feedback
7. **Avoid Test Logic**: Minimize conditional logic in test code
8. **Test Public Interfaces**: Focus on testing the public API of components

## Example Unit Test

```rust
#[test]
fn test_trim_operation_validation() {
    // Arrange: Set up test data
    let input_path = Path::new("test_input.mp4");
    let output_path = Path::new("test_output.mp4");
    
    // Test case 1: Invalid start time (negative)
    let op = TrimOperation::new(
        input_path, 
        output_path,
        Some(TimePosition::from_seconds(-10.0)),
        Some(TimePosition::from_seconds(30.0)),
        false,
    );
    
    // Act & Assert
    assert!(op.validate().is_err());
    
    // Test case 2: Valid parameters
    let op = TrimOperation::new(
        input_path, 
        output_path,
        Some(TimePosition::from_seconds(10.0)),
        Some(TimePosition::from_seconds(30.0)),
        false,
    );
    
    // Act & Assert
    assert!(op.validate().is_ok());
}
```

This comprehensive unit testing approach ensures that each component of the edv project functions correctly in isolation, providing a solid foundation for higher-level testing. 