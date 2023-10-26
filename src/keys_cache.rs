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

#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_initialize_an_instance() {
        let jwks_uri = Url::parse("https://example.com/jwks.json").unwrap();
        let min_refresh_rate = Duration::seconds(60);
        let keys_cache = KeysCache::new(jwks_uri.clone(), min_refresh_rate);

        assert_eq!(keys_cache.jwks_uri, jwks_uri);
        assert_eq!(keys_cache.min_refresh_rate, min_refresh_rate);
        assert_eq!(keys_cache.keys.len(), 0);
        assert!(keys_cache.should_refresh());
    }

    #[tokio::test]
    async fn it_should_referesh_the_cache() {
        let jwks_uri = Url::parse("https://example.com/jwks.json").unwrap();
        let min_refresh_rate = Duration::seconds(60);
        let mut _keys_cache = KeysCache::new(jwks_uri.clone(), min_refresh_rate);

        // TODO: mock the request and call the refresh method
    }
}
