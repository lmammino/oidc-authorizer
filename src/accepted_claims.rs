use serde_json::Value;
use std::fmt::Display;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct AcceptedClaims(Vec<String>, String);

pub enum StringOrArray<'a> {
    String(&'a str),
    Array(Vec<String>),
}

impl Display for StringOrArray<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringOrArray::String(s) => write!(f, "{}", s),
            StringOrArray::Array(s) => write!(f, "[{:?}]", s),
        }
    }
}

impl<'a> From<&'a str> for StringOrArray<'a> {
    fn from(s: &'a str) -> Self {
        StringOrArray::String(s)
    }
}

impl<S: Display> From<&[S]> for StringOrArray<'_> {
    fn from(a: &[S]) -> Self {
        StringOrArray::Array(a.iter().map(|s| s.to_string()).collect())
    }
}

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

    pub fn is_accepted(&self, claim_value: &StringOrArray) -> bool {
        self.0.is_empty()
            || match claim_value {
                StringOrArray::String(claim_value) => self.0.contains(&claim_value.to_string()),
                StringOrArray::Array(claim_values) => claim_values
                    .iter()
                    .any(|claim_value| self.0.contains(claim_value)),
            }
    }

    pub fn assert(&self, claims: &Value) -> Result<(), String> {
        if self.0.is_empty() {
            // if empty do not validate
            return Ok(());
        }

        let claim_value = match claims.get(&self.1) {
            Some(claim_value) => match claim_value {
                Value::String(claim_value) => StringOrArray::String(claim_value),
                Value::Array(claim_values) => {
                    let claim_values = claim_values
                        .iter()
                        .map(|claim_value| match claim_value {
                            Value::String(claim_value) => Ok(claim_value.to_string()),
                            _ => Err(format!(
                                "Claim '{}' is not a string or an array of strings",
                                self.1
                            )),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    StringOrArray::Array(claim_values)
                }
                _ => {
                    return Err(format!(
                        "Claim '{}' is not a string or an array of strings",
                        self.1
                    ))
                }
            },
            None => return Err(format!("Missing claim '{}'", self.1)),
        };

        match self.is_accepted(&claim_value) {
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
        // example.com and example.org are accepted
        // example.net is not accepted

        let accepted_claims = AcceptedClaims::from_comma_separated_values(
            "https://example.com, https://example.org",
            "iss".to_string(),
        );

        assert!(accepted_claims.is_accepted(&"https://example.com".into()));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.com"}))
            .is_ok());
        assert!(accepted_claims.is_accepted(&"https://example.org".into()));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.org"}))
            .is_ok());

        // if the claim is an array, it should accept if at least one of the values is accepted
        assert!(accepted_claims
            .assert(&json!({"iss": ["https://example.net", "https://example.com"]}))
            .is_ok());

        // do not accept example.net (not listed)
        assert!(!accepted_claims.is_accepted(&"https://example.net".into()));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.net"}))
            .is_err());

        // do not accept example.net (not listed) even when an array of claims was provided in the token
        assert!(!accepted_claims
            .is_accepted(&["https://example.net", "https://example.tld"][..].into()));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.net"}))
            .is_err());
    }

    #[test]
    fn it_should_accept_everything_when_using_an_empty_list() {
        let accepted_claims = AcceptedClaims::from_comma_separated_values("", "iss".to_string());

        assert!(accepted_claims.is_accepted(&"https://example.com".into()));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.com"}))
            .is_ok());
        assert!(accepted_claims.is_accepted(&"https://example.org".into()));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.org"}))
            .is_ok());
        assert!(accepted_claims.is_accepted(&"https://example.net".into()));
        assert!(accepted_claims
            .assert(&json!({"iss": "https://example.net"}))
            .is_ok());
        // it should accept an array of strings
        assert!(accepted_claims
            .assert(&json!({"iss": ["https://example.net", "https://example.com"]}))
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
    fn it_should_reject_if_the_claim_is_not_a_string_or_an_array_of_strings() {
        let accepted_claims = AcceptedClaims::from_comma_separated_values(
            "https://example.com, https://example.org",
            "iss".to_string(),
        );

        // not a string
        let result = accepted_claims.assert(&json!({
            "iss": 22
        }));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Claim 'iss' is not a string or an array of strings".to_string()
        );

        // not an array of string
        let result = accepted_claims.assert(&json!({
            "iss": ["https://example.com", 22, "https://example.org"]
        }));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Claim 'iss' is not a string or an array of strings".to_string()
        );
    }
}
