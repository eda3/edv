# edv - Coding Guidelines

This document outlines the coding guidelines and conventions to be followed when contributing to the edv project. Adhering to these guidelines ensures consistency, maintainability, and high quality throughout the codebase.

## Documentation and Comments

- **Write detailed comments in English**: All comments should be written in English and provide comprehensive explanations of the code's purpose, behavior, and implementation details.
- **Document public APIs thoroughly**: Every public function, struct, trait, and module should have documentation comments that explain their purpose, parameters, return values, and usage examples.
- **Use documentation comments (`///`) for public items**: Document public API elements using triple-slash comments.
- **Use regular comments (`//`) for implementation details**: Explain complex algorithms or non-obvious code with regular comments.
- **Keep documentation up to date**: When modifying code, always update the corresponding documentation to reflect the changes.

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