//! Owned editing state. No renderer handles or UI types cross this boundary.
use anyhow::{Context, Result, bail, ensure};
use lopdf::{Document, Object, ObjectId, Stream, dictionary};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

mod annotations;
pub mod fixtures;
mod forms;
pub use forms::{Field, FieldKind, FieldValue};

pub const MAX_FILE_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_PAGES: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}
impl Rect {
    pub fn new(a: Point, b: Point) -> Self {
        Self {
            left: a.x.min(b.x),
            bottom: a.y.min(b.y),
            right: a.x.max(b.x),
            top: a.y.max(b.y),
        }
    }
    pub fn width(self) -> f32 {
        self.right - self.left
    }
    pub fn height(self) -> f32 {
        self.top - self.bottom
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.left && p.x <= self.right && p.y >= self.bottom && p.y <= self.top
    }
    pub fn array(self) -> Vec<Object> {
        vec![
            self.left.into(),
            self.bottom.into(),
            self.right.into(),
            self.top.into(),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageGeometry {
    pub crop: Rect,
    pub rotation: u16,
}
impl PageGeometry {
    pub fn size(self, view_rotation: u16) -> (f32, f32) {
        if (self.rotation + view_rotation) % 180 == 90 {
            (self.crop.height(), self.crop.width())
        } else {
            (self.crop.width(), self.crop.height())
        }
    }
    /// Top-left display coordinates in points, before zoom/DPI/scroll translation.
    pub fn to_display(self, p: Point, view_rotation: u16) -> Point {
        let x = p.x - self.crop.left;
        let y = p.y - self.crop.bottom;
        let w = self.crop.width();
        let h = self.crop.height();
        match (self.rotation + view_rotation) % 360 {
            90 => Point { x: y, y: x },
            180 => Point { x: w - x, y },
            270 => Point { x: h - y, y: w - x },
            _ => Point { x, y: h - y },
        }
    }
    pub fn from_display(self, p: Point, view_rotation: u16) -> Point {
        let w = self.crop.width();
        let h = self.crop.height();
        let (x, y) = match (self.rotation + view_rotation) % 360 {
            90 => (p.y, p.x),
            180 => (w - p.x, p.y),
            270 => (w - p.y, h - p.x),
            _ => (p.x, h - p.y),
        };
        Point {
            x: x + self.crop.left,
            y: y + self.crop.bottom,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationKind {
    Highlight,
    Underline,
    StrikeOut,
    Ink,
    Note,
    FreeText,
    Rectangle,
    Ellipse,
    Line,
    Image,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub id: u64,
    pub page: usize,
    pub kind: AnnotationKind,
    pub rect: Rect,
    pub color: [f32; 3],
    pub opacity: f32,
    pub width: f32,
    pub text: String,
    pub strokes: Vec<Vec<Point>>,
    pub quads: Vec<Rect>,
    /// PNG/JPEG bytes only. Never a path into someone else's document.
    pub image: Option<Vec<u8>>,
}
impl Annotation {
    pub fn new(id: u64, page: usize, kind: AnnotationKind, rect: Rect) -> Self {
        Self {
            id,
            page,
            kind,
            rect,
            color: [0.16, 0.35, 0.8],
            opacity: 1.,
            width: 2.,
            text: String::new(),
            strokes: vec![],
            quads: vec![],
            image: None,
        }
    }
    pub fn move_to(&mut self, rect: Rect) {
        let sx = rect.width() / self.rect.width().max(0.01);
        let sy = rect.height() / self.rect.height().max(0.01);
        let map = |p: Point| Point {
            x: rect.left + (p.x - self.rect.left) * sx,
            y: rect.bottom + (p.y - self.rect.bottom) * sy,
        };
        for stroke in &mut self.strokes {
            for p in stroke {
                *p = map(*p);
            }
        }
        for q in &mut self.quads {
            *q = Rect::new(
                map(Point {
                    x: q.left,
                    y: q.bottom,
                }),
                map(Point {
                    x: q.right,
                    y: q.top,
                }),
            );
        }
        self.rect = rect;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edits {
    pub annotations: Vec<Annotation>,
    pub fields: BTreeMap<ObjectId, FieldValue>,
}

#[derive(Clone)]
pub struct DocumentModel {
    original: Arc<Vec<u8>>,
    pub pages: Vec<PageGeometry>,
    pub fields: Vec<Field>,
    pub signed: bool,
    pub restricted: bool,
    pub encrypted: bool,
    pub xfa: bool,
    pub edits: Edits,
    saved: Edits,
    undo: Vec<Edits>,
    redo: Vec<Edits>,
    owned_annotation_ids: Vec<ObjectId>,
    pub bookmarks: Vec<(String, usize)>,
}

impl DocumentModel {
    pub fn open(bytes: Vec<u8>, password: Option<&str>) -> Result<Self> {
        ensure!(
            bytes.len() <= MAX_FILE_BYTES,
            "Document exceeds the 256 MiB safety limit"
        );
        let mut doc = Document::load_mem(&bytes).context("The PDF structure could not be read")?;
        let encrypted = doc.is_encrypted();
        if encrypted {
            doc.decrypt(password.unwrap_or(""))
                .context("This PDF needs the correct password")?;
        }
        let page_ids = doc.get_pages();
        ensure!(
            !page_ids.is_empty() && page_ids.len() <= MAX_PAGES,
            "PDF must have between 1 and 10,000 pages"
        );
        let mut pages = vec![];
        for id in page_ids.values() {
            let crop = inherited(&doc, *id, b"CropBox")
                .or_else(|| inherited(&doc, *id, b"MediaBox"))
                .context("Page has no media box")?;
            let crop = object_rect(&crop)?;
            ensure!(
                crop.width() > 0.
                    && crop.height() > 0.
                    && crop.width() <= 20000.
                    && crop.height() <= 20000.,
                "Page dimensions exceed safety limits"
            );
            let rotation = inherited(&doc, *id, b"Rotate")
                .and_then(|o| o.as_i64().ok())
                .unwrap_or(0)
                .rem_euclid(360) as u16;
            ensure!(rotation.is_multiple_of(90), "Unsupported page rotation");
            pages.push(PageGeometry { crop, rotation });
        }
        let signed = doc
            .objects
            .values()
            .any(|o| o.as_dict().is_ok_and(|d| d.has(b"ByteRange")));
        let restricted = doc.objects.values().any(|o| {
            o.as_dict().is_ok_and(|d| {
                d.has(b"Perms")
                    || d.get(b"TransformMethod")
                        .and_then(Object::as_name)
                        .is_ok_and(|n| n == b"DocMDP" || n == b"FieldMDP")
            })
        });
        let xfa = doc
            .objects
            .values()
            .any(|o| o.as_dict().is_ok_and(|d| d.has(b"XFA")));
        let fields = forms::read_fields(&doc)?;
        let (annotations, owned_annotation_ids) = annotations::read_owned(&doc);
        let edits = Edits {
            annotations,
            fields: BTreeMap::new(),
        };
        let bookmarks = read_bookmarks(&doc, &page_ids);
        Ok(Self {
            original: Arc::new(bytes),
            pages,
            fields,
            signed,
            restricted,
            encrypted,
            xfa,
            saved: edits.clone(),
            edits,
            undo: vec![],
            redo: vec![],
            owned_annotation_ids,
            bookmarks,
        })
    }
    pub fn original(&self) -> Arc<Vec<u8>> {
        self.original.clone()
    }
    pub fn render_base(&self) -> Result<Vec<u8>> {
        if self.owned_annotation_ids.is_empty() || !self.editable() {
            return Ok(self.original.as_ref().clone());
        }
        let mut doc = Document::load_mem(&self.original)?;
        annotations::write(&mut doc, &[], &self.owned_annotation_ids)?;
        let mut out = vec![];
        doc.save_to(&mut out)?;
        Ok(out)
    }
    pub fn editable(&self) -> bool {
        !self.signed && !self.restricted && !self.encrypted && !self.xfa
    }
    pub fn dirty(&self) -> bool {
        self.edits != self.saved
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn change(&mut self, f: impl FnOnce(&mut Edits)) -> Result<()> {
        ensure!(
            self.editable(),
            "Editing is protected for signed, encrypted, certified, or XFA documents. The original is preserved."
        );
        let before = self.edits.clone();
        f(&mut self.edits);
        if before != self.edits {
            self.undo.push(before);
            self.redo.clear();
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
        }
        Ok(())
    }
    pub fn undo(&mut self) {
        if let Some(e) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.edits, e));
        }
    }
    pub fn redo(&mut self) {
        if let Some(e) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.edits, e));
        }
    }
    pub fn mark_saved(&mut self) {
        self.saved = self.edits.clone();
    }
    pub fn serialize(&self) -> Result<Vec<u8>> {
        if !self.dirty() && self.edits == Edits::default() {
            return Ok(self.original.as_ref().clone());
        }
        if !self.editable() {
            ensure!(!self.dirty(), "Cannot modify a protected PDF");
            return Ok(self.original.as_ref().clone());
        }
        let mut doc = Document::load_mem(&self.original)?;
        annotations::write(
            &mut doc,
            &self.edits.annotations,
            &self.owned_annotation_ids,
        )?;
        forms::write_fields(&mut doc, &self.fields, &self.edits.fields)?;
        let mut out = Vec::new();
        doc.save_to(&mut out)?;
        ensure!(out.len() <= MAX_FILE_BYTES, "Output exceeds safety limit");
        Ok(out)
    }
}

pub(crate) fn deref<'a>(doc: &'a Document, obj: &'a Object) -> Result<&'a Object> {
    let mut o = obj;
    for _ in 0..64 {
        if let Object::Reference(id) = o {
            o = doc.get_object(*id)?;
        } else {
            return Ok(o);
        }
    }
    bail!("Cyclic or excessive PDF object references")
}
fn inherited(doc: &Document, mut id: ObjectId, key: &[u8]) -> Option<Object> {
    for _ in 0..64 {
        let d = doc.get_dictionary(id).ok()?;
        if let Ok(v) = d.get(key) {
            return deref(doc, v).ok().cloned();
        }
        id = d.get(b"Parent").and_then(Object::as_reference).ok()?;
    }
    None
}
pub(crate) fn object_rect(o: &Object) -> Result<Rect> {
    let a = o.as_array()?;
    ensure!(a.len() == 4, "Invalid PDF rectangle");
    let v: Vec<f32> = a
        .iter()
        .map(Object::as_float)
        .collect::<std::result::Result<_, _>>()?;
    ensure!(
        v.iter().all(|v| v.is_finite()),
        "Non-finite PDF coordinates"
    );
    Ok(Rect::new(
        Point { x: v[0], y: v[1] },
        Point { x: v[2], y: v[3] },
    ))
}
pub(crate) fn pdf_string(s: &str) -> Object {
    let mut b = vec![0xfe, 0xff];
    for u in s.encode_utf16() {
        b.extend_from_slice(&u.to_be_bytes());
    }
    Object::String(b, lopdf::StringFormat::Hexadecimal)
}
pub(crate) fn text(o: &Object) -> String {
    let Ok(b) = o.as_str() else {
        return String::new();
    };
    if b.starts_with(&[0xfe, 0xff]) {
        String::from_utf16_lossy(
            &b[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect::<Vec<_>>(),
        )
    } else {
        b.iter().map(|b| char::from(*b)).collect()
    }
}
pub(crate) fn font(doc: &mut Document, italic: bool) -> ObjectId {
    doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>if italic {"Helvetica-Oblique"} else {"Helvetica"},"Encoding"=>"WinAnsiEncoding"})
}
/// Strictly restrict generated base-font appearance strings until embedded Unicode fonts are supported.
pub(crate) fn appearance_text(s: &str) -> Result<String> {
    ensure!(
        s.chars()
            .all(|c| c.is_ascii() && (c == '\n' || c == '\t' || !c.is_control())),
        "This appearance currently supports ASCII text only. Unicode PDF reading is supported."
    );
    Ok(s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)"))
}
pub(crate) fn appearance(
    doc: &mut Document,
    w: f32,
    h: f32,
    commands: String,
    font: ObjectId,
    opacity: f32,
) -> ObjectId {
    let resources = dictionary! {"Font"=>dictionary!{"F1"=>font},"ExtGState"=>dictionary!{"GS"=>dictionary!{"Type"=>"ExtGState","CA"=>opacity,"ca"=>opacity}}};
    doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),w.into(),h.into()],"Resources"=>resources},commands.into_bytes()))
}
fn read_bookmarks(doc: &Document, pages: &BTreeMap<u32, ObjectId>) -> Vec<(String, usize)> {
    fn walk(
        doc: &Document,
        mut id: ObjectId,
        pages: &BTreeMap<u32, ObjectId>,
        out: &mut Vec<(String, usize)>,
        seen: &mut std::collections::HashSet<ObjectId>,
    ) {
        while out.len() < 10000 && seen.insert(id) {
            let Ok(d) = doc.get_dictionary(id) else { break };
            if let Ok(dest) = d.get(b"Dest").or_else(|_| {
                d.get(b"A")
                    .and_then(Object::as_dict)
                    .and_then(|a| a.get(b"D"))
            }) && let Ok(Object::Array(a)) = deref(doc, dest)
                && let Some(Object::Reference(page)) = a.first()
                && let Some((n, _)) = pages.iter().find(|(_, p)| *p == page)
            {
                out.push((
                    d.get(b"Title").map(text).unwrap_or_default(),
                    *n as usize - 1,
                ));
            }
            if seen.len() < 10000
                && let Ok(child) = d.get(b"First").and_then(Object::as_reference)
            {
                walk(doc, child, pages, out, seen);
            }
            let Ok(next) = d.get(b"Next").and_then(Object::as_reference) else {
                break;
            };
            id = next;
        }
    }
    let mut out = vec![];
    if let Ok(root) = doc.trailer.get(b"Root").and_then(Object::as_reference)
        && let Ok(cat) = doc.get_dictionary(root)
        && let Ok(outlines) = cat.get(b"Outlines").and_then(Object::as_reference)
        && let Ok(d) = doc.get_dictionary(outlines)
        && let Ok(first) = d.get(b"First").and_then(Object::as_reference)
    {
        walk(doc, first, pages, &mut out, &mut Default::default());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinates_roundtrip_all_rotations_crop_zoom_dpi() {
        for rotation in [0, 90, 180, 270] {
            for view in [0, 90, 180, 270] {
                let g = PageGeometry {
                    crop: Rect {
                        left: 20.,
                        bottom: 30.,
                        right: 620.,
                        top: 830.,
                    },
                    rotation,
                };
                for p in [
                    Point { x: 20., y: 30. },
                    Point { x: 145., y: 705. },
                    Point { x: 620., y: 830. },
                ] {
                    for zoom in [0.25, 1., 2.5] {
                        for dpi in [1., 1.5, 2.] {
                            let d = g.to_display(p, view);
                            let screen = Point {
                                x: d.x * zoom * dpi + 51.,
                                y: d.y * zoom * dpi - 312.,
                            };
                            let result = g.from_display(
                                Point {
                                    x: (screen.x - 51.) / zoom / dpi,
                                    y: (screen.y + 312.) / zoom / dpi,
                                },
                                view,
                            );
                            assert!(
                                (p.x - result.x).abs() < 0.001 && (p.y - result.y).abs() < 0.001
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn malformed_rejected() {
        assert!(DocumentModel::open(b"not a PDF".to_vec(), None).is_err());
    }
}
