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
                "Unsupported algorithm (found={:?}, supported={:?})",
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
