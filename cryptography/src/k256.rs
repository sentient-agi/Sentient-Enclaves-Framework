fn main() {
    ecdh_k256_hkdf::print_shared_keys();
}

mod ecdh_k256_hkdf {
    use k256::{EncodedPoint, PublicKey, ecdh::EphemeralSecret};
    use rand_core::{RngCore, OsRng}; // requires 'getrandom' feature
    use sha3::{Sha3_512, Sha3_256};
    use hkdf::Hkdf;

    pub fn print_shared_keys() {
        let mut random_seed = [0u8; 64];
        OsRng.fill_bytes(&mut random_seed);
        println!("Random seed: {:?}\n", random_seed);

        let rng = OsRng;

        let alice_secret = EphemeralSecret::random(&mut rng.clone());
        let bob_secret = EphemeralSecret::random(&mut rng.clone());

        let alice_pk_bytes = EncodedPoint::from(alice_secret.public_key());
        let bob_pk_bytes = EncodedPoint::from(bob_secret.public_key());

        let alice_public = PublicKey::from_sec1_bytes(alice_pk_bytes.as_ref())
            .expect("alice's public key is invalid!"); // In real usage, don't panic, handle this!
        let bob_public = PublicKey::from_sec1_bytes(bob_pk_bytes.as_ref())
            .expect("bob's public key is invalid!"); // In real usage, don't panic, handle this!

        let alice_shared_with_bob = alice_secret.diffie_hellman(&bob_public);
        let bob_shared_with_alice = bob_secret.diffie_hellman(&alice_public);

        let cheshire_cat_secret = EphemeralSecret::random(&mut rng.clone());
        let cheshire_cat_pk_bytes = EncodedPoint::from(cheshire_cat_secret.public_key());
        let cheshire_cat_public = PublicKey::from_sec1_bytes(cheshire_cat_pk_bytes.as_ref())
            .expect("Cheshire Cat's public key is invalid!"); // In real usage, don't panic, handle this!

        let cheshire_cat_shared_with_alice = cheshire_cat_secret.diffie_hellman(&alice_public);
        let alice_shared_with_cheshire_cat = alice_secret.diffie_hellman(&cheshire_cat_public);

        let cheshire_cat_shared_with_bob = cheshire_cat_secret.diffie_hellman(&bob_public);
        let bob_shared_with_cheshire_cat = bob_secret.diffie_hellman(&cheshire_cat_public);

        println!("Alice shared with Bob: {:?}\n", alice_shared_with_bob.raw_secret_bytes().as_slice());
        println!("Bob shared with Alice: {:?}\n", bob_shared_with_alice.raw_secret_bytes().as_slice());
        println!("Alice shared with Cheshire Cat: {:?}\n", alice_shared_with_cheshire_cat.raw_secret_bytes().as_slice());
        println!("Cheshire Cat shared with Alice: {:?}\n", cheshire_cat_shared_with_alice.raw_secret_bytes().as_slice());
        println!("Bob shared with Cheshire Cat: {:?}\n", bob_shared_with_cheshire_cat.raw_secret_bytes().as_slice());
        println!("Cheshire Cat shared with Bob: {:?}\n", cheshire_cat_shared_with_bob.raw_secret_bytes().as_slice());

        // HMAC-based Extract-and-Expand Key Derivation Function (HKDF) for authenticated/hashed keys

        let alice_with_bob_shared_secret_authd_hashed = alice_shared_with_bob.extract::<Sha3_512>(Some(&random_seed[..]));
        let mut alice_with_bob_shared_secret_bytes = [0u8; 64];
        let _ = alice_with_bob_shared_secret_authd_hashed.expand(&[0u8; 0], &mut alice_with_bob_shared_secret_bytes)
            .expect("64 bytes + info expand bytes is a valid length for Sha3_512 hash with expanding operation to output");
        println!("Alice shared secret with Bob (Authd, Hashed, Expanded): {:?}\n", alice_with_bob_shared_secret_bytes);
    }
}

#[cfg(test)]
mod tests_k256 {
    use k256::{EncodedPoint, PublicKey, ecdh::EphemeralSecret};
    use rand_core::{RngCore, OsRng}; // requires 'getrandom' feature

