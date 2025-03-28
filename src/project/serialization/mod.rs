/// Project serialization and deserialization support.
///
/// This module provides functionality for saving and loading projects in
/// various formats, including JSON and binary representations.
// Re-export the public items from the json module
pub mod json;
pub use json::{SerializationError, deserialize_project, serialize_project};
