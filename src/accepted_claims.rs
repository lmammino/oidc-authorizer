use std::collections::HashSet;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct AcceptedClaims(HashSet<String>, String);

impl AcceptedClaims {
    pub fn new(accepted_values: HashSet<String>, claim_name: String) -> Self {
        Self(accepted_values, claim_name)
    }

    pub fn accepted_values(&self) -> Vec<String> {
        self.0.iter().cloned().collect()
    }

    pub fn from_comma_separated_values(comma_separated_values: &str, claim_name: String) -> Self {
        let accepted_values = comma_separated_values
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        Self::new(accepted_values.into_iter().collect(), claim_name)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_initialize_from_comma_separated_values() {
        let accepted_claims = AcceptedClaims::from_comma_separated_values(
            "https://example.com, https://example.org",
            "iss".to_string(),
        );

        assert_eq!(
            accepted_claims,
            AcceptedClaims::new(
                vec![
                    "https://example.com".to_string(),
                    "https://example.org".to_string()
                ]
                .into_iter()
                .collect(),
                "iss".to_string()
            )
        );
    }
}
