use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
pub struct TokenAuthorizerEvent {
    #[serde(rename = "authorizationToken")]
    pub authorization_token: String,
    #[serde(rename = "methodArn")]
    pub method_arn: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyStatement {
    #[serde(rename = "Action")]
    pub action: String,
    #[serde(rename = "Effect")]
    pub effect: String,
    #[serde(rename = "Resource")]
    pub resource: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyDocument {
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Statement")]
    pub statement: Vec<PolicyStatement>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenAuthorizerResponse {
    #[serde(rename = "principalId")]
    pub principal_id: String,
    #[serde(rename = "policyDocument")]
    pub policy_document: PolicyDocument,
    pub context: HashMap<String, String>,
}

impl TokenAuthorizerResponse {
    pub fn allow(principal_id: &str, resource: &str, token_claims: &Value) -> Self {
        let mut context = HashMap::new();
        context.insert("jwt_principal".to_string(), principal_id.to_string());
        context.insert(
            "jwt_claims".to_string(),
            serde_json::to_string(token_claims).unwrap(),
        );

        Self {
            context,
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

    pub fn deny(resource: &str) -> Self {
        Self {
            context: HashMap::new(),
            principal_id: "none".to_string(),
            policy_document: PolicyDocument {
                version: "2012-10-17".to_string(),
                statement: vec![PolicyStatement {
                    effect: "Deny".to_string(),
                    action: "execute-api:Invoke".to_string(),
                    resource: resource.to_string(),
                }],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_should_create_an_allow_response() {
        let principal_id = "John Doe";
        let resource = "arn::some:resource";
        let token_claims = json!({
          "iat": 1516239022,
          "name": "John Doe",
          "sub": "1234567890"
        });
        let response = TokenAuthorizerResponse::allow(principal_id, resource, &token_claims);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({
                "principalId": "John Doe",
                "policyDocument": {
                    "Version": "2012-10-17",
                    "Statement": [
                        {
                            "Action": "execute-api:Invoke",
                            "Effect": "Allow",
                            "Resource": "arn::some:resource"
                        }
                    ]
                },
                "context": {
                    "jwt_claims": "{\"iat\":1516239022,\"name\":\"John Doe\",\"sub\":\"1234567890\"}",
                    "jwt_principal": "John Doe",
                }
            })
        );
    }

    #[test]
    fn it_create_a_deny_response() {
        let resource = "arn::some:resource";
        let response = TokenAuthorizerResponse::deny(resource);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({
                "context": {},
                "policyDocument": {
                    "Statement": [
                        {
                            "Action": "execute-api:Invoke",
                            "Effect": "Deny",
                            "Resource": "arn::some:resource"
                        }
                    ],
                    "Version": "2012-10-17"
                },
                "principalId": "none"
            })
        );
    }
}
