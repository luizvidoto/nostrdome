use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use base64::engine::general_purpose;
use base64::Engine;
use nostr_sdk::secp256k1::SecretKey;
use rand::Rng;

use crate::error::Error; // aes-gcm crate

pub fn random_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    let mut rng = rand::thread_rng();
    rng.fill(&mut nonce);
    nonce
}

pub fn encrypt_local_message(secret_key: &SecretKey, message: &str) -> Result<String, Error> {
    let key = secret_key.secret_bytes();
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let nonce = random_nonce();

    let encrypted_message = cipher
        .encrypt(GenericArray::from_slice(&nonce), message.as_bytes())
        .map_err(|_| Error::EncryptionError("".into()))?;

    Ok(format!(
        "{}?iv={}",
        general_purpose::STANDARD.encode(encrypted_message),
        general_purpose::STANDARD.encode(nonce)
    ))
}

pub fn decrypt_local_message(
    secret_key: &SecretKey,
    encrypted_message: &str,
) -> Result<String, Error> {
    let key = secret_key.secret_bytes();
    let parts: Vec<&str> = encrypted_message.split("?nonce=").collect();
    if parts.len() != 2 {
        return Err(Error::DecryptionError(
            "Invalid local message format".into(),
        ));
    }

    let encrypted_message_base64 = parts[0];
    let nonce_base64 = parts[1];
    let encrypted_message = general_purpose::STANDARD.decode(encrypted_message_base64)?;
    let nonce = general_purpose::STANDARD.decode(nonce_base64)?;

    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let decrypted_message_bytes = cipher
        .decrypt(
            GenericArray::from_slice(&nonce),
            encrypted_message.as_slice(),
        )
        .map_err(|_| Error::DecryptionError("".into()))?;
    let decrypted_message =
        String::from_utf8(decrypted_message_bytes).map_err(|e| Error::Utf8Error(e.utf8_error()))?;

    Ok(decrypted_message)
}
