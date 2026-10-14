use super::*;
use std::io::Cursor;

pub(super) fn read_owned(doc: &Document) -> (Vec<Annotation>, Vec<ObjectId>) {
    let mut out = vec![];
    let mut ids = vec![];
    for (n, p) in doc.get_pages() {
        if let Ok(d) = doc.get_dictionary(p)
            && let Ok(a) = d.get(b"Annots").and_then(|o| {
                deref(doc, o)
                    .map_err(|_| lopdf::Error::ObjectNotFound(p))?
                    .as_array()
            })
        {
            for o in a {
                if let Ok(id) = o.as_reference()
                    && let Ok(d) = doc.get_dictionary(id)
                    && let Ok(bytes) = d.get(b"OpenPDFData").and_then(Object::as_str)
                    && let Ok(mut annotation) = serde_json::from_slice::<Annotation>(bytes)
                {
                    annotation.page = n as usize - 1;
                    out.push(annotation);
                    ids.push(id);
                }
            }
        }
    }
    (out, ids)
}

pub(super) fn write(doc: &mut Document, items: &[Annotation], old: &[ObjectId]) -> Result<()> {
    let pages = doc.get_pages();
    for page in pages.values() {
        let list = doc
            .get_dictionary(*page)?
            .get(b"Annots")
            .ok()
            .and_then(|o| deref(doc, o).ok())
            .and_then(|o| o.as_array().ok())
            .cloned()
            .unwrap_or_default();
        let list: Vec<Object> = list
            .into_iter()
            .filter(|o| !o.as_reference().is_ok_and(|id| old.contains(&id)))
            .collect();
        if !list.is_empty() || doc.get_dictionary(*page)?.has(b"Annots") {
            doc.get_dictionary_mut(*page)?.set("Annots", list);
        }
    }
    for a in items {
        ensure!(
            a.rect.width() > 0. && a.rect.height() > 0.,
            "Annotation must have positive dimensions"
        );
        let page = *pages
            .get(&(a.page as u32 + 1))
            .context("Annotation page does not exist")?;
        let subtype = match a.kind {
            AnnotationKind::Highlight => "Highlight",
            AnnotationKind::Underline => "Underline",
            AnnotationKind::StrikeOut => "StrikeOut",
            AnnotationKind::Ink => "Ink",
            AnnotationKind::Note => "Text",
            AnnotationKind::FreeText => "FreeText",
            AnnotationKind::Rectangle => "Square",
            AnnotationKind::Ellipse => "Circle",
            AnnotationKind::Line => "Line",
            AnnotationKind::Image => "Stamp",
        };
        let mut d = dictionary! {"Type"=>"Annot","Subtype"=>subtype,"Rect"=>a.rect.array(),"P"=>page,"F"=>4,"C"=>a.color.iter().map(|v|Object::Real(*v)).collect::<Vec<_>>(),"CA"=>a.opacity,"Contents"=>pdf_string(&a.text),"NM"=>Object::string_literal(format!("openpdf-{}",a.id)),"BS"=>dictionary!{"W"=>a.width,"S"=>"S"},"OpenPDFData"=>Object::String(serde_json::to_vec(a)?,lopdf::StringFormat::Literal)};
        let f = font(doc, a.kind == AnnotationKind::FreeText);
        let w = a.rect.width();
        let h = a.rect.height();
        let [r, g, b] = a.color;
        let mut cmd = format!(
            "q /GS gs {r} {g} {b} RG {r} {g} {b} rg {} w 1 J 1 j\n",
            a.width
        );
        let local = |p: Point| Point {
            x: p.x - a.rect.left,
            y: p.y - a.rect.bottom,
        };
        match a.kind {
            AnnotationKind::Highlight | AnnotationKind::Underline | AnnotationKind::StrikeOut => {
                let quads = if a.quads.is_empty() {
                    vec![a.rect]
                } else {
                    a.quads.clone()
                };
                d.set(
                    "QuadPoints",
                    quads
                        .iter()
                        .flat_map(|q| {
                            [
                                q.left, q.top, q.right, q.top, q.left, q.bottom, q.right, q.bottom,
                            ]
                        })
                        .map(Object::Real)
                        .collect::<Vec<_>>(),
                );
                for q in quads {
                    let x = q.left - a.rect.left;
                    let y = q.bottom - a.rect.bottom;
                    if a.kind == AnnotationKind::Highlight {
                        cmd += &format!("{x} {y} {} {} re f\n", q.width(), q.height());
                    } else {
                        let y = if a.kind == AnnotationKind::Underline {
                            y + 1.
                        } else {
                            y + q.height() / 2.
                        };
                        cmd += &format!("{x} {y} m {} {y} l S\n", x + q.width());
                    }
                }
            }
            AnnotationKind::Ink => {
                d.set(
                    "InkList",
                    a.strokes
                        .iter()
                        .map(|s| {
                            Object::Array(s.iter().flat_map(|p| [p.x.into(), p.y.into()]).collect())
                        })
                        .collect::<Vec<_>>(),
                );
                for stroke in &a.strokes {
                    for (i, p) in stroke.iter().enumerate() {
                        let p = local(*p);
                        cmd += &format!("{} {} {}\n", p.x, p.y, if i == 0 { "m" } else { "l" });
                    }
                    if stroke.len() > 1 {
                        cmd += "S\n";
                    }
                }
            }
            AnnotationKind::Note => {
                d.set("Name", "Comment");
                cmd += &format!("0 0 {w} {h} re f 0 0 0 RG 2 2 m {} {} l S", w - 2., h - 2.);
            }
            AnnotationKind::FreeText => {
                let escaped = appearance_text(&a.text)?;
                let size = (h * 0.6).min(24.);
                d.set(
                    "DA",
                    Object::string_literal(format!("/Helv {size} Tf {r} {g} {b} rg")),
                );
                cmd += &format!("BT /F1 {size} Tf 3 {} Td ", h - size - 2.);
                for (i, line) in escaped.lines().enumerate() {
                    if i > 0 {
                        cmd += &format!("0 -{} Td ", size * 1.2);
                    }
                    cmd += &format!("({line}) Tj ");
                }
                cmd += "ET";
            }
            AnnotationKind::Rectangle => {
                let s = a.width / 2.;
                cmd += &format!(
                    "{s} {s} {} {} re S",
                    (w - a.width).max(0.),
                    (h - a.width).max(0.)
                );
            }
            AnnotationKind::Ellipse => {
                let cx = w / 2.;
                let cy = h / 2.;
                let rx = (w - a.width) / 2.;
                let ry = (h - a.width) / 2.;
                let k = 0.5522848;
                cmd += &format!(
                    "{} {cy} m {} {} {} {} {cx} {} c {} {} {} {} {} {cy} c {} {} {} {} {cx} {} c {} {} {} {} {} {cy} c S",
                    cx + rx,
                    cx + rx,
                    cy + k * ry,
                    cx + k * rx,
                    cy + ry,
                    cy + ry,
                    cx - k * rx,
                    cy + ry,
                    cx - rx,
                    cy + k * ry,
                    cx - rx,
                    cx - rx,
                    cy - k * ry,
                    cx - k * rx,
                    cy - ry,
                    cy - ry,
                    cx + k * rx,
                    cy - ry,
                    cx + rx,
                    cy - k * ry,
                    cx + rx
                );
            }
            AnnotationKind::Line => {
                d.set(
                    "L",
                    vec![
                        a.rect.left.into(),
                        a.rect.bottom.into(),
                        a.rect.right.into(),
                        a.rect.top.into(),
                    ],
                );
                cmd += &format!("0 0 m {w} {h} l S");
            }
            AnnotationKind::Image => {}
        }
        cmd += "\nQ";
        let ap = if a.kind == AnnotationKind::Image {
            let bytes = a.image.as_ref().context("Missing signature image")?;
            ensure!(
                bytes.len() < 20 * 1024 * 1024,
                "Signature image exceeds 20 MiB"
            );
            let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(4096);
            limits.max_image_height = Some(4096);
            limits.max_alloc = Some(80 * 1024 * 1024);
            reader.limits(limits);
            let rgba = reader.decode()?.into_rgba8();
            let (iw, ih) = rgba.dimensions();
            let rgb: Vec<u8> = rgba.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
            let alpha: Vec<u8> = rgba.pixels().map(|p| p[3]).collect();
            let mask=doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Image","Width"=>iw as i64,"Height"=>ih as i64,"ColorSpace"=>"DeviceGray","BitsPerComponent"=>8},alpha));
            let img=doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Image","Width"=>iw as i64,"Height"=>ih as i64,"ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8,"SMask"=>mask},rgb));
            doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),w.into(),h.into()],"Resources"=>dictionary!{"XObject"=>dictionary!{"Im"=>img}}},format!("q {w} 0 0 {h} 0 0 cm /Im Do Q").into_bytes()))
        } else {
            appearance(doc, w, h, cmd, f, a.opacity)
        };
        d.set("AP", dictionary! {"N"=>ap});
        let id = doc.add_object(d);
        let page = doc.get_dictionary_mut(page)?;
        if !page.has(b"Annots") {
            page.set("Annots", Vec::<Object>::new());
        }
        page.get_mut(b"Annots")?.as_array_mut()?.push(id.into());
    }
    Ok(())
}
