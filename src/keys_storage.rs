use crate::keysmap::KeysMap;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{jwk::JwkSet, DecodingKey};
use reqwest::{Client, Url};
use std::{fs::File, path::PathBuf, sync::Arc};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum KeysStorageError {
    #[error("Failed to fetch JWKS: {0}")]
    FetchError(#[from] reqwest::Error),
    #[error("Failed to parse JWKS content: {0}")]
    JwksParseError(#[from] serde_json::Error),
    #[error("Key '{0}' not found")]
    KeyNotFound(String),
}

#[derive(Debug)]
pub struct KeysStorage {
    jwks_uri: Url,
    client: Client,
    storage: Arc<RwLock<(KeysMap, DateTime<Utc>)>>,
    min_refresh_rate: Duration,
    pre_warmed: bool,
}

impl KeysStorage {
    pub fn new(
        jwks_uri: Url,
        min_refresh_rate: Duration,
        jwks_pre_cached_file_path: Option<PathBuf>,
    ) -> Self {
        let (initial_keys, pre_warmed) = match jwks_pre_cached_file_path {
            Some(ref path) => {
                tracing::debug!(
                    "Loading pre-cached JWKS from '{}'",
                    path.as_path().display()
                );
                match File::open(path).and_then(|file| {
                    serde_json::from_reader::<_, JwkSet>(file)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                }) {
                    Ok(jwks) => {
                        tracing::debug!("Pre-warmed JWKS cache with keys from file");
                        (KeysMap::from(jwks), true)
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load pre-cached JWKS file '{}': {}. Starting with empty cache.",
                            path.display(),
                            e
                        );
                        (KeysMap::default(), false)
                    }
                }
            }
            None => (KeysMap::default(), false),
        };

        Self {
            jwks_uri,
            min_refresh_rate,
            client: Client::builder()
                .user_agent(format!("oidc-authorizer/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap(),
            storage: Arc::new(RwLock::new((initial_keys, Default::default()))),
            pre_warmed,
        }
    }

    pub async fn get(&self, key_id: &str) -> Result<DecodingKey, KeysStorageError> {
        let read_guard = self.storage.read().await;
        if let Some(key) = read_guard.0.get(key_id) {
            return Ok(key.clone());
        }

        let should_refresh = read_guard.1 + self.min_refresh_rate < Utc::now();
        drop(read_guard);

        if should_refresh {
            self.refresh().await?;
            let read_guard = self.storage.read().await;
            if let Some(key) = read_guard.0.get(key_id) {
                if self.pre_warmed {
                    tracing::warn!(
                        event_type = "jwks_refresh_needed",
                        jwks_uri = %self.jwks_uri,
                        key_id = key_id,
                        "Pre-warmed JWKS cache miss. Keys refreshed from JWKS endpoint. \
                         Consider updating the pre-cached JWKS file."
                    );
                }
                return Ok(key.clone());
            }
            drop(read_guard);
        }

        Err(KeysStorageError::KeyNotFound(key_id.to_string()))
    }

    async fn refresh(&self) -> Result<(), KeysStorageError> {
        tracing::debug!("Refreshing JWKS from '{}'", self.jwks_uri.as_ref());
        let res = self.client.get(self.jwks_uri.as_ref()).send().await?;
        tracing::debug!("JWKS fetched got status: {}", res.status());
        let jwks = res.text().await?;
        tracing::debug!("JWKS fetched got body: {}", jwks);
        let jwks: JwkSet = serde_json::from_str(&jwks)?;

        let mut write_guard = self.storage.write().await;
        write_guard.0 = jwks.into();
        write_guard.1 = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;
    use tempfile::NamedTempFile;
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn it_should_initialize_an_empty_instance() {
        let jwks_uri = Url::parse("https://example.com/jwks.json").unwrap();
        // SAFETY: safe to unwrap since (60 seconds) <= (i64::MAX / 1000)
        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let keys_cache = KeysStorage::new(jwks_uri.clone(), min_refresh_rate, None);

        assert_eq!(keys_cache.jwks_uri, jwks_uri);
        assert_eq!(keys_cache.min_refresh_rate, min_refresh_rate);
        assert_eq!(keys_cache.storage.read().await.0.len(), 0);
        assert!(!keys_cache.pre_warmed);
    }

    #[tokio::test]
    #[traced_test]
    async fn it_should_referesh_the_cache_when_retrieving_the_first_key() {
        let server = MockServer::start();
        let jwks_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"
                    {
                        "keys":[
                            {
                                "kty":"RSA",
                                "n":"0TF4RX87dOllFp12D8IZvSoJyp8D4IZ3JmlVG7Au2GOSp1WcrAqjyq3Gk-a_1tT31FHCLVqjH9vXE8g1sXika4mp8YCWyMfjT3KsfrciI_Fw-nBCawnqewBDcBo4cvBgTjHNBjcjGNr0U_4eCZPjP8pwqw6HrRgHf-ypNmtgWG6_2EaK-tOJtnNgGRtCYGZdqMDfKLDuqzU5-gT2ejt9P1kNAvFMMUm4dTOK-vJ7jwGKWZEzupHBlHMqu4K4IRoFbVr2XsAzV5YQ0u_r26NVtQTDUdTp9ixhexUp0eXye6m3uMklqUOHJbiqNjmH2ye4yXVJI0w6BFOeXXlwyR6slw",
                                "e":"AQAB",
                                "alg":"RS256",
                                "kid":"test/keys/rs256/public",
                                "use":"sig"
                            }
                        ]
                    }
                "#);
        });

        let key_id = "test/keys/rs256/public";
        let jwks_uri = Url::parse(server.url("/").as_str()).unwrap();
        // SAFETY: safe to unwrap since (60 seconds) <= (i64::MAX / 1000)
        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let keys_cache = KeysStorage::new(jwks_uri.clone(), min_refresh_rate, None);
        let key_result = keys_cache.get(key_id).await;
        assert!(key_result.is_ok());
        jwks_mock.assert_calls(1);

        // if it reads the key again it should be taken straight away from cache
        let key_result = keys_cache.get(key_id).await;
        assert!(key_result.is_ok());
        jwks_mock.assert_calls(1); // no new hits
    }

    #[tokio::test]
    #[traced_test]
    async fn it_should_return_an_error_if_the_key_is_not_found() {
        let server = MockServer::start();
        let jwks_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"
                    {
                        "keys":[
                            {
                                "kty":"RSA",
                                "n":"0TF4RX87dOllFp12D8IZvSoJyp8D4IZ3JmlVG7Au2GOSp1WcrAqjyq3Gk-a_1tT31FHCLVqjH9vXE8g1sXika4mp8YCWyMfjT3KsfrciI_Fw-nBCawnqewBDcBo4cvBgTjHNBjcjGNr0U_4eCZPjP8pwqw6HrRgHf-ypNmtgWG6_2EaK-tOJtnNgGRtCYGZdqMDfKLDuqzU5-gT2ejt9P1kNAvFMMUm4dTOK-vJ7jwGKWZEzupHBlHMqu4K4IRoFbVr2XsAzV5YQ0u_r26NVtQTDUdTp9ixhexUp0eXye6m3uMklqUOHJbiqNjmH2ye4yXVJI0w6BFOeXXlwyR6slw",
                                "e":"AQAB",
                                "alg":"RS256",
                                "kid":"test/keys/rs256/public",
                                "use":"sig"
                            }
                        ]
                    }
                "#);
        });
        let key_id = "invalid";
        let jwks_uri = Url::parse(server.url("/").as_str()).unwrap();
        // SAFETY: safe to unwrap since (60 seconds) <= (i64::MAX / 1000)
        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let keys_cache = KeysStorage::new(jwks_uri.clone(), min_refresh_rate, None);
        let key_result = keys_cache.get(key_id).await;
        if let Err(KeysStorageError::KeyNotFound(_)) = key_result {
            // expected
        } else {
            panic!("Expected a KeyNotFound error");
        }

        jwks_mock.assert();
    }

    #[tokio::test]
    #[traced_test]
    async fn it_should_fall_back_to_network_if_cache_file_path_is_invalid() {
        let server = MockServer::start();
        let jwks_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"
                    {
                        "keys":[
                            {
                                "kty":"RSA",
                                "n":"0TF4RX87dOllFp12D8IZvSoJyp8D4IZ3JmlVG7Au2GOSp1WcrAqjyq3Gk-a_1tT31FHCLVqjH9vXE8g1sXika4mp8YCWyMfjT3KsfrciI_Fw-nBCawnqewBDcBo4cvBgTjHNBjcjGNr0U_4eCZPjP8pwqw6HrRgHf-ypNmtgWG6_2EaK-tOJtnNgGRtCYGZdqMDfKLDuqzU5-gT2ejt9P1kNAvFMMUm4dTOK-vJ7jwGKWZEzupHBlHMqu4K4IRoFbVr2XsAzV5YQ0u_r26NVtQTDUdTp9ixhexUp0eXye6m3uMklqUOHJbiqNjmH2ye4yXVJI0w6BFOeXXlwyR6slw",
                                "e":"AQAB",
                                "alg":"RS256",
                                "kid":"test/keys/rs256/public",
                                "use":"sig"
                            }
                        ]
                    }
                "#);
        });
        let jwks_uri = Url::parse(server.url("/").as_str()).unwrap();
        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let keys_cache = KeysStorage::new(
            jwks_uri.clone(),
            min_refresh_rate,
            Some(PathBuf::from("/nonexistent/path/jwks.json")),
        );
        assert!(!keys_cache.pre_warmed);
        let key_result = keys_cache.get("test/keys/rs256/public").await;
        assert!(key_result.is_ok());
        jwks_mock.assert();
    }

    #[tokio::test]
    #[traced_test]
    async fn it_should_fall_back_to_network_if_cached_file_contains_invalid_json() {
        let server = MockServer::start();
        let jwks_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"
                    {
                        "keys":[
                            {
                                "kty":"RSA",
                                "n":"0TF4RX87dOllFp12D8IZvSoJyp8D4IZ3JmlVG7Au2GOSp1WcrAqjyq3Gk-a_1tT31FHCLVqjH9vXE8g1sXika4mp8YCWyMfjT3KsfrciI_Fw-nBCawnqewBDcBo4cvBgTjHNBjcjGNr0U_4eCZPjP8pwqw6HrRgHf-ypNmtgWG6_2EaK-tOJtnNgGRtCYGZdqMDfKLDuqzU5-gT2ejt9P1kNAvFMMUm4dTOK-vJ7jwGKWZEzupHBlHMqu4K4IRoFbVr2XsAzV5YQ0u_r26NVtQTDUdTp9ixhexUp0eXye6m3uMklqUOHJbiqNjmH2ye4yXVJI0w6BFOeXXlwyR6slw",
                                "e":"AQAB",
                                "alg":"RS256",
                                "kid":"test/keys/rs256/public",
                                "use":"sig"
                            }
                        ]
                    }
                "#);
        });
        let jwks_uri = Url::parse(server.url("/").as_str()).unwrap();
        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        std::fs::write(&path, "{{not valid json").unwrap();
        let keys_cache = KeysStorage::new(jwks_uri.clone(), min_refresh_rate, Some(path));
        assert!(!keys_cache.pre_warmed);
        let key_result = keys_cache.get("test/keys/rs256/public").await;
        assert!(key_result.is_ok());
        jwks_mock.assert();
    }

    #[tokio::test]
    #[traced_test]
    async fn it_should_fall_back_to_network_if_cached_file_has_wrong_schema() {
        let server = MockServer::start();
        let jwks_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"
                    {
                        "keys":[
                            {
                                "kty":"RSA",
                                "n":"0TF4RX87dOllFp12D8IZvSoJyp8D4IZ3JmlVG7Au2GOSp1WcrAqjyq3Gk-a_1tT31FHCLVqjH9vXE8g1sXika4mp8YCWyMfjT3KsfrciI_Fw-nBCawnqewBDcBo4cvBgTjHNBjcjGNr0U_4eCZPjP8pwqw6HrRgHf-ypNmtgWG6_2EaK-tOJtnNgGRtCYGZdqMDfKLDuqzU5-gT2ejt9P1kNAvFMMUm4dTOK-vJ7jwGKWZEzupHBlHMqu4K4IRoFbVr2XsAzV5YQ0u_r26NVtQTDUdTp9ixhexUp0eXye6m3uMklqUOHJbiqNjmH2ye4yXVJI0w6BFOeXXlwyR6slw",
                                "e":"AQAB",
                                "alg":"RS256",
                                "kid":"test/keys/rs256/public",
                                "use":"sig"
                            }
                        ]
                    }
                "#);
        });
        let jwks_uri = Url::parse(server.url("/").as_str()).unwrap();
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        let dynamic_json = json!({
            "something": "else",
            "array": [1, 2, 3]
        });
        std::fs::write(&path, dynamic_json.to_string()).unwrap();
        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let keys_cache = KeysStorage::new(jwks_uri.clone(), min_refresh_rate, Some(path));
        assert!(!keys_cache.pre_warmed);
        let key_result = keys_cache.get("test/keys/rs256/public").await;
        assert!(key_result.is_ok());
        jwks_mock.assert();
    }

    #[tokio::test]
    #[traced_test]
    async fn it_should_return_key_from_pre_warmed_cache_without_network_call() {
        let server = MockServer::start();
        let jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"keys":[]}"#);
        });
        let jwks_uri = Url::parse(server.url("/").as_str()).unwrap();
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        let jwks_json = json!({
            "keys":[
                {
                    "kty":"RSA",
                    "n":"0TF4RX87dOllFp12D8IZvSoJyp8D4IZ3JmlVG7Au2GOSp1WcrAqjyq3Gk-a_1tT31FHCLVqjH9vXE8g1sXika4mp8YCWyMfjT3KsfrciI_Fw-nBCawnqewBDcBo4cvBgTjHNBjcjGNr0U_4eCZPjP8pwqw6HrRgHf-ypNmtgWG6_2EaK-tOJtnNgGRtCYGZdqMDfKLDuqzU5-gT2ejt9P1kNAvFMMUm4dTOK-vJ7jwGKWZEzupHBlHMqu4K4IRoFbVr2XsAzV5YQ0u_r26NVtQTDUdTp9ixhexUp0eXye6m3uMklqUOHJbiqNjmH2ye4yXVJI0w6BFOeXXlwyR6slw",
                    "e":"AQAB",
                    "alg":"RS256",
                    "kid":"test/keys/rs256/public",
                    "use":"sig"
                }
            ]
        });
        std::fs::write(&path, jwks_json.to_string()).unwrap();

        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let keys_cache = KeysStorage::new(jwks_uri.clone(), min_refresh_rate, Some(path));
        assert!(keys_cache.pre_warmed);

        let key_result = keys_cache.get("test/keys/rs256/public").await;
        assert!(key_result.is_ok());
        jwks_mock.assert_calls(0);
    }
}
