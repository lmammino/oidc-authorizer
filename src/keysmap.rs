use std::{collections::HashMap, ops::Deref};

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
