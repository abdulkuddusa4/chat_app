use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;

type HmacSha256 = Hmac<Sha256>;

pub fn create_uuid(payload: &String, key: &String) -> String {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    
    mac.update(payload.as_bytes());
    
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    
    hex::encode(code_bytes)
}

pub fn verify_uuid(payload: &String, encoded_msg: &String, key: &String) -> bool {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .expect("HMAC can take key of any size");
    
    mac.update(payload.as_bytes());
    
    let Ok(existing_code) = hex::decode(encoded_msg) else {
        return false;
    };

    mac.verify_slice(&existing_code).is_ok()
}

