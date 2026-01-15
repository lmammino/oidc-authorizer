use cel_interpreter::{Context, Program, Value};
use jsonwebtoken::Header;
use serde_json::Value as JsonValue;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CelValidationError {
    #[error("Failed to compile CEL expression: {0}")]
    CompileError(String),
    #[error("CEL validation failed: {0}")]
    ExecutionError(String),
    #[error("CEL expression must evaluate to a boolean value")]
    NonBooleanResult,
    #[error("Failed to convert token data to CEL value: {0}")]
    ConversionError(String),
}

#[derive(Debug, Clone, Default)]
pub struct CelValidator {
    program: Option<Arc<Program>>,
    expression: String,
}

impl CelValidator {
    pub fn validate(&self, header: &Header, claims: &JsonValue) -> Result<(), CelValidationError> {
        // Skip validation if no expression configured (permissive default)
        let program = match &self.program {
            Some(p) => p,
            None => return Ok(()),
        };

        // Build CEL context with header and claims
        let mut context = Context::default();

        // Convert header to CEL-compatible value
        let header_json = serde_json::to_value(header)
            .map_err(|e| CelValidationError::ConversionError(e.to_string()))?;
        let header_value = json_to_cel_value(&header_json);
        context
            .add_variable("header", header_value)
            .map_err(|e| CelValidationError::ExecutionError(e.to_string()))?;

        // Convert claims to CEL Value
        let claims_value = json_to_cel_value(claims);
        context
            .add_variable("claims", claims_value)
            .map_err(|e| CelValidationError::ExecutionError(e.to_string()))?;

        // Execute the CEL program
        let result = program
            .execute(&context)
            .map_err(|e| CelValidationError::ExecutionError(e.to_string()))?;

        // Expect boolean result
        match result {
            Value::Bool(true) => Ok(()),
            Value::Bool(false) => Err(CelValidationError::ExecutionError(format!(
                "expression '{}' evaluated to false",
                self.expression
            ))),
            _ => Err(CelValidationError::NonBooleanResult),
        }
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }
}

impl FromStr for CelValidator {
    type Err = CelValidationError;

    fn from_str(expression: &str) -> Result<Self, Self::Err> {
        if expression.trim().is_empty() {
            return Ok(Self {
                program: None,
                expression: String::new(),
            });
        }

        let program = Program::compile(expression)
            .map_err(|e| CelValidationError::CompileError(e.to_string()))?;

        Ok(Self {
            program: Some(Arc::new(program)),
            expression: expression.to_string(),
        })
    }
}

