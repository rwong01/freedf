//! Local identities. Production storage fails closed if the OS credential store is unavailable.
use anyhow::{Context, Result, ensure};
use rand::RngCore;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, KeyUsagePurpose};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityInfo {
    pub display_name: String,
    pub email: String,
    pub fingerprint: String,
    pub created: String,
    pub expires: String,
    pub certificate_der: Vec<u8>,
}
pub struct SigningMaterial {
    pub info: IdentityInfo,
    pub pkcs12: Zeroizing<Vec<u8>>,
    pub password: Zeroizing<String>,
}
pub trait IdentityStore {
    fn load(&self) -> Result<Option<SigningMaterial>>;
    fn create(&self, name: &str, email: &str) -> Result<SigningMaterial>;
}
pub struct LocalIdentityStore {
    directory: PathBuf,
}
#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    info: IdentityInfo,
    pkcs12: Vec<u8>,
    credential_id: String,
}
impl LocalIdentityStore {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
    fn path(&self) -> PathBuf {
        self.directory.join("identity.json")
    }
    pub fn info(&self) -> Result<Option<IdentityInfo>> {
        if !self.path().exists() {
            return Ok(None);
        }
        let stored: StoredIdentity = serde_json::from_slice(&fs::read(self.path())?).context(
            "Local identity is damaged; restore it rather than replacing it automatically",
        )?;
        Ok(Some(stored.info))
    }
}
impl IdentityStore for LocalIdentityStore {
    fn load(&self) -> Result<Option<SigningMaterial>> {
        if !self.path().exists() {
            return Ok(None);
        }
        let stored: StoredIdentity = serde_json::from_slice(&fs::read(self.path())?)
            .context("Local identity metadata is damaged")?;
        ensure!(
            hex::encode(Sha256::digest(&stored.info.certificate_der)) == stored.info.fingerprint,
            "Certificate fingerprint mismatch"
        );
        let entry = keyring::Entry::new("org.openpdf.desktop", &stored.credential_id)?;
        let password=Zeroizing::new(entry.get_password().context("Cannot unlock the signing identity. Unlock your OS keychain or Secret Service and retry.")?);
        // Authenticate encrypted material before returning it to a signer.
        let ks = p12_keystore::KeyStore::from_pkcs12(&stored.pkcs12, &password)
            .context("Encrypted signing identity is damaged or cannot be unlocked")?;
        ensure!(ks.entries().count() > 0, "Identity contains no entries");
        Ok(Some(SigningMaterial {
            info: stored.info,
            pkcs12: Zeroizing::new(stored.pkcs12),
            password,
        }))
    }
    fn create(&self, name: &str, email: &str) -> Result<SigningMaterial> {
        ensure!(
            !self.path().exists(),
            "An identity already exists. It will not be overwritten."
        );
        fs::create_dir_all(&self.directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.directory, fs::Permissions::from_mode(0o700))?;
        }
        let material = generate(name, email)?;
        let credential_id = format!("identity-{}", material.info.fingerprint);
        let entry = keyring::Entry::new("org.openpdf.desktop", &credential_id)?;
        entry.set_password(&material.password).context("Secure OS storage is unavailable. Enable Windows Credential Manager, macOS Keychain, or Linux Secret Service. No plaintext key was saved.")?;
        let stored = StoredIdentity {
            info: material.info.clone(),
            pkcs12: material.pkcs12.to_vec(),
            credential_id,
        };
        let result = write_new(&self.path(), &serde_json::to_vec(&stored)?);
        if result.is_err() {
            let _ = entry.delete_credential();
        }
        result?;
        Ok(material)
    }
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = tempfile::NamedTempFile::new_in(path.parent().context("Invalid identity path")?)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist_noclobber(path).map_err(|e| e.error)?;
    Ok(())
}
/// Cryptographic generation, also usable by tests without a production keychain.
/// Callers must not persist the returned password outside a protected key store.
pub fn generate(name: &str, email: &str) -> Result<SigningMaterial> {
    let name = name.trim();
    ensure!(
        !name.is_empty() && name.len() <= 200 && !name.chars().any(char::is_control),
        "Enter a display name of 1–200 bytes"
    );
    ensure!(
        email.len() <= 254 && !email.chars().any(char::is_control),
        "Invalid email address"
    );
    let key = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)?;
    let mut params = CertificateParams::new(Vec::<String>::new())?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, name);
    params.distinguished_name = dn;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now - time::Duration::minutes(5);
    params.not_after = now + time::Duration::days(365 * 3);
    let mut serial = [0u8; 20];
    rand::rngs::OsRng.fill_bytes(&mut serial);
    serial[0] &= 0x7f;
    serial[0] |= 1;
    params.serial_number = Some(serial.to_vec().into());
    let cert = params.self_signed(&key)?;
    let der = cert.der().to_vec();
    let key_der = Zeroizing::new(key.serialize_der());
    let cert_p12 = p12_keystore::Certificate::from_der(&der)?;
    let chain =
        p12_keystore::PrivateKeyChain::new(&key_der, b"OpenPDF local identity", vec![cert_p12]);
    let mut ks = p12_keystore::KeyStore::new();
    ks.add_entry(
        "openpdf",
        p12_keystore::KeyStoreEntry::PrivateKeyChain(chain),
    );
    let mut random = Zeroizing::new([0u8; 32]);
    rand::rngs::OsRng.fill_bytes(random.as_mut());
    let password = Zeroizing::new(hex::encode(random.as_ref()));
    let pkcs12 = Zeroizing::new(ks.writer(&password).write()?);
    let info = IdentityInfo {
        display_name: name.into(),
        email: email.into(),
        fingerprint: hex::encode(Sha256::digest(&der)),
        created: now.date().to_string(),
        expires: params.not_after.date().to_string(),
        certificate_der: der,
    };
    Ok(SigningMaterial {
        info,
        pkcs12,
        password,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_random_identities_encrypted() {
        let a = generate("Alice", "").unwrap();
        let b = generate("Alice", "").unwrap();
        assert_ne!(a.info.fingerprint, b.info.fingerprint);
        assert!(p12_keystore::KeyStore::from_pkcs12(&a.pkcs12, "wrong password").is_err());
        assert!(p12_keystore::KeyStore::from_pkcs12(&a.pkcs12, &a.password).is_ok());
    }
    #[test]
    fn invalid_names() {
        assert!(generate("", "").is_err());
        assert!(generate("A\nB", "").is_err());
    }
}
