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
                map.insert(
                    key_id.clone(),
                    // TODO: handle possile errors in creating the decoding key from jwk
                    DecodingKey::from_jwk(&key).expect("Failed to create a decoding key from JWK"),
                );
            }
        }

        Self(map)
    }
}
