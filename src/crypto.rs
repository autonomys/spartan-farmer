use crate::PRIME_SIZE_BYTES;
use ring::{digest, hmac};
use schnorrkel::PublicKey;

pub(crate) fn hash_public_key(public_key: &PublicKey) -> [u8; PRIME_SIZE_BYTES] {
    let mut array = [0u8; PRIME_SIZE_BYTES];
    let hash = digest::digest(&digest::SHA256, public_key.as_ref());
    array.copy_from_slice(&hash.as_ref()[..PRIME_SIZE_BYTES]);
    array
}

/// Returns a hash bashed message authentication code unique to a message and challenge.
pub(crate) fn create_hmac(message: &[u8], challenge: &[u8]) -> [u8; 32] {
    let key = hmac::Key::new(hmac::HMAC_SHA256, challenge);
    let mut array = [0u8; 32];
    let hmac = hmac::sign(&key, message).as_ref().to_vec();
    array.copy_from_slice(&hmac[0..32]);
    array
}
