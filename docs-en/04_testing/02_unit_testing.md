# edv - Unit Testing Strategy

This document outlines the unit testing approach for the edv project, focusing on testing individual components in isolation.

## Unit Testing Overview

Unit tests form the foundation of the testing pyramid and focus on testing individual components in isolation:

- **Scope**: Individual functions, methods, and structs
- **Isolation**: Test components in isolation where possible
- **Coverage**: Aim for >80% code coverage with unit tests
- **Location**: Co-located with implementation code using Rust's module tests

## Implementation Approach

### Testing Structure

Unit tests in the edv project follow Rust's standard testing approach with tests in the same file as the implementation:

```rust
// In the same file as the implementation
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_function_name() {
        // Test implementation
    }
}
```

### Actual Examples

Here are examples of unit tests from the current codebase:

#### Time Utilities Testing

```rust
// From src/utility/time.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_from_seconds() {
        let d = Duration::from_seconds(5.0);
        assert_eq!(d.as_seconds(), 5.0);
    }

    #[test]
    fn test_duration_from_millis() {
        let d = Duration::from_millis(5000.0);
        assert_eq!(d.as_seconds(), 5.0);
    }

    #[test]
    fn test_duration_from_frames() {
        let d = Duration::from_frames(120.0, 24.0);
        assert_eq!(d.as_seconds(), 5.0);
    }

    #[test]
    fn test_duration_to_timecode() {
        let d = Duration::from_seconds(3661.5); // 1h 1m 1s 12f @ 24fps
        assert_eq!(d.to_timecode(24.0), "01:01:01:12");
    }
}
```

#### CLI Argument Parsing

```rust
// From src/cli/args.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_arg() {
        let args = vec!["command".to_string(), "value".to_string()];

        // Argument exists
        let result = required_arg(&args, 1, "test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "value");

        // Argument missing
        let result = required_arg(&args, 2, "missing");
        assert!(result.is_err());
        match result {
            Err(Error::MissingArgument(name)) => assert_eq!(name, "missing"),
            _ => panic!("Expected MissingArgument error"),
        }
    }

    #[test]
    fn test_parse_required_arg() {
        let args = vec![
            "command".to_string(),
            "123".to_string(),
            "invalid".to_string(),
        ];

        // Valid parsing
        let result: Result<i32> = parse_required_arg(&args, 1, "number");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);

        // Invalid parsing
        let result: Result<i32> = parse_required_arg(&args, 2, "number");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidArgument(_)) => {}
            _ => panic!("Expected InvalidArgument error"),
        }
    }
}
```

#### Audio Processing Tests

```rust
// From src/audio/mod.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_format() {
        assert!(common::is_supported_format("mp3"));
        assert!(common::is_supported_format("MP3"));
        assert!(common::is_supported_format("wav"));
        assert!(!common::is_supported_format("xyz"));
    }

    #[test]
    fn test_db_to_linear() {
        assert!((common::db_to_linear(0.0) - 1.0).abs() < 1e-10);
        assert!((common::db_to_linear(6.0) - 1.9952623149688797).abs() < 1e-10);
        assert!((common::db_to_linear(-6.0) - 0.501187233627272).abs() < 1e-10);
    }
}
```

#### Command Registry Tests

```rust
// From src/cli/commands.rs
#[cfg(test)]
mod tests {
    use super::*;

    /// Mock command for testing
    #[derive(Debug)]
    struct MockCommand {
        name: String,
        description: String,
        usage: String,
    }

    impl Command for MockCommand {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn usage(&self) -> &str {
            &self.usage
        }

        fn execute(&self, _context: &Context, _args: &[String]) -> Result<()> {
            // Mock implementation that does nothing
            Ok(())
        }
    }

    #[test]
    fn test_register_and_get_command() {
        let mut registry = CommandRegistry::new();
        let command = MockCommand::new("test", "Test command", "test --arg value");
        let command_name = command.name().to_string();

        // Register the command
        registry.register(Box::new(command)).unwrap();

        // Verify command is in registry
        assert!(registry.has_command(&command_name));
        assert_eq!(registry.command_count(), 1);

        // Get the command
        let cmd = registry.get(&command_name).unwrap();
        assert_eq!(cmd.name(), "test");
        assert_eq!(cmd.description(), "Test command");
        assert_eq!(cmd.usage(), "test --arg value");
    }
}
```

### Test Data Management

For unit tests:

- Use simple, inline test data when possible
- Keep test files separate if needed
- Use deterministic test data to ensure consistent results

## Key Unit Testing Areas Currently Implemented

### 1. Core/Utility Components

The time utilities and other foundational components have effective unit tests:

- **Time Handling**:
  - Test duration and time position calculations
  - Validate time format conversions
  - Test timecode generation and parsing

- **Error Handling**:
  - Test error propagation and context
  - Validate error formatting

### 2. CLI Module

The CLI module has good test coverage for:

- **Command Argument Parsing**:
  - Test parsing of valid command arguments
  - Validate error handling for invalid arguments
  - Test handling of optional and required parameters

- **Command Registration**:
  - Test command registry mechanisms
  - Validate command lookup and management

### 3. FFmpeg Integration

The FFmpeg integration has tests for:

- **Version Detection**:
  - Test version string parsing
  - Validate version compatibility checks

- **Command Building**:
  - Basic command generation tests
  - Parameter validation

### 4. Audio Processing

The audio module includes tests for:

- **Format Support**:
  - Test audio format detection
  - Validate format support checks

- **Audio Conversion**:
  - Test decibel/linear conversion
  - Validate volume normalization

### 5. Subtitle Processing

The subtitle module includes tests for:

- **Subtitle Parsing**:
  - Test parsing different subtitle formats
  - Validate handling of malformed subtitle files

## Testing Conventions

### Naming Conventions

Unit tests follow a consistent naming pattern:

```
test_<function_name>_<scenario>
```

For example:
- `test_parse_required_arg` - Tests the `parse_required_arg` function
- `test_duration_from_seconds` - Tests the `from_seconds` constructor of `Duration`

### Assertion Patterns

Tests use clear assertion patterns:

- Use `assert!`, `assert_eq!`, and `assert_ne!` for most cases
- Use pattern matching for error validation
- Include helpful error messages for failed assertions

## Implementation Status and Next Steps

### Current Coverage

As of March 2024:

- **Time Utilities**: ~90% coverage
- **CLI Argument Parsing**: ~85% coverage
- **FFmpeg Integration**: ~70% coverage
- **Audio Module**: ~60% coverage
- **Subtitle Module**: ~50% coverage

### Focus Areas for Improvement

1. **FFmpeg Command Builder**:
   - More comprehensive testing of command generation
   - Testing complex filter graph creation
   - Testing error conditions and edge cases

2. **Advanced CLI Features**:
   - Testing complex command composition
   - Testing help generation and display
   - More interactive command testing

3. **Error Recovery**:
   - Testing more error scenarios
   - Validating error message formatting
   - Testing error context and propagation

### Testing Tools and Utilities

The following tools are used for unit testing:

- **Rust's built-in testing framework**: For running tests
- **assert_* macros**: For validating test conditions

## Conclusion

Unit testing forms the foundation of the testing strategy for the edv project. By focusing on thorough testing of individual components, we can ensure that the building blocks of the application are solid, which simplifies integration and system testing. 