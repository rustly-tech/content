//! Content model, validation, and tooling for the Rustly `content` repository.
//!
//! # Why the Rust types are the schema
//!
//! The JSON Schema files in `schemas/` are **generated** from the types in
//! [`model`], and CI fails if the committed schemas differ from what the types
//! produce. Hand-maintaining both would guarantee drift, and a schema that
//! disagrees with the validator is worse than no schema at all.
//!
//! Editors and contributors get JSON Schema for completion and inline errors;
//! the Rust types remain the single authority.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod index;
pub mod model;
pub mod validate;

pub use validate::{Diagnostic, Severity, ValidationReport};
