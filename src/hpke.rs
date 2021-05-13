use hex::decode;
use hpke::{
    aead::{AeadTag, ChaCha20Poly1305},
    kdf::HkdfSha384,
    kem::X25519HkdfSha256,
    kex::KeyExchange,
    Deserializable, EncappedKey, Kem as KemTrait, OpModeR, OpModeS, Serializable
};
use rand::{rngs::StdRng, SeedableRng};
use serde::{Serialize, Deserialize};

const INFO_STR: &'static [u8] = b"Private analytics";

type Kem = X25519HkdfSha256;
type Aead = ChaCha20Poly1305;
type Kdf = HkdfSha384;

pub type Kex = <Kem as KemTrait>::Kex;

pub fn gen_keypair() -> (
    <Kex as KeyExchange>::PrivateKey,
    <Kex as KeyExchange>::PublicKey,
) {
    let mut csprng = StdRng::from_entropy();
    Kem::gen_keypair(&mut csprng)
}

pub fn get_privkey(privkey_str: &String) -> <Kex as KeyExchange>::PrivateKey {
    let privkey_bytes = decode(&privkey_str).unwrap();
    let privkey = <Kex as KeyExchange>::PrivateKey::from_bytes(&privkey_bytes)
        .expect("Could not deserialize privkey.");
    privkey
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HPKEOut {
    pub encapped_key: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub tag: Vec<u8>
}

pub fn hpke_encrypt(
    msg: &[u8],
    associated_data: &[u8],
    server_pk: &<Kex as KeyExchange>::PublicKey,
) -> HPKEOut {
    let mut csprng = StdRng::from_entropy();

    let (encapped_key, mut sender_ctx) =
        hpke::setup_sender::<Aead, Kdf, Kem, _>(&OpModeS::Base, server_pk, INFO_STR, &mut csprng)
        .expect("Invalid server pubkey.");
    let encapped_key = encapped_key.to_bytes().to_vec();

    let mut msg_copy = msg.to_vec();
    let tag = sender_ctx
        .seal(&mut msg_copy, associated_data)
        .expect("Encryption failed.");
    let tag = tag.to_bytes().to_vec();

    let ciphertext = msg_copy;

    HPKEOut {encapped_key, ciphertext, tag}
}

pub fn hpke_decrypt(
    server_sk: &<Kex as KeyExchange>::PrivateKey,    
    hpke_out: &HPKEOut,
    associated_data: &[u8],
) -> Vec<u8> {
    let tag =AeadTag::<Aead>::from_bytes(hpke_out.tag.as_slice())
        .expect("Could not deserialize AEAD tag.");
    let encapped_key = EncappedKey::<Kex>::from_bytes(hpke_out.encapped_key.as_slice())
        .expect("Could not deserialize the encapsulated pubkey.");

    let mut receiver_ctx =
        hpke::setup_receiver::<Aead, Kdf, Kem>(&OpModeR::Base, &server_sk, &encapped_key, INFO_STR)
            .expect("Failed to set up receiver.");

    let mut ciphertext_copy = hpke_out.ciphertext.to_vec();
    receiver_ctx
        .open(&mut ciphertext_copy, associated_data, &tag)
        .expect("Invalid ciphertext.");

    let plaintext = ciphertext_copy;

    plaintext
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::*;
    use crate::flpcp::*;
    use crate::payload::*;
    use rand::Rng;        
    use rug::Integer;
    use uuid::Uuid;    

    #[test]
    fn hpke_test() {
        let p = Integer::from(18446744073709547521u64);
        let generator = Integer::from(323234694403053661u64);
        let input_len = 100;
        let circuit = bitvector_test(p, input_len);
        let inputs = vec![Integer::from(1); input_len];
        let ctxt = Context {
            generator,
            circuit,
        };
        let mut rng = rand::thread_rng();
        let prover = Prover {
            ctxt: ctxt.clone(),
            inputs,
            seed: rng.gen(),
        };
        let (pi, _) = prover.gen_proofs();
        let payload = Payload {
            uuid: Uuid::new_v4(),
            index: 0,
            proof: pi.to_u64(),
        };
        
        let (server_privkey, server_pubkey) = gen_keypair();

        let msg = bincode::serialize(&payload).unwrap();
        let associated_data = b"";

        let hpke_out = hpke_encrypt(&msg, associated_data, &server_pubkey);

        let decrypted_msg = hpke_decrypt(
            &server_privkey,
            &hpke_out,
            associated_data,
        );

        let decrypted_payload: Payload = bincode::deserialize(&decrypted_msg[..]).unwrap();

        assert_eq!(decrypted_msg, msg);
        assert_eq!(decrypted_payload, payload);
    }
}
