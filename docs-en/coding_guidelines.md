# edv - Coding Guidelines

This document outlines the coding guidelines and conventions to be followed when contributing to the edv project. Adhering to these guidelines ensures consistency, maintainability, and high quality throughout the codebase.

## Documentation and Comments

- **Write detailed comments in English**: All comments should be written in English and provide comprehensive explanations of the code's purpose, behavior, and implementation details.
- **Document public APIs thoroughly**: Every public function, struct, trait, and module should have documentation comments that explain their purpose, parameters, return values, and usage examples.
- **Use documentation comments (`///`) for public items**: Document public API elements using triple-slash comments.
- **Use regular comments (`//`) for implementation details**: Explain complex algorithms or non-obvious code with regular comments.
- **Keep documentation up to date**: When modifying code, always update the corresponding documentation to reflect the changes.
- **Enclose specialized terms in backticks**: Always wrap technical terms, type names, field names, and other code elements in backticks.

Example:
```rust
/// Trims a video file to the specified start and end times.
///
/// This function creates a new video file that contains only the portion
/// of the original video between the start and end times. It preserves
/// all video and audio streams, along with their original quality.
///
/// # Arguments
///
/// * `input_path` - Path to the input video file
/// * `output_path` - Path where the trimmed video will be saved
/// * `start_time` - Starting time position for the trim operation
/// * `end_time` - Ending time position for the trim operation
/// * `recompress` - Whether to recompress the video or use stream copying
///
/// # Returns
///
/// A `Result` containing nothing on success, or an `Error` if the operation failed.
///
/// # Examples
///
/// ```
/// let result = trim_video(
///     &Path::new("input.mp4"),
///     &Path::new("output.mp4"),
///     TimePosition::from_seconds(10.0),
///     TimePosition::from_seconds(30.0),
///     false
/// );
/// ```
pub fn trim_video(
    input_path: &Path,
    output_path: &Path,
    start_time: TimePosition,
    end_time: TimePosition,
    recompress: bool,
) -> Result<()> {
    // Implementation details...
}
```

### Documentation Backtick Rules

Always enclose the following elements in backticks when they appear in documentation comments:

- Technical terms and formats (e.g., `FFmpeg`, `WebVTT`, `SubRip`)
- Type names (e.g., `TimePosition`, `Error::RenderError`)
- Parameter and field names (e.g., `end_time`)
- Other code elements (e.g., `Option`, `String`)

Examples:
```rust
/// Converts the video using `FFmpeg` with the specified codec.
/// Formats the subtitle in `WebVTT` format.
/// Returns a `TimePosition` or an error if parsing fails.
/// Uses the `end_time` parameter to determine the duration.
```

### Error and Panic Documentation

- Always include an `# Errors` section in documentation for functions that return `Result`
- Include a `# Panics` section for functions that might panic

## Rust Best Practices

- **Follow the Rust API Guidelines**: Adhere to the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) for creating a consistent and idiomatic API.
- **Use Rust idioms**: Prefer Rust-specific idioms and patterns rather than those from other languages.
- **Leverage the type system**: Use Rust's powerful type system to prevent errors at compile time rather than runtime.
- **Handle errors properly**: Use the `Result` type for operations that can fail, and provide meaningful error messages.
- **Use `Option` for optional values**: Represent optional values with `Option<T>` rather than nullable types or sentinel values.
- **Make illegal states unrepresentable**: Design data structures so that invalid states cannot be represented.
- **Minimize use of unsafe code**: Use `unsafe` only when absolutely necessary and document the safety invariants thoroughly.
- **Implement appropriate traits**: Implement standard traits like `Debug`, `Clone`, `PartialEq`, etc. when applicable.
- **Respect ownership and borrowing rules**: Use references (`&T` and `&mut T`) appropriately and avoid unnecessary cloning.

### String Formatting

Use direct variable embedding in `format!` macro and related macros:

```rust
// Bad practice
format!("Failed to parse: {}", error)
format!("Invalid time format: {}", time_str)

// Good practice
format!("Failed to parse: {error}")
format!("Invalid time format: {time_str}")
```

For debug formatting, place the format specifier after the variable name:

```rust
// Bad practice
format!("Unsupported extension: {:?}", extension)

// Good practice
format!("Unsupported extension: {extension:?}")
```

### Match Patterns

Consolidate duplicate match arms using the pipe operator:

```rust
// Bad practice
match format {
    Format::Srt => "srt",
    Format::Vtt => "vtt",
    Format::WebVtt => "vtt",  // Duplicate logic
    Format::Ass => "ass",
    Format::AdvancedSsa => "ass",  // Duplicate logic
}

// Good practice
match format {
    Format::Srt => "srt",
    Format::Vtt | Format::WebVtt => "vtt",
    Format::Ass | Format::AdvancedSsa => "ass",
}
```

Avoid arms that duplicate the wildcard pattern behavior:

```rust
// Bad practice (Srt arm duplicates wildcard logic)
match format {
    Format::Vtt => SubtitleFormat::WebVtt,
    Format::Srt => SubtitleFormat::Srt,
    _ => SubtitleFormat::Srt,  // Default to SRT
}

// Good practice
match format {
    Format::Vtt => SubtitleFormat::WebVtt,
    _ => SubtitleFormat::Srt,  // Default to SRT
}
```

