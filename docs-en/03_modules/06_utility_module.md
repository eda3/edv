# edv - Utility Module Implementation

This document provides detailed implementation guidelines for the Utility module of the edv application.

## Overview

The Utility module provides common functionality and shared utilities used across the edv application. It serves as a foundation for many core operations, handling time code parsing and formatting, and various other general-purpose tasks that support the application's functionality.

## Structure

```
src/utility/
├── mod.rs      // Module exports
└── time.rs     // Time handling utilities
```

## Key Components

### Time Handling (time.rs)

The time handling utilities provide robust time position and duration manipulation:

```rust
/// A duration of time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Duration {
    /// Duration in seconds.
    seconds: f64,
}

impl Duration {
    /// Creates a new duration from seconds.
    #[must_use]
    pub fn from_seconds(seconds: f64) -> Self {
            Self { seconds }
        }

    /// Creates a new duration from milliseconds.
    #[must_use]
    pub fn from_millis(ms: f64) -> Self {
        Self {
            seconds: ms / 1000.0,
        }
    }
    
    /// Creates a new duration from frames at a given frame rate.
    #[must_use]
    pub fn from_frames(frames: f64, fps: f64) -> Self {
        Self {
            seconds: frames / fps,
        }
    }
    
    /// Creates a zero duration.
    #[must_use]
    pub fn zero() -> Self {
        Self { seconds: 0.0 }
    }

    /// Gets the duration in seconds.
    #[must_use]
    pub fn as_seconds(&self) -> f64 {
        self.seconds
    }
    
    /// Gets the duration in milliseconds.
    #[must_use]
    pub fn as_millis(&self) -> f64 {
        self.seconds * 1000.0
    }
    
    /// Gets the duration in frames at a given frame rate.
    #[must_use]
    pub fn as_frames(&self, fps: f64) -> f64 {
        self.seconds * fps
    }

    /// Gets the whole number of frames at a given frame rate.
    #[must_use]
    pub fn frames(&self, fps: f64) -> u64 {
        ((self.seconds * fps).floor().max(0.0)) as u64
    }

    /// Converts the duration to a timecode string.
    #[must_use]
    pub fn to_timecode(&self, fps: f64) -> String {
        let total_seconds = (self.seconds.max(0.0)) as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let frames = self.frames(fps) % (fps.max(0.0).floor() as u64);

        format!("{hours:02}:{minutes:02}:{seconds:02}:{frames:02}")
    }
}

/// A position in time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimePosition {
    /// Position in seconds.
    seconds: f64,
}

impl TimePosition {
    /// Creates a new time position from seconds.
    #[must_use]
    pub fn from_seconds(seconds: f64) -> Self {
        Self {
            seconds: seconds.max(0.0),
        }
    }
    
    /// Creates a new time position from milliseconds.
    #[must_use]
    pub fn from_millis(ms: f64) -> Self {
        Self {
            seconds: (ms / 1000.0).max(0.0),
        }
    }
    
    /// Creates a new time position from frames at a given frame rate.
    #[must_use]
    pub fn from_frames(frames: f64, fps: f64) -> Self {
        Self {
            seconds: (frames / fps).max(0.0),
        }
    }
    
    /// Creates a zero time position.
    #[must_use]
    pub fn zero() -> Self {
        Self { seconds: 0.0 }
    }
    
    /// Gets the time position in seconds.
    #[must_use]
    pub fn as_seconds(&self) -> f64 {
        self.seconds
    }
    
    /// Gets the time position in milliseconds.
    #[must_use]
    pub fn as_millis(&self) -> f64 {
        self.seconds * 1000.0
    }

    /// Gets the time position in frames at a given frame rate.
    #[must_use]
    pub fn as_frames(&self, fps: f64) -> f64 {
        self.seconds * fps
    }
    
    /// Converts the time position to a timecode string.
    #[must_use]
    pub fn to_timecode(&self, fps: f64) -> String {
        let total_seconds = (self.seconds.max(0.0)) as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let frames = self.frames(fps) % (fps.max(0.0).floor() as u64);

        format!("{hours:02}:{minutes:02}:{seconds:02}:{frames:02}")
    }

    /// Creates a time position from a string.
    ///
    /// The string can be in one of these formats:
    /// - Seconds: "123.45"
    /// - Hours:Minutes:Seconds: "01:23:45"
    /// - Hours:Minutes:Seconds.Milliseconds: "01:23:45.678"
    pub fn parse(s: &str) -> Result<Self, String> {
        // Try to parse as seconds
        if let Ok(seconds) = s.parse::<f64>() {
            return Ok(Self::from_seconds(seconds));
        }

        // Try to parse as HH:MM:SS or HH:MM:SS.mmm
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 3 {
            let hours = parts[0]
                .parse::<f64>()
                .map_err(|_| format!("Invalid hours: {}", parts[0]))?;
            let minutes = parts[1]
                .parse::<f64>()
                .map_err(|_| format!("Invalid minutes: {}", parts[1]))?;

            // Handle seconds which might have milliseconds
            let seconds_parts: Vec<&str> = parts[2].split('.').collect();
            let seconds = seconds_parts[0]
                .parse::<f64>()
                .map_err(|_| format!("Invalid seconds: {}", seconds_parts[0]))?;

            let millis = if seconds_parts.len() > 1 {
                let ms_str = seconds_parts[1];
                let padding = 3 - ms_str.len(); // Ensure correct scaling for partial ms digits
                let ms_val = ms_str
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid milliseconds: {}", ms_str))?;
                ms_val * 10f64.powi(-(ms_str.len() as i32))
        } else {
                0.0
            };

            let total_seconds = hours * 3600.0 + minutes * 60.0 + seconds + millis;
            return Ok(Self::from_seconds(total_seconds));
        }

        Err(format!("Invalid time format: {}", s))
    }
}
```

