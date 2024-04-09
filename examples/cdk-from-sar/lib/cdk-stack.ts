import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as aws_apigw from 'aws-cdk-lib/aws-apigateway';
import * as aws_lambda from 'aws-cdk-lib/aws-lambda';

export class CdkStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // import the authorizer lambda for the Serverless Application Repository
    const authorizerApp = new cdk.aws_sam.CfnApplication(this, 'AuthorizerApp', {
      location: {
        applicationId: 'arn:aws:serverlessrepo:eu-west-1:795006566846:applications/oidc-authorizer',
        semanticVersion: '0.0.9' // 👀 CHANGE ME
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
  }
}
