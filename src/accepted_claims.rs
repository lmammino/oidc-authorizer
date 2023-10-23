use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct AcceptedClaims(Vec<String>, String);

impl AcceptedClaims {
    pub fn new(accepted_values: Vec<String>, claim_name: String) -> Self {
        Self(accepted_values, claim_name)
    }

    pub fn from_comma_separated_values(comma_separated_values: &str, claim_name: String) -> Self {
        let accepted_values = comma_separated_values
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        Self::new(accepted_values, claim_name)
    }

    pub fn is_accepted(&self, claim_value: &str) -> bool {
        self.0.is_empty() || self.0.contains(&claim_value.to_string())
    }

    pub fn assert(&self, claims: &Value) -> Result<(), String> {
        if self.0.is_empty() {
            // if empty do not validate
            return Ok(());
        }

        let claim_value = match claims.get(&self.1) {
            Some(claim_value) => match claim_value.as_str() {
                Some(claim_value) => claim_value,
                None => return Err(format!("Claim '{}' is not a string", self.1)),
            },
            None => return Err(format!("Missing claim '{}'", self.1)),
        };

        match self.is_accepted(claim_value) {
            true => Ok(()),
            false => Err(format!(
                "Unsupported value for claim '{}' (found={}, supported={:?})",
                self.1, claim_value, self.0
            )),
        }
    }
}
