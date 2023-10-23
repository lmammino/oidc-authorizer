use std::{convert::Infallible, str::FromStr};

#[derive(Debug, Clone, Default)]
pub struct AcceptedClaims(Vec<String>);

impl AcceptedClaims {
    pub fn is_accepted(&self, claim_value: &str) -> bool {
        self.0.is_empty() || self.0.contains(&claim_value.to_string())
    }
}

impl FromStr for AcceptedClaims {
    type Err = Infallible;
    fn from_str(claim_values: &str) -> Result<Self, Self::Err> {
        let accepted_values = claim_values
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        Ok(Self(accepted_values))
    }
}
