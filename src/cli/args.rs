/// Utilities for parsing and handling command-line arguments.
///
/// This module provides helper functions and types for working with command-line
/// arguments in a type-safe and user-friendly manner. It abstracts common patterns
/// for argument validation, conversion, and error handling.

use std::path::PathBuf;
use std::str::FromStr;
use std::fmt::Display;

use super::{Result, Error};

/// Validates that a required argument is present.
///
/// # Arguments
///
/// * `args` - The argument slice to check
/// * `index` - The index of the required argument
/// * `arg_name` - The name of the argument for error messages
///
/// # Returns
///
/// A reference to the argument string if present, or an error if missing.
///
/// # Errors
///
/// Returns `Error::MissingArgument` if the required argument is not present.
pub fn required_arg(args: &[String], index: usize, arg_name: &str) -> Result<&String> {
    args.get(index).ok_or_else(|| Error::MissingArgument(arg_name.to_string()))
}

/// Parses a required argument to the specified type.
///
/// # Arguments
///
/// * `args` - The argument slice to check
/// * `index` - The index of the required argument
/// * `arg_name` - The name of the argument for error messages
///
/// # Returns
///
/// The parsed value if successful, or an error if the argument is missing or invalid.
///
/// # Errors
///
/// Returns `Error::MissingArgument` if the required argument is not present,
/// or `Error::InvalidArgument` if the argument cannot be parsed to the target type.
///
/// # Examples
///
/// ```
/// let args = vec!["command".to_string(), "123".to_string()];
/// let value: i32 = parse_required_arg(&args, 1, "count")?;
/// assert_eq!(value, 123);
/// ```
pub fn parse_required_arg<T>(args: &[String], index: usize, arg_name: &str) -> Result<T>
where
    T: FromStr,
    T::Err: Display,
{
    let arg_str = required_arg(args, index, arg_name)?;
    arg_str.parse::<T>().map_err(|e| {
        Error::InvalidArgument(format!("{} (expected {}): {}", arg_name, std::any::type_name::<T>(), e))
    })
}

/// Parses an optional argument to the specified type.
///
/// # Arguments
///
/// * `args` - The argument slice to check
/// * `index` - The index of the optional argument
/// * `arg_name` - The name of the argument for error messages
///
/// # Returns
///
/// Some(parsed_value) if the argument is present and valid,
/// None if the argument is not present, or an error if the argument is invalid.
///
/// # Errors
///
/// Returns `Error::InvalidArgument` if the argument is present but cannot be
/// parsed to the target type.
pub fn parse_optional_arg<T>(args: &[String], index: usize, arg_name: &str) -> Result<Option<T>>
where
    T: FromStr,
    T::Err: Display,
{
    match args.get(index) {
        Some(arg_str) => {
            let parsed = arg_str.parse::<T>().map_err(|e| {
                Error::InvalidArgument(format!("{} (expected {}): {}", arg_name, std::any::type_name::<T>(), e))
            })?;
            Ok(Some(parsed))
        },
        None => Ok(None),
    }
}

/// Parses a path argument and validates that it exists.
///
/// # Arguments
///
/// * `args` - The argument slice to check
/// * `index` - The index of the path argument
/// * `arg_name` - The name of the argument for error messages
/// * `must_exist` - Whether the path must exist on the filesystem
///
/// # Returns
///
/// The path if valid, or an error otherwise.
///
/// # Errors
///
/// Returns `Error::MissingArgument` if the required argument is not present,
/// or `Error::InvalidPath` if the path doesn't exist and `must_exist` is true.
pub fn parse_path_arg(args: &[String], index: usize, arg_name: &str, must_exist: bool) -> Result<PathBuf> {
    let path_str = required_arg(args, index, arg_name)?;
    let path = PathBuf::from(path_str);
    
    if must_exist && !path.exists() {
        return Err(Error::InvalidPath(format!("{} does not exist: {}", arg_name, path.display())));
    }
    
    Ok(path)
}

/// Parses a flag from the arguments.
///
/// A flag is a boolean option that is considered true if the specified flag name
/// is present in the arguments.
///
/// # Arguments
///
/// * `args` - The arguments to search in
/// * `flag_name` - The name of the flag to search for (including any prefixes like "--")
///
/// # Returns
///
/// `true` if the flag is present, `false` otherwise.
#[must_use]
pub fn has_flag(args: &[String], flag_name: &str) -> bool {
    args.iter().any(|arg| arg == flag_name)
}

