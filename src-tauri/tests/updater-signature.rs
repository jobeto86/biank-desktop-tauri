use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
#[test]
fn real_candidate_signature_accepts_original_and_rejects_modified_bytes() {
    let public = String::from_utf8(
        STANDARD
            .decode(include_str!("../test-fixtures/updater.pub").trim())
            .unwrap(),
    )
    .unwrap();
    let signature = String::from_utf8(
        STANDARD
            .decode(include_str!("../test-fixtures/signed-candidate.txt.sig").trim())
            .unwrap(),
    )
    .unwrap();
    let key = PublicKey::decode(&public).unwrap();
    let signature = Signature::decode(&signature).unwrap();
    let artifact = include_bytes!("../test-fixtures/signed-candidate.txt");
    assert!(key.verify(artifact, &signature, true).is_ok());
    assert!(signature.trusted_comment().contains("0.3.0"));
    assert!(key.verify(b"modified candidate", &signature, true).is_err());
}
