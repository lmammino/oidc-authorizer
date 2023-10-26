use std::{collections::HashMap, fmt::Debug, ops::Deref};

use jsonwebtoken::{jwk::JwkSet, DecodingKey};

#[derive(Default)]
pub struct KeysMap(HashMap<String, DecodingKey>);

impl Deref for KeysMap {
    type Target = HashMap<String, DecodingKey>;

    fn deref(&self) -> &Self::Target {
        &(self.0)
    }
}

impl From<JwkSet> for KeysMap {
    fn from(jwks: JwkSet) -> Self {
        let mut map = HashMap::with_capacity(jwks.keys.len());
        for key in jwks.keys {
            if let Some(key_id) = &key.common.key_id {
                match DecodingKey::from_jwk(&key) {
                    Ok(k) => {
                        map.insert(key_id.clone(), k);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create a decoding key from JWK: {}. This key won't be indexed and it will be ignored", e);
                        continue;
                    }
                }
            }
        }

        Self(map)
    }
}

impl Debug for KeysMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeysMap")
            .field("keys", &self.0.keys())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn it_should_create_a_map_from_a_jwkset() {
        let jwkset: JwkSet = serde_json::from_value(json!({
            "keys": [
                {
                    "kty": "RSA",
                    "n": "0TF4RX87dOllFp12D8IZvSoJyp8D4IZ3JmlVG7Au2GOSp1WcrAqjyq3Gk-a_1tT31FHCLVqjH9vXE8g1sXika4mp8YCWyMfjT3KsfrciI_Fw-nBCawnqewBDcBo4cvBgTjHNBjcjGNr0U_4eCZPjP8pwqw6HrRgHf-ypNmtgWG6_2EaK-tOJtnNgGRtCYGZdqMDfKLDuqzU5-gT2ejt9P1kNAvFMMUm4dTOK-vJ7jwGKWZEzupHBlHMqu4K4IRoFbVr2XsAzV5YQ0u_r26NVtQTDUdTp9ixhexUp0eXye6m3uMklqUOHJbiqNjmH2ye4yXVJI0w6BFOeXXlwyR6slw",
                    "e": "AQAB",
                    "alg": "RS256",
                    "kid": "test/keys/rs256/public",
                    "use": "sig"
                },
                {
                    "kty": "RSA",
                    "e": "AQAB",
                    "use": "sig",
                    "kid": "test/keys/rs512/public",
                    "alg": "RS512",
                    "n": "jwrjFyp2WiJnr4m_M7kEJkLEFWhcKR2FTxb3frE27Fig6hqiY6_8nUMtTD4DCBu9bNzlWLcLGs1-XXV-sCQzXpK_N5tR-kd5iWuH9nzxhwewVy7q9ZxC0ejk1LKMfcWr3EalvcS0Iv-v7oZ9of23YFzBwELxeD3bjHZ1q22kpt3J_XbuYM29ZGYX_2BIl1NVJ0bhZPJDLPVVbvoDwLwL6W3AxHJUYQGNFR_mOBjpuISXxtkguErDUeTXbTdMOLCR_hLpRVwY96132-1Cd1amLLVo4nv6pV2-83GQe-qiXrtXGJ_VDwvJxW4F_p5KDKtSZ7lwXSQOHwobDyxQik1n6w"
                }
            ]
        })).unwrap();
        let keysmap: KeysMap = jwkset.into();
        assert_eq!(keysmap.len(), 2);
        assert!(keysmap.contains_key(&"test/keys/rs256/public".to_string()));
        assert!(keysmap.contains_key(&"test/keys/rs512/public".to_string()));
    }
}
