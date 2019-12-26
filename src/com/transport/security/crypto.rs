use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn run_me() {
    let mut _mac = HmacSha256::new_varkey(b"secret key");
    println!("This is a test {}", "yep");
}
