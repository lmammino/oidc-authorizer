# Security Policy

Thank you for helping keep **oidc-authorizer** and its users safe.

This project is a Lambda authorizer intended to validate OIDC-issued JWTs for Amazon API Gateway. Security reports—especially those involving authentication/authorization bypass, token validation, or JWKS handling—are taken seriously.

## Supported Versions

We provide security updates for the latest major version.

> [!NOTE]
> While the project is still in the `0.y.z` versioning phase, the `y` value determines what is the latest major release.

## Reporting a Vulnerability

### Where to report

Please **do not** open a public GitHub issue, pull request, or discussion for suspected security vulnerabilities.

Preferred reporting method:

1. **GitHub Security Advisories / Private Vulnerability Reporting**  
   If this repository shows a **“Report a vulnerability”** option under the **Security** tab, please use it to submit a private report.

Fallback reporting method:

2. **Email**  
   Send a report to: **klr1b1n5n@mozmail.com**  

### What to include

To help us triage quickly, please include:

- A clear description of the issue and the **security impact**
- Steps to reproduce (ideally a minimal PoC)
- Affected versions and any relevant configuration (redact secrets)
- Any relevant logs, stack traces, or packet captures (again, redact secrets)
- Whether you believe the issue is being actively exploited

### What to expect

We aim to follow coordinated vulnerability disclosure:

- **Acknowledgement:** within **3 business days**
- **Initial triage update:** within **10 business days**
- If the issue is accepted, we will work on a fix and coordinate a disclosure timeline with you.
- If the issue is declined, we will explain why (e.g., not a security issue, out of scope, or not reproducible).

### Coordinated disclosure

We request that you keep vulnerability details private until:

- A fix has been released, **or**
- **90 days** have passed since our initial acknowledgement or triage update,
  whichever comes first—unless there is evidence of active exploitation, in which case we may accelerate mitigation and disclosure. The 90‑day period is measured from the time we first acknowledge or provide a substantive triage update for your report, not from the time you initially submit it.

### Scope guidance

**In scope** (examples):
- Authentication/authorization bypass
- JWT validation flaws (e.g., signature verification, `iss`/`aud` validation, algorithm handling)
- JWKS retrieval/caching/rotation issues that can lead to accepting invalid tokens
- Denial-of-service vectors (e.g., pathological inputs causing excessive CPU/memory use)

**Out of scope** (examples):
- Issues caused solely by **misconfiguration** of the deployer’s AWS infrastructure or IAM policies
- Vulnerabilities in AWS services themselves or in upstream OIDC providers
- Social engineering, phishing, or physical attacks

### Recognition

If you would like to be credited for a report, we are happy to acknowledge you in the eventual advisory/release notes. If you prefer to remain anonymous, please tell us.
