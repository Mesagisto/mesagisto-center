use std::io::BufReader;

use color_eyre::eyre::Result;

pub fn read_certs_from_file() -> Result<(Vec<rustls::Certificate>, rustls::PrivateKey)> {
  let mut cert_chain_reader = BufReader::new(std::fs::File::open("./res/server-cert.pem")?);
  let certs = rustls_pemfile::certs(&mut cert_chain_reader)?
    .into_iter()
    .map(rustls::Certificate)
    .collect::<Vec<_>>();
  assert!(!certs.is_empty());

  let mut key_reader = BufReader::new(std::fs::File::open("./res/server-key.pem")?);
  // if the file starts with "BEGIN RSA PRIVATE KEY"
  // let mut keys = rustls_pemfile::rsa_private_keys(&mut key_reader)?;
  // if the file starts with "BEGIN PRIVATE KEY"
  let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)?;
  // assert!(!keys.is_empty());
  assert_eq!(keys.len(), 1);
  let key = rustls::PrivateKey(keys.remove(0));

  Ok((certs, key))
}

#[test]
fn gen() -> Result<()> {
  use rcgen::generate_simple_self_signed;
  let subject_alt_names = vec!["hello.world.example".to_string(), "localhost".to_string()];
  let cert = generate_simple_self_signed(subject_alt_names).unwrap();
  // The certificate is now valid for localhost and the domain
  // "hello.world.example"
  std::fs::write("./res/server-cert.pem", cert.serialize_pem().unwrap())?;
  std::fs::write("./res/server-key.pem", cert.serialize_private_key_pem())?;
  Ok(())
}
