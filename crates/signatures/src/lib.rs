//! Offline PDF signature adapter; trust is never inferred from mathematical validity.
use anyhow::{Context, Result, ensure};
use openpdf_identity::SigningMaterial;
pub use pdf_signer::{Appearance, SignatureReport, VerifiedSignature};

pub trait PdfSigner {
    fn sign(
        &self,
        pdf: &[u8],
        identity: &SigningMaterial,
        appearance: Option<Appearance>,
    ) -> Result<Vec<u8>>;
    fn verify(&self, pdf: &[u8]) -> Result<SignatureReport>;
}
pub struct OfflineSigner;
impl PdfSigner for OfflineSigner {
    fn sign(
        &self,
        pdf: &[u8],
        identity: &SigningMaterial,
        appearance: Option<Appearance>,
    ) -> Result<Vec<u8>> {
        let doc = lopdf::Document::load_mem(pdf)?;
        ensure!(
            !doc.is_encrypted(),
            "Signing encrypted PDFs is not supported"
        );
        for o in doc.objects.values() {
            if let Ok(d) = o.as_dict() {
                ensure!(
                    !d.has(b"Perms")
                        && !d.has(b"Lock")
                        && !d
                            .get(b"TransformMethod")
                            .and_then(lopdf::Object::as_name)
                            .is_ok_and(|n| n == b"DocMDP" || n == b"FieldMDP"),
                    "This document has certification or field restrictions. Signing is blocked to protect them."
                );
            }
        }
        let before = self.verify(pdf)?;
        ensure!(
            before.signatures.iter().all(|s| s.valid),
            "An existing signature failed integrity verification. Signing is blocked."
        );
        let opts = pdf_signer::SignOptions {
            name: Some(identity.info.display_name.clone()),
            contact_info: if identity.info.email.is_empty() {
                None
            } else {
                Some(identity.info.email.clone())
            },
            reason: Some("Signed with a local OpenPDF identity".into()),
            appearance,
            ..Default::default()
        };
        let output = pdf_signer::sign_pdf_bytes(pdf, &identity.pkcs12, &identity.password, &opts)
            .context("Cryptographic signing failed; no signed output was saved")?;
        ensure!(
            output.starts_with(pdf),
            "Signer did not preserve original bytes"
        );
        let after = self.verify(&output)?;
        ensure!(
            after.signatures.len() == before.signatures.len() + 1
                && after.signatures.iter().all(|s| s.valid)
                && after.document_intact,
            "Signed output failed verification; no file was saved"
        );
        Ok(output)
    }
    fn verify(&self, pdf: &[u8]) -> Result<SignatureReport> {
        Ok(pdf_signer::verify_pdf_bytes(pdf)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signing_reuse_incremental_and_tampering() {
        let identity = openpdf_identity::generate("Test Signer", "").unwrap();
        let pdf = openpdf_document::fixtures::sample(2, false);
        let signed = OfflineSigner.sign(&pdf, &identity, None).unwrap();
        let report = OfflineSigner.verify(&signed).unwrap();
        assert!(report.document_intact);
        assert_eq!(report.signatures[0].chain_trusted, None);
        let twice = OfflineSigner.sign(&signed, &identity, None).unwrap();
        let report = OfflineSigner.verify(&twice).unwrap();
        assert_eq!(report.signatures.len(), 2);
        assert!(report.signatures.iter().all(|s| s.valid));
        assert!(twice.starts_with(&signed));
        assert_eq!(report.signatures[0].signer, report.signatures[1].signer);
        let mut tampered = signed;
        let pos = tampered.windows(7).position(|w| w == b"OpenPDF").unwrap();
        tampered[pos] = b'X';
        let report = OfflineSigner.verify(&tampered).unwrap();
        assert!(!report.signatures[0].valid);
    }
}
