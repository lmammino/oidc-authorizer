use crate::{
    accepted_algorithms::{self, AcceptedAlgorithms},
    accepted_claims::AcceptedClaims,
    keys_cache::KeysCache,
    models::{TokenAuthorizerEvent, TokenAuthorizerResponse},
};
use jsonwebtoken::{decode, decode_header, Validation};
use lambda_runtime::{Error, LambdaEvent, Service};
use reqwest::Url;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::RwLock;

pub struct Handler {
    pub keys_cache: Arc<RwLock<KeysCache>>,
    pub accepted_issuers: AcceptedClaims,
    pub accepted_audiences: AcceptedClaims,
    pub accepted_signing_algorithms: AcceptedAlgorithms,
}

impl Handler {
    pub fn new(
        jwks_uri: Url,
        min_refresh_rate: chrono::Duration,
        accepted_issuers: AcceptedClaims,
        accepted_audiences: AcceptedClaims,
        accepted_algorithms: AcceptedAlgorithms,
    ) -> Self {
        let keys_cache = KeysCache::new(jwks_uri, min_refresh_rate);
        Self {
            keys_cache: Arc::new(RwLock::new(keys_cache)),
            accepted_issuers,
            accepted_audiences,
            accepted_signing_algorithms: accepted_algorithms,
        }
    }
}

fn parse_token_from_header(authorization_token: &str) -> Result<&str, Error> {
    if authorization_token.len() >= 8 && &(authorization_token[0..7]) == "Bearer " {
        return Ok(&(authorization_token[7..]));
    }
    Err(Error::from("Authorization token must start with 'Bearer '"))
}

impl Service<LambdaEvent<TokenAuthorizerEvent>> for Handler {
    type Response = TokenAuthorizerResponse;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: LambdaEvent<TokenAuthorizerEvent>) -> Self::Future {
        let jwks_cache = self.keys_cache.clone();

        Box::pin(async move {
            // TODO: logging
            let (event, _) = req.into_parts();
            // TODO: convert exceptions into explicit deny (with proper logs)
            let token = parse_token_from_header(&event.authorization_token)?;
            let token_header = decode_header(token)?;
            if token_header.kid.is_none() {
                return Err("Missing kid".into());
            }
            // get the key from the cache
            let key_id = token_header.kid.unwrap();
            // TODO: validate supported signing algorithms
            let read_guard = jwks_cache.read().await;
            let maybe_key = read_guard.keys.get(&key_id);
            if maybe_key.is_none() && read_guard.should_refresh() {
                jwks_cache.write().await.refresh().await?;
            }
            let key = read_guard.keys.get(&key_id).cloned();
            // TODO: fail if cannot find key
            let token_payload = decode::<serde_json::Value>(
                token,
                &key.unwrap(),
                &Validation::new(token_header.alg),
            )?;
            // TODO: validate issuer, audience and other claims

            println!("{:?}", token);

            Ok(TokenAuthorizerResponse::allow(&event.method_arn))
        })
    }
}
