use crate::{
    accepted_algorithms::AcceptedAlgorithms,
    accepted_claims::AcceptedClaims,
    keys_storage::KeysStorage,
    models::{TokenAuthorizerEvent, TokenAuthorizerResponse},
    parse_token_from_header::parse_token_from_header,
    principalid_claims::PrincipalIDClaims,
};
use futures_util::future::{BoxFuture, FutureExt};
use jsonwebtoken::{decode, decode_header, Validation};
use lambda_runtime::{Error, LambdaEvent, Service};
use std::task::{Context, Poll};

pub struct Handler {
    pub keys: &'static KeysStorage,
    pub principal_id_claims: &'static PrincipalIDClaims,
    pub accepted_issuers: &'static AcceptedClaims,
    pub accepted_audiences: &'static AcceptedClaims,
    pub accepted_signing_algorithms: &'static AcceptedAlgorithms,
}

impl Handler {
    pub fn new(
        keys: &'static KeysStorage,
        principal_id_claims: &'static PrincipalIDClaims,
        accepted_issuers: &'static AcceptedClaims,
        accepted_audiences: &'static AcceptedClaims,
        accepted_signing_algorithms: &'static AcceptedAlgorithms,
    ) -> Self {
        Self {
            keys,
            principal_id_claims,
            accepted_issuers,
            accepted_audiences,
            accepted_signing_algorithms,
        }
    }

    async fn do_call(self, event: TokenAuthorizerEvent) -> Result<TokenAuthorizerResponse, Error> {
        // TODO: custom metrics using EMF logs
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

        // get the key from the storage
        let key_id = token_header.kid.unwrap();
        let key_result = self.keys.get(&key_id).await;
        if let Err(e) = key_result {
            tracing::info!("Failed to retrieve key (key_id='{}'): {}", key_id, e);
            return Ok(deny);
        }
        let key = key_result.unwrap();

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
            keys: self.keys,
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use httpmock::prelude::*;
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use reqwest::Url;
    use serde_json::json;
    use tracing_test::traced_test;

    async fn test_with(algorithm: Algorithm, encoding_key: EncodingKey, jwk: &str, kid: &str) {
        // create a mock server that will serve the jwks
        let server = MockServer::start();
        let jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!("{{\"keys\":[ {} ]}}", jwk));
        });

        let server_url = server.url("/");
        let iss = server_url.clone();
        let aud = "test-app";
        let exp = (Utc::now() + Duration::hours(1)).timestamp();

        // creates a token using the private PEM
        let token_header: Header =
            serde_json::from_value(json!({ "alg": algorithm, "kid": kid })).unwrap();
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user" }),
            &encoding_key,
        );
        assert!(token.is_ok());
        let token = token.unwrap();

        // instantiates an handler
        let min_refresh_rate = Duration::seconds(600);
        let jwks_uri = server_url.clone();
        let principal_id_claims = PrincipalIDClaims::from_comma_separated_values(
            "preferred_username, sub",
            "unknown".to_string(),
        );
        let accepted_issuers = AcceptedClaims::from_comma_separated_values(&iss, "iss".to_string());
        let accepted_audiences =
            AcceptedClaims::from_comma_separated_values(aud, "aud".to_string());
        let accepted_signing_algorithms: AcceptedAlgorithms = Default::default();
        let keys = KeysStorage::new(Url::parse(&jwks_uri).unwrap(), min_refresh_rate);
        let mut handler = Handler::new(
            Box::leak(Box::new(keys)),
            Box::leak(Box::new(principal_id_claims)),
            Box::leak(Box::new(accepted_issuers)),
            Box::leak(Box::new(accepted_audiences)),
            Box::leak(Box::new(accepted_signing_algorithms)),
        );

        // creates the event
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "arn:aws:execute-api:us-east-1:123456789012:ymy8tbxw7b/*/GET/".to_string(),
        };

        // calls the handler service and get the response
        let request = LambdaEvent::new(event, Default::default());
        let response = handler.call(request).await;

        jwks_mock.assert();
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(
            response.policy_document.statement.get(0).unwrap().effect,
            "Allow"
        );
        assert_eq!(response.principal_id, "some_user");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_validates_rs256_tokens() {
        let private_key = include_str!("../tests/fixtures/keys/rs256/private.pem");
        test_with(
            Algorithm::RS256,
            EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap(),
            include_str!("../tests/fixtures/keys/rs256/jwk.json"),
            "test/keys/rs256/public",
        )
        .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn it_validates_rs384_tokens() {
        let private_key = include_str!("../tests/fixtures/keys/rs384/private.pem");
        test_with(
            Algorithm::RS384,
            EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap(),
            include_str!("../tests/fixtures/keys/rs384/jwk.json"),
            "test/keys/rs384/public",
        )
        .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn it_validates_rs512_tokens() {
        let private_key = include_str!("../tests/fixtures/keys/rs512/private.pem");
        test_with(
            Algorithm::RS512,
            EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap(),
            include_str!("../tests/fixtures/keys/rs512/jwk.json"),
            "test/keys/rs512/public",
        )
        .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn it_validates_es256_tokens() {
        let private_key = include_str!("../tests/fixtures/keys/es256/private.pem");
        test_with(
            Algorithm::ES256,
            EncodingKey::from_ec_pem(private_key.as_bytes()).unwrap(),
            include_str!("../tests/fixtures/keys/es256/jwk.json"),
            "test/keys/es256/public",
        )
        .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn it_validates_es384_tokens() {
        let private_key = include_str!("../tests/fixtures/keys/es384/private.pem");
        test_with(
            Algorithm::ES384,
            EncodingKey::from_ec_pem(private_key.as_bytes()).unwrap(),
            include_str!("../tests/fixtures/keys/es384/jwk.json"),
            "test/keys/es384/public",
        )
        .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn it_validates_ps256_tokens() {
        let private_key = include_str!("../tests/fixtures/keys/ps256/private.pem");
        test_with(
            Algorithm::PS256,
            EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap(),
            include_str!("../tests/fixtures/keys/ps256/jwk.json"),
            "test/keys/ps256/public",
        )
        .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn it_validates_ps384_tokens() {
        let private_key = include_str!("../tests/fixtures/keys/ps384/private.pem");
        test_with(
            Algorithm::PS384,
            EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap(),
            include_str!("../tests/fixtures/keys/ps384/jwk.json"),
            "test/keys/ps384/public",
        )
        .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn it_validates_ps512_tokens() {
        let private_key = include_str!("../tests/fixtures/keys/ps512/private.pem");
        test_with(
            Algorithm::PS512,
            EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap(),
            include_str!("../tests/fixtures/keys/ps512/jwk.json"),
            "test/keys/ps512/public",
        )
        .await;
    }

    #[tokio::test]
    #[traced_test]
    async fn it_validates_eddsa_tokens() {
        let private_key = include_str!("../tests/fixtures/keys/eddsa/private.pem");
        test_with(
            Algorithm::EdDSA,
            EncodingKey::from_ed_pem(private_key.as_bytes()).unwrap(),
            include_str!("../tests/fixtures/keys/eddsa/jwk.json"),
            "test/keys/eddsa/public",
        )
        .await;
    }
}
