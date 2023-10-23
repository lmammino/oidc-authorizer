use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct TokenAuthorizerEvent {
    #[serde(rename = "authorizationToken")]
    pub authorization_token: String,
    #[serde(rename = "methodArn")]
    pub method_arn: String,
}

#[derive(Serialize)]
pub struct PolicyStatement {
    #[serde(rename = "Action")]
    pub action: String,
    #[serde(rename = "Effect")]
    pub effect: String,
    #[serde(rename = "Resource")]
    pub resource: String,
}

#[derive(Serialize)]
pub struct PolicyDocument {
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Statement")]
    pub statement: Vec<PolicyStatement>,
}

#[derive(Serialize)]
pub struct TokenAuthorizerResponse {
    #[serde(rename = "principalId")]
    pub principal_id: String,
    #[serde(rename = "policyDocument")]
    pub policy_document: PolicyDocument,
    pub context: HashMap<String, String>,
    #[serde(rename = "usageIdentifierKey")]
    pub usage_identifier_key: String,
}

impl TokenAuthorizerResponse {
    pub fn allow(resource: &str) -> Self {
        Self {
            context: HashMap::new(),              // TODO: verify if needed
            usage_identifier_key: "".to_string(), // TODO: verify if needed
            principal_id: "api_request".to_string(),
            policy_document: PolicyDocument {
                version: "2012-10-17".to_string(),
                statement: vec![PolicyStatement {
                    effect: "Allow".to_string(),
                    action: "execute-api:Invoke".to_string(),
                    resource: resource.to_string(),
                }],
            },
        }
    }

    pub fn deny() -> Self {
        // TODO: see if can use context for specifying the reject reason
        Self {
            context: HashMap::new(),              // TODO: verify if needed
            usage_identifier_key: "".to_string(), // TODO: verify if needed
            principal_id: "api_request".to_string(),
            policy_document: PolicyDocument {
                version: "2012-10-17".to_string(),
                statement: vec![PolicyStatement {
                    effect: "Deny".to_string(),
                    action: "execute-api:Invoke".to_string(),
                    resource: "*".to_string(),
                }],
            },
        }
    }
}
