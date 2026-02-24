use crate::keysmap::KeysMap;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{jwk::JwkSet, DecodingKey};
use reqwest::{Client, Url};
use std::{fs::File, io::Error, path::PathBuf, sync::Arc};
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
    jwks_pre_cached_file_path: Option<PathBuf>,
}

impl KeysStorage {
    pub fn new(
        jwks_uri: Url,
        min_refresh_rate: Duration,
        jwks_pre_cached_file_path: Option<PathBuf>,
    ) -> Self {
        Self {
            jwks_uri,
            min_refresh_rate,
            client: Client::builder()
                .user_agent(format!("oidc-authorizer/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap(),
            storage: Arc::new(RwLock::new((KeysMap::default(), Default::default()))),
            jwks_pre_cached_file_path,
        }
    }

    pub async fn get(&self, key_id: &str) -> Result<DecodingKey, KeysStorageError> {
        let read_guard = self.storage.read().await;
        let maybe_key = read_guard.0.get(key_id);
        if let Some(key) = maybe_key {
            tracing::debug!("Key found in memory storage: {}", key_id);
            return Ok(key.clone());
        }

        // Is the memory storage expired? Determine this before we muddy the memory from L2.
        let should_refresh = read_guard.1 + self.min_refresh_rate < Utc::now();
        drop(read_guard); // so that we can write to storage.

        let res = self.load_cached_file().await;
        if res.is_ok() {
            let read_guard = self.storage.read().await;
            let maybe_key = read_guard.0.get(key_id);
            if let Some(key) = maybe_key {
                tracing::debug!("Key found in cached JWKS file: {}", key_id);
                return Ok(key.clone());
            }
        }

        if should_refresh {
            tracing::debug!("Refreshing JWKS from origin.");
            self.refresh().await?;
            let read_guard = self.storage.read().await;
            let maybe_key = read_guard.0.get(key_id);
            if let Some(key) = maybe_key {
                return Ok(key.clone());
            }
            drop(read_guard);
        }

        tracing::error!(
            "Key not found in memory storage or cached JWKS file: {}",
            key_id
        );
        Err(KeysStorageError::KeyNotFound(key_id.to_string()))
    }

    async fn load_cached_file(&self) -> Result<(), Error> {
        if self.jwks_pre_cached_file_path.is_none() {
            tracing::debug!("Invalid JWKS pre-cache file path. Skipping cache load from disk.");
            return Ok(());
        }
        let path = self.jwks_pre_cached_file_path.as_ref().unwrap();
        tracing::debug!("Loading cached JWKS from '{}'", path.as_path().display());
        let file = File::open(path.as_path())?;
        let jwks: JwkSet = serde_json::from_reader(file)?;
        let mut write_guard = self.storage.write().await;
        write_guard.0 = jwks.into();
        write_guard.1 = write_guard.1.min(Utc::now());
        Ok(())
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

    #[tokio::test]
    async fn it_should_initialize_an_empty_instance() {
        let jwks_uri = Url::parse("https://example.com/jwks.json").unwrap();
        // SAFETY: safe to unwrap since (60 seconds) <= (i64::MAX / 1000)
        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let keys_cache = KeysStorage::new(jwks_uri.clone(), min_refresh_rate, None);

        assert_eq!(keys_cache.jwks_uri, jwks_uri);
        assert_eq!(keys_cache.min_refresh_rate, min_refresh_rate);
        assert_eq!(keys_cache.storage.read().await.0.len(), 0);
    }

    #[tokio::test]
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
    async fn it_should_not_error_if_the_cache_file_path_is_invalid() {
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
            Some(PathBuf::from("invalid")),
        );
        let key_result = keys_cache.get("test/keys/rs256/public").await;
        assert!(key_result.is_ok());
        jwks_mock.assert();
    }

    #[tokio::test]
    async fn it_should_not_return_an_error_if_the_cached_file_contains_invalid_json() {
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
            Some(PathBuf::from("{{invalid")),
        );
        let key_result = keys_cache.get("test/keys/rs256/public").await;
        assert!(key_result.is_ok());
        jwks_mock.assert();
    }

    #[tokio::test]
    async fn it_should_handle_invalid_jwks_cached_file_by_falling_back_to_refetching_jwks() {
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
        let path = file.into_temp_path().to_path_buf();
        let dynamic_json = json!({
            "something": "else",
            "array": [1, 2, 3]
        });
        std::fs::write(&path, dynamic_json.to_string()).unwrap();
        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let keys_cache = KeysStorage::new(jwks_uri.clone(), min_refresh_rate, Some(path));
        let key_result = keys_cache.get("test/keys/rs256/public").await;
        assert!(key_result.is_ok());
        jwks_mock.assert();
    }

    #[tokio::test]
    async fn it_should_return_key_from_valid_jwks_cache_file() {
        println!("it_should_return_key_from_valid_jwks_cache_file");
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
        println!("server setup complete");
        let jwks_uri = Url::parse(server.url("/").as_str()).unwrap();
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        println!("path: {}", path.display());
        let dynamic_json = json!({
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
        std::fs::write(path.clone(), dynamic_json.to_string()).unwrap();
        println!("file written to path: {}", path.display());

        let min_refresh_rate = Duration::try_seconds(60).unwrap();
        let keys_cache = KeysStorage::new(jwks_uri.clone(), min_refresh_rate, Some(path));
        println!("keys_cache created");
        let key_result = keys_cache.get("test/keys/rs256/public").await;
        println!("key_result");
        assert!(key_result.is_ok());
        jwks_mock.assert_calls(0);
    }
}
