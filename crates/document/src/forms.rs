use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    Text { multiline: bool },
    Checkbox,
    Radio,
    Combo,
    List { multiple: bool },
    Signature,
    Unsupported,
}
#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Text(String),
    Choice(Vec<String>),
    Button(String),
}
#[derive(Clone, Debug)]
pub struct Field {
    pub id: ObjectId,
    pub widget: ObjectId,
    pub name: String,
    pub page: usize,
    pub rect: Rect,
    pub kind: FieldKind,
    pub readonly: bool,
    pub value: FieldValue,
    pub options: Vec<(String, String)>,
    pub on_value: String,
    pub max_len: Option<usize>,
}

pub(super) fn read_fields(doc: &Document) -> Result<Vec<Field>> {
    let mut out = vec![];
    for (n, page) in doc.get_pages() {
        let d = doc.get_dictionary(page)?;
        let Some(arr) = d
            .get(b"Annots")
            .ok()
            .and_then(|o| deref(doc, o).ok())
            .and_then(|o| o.as_array().ok())
        else {
            continue;
        };
        for obj in arr {
            let Ok(widget) = obj.as_reference() else {
                continue;
            };
            let Ok(d) = doc.get_dictionary(widget) else {
                continue;
            };
            if !d
                .get(b"Subtype")
                .and_then(Object::as_name)
                .is_ok_and(|n| n == b"Widget")
            {
                continue;
            }
            let Some(ft) = inherited(doc, widget, b"FT") else {
                continue;
            };
            let ft = ft.as_name()?;
            let flags = inherited(doc, widget, b"Ff")
                .and_then(|o| o.as_i64().ok())
                .unwrap_or(0);
            let mut id = widget;
            for _ in 0..64 {
                let d = doc.get_dictionary(id)?;
                if d.has(b"T") || !d.has(b"Parent") {
                    break;
                }
                id = d.get(b"Parent")?.as_reference()?;
            }
            let name = inherited(doc, id, b"T")
                .map(|o| text(&o))
                .unwrap_or_else(|| format!("Field {}", id.0));
            let kind = match ft {
                b"Tx" => FieldKind::Text {
                    multiline: flags & (1 << 12) != 0,
                },
                b"Btn" if flags & (1 << 16) != 0 => FieldKind::Unsupported,
                b"Btn" if flags & (1 << 15) != 0 => FieldKind::Radio,
                b"Btn" => FieldKind::Checkbox,
                b"Ch" if flags & (1 << 17) != 0 => FieldKind::Combo,
                b"Ch" => FieldKind::List {
                    multiple: flags & (1 << 21) != 0,
                },
                b"Sig" => FieldKind::Signature,
                _ => FieldKind::Unsupported,
            };
            let value = inherited(doc, id, b"V");
            let value = match kind {
                FieldKind::Checkbox | FieldKind::Radio => FieldValue::Button(
                    value
                        .as_ref()
                        .and_then(|o| o.as_name().ok())
                        .map(|n| String::from_utf8_lossy(n).into_owned())
                        .unwrap_or_else(|| "Off".into()),
                ),
                FieldKind::Combo | FieldKind::List { .. } => FieldValue::Choice(
                    value
                        .as_ref()
                        .map(|o| {
                            if let Ok(a) = o.as_array() {
                                a.iter().map(text).collect()
                            } else {
                                vec![text(o)]
                            }
                        })
                        .unwrap_or_default(),
                ),
                _ => FieldValue::Text(value.as_ref().map(text).unwrap_or_default()),
            };
            let options = inherited(doc, id, b"Opt")
                .and_then(|o| o.as_array().ok().cloned())
                .unwrap_or_default()
                .iter()
                .map(|o| {
                    if let Ok(a) = o.as_array() {
                        (
                            a.first().map(text).unwrap_or_default(),
                            a.get(1).map(text).unwrap_or_default(),
                        )
                    } else {
                        (text(o), text(o))
                    }
                })
                .collect();
            let on_value = d
                .get(b"AP")
                .ok()
                .and_then(|o| deref(doc, o).ok())
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"N").ok())
                .and_then(|o| deref(doc, o).ok())
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| {
                    d.iter()
                        .find(|(k, _)| k.as_slice() != b"Off")
                        .map(|(k, _)| String::from_utf8_lossy(k).into_owned())
                })
                .unwrap_or_else(|| "Yes".into());
            let max_len = inherited(doc, id, b"MaxLen")
                .and_then(|o| o.as_i64().ok())
                .and_then(|v| usize::try_from(v).ok());
            out.push(Field {
                id,
                widget,
                name,
                page: n as usize - 1,
                rect: object_rect(d.get(b"Rect")?)?,
                kind,
                readonly: flags & 1 != 0 || flags & (1 << 13) != 0,
                value,
                options,
                on_value,
                max_len,
            });
        }
    }
    Ok(out)
}

