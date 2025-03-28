# edv - Project Overview

This document outlines the implementation plan for `edv` - a CLI-based video editing tool written in Rust. The implementation will follow the specifications detailed in the design documents and will be carried out in phases to ensure steady progress and maintainable code.

## Key Goals

- Create an efficient CLI-based video editing tool
- Leverage FFmpeg's capabilities through a user-friendly interface
- Support script and pipeline integration
- Enable batch processing and automation
- Provide functionality in resource-limited environments

## Project Scope

The edv project encompasses:

1. A command-line interface for video editing operations
2. Core video processing capabilities using FFmpeg
3. Project management and timeline editing
4. Batch processing capabilities
5. Extensible architecture for future enhancements

## Target Users

- Command-line proficient developers and power users
- Users needing automated video processing pipelines
- Users with resource-constrained environments
- Users requiring batch processing of video files

## Project Structure

The implementation details are further divided into the following documents:

- [Implementation Strategy](02_strategy.md)
- [Implementation Phases](03_phases_overview.md)
- [Development Timeline](04_timeline.md)
- [Implementation Priorities](05_priorities.md)
- [Risk Management](06_risk_management.md)
- [Quality Assurance](07_quality_assurance.md)
- [Next Steps](08_next_steps.md)

For detailed development phases, module implementation details, and testing strategy, refer to the separate documentation sections. 

## Current Status (2024)

The EDV project has made significant progress since its initial planning:

- **Phase 1** (Core Infrastructure) has been fully implemented, establishing the foundation for the project including the CLI framework and FFmpeg integration.

- **Phase 2** (Extended Functionality) is approximately 70% complete, with comprehensive audio processing features and subtitle support already implemented. The remaining work in this phase focuses on timeline data model implementation.

- **Phases 3 & 4** are scheduled for future development once Phase 2 is completed.

For the most up-to-date information on implementation status, refer to the following documents:

- [Implementation Status in Phases Overview](03_phases_overview.md#implementation-status-2024-update)
- [Current Progress in Timeline](04_timeline.md#進捗状況の更新-2023-2024年)
- [Current Status in Priorities](05_priorities.md#現在の進捗状況-current-implementation-status)
- [Updated Next Steps](08_next_steps.md#2023-2024年更新現在までの進捗状況と次のステップ) 