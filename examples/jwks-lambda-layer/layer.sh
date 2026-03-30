#!/usr/bin/env bash
set -euo pipefail

# Downloads JWKS from your OIDC provider, packages it as a Lambda layer,
# and attaches it to your authorizer function.
# See the README.md in this directory for full context and alternative approaches.

JWKS_URI="<YOUR_JWKS_URI_HERE>"
AUTH_LAMBDA_FUNCTION_NAME="<YOUR_LAMBDA_FUNCTION_NAME_HERE>"
LAMBDA_LAYER_NAME="<YOUR_LAMBDA_LAYER_NAME_HERE>"

WORK_DIR=$(mktemp -d)
trap 'rm -rf "${WORK_DIR}"' EXIT

echo "Downloading JWKS from ${JWKS_URI}..."
curl -sf -o "${WORK_DIR}/jwks.json" "${JWKS_URI}"

echo "Packaging layer..."
(cd "${WORK_DIR}" && zip -j jwks-layer.zip jwks.json)

echo "Publishing layer..."
layer_version=$(aws lambda publish-layer-version \
  --output json \
  --no-cli-pager \
  --no-cli-auto-prompt \
  --layer-name "${LAMBDA_LAYER_NAME}" \
  --zip-file "fileb://${WORK_DIR}/jwks-layer.zip")

layer_version_arn=$(echo "${layer_version}" | jq -r '.LayerVersionArn')
echo "Published layer: ${layer_version_arn}"

echo "Attaching layer to ${AUTH_LAMBDA_FUNCTION_NAME}..."
aws lambda update-function-configuration \
  --output json \
  --no-cli-pager \
  --no-cli-auto-prompt \
  --function-name "${AUTH_LAMBDA_FUNCTION_NAME}" \
  --layers "${layer_version_arn}" \
  | jq '{FunctionName, Layers}'

echo ""
echo "Done! Set the following environment variable on your authorizer Lambda:"
echo "  JWKS_PRE_CACHED_FILE_PATH=/opt/jwks.json"
