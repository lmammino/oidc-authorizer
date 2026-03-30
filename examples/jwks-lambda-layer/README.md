# Pre-warming the JWKS cache with a Lambda Layer

The oidc-authorizer supports pre-warming its in-memory key cache from a JWKS file on disk at startup. This avoids the initial network call to the OIDC provider's JWKS endpoint, significantly reducing cold start latency.

The recommended way to make a JWKS file available to the Lambda is through a [Lambda layer](https://docs.aws.amazon.com/lambda/latest/dg/chapter-layers.html). When a layer is attached, its contents are extracted to `/opt/` in the Lambda execution environment.

## When to use this script

**Use `layer.sh`** when you manage layers outside your IaC stack, for example:

- CI/CD pipelines that periodically refresh the JWKS layer
- Automated rotation triggered by log events (see [Automated layer rotation](#automated-layer-rotation) below)
- One-off manual updates

**Use SAM or CDK instead** if the layer is part of your infrastructure-as-code stack. Both tools can create layers from a local file in a single deployment:

- **SAM**: use `AWS::Serverless::LayerVersion` with `ContentUri` pointing to a local directory — see [`examples/sam/template.yml`](../sam/template.yml) and [`examples/sam-from-sar/template.yml`](../sam-from-sar/template.yml)
- **CDK**: use `aws_lambda.LayerVersion` with `Code.fromAsset()` — see [`examples/cdk-from-sar/lib/cdk-stack.ts`](../cdk-from-sar/lib/cdk-stack.ts)

## How it works

The [`layer.sh`](./layer.sh) script:

1. Downloads the JWKS JSON from your OIDC provider's endpoint
2. Packages it into a zip file (the file will be available at `/opt/jwks.json` in Lambda)
3. Publishes the zip as a new Lambda layer version
4. Attaches the layer to your authorizer Lambda function

After running the script, set the `JWKS_PRE_CACHED_FILE_PATH` environment variable on your authorizer Lambda to `/opt/jwks.json`.

### Prerequisites

- [AWS CLI](https://aws.amazon.com/cli/) (configured with appropriate permissions)
- `curl`, `zip`, `jq`

### Usage

Edit the variables at the top of `layer.sh` and run:

```bash
bash layer.sh
```

## Automated layer rotation

The pre-cached JWKS file is a snapshot — if the OIDC provider rotates its keys (i.e. when a token signed by a new key is encountered), the file becomes stale. The authorizer handles this gracefully by falling back to a network fetch, but cold starts will temporarily lose the pre-warming benefit until the layer is updated.

To automate layer updates, the authorizer emits a structured log line when it detects a cache miss on a pre-warmed cache:

```
event_type=jwks_refresh_needed jwks_uri=https://... key_id=...
  Pre-warmed JWKS cache miss. Keys refreshed from JWKS endpoint. Consider updating the pre-cached JWKS file.
```

You can use this log event to trigger an automated layer update:

1. Create a [CloudWatch Logs subscription filter](https://docs.aws.amazon.com/AmazonCloudWatch/latest/logs/SubscriptionFilters.html) on the authorizer's log group, matching the pattern `"jwks_refresh_needed"`
2. Route the event to a target that regenerates the layer (e.g., a Lambda function that runs this script, an SNS topic, or a Step Function)

This makes layer updates event-driven: the layer is regenerated only when the OIDC provider actually rotates keys, rather than on a fixed schedule.
