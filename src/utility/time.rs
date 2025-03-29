/// Time handling utilities.
///
/// This module provides types and functions for working with time values in
/// a video editing context, such as durations, time positions, and timecodes.
use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// A duration of time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Duration {
    /// Duration in seconds.
    seconds: f64,
}

// Implement Eq manually even though it's not completely accurate for floats
impl Eq for Duration {}

// まずOrdを実装
impl Ord for Duration {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.seconds
            .partial_cmp(&other.seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

// その後、PartialOrdをOrdに基づいて実装
impl PartialOrd for Duration {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Duration {
    /// Creates a new duration from seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The duration in seconds
    #[must_use]
    pub fn from_seconds(seconds: f64) -> Self {
        Self { seconds }
    }

    /// Creates a new duration from milliseconds.
    ///
    /// # Arguments
    ///
    /// * `ms` - The duration in milliseconds
    #[must_use]
    pub fn from_millis(ms: f64) -> Self {
        Self {
            seconds: ms / 1000.0,
        }
    }

    /// Creates a new duration from frames at a given frame rate.
    ///
    /// # Arguments
    ///
    /// * `frames` - The duration in frames
    /// * `fps` - The frame rate in frames per second
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
    ///
    /// # Arguments
    ///
    /// * `fps` - The frame rate in frames per second
    #[must_use]
    pub fn as_frames(&self, fps: f64) -> f64 {
        self.seconds * fps
    }

    /// Gets the whole number of frames at a given frame rate.
    ///
    /// # Arguments
    ///
    /// * `fps` - The frame rate in frames per second
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub fn frames(&self, fps: f64) -> u64 {
        ((self.seconds * fps).floor().max(0.0)) as u64
    }

    /// Converts the duration to a timecode string.
    ///
    /// # Arguments
    ///
    /// * `fps` - The frame rate in frames per second
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub fn to_timecode(&self, fps: f64) -> String {
        let total_seconds = (self.seconds.max(0.0)) as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = self.frames(fps) % (fps.max(0.0).floor() as u64);

        format!("{hours:02}:{minutes:02}:{seconds:02}:{frames:02}")
    }
}

impl Add for Duration {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            seconds: self.seconds + other.seconds,
        }
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, other: Self) {
        self.seconds += other.seconds;
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

impl SubAssign for Duration {
    fn sub_assign(&mut self, other: Self) {
        self.seconds = (self.seconds - other.seconds).max(0.0);
    }
}

impl Default for Duration {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}s", self.seconds)
    }
}

/// A position in time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimePosition {
    /// Position in seconds.
    seconds: f64,
}

// Implement Eq manually even though it's not completely accurate for floats
impl Eq for TimePosition {}

// まずOrdを実装
impl Ord for TimePosition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.seconds
            .partial_cmp(&other.seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

// その後、PartialOrdをOrdに基づいて実装
impl PartialOrd for TimePosition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TimePosition {
    /// Creates a new time position from seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The position in seconds
    #[must_use]
    pub fn from_seconds(seconds: f64) -> Self {
        Self {
            seconds: seconds.max(0.0),
        }
    }

    /// Creates a new time position from milliseconds.
    ///
    /// # Arguments
    ///
    /// * `ms` - The position in milliseconds
    #[must_use]
    pub fn from_millis(ms: f64) -> Self {
        Self {
            seconds: (ms / 1000.0).max(0.0),
        }
    }

    /// Creates a new time position from frames at a given frame rate.
    ///
    /// # Arguments
    ///
    /// * `frames` - The position in frames
    /// * `fps` - The frame rate in frames per second
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
    ///
    /// # Arguments
    ///
    /// * `fps` - The frame rate in frames per second
    #[must_use]
    pub fn as_frames(&self, fps: f64) -> f64 {
        self.seconds * fps
    }

    /// Gets the whole number of frames at a given frame rate.
    ///
    /// # Arguments
    ///
    /// * `fps` - The frame rate in frames per second
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub fn frames(&self, fps: f64) -> u64 {
        ((self.seconds * fps).floor().max(0.0)) as u64
    }

    /// Converts the time position to a timecode string.
    ///
    /// # Arguments
    ///
    /// * `fps` - The frame rate in frames per second
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub fn to_timecode(&self, fps: f64) -> String {
        let total_seconds = (self.seconds.max(0.0)) as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = self.frames(fps) % (fps.max(0.0).floor() as u64);

        format!("{hours:02}:{minutes:02}:{seconds:02}:{frames:02}")
    }

    /// Creates a time position from a string.
    ///
    /// The string can be in one of these formats:
    /// - Seconds: "123.45"
    /// - Hours:Minutes:Seconds: "01:23:45"
    /// - Hours:Minutes:Seconds.Milliseconds: "01:23:45.678"
    ///
    /// # Returns
    ///
    /// Result containing the parsed TimePosition, or an error if parsing failed.
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

impl Add<Duration> for TimePosition {
    type Output = Self;

    fn add(self, other: Duration) -> Self {
        Self {
            seconds: self.seconds + other.as_seconds(),
        }
    }
}

impl AddAssign<Duration> for TimePosition {
    fn add_assign(&mut self, other: Duration) {
        self.seconds += other.as_seconds();
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

impl SubAssign<Duration> for TimePosition {
    fn sub_assign(&mut self, other: Duration) {
        self.seconds = (self.seconds - other.as_seconds()).max(0.0);
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

impl Default for TimePosition {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for TimePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}s", self.seconds)
    }
}

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
    fn test_duration_add() {
        let d1 = Duration::from_seconds(5.0);
        let d2 = Duration::from_seconds(3.0);
        assert_eq!((d1 + d2).as_seconds(), 8.0);
    }

    #[test]
    fn test_duration_sub() {
        let d1 = Duration::from_seconds(5.0);
        let d2 = Duration::from_seconds(3.0);
        assert_eq!((d1 - d2).as_seconds(), 2.0);
    }

    #[test]
    fn test_time_position_from_seconds() {
        let t = TimePosition::from_seconds(5.0);
        assert_eq!(t.as_seconds(), 5.0);
    }

    #[test]
    fn test_time_position_add_duration() {
        let t = TimePosition::from_seconds(5.0);
        let d = Duration::from_seconds(3.0);
        assert_eq!((t + d).as_seconds(), 8.0);
    }

    #[test]
    fn test_time_position_sub_duration() {
        let t = TimePosition::from_seconds(5.0);
        let d = Duration::from_seconds(3.0);
        assert_eq!((t - d).as_seconds(), 2.0);
    }

    #[test]
    fn test_time_position_sub_position() {
        let t1 = TimePosition::from_seconds(5.0);
        let t2 = TimePosition::from_seconds(3.0);
        assert_eq!((t1 - t2).as_seconds(), 2.0);
    }
}