/// Gets the value following a named argument.
///
/// # Arguments
///
/// * `args` - The arguments to search in
/// * `arg_name` - The name of the argument to search for (including any prefixes like "--")
///
/// # Returns
///
/// Some(value) if the argument is found and followed by a value,
/// None if the argument is not found or has no following value.
#[must_use]
pub fn get_named_arg_value<'a>(args: &'a [String], arg_name: &str) -> Option<&'a String> {
    let positions = args.iter().enumerate()
        .filter_map(|(i, arg)| if arg == arg_name { Some(i) } else { None })
        .collect::<Vec<_>>();
    
    // If the argument appears multiple times, use the last occurrence
    if let Some(&pos) = positions.last() {
        args.get(pos + 1)
    } else {
        None
    }
}

/// Extracts key-value pairs from arguments in the format `--key=value`.
///
/// # Arguments
///
/// * `args` - The arguments to process
/// * `prefix` - The prefix that indicates a key-value pair (e.g., "--")
///
/// # Returns
///
/// A vector of (key, value) tuples extracted from the arguments.
#[must_use]
pub fn extract_key_value_pairs(args: &[String], prefix: &str) -> Vec<(String, String)> {
    args.iter()
        .filter_map(|arg| {
            if !arg.starts_with(prefix) {
                return None;
            }
            
            let trimmed = arg.trim_start_matches(prefix);
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            
            if parts.len() != 2 {
                return None;
            }
            
            Some((parts[0].to_string(), parts[1].to_string()))
        })
        .collect()
}

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
        let args = vec!["command".to_string(), "123".to_string(), "invalid".to_string()];
        
        // Valid parsing
        let result: Result<i32> = parse_required_arg(&args, 1, "number");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
        
        // Invalid parsing
        let result: Result<i32> = parse_required_arg(&args, 2, "number");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidArgument(_)) => {},
            _ => panic!("Expected InvalidArgument error"),
        }
        
        // Missing argument
        let result: Result<i32> = parse_required_arg(&args, 3, "number");
        assert!(result.is_err());
        match result {
            Err(Error::MissingArgument(_)) => {},
            _ => panic!("Expected MissingArgument error"),
        }
    }
    
    #[test]
    fn test_parse_optional_arg() {
        let args = vec!["command".to_string(), "123".to_string(), "invalid".to_string()];
        
        // Valid parsing
        let result: Result<Option<i32>> = parse_optional_arg(&args, 1, "number");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(123));
        
        // Invalid parsing
        let result: Result<Option<i32>> = parse_optional_arg(&args, 2, "number");
        assert!(result.is_err());
        
        // Missing argument (not an error for optional args)
        let result: Result<Option<i32>> = parse_optional_arg(&args, 3, "number");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }
    
    #[test]
    fn test_has_flag() {
        let args = vec![
            "command".to_string(),
            "--verbose".to_string(),
            "value".to_string(),
        ];
        
        assert!(has_flag(&args, "--verbose"));
        assert!(!has_flag(&args, "--quiet"));
    }
    
    #[test]
    fn test_get_named_arg_value() {
        let args = vec![
            "command".to_string(),
            "--input".to_string(),
            "file.mp4".to_string(),
            "--output".to_string(),
            "result.mp4".to_string(),
            "--input".to_string(),  // Duplicated argument
            "second.mp4".to_string(),
        ];
        
        // Existing argument
        let result = get_named_arg_value(&args, "--input");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "second.mp4");  // Should get the last occurrence
        
        // Another existing argument
        let result = get_named_arg_value(&args, "--output");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "result.mp4");
        
        // Missing argument
        let result = get_named_arg_value(&args, "--missing");
        assert!(result.is_none());
        
        // Argument at the end with no value
        let args_with_no_value = vec!["command".to_string(), "--flag".to_string()];
        let result = get_named_arg_value(&args_with_no_value, "--flag");
        assert!(result.is_none());
    }
    
    #[test]
    fn test_extract_key_value_pairs() {
        let args = vec![
            "command".to_string(),
            "--option1=value1".to_string(),
            "positional".to_string(),
            "--option2=value2".to_string(),
            "--invalid".to_string(),
        ];
        
        let pairs = extract_key_value_pairs(&args, "--");
        assert_eq!(pairs.len(), 2);
        assert!(pairs.contains(&("option1".to_string(), "value1".to_string())));
        assert!(pairs.contains(&("option2".to_string(), "value2".to_string())));
    }
} 