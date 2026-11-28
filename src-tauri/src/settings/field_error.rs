//! The error one setting field reports, and the block-shape helpers every
//! validator shares.

use serde::Serialize;
use serde_json::{Map, Value};

/// One bad field of `settings.json`. `field` is the JSON key path relative to
/// the file (`gitlabFilter.projects.include`), `index` the position in a list.
/// `message` is Polish and, being `&'static` at construction, never carries the
/// offending value; neither does `field`, which is built from known keys only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    pub field: String,
    pub index: Option<usize>,
    pub message: String,
}

impl FieldError {
    pub(super) fn new(field: impl Into<String>, message: &'static str) -> Self {
        FieldError {
            field: field.into(),
            index: None,
            message: message.to_string(),
        }
    }

    /// An error on one entry of a list field.
    pub(super) fn at(field: impl Into<String>, index: usize, message: &'static str) -> Self {
        FieldError {
            index: Some(index),
            ..FieldError::new(field, message)
        }
    }
}

pub(super) const NOT_TRUE_OR_FALSE: &str = "Wartość musi być true albo false.";

/// `value` as an object holding no keys but `keys`. Only an object is valid;
/// the key names of the file are never echoed.
pub(super) fn object_with_keys<'a>(
    value: &'a Value,
    field: &str,
    keys: &[&str],
) -> Result<&'a Map<String, Value>, FieldError> {
    let Some(map) = value.as_object() else {
        return Err(FieldError::new(field, "Oczekiwano obiektu."));
    };
    if map.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err(FieldError::new(field, "Nieznany klucz."));
    }
    Ok(map)
}

/// The value under `key`; `null` counts as absent.
pub(super) fn present<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    map.get(key).filter(|v| !v.is_null())
}

/// `value`, unless any field reported an error.
pub(super) fn checked<T>(value: T, errors: Vec<FieldError>) -> Result<T, Vec<FieldError>> {
    if errors.is_empty() {
        Ok(value)
    } else {
        Err(errors)
    }
}
