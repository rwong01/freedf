//! A single owner thread confines PDFium handles. Only owned data crosses channels.
use anyhow::{Context, Result, ensure};
use openpdf_document::{DocumentModel, Rect};
use pdfium_render::prelude::*;
#[cfg(windows)]
mod printing;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender},
    },
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Glyph {
    pub text: String,
    pub rect: Rect,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderKey {
    pub document: u64,
    pub page: usize,
    pub width: u32,
    pub rotation: u16,
}
pub struct Raster {
    pub key: RenderKey,
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    pub glyphs: Vec<Glyph>,
    pub milliseconds: u128,
}
pub enum Request {
    #[cfg(windows)]
    Print {
        id: u64,
        bytes: Vec<u8>,
    },
    Open {
        id: u64,
        bytes: Vec<u8>,
        password: Option<String>,
    },
    Render(RenderKey),
    Close(u64),
    Search {
        id: u64,
        query: String,
        generation: u64,
    },
}
pub enum Event {
    #[cfg(windows)]
    Printed {
        id: u64,
        accepted: bool,
    },
    Opened {
        id: u64,
        model: Box<DocumentModel>,
    },
    Raster(Raster),
    Matches {
        id: u64,
        generation: u64,
        page: usize,
        rects: Vec<Rect>,
        finished: bool,
    },
    Error {
        id: u64,
        message: String,
    },
}
pub struct Worker {
    pub requests: SyncSender<Request>,
    pub events: Receiver<Event>,
}

pub fn library_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("OPENPDF_PDFIUM") {
        return Ok(path.into());
    }
    let executable = std::env::current_exe()?;
    let dir = executable.parent().context("No executable directory")?;
    let name = if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "libpdfium.so"
    };
    let path = dir.join(name);
    ensure!(
        path.is_file(),
        "The bundled PDFium runtime is missing. Reinstall OpenPDF (expected {}).",
        path.display()
    );
    Ok(path)
}
pub fn bind() -> Result<Pdfium> {
    let path = library_path()?;
    Ok(Pdfium::new(
        Pdfium::bind_to_library(path).context("Could not load the bundled PDFium runtime")?,
    ))
}

