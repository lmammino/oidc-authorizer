use serde_json::Value;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
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
                "Unsupported value for claim '{}' (found='{}', supported={:?})",
                self.1, claim_value, self.0
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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
                ],
                "iss".to_string()
            )
        );
    }

    #[test]
    fn it_should_accept_expected_claims_and_reject_others() {
        let accepted_claims = AcceptedClaims::from_comma_separated_values(
            "https://example.com, https://example.org",
            "iss".to_string(),
        );

        assert!(accepted_claims.is_accepted("https://example.com"));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.com"}))
            .is_ok());
        assert!(accepted_claims.is_accepted("https://example.org"));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.org"}))
            .is_ok());
        assert!(!accepted_claims.is_accepted("https://example.net"));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.net"}))
            .is_err());
    }

    #[test]
    fn it_should_accept_everything_when_using_an_empty_list() {
        let accepted_claims = AcceptedClaims::from_comma_separated_values("", "iss".to_string());

        assert!(accepted_claims.is_accepted("https://example.com"));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.com"}))
            .is_ok());
        assert!(accepted_claims.is_accepted("https://example.org"));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.org"}))
            .is_ok());
        assert!(accepted_claims.is_accepted("https://example.net"));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.net"}))
            .is_ok());
        // it should also accept tokens with the missing claim
        assert!(accepted_claims
            .assert(&json!({"some_other_claim": "some_value"}))
            .is_ok());
    }

    #[test]
    fn it_should_reject_if_the_claim_is_missing() {
        let accepted_claims = AcceptedClaims::from_comma_separated_values(
            "https://example.com, https://example.org",
            "iss".to_string(),
        );

        let result = accepted_claims.assert(&json!({
            "some_other_claim": "some_value"
        }));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing claim 'iss'".to_string());
    }

    #[test]
    fn it_should_reject_if_the_claim_is_not_a_string() {
        let accepted_claims = AcceptedClaims::from_comma_separated_values(
            "https://example.com, https://example.org",
            "iss".to_string(),
        );

        let result = accepted_claims.assert(&json!({
            "iss": 22
        }));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Claim 'iss' is not a string".to_string()
        );
    }
}