### Return Value Annotations

Use the `#[must_use]` attribute for functions that return important values:

```rust
// Bad practice
pub fn to_extension(&self) -> &'static str {
    // Implementation
}

// Good practice
#[must_use]
pub fn to_extension(&self) -> &'static str {
    // Implementation
}
```

Particularly important for:
- Functions that create new instances
- Functions that return computation results
- Functions that return error-checking results
- Functions that return iterators

### Modern Rust Syntax

Use `let...else` for error handling patterns:

```rust
// Bad practice
let subtitle = match self.track.get_subtitle(id) {
    Some(s) => s,
    None => return Err(Error::not_found()),
};

// Good practice
let Some(subtitle) = self.track.get_subtitle(id) else {
    return Err(Error::not_found())
};
```

Use method references instead of trivial closures:

```rust
// Bad practice
items.map(|e| e.to_lowercase())

// Good practice
items.map(str::to_lowercase)
```

### Type Conversions

Prefer `From`/`Into` traits over `as` for safe conversions:

```rust
// Bad practice
let f = self.seconds as f64 + (self.milliseconds as f64 / 1000.0);

// Good practice
let f = f64::from(self.seconds) + (f64::from(self.milliseconds) / 1000.0);
```

Use `char` for single-character pattern matching:

```rust
// Bad practice
if line.contains(":") {
    // Implementation
}

// Good practice
if line.contains(':') {
    // Implementation
}
```

## Functional Programming Principles

- **Prefer immutability**: Make variables immutable by default (`let` instead of `let mut`) unless mutation is necessary.
- **Use pure functions**: Write functions that avoid side effects and have outputs determined solely by their inputs.
- **Compose functions**: Build complex operations by composing simpler functions together.
- **Use higher-order functions**: Leverage functions that take functions as parameters or return functions.
- **Embrace pattern matching**: Use pattern matching to handle different cases clearly and exhaustively.
- **Avoid shared mutable state**: Minimize use of shared mutable state to reduce complexity and potential for bugs.
- **Use transformation over mutation**: Transform data into new forms rather than mutating existing data structures.

## Code Structure and Organization

- **Avoid deep nesting of conditional statements**: Limit `if` statements to at most one level of nesting. Use early returns, guard clauses, or separate functions to reduce nesting.
- **Keep functions focused and small**: Each function should have a single, well-defined responsibility.
- **Use iterators instead of loops**: Prefer iterator methods (`map`, `filter`, `fold`, etc.) over explicit loops when processing collections.
- **Organize code by functionality**: Group related functionality into modules with clear responsibilities.
- **Follow the standard module hierarchy**: Use the conventional Rust module structure with `mod.rs` or module files.
- **Separate interface from implementation**: Define clear interfaces through traits and keep implementation details private.

Examples:

Instead of nested if statements:
```rust
// Bad - nested if statements
fn process_value(value: Option<i32>) -> Result<i32> {
    if let Some(v) = value {
        if v > 0 {
            if v < 100 {
                return Ok(v * 2);
            } else {
                return Err(Error::ValueTooLarge);
            }
        } else {
            return Err(Error::ValueNotPositive);
        }
    } else {
        return Err(Error::NoValue);
    }
}

// Good - using early returns
fn process_value(value: Option<i32>) -> Result<i32> {
    let v = value.ok_or(Error::NoValue)?;
    
    if v <= 0 {
        return Err(Error::ValueNotPositive);
    }
    
    if v >= 100 {
        return Err(Error::ValueTooLarge);
    }
    
    Ok(v * 2)
}
```

Instead of explicit loops:
```rust
// Bad - explicit loop
fn sum_even_numbers(numbers: &[i32]) -> i32 {
    let mut sum = 0;
    for &num in numbers {
        if num % 2 == 0 {
            sum += num;
        }
    }
    sum
}

// Good - using iterators
fn sum_even_numbers(numbers: &[i32]) -> i32 {
    numbers.iter()
        .filter(|&&num| num % 2 == 0)
        .sum()
}
```

## Code Quality and Linting

- **Address all clippy warnings**: Run `cargo clippy -- -W clippy::pedantic` and resolve all warnings.
- **Use automated formatting**: Format code using `cargo fmt` to ensure consistent style.
- **Write comprehensive tests**: Include unit tests, integration tests, and documentation tests.
- **Achieve high test coverage**: Aim for high test coverage, especially for critical functionality.
- **Profile performance**: Identify and optimize performance-critical sections of code.
- **Review security implications**: Consider security implications of code, especially with external inputs.

## Performance Considerations

- **Avoid premature optimization**: Write clear, correct code first, then optimize if necessary.
- **Measure before optimizing**: Use benchmarks to identify actual performance bottlenecks.
- **Consider memory usage**: Be mindful of memory allocation patterns, especially for performance-critical code.
- **Use appropriate data structures**: Choose data structures suitable for the operations being performed.
- **Leverage zero-cost abstractions**: Use Rust's zero-cost abstractions to write high-level code without performance penalties.
- **Consider async when appropriate**: Use async/await for I/O-bound operations but be aware of the complexity trade-offs.

