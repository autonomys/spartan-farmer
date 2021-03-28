use crate::{Piece, PIECE_SIZE, PRIME_SIZE_BYTES};
use ring::{digest, hmac};
use schnorrkel::PublicKey;
use std::io::Write;

pub fn genesis_piece_from_seed(seed: &str) -> Piece {
    // TODO: This is not efficient
    let mut piece = [0u8; PIECE_SIZE];
    let mut input = seed.as_bytes().to_vec();
    for mut chunk in piece.chunks_mut(digest::SHA256.output_len) {
        input = digest::digest(&digest::SHA256, &input).as_ref().to_vec();
        chunk.write_all(input.as_ref()).unwrap();
    }
    piece
}

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
