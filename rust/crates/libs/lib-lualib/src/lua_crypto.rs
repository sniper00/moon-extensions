use lib_lua::{cstr, luaL_newlib};
use anyhow::{anyhow, Result};
use lib_lua::{
    ffi::{self, luaL_Reg},
    laux::{self, LuaValue},
    lreg, lreg_null,
};
use ring::aead::{
    Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_128_GCM, AES_256_GCM, NONCE_LEN
};
use ring::error::Unspecified;
use ring::rand::{SecureRandom, SystemRandom};
use std::ffi::c_int;

const AES_128_KEY_LEN: usize = 16; // 128 bits
const AES_256_KEY_LEN: usize = 32; // 256 bits

struct OneNonceSequence(Option<Nonce>);

impl OneNonceSequence {
    fn new(nonce: Nonce) -> Self {
        Self(Some(nonce))
    }
}

impl NonceSequence for OneNonceSequence {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        self.0.take().ok_or(Unspecified)
    }
}

/// AES-GCM encryption
/// Parameters: data (string), key (string), nonce (string, optional)
/// Returns: encrypted_data (string), nonce (string)
fn aes_encrypt_imp(state: *mut ffi::lua_State) -> Result<c_int> {
    let data: &[u8] = laux::lua_get(state, 1);
    let key: &[u8] = laux::lua_get(state, 2);

    if key.len() != AES_256_KEY_LEN && key.len() != AES_128_KEY_LEN {
        return Err(anyhow!("Key must be 32 bytes for AES-256 or 16 bytes for AES-128"));
    }

    let mut nonce_bytes = [0u8; NONCE_LEN];
    let rng = SystemRandom::new();
    let nonce = if let LuaValue::String(nonce_str) = LuaValue::from_stack(state, 3) {
        if nonce_str.len() != NONCE_LEN {
            return Err(anyhow!("Nonce must be 12 bytes"));
        }
        nonce_bytes.copy_from_slice(nonce_str);
        Nonce::try_assume_unique_for_key(&nonce_bytes).unwrap()
    } else {
        rng.fill(&mut nonce_bytes)
            .map_err(|_| anyhow!("Failed to generate random nonce"))?;
        Nonce::try_assume_unique_for_key(&nonce_bytes).unwrap()
    };

    let nonce_sequence = OneNonceSequence::new(nonce);
    let mut sealing_key = if key.len() == AES_128_KEY_LEN {
        SealingKey::new(UnboundKey::new(&AES_128_GCM, key).unwrap(), nonce_sequence)
    } else {
        SealingKey::new(UnboundKey::new(&AES_256_GCM, key).unwrap(), nonce_sequence)
    };

    // Encrypt data
    let mut in_out = data.to_vec();
    let tag = sealing_key
        .seal_in_place_separate_tag(Aad::empty(), &mut in_out)
        .map_err(|err| anyhow!(format!("Encryption failed: {0}", err.to_string())))?;

    // Combine ciphertext and tag
    in_out.extend_from_slice(tag.as_ref());

    // Return encrypted data and nonce
    laux::lua_push(state, in_out.as_slice());
    laux::lua_push(state, nonce_bytes.as_slice());

    Ok(2)
}

unsafe extern "C-unwind" fn aes_encrypt(state: *mut ffi::lua_State) -> c_int {
    match aes_encrypt_imp(state) {
        Ok(n) => n,
        Err(e) => {
            laux::lua_error(state, e.to_string().as_str());
        }
    }
}

/// AES-GCM decryption
/// Parameters: encrypted_data (string), key (string), nonce (string)
/// Returns: decrypted_data (string)
fn aes_decrypt_imp(state: *mut ffi::lua_State) -> Result<c_int> {
    let encrypted_data: &[u8] = laux::lua_get(state, 1);
    let key: &[u8] = laux::lua_get(state, 2);
    let nonce_bytes: &[u8] = laux::lua_get(state, 3);

    // Check parameter lengths
    if key.len() != AES_256_KEY_LEN && key.len() != AES_128_KEY_LEN {
        return Err(anyhow!("Key must be 32 bytes for AES-256 or 16 bytes for AES-128"));
    }

    if nonce_bytes.len() != NONCE_LEN {
        return Err(anyhow!("Nonce must be 12 bytes"));
    }

    if encrypted_data.len() < 16 {
        return Err(anyhow!("Encrypted data too short"));
    }

    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes).unwrap();

    // Create decryption key
    let nonce_sequence = OneNonceSequence::new(nonce);
    let mut opening_key = if key.len() == AES_128_KEY_LEN {
        OpeningKey::new(UnboundKey::new(&AES_128_GCM, key).unwrap(), nonce_sequence)
    } else {
        OpeningKey::new(UnboundKey::new(&AES_256_GCM, key).unwrap(), nonce_sequence)
    };

    // Decrypt data
    let mut in_out = encrypted_data.to_vec();
    let plaintext = opening_key
        .open_in_place(Aad::empty(), &mut in_out)
        .map_err(|err| anyhow!(format!("Decryption failed: {}", err.to_string())))?;

    laux::lua_push(state, plaintext as &[u8]);
    Ok(1)
}

unsafe extern "C-unwind" fn aes_decrypt(state: *mut ffi::lua_State) -> c_int {
    match aes_decrypt_imp(state) {
        Ok(n) => n,
        Err(e) => {
            laux::lua_error(state, e.to_string().as_str());
        }
    }
}

/// Generate random key
/// Parameters: key_size (number, optional, default: 32)
/// Returns: key (string)
unsafe extern "C-unwind" fn generate_key(state: *mut ffi::lua_State) -> c_int {
    let key_size = laux::lua_opt(state, 1).unwrap_or(32);

    if key_size == 0 || key_size > 64 {
        laux::lua_push(state, "Key size must be between 1 and 64 bytes");
        ffi::lua_error(state);
    }

    let rng = SystemRandom::new();
    let mut key = vec![0u8; key_size];
    match rng.fill(&mut key) {
        Ok(_) => {
            laux::lua_push(state, key.as_slice());
            1
        }
        Err(_) => {
            laux::lua_error(state, "Failed to generate random key");
        }
    }
}

/// Generate random nonce
/// Returns: nonce (string)
unsafe extern "C-unwind" fn generate_nonce(state: *mut ffi::lua_State) -> c_int {
    let rng = SystemRandom::new();
    let mut nonce = [0u8; 12];

    match rng.fill(&mut nonce) {
        Ok(_) => {
            laux::lua_push(state, nonce.as_slice());
            1
        }
        Err(_) => {
            laux::lua_error(state, "Failed to generate random key");
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C-unwind" fn luaopen_rust_crypto(state: *mut ffi::lua_State) -> c_int {
    let l = [
        lreg!("aes_encrypt", aes_encrypt),
        lreg!("aes_decrypt", aes_decrypt),
        lreg!("generate_key", generate_key),
        lreg!("generate_nonce", generate_nonce),
        lreg_null!(),
    ];

    luaL_newlib!(state, l);
    1
}
