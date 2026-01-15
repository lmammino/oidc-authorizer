use crate::{
    accepted_algorithms::AcceptedAlgorithms,
    accepted_claims::AcceptedClaims,
    cel_validation::CelValidator,
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
    pub cel_validator: &'static CelValidator,
}

impl Handler {
    pub fn new(
        keys: &'static KeysStorage,
        principal_id_claims: &'static PrincipalIDClaims,
        accepted_issuers: &'static AcceptedClaims,
        accepted_audiences: &'static AcceptedClaims,
        accepted_signing_algorithms: &'static AcceptedAlgorithms,
        cel_validator: &'static CelValidator,
    ) -> Self {
        Self {
            keys,
            principal_id_claims,
            accepted_issuers,
            accepted_audiences,
            accepted_signing_algorithms,
            cel_validator,
        }
    }

    async fn do_call(self, event: TokenAuthorizerEvent) -> Result<TokenAuthorizerResponse, Error> {
        // TODO: custom metrics using EMF logs
        // extract token from header
        let token = match parse_token_from_header(&event.authorization_token) {
            Ok(token) => token,
            Err(e) => {
                tracing::info!(
                    "Failed to extract token fron header (header_value='{}'): {}",
                    event.authorization_token,
                    e
                );
                return Ok(TokenAuthorizerResponse::deny(&event.method_arn));
            }
        };

        // parse token header
        let token_header = match decode_header(token) {
            Ok(token_header) => token_header,
            Err(e) => {
                tracing::info!("Failed to parse token header (token='{}'): {}", token, e);
                return Ok(TokenAuthorizerResponse::deny(&event.method_arn));
            }
        };

        // validate the signing algorithm
        if let Err(e) = self.accepted_signing_algorithms.assert(&token_header.alg) {
            tracing::info!(e);
            return Ok(TokenAuthorizerResponse::deny(&event.method_arn));
        }

        let key = match &token_header.kid {
            Some(key_id) => match self.keys.get(key_id).await {
                Ok(key) => key,
                Err(e) => {
                    tracing::info!("Failed to retrieve key (key_id='{}'): {}", key_id, e);
                    return Ok(TokenAuthorizerResponse::deny(&event.method_arn));
                }
            },
            None => {
                tracing::info!(
                    "Missing kid in token header (token_header='{:?}')",
                    token_header
                );
                return Ok(TokenAuthorizerResponse::deny(&event.method_arn));
            }
        };

        let mut validation = Validation::new(token_header.alg);
        validation.set_audience(&self.accepted_audiences.accepted_values());
        validation.set_issuer(&self.accepted_issuers.accepted_values());
        let token_payload = match decode::<serde_json::Value>(token, &key, &validation) {
            Ok(token_payload) => token_payload,
            Err(e) => {
                tracing::info!("Failed to validate token (token='{}'): {}", token, e);
                return Ok(TokenAuthorizerResponse::deny(&event.method_arn));
            }
        };

        // CEL validation (if configured)
        if let Err(e) = self
            .cel_validator
            .validate(&token_header, &token_payload.claims)
        {
            tracing::info!(
                "CEL validation failed (expression='{}'): {}",
                self.cel_validator.expression(),
                e
            );
            return Ok(TokenAuthorizerResponse::deny(&event.method_arn));
        }

        let principal_id = self
            .principal_id_claims
            .get_principal_id_from_claims(&token_payload.claims);

        Ok(TokenAuthorizerResponse::allow(
            &principal_id,
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
            cel_validator: self.cel_validator,
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

    fn make_simple_handler() -> Handler {
        let key_storage = Box::leak(Box::new(KeysStorage::new(
            Url::parse("http://localhost").unwrap(),
            Duration::try_seconds(600).unwrap(),
        )));
        let principal_id_claims =
            Box::leak(Box::new(PrincipalIDClaims::from_comma_separated_values(
                "preferred_username, sub",
                "unknown".to_string(),
            )));
        let accepted_issuers = Box::leak(Box::new(AcceptedClaims::from_comma_separated_values(
            "http://localhost",
            "iss".to_string(),
        )));
        let accepted_audiences = Box::leak(Box::new(AcceptedClaims::from_comma_separated_values(
            "test-app",
            "aud".to_string(),
        )));
        let accepted_signing_algorithms = Box::leak(Box::default());
        let cel_validator = Box::leak(Box::default());

        Handler::new(
            key_storage,
            principal_id_claims,
            accepted_issuers,
            accepted_audiences,
            accepted_signing_algorithms,
            cel_validator,
        )
    }

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
        // SAFETY: safe to unwrap since (1 hour = 3600 seconds) <= (i64::MAX / 1000)
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();

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
        // SAFETY: safe to unwrap since (600 seconds) <= (i64::MAX / 1000)
        let min_refresh_rate = Duration::try_seconds(600).unwrap();
        let jwks_uri = server_url.clone();
        let principal_id_claims = PrincipalIDClaims::from_comma_separated_values(
            "preferred_username, sub",
            "unknown".to_string(),
        );
        let accepted_issuers = AcceptedClaims::from_comma_separated_values(&iss, "iss".to_string());
        let accepted_audiences =
            AcceptedClaims::from_comma_separated_values(aud, "aud".to_string());
        let accepted_signing_algorithms: AcceptedAlgorithms = Default::default();
        let cel_validator: CelValidator = Default::default();
        let keys = KeysStorage::new(Url::parse(&jwks_uri).unwrap(), min_refresh_rate);
        let mut handler = Handler::new(
            Box::leak(Box::new(keys)),
            Box::leak(Box::new(principal_id_claims)),
            Box::leak(Box::new(accepted_issuers)),
            Box::leak(Box::new(accepted_audiences)),
            Box::leak(Box::new(accepted_signing_algorithms)),
            Box::leak(Box::new(cel_validator)),
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
            response.policy_document.statement.first().unwrap().effect,
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

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_cant_get_token_from_header() {
        let event = TokenAuthorizerEvent {
            authorization_token: "NotBearer sometoken".to_string(),
            method_arn: "some_arn".to_string(),
        };

        let handler = make_simple_handler();
        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
        assert_eq!(statement.resource, "some_arn");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_cant_parse_from_header() {
        let event = TokenAuthorizerEvent {
            authorization_token: "Bearer not_a_jwt".to_string(),
            method_arn: "some_arn".to_string(),
        };
        let handler = make_simple_handler();

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
        assert_eq!(statement.resource, "some_arn");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_the_token_uses_an_unsupported_algorithm() {
        let iss = "some_iss";
        let aud = "test-app";
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header =
            serde_json::from_value(json!({ "alg": Algorithm::RS256, "kid": "somekey" })).unwrap();
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user" }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let accepted_signing_algorithms: AcceptedAlgorithms = "ES256".parse().unwrap();
        let accepted_signing_algorithms = Box::leak(Box::new(accepted_signing_algorithms));
        let mut handler = make_simple_handler();
        handler.accepted_signing_algorithms = accepted_signing_algorithms;

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
        assert_eq!(statement.resource, "some_arn");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_the_token_has_no_kid() {
        let iss = "some_iss";
        let aud = "test-app";
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header =
            serde_json::from_value(json!({ "alg": Algorithm::RS256 })).unwrap();
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user" }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let handler = make_simple_handler();

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
        assert_eq!(statement.resource, "some_arn");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_it_fails_to_retrieve_the_key() {
        // create a mock server that will serve an empty list of jwks
        let server = MockServer::start();
        let _jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body("{\"keys\":[]}");
        });

        let iss = "some_iss";
        let aud = "test-app";
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header =
            serde_json::from_value(json!({ "alg": Algorithm::RS256, "kid": "somekey" })).unwrap();
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user" }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let mut handler = make_simple_handler();
        handler.keys = Box::leak(Box::new(KeysStorage::new(
            Url::parse(&server.url("/")).unwrap(),
            Duration::try_seconds(600).unwrap(),
        )));

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
        assert_eq!(statement.resource, "some_arn");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_it_fails_to_validate_the_token() {
        let server = MockServer::start();
        let _jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    "{{\"keys\":[ {} ]}}",
                    include_str!("../tests/fixtures/keys/rs256/jwk.json")
                ));
        });
        let iss = "some_iss";
        let aud = "test-app";
        // makes the token already expired by 1 hour
        let exp = (Utc::now() - Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header = serde_json::from_value(
            json!({ "alg": Algorithm::RS256, "kid": "test/keys/rs256/public" }),
        )
        .unwrap();
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user" }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let mut handler = make_simple_handler();
        handler.keys = Box::leak(Box::new(KeysStorage::new(
            Url::parse(&server.url("/")).unwrap(),
            Duration::try_seconds(600).unwrap(),
        )));

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
        assert_eq!(statement.resource, "some_arn");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_it_fails_to_validate_the_issuer() {
        let server = MockServer::start();
        let _jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    "{{\"keys\":[ {} ]}}",
                    include_str!("../tests/fixtures/keys/rs256/jwk.json")
                ));
        });
        let iss = "http://notlocalhost"; // invalid issuer
        let aud = "test-app";
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header = serde_json::from_value(
            json!({ "alg": Algorithm::RS256, "kid": "test/keys/rs256/public" }),
        )
        .unwrap();
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user" }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let mut handler = make_simple_handler();
        handler.keys = Box::leak(Box::new(KeysStorage::new(
            Url::parse(&server.url("/")).unwrap(),
            Duration::try_seconds(600).unwrap(),
        )));

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
        assert_eq!(statement.resource, "some_arn");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_it_fails_to_validate_the_audience() {
        let server = MockServer::start();
        let _jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    "{{\"keys\":[ {} ]}}",
                    include_str!("../tests/fixtures/keys/rs256/jwk.json")
                ));
        });
        let iss = "http://localhost";
        let aud = "not-test-app"; // invalid audience
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header = serde_json::from_value(
            json!({ "alg": Algorithm::RS256, "kid": "test/keys/rs256/public" }),
        )
        .unwrap();
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user" }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let mut handler = make_simple_handler();
        handler.keys = Box::leak(Box::new(KeysStorage::new(
            Url::parse(&server.url("/")).unwrap(),
            Duration::try_seconds(600).unwrap(),
        )));

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
        assert_eq!(statement.resource, "some_arn");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_cel_validation_fails() {
        let server = MockServer::start();
        let _jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    "{{\"keys\":[ {} ]}}",
                    include_str!("../tests/fixtures/keys/rs256/jwk.json")
                ));
        });
        let iss = "http://localhost";
        let aud = "test-app";
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header = serde_json::from_value(
            json!({ "alg": Algorithm::RS256, "kid": "test/keys/rs256/public" }),
        )
        .unwrap();
        // Token with email_verified: false - CEL expression will fail
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user", "email_verified": false }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let mut handler = make_simple_handler();
        handler.keys = Box::leak(Box::new(KeysStorage::new(
            Url::parse(&server.url("/")).unwrap(),
            Duration::try_seconds(600).unwrap(),
        )));
        // CEL expression that requires email_verified to be true
        let cel_validator: CelValidator = "claims.email_verified == true".parse().unwrap();
        handler.cel_validator = Box::leak(Box::new(cel_validator));

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
        assert_eq!(statement.resource, "some_arn");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_allows_if_cel_validation_passes() {
        let server = MockServer::start();
        let _jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    "{{\"keys\":[ {} ]}}",
                    include_str!("../tests/fixtures/keys/rs256/jwk.json")
                ));
        });
        let iss = "http://localhost";
        let aud = "test-app";
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header = serde_json::from_value(
            json!({ "alg": Algorithm::RS256, "kid": "test/keys/rs256/public" }),
        )
        .unwrap();
        // Token with email_verified: true - CEL expression will pass
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user", "email_verified": true }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let mut handler = make_simple_handler();
        handler.keys = Box::leak(Box::new(KeysStorage::new(
            Url::parse(&server.url("/")).unwrap(),
            Duration::try_seconds(600).unwrap(),
        )));
        // CEL expression that requires email_verified to be true
        let cel_validator: CelValidator = "claims.email_verified == true".parse().unwrap();
        handler.cel_validator = Box::leak(Box::new(cel_validator));

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Allow");
        assert_eq!(response.principal_id, "some_user");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_denies_if_cel_role_check_fails() {
        let server = MockServer::start();
        let _jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    "{{\"keys\":[ {} ]}}",
                    include_str!("../tests/fixtures/keys/rs256/jwk.json")
                ));
        });
        let iss = "http://localhost";
        let aud = "test-app";
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header = serde_json::from_value(
            json!({ "alg": Algorithm::RS256, "kid": "test/keys/rs256/public" }),
        )
        .unwrap();
        // Token with roles that don't include "admin"
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user", "roles": ["user", "reader"] }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let mut handler = make_simple_handler();
        handler.keys = Box::leak(Box::new(KeysStorage::new(
            Url::parse(&server.url("/")).unwrap(),
            Duration::try_seconds(600).unwrap(),
        )));
        // CEL expression that requires admin role
        let cel_validator: CelValidator =
            r#"claims.roles.exists(r, r == "admin")"#.parse().unwrap();
        handler.cel_validator = Box::leak(Box::new(cel_validator));

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Deny");
    }

    #[tokio::test]
    #[traced_test]
    async fn it_allows_if_cel_role_check_passes() {
        let server = MockServer::start();
        let _jwks_mock = server.mock(|when, then| {
            when.method(GET).path("/");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(
                    "{{\"keys\":[ {} ]}}",
                    include_str!("../tests/fixtures/keys/rs256/jwk.json")
                ));
        });
        let iss = "http://localhost";
        let aud = "test-app";
        let exp = (Utc::now() + Duration::try_hours(1).unwrap()).timestamp();
        let encoding_key =
            EncodingKey::from_rsa_pem(include_bytes!("../tests/fixtures/keys/rs256/private.pem"))
                .unwrap();
        let token_header: Header = serde_json::from_value(
            json!({ "alg": Algorithm::RS256, "kid": "test/keys/rs256/public" }),
        )
        .unwrap();
        // Token with roles that include "admin"
        let token = jsonwebtoken::encode(
            &token_header,
            &json!({ "iss": iss, "aud": aud, "exp": exp, "sub": "some_user", "preferred_username": "some_user", "roles": ["user", "admin"] }),
            &encoding_key,
        ).unwrap();
        let event = TokenAuthorizerEvent {
            authorization_token: format!("Bearer {}", token),
            method_arn: "some_arn".to_string(),
        };
        let mut handler = make_simple_handler();
        handler.keys = Box::leak(Box::new(KeysStorage::new(
            Url::parse(&server.url("/")).unwrap(),
            Duration::try_seconds(600).unwrap(),
        )));
        // CEL expression that requires admin role
        let cel_validator: CelValidator =
            r#"claims.roles.exists(r, r == "admin")"#.parse().unwrap();
        handler.cel_validator = Box::leak(Box::new(cel_validator));

        let response = handler.do_call(event).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        let statement = response.policy_document.statement.first().unwrap();
        assert_eq!(statement.effect, "Allow");
        assert_eq!(response.principal_id, "some_user");
    }
}
