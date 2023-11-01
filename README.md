# oidc-authorizer

[![Rust](https://github.com/lmammino/oidc-authorizer/actions/workflows/rust.yml/badge.svg)](https://github.com/lmammino/oidc-authorizer/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/lmammino/oidc-authorizer/graph/badge.svg?token=P9bsrl4YQB)](https://codecov.io/gh/lmammino/oidc-authorizer)
[![Published on SAR: Serverless Application Repository](https://img.shields.io/badge/SAR-Serverless%20Application%20Repository-FFB71B?style=solid&logo=amazonaws)](https://serverlessrepo.aws.amazon.com/applications/eu-west-1/795006566846/oidc-authorizer)


A high-performance token-based API Gateway authorizer Lambda that can validate OIDC-issued JWT tokens.

![An industrious otter as a logo for this project. Generated with Stable Diffusion (prompt: Intricate illuminated otter made of blown glass catch a fish in the air, breathtaking borderland fantasycore artwork by Android Jones, Jean Baptiste monge, Alberto Seveso, Erin Hanson, Jeremy Mann. maximalist highly detailed and intricate professional_photography, a masterpiece, 8k resolution concept art, Artstation, triadic colors, Unreal Engine 5, cgsociety)](https://github.com/lmammino/oidc-authorizer/raw/main/docs/logo.png)


## 🤌 Use case

This project provides a easy-to-install AWS Lambda function that can be used as a custom authorizer for AWS API Gateway. This authorizer can validate OIDC-issued JWT tokens and it can be used to secure your API endpoints using your OIDC provider of choice (e.g. Apple, Auth0, AWS Cognito, Azure AD / Micsosoft Entra ID, Facebook, GitLab, Google, Keycloak, LinkedIn, Okta, Salesforce, Twitch, etc.).

![A diagram illustrating how this project can be integrated. A user sends an authenticated request to API Gateway. API Gateway is configure to use a custom lambda as an authorizer (THIS PROJECT!). The lambda talks with your OIDC provider to get the public key to validate the user token and responds to API Gateway to Allow or Deny the request.](https://github.com/lmammino/oidc-authorizer/raw/main/docs/lovely-diagram.png)

> A diagram illustrating how this project can be integrated.
>
> A user sends an authenticated request to API Gateway. API Gateway is configure to use a custom lambda as an authorizer (THIS PROJECT!). The lambda talks with your OIDC provider to get the public key to validate the user token and responds to API Gateway to Allow or Deny the request.

API Gateway currently exists in 2 flavours: **HTTP APIs** and **REST APIs**. As of today, only HTTP APIs implement a [built-in JWT authorizer](https://docs.aws.amazon.com/apigateway/latest/developerguide/http-api-jwt-authorizer.html) that supports OIDC-issued tokens.

You might want to consider using this project in the following cases:

- You are using REST APIs and you want to secure your endpoints using OIDC-issued tokens. For instance, if you want to build APIs that are only available in a private VPC, you are currently forced to use REST APIs.
- You are using HTTP APIs but your OIDC provider gives you tokens that are not signed with the RSA algorithm (currently the only one supported by the built-in JWT authorizer).
- You want more flexibility in the validation process of your tokens. For instance, you might want to validate the `aud` claim of your tokens against a list of values, instead of a single value (which is the only option available with the built-in JWT authorizer).
- You want to customise the validation process even further. In this case, you can fork this project and customise the validation logic to your needs.


## ⚽️ Design goals

This custom Lambda Authorizer is designed to be **easy to install and configure**, **cheap**, **highly performant**, and **memory-efficient**. It is currently written in Rust, which is currently the fastest lambda Runtime in terms of cold start and it produces binaries that can provide best-in-class execution performance and a low memory footprint. Rust makes it also easy to compile the Authorizer Lambda for ARM, which helps even further with performance and cost. Ideally this Lambda, should provide minimal cost, even when used to protect Lambda functions that are invoked very frequently.


## 🚀 Installation

This project is meant to be integrated into existing applications (after all, an authorizer is useless without an API).



Different deployment options are available. Check out the [deployment docs](https://github.com/lmammino/oidc-authorizer/blob/main/docs/deploy.md) for an extensive explaination of all the possible approaches.

Alternatively, you can also consult some of the quick examples listed below:

- [Deploy from SAR (Serverless Application Repository) using SAM](https://github.com/lmammino/oidc-authorizer/blob/main/examples/sam-from-sar/template.yml)
- TODO: Deploy from SAR (Serverless Application Repository) using CDK
- TODO: build and package yourself
- TODO: use pre-published binaries and package yourself
- [Build yourself and deploy using SAM](https://github.com/lmammino/oidc-authorizer/blob/main/examples/sam/template.yml)
- TODO: use pre-published binaries and deploy using CDK
- TODO: use pre-published binaries and deploy using Terraform
- TODO: use pre-published binaries and deploy using CloudFormation one-click templates

If you prefer, you can also learn [how to host your own SAR application](https://github.com/lmammino/oidc-authorizer/blob/main/docs/deploy.md#maintain-your-own-sar-application).


## 🛠️ Configuration

The authorizer needs to be configured to be adapted to your needs and to be able to communicate with your OIDC provider of choice.

Here's a list of the configuration options that are supported:

### JwksUri

- **Environment variable**: `JWKS_URI`
- **Description**: The URL of the OIDC provider JWKS (Endpoint providing public keys for verification).
- **Mandatory**: Yes


### MinRefreshRate

- **Environment variable**: `MIN_REFRESH_RATE`
- **Description**: The minumum number of seconds to wait before keys are refreshed when the given key is not found.
- **Mandatory**: No
- **Default value**: `"900"` (15 minutes)

### PrincipalIdClaims

- **Environment variable**: `PRINCIPAL_ID_CLAIMS`
- **Description**: A comma-separated list of claims defining the token fields that should be used to determine the principal Id from the token. The fields will be tested in order. If there's no match the value specified in the `DefaultPrincipalId` parameter will be used.
- **Mandatory**: No
- **Default value**: `"preferred_username, sub"`

### DefaultPrincipalId

- **Environment variable**: `DEFAULT_PRINCIPAL_ID`
- **Description**: A fallback value for the Principal ID to be used when a principal ID claim is not found in the token.
- **Mandatory**: No
- **Default value**: `"unknown"`

### AcceptedIssuers

- **Environment variable**: `ACCEPTED_ISSUERS`
- **Description**: A comma-separated list of accepted values for the `iss` claim. If one of the provided values matches, the token issuer is considered valid. If left empty, any issuer will be accepted.
- **Mandatory**: No
- **Default value**: `""`

### AcceptedAudiences

- **Environment variable**: `ACCEPTED_AUDIENCES`
- **Description**: A comma-separated list of accepted values for the `aud` claim. If one of the provided values matches, the token audience is considered valid. If left empty, any issuer audience be accepted.
- **Mandatory**: No
- **Default value**: `""`

### AcceptedAlgorithms

- **Environment variable**: `ACCEPTED_ALGORITHMS`
- **Description**: A comma-separated list of accepted signing algorithms. If one of the provided values matches, the token signing algorithm is considered valid. If left empty, any supported token signing algorithm is accepted. Supported values: `ES256`, `ES384`, `RS256`, `RS384`, `PS256`, `PS384`, `PS512`, `RS512`, `EdDSA`
- **Mandatory**: No
- **Default value**: `""`


## 🛑 Validation Flow

The following section describes the steps that are followed to validate a token:

  1. The token is parsed from the `Authorization` header of the request. It is expected to be in the form `Bearer <token>`, where `<token>` needs to be a valid JWT token.
  2. The token is decoded and the header is parsed to extract the `kid` (key id) and the `alg` (algorithm) claims. If the `kid` is not found, the token is rejected. If the `alg` is not supported, the token is rejected.
  3. The `kid` is used to look up the public key in the JWKS (JSON Web Key Set) provided by the OIDC provider. If the key is not found, the key is refreshed and the lookup is retried. If the key is still not found, the token is rejected. The JWKS cache is optimistic, it does not automatically refresh keys unless a lookup fails. It also does not auto-refresh keys too often (to avoid unnecessary calls to the JWKS endpoint). You can configure the minimum refresh rate (in seconds) using the `MIN_REFRESH_RATE` environment variable.
  4. The token is decoded and validated using the public key. If the validation fails, the token is rejected. This validation also checks the `exp` (expiration time) claim and the `nbf` (not before) claim. If the token is expired or not yet valid, the token is rejected.
  5. The `iss` (issuer) claim is checked against the list of accepted issuers. If the issuer is not found in the list, the token is rejected. If the list is empty, any issuer is accepted.
  6. The `aud` (audience) claim is checked against the list of accepted audiences. If the audience is not found in the list, the token is rejected. If the list is empty, any audience is accepted.
  7. If all these checks are passed, the token is considered valid and the request is allowed to proceed. The principal ID is extracted from the token using the list of principal ID claims. If no principal ID claim is found, the default principal ID is used.


## 🤑 Context Enrichment

The authorizer enriches the context of the request with the following values:

- `principalId`: the principal ID extracted from the token.
- `jwtClaims`: a JSON string containing the entire token payload (claims).

These values are injected into the context of the request and can be used to enrich your logging, tracing  or to implement app-level authentication.

When you use the [Lambda-proxy integration](https://docs.aws.amazon.com/apigateway/latest/developerguide/set-up-lambda-proxy-integrations.html#api-gateway-create-api-as-simple-proxy) these values are made available under `event.requestContext.authorizer`.

For example this is how you can access the `principalId` and `jwtClaims` values in a Lambda function written in Python:

```python
import json

def handler(event, context):
  print('principalId: ')
  print(event['requestContext']['authorizer']['principalId'])

  print('jwtClaims: ')
  jwtClaims = json.loads(event['requestContext']['authorizer']['jwtClaims'])
  print(jwtClaims)

  return {'body': 'Hello', 'statusCode': 200}
```


## 🏃‍♂️ Benchmarks

Proper benchmarks are yet to be written (SORRY 😇), but for now, to prove that this Lambda is still reasonable fast, here's some data observed during some manual tests (128 MB Memory deployment):

- Cold start times: ~48ms
- Cold start requests (including fetching JWKS from Azure): 120-300ms
- Warm requests (with JWKS in cache): ~10ms
- Actual memory consumption: ~19 MB


## 🙌 Contributing

Everyone is very welcome to contribute to this project.
You can contribute just by submitting bugs or suggesting improvements by
[opening an issue on GitHub](https://github.com/lmammino/oidc-authorizer/issues).


## 👨‍⚖️ License

Licensed under [MIT License](LICENSE). © Luciano Mammino.


## 🙏 Acknowledgements

Big thanks to:

- [@Lodewyk11](https://github.com/Lodewyk11) & [@gsingh1](https://github.com/gsingh1) for writing the original Python implementation that inspired this work.
- [@eoinsha](https://github.com/eoinsha) for suggesting [various ways to package and distribute this project](https://awsbites.com/101-package-and-distribute-lambda-functions-for-fun-and-profit/).
- [@allevo](https://github.com/allevo) for tons of great Rust suggestions.
- [@alexdebrie](https://github.com/alexdebrie) for his amazing [article on custom ApiGateway Authorizers](https://www.alexdebrie.com/posts/lambda-custom-authorizers/).
