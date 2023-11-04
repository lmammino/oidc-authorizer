This folder contains the SAM template that is used to bootstrap the necessary infrastructure and integration between GitHub and AWS.

This is intended to be a one off operation to he deployed manually. Once deployed, the GitHub repository will be able to perform certain operations against the given AWS account (e.g. publish files in a bucket or publish to the Serverless Application Repository).

Deploy with (from this folder):

```bash
sam deploy
```