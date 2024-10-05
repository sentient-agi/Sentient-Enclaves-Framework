fn main() {
    ecdh_p224_hkdf::print_shared_keys();
}

mod ecdh_p224_hkdf {
    use p224::{EncodedPoint, PublicKey, ecdh::EphemeralSecret};
    use rand_core::{RngCore, OsRng}; // requires 'getrandom' feature
    use sha3::{Sha3_512, Sha3_224};
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

        // Correct Example Variant #1
        let alice_with_bob_shared_secret_authd_hashed = alice_shared_with_bob.extract::<Sha3_512>(Some(&random_seed[..]));
        let mut alice_with_bob_shared_secret_bytes = [0u8; 64];
        let _ = alice_with_bob_shared_secret_authd_hashed.expand(&[0u8; 0], &mut alice_with_bob_shared_secret_bytes)
            .expect("64 bytes + info expand bytes is a valid length for Sha3_512 hash with expanding operation to output");
        println!("Alice shared secret with Bob (Authd, Hashed, Expanded): {:?}\n", alice_with_bob_shared_secret_bytes);


        // Correct Example Variant #2
        let (alice_shared_with_bob_prk, alice_shared_with_bob_hk) = Hkdf::<Sha3_512>::extract(Some(&random_seed[..]), alice_shared_with_bob.raw_secret_bytes().as_slice());
        println!("Alice shared secret with Bob (Authd, Hashed, pseudo-random key bytes): {:?}\n", alice_shared_with_bob_prk.as_slice());
        let mut alice_shared_with_bob_hk_bytes = [0u8; 64];
        let _ = alice_shared_with_bob_hk.expand(&[0u8; 0], &mut alice_shared_with_bob_hk_bytes)
            .expect("64 bytes + info expand bytes is a valid length for Sha3_512 hash with expanding operation to output");
        println!("Alice shared secret with Bob (Authd, Hashed, Expanded): {:?}\n", alice_shared_with_bob_hk_bytes);

        // Correct Example Variant #3
        let alice_shared_with_bob_hk_1 = Hkdf::<Sha3_512>::new(Some(&random_seed[..]), alice_shared_with_bob.raw_secret_bytes().as_slice());
        let mut alice_shared_with_bob_hk_bytes_1 = [0u8; 64];
        let _ = alice_shared_with_bob_hk_1.expand(&[0u8; 0], &mut alice_shared_with_bob_hk_bytes_1)
            .expect("64 bytes + info expand bytes is a valid length for Sha3_512 hash with expanding operation to output");
        println!("Alice shared secret with Bob (Authd, Hashed, Expanded): {:?}\n", alice_shared_with_bob_hk_bytes_1);

        // Misuse of PRK, just for correctness checking
        let alice_shared_with_bob_hk_2 = Hkdf::<Sha3_512>::new(Some(&random_seed[..]), alice_shared_with_bob_prk.as_slice());
        let mut alice_shared_with_bob_hk_bytes_2 = [0u8; 64];
        let _ = alice_shared_with_bob_hk_2.expand(&[0u8; 0], &mut alice_shared_with_bob_hk_bytes_2)
            .expect("64 bytes + info expand bytes is a valid length for Sha3_512 hash with expanding operation to output");
        println!("Alice shared secret with Bob (Authd, Hashed, Expanded): {:?}\n", alice_shared_with_bob_hk_bytes_2);

        // Correct Example Variant #4
        let alice_shared_with_bob_hk_3 = Hkdf::<Sha3_512>::from_prk(alice_shared_with_bob_prk.as_slice())
            .expect("PRK should be conform the size of hash, i.e. PRK should be large enough and equal to used hash function output");
        let mut alice_shared_with_bob_hk_bytes_3 = [0u8; 64];
        let _ = alice_shared_with_bob_hk_3.expand(&[0u8; 0], &mut alice_shared_with_bob_hk_bytes_3)
            .expect("64 bytes + info expand bytes is a valid length for Sha3_512 hash with expanding operation to output");
        println!("Alice shared secret with Bob (Authd, Hashed, Expanded): {:?}\n", alice_shared_with_bob_hk_bytes_3);

        // Misuse of PRK, just for correctness checking
        let alice_shared_with_bob_hk_4 = Hkdf::<Sha3_224>::from_prk(alice_shared_with_bob.raw_secret_bytes().as_slice())
            .expect("PRK should be conform the size of hash, i.e. PRK should be large enough and equal to used hash function output");
        let mut alice_shared_with_bob_hk_bytes_4 = [0u8; 28];
        let _ = alice_shared_with_bob_hk_4.expand(&[0u8; 0], &mut alice_shared_with_bob_hk_bytes_4)
            .expect("28 bytes + info expand bytes is a valid length for Sha3_224 hash with expanding operation to output");
        println!("Alice shared secret with Bob (Authd, Hashed, Expanded): {:?}\n", alice_shared_with_bob_hk_bytes_4);

        assert_eq!(alice_with_bob_shared_secret_bytes, alice_shared_with_bob_hk_bytes);
        assert_eq!(alice_shared_with_bob_hk_bytes, alice_shared_with_bob_hk_bytes_1);
        assert_eq!(alice_shared_with_bob_hk_bytes_1, alice_shared_with_bob_hk_bytes_3);

    }
}

#[cfg(test)]
mod tests_p224 {
    use p224::{EncodedPoint, PublicKey, ecdh::EphemeralSecret};
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
