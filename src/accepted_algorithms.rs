use jsonwebtoken::Algorithm;
use std::{collections::HashSet, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct AcceptedAlgorithms(HashSet<Algorithm>);

#[derive(Debug, Error)]
pub enum AcceptedAlgorithmsError {
    #[error("Invalid algorithm name")]
    InvalidAlgorithmName(#[from] jsonwebtoken::errors::Error),
    #[error("Unsupported algorithm '{0:?}'. Only public-key algorithms are supported")]
    UnsupportedAlgorithm(Algorithm),
}

static SUPPORTED_ALGORITHMS: &[Algorithm] = &[
    Algorithm::ES256,
    Algorithm::ES384,
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::PS256,
    Algorithm::PS384,
    Algorithm::PS512,
    Algorithm::EdDSA,
];

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

impl FromStr for AcceptedAlgorithms {
    type Err = AcceptedAlgorithmsError;
    fn from_str(algorithms: &str) -> Result<Self, Self::Err> {
        let algorithms = algorithms
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|s| Algorithm::from_str(&s))
            .collect::<Result<Vec<_>, _>>()?;

        for algorithm in &algorithms {
            if !SUPPORTED_ALGORITHMS.contains(algorithm) {
                return Err(AcceptedAlgorithmsError::UnsupportedAlgorithm(*algorithm));
            }
        }

        Ok(Self(algorithms.into_iter().collect()))
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
    fn it_should_accept_any_algorithm_with_an_empty_string() {
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
    fn it_should_accept_any_algorithm_by_default() {
        let accepted_alg: AcceptedAlgorithms = Default::default();
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
            accepted_alg.unwrap_err().to_string(),
            "Invalid algorithm name".to_string()
        );
    }

    #[test]
    fn it_should_not_allow_secret_based_algorithms_to_be_instantiated() {
        let s = "HS256";
        let accepted_alg: Result<AcceptedAlgorithms, _> = s.parse();
        assert!(accepted_alg.is_err());
        assert_eq!(
            accepted_alg.unwrap_err().to_string(),
            "Unsupported algorithm 'HS256'. Only public-key algorithms are supported".to_string()
        );
    }
}
