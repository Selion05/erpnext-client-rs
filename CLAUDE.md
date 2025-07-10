# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is an async Rust client library for interacting with ERPNext/Frappe via their REST API. It provides a clean interface for CRUD operations on ERPNext doctypes using basic authentication.

## Architecture

The project is structured as a Cargo workspace with two main crates:

- **client/**: Main library crate (`erpnext_client`) containing the HTTP client and API methods
- **macro/**: Procedural macro crate (`erpnext_client_macro`) providing the `#[derive(Fieldnames)]` macro

### Key Components

- `Client`: Main API client with methods for CRUD operations
- `Settings`: Configuration struct holding URL, API key, and secret
- `Fieldnames` trait: Auto-generated trait for listing struct field names (required for filtering)
- `filter.rs`: Contains `Comparator` and `FilterValue` enums for building ERPNext filters

## Common Development Commands

```bash
# Build the entire workspace
cargo build

# Run tests
cargo test

# Check code without building
cargo check

# Run clippy (linting)
cargo clippy

# Format code
cargo fmt

# Build and run examples
cargo run --example <example_name>
```

## API Methods

The client provides these main methods:
- `get_doctype_by_name<T>()`: Fetch a single document by name
- `list_doctype<T>()`: List documents with filtering and pagination
- `insert_doctype<T>()`: Create new documents
- `insert_doctype_with_return<T, R>()`: Create documents and return the response
- `update_doctype<T>()`: Update existing documents

## Environment Variables

The client can be configured using environment variables:
- `ERPNEXT_URL`: Base URL of the ERPNext instance (without trailing slash)
- `ERPNEXT_KEY`: API key for authentication
- `ERPNEXT_SECRET`: API secret for authentication

## Testing and Development

When working with ERPNext API calls, note that:
- All API responses are wrapped in a `data` field
- Error responses contain `exception` fields that need to be handled
- The client uses tracing for logging HTTP requests and responses
- `DoesNotExistError` is handled specially to return `None` instead of failing