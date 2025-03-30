# edv - Architecture Overview

This document provides an overview of the edv application architecture, explaining the main modules and their relationships.

## Module Structure

The edv application is divided into several key modules, each with a specific responsibility:

```mermaid
flowchart TD
    subgraph Core["Core System"]
        CLI["CLI Module"]
        Core["Core Module"]
        Asset["Asset Module"]
        Utility["Utility Module"]
    end
    
    subgraph Media["Media Processing"]
        FFmpeg["FFmpeg Module"]
        Processing["Processing Module"]
        Audio["Audio Module"]
        Subtitle["Subtitle Module"]
    end
    
    subgraph Project["Project Management"]
        Project["Project Module"]
        Rendering["Rendering Module"]
    end
    
    CLI --> Core
    CLI --> Processing
    Core --> Asset
    Core --> Project
    
    Processing --> FFmpeg
    Audio --> FFmpeg
    Subtitle --> Utility
    Project --> Asset
    Project --> Rendering
    
    Rendering --> FFmpeg
    Audio --> Utility
    Asset --> Utility
```

### Core Modules

- **CLI Module**: Provides the command-line interface for the application.
- **Core Module**: Contains core data structures and utilities used throughout the application.
- **Asset Module**: Manages media assets and their metadata.
- **Utility Module**: Contains common utility functions and shared code.

### Media Processing Modules

- **FFmpeg Module**: Integrates with FFmpeg for media processing operations.
- **Processing Module**: Handles video processing operations through FFmpeg.
- **Audio Module**: Manages audio extraction, processing, and replacement.
- **Subtitle Module**: Handles subtitle extraction, editing, and embedding.

### Project Management Modules

- **Project Module**: Manages project data, including timelines and tracks.
- **Rendering Module**: Handles the rendering of projects to output files.

## Module Dependencies

```mermaid
classDiagram
    CLI --> Core: Commands
    CLI --> FFmpeg: Media Info
    CLI --> Processing: Video Ops
    CLI --> Project: Project Ops
    
    Core --> Asset: Manages
    Core --> Project: Defines
    
    Project --> Rendering: Uses
    Project --> Asset: References
    Project --> Audio: Contains
    Project --> Subtitle: Contains
    
    Processing --> FFmpeg: Utilizes
    Audio --> FFmpeg: Uses
    Subtitle --> Utility: Formatting
    
    Asset --> Utility: Time Handling
    Audio --> Utility: Time Handling
    Subtitle --> Utility: Time Handling
    Rendering --> FFmpeg: Output
    
    class CLI {
        Commands
        Arguments
        UI
    }
    
    class Core {
        Data Structures
        Common Types
    }
    
    class Asset {
        Media Files
        Metadata
    }
    
    class Project {
        Timeline
        Tracks
        Clips
    }
    
    class FFmpeg {
        Media Processing
        Command Building
    }
    
    class Processing {
        Video Operations
        Filter Management
    }
    
    class Audio {
        Audio Handling
        Sound Effects
    }
    
    class Subtitle {
        Subtitle Parsing
        Text Formatting
    }
    
    class Rendering {
        Composition
        Output Generation
    }
    
    class Utility {
        Time Utilities
        Format Conversions
    }
```

## Data Flow

```mermaid
flowchart LR
    Input[Input Files] --> CLI
    CLI --> Commands{Commands}
    
    Commands --> |Info| FFmpeg
    Commands --> |Trim/Concat| Processing
    Commands --> |Project| Project
    Commands --> |Audio| Audio
    Commands --> |Subtitle| Subtitle
    
    FFmpeg --> MediaInfo[Media Information]
    Processing --> ProcessedVideo[Processed Video]
    Audio --> ProcessedAudio[Processed Audio]
    Subtitle --> ProcessedSubtitles[Processed Subtitles]
    Project --> |Render| Rendering
    
    Rendering --> |Video Tracks| FFmpeg
    Rendering --> |Audio Tracks| FFmpeg
    Rendering --> |Subtitle Tracks| FFmpeg
    
    FFmpeg --> Output[Output File]
```

## Key Interfaces

1. **Command Line Interface**: Provides user access to application functionality.
2. **FFmpeg Integration**: Abstracts FFmpeg command-line operations for higher-level modules.
3. **Asset Management**: Provides a unified interface for managing media assets.
4. **Project Management**: Handles project configuration, saving, and loading.
5. **Rendering Pipeline**: Facilitates the rendering of projects to output files.

## Future Architecture Enhancements

The following architectural enhancements are planned for future versions:

1. **Plugin System**: Allow for extensibility through custom plugins.
2. **Distributed Processing**: Support for distributed rendering across multiple machines.
3. **GPU Acceleration**: Integration with hardware acceleration for faster processing.
4. **Cloud Storage**: Support for cloud-based asset storage and management.
5. **Web API**: RESTful API for integration with web applications.

## Implementation Status

```mermaid
gantt
    title Implementation Progress
    dateFormat  YYYY-MM-DD
    
    section Core System
    Utility Module       :done, 2024-01-01, 2024-02-01
    Core Module          :done, 2024-01-15, 2024-02-15
    Asset Module         :done, 2024-02-01, 2024-03-01
    CLI Module           :active, 2024-01-15, 2024-04-15
    
    section Media Processing
    FFmpeg Module        :done, 2024-01-01, 2024-03-01
    Processing Module    :active, 2024-02-01, 2024-04-01
    Audio Module         :active, 2024-03-01, 2024-05-01
    Subtitle Module      :active, 2024-03-15, 2024-05-15
    
    section Project Management
    Project Module       :active, 2024-03-01, 2024-05-01
    Rendering Module     :active, 2024-04-01, 2024-06-01
```

The architecture of edv is designed to be modular and extensible, with clear separation of concerns between different parts of the system. This enables easier maintenance, testing, and future enhancements to the application. 