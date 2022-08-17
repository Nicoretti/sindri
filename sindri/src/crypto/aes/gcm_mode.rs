use super::*;

use crate::common::limits::{MAX_CIPHERTEXT_SIZE, MAX_PLAINTEXT_SIZE};
use aes_gcm::{AeadInPlace, Aes128Gcm, Aes256Gcm, KeyInit};
use heapless::Vec;

/// AES-GCM encryption: generic over an underlying AES implementation.
fn aes_gcm_encrypt<C>(
    key: &[u8],
    nonce: &[u8],
    associated_data: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8, { MAX_CIPHERTEXT_SIZE + GCM_TAG_SIZE }>, Error>
where
    C: KeyInit + AeadInPlace,
{
    check_sizes(key, nonce, C::KeySize::USIZE, C::NonceSize::USIZE)?;

    let mut ciphertext_and_tag = Vec::new();
    ciphertext_and_tag
        .extend_from_slice(plaintext)
        .map_err(|_| Error::Alloc)?;

    let tag = C::new(key.into())
        .encrypt_in_place_detached(nonce.into(), associated_data, &mut ciphertext_and_tag)
        .map_err(|_| Error::Encryption)?;
    ciphertext_and_tag
        .extend_from_slice(&tag)
        .map_err(|_| Error::Alloc)?;

    Ok(ciphertext_and_tag)
}

/// AES-GCM decryption: generic over an underlying AES implementation.
fn aes_gcm_decrypt<C>(
    key: &[u8],
    nonce: &[u8],
    associated_data: &[u8],
    ciphertext_and_tag: &[u8],
) -> Result<Vec<u8, MAX_PLAINTEXT_SIZE>, Error>
where
    C: KeyInit + AeadInPlace,
{
    check_sizes(key, nonce, C::KeySize::USIZE, C::NonceSize::USIZE)?;
    if ciphertext_and_tag.len() < C::TagSize::USIZE {
        return Err(Error::InvalidBufferSize);
    }

    let (ciphertext, tag) =
        ciphertext_and_tag.split_at(ciphertext_and_tag.len() - C::TagSize::USIZE);
    let mut plaintext = Vec::new();
    plaintext
        .extend_from_slice(ciphertext)
        .map_err(|_| Error::Alloc)?;
    C::new(key.into())
        .decrypt_in_place_detached(nonce.into(), associated_data, &mut plaintext, tag.into())
        .map_err(|_| Error::Decryption)?;
    Ok(plaintext)
}

macro_rules! define_aes_gcm_impl {
    (
        $encryptor:ident,
        $decryptor:ident,
        $core:tt
    ) => {
        pub fn $encryptor(
            key: &[u8],
            nonce: &[u8],
            aad: &[u8],
            plaintext: &[u8],
        ) -> Result<Vec<u8, { MAX_PLAINTEXT_SIZE + GCM_TAG_SIZE }>, Error> {
            aes_gcm_encrypt::<$core>(key, nonce, aad, plaintext)
        }

        pub fn $decryptor(
            key: &[u8],
            nonce: &[u8],
            aad: &[u8],
            ciphertext: &[u8],
        ) -> Result<Vec<u8, MAX_PLAINTEXT_SIZE>, Error> {
            aes_gcm_decrypt::<$core>(key, nonce, aad, ciphertext)
        }
    };
}

define_aes_gcm_impl!(aes128gcm_encrypt, aes128gcm_decrypt, Aes128Gcm);
define_aes_gcm_impl!(aes256gcm_encrypt, aes256gcm_decrypt, Aes256Gcm);

#[cfg(test)]
pub mod test {
    use super::*;

    const KEY128: &[u8; KEY128_SIZE] = b"Open sesame! ...";
    const KEY256: &[u8; KEY256_SIZE] = b"Or was it 'open quinoa' instead?";
    const NONCE: &[u8; GCM_NONCE_SIZE] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    const PLAINTEXT: &[u8] = b"Hello, World!";
    const AAD: &[u8] = b"Never gonna give you up, Never gonna let you down!";

    macro_rules! define_aes_gcm_encrypt_decrypt_test {
        (
        $test_name:ident,
        $cipher:ty,
        $key:tt,
        $nonce:tt,
        $associated_data:expr,
        $plaintext:tt,
        $ciphertext:tt
    ) => {
            #[test]
            fn $test_name() {
                let encrypted =
                    aes_gcm_encrypt::<$cipher>($key, $nonce, $associated_data, $plaintext)
                        .expect("encryption error");
                let decrypted =
                    aes_gcm_decrypt::<$cipher>($key, $nonce, $associated_data, &encrypted)
                        .expect("decryption error");
                assert_eq!(encrypted, $ciphertext, "ciphertext mismatch");
                assert_eq!(decrypted, $plaintext, "plaintext mismatch");
            }
        };
    }

    define_aes_gcm_encrypt_decrypt_test!(
        test_aes128gcm_no_aad_encrypt_decrypt,
        Aes128Gcm,
        KEY128,
        NONCE,
        &[],
        PLAINTEXT,
        [
            // ciphertext
            0xbb, 0xfe, 0x8, 0x2b, 0x97, 0x86, 0xd4, 0xe4, 0xa4, 0xec, 0x19, 0xdb, 0x63,
            // tag
            0x40, 0xce, 0x93, 0x5a, 0x71, 0x5e, 0x63, 0x9, 0xb, 0x11, 0xad, 0x51, 0x4d, 0xe8, 0x23,
            0x50,
        ]
    );

    define_aes_gcm_encrypt_decrypt_test!(
        test_aes256gcm_no_aad_encrypt_decrypt,
        Aes256Gcm,
        KEY256,
        NONCE,
        &[],
        PLAINTEXT,
        [
            // ciphertext
            0xab, 0xe2, 0x9e, 0x5a, 0x8d, 0xd3, 0xbd, 0x62, 0xc9, 0x46, 0x71, 0x8e, 0x50,
            // tag
            0xa8, 0xcb, 0x47, 0x81, 0xad, 0x51, 0x89, 0x1f, 0x23, 0x78, 0x11, 0xcb, 0x9f, 0xc5,
            0xbf, 0x8b,
        ]
    );

    define_aes_gcm_encrypt_decrypt_test!(
        test_aes128gcm_with_aad_encrypt_decrypt,
        Aes128Gcm,
        KEY128,
        NONCE,
        AAD,
        PLAINTEXT,
        [
            // ciphertext
            0xbb, 0xfe, 0x08, 0x2b, 0x97, 0x86, 0xd4, 0xe4, 0xa4, 0xec, 0x19, 0xdb, 0x63,
            // tag
            0x15, 0x6d, 0x9e, 0xd9, 0x50, 0x1d, 0x7a, 0x51, 0x77, 0x44, 0x98, 0x97, 0x7d, 0x54,
            0x1c, 0x19,
        ]
    );

    define_aes_gcm_encrypt_decrypt_test!(
        test_aes256gcm_with_aad_encrypt_decrypt,
        Aes256Gcm,
        KEY256,
        NONCE,
        AAD,
        PLAINTEXT,
        [
            // ciphertext
            0xab, 0xe2, 0x9e, 0x5a, 0x8d, 0xd3, 0xbd, 0x62, 0xc9, 0x46, 0x71, 0x8e, 0x50,
            // tag
            0xc2, 0xb4, 0x2e, 0x65, 0x8f, 0xa9, 0xfc, 0xc4, 0x2d, 0xaf, 0x8e, 0x22, 0xd3, 0xc5,
            0x8b, 0x6c,
        ]
    );

    macro_rules! define_aes_gcm_errors_test {
        (
        $test_name:ident,
        $cipher:ty,
        $key:tt,
        $nonce:tt,
        $plaintext:tt,
        $wrong_key_sizes:tt
    ) => {
            #[test]
            fn $test_name() {
                for size in $wrong_key_sizes {
                    let mut wrong_key: Vec<u8, 256> = Vec::new();
                    wrong_key.resize(size, 0).expect("Allocation error");
                    assert_eq!(
                        aes_gcm_encrypt::<$cipher>(&wrong_key, $nonce, &[], $plaintext),
                        Err(Error::InvalidKeySize)
                    );
                    let mut zeros: Vec<u8, { MAX_PLAINTEXT_SIZE + GCM_TAG_SIZE }> = Vec::new();
                    zeros
                        .resize($plaintext.len() + GCM_TAG_SIZE, 0)
                        .expect("Allocation error");
                    assert_eq!(
                        aes_gcm_decrypt::<$cipher>(&wrong_key, $nonce, &[], &zeros),
                        Err(Error::InvalidKeySize)
                    );
                }

                for size in [0, 1, 10, 16, 32] {
                    let mut wrong_nonce: Vec<u8, 32> = Vec::new();
                    wrong_nonce.resize(size, 0).expect("Allocation error");
                    assert_eq!(
                        aes_gcm_encrypt::<$cipher>($key, &wrong_nonce, &[], $plaintext),
                        Err(Error::InvalidIvSize)
                    );
                    let mut zeros: Vec<u8, { MAX_PLAINTEXT_SIZE + GCM_TAG_SIZE }> = Vec::new();
                    zeros
                        .resize($plaintext.len() + GCM_TAG_SIZE, 0)
                        .expect("Allocation error");
                    assert_eq!(
                        aes_gcm_decrypt::<$cipher>($key, &wrong_nonce, &[], &zeros),
                        Err(Error::InvalidIvSize)
                    );
                }

                for size in [0, 1, GCM_TAG_SIZE - 1] {
                    const MAX_SIZE: usize = GCM_TAG_SIZE - 1;
                    let mut wrong_ciphertext: Vec<u8, MAX_SIZE> = Vec::new();
                    wrong_ciphertext.resize(size, 0).expect("Allocation error");
                    assert_eq!(
                        aes_gcm_decrypt::<$cipher>($key, $nonce, &[], &wrong_ciphertext),
                        Err(Error::InvalidBufferSize)
                    );
                }

                let mut corrupted_ciphertext =
                    aes_gcm_encrypt::<$cipher>($key, $nonce, &[], $plaintext)
                        .expect("encryption error");
                corrupted_ciphertext[0] += 1;
                assert_eq!(
                    aes_gcm_decrypt::<$cipher>($key, $nonce, &[], &corrupted_ciphertext),
                    Err(Error::Decryption)
                );
            }
        };
    }

    define_aes_gcm_errors_test!(
        test_aes128gcm_errors,
        Aes128Gcm,
        KEY128,
        NONCE,
        PLAINTEXT,
        [0, 1, 8, 24, 32, 128]
    );

    define_aes_gcm_errors_test!(
        test_aes256gcm_errors,
        Aes256Gcm,
        KEY256,
        NONCE,
        PLAINTEXT,
        [0, 1, 8, 16, 24, 256]
    );
}