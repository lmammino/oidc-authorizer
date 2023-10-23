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

    pub fn new_from_comma_separated_values(
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

    pub fn get_principal_id_from_claims(&self, claims: Value) -> String {
        for field in &self.fields {
            if let Some(claim_value) = claims.get(field) {
                return claim_value.to_string();
            }
        }

        self.default_value.clone()
    }
}