pub(super) fn write_fields(
    doc: &mut Document,
    fields: &[Field],
    values: &BTreeMap<ObjectId, FieldValue>,
) -> Result<()> {
    for (id, value) in values {
        let matches: Vec<&Field> = fields.iter().filter(|f| f.id == *id).collect();
        ensure!(!matches.is_empty(), "Unknown form field");
        ensure!(
            matches.iter().all(|f| !f.readonly),
            "Cannot modify a readonly field"
        );
        let f = matches[0];
        match (&f.kind, value) {
            (FieldKind::Text { .. }, FieldValue::Text(s)) => {
                ensure!(
                    s.chars().count() <= f.max_len.unwrap_or(100000),
                    "Field exceeds its maximum length"
                );
                appearance_text(s)?;
                doc.get_dictionary_mut(*id)?.set("V", pdf_string(s));
            }
            (FieldKind::Combo | FieldKind::List { .. }, FieldValue::Choice(selected)) => {
                ensure!(
                    selected.iter().all(|v| f.options.iter().any(|o| &o.0 == v)),
                    "Invalid choice value"
                );
                if !matches!(f.kind, FieldKind::List { multiple: true }) {
                    ensure!(selected.len() <= 1, "Field accepts one choice");
                }
                let d = doc.get_dictionary_mut(*id)?;
                d.set(
                    "V",
                    if selected.len() == 1 {
                        pdf_string(&selected[0])
                    } else {
                        Object::Array(selected.iter().map(|s| pdf_string(s)).collect())
                    },
                );
                let indices: Vec<Object> = f
                    .options
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| selected.contains(&o.0))
                    .map(|(i, _)| (i as i64).into())
                    .collect();
                d.set("I", indices);
            }
            (FieldKind::Checkbox | FieldKind::Radio, FieldValue::Button(v)) => {
                ensure!(
                    v == "Off" || matches.iter().any(|f| &f.on_value == v),
                    "Invalid button export value"
                );
                doc.get_dictionary_mut(*id)?
                    .set("V", Object::Name(v.as_bytes().to_vec()));
            }
            _ => bail!("Unsupported form field change"),
        }
        for f in matches {
            let w = f.rect.width();
            let h = f.rect.height();
            let ft = font(doc, false);
            if let FieldValue::Button(v) = value {
                let on = appearance(
                    doc,
                    w,
                    h,
                    format!(
                        "q 1 1 1 rg 0 0 {w} {h} re f 0.1 0.3 0.7 RG 2 w 2 2 m {} {} l 2 {} m {} 2 l S Q",
                        w - 2.,
                        h - 2.,
                        h - 2.,
                        w - 2.
                    ),
                    ft,
                    1.,
                );
                let off = appearance(
                    doc,
                    w,
                    h,
                    format!("q 1 1 1 rg 0 0 {w} {h} re f 0.5 G 0 0 {w} {h} re S Q"),
                    ft,
                    1.,
                );
                let mut normal = lopdf::Dictionary::new();
                normal.set("Off", off);
                normal.set(f.on_value.as_bytes().to_vec(), on);
                let d = doc.get_dictionary_mut(f.widget)?;
                d.set("AP", dictionary! {"N"=>normal});
                d.set(
                    "AS",
                    Object::Name(if v == &f.on_value {
                        v.as_bytes().to_vec()
                    } else {
                        b"Off".to_vec()
                    }),
                );
            } else {
                let s = match value {
                    FieldValue::Text(s) => s.clone(),
                    FieldValue::Choice(v) => v
                        .iter()
                        .map(|v| {
                            f.options
                                .iter()
                                .find(|o| &o.0 == v)
                                .map(|o| o.1.clone())
                                .unwrap_or_else(|| v.clone())
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => unreachable!(),
                };
                let escaped = appearance_text(&s)?;
                let size = 12f32.min((h - 4.).max(4.));
                let mut cmd = format!(
                    "q 0 0 {w} {h} re W n 1 1 1 rg 0 0 {w} {h} re f 0 g BT /F1 {size} Tf 2 {} Td ",
                    h - size - 2.
                );
                for (i, line) in escaped.lines().enumerate() {
                    if i > 0 {
                        cmd += &format!("0 -{} Td ", size * 1.2);
                    }
                    cmd += &format!("({line}) Tj ");
                }
                cmd += "ET Q";
                let ap = appearance(doc, w, h, cmd, ft, 1.);
                doc.get_dictionary_mut(f.widget)?
                    .set("AP", dictionary! {"N"=>ap});
            }
        }
    }
    Ok(())
}
