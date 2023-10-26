use std::str::FromStr;

use jsonwebtoken::Algorithm;

#[derive(Debug, Clone)]
pub struct AcceptedAlgorithms(Vec<Algorithm>);

impl AcceptedAlgorithms {
    pub fn is_accepted(&self, algorithm: &Algorithm) -> bool {
        self.0.is_empty() || self.0.contains(algorithm)
    }

    pub fn assert(&self, algorithm: &Algorithm) -> Result<(), String> {
        match self.is_accepted(algorithm) {
            true => Ok(()),
            false => Err(format!(
                "Unsupported algorithm (found='{:?}', supported={:?})",
                algorithm, self.0
            )),
        }
    }
}

impl Default for AcceptedAlgorithms {
    fn default() -> Self {
        Self(vec![
            // supports all asymmetric signature algorithms by default
            Algorithm::ES256,
            Algorithm::ES384,
            Algorithm::RS256,
            Algorithm::RS384,
            Algorithm::RS512,
            Algorithm::PS256,
            Algorithm::PS384,
            Algorithm::PS512,
            Algorithm::EdDSA,
        ])
    }
}

impl FromStr for AcceptedAlgorithms {
    type Err = jsonwebtoken::errors::Error;
    fn from_str(algorithms: &str) -> Result<Self, Self::Err> {
        let algorithms = algorithms
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|s| Algorithm::from_str(&s))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self(algorithms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_initialize_from_a_string_with_one_valid_item() {
        let s = "RS512";
        let accepted_alg: Result<AcceptedAlgorithms, _> = s.parse();
        assert!(accepted_alg.is_ok());
        let accepted_alg = accepted_alg.unwrap();
        assert!(accepted_alg.is_accepted(&Algorithm::RS512));
        assert!(accepted_alg.assert(&Algorithm::RS512).is_ok());
        assert!(!accepted_alg.is_accepted(&Algorithm::EdDSA));
        assert!(accepted_alg.assert(&Algorithm::EdDSA).is_err());
    }

    #[test]
    fn it_should_initialize_from_a_string_with_multiple_valid_items() {
        let s = "RS512,EdDSA,   ES384"; // spacing is intentional to validate proper trimming
        let accepted_alg: Result<AcceptedAlgorithms, _> = s.parse();
        assert!(accepted_alg.is_ok());
        let accepted_alg = accepted_alg.unwrap();
        assert!(accepted_alg.is_accepted(&Algorithm::RS512));
        assert!(accepted_alg.assert(&Algorithm::RS512).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::EdDSA));
        assert!(accepted_alg.assert(&Algorithm::EdDSA).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::ES384));
        assert!(accepted_alg.assert(&Algorithm::ES384).is_ok());
        assert!(!accepted_alg.is_accepted(&Algorithm::ES256));
        assert!(accepted_alg.assert(&Algorithm::ES256).is_err());
    }

    #[test]
    fn it_should_accept_any_algorithm_if_empty() {
        let s = "";
        let accepted_alg: Result<AcceptedAlgorithms, _> = s.parse();
        assert!(accepted_alg.is_ok());
        let accepted_alg = accepted_alg.unwrap();
        assert!(accepted_alg.is_accepted(&Algorithm::ES256));
        assert!(accepted_alg.assert(&Algorithm::ES256).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::ES384));
        assert!(accepted_alg.assert(&Algorithm::ES384).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::RS256));
        assert!(accepted_alg.assert(&Algorithm::RS256).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::RS384));
        assert!(accepted_alg.assert(&Algorithm::RS384).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::RS512));
        assert!(accepted_alg.assert(&Algorithm::RS512).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::PS256));
        assert!(accepted_alg.assert(&Algorithm::PS256).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::PS384));
        assert!(accepted_alg.assert(&Algorithm::PS384).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::PS512));
        assert!(accepted_alg.assert(&Algorithm::PS512).is_ok());
        assert!(accepted_alg.is_accepted(&Algorithm::EdDSA));
        assert!(accepted_alg.assert(&Algorithm::EdDSA).is_ok());
    }

    #[test]
    fn it_should_fail_to_parse_if_invalid_algorithms_are_passed() {
        let s = "RS512, invalid, EdDSA";
        let accepted_alg: Result<AcceptedAlgorithms, _> = s.parse();
        assert!(accepted_alg.is_err());
        assert_eq!(
            accepted_alg.unwrap_err().kind(),
            &jsonwebtoken::errors::ErrorKind::InvalidAlgorithmName
        );
    }
}
