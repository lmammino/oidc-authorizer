JWKS_URI="<YOUR_JWKS_URI_HERE>"
AUTH_LAMBDA_FUNCTION_NAME="<YOUR_LAMBDA_FUNCTION_NAME_HERE>"
LAMBDA_LAYER_NAME="<YOUR_LAMBDA_LAYER_NAME_HERE>"
JWKS_CACHE_DIR="./.jwks-cache"


echo "--------------------------------"
echo "Current directory: $(pwd)"
echo "--------------------------------"
echo "Downloading JWKS from $JWKS_URI"
mkdir -p $JWKS_CACHE_DIR
curl -o $JWKS_CACHE_DIR/jwks.json $JWKS_URI
echo ""
echo "--------------------------------"
echo "Zipping JWKS"
# create a new zip file
rm -f jwks-layer.zip
zip -FSr jwks-layer.zip $JWKS_CACHE_DIR/
echo ""
echo "--------------------------------"
unzip -l jwks-layer.zip
echo ""
echo "--------------------------------"
echo "Publishing layer to AWS..."
echo ""
layer_version=$(aws lambda publish-layer-version \
  --output json \
  --no-cli-pager \
  --no-cli-auto-prompt \
  --layer-name $LAMBDA_LAYER_NAME \
  --zip-file fileb://jwks-layer.zip)

if [ $? -ne 0 ]; then
  echo "Failed to publish layer."
  exit 1
fi

echo $layer_version | jq

layer_version_arn=$(echo $layer_version | jq -r '.LayerVersionArn')

echo ""
echo "--------------------------------"
echo "Configuring version arn on authorizer lambda."
echo "Layer ARN: $layer_version_arn"
echo "Target Function: $AUTH_LAMBDA_FUNCTION_NAME"

aws lambda update-function-configuration \
  --output json \
  --no-cli-pager \
  --no-cli-auto-prompt \
  --function-name $AUTH_LAMBDA_FUNCTION_NAME \
  --layers $layer_version_arn \
  | jq

echo ""


echo "--------------------------------"
echo "FYI:"
echo "--------------------------------"
echo "When this layer is configured on the lambda function, the zip file will be extracted to /opt/"
echo "So, the 'JWKS_PRE_CACHED_FILE_PATH' would be '/opt/.jwks-cache/jwks.json'"
echo "--------------------------------"
echo "TODO: Conditional publish of layer based on if the jwks contents have changed or not is left to you."
echo "--------------------------------"