## Key Interfaces

### Time Handling Interface

The time utilities provide interfaces for:

- **Time Position Management**: Create, manipulate, and format time positions
- **Duration Handling**: Create, manipulate, and format time durations
- **Time Conversion**: Convert between different time representations (seconds, milliseconds, frames)
- **Time Format Parsing**: Parse time values from different string formats
- **Time Format Display**: Format time values for display, including timecode format

## Operator Implementations

The time types include useful operator implementations:

```rust
// Duration operators
impl Add for Duration {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            seconds: self.seconds + other.seconds,
            }
        }
}

impl Sub for Duration {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            seconds: (self.seconds - other.seconds).max(0.0),
        }
    }
}

// TimePosition operators
impl Add<Duration> for TimePosition {
    type Output = Self;

    fn add(self, other: Duration) -> Self {
        Self {
            seconds: self.seconds + other.as_seconds(),
        }
    }
}

impl Sub<Duration> for TimePosition {
    type Output = Self;

    fn sub(self, other: Duration) -> Self {
        Self {
            seconds: (self.seconds - other.as_seconds()).max(0.0),
            }
        }
    }
    
impl Sub for TimePosition {
    type Output = Duration;
        
    fn sub(self, other: Self) -> Duration {
        if self.seconds <= other.seconds {
            return Duration::zero();
        }

        Duration::from_seconds(self.seconds - other.seconds)
    }
}
```

## Testing Strategy

The Utility module includes comprehensive unit tests:

```rust
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

    #[test]
    fn test_time_position_from_seconds() {
        let t = TimePosition::from_seconds(5.0);
        assert_eq!(t.as_seconds(), 5.0);
    }

    #[test]
    fn test_time_position_sub_position() {
        let t1 = TimePosition::from_seconds(5.0);
        let t2 = TimePosition::from_seconds(3.0);
        assert_eq!((t1 - t2).as_seconds(), 2.0);
    }
}
```

## Performance Considerations

- **Time Calculation Efficiency**: Optimize time calculations for performance
- **Immutable Value Types**: `Duration` and `TimePosition` are designed as immutable value types
- **Zero-Cost Abstractions**: Operations on time values compile to efficient machine code
- **Memory Efficiency**: Small, stack-allocated types with minimal memory footprint

## Implementation Status Update (2024)

### Current Implementation Status: COMPLETE (100%)

The Utility module has been implemented and is providing core functionality to other modules across the edv application. As a foundational module, its implementation was prioritized early in the development process.

| Component | Status | Implementation Level | Notes |
|-----------|--------|----------------------|-------|
| Time Utilities | ✅ Complete | 100% | Full time handling functionality implemented |

### Key Features Implemented

1. **Time Handling**
   - Robust time position representation
   - Duration calculations and manipulations
   - Multiple time format conversions (seconds, milliseconds, frames)
   - Timecode parsing and formatting
   - Mathematical operations on time values

### Integration with Other Modules

The Utility module serves as a foundation for other modules in the application:

1. **Processing Module**: Uses time utilities for media duration calculations
2. **CLI Module**: Uses time utilities for command input parsing and output formatting
3. **Audio Module**: Uses time utilities for precise audio positioning
4. **Subtitle Module**: Uses time utilities for subtitle timing

The Utility module provides a solid foundation for the edv application, with robust time handling capabilities that are used throughout the codebase. Its complete implementation reflects its critical importance to the overall architecture of the application.

### Future Enhancements

While the current implementation is complete and fully functional, there are opportunities for future enhancements:

1. **Extended Time Formats**
   - Support for additional time formats like SMPTE timecodes
   - Drop-frame timecode support
   - Custom time format parsers and formatters

2. **Performance Optimizations**
   - Further optimizations for time-critical operations
   - SIMD optimizations for batch time calculations
   - Cache-friendly time algorithms