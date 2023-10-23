use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub fn allow(
        raw_token: &str,
        principal_id: &str,
        resource: &str,
        token_claims: &Value,
    ) -> Self {
        let mut context = HashMap::new();
        if let Some(claims) = token_claims.as_object() {
            for (key, value) in claims.iter() {
                context.insert(format!("jwt_claim_{}", key), value.to_string());
            }
        }

        Self {
            context,
            usage_identifier_key: raw_token.to_string(),
            principal_id: principal_id.to_string(),
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

    pub fn deny(token: &str) -> Self {
        Self {
            context: HashMap::new(),
            usage_identifier_key: token.to_string(),
            principal_id: "none".to_string(),
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
