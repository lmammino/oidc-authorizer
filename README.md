# oidc-authorizer

[![Rust](https://github.com/lmammino/oidc-authorizer/actions/workflows/rust.yml/badge.svg)](https://github.com/lmammino/oidc-authorizer/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/lmammino/oidc-authorizer/graph/badge.svg?token=P9bsrl4YQB)](https://codecov.io/gh/lmammino/oidc-authorizer)


A high-performance token-based API Gateway authorizer Lambda that can validate OIDC-issued JWT tokens.

![An industrious otter as a logo for this project. Generated with Stable Diffusion (prompt: Intricate illuminated otter made of blown glass catch a fish in the air, breathtaking borderland fantasycore artwork by Android Jones, Jean Baptiste monge, Alberto Seveso, Erin Hanson, Jeremy Mann. maximalist highly detailed and intricate professional_photography, a masterpiece, 8k resolution concept art, Artstation, triadic colors, Unreal Engine 5, cgsociety)](/docs/logo.png)


## Use case

This project provides a easy-to-install AWS Lambda function that can be used as a custom authorizer for AWS API Gateway. This authorizer can validate OIDC-issued JWT tokens and it can be used to secure your API endpoints using your OIDC provider of choice (e.g. Apple, Auth0, AWS Cognito, Azure AD / Micsosoft Entra ID, Facebook, GitLab, Google, Keycloak, LinkedIn, Okta, Salesforce, Twitch, etc.).

![A diagram illustrating how this project can be integrated. A user sends an authenticated request to API Gateway. API Gateway is configure to use a custom lambda as an authorizer (THIS PROJECT!). The lambda talks with your OIDC provider to get the public key to validate the user token and responds to API Gateway to Allow or Deny the request.](/docs/lovely-diagram.png)

> A diagram illustrating how this project can be integrated.
>
> A user sends an authenticated request to API Gateway. API Gateway is configure to use a custom lambda as an authorizer (THIS PROJECT!). The lambda talks with your OIDC provider to get the public key to validate the user token and responds to API Gateway to Allow or Deny the request.

API Gateway currently exists in 2 flavours: **HTTP APIs** and **REST APIs**. As of today, only HTTP APIs implement a [built-in JWT authorizer](https://docs.aws.amazon.com/apigateway/latest/developerguide/http-api-jwt-authorizer.html) that supports OIDC-issued tokens.

You might want to consider using this project in the following cases:

- You are using REST APIs and you want to secure your endpoints using OIDC-issued tokens. For instance, if you want to build APIs that are only available in a private VPC, you are currently forced to use REST APIs.
- You are using HTTP APIs but your OIDC provider gives you tokens that are not signed with the RSA algorithm (currently the only one supported by the built-in JWT authorizer).
- You want more flexibility in the validation process of your tokens. For instance, you might want to validate the `aud` claim of your tokens against a list of values, instead of a single value (which is the only option available with the built-in JWT authorizer).
- You want to customise the validation process even further. In this case, you can fork this project and customise the validation logic to your needs.


## Design goals

This custom Lambda Authorizer is designed to be **easy to install and configure**, **cheap**, **highly performant**, and **memory-efficient**. It is currently written in Rust, which is currently the fastest lambda Runtime in terms of cold start and it produces binaries that can provide best-in-class execution performance and a low memory footprint. Rust makes it also easy to compile the Authorizer Lambda for ARM, which helps even further with performance and cost. Ideally this Lambda, should provide minimal cost, even when used to protect Lambda functions that are invoked very frequently.


## Installation

This project is meant to be integrated into existing applications (after all, an authorizer is useless without an API).

Different deployment options are available:

- TODO: Deploy from SAR (Serverless Application Repository) using SAM
- TODO: Deploy from SAR (Serverless Application Repository) using CDK
- TODO: build and package yourself
- TODO: use pre-published binaries and package yourself
- [Build yourself and deploy using SAM](/examples/sam/template.yml)
- TODO: use pre-published binaries and deploy using CDK
- TODO: use pre-published binaries and deploy using Terraform
- TODO: use pre-published binaries and deploy using CloudFormation one-click templates

If you prefer, you can also learn [how to host your own SAR application](/docs/deploy.md#maintain-your-own-sar-application).


## Configuration

The authorizer needs to be configured to be adapted to your needs and to be able to communicate with your OIDC provider of choice.

Here's a list of the configuration options that are supported:

| **Parameter Name** | **Environment variable** | **Description**                                                                                                                                                                                                                                                                                                    | **Mandatory** | **Default Value**           |
|--------------------|--------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------|-----------------------------|
| JwksUri            | JWKS_URI                 | The URL of the OIDC provider JWKS (Endpoint providing public keys for verification).                                                                                                                                                                                                                               | Y             |                             |
| MinRefreshRate     | MIN_REFRESH_RATE         | The minumum number of seconds to wait before keys are refreshed when the given key is not found.                                                                                                                                                                                                                   | N             | `"900"`                     |
| PrincipalIdClaims  | PRINCIPAL_ID_CLAIMS      | A comma-separated list of claims defining the token fields that should be used to determine the principal Id from the token. The fields will be tested in order. If there's no match the value specified in the `DefaultPrincipalId` parameter will be used.                                                       | N             | `"preferred_username, sub"` |
| DefaultPrincipalId | DEFAULT_PRINCIPAL_ID     | A fallback value for the Principal ID to be used when a principal ID claim is not found in the token.                                                                                                                                                                                                              | N             | `"unknown"`                 |
| AcceptedIssuers    | ACCEPTED_ISSUERS         | A comma-separated list of accepted values for the `iss` claim. If one of the provided values matches, the token issuer is considered valid. If left empty, any issuer will be accepted.                                                                                                                            | N             | `""`                        |
| AcceptedAudiences  | ACCEPTED_AUDIENCES       | A comma-separated list of accepted values for the `aud` claim. If one of the provided values matches, the token audience is considered valid. If left empty, any issuer audience be accepted.                                                                                                                      | N             | `""`                        |
| AcceptedAlgorithms | ACCEPTED_ALGORITHMS      | A comma-separated list of accepted signing algorithms. If one of the provided values matches, the token signing algorithm is considered valid. If left empty, any supported token signing algorithm is accepted. Supported values: `ES256`, `ES384`, `RS256`, `RS384`, `PS256`, `PS384`, `PS512`, `RS512`, `EdDSA` | N             | `""`                        |


## ⚠️ WIP

- TODO: document installation process
- TODO: document supported algorithms
- TODO: document validation flow
- TODO: document what gets added into the context and how it can be used for app-level authentication
- TODO: benchmarks
- TODO: release from GitHub actions


## Contributing

Everyone is very welcome to contribute to this project.
You can contribute just by submitting bugs or suggesting improvements by
[opening an issue on GitHub](https://github.com/lmammino/oidc-authorizer/issues).


## License

Licensed under [MIT License](LICENSE). © Luciano Mammino.


## Acknowledgements

Big thanks to:

- [@Lodewyk11](https://github.com/Lodewyk11) & [@gsingh1](https://github.com/gsingh1) for writing the original Python implementation that inspired this work
- [@allevo](https://github.com/allevo) for tons of great Rust suggestions
- [@alexdebrie](https://github.com/alexdebrie) for his amazing [article on custom ApiGateway Authorizers](https://www.alexdebrie.com/posts/lambda-custom-authorizers/).