impl Worker {
    pub fn spawn(wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (tx, rx) = mpsc::sync_channel(32);
        let (etx, erx) = mpsc::sync_channel(8);
        std::thread::Builder::new()
            .name("pdfium-owner".into())
            .spawn(move || {
                let pdfium = match bind() {
                    Ok(p) => p,
                    Err(e) => {
                        let _ = etx.send(Event::Error {
                            id: 0,
                            message: format!("{e:#}"),
                        });
                        wake();
                        return;
                    }
                };
                let mut docs = HashMap::new();
                let mut search: Option<(u64, String, u64, usize)> = None;
                loop {
                    let req = match rx.recv_timeout(Duration::from_millis(if search.is_some() {
                        0
                    } else {
                        100
                    })) {
                        Ok(r) => Some(r),
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(_) => None,
                    };
                    match req {
                        #[cfg(windows)]
                        Some(Request::Print { id, bytes }) => {
                            let event = match printing::print(&pdfium, bytes) {
                                Ok(accepted) => Event::Printed { id, accepted },
                                Err(e) => Event::Error {
                                    id,
                                    message: format!("{e:#}"),
                                },
                            };
                            let _ = etx.send(event);
                            wake();
                        }
                        Some(Request::Open {
                            id,
                            bytes,
                            password,
                        }) => {
                            let result = (|| -> Result<_> {
                                ensure!(
                                    docs.len() < 8,
                                    "At most eight documents may be open at once"
                                );
                                let model = DocumentModel::open(bytes, password.as_deref())?;
                                let base = model.render_base()?;
                                let doc =
                                    pdfium.load_pdf_from_byte_vec(base, password.as_deref())?;
                                Ok((model, doc))
                            })();
                            match result {
                                Ok((model, doc)) => {
                                    docs.insert(id, doc);
                                    if etx
                                        .send(Event::Opened {
                                            id,
                                            model: Box::new(model),
                                        })
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    let _ = etx.send(Event::Error {
                                        id,
                                        message: format!("{e:#}"),
                                    });
                                }
                            }
                            wake();
                        }
                        Some(Request::Close(id)) => {
                            docs.remove(&id);
                            if search.as_ref().is_some_and(|s| s.0 == id) {
                                search = None;
                            }
                        }
                        Some(Request::Search {
                            id,
                            query,
                            generation,
                        }) => {
                            search = if query.trim().is_empty() {
                                None
                            } else {
                                Some((id, query, generation, 0))
                            };
                        }
                        Some(Request::Render(key)) => {
                            let result = docs
                                .get(&key.document)
                                .context("Document is no longer open")
                                .and_then(|d| render(d, key));
                            match result {
                                Ok(r) => {
                                    if etx.send(Event::Raster(r)).is_err() {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    let _ = etx.send(Event::Error {
                                        id: key.document,
                                        message: format!("{e:#}"),
                                    });
                                }
                            }
                            wake();
                        }
                        None => {
                            if let Some((id, query, generation, page)) = search.take()
                                && let Some(doc) = docs.get(&id)
                            {
                                let rects = search_page(doc, page, &query).unwrap_or_default();
                                let finished = page + 1 >= doc.pages().len() as usize;
                                if etx
                                    .send(Event::Matches {
                                        id,
                                        generation,
                                        page,
                                        rects,
                                        finished,
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                                wake();
                                if !finished {
                                    search = Some((id, query, generation, page + 1));
                                }
                            }
                        }
                    }
                }
            })
            .expect("cannot start PDF worker");
        Self {
            requests: tx,
            events: erx,
        }
    }
}
fn glyphs(page: &PdfPage<'_>) -> Result<Vec<Glyph>> {
    let text = page.text()?;
    ensure!(text.len() <= 500000, "Text layer exceeds safety limit");
    Ok(text
        .chars()
        .iter()
        .filter_map(|c| {
            let rect = c.tight_bounds().ok()?;
            Some(Glyph {
                text: c.unicode_string().unwrap_or_default(),
                rect: Rect {
                    left: rect.left().value,
                    bottom: rect.bottom().value,
                    right: rect.right().value,
                    top: rect.top().value,
                },
            })
        })
        .collect())
}
pub fn render(doc: &PdfDocument<'_>, key: RenderKey) -> Result<Raster> {
    let start = std::time::Instant::now();
    ensure!(
        key.width > 0 && key.width <= 4096,
        "Render width exceeds safety limit"
    );
    let page = doc.pages().get(i32::try_from(key.page)?)?;
    let rotation = match key.rotation % 360 {
        90 => PdfPageRenderRotation::Degrees90,
        180 => PdfPageRenderRotation::Degrees180,
        270 => PdfPageRenderRotation::Degrees270,
        _ => PdfPageRenderRotation::None,
    };
    let config = PdfRenderConfig::new()
        .set_target_width(key.width as i32)
        .set_maximum_height(4096)
        .rotate(rotation, true)
        .render_form_data(true);
    let bitmap = page.render_with_config(&config)?;
    let width = bitmap.width() as usize;
    let height = bitmap.height() as usize;
    ensure!(
        width * height <= 16_777_216,
        "Render bitmap exceeds safety limit"
    );
    Ok(Raster {
        key,
        width,
        height,
        rgba: bitmap.as_rgba_bytes(),
        glyphs: glyphs(&page)?,
        milliseconds: start.elapsed().as_millis(),
    })
}
fn search_page(doc: &PdfDocument<'_>, page: usize, query: &str) -> Result<Vec<Rect>> {
    let page = doc.pages().get(i32::try_from(page)?)?;
    let glyphs = glyphs(&page)?;
    // Map every UTF-8 byte back to its originating character, including lowercase expansions.
    let mut s = String::new();
    let mut map = vec![];
    for (i, g) in glyphs.iter().enumerate() {
        let lower = g.text.to_lowercase();
        map.extend(std::iter::repeat_n(i, lower.len()));
        s.push_str(&lower);
    }
    let query = query.to_lowercase();
    let mut out = vec![];
    for (start, _) in s.match_indices(&query).take(10000) {
        let end = start + query.len();
        if let (Some(a), Some(b)) = (map.get(start), map.get(end - 1)) {
            let mut rect = glyphs[*a].rect;
            for g in &glyphs[*a..=*b] {
                rect.left = rect.left.min(g.rect.left);
                rect.right = rect.right.max(g.rect.right);
                rect.bottom = rect.bottom.min(g.rect.bottom);
                rect.top = rect.top.max(g.rect.top);
            }
            out.push(rect);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires bundled PDFium; run with OPENPDF_PDFIUM"]
    fn actual_render_and_search() {
        let p = bind().unwrap();
        let d = p
            .load_pdf_from_byte_vec(openpdf_document::fixtures::sample(3, true), None)
            .unwrap();
        let r = render(
            &d,
            RenderKey {
                document: 1,
                page: 0,
                width: 612,
                rotation: 0,
            },
        )
        .unwrap();
        assert_eq!((r.width, r.height), (612, 792));
        assert!(r.rgba.iter().any(|b| *b < 100));
        assert_eq!(search_page(&d, 0, "searchable").unwrap().len(), 1);
        assert!(r.glyphs.iter().any(|g| g.text == "O"));
        for rotation in [90, 180, 270] {
            let r = render(
                &d,
                RenderKey {
                    document: 1,
                    page: 0,
                    width: 612,
                    rotation,
                },
            )
            .unwrap();
            let expected = if rotation == 180 {
                612. / 792.
            } else {
                792. / 612.
            };
            assert!((r.width as f32 / r.height as f32 - expected).abs() < 0.01);
        }
    }
}
