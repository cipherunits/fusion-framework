//! Shared path-param coercion rules for all language bindings.

use serde_json::{Number, Value};

/// Target type for coercing a path/query string parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    String,
    Int,
    Float,
    Bool,
    Auto,
}

/// Coerce a raw path parameter string into a JSON value.
pub fn coerce_param(raw: &str, kind: ParamKind) -> Value {
    match kind {
        ParamKind::String => Value::String(raw.to_string()),
        ParamKind::Int => match raw.parse::<i64>() {
            Ok(n) => Value::Number(Number::from(n)),
            Err(_) => Value::String(raw.to_string()),
        },
        ParamKind::Float => match raw.parse::<f64>() {
            Ok(f) => Number::from_f64(f)
                .map(Value::Number)
                .unwrap_or_else(|| Value::String(raw.to_string())),
            Err(_) => Value::String(raw.to_string()),
        },
        ParamKind::Bool => {
            let lower = raw.to_ascii_lowercase();
            Value::Bool(matches!(lower.as_str(), "1" | "true" | "yes" | "on"))
        }
        ParamKind::Auto => {
            if let Ok(n) = raw.parse::<i64>() {
                return Value::Number(Number::from(n));
            }
            if let Ok(f) = raw.parse::<f64>() {
                if let Some(n) = Number::from_f64(f) {
                    return Value::Number(n);
                }
            }
            let lower = raw.to_ascii_lowercase();
            if lower == "true" {
                return Value::Bool(true);
            }
            if lower == "false" {
                return Value::Bool(false);
            }
            Value::String(raw.to_string())
        }
    }
}

/// Parse a type hint name used by bindings (`"int"`, `"bool"`, …).
pub fn param_kind_from_name(name: &str) -> ParamKind {
    match name.trim().to_ascii_lowercase().as_str() {
        "int" | "i64" | "integer" => ParamKind::Int,
        "float" | "f64" | "number" | "double" => ParamKind::Float,
        "bool" | "boolean" => ParamKind::Bool,
        "str" | "string" => ParamKind::String,
        "auto" | "" => ParamKind::Auto,
        _ => ParamKind::String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coerces_common_kinds() {
        assert_eq!(coerce_param("42", ParamKind::Int), Value::from(42));
        assert_eq!(coerce_param("true", ParamKind::Bool), Value::Bool(true));
        assert_eq!(
            coerce_param("hi", ParamKind::String),
            Value::String("hi".into())
        );
        assert_eq!(coerce_param("7", ParamKind::Auto), Value::from(7));
    }
}
