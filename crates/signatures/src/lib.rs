//! Offline PDF signature adapter; trust is never inferred from mathematical validity.
use anyhow::{Context, Result, ensure};
use openpdf_identity::SigningMaterial;
pub use pdf_signer::{SignatureReport, VerifiedSignature};

/// Application-level appearance: page indices are zero-based, as in the document model.
#[derive(Clone, Debug)]
pub struct Appearance {
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub text: String,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            page: 0,
            x: 36.,
            y: 36.,
            width: 260.,
            height: 70.,
            text: String::new(),
        }
    }
}

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
        let appearance = appearance
            .map(|a| -> Result<pdf_signer::Appearance> {
                ensure!(
                    a.page < doc.get_pages().len(),
                    "Signature page is out of range"
                );
                ensure!(
                    [a.x, a.y, a.width, a.height].iter().all(|n| n.is_finite())
                        && a.width > 0.
                        && a.height > 0.,
                    "Invalid signature rectangle"
                );
                Ok(pdf_signer::Appearance {
                    page: a.page + 1,
                    x: a.x,
                    y: a.y,
                    width: a.width,
                    height: a.height,
                    text: a.text,
                    ..Default::default()
                })
            })
            .transpose()?;
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
        let mut protected = openpdf_document::DocumentModel::open(signed.clone(), None).unwrap();
        assert!(!protected.editable());
        assert!(protected.change(|e| e.annotations.clear()).is_err());
        assert_eq!(protected.serialize().unwrap(), signed);
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
    #[test]
    fn visible_signature_targets_first_and_last_model_page() {
        let identity = openpdf_identity::generate("Page placement test", "").unwrap();
        let input = openpdf_document::fixtures::sample(3, false);
        for page in [0, 2] {
            let output = OfflineSigner
                .sign(
                    &input,
                    &identity,
                    Some(Appearance {
                        page,
                        text: "Page placement test".into(),
                        ..Default::default()
                    }),
                )
                .unwrap();
            let doc = lopdf::Document::load_mem(&output).unwrap();
            let target = doc.get_pages()[&(page as u32 + 1)];
            let signature_pages: Vec<_> = doc
                .get_pages()
                .values()
                .filter(|id| {
                    doc.get_dictionary(**id)
                        .ok()
                        .and_then(|d| d.get(b"Annots").ok())
                        .and_then(|o| o.as_array().ok())
                        .is_some_and(|annots| {
                            annots.iter().any(|a| {
                                a.as_reference()
                                    .ok()
                                    .and_then(|id| doc.get_dictionary(id).ok())
                                    .is_some_and(|d| {
                                        d.get(b"FT")
                                            .and_then(lopdf::Object::as_name)
                                            .is_ok_and(|n| n == b"Sig")
                                    })
                            })
                        })
                })
                .copied()
                .collect();
            assert_eq!(signature_pages, vec![target]);
            assert!(OfflineSigner.verify(&output).unwrap().document_intact);
        }
        assert!(
            OfflineSigner
                .sign(
                    &input,
                    &identity,
                    Some(Appearance {
                        page: 3,
                        ..Default::default()
                    })
                )
                .is_err()
        );
    }
    #[test]
    fn certification_restrictions_refused() {
        let identity = openpdf_identity::generate("Policy Test", "").unwrap();
        let mut doc =
            lopdf::Document::load_mem(&openpdf_document::fixtures::sample(1, false)).unwrap();
        let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        doc.get_dictionary_mut(root)
            .unwrap()
            .set("Perms", lopdf::Dictionary::new());
        let mut bytes = vec![];
        doc.save_to(&mut bytes).unwrap();
        assert!(
            !openpdf_document::DocumentModel::open(bytes.clone(), None)
                .unwrap()
                .editable()
        );
        assert!(OfflineSigner.sign(&bytes, &identity, None).is_err());
    }
}
