# Changelog

## [0.4.1](https://github.com/lmammino/oidc-authorizer/compare/oidc-authorizer-v0.4.0...oidc-authorizer-v0.4.1) (2026-04-07)


### Features

* add SECURITY.md with reporting and scope details ([#54](https://github.com/lmammino/oidc-authorizer/issues/54)) ([dc1329c](https://github.com/lmammino/oidc-authorizer/commit/dc1329cb0bf4819fdf3453b0104e3f938c6db7e8))
* added proper validation ([37b4725](https://github.com/lmammino/oidc-authorizer/commit/37b4725f669ccc528575642253cf29be65613fcb))
* **algorithm:** makes sure only public-key algs can be selected ([71379f5](https://github.com/lmammino/oidc-authorizer/commit/71379f5915454e7fe8d79472107d979769ca87fb))
* can extract principal id from claims ([f0bda15](https://github.com/lmammino/oidc-authorizer/commit/f0bda15516e8197aca9e81a87c34d77ab1cfc225))
* deny delete creation ([#14](https://github.com/lmammino/oidc-authorizer/issues/14)) ([efbac0c](https://github.com/lmammino/oidc-authorizer/commit/efbac0ca2f3e0ddaa5ed83fd107c47e441b0e96b))
* first commit ([e9d40a0](https://github.com/lmammino/oidc-authorizer/commit/e9d40a0829917560c006ae8811f99481bdeb2bec))
* implement token validation using CEL expressions ([#52](https://github.com/lmammino/oidc-authorizer/issues/52)) ([deb19a2](https://github.com/lmammino/oidc-authorizer/commit/deb19a250cb401770df0976060b5a9d8401ba1bf))
* logging level and added debug logs for keys refresh ([#11](https://github.com/lmammino/oidc-authorizer/issues/11)) ([f72f90f](https://github.com/lmammino/oidc-authorizer/commit/f72f90f1af412a387e9c4071ba3db190aec6fe1b))
* megarefactor by [@allevo](https://github.com/allevo) ([bea134f](https://github.com/lmammino/oidc-authorizer/commit/bea134f183d47dee95bb798a08c0dd35b88b06b8))
* pre-cached JWKS ([#66](https://github.com/lmammino/oidc-authorizer/issues/66)) ([9977277](https://github.com/lmammino/oidc-authorizer/commit/99772777ed00e69148b57a408d00889fa5bd2266))
* **release:** adding automation to publish on SAR on new GH releases ([4a5343d](https://github.com/lmammino/oidc-authorizer/commit/4a5343dda04ea240a341ac97f86ad19c729be329))
* support stack prefixes as parameters ([#13](https://github.com/lmammino/oidc-authorizer/issues/13)) ([f04b2e4](https://github.com/lmammino/oidc-authorizer/commit/f04b2e4c8a2bec7fefe249b75b3030294a83a180))
* supports custom log groups and defines log retention ([#53](https://github.com/lmammino/oidc-authorizer/issues/53)) ([a8937b3](https://github.com/lmammino/oidc-authorizer/commit/a8937b30d9803572f7cd6ecff2e4132603aa2811))
* tests and other improvements ([2bc4ca3](https://github.com/lmammino/oidc-authorizer/commit/2bc4ca3b5fd2f5958fde5aac691f645037ec72ae))


### Bug Fixes

* audit issues ([#49](https://github.com/lmammino/oidc-authorizer/issues/49)) ([0b7cc6e](https://github.com/lmammino/oidc-authorizer/commit/0b7cc6e3021e500c419cccab243f7db01d845455))
* broken release pipeline ([f5012b8](https://github.com/lmammino/oidc-authorizer/commit/f5012b83e2aa96974f6b40abe00eb4663477bca4))
* Bug claim aud is not a string ([#9](https://github.com/lmammino/oidc-authorizer/issues/9)) ([09459db](https://github.com/lmammino/oidc-authorizer/commit/09459db52cc2cac5bc52575e53329351a976c1a6))
* bug with loading keys already in cache ([d058964](https://github.com/lmammino/oidc-authorizer/commit/d0589644a2487766d2e5af14686810f26f6b3315))
* GitHub Actions build produce a broken build ([#2](https://github.com/lmammino/oidc-authorizer/issues/2)) ([00f8adf](https://github.com/lmammino/oidc-authorizer/commit/00f8adfa133c71a0a0d80394bb47bbb11c1a3fb6))
* GitHub Actions build produce a broken build ([#2](https://github.com/lmammino/oidc-authorizer/issues/2)) ([00f8adf](https://github.com/lmammino/oidc-authorizer/commit/00f8adfa133c71a0a0d80394bb47bbb11c1a3fb6))
* usage of invalid usageIdentifierKey ([5e7533b](https://github.com/lmammino/oidc-authorizer/commit/5e7533b47e9f926b226288953fcac39b086b42a1))
* uses loose policy to avoid cache conflicts ([61bc1eb](https://github.com/lmammino/oidc-authorizer/commit/61bc1eb1e78dc554c3a9834f1a0ad1cb67114360))
