# Deploy

## Deploy from SAR (Serverless Application Repository)

TODO:

- Show example with SAM
- Show example with CDK

> **Note**: 
> If you don't want to use the public SAR application, you can [publish your own](#maintain-your-own-sar-application).


## Maintain your own SAR application

This application is already published to SAR and [you can deploy it directly from there](#deploy-from-sar-serverless-application-repository).
But, if you want to publish and maintain your own version, here's how you can do it.

### 1. Create a new S3 bucket and give it the right permissions

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

### 2. Build the lambda

Requires the Rust toolchain to be installed, an updated version of `cargo-lambda` and `sam`:

```bash
sam build
sam package --output-template-file .aws-sam/packaged.yml --s3-bucket ${YOUR_OWN_BUCKET_NAME}
```

Make sure to replace `${YOUR_OWN_BUCKET_NAME}` with the right value.

### 3. Publish the application to SAR

```bash
sam publish --template .aws-sam/packaged.yml --region ${YOUR_OWN_REGION}
```

Make sure to replace `${YOUR_OWN_REGION}` with the right value.