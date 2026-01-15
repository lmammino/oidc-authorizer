use accepted_algorithms::AcceptedAlgorithms;
use accepted_claims::AcceptedClaims;
use cel_validation::CelValidator;
use chrono::Duration;
use keys_storage::KeysStorage;
use lambda_runtime::{run, tracing, Error};
use principalid_claims::PrincipalIDClaims;
use reqwest::Url;
use std::env;
mod accepted_algorithms;
mod accepted_claims;
mod cel_validation;
mod handler;
mod keys_storage;
mod keysmap;
mod models;
mod parse_token_from_header;
mod principalid_claims;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let jwks_uri = env::var("JWKS_URI")?;
    let jwks_uri: Url = jwks_uri.parse()?;
    let min_refresh_rate = env::var("MIN_REFRESH_RATE").unwrap_or("900".to_string());
    let min_refresh_rate =
        Duration::try_seconds(min_refresh_rate.parse::<u64>()? as i64).ok_or(format!(
            "Invalid MIN_REFRESH_RATE value provided. '{}' should be less than {}",
            min_refresh_rate,
            i64::MAX / 1000
        ))?;
    let principal_id_claims =
        env::var("PRINCIPAL_ID_CLAIMS").unwrap_or("preferred_username, sub".to_string());
    let default_principal_id = env::var("DEFAULT_PRINCIPAL_ID").unwrap_or("unknown".to_string());
    let principal_id_claims = PrincipalIDClaims::from_comma_separated_values(
        principal_id_claims.as_str(),
        default_principal_id,
    );
    let accepted_issuers = env::var("ACCEPTED_ISSUERS").unwrap_or_default();
    let accepted_issuers =
        AcceptedClaims::from_comma_separated_values(accepted_issuers.as_str(), "iss".to_string());
    let accepted_audiences = env::var("ACCEPTED_AUDIENCES").unwrap_or_default();
    let accepted_audiences: AcceptedClaims =
        AcceptedClaims::from_comma_separated_values(accepted_audiences.as_str(), "aud".to_string());
    let accepted_signing_algorithms = env::var("ACCEPTED_ALGORITHMS").unwrap_or_default();
    let accepted_signing_algorithms: AcceptedAlgorithms = accepted_signing_algorithms.parse()?; // infallible
    let token_validation_cel = env::var("TOKEN_VALIDATION_CEL").unwrap_or_default();
    let cel_validator: CelValidator = token_validation_cel.parse()?;

    tracing::init_default_subscriber();

    let keys = KeysStorage::new(jwks_uri, min_refresh_rate);
    run(handler::Handler::new(
        Box::leak(Box::new(keys)),
        Box::leak(Box::new(principal_id_claims)),
        Box::leak(Box::new(accepted_issuers)),
        Box::leak(Box::new(accepted_audiences)),
        Box::leak(Box::new(accepted_signing_algorithms)),
        Box::leak(Box::new(cel_validator)),
    ))
    .await
}
