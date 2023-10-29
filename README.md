# oidc-authorizer

[![Rust](https://github.com/lmammino/oidc-authorizer/actions/workflows/rust.yml/badge.svg)](https://github.com/lmammino/oidc-authorizer/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/lmammino/oidc-authorizer/graph/badge.svg?token=P9bsrl4YQB)](https://codecov.io/gh/lmammino/oidc-authorizer)


A high-performance token-based API Gateway authorizer Lambda that can validate OIDC-issued JWT tokens.

![An industrious otter as a logo for this project. Generated with Stable Diffusion (prompt: Intricate illuminated otter made of blown glass catch a fish in the air, breathtaking borderland fantasycore artwork by Android Jones, Jean Baptiste monge, Alberto Seveso, Erin Hanson, Jeremy Mann. maximalist highly detailed and intricate professional_photography, a masterpiece, 8k resolution concept art, Artstation, triadic colors, Unreal Engine 5, cgsociety)](/docs/logo.png)


## Use case

This project provides a easy-to-install AWS Lambda function that can be used as a custom authorizer for AWS API Gateway. This authorizer can validate OIDC-issued JWT tokens and it can be used to secure your API endpoints using your OIDC provider of choice (e.g. Apple, Auth0, AWS Cognito, Azure AD / Micsosoft Entra ID, Facebook, GitLab, Google, Keycloak, LinkedIn, Okta, Salesforce, Twitch, etc.).

API Gateway currently exists in 2 flavours: **HTTP APIs** and **REST APIs**. As of today, only HTTP APIs implement a [built-in JWT authorizer](https://docs.aws.amazon.com/apigateway/latest/developerguide/http-api-jwt-authorizer.html) that supports OIDC-issued tokens.

You might want to consider using this project in the following cases:

- You are using REST APIs and you want to secure your endpoints using OIDC-issued tokens. For instance, if you want to build APIs that are only available in a private VPC, you are currently forced to use REST APIs.
- You are using HTTP APIs but your OIDC provider gives you tokens that are not signed with the RSA algorithm (currently the only one supported by the built-in JWT authorizer).
- You want more flexibility in the validation process of your tokens. For instance, you might want to validate the `aud` claim of your tokens against a list of values, instead of a single value (which is the only option available with the built-in JWT authorizer).
- You want to customise the validation process even further. In this case, you can fork this project and customise the validation logic to your needs.


## Design goals

This custom Lambda Authorizer is designed to be **easy to install and configure**, **cheap**, **highly performant**, and **memory-efficient**. It is currently written in Rust, which is currently the fastest lambda Runtime in terms of cold start and it produces binaries that can provide best-in-class execution performance and a low memory footprint. Rust makes it also easy to compile the Authorizer Lambda for ARM, which helps even further with performance and cost. Ideally this Lambda, should provide minimal cost, even when used to protect Lambda functions that are invoked very frequently.


## ⚠️ WIP

- TODO: add architecture diagram in use case section
- TODO: document installation process
- TODO: document supported algorithms
- TODO: document validation flow
- TODO: document configuration options
- TODO: document what gets added into the context and how it can be used for app-level authentication
- TODO: benchmarks
- TODO: release from GitHub actions


## Contributing

Everyone is very welcome to contribute to this project.
You can contribute just by submitting bugs or suggesting improvements by
[opening an issue on GitHub](https://github.com/lmammino/oidc-authorizer/issues).


## License

Licensed under [MIT License](LICENSE). © Luciano Mammino.