# Deploy

There are a few different ways of getting and using the binary for this Lambda function:

  1. [From SAR (Serverless Application Repository)](#deploy-from-sar-serverless-application-repository)
  2. [Download a pre-built binary from GitHub](#download-a-pre-built-binary-from-github)
  3. [Build the binary yourself](#build-the-binary-yourself)
  4. [Other approaches](#other-approaches)


## Deploy from SAR (Serverless Application Repository)

This Lambda is [hosted on SAR](https://serverlessrepo.aws.amazon.com/applications/eu-west-1/795006566846/oidc-authorizer) which means you can deploy it directly from there.

### Use the SAR application with SAM:

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Transform: AWS::Serverless-2016-10-31
Description: AWS SAM template with a simple API definition

Resources:
  # The oidc-authorizer imported from SAR
  oidcauthorizer:
    Type: AWS::Serverless::Application
    Properties:
      Location:
        ApplicationId: arn:aws:serverlessrepo:eu-west-1:795006566846:applications/oidc-authorizer
        SemanticVersion: 0.0.7 # ⬅️ CHANGE: SPECIFY THE EXACT VERSION
      Parameters:
        # 👀 CHANGE THE FOLLOWING PARAMETERS
        AcceptedAlgorithms: ""
        AcceptedAudiences: ""
        AcceptedIssuers: ""
        DefaultPrincipalId: "unknown"
        JwksUri: "THE ENDPOINT OF YOUR OIDC PROVIDER JWKS"
        MinRefreshRate: "900"
        PrincipalIdClaims: "preferred_username, sub"
        # The amount of memory (in MB) to give to the authorizer Lambda.
        LambdaMemorySize: "128"
        # The timeout to give to the authorizer Lambda.
        LambdaTimeout: "3"

  # YOUR APIs HERE
  ApiGatewayApi:
    Type: AWS::Serverless::Api
    Properties:
      StageName: prod
      Auth:
        DefaultAuthorizer: OidcAuthorizer
        Authorizers:
          OidcAuthorizer:
            FunctionArn: !GetAtt oidcauthorizer.Outputs.OidcAuthorizerArn # ⬅️ This is how your reference the actual lambda deployed by the SAR app
```

To deploy this stack with sam you'll need to enable the following capabilites: `CAPABILITY_AUTO_EXPAND`, `CAPABILITY_NAMED_IAM`, and `CAPABILITY_IAM`.

A full example is available in the [`examples` folder](https://github.com/lmammino/oidc-authorizer/blob/main/examples/sam-from-sar/template.yml).

### Use the SAR application with CDK:

The following snippet shows how to use the SAR application with CDK (using Typescript):

```typescript
// import the authorizer lambda for the Serverless Application Repository
const authorizerApp = new cdk.aws_sam.CfnApplication(this, 'AuthorizerApp', {
  location: {
    applicationId: 'arn:aws:serverlessrepo:eu-west-1:795006566846:applications/oidc-authorizer',
    semanticVersion: '0.0.7' // 👀 CHANGE ME
  },
  parameters: {
    // 👀 CHANGE THE FOLLOWING PARAMETERS
    AcceptedAlgorithms: "",
    AcceptedAudiences: "",
    AcceptedIssuers: "",
    DefaultPrincipalId: "unknown",
    JwksUri: "https://login.microsoftonline.com/3e4abf5a-fdc9-485c-9853-af03c4a32976/discovery/v2.0/keys",
    MinRefreshRate: "900",
    PrincipalIdClaims: "preferred_username, sub",
    // The amount of memory (in MB) to give to the authorizer Lambda.
    LambdaMemorySize: "128",
    // The timeout to give to the authorizer Lambda.
    LambdaTimeout: "3"
  }
})

const lambdaAuthorizer = aws_lambda.Function.fromFunctionAttributes(this, 'AuthorizerFunction', {
  functionArn: authorizerApp.getAtt('Outputs.OidcAuthorizerArn').toString(),
  sameEnvironment: true, // Note: this is important since the lambda is created in another stack we need to make sure CDK knows it's in the same region
})

// creates the authorizer definition
const authorizer = new aws_apigw.TokenAuthorizer(this, 'Authorizer', {
  handler: lambdaAuthorizer,
  identitySource: aws_apigw.IdentitySource.header('authorization'),
  authorizerName: 'OidcAuthorizer',
});

// Your API is here
const apiGw = new aws_apigw.RestApi(this, 'api', {
  restApiName: 'OIDC Authorizer Demo',
  description: 'A demo app to test the OIDC authorizer',
  deployOptions: {
    stageName: 'prod',
  },
  defaultCorsPreflightOptions: {
    allowOrigins: aws_apigw.Cors.ALL_ORIGINS,
    allowMethods: aws_apigw.Cors.ALL_METHODS,
  },
  endpointTypes: [aws_apigw.EndpointType.REGIONAL],
  deploy: true,
});

const sampleApiLambda1 = new aws_lambda.Function(this, 'sampleApiLambda1', {
  runtime: aws_lambda.Runtime.PYTHON_3_9,
  handler: 'index.handler',
  code: aws_lambda.Code.fromInline(`
def handler(event, context):
return {'body': 'Hello from endpoint1!', 'statusCode': 200}
`)
});

const sampleApiLambda2 = new aws_lambda.Function(this, 'sampleApiLambda2', {
  runtime: aws_lambda.Runtime.PYTHON_3_9,
  handler: 'index.handler',
  code: aws_lambda.Code.fromInline(`
def handler(event, context):
return {'body': 'Hello ' + event['requestContext']['authorizer']['principalId'] + ' from endpoint2! These are your claims: ' + event['requestContext']['authorizer']['jwtClaims'], 'statusCode': 200}
`)
});

apiGw
  .root
  .addResource('1')
  .addMethod('GET', new aws_apigw.LambdaIntegration(sampleApiLambda1), {
    authorizer: authorizer,
    authorizationType: aws_apigw.AuthorizationType.CUSTOM,
  });

apiGw
  .root
  .addResource('2')
  .addMethod('GET', new aws_apigw.LambdaIntegration(sampleApiLambda2), {
    authorizer: authorizer,
    authorizationType: aws_apigw.AuthorizationType.CUSTOM,
  });

const apiGwEndpoint1Output = new cdk.CfnOutput(this, 'ApiEndpoint1', {
  description: 'API Gateway endpoint 1',
  value: `${apiGw.url}1`
});

const apiGwEndpoint2Output = new cdk.CfnOutput(this, 'ApiEndpoint2', {
  description: 'API Gateway endpoint 2',
  value: `${apiGw.url}2`
});
```

A full example is available in the [`examples` folder](https://github.com/lmammino/oidc-authorizer/blob/main/examples/cdk-from-sar/lib/cdk-stacks.ts).


> **Note**
If you don't want to use the public SAR application, you can [publish your own](#maintain-your-own-sar-application).


## Download a pre-built binary from GitHub

Every new release of this project is automatically built and published to GitHub as a release asset.

You can easily download the `ARM64` binary for a given release by using the following URL template:

```plain
https://github.com/lmammino/oidc-authorizer/releases/download/<VERSION>/bootstrap.zip
```

Make sure to replace `<VERSION>` with the actual version you intend to use.

Once you download the binary, you can easily add it to your project and reference it in your SAM template, CDK project, Terraform configuration or whatever you are using to deploy your Lambda functions.

Just make sure to define set the following Lambda properties as follow:

```yaml
CodeUri: . # OR the path to the **directory** containing lambda binary (which needs to be unzipped)
Handler: bootstrap
Runtime: provided.al2
Architectures: [arm64]
```

> **Note**
`x86` binaries are currently not provided. If you want to use those, you have to build them by yourself.


## Build the binary yourself

If you have the [Rust toolchain](https://rustup.rs/) and [Cargo Lambda](https://www.cargo-lambda.info/) installed in your system. You can compile the binary yourself with the following command:

```bash
cargo lambda build --arm64 --release
```

The compiled binary will be available in `target/lambda/oidc-authorizer/bootstrap`.

> **Note**:
`cargo lambda` also allows you to cross-compile for other architectures and operative systems. [Check out the official documentation to learn how to do that](https://www.cargo-lambda.info/guide/cross-compiling.html).


## Other approaches

Other ways of building and deploying the Lambda function.

### Maintain your own SAR application

This application is already published to SAR and [you can deploy it directly from there](#deploy-from-sar-serverless-application-repository).
But, if you want to publish and maintain your own version, here's how you can do it.

#### 1. Create a new S3 bucket and give it the right permissions

```bash
aws s3 mb s3://${YOUR_OWN_BUCKET_NAME}
```

Then apply the following policy to the bucket:

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Principal": {
                "Service": "serverlessrepo.amazonaws.com"
            },
            "Action": "s3:GetObject",
            "Resource": "arn:aws:s3:::${YOUR_OWN_BUCKET_NAME}/*",
            "Condition": {
                "StringEquals": {
                    "aws:SourceAccount": "${YOUR_OWN_ACCOUNT}"
                }
            }
        }
    ]
}
```

Make sure to replace both `${YOUR_OWN_BUCKET_NAME}` and `${YOUR_OWN_ACCOUNT}` with the right values.

#### 2. Build the lambda

Requires the Rust toolchain to be installed, an updated version of `cargo-lambda` and `sam`:

```bash
sam build
sam package --output-template-file .aws-sam/packaged.yml --s3-bucket ${YOUR_OWN_BUCKET_NAME}
```

Make sure to replace `${YOUR_OWN_BUCKET_NAME}` with the right value.

#### 3. Publish the application to SAR

```bash
sam publish --template .aws-sam/packaged.yml --region ${YOUR_OWN_REGION}
```

Make sure to replace `${YOUR_OWN_REGION}` with the right value.