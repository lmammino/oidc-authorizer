use crate::keysmap::KeysMap;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::jwk::JwkSet;
use reqwest::{Client, Url};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeysCacheError {
    #[error("Failed to fetch JWKS: {0}")]
    FetchError(#[from] reqwest::Error),
    #[error("Failed to parse JWKS content: {0}")]
    JwksParseError(#[from] serde_json::Error),
}

pub struct KeysCache {
    jwks_uri: Url,
    client: Client,
    pub keys: KeysMap,
    last_jwks_fetch: DateTime<Utc>,
    min_refresh_rate: Duration,
}

impl KeysCache {
    pub fn new(jwks_uri: Url, min_refresh_rate: Duration) -> Self {
        Self {
            jwks_uri,
            min_refresh_rate,
            client: Client::new(),
            keys: KeysMap::default(),
            last_jwks_fetch: Default::default(),
        }
    }

    pub async fn refresh(&mut self) -> Result<(), KeysCacheError> {
        let res = self.client.get(self.jwks_uri.as_ref()).send().await?;
        let jwks = res.text().await?;
        let jwks: JwkSet = serde_json::from_str(&jwks)?;
        self.keys = jwks.into();
        self.last_jwks_fetch = Utc::now();

        Ok(())
    }

    pub fn should_refresh(&self) -> bool {
        self.last_jwks_fetch + self.min_refresh_rate < Utc::now()
    }
}
