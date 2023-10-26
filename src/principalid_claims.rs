use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct PrincipalIDClaims {
    fields: Vec<String>,
    default_value: String,
}

impl PrincipalIDClaims {
    pub fn new(fields: Vec<String>, default_value: String) -> Self {
        Self {
            fields,
            default_value,
        }
    }

    pub fn from_comma_separated_values(
        comma_separated_values: &str,
        default_value: String,
    ) -> Self {
        let fields = comma_separated_values
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        Self::new(fields, default_value)
    }

    pub fn get_principal_id_from_claims(&self, claims: &Value) -> String {
        for field in &self.fields {
            if let Some(claim_value) = claims.get(field) {
                return claim_value
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| claim_value.to_string());
            }
        }

        self.default_value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_should_get_the_principal_id_from_the_claims() {
        let principal_id_claims =
            PrincipalIDClaims::from_comma_separated_values("foo, bar", "some_default".to_string());
        // first match
        assert_eq!(
            principal_id_claims
                .get_principal_id_from_claims(&json!({"foo": "some_foo", "bar": "some_bar"})),
            "some_foo"
        );
        // second match
        assert_eq!(
            principal_id_claims.get_principal_id_from_claims(&json!({"bar": "some_bar"})),
            "some_bar"
        );
        // if it's not a string, it get's converted to a JSON string
        assert_eq!(
            principal_id_claims.get_principal_id_from_claims(&json!({"bar": {"a": "b"}})),
            "{\"a\":\"b\"}"
        );
    }

    #[test]
    fn it_should_get_fallback_to_the_default_value_if_all_the_expected_claims_are_missing() {
        let principal_id_claims =
            PrincipalIDClaims::from_comma_separated_values("foo", "some_default".to_string());
        assert_eq!(
            principal_id_claims.get_principal_id_from_claims(&json!({"bar": "some_bar"})), // foo is missing
            "some_default"
        );
    }
}
