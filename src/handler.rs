use crate::{
    accepted_algorithms::AcceptedAlgorithms,
    accepted_claims::AcceptedClaims,
    keys_cache::KeysCache,
    models::{TokenAuthorizerEvent, TokenAuthorizerResponse},
    parse_token_from_header::parse_token_from_header,
    principalid_claims::PrincipalIDClaims,
};
use futures_util::future::{BoxFuture, FutureExt};
use jsonwebtoken::{decode, decode_header, Validation};
use lambda_runtime::{Error, LambdaEvent, Service};
use reqwest::Url;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::RwLock;

pub struct Handler {
    pub keys_cache: Arc<RwLock<KeysCache>>,
    pub principal_id_claims: &'static PrincipalIDClaims,
    pub accepted_issuers: &'static AcceptedClaims,
    pub accepted_audiences: &'static AcceptedClaims,
    pub accepted_signing_algorithms: &'static AcceptedAlgorithms,
}

impl Handler {
    pub fn new(
        jwks_uri: Url,
        min_refresh_rate: chrono::Duration,
        principal_id_claims: &'static PrincipalIDClaims,
        accepted_issuers: &'static AcceptedClaims,
        accepted_audiences: &'static AcceptedClaims,
        accepted_signing_algorithms: &'static AcceptedAlgorithms,
    ) -> Self {
        let keys_cache = KeysCache::new(jwks_uri, min_refresh_rate);
        Self {
            keys_cache: Arc::new(RwLock::new(keys_cache)),
            principal_id_claims,
            accepted_issuers,
            accepted_audiences,
            accepted_signing_algorithms,
        }
    }

    async fn do_call(self, event: TokenAuthorizerEvent) -> Result<TokenAuthorizerResponse, Error> {
        // extract token from header
        let token = parse_token_from_header(&event.authorization_token);
        if let Err(e) = token {
            tracing::info!(
                "Failed to extract token fron header (header_value='{}'): {}",
                event.authorization_token,
                e
            );
            return Ok(TokenAuthorizerResponse::deny(""));
        }
        let token = token.unwrap();
        let deny = TokenAuthorizerResponse::deny(&event.method_arn);

        // parse token header
        let token_header = decode_header(token);
        if let Err(e) = token_header {
            tracing::info!("Failed to parse token header (token='{}'): {}", token, e);
            return Ok(deny);
        }
        let token_header = token_header.unwrap();

        // validate the signing algorithm
        if let Err(e) = self.accepted_signing_algorithms.assert(&token_header.alg) {
            tracing::info!(e);
            return Ok(deny);
        }

        // retrieve token key
        if token_header.kid.is_none() {
            tracing::info!(
                "Missing kid in token header (token_header='{:?}')",
                token_header
            );
            return Ok(deny);
        }

        // get the key from the cache
        let key_id = token_header.kid.unwrap();
        let read_guard = self.keys_cache.read().await;
        let maybe_key = read_guard.keys.get(&key_id);
        if maybe_key.is_none() && read_guard.should_refresh() {
            self.keys_cache.write().await.refresh().await?;
        }
        let key = read_guard.keys.get(&key_id).cloned();
        if key.is_none() {
            tracing::info!("Failed to find key for kid='{}'", key_id);
            return Ok(deny);
        }
        let key = key.unwrap();

        // validate token and get payload
        let token_payload =
            decode::<serde_json::Value>(token, &key, &Validation::new(token_header.alg));
        if let Err(e) = token_payload {
            tracing::info!("Failed to validate token (token='{}'): {}", token, e);
            return Ok(deny);
        }
        let token_payload = token_payload.unwrap();

        // validate issuer
        if let Err(e) = self.accepted_issuers.assert(&token_payload.claims) {
            tracing::info!(e);
            return Ok(deny);
        }

        // validate audience
        if let Err(e) = self.accepted_audiences.assert(&token_payload.claims) {
            tracing::info!(e);
            return Ok(deny);
        }

        let principal_id = self
            .principal_id_claims
            .get_principal_id_from_claims(&token_payload.claims);

        Ok(TokenAuthorizerResponse::allow(
            token,
            &principal_id,
            &event.method_arn,
            &token_payload.claims,
        ))
    }
}

impl Clone for Handler {
    fn clone(&self) -> Self {
        Self {
            keys_cache: self.keys_cache.clone(),
            principal_id_claims: self.principal_id_claims,
            accepted_issuers: self.accepted_issuers,
            accepted_audiences: self.accepted_audiences,
            accepted_signing_algorithms: self.accepted_signing_algorithms,
        }
    }
}

impl Service<LambdaEvent<TokenAuthorizerEvent>> for Handler {
    type Response = TokenAuthorizerResponse;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: LambdaEvent<TokenAuthorizerEvent>) -> Self::Future {
        let (event, _) = req.into_parts();
        self.clone().do_call(event).boxed()
    }
}
