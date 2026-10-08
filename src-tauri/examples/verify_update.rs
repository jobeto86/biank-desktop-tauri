use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use std::{env, fs, io::Read};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 5 {
        return Err("Uso: verify_update artefacto firma public-key version".into());
    }
    let public = String::from_utf8(STANDARD.decode(fs::read_to_string(&args[3])?.trim())?)?;
    let signature = String::from_utf8(STANDARD.decode(fs::read_to_string(&args[2])?.trim())?)?;
    let key = PublicKey::decode(&public)?;
    let signature = Signature::decode(&signature)?;
    let mut verifier = key.verify_stream(&signature)?;
    let mut file = fs::File::open(&args[1])?;
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        verifier.update(&buffer[..count]);
    }
    verifier.finalize()?;
    let expected = format!("version:{}", args[4]);
    if !signature
        .trusted_comment()
        .split_whitespace()
        .any(|part| part == expected)
    {
        return Err("La firma no acredita la versión anunciada".into());
    }
    println!("Firma y versión del artefacto: PASS");
    Ok(())
}
