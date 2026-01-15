# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build the project (standard Rust build)
cargo build --release

# Build for AWS Lambda deployment (requires cargo-lambda)
cargo lambda build --locked --release

# Run tests
cargo test

# Run a single test
cargo test <test_name>

# Run tests with output visible
cargo test -- --nocapture

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Run linter
cargo clippy -- -D warnings

# Generate code coverage (requires cargo-llvm-cov)
cargo llvm-cov --all-features --lcov --output-path lcov.info
```

## Architecture

This is an AWS Lambda authorizer for API Gateway that validates OIDC-issued JWT tokens. Written in Rust for optimal cold start performance and low memory usage.

### Core Flow

1. **main.rs** - Entry point. Reads environment variables, initializes the `KeysStorage` and `Handler`, then starts the Lambda runtime.

2. **handler.rs** - Implements the Lambda `Service` trait. The `do_call` method orchestrates the validation:
   - Parses Bearer token from Authorization header
   - Decodes JWT header to extract `kid` and `alg`
   - Validates signing algorithm against accepted list
   - Retrieves public key from JWKS endpoint
   - Validates token (signature, expiration, issuer, audience)
   - Returns Allow/Deny policy document

3. **keys_storage.rs** - Async JWKS key cache with rate-limited refresh. Uses `RwLock` for concurrent access. Fetches keys from OIDC provider's JWKS endpoint and caches them.

4. **keysmap.rs** - Wraps `HashMap<String, DecodingKey>` for storing decoded public keys by `kid`.

5. **models.rs** - API Gateway authorizer request/response types (`TokenAuthorizerEvent`, `TokenAuthorizerResponse`).

### Supporting Modules

- **accepted_algorithms.rs** - Validates JWT signing algorithms against configured allow-list
- **accepted_claims.rs** - Validates `iss` and `aud` claims
- **principalid_claims.rs** - Extracts principal ID from token claims
- **parse_token_from_header.rs** - Extracts Bearer token from Authorization header

### Key Design Decisions

- Static lifetime references (`Box::leak`) for handler dependencies to satisfy Lambda runtime requirements
- Optimistic JWKS caching - keys refresh only on cache miss, rate-limited by `MIN_REFRESH_RATE`
- Allow policy uses wildcard resource (`*`) to avoid cache conflicts across endpoints

## Environment Variables

- `JWKS_URI` (required) - OIDC provider's JWKS endpoint URL
- `ACCEPTED_ISSUERS` - Comma-separated list of valid `iss` values
- `ACCEPTED_AUDIENCES` - Comma-separated list of valid `aud` values
- `ACCEPTED_ALGORITHMS` - Comma-separated list of valid signing algorithms (ES256, RS256, etc.)
- `MIN_REFRESH_RATE` - Seconds between JWKS refreshes (default: 900)
- `PRINCIPAL_ID_CLAIMS` - Claims to use for principal ID (default: "preferred_username, sub")
- `DEFAULT_PRINCIPAL_ID` - Fallback principal ID (default: "unknown")

## Testing

Tests use `httpmock` to simulate JWKS endpoints. Test fixtures in `tests/fixtures/keys/` contain key pairs for various algorithms (RS256, RS384, RS512, ES256, ES384, PS256, PS384, PS512, EdDSA).