## Handling Compiler and Clippy Warnings

The edv project aims to maintain a clean codebase free of warnings. Here are guidelines for handling various warnings:

### Type Conversions and Casting

- **Handle sign loss warnings appropriately**: When casting from signed types (like `f64`) to unsigned types (like `u64`):
  - Add bounds checks using `max(0.0)` to prevent negative values
  - Use `#[allow(clippy::cast_sign_loss)]` only when the conversion is intentional and safe

  ```rust
  // Bad practice - potential sign loss
  let secs = self.seconds as u64;
  
  // Good practice - prevent negative values
  let secs = (self.seconds.floor().max(0.0)) as u64;
  ```

### Trait Implementations

- **Implement `PartialOrd` and `Ord` in the correct order**:
  - Implement `Ord` first, containing the comparison logic
  - Then implement `PartialOrd` based on `Ord`

  ```rust
  // Good practice
  impl Ord for Duration {
      fn cmp(&self, other: &Self) -> std::cmp::Ordering {
          self.seconds.partial_cmp(&other.seconds).unwrap_or(std::cmp::Ordering::Equal)
      }
  }
  
  impl PartialOrd for Duration {
      fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
          Some(self.cmp(other))
      }
  }
  ```

### Collections and Default Values

- **Use `or_default()` instead of `or_insert_with(Collection::new)`**:
  - Prefer `or_default()` when inserting default collection values
  - This applies to `HashMap`, `HashSet`, `Vec`, etc.

  ```rust
  // Bad practice
  self.dependencies
      .entry(source_id)
      .or_insert_with(HashMap::new)
      .insert(target_id, relationship);
  
  // Good practice
  self.dependencies
      .entry(source_id)
      .or_default()
      .insert(target_id, relationship);
  ```

### Closures and Functions

- **Avoid redundant closures**:
  - Pass function pointers directly instead of wrapping them in closures
  - Especially important for error conversions and mapping functions

  ```rust
  // Bad practice
  fs::create_dir_all(&config.cache_dir).map_err(|e| Error::Io(e))?;
  
  // Good practice
  fs::create_dir_all(&config.cache_dir).map_err(Error::Io)?;
  ```

### Option Handling

- **Use `is_some_and()` instead of `map_or()`** for Option checks with boolean predicates:
  - Simplifies common pattern of checking and testing Option values

  ```rust
  // Bad practice
  path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("mp4"))
  
  // Good practice
  path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
  ```

### Default Trait Implementation

- **Implement `Default` trait instead of custom `default()` methods**:
  - Prefer implementing the standard `Default` trait
  - This enables compatibility with standard library functions and macros

  ```rust
  // Bad practice
  impl OutputFormatter {
      pub fn default() -> Self {
          Self {
              colored: true,
              verbose: false,
          }
      }
  }
  
  // Good practice
  impl Default for OutputFormatter {
      fn default() -> Self {
          Self {
              colored: true,
              verbose: false,
          }
      }
  }
  ```

### Handling Unused Code

- **Indicate intentionally unused variables** with underscore prefix:
  - Add `_` before variable names that are intentionally unused

  ```rust
  // Bad practice
  fn synchronize_locked_tracks(&self, source_id: TrackId, target_id: TrackId, timeline: &mut Timeline) -> Result<()> {
      // Implementation placeholder
      Ok(())
  }
  
  // Good practice
  fn synchronize_locked_tracks(&self, _source_id: TrackId, _target_id: TrackId, _timeline: &mut Timeline) -> Result<()> {
      // Implementation placeholder
      Ok(())
  }
  ```

- **Handle dead code warnings appropriately**:
  - For temporary/development code, add `#[allow(dead_code)]` on specific items
  - For project-wide settings, configure in `Cargo.toml`:
    ```toml
    [lints.rust]
    dead_code = "allow"
    ```

### Import Management

- **Only import what you use**:
  - Remove unnecessary imports
  - Use specific imports rather than broad ones
  - Place test-specific imports in the test module, not at the file level

  ```rust
  // Bad practice
  use std::io::{BufReader, BufWriter, Read, Write};
  
  // Good practice - main module
  use std::io::{BufReader, BufWriter};
  
  // In test module
  #[cfg(test)]
  mod tests {
      use super::*;
      use std::io::{Read, Write};
      // Test code...
  }
  ```

## Review Process

All code contributions should undergo a review process that verifies:

1. Adherence to these coding guidelines
2. Correctness of implementation
3. Comprehensive test coverage
4. Appropriate error handling
5. Complete and accurate documentation
6. Consistent code style

## Continuous Integration

The continuous integration pipeline includes checks for:

- Compilation success with `cargo build`
- Test passing with `cargo test`
- No warnings with `cargo clippy -- -W clippy::pedantic`
- Consistent formatting with `cargo fmt --check`
- Documentation generation with `cargo doc`

Following these guidelines will help maintain a high-quality, consistent, and maintainable codebase for the edv project. 