    #[test]
    pub fn test_cipher() {

        let mut random_seed = [0u8; 64];
        OsRng.fill_bytes(&mut random_seed);
        println!("Random seed: {:?}", random_seed);

        let rng = OsRng;

        let alice_secret = EphemeralSecret::random(&mut rng.clone());
        let bob_secret = EphemeralSecret::random(&mut rng.clone());

        let alice_pk_bytes = EncodedPoint::from(alice_secret.public_key());
        let bob_pk_bytes = EncodedPoint::from(bob_secret.public_key());

        let alice_public = PublicKey::from_sec1_bytes(alice_pk_bytes.as_ref())
            .expect("alice's public key is invalid!"); // In real usage, don't panic, handle this!
        let bob_public = PublicKey::from_sec1_bytes(bob_pk_bytes.as_ref())
            .expect("bob's public key is invalid!"); // In real usage, don't panic, handle this!

        let alice_shared_with_bob = alice_secret.diffie_hellman(&bob_public);
        let bob_shared_with_alice = bob_secret.diffie_hellman(&alice_public);

        let cheshire_cat_secret = EphemeralSecret::random(&mut rng.clone());
        let cheshire_cat_pk_bytes = EncodedPoint::from(cheshire_cat_secret.public_key());
        let cheshire_cat_public = PublicKey::from_sec1_bytes(cheshire_cat_pk_bytes.as_ref())
            .expect("Cheshire Cat's public key is invalid!"); // In real usage, don't panic, handle this!

        let cheshire_cat_shared_with_alice = cheshire_cat_secret.diffie_hellman(&alice_public);
        let alice_shared_with_cheshire_cat = alice_secret.diffie_hellman(&cheshire_cat_public);

        let cheshire_cat_shared_with_bob = cheshire_cat_secret.diffie_hellman(&bob_public);
        let bob_shared_with_cheshire_cat = bob_secret.diffie_hellman(&cheshire_cat_public);

        assert_eq!(alice_shared_with_bob.raw_secret_bytes(), bob_shared_with_alice.raw_secret_bytes());
        assert_eq!(alice_shared_with_cheshire_cat.raw_secret_bytes(), cheshire_cat_shared_with_alice.raw_secret_bytes());
        assert_eq!(bob_shared_with_cheshire_cat.raw_secret_bytes(), cheshire_cat_shared_with_bob.raw_secret_bytes());


        assert_ne!(alice_shared_with_bob.raw_secret_bytes(), alice_shared_with_cheshire_cat.raw_secret_bytes());
        assert_ne!(alice_shared_with_bob.raw_secret_bytes(), cheshire_cat_shared_with_alice.raw_secret_bytes());
        assert_ne!(alice_shared_with_bob.raw_secret_bytes(), bob_shared_with_cheshire_cat.raw_secret_bytes());
        assert_ne!(alice_shared_with_bob.raw_secret_bytes(), cheshire_cat_shared_with_bob.raw_secret_bytes());

        assert_ne!(bob_shared_with_alice.raw_secret_bytes(), alice_shared_with_cheshire_cat.raw_secret_bytes());
        assert_ne!(bob_shared_with_alice.raw_secret_bytes(), cheshire_cat_shared_with_alice.raw_secret_bytes());
        assert_ne!(bob_shared_with_alice.raw_secret_bytes(), bob_shared_with_cheshire_cat.raw_secret_bytes());
        assert_ne!(bob_shared_with_alice.raw_secret_bytes(), cheshire_cat_shared_with_bob.raw_secret_bytes());

        assert_ne!(alice_shared_with_cheshire_cat.raw_secret_bytes(), bob_shared_with_cheshire_cat.raw_secret_bytes());
        assert_ne!(alice_shared_with_cheshire_cat.raw_secret_bytes(), cheshire_cat_shared_with_bob.raw_secret_bytes());

        assert_ne!(cheshire_cat_shared_with_alice.raw_secret_bytes(), bob_shared_with_cheshire_cat.raw_secret_bytes());
        assert_ne!(cheshire_cat_shared_with_alice.raw_secret_bytes(), cheshire_cat_shared_with_bob.raw_secret_bytes());
    }
}