/// Convert a serde_json::Value to a CEL Value
fn json_to_cel_value(json: &JsonValue) -> Value {
    match json {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(b) => Value::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::UInt(u)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        JsonValue::String(s) => Value::String(s.clone().into()),
        JsonValue::Array(arr) => {
            Value::List(arr.iter().map(json_to_cel_value).collect::<Vec<_>>().into())
        }
        JsonValue::Object(obj) => {
            let map: std::collections::HashMap<String, Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_cel_value(v)))
                .collect();
            Value::Map(map.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_should_skip_validation_with_empty_expression() {
        let validator: CelValidator = "".parse().unwrap();
        let header = Header::default();
        let claims = json!({"sub": "user123"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_skip_validation_with_whitespace_expression() {
        let validator: CelValidator = "   ".parse().unwrap();
        let header = Header::default();
        let claims = json!({"sub": "user123"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_pass_when_expression_is_true() {
        let validator: CelValidator = r#"claims.sub != """#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"sub": "user123"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_fail_when_expression_is_false() {
        let validator: CelValidator = r#"claims.sub == """#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"sub": "user123"});
        let result = validator.validate(&header, &claims);
        assert!(result.is_err());
        assert!(matches!(result, Err(CelValidationError::ExecutionError(_))));
    }

    #[test]
    fn it_should_access_header_fields() {
        let validator: CelValidator = r#"header.typ == "JWT""#.parse().unwrap();
        let mut header = Header::default();
        header.typ = Some("JWT".to_string());
        let claims = json!({});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_access_header_alg() {
        let validator: CelValidator = r#"header.alg == "HS256""#.parse().unwrap();
        let header = Header::default(); // default is HS256
        let claims = json!({});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_has_function() {
        let validator: CelValidator = "has(claims.email)".parse().unwrap();
        let header = Header::default();
        let claims = json!({"email": "user@example.com"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_has_function_for_missing_fields() {
        let validator: CelValidator = "!has(claims.email)".parse().unwrap();
        let header = Header::default();
        let claims = json!({"sub": "user123"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_optional_claim_validation() {
        // Pattern: !has(field) || field == expected_value
        let validator: CelValidator =
            r#"!has(claims.acr) || claims.acr == "urn:mfa""#.parse().unwrap();
        let header = Header::default();

        // Case 1: field is missing - should pass
        let claims = json!({"sub": "user123"});
        assert!(validator.validate(&header, &claims).is_ok());

        // Case 2: field is present with correct value - should pass
        let claims = json!({"sub": "user123", "acr": "urn:mfa"});
        assert!(validator.validate(&header, &claims).is_ok());

        // Case 3: field is present with wrong value - should fail
        let claims = json!({"sub": "user123", "acr": "wrong"});
        assert!(validator.validate(&header, &claims).is_err());
    }

    #[test]
    fn it_should_support_string_endswith() {
        let validator: CelValidator = r#"claims.email.endsWith("@example.com")"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"email": "user@example.com"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_string_startswith() {
        let validator: CelValidator = r#"claims.email.startsWith("user")"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"email": "user@example.com"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_string_contains() {
        let validator: CelValidator = r#"claims.email.contains("@")"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"email": "user@example.com"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_string_matches() {
        let validator: CelValidator = r#"claims.email.matches("^[a-z]+@[a-z]+\\.[a-z]+$")"#
            .parse()
            .unwrap();
        let header = Header::default();
        let claims = json!({"email": "user@example.com"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_in_operator_for_lists() {
        let validator: CelValidator = r#""admin" in claims.roles"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"roles": ["user", "admin"]});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_exists_macro() {
        let validator: CelValidator = r#"claims.roles.exists(r, r == "admin")"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"roles": ["user", "admin"]});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_fail_exists_when_no_match() {
        let validator: CelValidator =
            r#"claims.roles.exists(r, r == "superadmin")"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"roles": ["user", "admin"]});
        assert!(validator.validate(&header, &claims).is_err());
    }

    #[test]
    fn it_should_support_all_macro() {
        let validator: CelValidator =
            r#"claims.scopes.all(s, s.startsWith("read:"))"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"scopes": ["read:users", "read:posts"]});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_fail_all_when_not_all_match() {
        let validator: CelValidator =
            r#"claims.scopes.all(s, s.startsWith("read:"))"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"scopes": ["read:users", "write:posts"]});
        assert!(validator.validate(&header, &claims).is_err());
    }

    #[test]
    fn it_should_support_boolean_and() {
        let validator: CelValidator = r#"claims.sub != "" && claims.email_verified == true"#
            .parse()
            .unwrap();
        let header = Header::default();
        let claims = json!({"sub": "user123", "email_verified": true});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_boolean_or() {
        let validator: CelValidator = r#"claims.role == "admin" || claims.role == "superuser""#
            .parse()
            .unwrap();
        let header = Header::default();
        let claims = json!({"role": "superuser"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_support_ternary_operator() {
        // Test basic ternary operator
        let validator: CelValidator = r#"claims.count > 5 ? true : false"#.parse().unwrap();
        let header = Header::default();

        let claims = json!({"count": 10});
        assert!(validator.validate(&header, &claims).is_ok());

        let claims = json!({"count": 3});
        assert!(validator.validate(&header, &claims).is_err());
    }

    #[test]
    fn it_should_handle_audience_as_string() {
        // When aud is a string, we can directly compare
        let validator: CelValidator = r#"claims.aud == "my-client-id""#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"aud": "my-client-id"});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_handle_audience_as_array() {
        // When aud is an array, we can use 'in' operator
        let validator: CelValidator = r#""my-client-id" in claims.aud"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"aud": ["other-client", "my-client-id"]});
        assert!(validator.validate(&header, &claims).is_ok());
    }

    #[test]
    fn it_should_fail_to_compile_invalid_expression() {
        let result: Result<CelValidator, _> = "invalid syntax {{{{".parse();
        assert!(result.is_err());
        assert!(matches!(result, Err(CelValidationError::CompileError(_))));
    }

    #[test]
    fn it_should_fail_if_expression_returns_non_boolean() {
        let validator: CelValidator = r#"claims.sub"#.parse().unwrap();
        let header = Header::default();
        let claims = json!({"sub": "user123"});
        let result = validator.validate(&header, &claims);
        assert!(matches!(result, Err(CelValidationError::NonBooleanResult)));
    }

    #[test]
    fn it_should_return_expression() {
        let validator: CelValidator = r#"claims.sub != """#.parse().unwrap();
        assert_eq!(validator.expression(), r#"claims.sub != """#);
    }

    #[test]
    fn it_should_return_empty_expression_for_default() {
        let validator: CelValidator = Default::default();
        assert_eq!(validator.expression(), "");
    }
}
