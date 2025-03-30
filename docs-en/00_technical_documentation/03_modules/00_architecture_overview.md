# edv - Module Architecture Overview

This document provides an overview of the module architecture for the edv project.

## 1. Architecture Overview

The edv project follows a modular architecture with clear separation of concerns. The architecture is designed to provide flexibility, maintainability, and extensibility while ensuring high performance for video processing operations.

### 1.1 High-Level Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│                 │     │                 │     │                 │
│   CLI Layer     │────►│   Core Layer    │────►│  Processing     │
│                 │     │                 │     │  Layer          │
│                 │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
         │                      │                       │
         │                      │                       │
         ▼                      ▼                       ▼
┌─────────────────┐            /│\                ┌─────────────────┐
│                 │            / \                │                 │
│  Project Layer  │◄──────────┘   └──────────────►│  Utility Layer  │
│                 │     ┌─────────────────┐       │                 │
│                 │     │                 │       │                 │
└─────────────────┘     │  Audio Layer    │       └─────────────────┘
                        │                 │               │
                        └─────────────────┘               │
                                │                         │
                                ▼                         ▼
                        ┌─────────────────┐     
                        │                 │     
                        │ Subtitle Layer  │     
                        │                 │     
                        │                 │     
                        └─────────────────┘     
```

### 1.2 Design Principles

1. **Separation of Concerns**: Each module has a well-defined responsibility
2. **Dependency Injection**: Components depend on abstractions, not concrete implementations
3. **Interface Stability**: Public interfaces are stable and well-documented
4. **Error Handling**: Comprehensive error handling using Rust's Result type
5. **Performance Awareness**: Performance-critical paths are identified and optimized
6. **Testing**: All modules are designed to be testable in isolation

## 2. Module Structure

The edv project is organized into the following primary modules:

| Module | Description | Key Responsibilities |
|--------|-------------|---------------------|
| CLI | Command-line interface | Command parsing, user interaction, help text |
| Core | Core functionality | Configuration, logging, execution context |
| Processing | Video processing | FFmpeg integration, operation execution |
| Project | Project management | Timeline editing, asset management, clip management, project serialization, rendering |
| Audio | Audio processing | Volume adjustment, extraction, replacement, fading |
| Subtitle | Subtitle processing | Parsing, editing, formatting, styling, timing |
| Utility | Shared utilities | Time code handling, file operations |

### 2.1 Dependencies Between Modules

```
cli → core → processing ┐
 ↓      ↓        ↓      │
 └──► project ◄──┘      │
      ↓    ↑            │
      │    │            │
      │    └── audio ◄──┘
      │         ↓
      │        subtitle
      ↓
    utility
```

## 3. Cross-Cutting Concerns

Cross-cutting concerns that are handled across modules include:

### 3.1 Error Handling

Error handling is implemented using Rust's Result type system:

```rust
// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

// Central error enum
#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("FFmpeg error: {0}")]
    FFmpeg(String),
    
    // Other error variants...
}
```

### 3.2 Logging

Logging is implemented using a custom logging facade:

```rust
// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

// Logger trait
pub trait Logger: Send + Sync {
    fn log(&self, level: LogLevel, message: &str);
    
    // Level-specific method implementations...
}
```

### 3.3 Configuration

Configuration is handled through a central configuration system:

```rust
// Configuration source
pub enum ConfigSource {
    File(PathBuf),
    Environment,
    Defaults,
}

