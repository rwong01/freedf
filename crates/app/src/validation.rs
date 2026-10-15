use super::*;
use anyhow::Result;
pub fn identity_roundtrip(directory: PathBuf) -> Result<()> {
    std::fs::create_dir_all(&directory)?;
    let store = LocalIdentityStore::new(directory.join("identity"));
    let identity = match store.load()? {
        Some(i) => i,
        None => store.create("OpenPDF Keychain Test", "")?,
    };
    let previous = directory.join("fingerprint.txt");
    if previous.exists() {
        anyhow::ensure!(
            std::fs::read_to_string(&previous)? == identity.info.fingerprint,
            "Persisted identity changed between processes"
        );
    }
    std::fs::write(previous, &identity.info.fingerprint)?;
    let signed = OfflineSigner.sign(
        &openpdf_document::fixtures::sample(1, false),
        &identity,
        None,
    )?;
    openpdf_persistence::save_pdf(&directory.join("keychain-signed.pdf"), &signed, None)?;
    println!(
        "OS-protected identity loaded and signed successfully; fingerprint {}",
        identity.info.fingerprint
    );
    Ok(())
}
pub fn run(directory: PathBuf) -> Result<()> {
    std::fs::create_dir_all(&directory)?;
    let start = Instant::now();
    let sample = openpdf_document::fixtures::sample(3, true);
    let mut model = DocumentModel::open(sample.clone(), None)?;
    openpdf_persistence::save_pdf(&directory.join("forms.pdf"), &sample, None)?;
    let name = model.fields.iter().find(|f| f.name == "Name").unwrap().id;
    let checkbox = model.fields.iter().find(|f| f.name == "Agree").unwrap().id;
    model.change(|e| {
        e.fields.insert(
            name,
            openpdf_document::FieldValue::Text("Alice Example".into()),
        );
        e.fields
            .insert(checkbox, openpdf_document::FieldValue::Button("Yes".into()));
    })?;
    for (i, kind) in [
        AnnotationKind::Rectangle,
        AnnotationKind::Ellipse,
        AnnotationKind::Line,
        AnnotationKind::Note,
        AnnotationKind::FreeText,
        AnnotationKind::Highlight,
        AnnotationKind::Underline,
        AnnotationKind::StrikeOut,
        AnnotationKind::Ink,
    ]
    .into_iter()
    .enumerate()
    {
        let mut a = Annotation::new(
            i as u64,
            1,
            kind,
            Rect {
                left: 50.,
                bottom: 50. + i as f32 * 65.,
                right: 300.,
                top: 90. + i as f32 * 65.,
            },
        );
        a.text = format!("Annotation {i}");
        if kind == AnnotationKind::Highlight {
            a.color = [1., 0.85, 0.];
            a.opacity = 0.3;
        }
        if kind == AnnotationKind::Ink {
            a.strokes = vec![vec![
                Point {
                    x: 60.,
                    y: a.rect.bottom + 10.,
                },
                Point {
                    x: 100.,
                    y: a.rect.top - 10.,
                },
                Point {
                    x: 200.,
                    y: a.rect.bottom + 20.,
                },
            ]];
        }
        model.change(|e| e.annotations.push(a))?;
    }
    let (appearance, aspect) = signature_appearance::typed("Alice Example", 0)?;
    let mut visual = Annotation::new(
        100,
        2,
        AnnotationKind::Image,
        Rect {
            left: 50.,
            bottom: 450.,
            right: 320.,
            top: 450. + 270. * aspect,
        },
    );
    visual.image = Some(appearance);
    visual.signature = true;
    model.change(|e| e.annotations.push(visual))?;
    let edited = model.serialize()?;
    openpdf_persistence::save_pdf(&directory.join("edited.pdf"), &edited, None)?;
    let reopened = DocumentModel::open(edited.clone(), None)?;
    anyhow::ensure!(
        reopened.edits.annotations.len() == 10,
        "Annotation roundtrip failed"
    );
    let identity = openpdf_identity::generate("OpenPDF Automated Test Identity", "")?;
    let handwriting_certified = OfflineSigner.sign(&edited, &identity, None)?;
    openpdf_persistence::save_pdf(
        &directory.join("handwriting-certified.pdf"),
        &handwriting_certified,
        None,
    )?;
    let signed = OfflineSigner.sign(
        &edited,
        &identity,
        Some(openpdf_signatures::Appearance {
            text: "OpenPDF test signature".into(),
            ..Default::default()
        }),
    )?;
    openpdf_persistence::save_pdf(&directory.join("signed.pdf"), &signed, None)?;
    let twice = OfflineSigner.sign(&signed, &identity, None)?;
    openpdf_persistence::save_pdf(&directory.join("signed-twice.pdf"), &twice, None)?;
    let mut tampered = signed.clone();
    if let Some(p) = tampered.windows(13).position(|w| w == b"Alice Example") {
        tampered[p] = b'X';
    } else {
        let p = tampered.windows(7).position(|w| w == b"OpenPDF").unwrap();
        tampered[p] = b'X';
    }
    std::fs::write(directory.join("tampered.pdf"), &tampered)?;
    anyhow::ensure!(
        !OfflineSigner.verify(&tampered)?.signatures[0].valid,
        "Tampering was not detected"
    );
    let pdfium = openpdf_render::bind()?;
    let doc = pdfium.load_pdf_from_byte_vec(edited, None)?;
    let certified_doc = pdfium.load_pdf_from_byte_vec(handwriting_certified, None)?;
    let appearance_key = RenderKey {
        document: 1,
        page: 2,
        width: 816,
        rotation: 0,
    };
    anyhow::ensure!(
        openpdf_render::render(&doc, appearance_key)?.rgba
            == openpdf_render::render(&certified_doc, appearance_key)?.rgba,
        "Certificate signing changed or covered the placed handwriting"
    );
    let render_start = Instant::now();
    let raster = openpdf_render::render(
        &doc,
        RenderKey {
            document: 1,
            page: 0,
            width: 816,
            rotation: 0,
        },
    )?;
    let render_ms = render_start.elapsed().as_millis();
    image::save_buffer(
        directory.join("pdfium-forms.png"),
        &raster.rgba,
        raster.width as u32,
        raster.height as u32,
        image::ColorType::Rgba8,
    )?;
    let large = openpdf_document::fixtures::sample(500, false);
    openpdf_persistence::save_pdf(&directory.join("500-pages.pdf"), &large, None)?;
    let report = serde_json::json!({"tests":"forms/annotations roundtrip, signature, reuse, second incremental signature, tampering, PDFium rendering","identity_fingerprint":identity.info.fingerprint,"total_ms":start.elapsed().as_millis(),"render_816px_ms":render_ms,"signed_bytes":signed.len(),"independent_validation":"Run qpdf and pdfsig; this report is internal validation only"});
    std::fs::write(
        directory.join("internal-validation.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
