use accepted_algorithms::AcceptedAlgorithms;
use accepted_claims::AcceptedClaims;
use chrono::Duration;
use lambda_runtime::{run, Error};
use reqwest::Url;
use std::env;
mod accepted_algorithms;
mod accepted_claims;
mod handler;
mod keys_cache;
mod keysmap;
mod models;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let jws_uri = env::var("JWKS_URI")?;
    let jws_uri: Url = jws_uri.parse()?;
    let min_refresh_rate = env::var("MIN_REFRESH_RATE").unwrap_or("900".to_string());
    let min_refresh_rate = Duration::seconds(min_refresh_rate.parse::<u64>()? as i64);
    let accepted_issuers = env::var("ACCEPTED_ISSUERS").unwrap_or_default();
    let accepted_issuers: AcceptedClaims = accepted_issuers.parse().unwrap(); // infallible
    let accepted_audiences = env::var("ACCEPTED_AUDIENCES").unwrap_or_default();
    let accepted_audiences: AcceptedClaims = accepted_audiences.parse().unwrap(); // infallible
    let accepted_algorithms = env::var("ACCEPTED_ALGORITHMS").unwrap_or_default();
    let accepted_algorithms: AcceptedAlgorithms = accepted_algorithms.parse()?; // infallible

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    run(handler::Handler::new(
        jws_uri,
        min_refresh_rate,
        accepted_issuers,
        accepted_audiences,
        accepted_algorithms,
    ))
    .await
}