// Configuration manager
pub struct ConfigManager {
    sources: Vec<ConfigSource>,
    current_config: AppConfig,
}
```

## 4. Module Integration

### 4.1 Startup Sequence

1. Initialize CLI and parse arguments
2. Load configuration based on CLI arguments
3. Initialize logging system
4. Create execution context
5. Execute the requested command

### 4.2 Command Execution Flow

1. Validate command arguments
2. Create appropriate operation
3. Initialize processing pipeline
4. Execute operation
5. Handle and report results

### 4.3 Timeline Editing Flow

1. Load or create project
2. Perform timeline operations
3. Update project state
4. Save project

## 5. Extension Points

The edv application provides several extension points for future enhancements:

- Implementation of new operations
- Implementation of custom effects
- Plugin system (to be implemented in later development phases)

## 6. Performance Considerations

- **Memory Usage Optimization**: Efficient methods for processing large files
- **CPU Optimization**: Parallel processing and FFmpeg optimization
- **I/O Optimization**: Optimization of file operations

## 7. Future Enhancements

The module structure is designed to accommodate future enhancements:

1. **GPU Acceleration**: Integration with hardware acceleration
2. **Advanced Filtering**: Support for complex filter graphs
3. **Extended Timeline**: Support for multi-track timeline editing
4. **Batch Processing**: Enhanced support for batch operations
5. **Plugin System**: Support for third-party plugins

This modular architecture provides a solid foundation for building a powerful, efficient, and extensible video editing tool that meets the needs of command-line users while maintaining high performance and reliability. 

## 8. Implementation Status (2024 Update)

The implementation of the module architecture has progressed significantly, with some modules more fully developed than others. Below is the current implementation status of each module:

### 8.1 Module Implementation Status

| Module | Status | Implementation Level | Key Components Completed |
|--------|--------|----------------------|--------------------------|
| CLI | ✅ Complete | 90% | Command parsing, help system, output formatting |
| Core | ✅ Complete | 95% | Configuration, logging, execution context |
| Processing | ✅ Complete | 85% | FFmpeg wrapper, command building, execution |
| Audio | ✅ Complete | 95% | Volume adjustment, extraction, replacement, fading |
| Subtitle | ✅ Complete | 90% | Parsing, editing, formatting, styling, timing |
| Project | ✅ Complete | 85% | Timeline structure, multi-track editing, asset management, edit history, serialization, rendering pipeline |
| Utility | ✅ Complete | 80% | Time code handling, file operations, common utilities |

### 8.2 Implementation Highlights

1. **FFmpeg Integration Success**
   - The FFmpeg wrapper module has been successfully implemented with a clean API
   - Command building with proper parameter sanitization is working well
   - Process execution and monitoring is fully functional

2. **Audio Processing Capabilities**
   - Complete implementation of volume adjustment with dB/percentage-based controls
   - Flexible audio extraction with format selection and quality control
   - Robust audio replacement with synchronization handling
   - Customizable audio fading with various curve options

3. **Subtitle Support**
   - Comprehensive subtitle model implemented with support for multiple subtitle formats (SRT, VTT)
   - Advanced editing capabilities including timing adjustments and text modifications
   - Styling and positioning options for supported formats
   - Subtitle overlapping resolution with multiple strategies

4. **Project Management**
   - Robust timeline model with multi-track support
   - Track dependency management with relationship types
   - Comprehensive edit history with undo/redo capabilities
   - JSON serialization for project persistence
   - Rendering pipeline with FFmpeg integration

5. **Architecture Refinements**
   - Improved error handling throughout the codebase
   - Enhanced ownership model to follow Rust best practices
   - Reference lifetime improvements to address borrowing issues
   - Mutable borrowing optimizations for complex operations

### 8.3 Next Development Priorities

1. **Enhance Audio and Subtitle Capabilities**
   - Add support for additional subtitle formats (ASS/SSA)
   - Implement more advanced audio effects (equalization, normalization, noise reduction)
   - Improve synchronization between audio, video, and subtitles
   - Add batch processing for audio and subtitle operations

2. **Optimize Rendering Pipeline**
   - Enhance FFmpeg integration for better performance
   - Implement parallel processing for rendering tasks
   - Add more output format and codec options
   - Improve progress reporting and cancellation support

3. **Prepare for Advanced Features**
   - Design and implement extension points for video effects and transitions
   - Develop foundation for advanced timeline operations
   - Create infrastructure for plugin support
   - Add support for composition and layering effects

### 8.4 Architecture Validation

The implementation thus far has validated the core architectural decisions:

- The separation of concerns has proven effective for development and testing
- Module boundaries have remained clear and well-defined
- Error handling strategies have worked well across module boundaries
- Performance considerations have been successfully addressed

This update reflects the current state of implementation as of March 2024, with development continuing according to the phased approach outlined in the implementation plan. 