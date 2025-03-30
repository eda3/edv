# edv - Asset Functionality (Project Module Integration)

This document has been deprecated as of the March 2024 architecture update.

## Integration Notice

The Asset functionality described in this document has been fully integrated into the Project module. This integration provides a more cohesive approach to managing assets within the context of editing projects.

## Current Implementation

Asset-related types and functionality are now implemented directly in the Project module:

```
src/project/mod.rs       # Contains AssetId, AssetReference, AssetMetadata
```

For the current implementation details of asset management, please refer to:

1. [Project Module Documentation](04_project_module.md) - Includes the complete asset management implementation
2. [Architecture Overview](00_architecture_overview.md) - Describes the updated module architecture

## Key Asset Types in Project Module

The following types are now defined in `src/project/mod.rs`:

- `AssetId`: Unique identifier for assets (UUID wrapper)
- `AssetReference`: Holds the asset path, ID, and metadata
- `AssetMetadata`: Contains metadata such as duration, dimensions, asset type, etc.

## Asset Management in Project

Assets are managed as part of the `Project` struct, which maintains a vector of `AssetReference` objects. The Project implementation provides methods for:

- Adding assets (`add_asset`)
- Retrieving assets (`get_asset`, `get_asset_mut`)
- Removing assets (`remove_asset`)

## Future Development

As part of the Project module, asset management will evolve alongside the project implementation. Future enhancements may include:

- Improved metadata extraction
- Media analysis capabilities
- Automated asset organization
- Asset tagging systems

Please refer to the Project module documentation for the most recent information about asset management functionality. 