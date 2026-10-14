#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
use eframe::egui::{self, Color32, TextureHandle};
use openpdf_document::{Annotation, AnnotationKind, DocumentModel, Point, Rect};
use openpdf_identity::{IdentityStore, LocalIdentityStore};
use openpdf_render::{Event, Glyph, RenderKey, Request, Worker};
use openpdf_signatures::{OfflineSigner, PdfSigner};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, mpsc},
    time::Instant,
};
mod canvas;
mod dialogs;
mod validation;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
    Select,
    Hand,
    Highlight,
    Underline,
    StrikeOut,
    Ink,
    Note,
    Text,
    Rectangle,
    Ellipse,
    Line,
    Signature,
}
#[derive(Clone, Copy, PartialEq)]
enum Fit {
    Manual,
    Width,
    Page,
}
struct Tab {
    id: u64,
    path: PathBuf,
    model: Option<DocumentModel>,
    disk: Option<Arc<Vec<u8>>>,
    zoom: f32,
    fit: Fit,
    page: usize,
    rotation: u16,
    jump: Option<usize>,
    single: bool,
    query: String,
    search_generation: u64,
    searching: bool,
    matches: Vec<(usize, Rect)>,
    match_index: usize,
    selected: Option<u64>,
    selection: Vec<(usize, Rect, String)>,
    signature_report: String,
    busy: bool,
}
impl Tab {
    fn title(&self) -> String {
        format!(
            "{}{}",
            self.path.file_name().unwrap_or_default().to_string_lossy(),
            if self.model.as_ref().is_some_and(|m| m.dirty()) {
                " *"
            } else {
                ""
            }
        )
    }
}
struct Cached {
    texture: TextureHandle,
    bytes: usize,
    last_used: u64,
    glyphs: Vec<Glyph>,
}
enum Job {
    #[cfg(windows)]
    PrintReady {
        id: u64,
        bytes: Vec<u8>,
    },
    Read {
        id: u64,
        bytes: Vec<u8>,
        password: Option<String>,
        report: String,
    },
    Saved {
        id: u64,
        path: PathBuf,
        bytes: Vec<u8>,
        signed: bool,
    },
    Error {
        id: u64,
        message: String,
    },
    Printed {
        id: u64,
    },
}
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Preferences {
    recent: Vec<PathBuf>,
    dark: bool,
}
struct App {
    worker: Worker,
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
    cache: HashMap<RenderKey, Cached>,
    pending: HashSet<RenderKey>,
    frame: u64,
    status: String,
    error: Option<String>,
    jobs: mpsc::Receiver<Job>,
    job_tx: mpsc::Sender<Job>,
    tool: Tool,
    sidebar: usize,
    search_open: bool,
    drag_start: Option<Point>,
    stroke: Vec<Point>,
    annotation_text: String,
    color: [f32; 3],
    opacity: f32,
    stroke_width: f32,
    prefs: Preferences,
    close_tab: Option<usize>,
    close_app: bool,
    allow_exit: bool,
    signature_dialog: bool,
    sign_dialog: bool,
    signature_strokes: Vec<Vec<Point>>,
    signature_drawing: bool,
    signature_mode: usize,
    signature_name: String,
    signature_image: Option<Vec<u8>>,
    placement: Option<Annotation>,
    identity_name: String,
    identity_email: String,
    identity_info: Option<openpdf_identity::IdentityInfo>,
    settings: bool,
    password_path: Option<PathBuf>,
    password: String,
    profile: PathBuf,
    print_dialog: bool,
    printer: String,
    print_range: String,
    copies: u32,
    printing_available: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, paths: Vec<PathBuf>) -> Self {
        let prefs = cc
            .storage
            .and_then(|s| eframe::get_value::<Preferences>(s, "preferences"))
            .unwrap_or_default();
        cc.egui_ctx.set_visuals(if prefs.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8., 8.);
            style.spacing.button_padding = egui::vec2(10., 6.);
        });
        let context = cc.egui_ctx.clone();
        let worker = Worker::spawn(Arc::new(move || context.request_repaint()));
        let (job_tx, jobs) = mpsc::channel();
        let profile = directories::ProjectDirs::from("org", "OpenPDF", "OpenPDF Desktop")
            .map(|p| p.data_local_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".openpdf"));
        let mut app = Self {
            worker,
            tabs: vec![],
            active: 0,
            next_id: 1,
            cache: HashMap::new(),
            pending: HashSet::new(),
            frame: 0,
            status: "Ready · All document processing stays on this device".into(),
            error: None,
            jobs,
            job_tx,
            tool: Tool::Select,
            sidebar: 0,
            search_open: false,
            drag_start: None,
            stroke: vec![],
            annotation_text: String::new(),
            color: [0.12, 0.32, 0.75],
            opacity: 1.,
            stroke_width: 2.,
            prefs,
            close_tab: None,
            close_app: false,
            allow_exit: false,
            signature_dialog: false,
            sign_dialog: false,
            signature_strokes: vec![],
            signature_drawing: false,
            signature_mode: 0,
            signature_name: String::new(),
            signature_image: None,
            placement: None,
            identity_name: String::new(),
            identity_email: String::new(),
            identity_info: None,
            settings: false,
            password_path: None,
            password: String::new(),
            profile,
            print_dialog: false,
            printer: String::new(),
            print_range: String::new(),
            copies: 1,
            printing_available: cfg!(windows)
                || (cfg!(target_os = "linux")
                    && std::process::Command::new("lp").arg("-V").output().is_ok()),
        };
        for path in paths {
            app.open(path, None, cc.egui_ctx.clone());
        }
        app
    }
    fn open(&mut self, path: PathBuf, password: Option<String>, ctx: egui::Context) {
        if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
            self.active = i;
            return;
        }
        if self.tabs.len() >= 8 {
            self.error = Some("Close a document before opening more than eight tabs.".into());
            return;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.active = self.tabs.len();
        self.tabs.push(Tab {
            id,
            path: path.clone(),
            model: None,
            disk: None,
            zoom: 1.,
            fit: Fit::Width,
            page: 0,
            rotation: 0,
            jump: None,
            single: false,
            query: String::new(),
            search_generation: 0,
            searching: false,
            matches: vec![],
            match_index: 0,
            selected: None,
            selection: vec![],
            signature_report: String::new(),
            busy: true,
        });
        self.status = "Opening document…".into();
        let tx = self.job_tx.clone();
        std::thread::spawn(move || {
            let result = openpdf_persistence::read_pdf(&path);
            let job = match result {
                Ok(bytes) => {
                    let report=match OfflineSigner.verify(&bytes){Ok(r) if r.signatures.is_empty()=>"No digital signatures found.".into(),Ok(r)=>{
                let mut s=format!("{} digital signature(s). Latest signed revision covers current file: {}\n",r.signatures.len(),r.document_intact);
                for signature in r.signatures {s+=&format!("\nSigner: {}\nCryptographic integrity: {}\nIdentity trust: not independently verified\nLater document bytes: {}\nTimestamp: {}\nRevocation: not checked offline\n{}\n",signature.signer.unwrap_or_else(||"Unknown".into()),if signature.valid{"verified"}else{"FAILED"},!signature.covers_whole_document,if signature.trusted_time.is_some(){"verified by signer library"}else{"no trusted timestamp"},signature.detail);}s
            },Err(_)=>"Signature status could not be determined. Treat this document as unverified.".into()};
                    Job::Read {
                        id,
                        bytes,
                        password,
                        report,
                    }
                }
                Err(e) => Job::Error {
                    id,
                    message: format!("{e:#}"),
                },
            };
            let _ = tx.send(job);
            ctx.request_repaint();
        });
    }
    fn open_dialog(&mut self, ctx: &egui::Context) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("PDF documents", &["pdf"])
            .pick_files()
        {
            for path in paths {
                self.open(path, None, ctx.clone());
            }
        }
    }
    fn close(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(index);
        let _ = self.worker.requests.try_send(Request::Close(tab.id));
        self.cache.retain(|k, _| k.document != tab.id);
        self.pending.retain(|k| k.document != tab.id);
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
    }
    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(job) = self.jobs.try_recv() {
            match job {
                #[cfg(windows)]
                Job::PrintReady { id, bytes } => {
                    if self
                        .worker
                        .requests
                        .try_send(Request::Print { id, bytes })
                        .is_err()
                    {
                        if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                            t.busy = false;
                        }
                        self.error =
                            Some("Print worker is busy or unavailable. Retry printing.".into());
                    }
                }
                Job::Read {
                    id,
                    bytes,
                    password,
                    report,
                } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
                        tab.disk = Some(Arc::new(bytes.clone()));
                        tab.signature_report = report;
                        if self
                            .worker
                            .requests
                            .try_send(Request::Open {
                                id,
                                bytes,
                                password,
                            })
                            .is_err()
                        {
                            tab.busy = false;
                            self.error =
                                Some("PDF worker is unavailable. Restart the application.".into());
                        }
                    }
                }
                Job::Saved {
                    id,
                    path,
                    bytes,
                    signed,
                } => {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
                        tab.busy = false;
                        if signed {
                            self.status="Digital signature verified and saved. Local signer identity is self-signed.".into();
                            self.open(path, None, ctx.clone());
                        } else {
                            tab.path = path;
                            tab.disk = Some(Arc::new(bytes));
                            if let Some(m) = &mut tab.model {
                                m.mark_saved();
                            }
                            self.status = "Saved successfully".into();
                        }
                    }
                }
                Job::Error { id, message } => {
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                        t.busy = false;
                    }
                    self.error = Some(message);
                }
                Job::Printed { id } => {
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                        t.busy = false;
                    }
                    self.status = "Print job accepted by the system spooler".into();
                }
            }
        }
        while let Ok(event) = self.worker.events.try_recv() {
            match event {
                #[cfg(windows)]
                Event::Printed { id, accepted } => {
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                        t.busy = false;
                    }
                    self.status = if accepted {
                        "Print job accepted by Windows"
                    } else {
                        "Printing cancelled"
                    }
                    .into();
                }
                Event::Opened { id, model } => {
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                        t.model = Some(*model);
                        t.busy = false;
                        self.prefs.recent.retain(|p| p != &t.path);
                        self.prefs.recent.insert(0, t.path.clone());
                        self.prefs.recent.truncate(12);
                        self.status = "Document opened".into();
                    }
                }
                Event::Raster(r) => {
                    self.pending.remove(&r.key);
                    if self.tabs.iter().any(|t| t.id == r.key.document) {
                        let bytes = r.rgba.len();
                        let texture = ctx.load_texture(
                            format!("{:?}", r.key),
                            egui::ColorImage::from_rgba_unmultiplied([r.width, r.height], &r.rgba),
                            egui::TextureOptions::LINEAR,
                        );
                        self.cache.insert(
                            r.key,
                            Cached {
                                texture,
                                bytes,
                                last_used: self.frame,
                                glyphs: r.glyphs,
                            },
                        );
                        self.status = format!("Page rendered in {} ms", r.milliseconds);
                    }
                }
                Event::Matches {
                    id,
                    generation,
                    page,
                    rects,
                    finished,
                } => {
                    if let Some(t) = self
                        .tabs
                        .iter_mut()
                        .find(|t| t.id == id && t.search_generation == generation)
                    {
                        t.matches.extend(rects.into_iter().map(|r| (page, r)));
                        t.searching = !finished;
                    }
                }
                Event::Error { id, message } => {
                    self.pending.retain(|k| k.document != id);
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                        t.busy = false;
                    }
                    self.error = Some(message);
                }
            }
        }
        while self.cache.values().map(|v| v.bytes).sum::<usize>() > 128 * 1024 * 1024 {
            let Some(key) = self
                .cache
                .iter()
                .min_by_key(|(_, v)| v.last_used)
                .map(|(k, _)| *k)
            else {
                break;
            };
            self.cache.remove(&key);
        }
    }
    fn save_document(&mut self, save_as: bool, ctx: &egui::Context) {
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        if t.busy {
            return;
        }
        let Some(model) = t.model.clone() else { return };
        let path = if save_as {
            rfd::FileDialog::new()
                .add_filter("PDF document", &["pdf"])
                .set_file_name(t.path.file_name().unwrap_or_default().to_string_lossy())
                .save_file()
        } else {
            Some(t.path.clone())
        };
        let Some(path) = path else { return };
        let expected = if path == t.path { t.disk.clone() } else { None };
        let id = t.id;
        t.busy = true;
        let tx = self.job_tx.clone();
        let ctx = ctx.clone();
        self.status = "Preparing and validating PDF…".into();
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<Vec<u8>> {
                let bytes = model.serialize()?;
                openpdf_persistence::save_pdf(
                    &path,
                    &bytes,
                    expected.as_deref().map(|v| v.as_slice()),
                )?;
                Ok(bytes)
            })();
            let job = match result {
                Ok(bytes) => Job::Saved {
                    id,
                    path,
                    bytes,
                    signed: false,
                },
                Err(e) => Job::Error {
                    id,
                    message: format!("{e:#}"),
                },
            };
            let _ = tx.send(job);
            ctx.request_repaint();
        });
    }
    fn request(&mut self, key: RenderKey) {
        if !self.cache.contains_key(&key)
            && !self.pending.contains(&key)
            && self.pending.len() < 8
            && self.worker.requests.try_send(Request::Render(key)).is_ok()
        {
            self.pending.insert(key);
        }
    }
    fn start_search(&mut self) {
        if let Some(t) = self.tabs.get_mut(self.active) {
            t.search_generation += 1;
            t.matches.clear();
            t.match_index = 0;
            t.searching = !t.query.is_empty();
            let _ = self.worker.requests.try_send(Request::Search {
                id: t.id,
                query: t.query.clone(),
                generation: t.search_generation,
            });
        }
    }
    fn add_annotation(&mut self, a: Annotation) {
        if let Some(t) = self.tabs.get_mut(self.active)
            && let Some(m) = &mut t.model
        {
            let id = a.id;
            match m.change(|e| e.annotations.push(a)) {
                Ok(()) => t.selected = Some(id),
                Err(e) => self.error = Some(e.to_string()),
            }
        }
    }
    fn shortcut(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.open_dialog(ctx);
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::S,
            )
        }) {
            self.save_document(true, ctx);
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_document(false, ctx);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
            self.search_open = true;
        }
        if let Some(t) = self.tabs.get_mut(self.active)
            && !t.busy
            && let Some(m) = &mut t.model
        {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                m.undo();
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)) {
                m.redo();
            }
            if !ctx.egui_wants_keyboard_input() {
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::C)) {
                    ctx.copy_text(t.selection.iter().map(|(_, _, s)| s.as_str()).collect());
                }
                if ctx.input(|i| i.key_pressed(egui::Key::PageDown)) {
                    t.jump = Some((t.page + 1).min(m.pages.len() - 1));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::PageUp)) {
                    t.jump = Some(t.page.saturating_sub(1));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::Delete))
                    && let Some(id) = t.selected
                {
                    let _ = m.change(|e| e.annotations.retain(|a| a.id != id));
                    t.selected = None;
                }
            }
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frame += 1;
        self.shortcut(&ctx);
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_exit
            && self
                .tabs
                .iter()
                .any(|t| t.busy || t.model.as_ref().is_some_and(|m| m.dirty()))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_app = true;
        }
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            self.open(f.path().to_path_buf(), None, ctx.clone());
        }
        egui::Panel::top("toolbar").show(ui,|ui|{
            ui.horizontal(|ui|{ui.heading(egui::RichText::new("OpenPDF").color(Color32::from_rgb(50,112,220)));ui.label(egui::RichText::new("DESKTOP  ·  DEVELOPMENT BUILD").small().weak());ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if ui.button("Preferences").clicked(){self.settings=true;}
if ui.button(if self.prefs.dark{"Light"}else{"Dark"}).clicked(){self.prefs.dark= !self.prefs.dark;ctx.set_visuals(if self.prefs.dark{egui::Visuals::dark()}else{egui::Visuals::light()});}});});
            ui.horizontal_wrapped(|ui|{
                if ui.button("Open…").on_hover_text("Ctrl+O").clicked(){self.open_dialog(&ctx);}
                let ready=self.tabs.get(self.active).is_some_and(|t|t.model.is_some()&&!t.busy);
                ui.add_enabled_ui(ready,|ui|{if ui.button("Save").clicked(){self.save_document(false,&ctx);}
if ui.button("Save as…").clicked(){self.save_document(true,&ctx);}
                    if ui.add_enabled(self.printing_available,egui::Button::new("Print…")).on_hover_text("Windows native print dialog, or Linux system CUPS print service").clicked(){self.print_dialog=true;}
                    if let Some(t)=self.tabs.get_mut(self.active)&&let Some(m)=&mut t.model{if ui.add_enabled(m.can_undo(),egui::Button::new("Undo")).clicked(){m.undo();}
if ui.add_enabled(m.can_redo(),egui::Button::new("Redo")).clicked(){m.redo();}}
                    if ui.selectable_label(self.search_open,"Search").clicked(){self.search_open= !self.search_open;}
                });
            });
            if !self.tabs.is_empty(){ui.horizontal_wrapped(|ui|{for i in 0..self.tabs.len(){if ui.selectable_label(self.active==i,self.tabs[i].title()).clicked(){self.active=i;}
if ui.small_button("×").clicked(){if self.tabs[i].busy{self.error=Some("Wait for the current operation to finish.".into());}else if self.tabs[i].model.as_ref().is_some_and(|m|m.dirty()){self.close_tab=Some(i);}else{self.close(i);}break;}}});}
            if let Some(t)=self.tabs.get_mut(self.active)&&let Some(m)=&t.model {
                ui.horizontal_wrapped(|ui|{
                    if ui.button("‹").clicked(){t.jump=Some(t.page.saturating_sub(1));}let mut page=t.page+1;if ui.add(egui::DragValue::new(&mut page).range(1..=m.pages.len())).changed(){t.jump=Some(page-1);}ui.label(format!("/ {}",m.pages.len()));if ui.button("›").clicked(){t.jump=Some((t.page+1).min(m.pages.len()-1));}ui.separator();
                    if ui.button("−").clicked(){t.zoom=(t.zoom/1.2).max(0.1);t.fit=Fit::Manual;}let mut percent=t.zoom*100.;if ui.add(egui::DragValue::new(&mut percent).range(10. ..=500.).suffix("%")).changed(){t.zoom=percent/100.;t.fit=Fit::Manual;}
if ui.button("+").clicked(){t.zoom=(t.zoom*1.2).min(5.);t.fit=Fit::Manual;}
                    if ui.selectable_label(t.fit==Fit::Width,"Fit width").clicked(){t.fit=Fit::Width;}
if ui.selectable_label(t.fit==Fit::Page,"Fit page").clicked(){t.fit=Fit::Page;}
if ui.button("Rotate").clicked(){t.rotation=(t.rotation+90)%360;}ui.checkbox(&mut t.single,"Single page");
                });
                let editable=m.editable()&&!t.busy;
                ui.horizontal_wrapped(|ui|{
                    ui.selectable_value(&mut self.tool,Tool::Select,"Select");ui.selectable_value(&mut self.tool,Tool::Hand,"Hand");
                    ui.add_enabled_ui(editable,|ui|{for (tool,label) in [(Tool::Highlight,"Highlight"),(Tool::Underline,"Underline"),(Tool::StrikeOut,"Strike"),(Tool::Ink,"Ink"),(Tool::Note,"Comment"),(Tool::Text,"Text"),(Tool::Rectangle,"Rectangle"),(Tool::Ellipse,"Ellipse"),(Tool::Line,"Line")]{ui.selectable_value(&mut self.tool,tool,label);}
if ui.button("Fill & Sign…").clicked(){self.signature_dialog=true;}});
                    if ui.add_enabled(!t.busy&&!m.encrypted&&!m.restricted&&!m.xfa,egui::Button::new("Digitally sign…")).clicked(){self.identity_info=LocalIdentityStore::new(self.profile.join("identity")).info().unwrap_or_else(|e|{self.error=Some(e.to_string());None});self.sign_dialog=true;}
                });
                if !editable{ui.label(egui::RichText::new("Protected document: ordinary edits are disabled to preserve its contents and signatures.").color(Color32::from_rgb(170,110,20)));}
            }
            if self.search_open && !self.tabs.is_empty(){let t=&mut self.tabs[self.active];let mut search=false;ui.horizontal(|ui|{let r=ui.add(egui::TextEdit::singleline(&mut t.query).hint_text("Search document…"));if r.changed(){search=true;}ui.label(format!("{} results{}",t.matches.len(),if t.searching{" · searching…"}else{""}));if !t.matches.is_empty(){if ui.button("Previous").clicked(){t.match_index=(t.match_index+t.matches.len()-1)%t.matches.len();t.jump=Some(t.matches[t.match_index].0);}
if ui.button("Next").clicked(){t.match_index=(t.match_index+1)%t.matches.len();t.jump=Some(t.matches[t.match_index].0);}}});if search{self.start_search();}}
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.tabs.get(self.active).is_some_and(|t| t.busy) {
                    ui.spinner();
                }
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(format!(
                        "Cache: {} MiB",
                        self.cache.values().map(|c| c.bytes).sum::<usize>() / 1024 / 1024
                    ));
                });
            });
        });
        if !self.tabs.is_empty() {
            egui::Panel::left("navigation")
                .default_size(205.)
                .size_range(160. ..=350.)
                .show(ui, |ui| {
                    self.sidebar_ui(ui);
                });
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(if self.prefs.dark{Color32::from_rgb(30,33,40)}else{Color32::from_rgb(227,232,240)}).inner_margin(12.)).show(ui,|ui|{
            if self.tabs.is_empty(){ui.add_space(70.);ui.vertical_centered(|ui|{ui.heading("Your documents. On your device.");ui.label("Read, annotate, fill forms and sign PDFs offline.");ui.add_space(20.);if ui.button("Open a PDF…").clicked(){self.open_dialog(&ctx);}ui.add_space(25.);for path in self.prefs.recent.clone(){if ui.link(path.file_name().unwrap_or_default().to_string_lossy()).clicked(){self.open(path,None,ctx.clone());}}ui.add_space(30.);ui.weak("Pre-release: see the feature matrix for supported and unverified workflows.");});}
            else{self.canvas(ui);}
        });
        self.dialogs(&ctx);
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "preferences", &self.prefs);
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--validate-fixtures") {
        return validation::run(
            args.get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("fixtures")),
        );
    }
    let paths = args.into_iter().map(PathBuf::from).collect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenPDF Desktop")
            .with_inner_size([1280., 900.])
            .with_min_inner_size([900., 620.]),
        ..Default::default()
    };
    eframe::run_native(
        "OpenPDF Desktop",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, paths)))),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}
