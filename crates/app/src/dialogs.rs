use super::*;
use std::io::Write;

impl App {
    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(message) = self.error.clone() {
            egui::Window::new("OpenPDF needs your attention")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                .show(ctx, |ui| {
                    ui.set_max_width(560.);
                    ui.label(message);
                    if ui.button("Close").clicked() {
                        self.error = None;
                    }
                });
        }
        if self.close_app || self.close_tab.is_some() {
            egui::Window::new("Unsaved changes").collapsible(false).anchor(egui::Align2::CENTER_CENTER,[0.,0.]).show(ctx,|ui|{
            let busy=self.tabs.iter().any(|t|t.busy);ui.label(if busy{"An operation is still running. Wait for it to finish before closing."}else{"There are unsaved changes. Return to the document to save them, or discard them."});
            if ui.button("Return to document").clicked(){self.close_app=false;self.close_tab=None;}
            if ui.add_enabled(!busy,egui::Button::new("Discard changes and close")).clicked(){if self.close_app{self.allow_exit=true;ctx.send_viewport_cmd(egui::ViewportCommand::Close);}else if let Some(i)=self.close_tab.take(){self.close(i);}self.close_app=false;}
        });
        }
        if let Some(path) = self.password_path.clone() {
            egui::Window::new("Password-protected PDF")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(
                        "Enter the document password. Encrypted PDFs open read-only in this build.",
                    );
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                    if ui.button("Open document").clicked() {
                        if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
                            self.close(i);
                        }
                        let password = std::mem::take(&mut self.password);
                        self.open(path, Some(password), ctx.clone());
                        self.password_path = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.password.clear();
                        self.password_path = None;
                    }
                });
        }
        if self.settings {
            let mut open = true;
            egui::Window::new("Preferences & local identity").open(&mut open).default_width(560.).show(ctx,|ui|{
            ui.heading("Private by design");ui.label("No account, telemetry, document uploads, or signing-time network services.");ui.separator();
            match LocalIdentityStore::new(self.profile.join("identity")).info(){Ok(Some(info))=>{ui.label(format!("Local identity: {}",info.display_name));ui.label(format!("Created {} · Expires {}",info.created,info.expires));ui.label("SHA-256 certificate fingerprint");ui.add(egui::TextEdit::multiline(&mut info.fingerprint.clone()).interactive(false));ui.label("Self-signed identity. Document integrity can be verified; the person's identity has not been independently verified.");ui.label("Exportable software key encrypted at rest; its password is protected by the OS credential store. Hardware-backed keys and identity migration are not implemented.");},Ok(None)=>{ui.label("No local signing identity yet. It will be created when you first digitally sign.");},Err(e)=>{ui.colored_label(Color32::RED,e.to_string());}}
            ui.separator();ui.label("OpenPDF Desktop 0.1.0 · GPL-3.0-or-later");ui.label("Development build. Windows/macOS validation and several release requirements are still pending. See docs/FEATURE_MATRIX.md.");if ui.button("Clear recent document list").clicked(){self.prefs.recent.clear();}
        });
            self.settings = open;
        }
        if self.signature_dialog {
            self.visual_signature_dialog(ctx);
        }
        if self.sign_dialog {
            self.digital_signature_dialog(ctx);
        }
        if self.print_dialog {
            self.print_ui(ctx);
        }
    }
    fn visual_signature_dialog(&mut self, ctx: &egui::Context) {
        if self.signature_mode == 3 && self.saved_signatures.is_none() {
            let mut saved = vec![];
            let mut total = 0;
            if let Ok(files) = std::fs::read_dir(self.profile.join("appearances")) {
                for file in files.flatten() {
                    let path = file.path();
                    if path.extension().is_none_or(|e| e != "json") {
                        continue;
                    }
                    let size = file.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
                    if size > 96 * 1024 * 1024 || total + size > 96 * 1024 * 1024 {
                        continue;
                    }
                    if let Ok(bytes) = std::fs::read(&path)
                        && let Ok(mut a) = serde_json::from_slice::<Annotation>(&bytes)
                    {
                        a.signature = true;
                        total += size;
                        saved.push((path, a));
                    }
                }
            }
            saved.sort_by(|a, b| a.0.cmp(&b.0));
            self.saved_signatures = Some(saved);
        }
        let mut open = true;
        egui::Window::new("Add your signature")
            .open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER,[0.,0.])
            .default_width(580.).show(ctx, |ui| {
                ui.label("Put your handwritten signature or initials on this PDF.");
                ui.weak("You can move or resize it before saving the document.");
                ui.add_space(6.);
                ui.horizontal(|ui| {
                    for (i, label) in ["Draw", "Type", "Import image", "Saved signatures"].iter().enumerate() {
                        ui.selectable_value(&mut self.signature_mode, i, *label);
                    }
                });
                ui.separator();
                match self.signature_mode {
                    0 => {
                        ui.label("Draw below with your mouse or pen.");
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(540.,180.), egui::Sense::drag());
                        ui.painter().rect_filled(rect, 4., Color32::WHITE);
                        ui.painter().line_segment([rect.left_bottom()-egui::vec2(0.,30.),rect.right_bottom()-egui::vec2(0.,30.)], egui::Stroke::new(1.,Color32::LIGHT_GRAY));
                        if response.drag_started() { self.signature_strokes.push(vec![]); }
                        if response.dragged() && let Some(p) = response.interact_pointer_pos()
                            && let Some(stroke) = self.signature_strokes.last_mut() {
                            stroke.push(Point{x:(p.x-rect.left()).clamp(0.,540.),y:(180.-(p.y-rect.top())).clamp(0.,180.)});
                        }
                        for stroke in &self.signature_strokes {
                            for pair in stroke.windows(2) {
                                ui.painter().line_segment([rect.left_top()+egui::vec2(pair[0].x,180.-pair[0].y),rect.left_top()+egui::vec2(pair[1].x,180.-pair[1].y)],egui::Stroke::new(2.,Color32::from_rgb(20,50,110)));
                            }
                        }
                        if ui.button("Clear drawing").clicked() { self.signature_strokes.clear(); }
                    }
                    1 => {
                        ui.label("Your name or initials");
                        ui.add(egui::TextEdit::singleline(&mut self.signature_name).hint_text("e.g. Alex Morgan").desired_width(400.));
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.signature_style,0,"Simple");
                            ui.selectable_value(&mut self.signature_style,1,"Flowing");
                        });
                        let family = if self.signature_style==0 { "Caveat" } else { "Dancing Script" };
                        egui::Frame::new().fill(Color32::WHITE).inner_margin(16.).show(ui, |ui| {
                            ui.set_min_size(egui::vec2(500.,65.));
                            ui.label(egui::RichText::new(&self.signature_name).color(Color32::from_rgb(20,46,102)).font(egui::FontId::new(40.,egui::FontFamily::Name(family.into()))));
                        });
                    }
                    2 => {
                        if ui.button("Choose signature image…").clicked()
                            && let Some(path)=rfd::FileDialog::new().add_filter("Signature images",&["png","jpg","jpeg"]).pick_file() {
                            match std::fs::metadata(&path).and_then(|m| {
                                if m.len()>20*1024*1024 { return Err(std::io::Error::other("Signature image exceeds 20 MiB")); }
                                std::fs::read(path)
                            }).map_err(anyhow::Error::from).and_then(|b| signature_appearance::imported(&b)) {
                                Ok((bytes,aspect)) => {self.signature_image=Some(bytes);self.signature_image_aspect=aspect;},
                                Err(e) => self.error=Some(e.to_string()),
                            }
                        }
                        if self.signature_image.is_some() && let Some(a)=self.make_appearance() {
                            signature_preview(ui, &a);
                        }
                        ui.weak("PNG or JPEG. Transparent margins are trimmed; proportions stay intact.");
                    }
                    _ => {
                        let mut choose = None;
                        let mut delete = None;
                        if let Some(saved) = &self.saved_signatures {
                            if saved.is_empty() { ui.label("No saved signatures yet. Create one and check Remember for next time."); }
                            egui::ScrollArea::vertical().max_height(300.).show(ui, |ui| {
                                for (path, a) in saved {
                                    ui.push_id(path, |ui| {
                                        signature_preview(ui, a);
                                        ui.horizontal(|ui| {
                                            let label = if a.text.is_empty() { "Saved signature" } else { &a.text };
                                            ui.label(label);
                                            if ui.button("Use this signature").clicked() { choose=Some(a.clone()); }
                                            if ui.small_button("Delete saved copy").clicked() { delete=Some(path.clone()); }
                                        });
                                        ui.separator();
                                    });
                                }
                            });
                        }
                        if let Some(path)=delete {
                            match std::fs::remove_file(path) {
                                Ok(()) => self.saved_signatures=None,
                                Err(e) => self.error=Some(e.to_string()),
                            }
                        }
                        if let Some(a)=choose { self.start_signature_placement(a); }
                    }
                }
                if self.signature_mode<3 {
                    ui.separator();
                    ui.checkbox(&mut self.remember_signature,"Remember for next time on this device");
                    ui.weak("Keeps a reusable copy of your handwriting. Save the PDF to keep it in this document.");
                    ui.add_space(6.);
                    ui.label("Click the page for automatic sizing, or drag a box to fit a specific space.");
                    let ready=match self.signature_mode {
                        0=>self.signature_strokes.iter().any(|s|s.len()>1),
                        1=>!self.signature_name.trim().is_empty(),
                        _=>self.signature_image.is_some(),
                    };
                    ui.horizontal(|ui| {
                        if ui.add_enabled(ready,egui::Button::new(egui::RichText::new("Place on document").color(Color32::WHITE)).fill(Color32::from_rgb(45,105,200))).clicked()
                            && let Some(a)=self.make_appearance() {
                            let result=if self.remember_signature {self.remember_appearance(&a)} else {Ok(())};
                            match result {
                                Ok(())=>self.start_signature_placement(a),
                                Err(e)=>self.error=Some(e.to_string()),
                            }
                        }
                        if ui.button("Cancel").clicked() { self.signature_dialog=false; }
                    });
                }
                ui.separator();
                ui.collapsing("Need a certificate-backed digital signature?", |ui| {
                    ui.label("A certificate signature also lets readers check whether the PDF changed after you signed it. It uses a private signing identity stored on this device.");
                    ui.label("Finish placing your handwriting, then choose Sign with certificate in the toolbar. Saving your handwriting for reuse does not create a certificate signature.");
                });
            });
        if !open {
            self.signature_dialog = false;
        }
    }

    fn start_signature_placement(&mut self, mut a: Annotation) {
        a.signature = true;
        self.placement = Some(a);
        self.tool = Tool::Signature;
        self.signature_dialog = false;
        self.drag_start = None;
        if let Some(t) = self.tabs.get_mut(self.active) {
            t.selected = None;
            t.resizing = None;
        }
        self.status = "Click to place your signature, or drag a box to fit. Esc cancels.".into();
    }

    fn remember_appearance(&mut self, a: &Annotation) -> anyhow::Result<()> {
        let dir = self.profile.join("appearances");
        std::fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
        }
        let mut tmp = tempfile::NamedTempFile::new_in(&dir)?;
        tmp.write_all(&serde_json::to_vec(a)?)?;
        tmp.as_file().sync_all()?;
        let name = format!(
            "signature-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        tmp.persist_noclobber(dir.join(name)).map_err(|e| e.error)?;
        self.saved_signatures = None;
        Ok(())
    }

    fn make_appearance(&mut self) -> Option<Annotation> {
        let mut a = Annotation::new(
            0,
            0,
            AnnotationKind::Ink,
            Rect {
                left: 0.,
                bottom: 0.,
                right: 180.,
                top: 54.,
            },
        );
        a.signature = true;
        a.color = [0.08, 0.18, 0.4];
        a.width = 1.5;
        if self.signature_mode == 0 {
            a.strokes = self
                .signature_strokes
                .iter()
                .filter(|s| s.len() > 1)
                .cloned()
                .collect();
            let mut points = a.strokes.iter().flatten();
            let first = *points.next()?;
            let mut r = Rect {
                left: first.x,
                bottom: first.y,
                right: first.x,
                top: first.y,
            };
            for p in points {
                r.left = r.left.min(p.x);
                r.right = r.right.max(p.x);
                r.bottom = r.bottom.min(p.y);
                r.top = r.top.max(p.y);
            }
            r.left -= 3.;
            r.right += 3.;
            r.bottom -= 3.;
            r.top += 3.;
            a.rect = r;
            a.move_to(signature_layout::fit_in(
                r,
                Rect {
                    left: 0.,
                    bottom: 0.,
                    right: 180.,
                    top: 54.,
                },
            ));
        } else {
            let result = if self.signature_mode == 1 {
                signature_appearance::typed(&self.signature_name, self.signature_style)
            } else {
                Ok((self.signature_image.clone()?, self.signature_image_aspect))
            };
            match result {
                Ok((bytes, aspect)) => {
                    a.kind = AnnotationKind::Image;
                    a.rect.top = 180. * aspect;
                    a.image = Some(bytes);
                    if self.signature_mode == 1 {
                        a.text = self.signature_name.trim().into();
                    }
                }
                Err(e) => {
                    self.error = Some(e.to_string());
                    return None;
                }
            }
        }
        Some(a)
    }
    fn digital_signature_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut sign = false;
        egui::Window::new("Sign with a certificate").open(&mut open).collapsible(false).anchor(egui::Align2::CENTER_CENTER,[0.,0.]).default_width(560.).show(ctx,|ui| {
            ui.heading("Let readers check that this PDF has not changed");
            ui.label("This adds a digital certificate signature and saves a new PDF copy, including your edits and any handwriting you placed.");
            ui.label("Finish your edits first. The signed copy opens with editing disabled to protect its signature.");
            ui.separator();
            if let Some(info)=&self.identity_info {
                ui.label(format!("Signing as {}",info.display_name));
                ui.collapsing("Certificate details",|ui| {
                    ui.label(format!("Expires {}",info.expires));
                    ui.label("SHA-256 fingerprint");ui.monospace(&info.fingerprint);
                });
            } else {
                ui.label("First time only: enter the name to show on your certificate.");
                ui.label("Your name");ui.text_edit_singleline(&mut self.identity_name);
                ui.label("Email (optional)");ui.text_edit_singleline(&mut self.identity_email);
                ui.weak("Your computer protects the private signing key. OpenPDF reuses it next time. No account or password setup is required.");
            }
            ui.add_space(8.);
            ui.label("This is a self-signed certificate: it detects document changes, but does not independently verify who you are.");
            let handwriting=self.tabs.get(self.active).and_then(|t|t.model.as_ref()).is_some_and(|m|m.edits.annotations.iter().any(|a|a.signature||a.kind==AnnotationKind::Image));
            ui.weak(if handwriting {"Your placed handwriting stays visible. Certificate information appears in Signature details; no extra stamp covers your handwriting."} else {"A certificate stamp will appear at the bottom of the current page. To add handwriting first, cancel and choose Add signature."});
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(self.identity_info.is_some()||!self.identity_name.trim().is_empty(),egui::Button::new("Sign and save a copy…")).clicked(){sign=true;}
                if ui.button("Cancel").clicked(){self.sign_dialog=false;}
            });
        });
        if !open {
            self.sign_dialog = false;
        }
        if !sign {
            return;
        }
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        let Some(model) = t.model.clone() else { return };
        let suggested = format!(
            "{}-signed.pdf",
            t.path.file_stem().unwrap_or_default().to_string_lossy()
        );
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF document", &["pdf"])
            .set_file_name(suggested)
            .save_file()
        else {
            return;
        };
        if path == t.path {
            self.error =
                Some("Choose a new file name for the signed copy to preserve the original.".into());
            return;
        }
        let g = model.pages[t.page];
        let has_handwriting = model
            .edits
            .annotations
            .iter()
            .any(|a| a.signature || a.kind == AnnotationKind::Image);
        let rect = signature_layout::within(
            Rect {
                left: g.crop.left + 30.,
                bottom: g.crop.bottom + 30.,
                right: g.crop.left + 270.,
                top: g.crop.bottom + 85.,
            },
            g.crop,
        );
        let appearance = (!has_handwriting).then_some(openpdf_signatures::Appearance {
            page: t.page,
            x: rect.left as f64,
            y: rect.bottom as f64,
            width: rect.width() as f64,
            height: rect.height() as f64,
            text: "Digitally signed with OpenPDF\nLocal self-signed identity".into(),
        });
        let id = t.id;
        t.busy = true;
        let name = self.identity_name.clone();
        let email = self.identity_email.clone();
        let directory = self.profile.join("identity");
        let tx = self.job_tx.clone();
        let ctx = ctx.clone();
        self.sign_dialog = false;
        self.status = "Unlocking local identity and signing…".into();
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<Vec<u8>> {
                let store = LocalIdentityStore::new(directory);
                let identity = match store.load()? {
                    Some(i) => i,
                    None => store.create(&name, &email)?,
                };
                let input = model.serialize()?;
                let bytes = OfflineSigner.sign(&input, &identity, appearance)?;
                openpdf_persistence::save_pdf(&path, &bytes, None)?;
                Ok(bytes)
            })();
            let job = match result {
                Ok(bytes) => Job::Saved {
                    id,
                    path,
                    bytes,
                    signed: true,
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
    fn print_ui(&mut self, ctx: &egui::Context) {
        #[cfg(windows)]
        {
            self.print_dialog = false;
            if let Some(t) = self.tabs.get_mut(self.active)
                && let Some(model) = t.model.clone()
            {
                let id = t.id;
                t.busy = true;
                let tx = self.job_tx.clone();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let event = match model.serialize() {
                        Ok(bytes) => Job::PrintReady { id, bytes },
                        Err(e) => Job::Error {
                            id,
                            message: format!("{e:#}"),
                        },
                    };
                    let _ = tx.send(event);
                    ctx.request_repaint();
                });
            }
        }
        #[cfg(not(windows))]
        self.print_cups_ui(ctx);
    }
    #[cfg(not(windows))]
    fn print_cups_ui(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut print = false;
        egui::Window::new("Print with system spooler")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Printer (leave empty for the system default)");
                ui.text_edit_singleline(&mut self.printer);
                ui.label("Pages (blank for all, or 1-3,5)");
                ui.text_edit_singleline(&mut self.print_range);
                ui.add(
                    egui::DragValue::new(&mut self.copies)
                        .range(1..=99)
                        .prefix("Copies "),
                );
                ui.label("Fit to page. Uses the printer's configured paper and orientation.");
                if ui.button("Print").clicked() {
                    print = true;
                }
            });
        self.print_dialog = open;
        if !print {
            return;
        }
        if !self
            .print_range
            .chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == '-')
        {
            self.error = Some("Page range must contain only numbers, commas and hyphens.".into());
            return;
        }
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        let Some(model) = t.model.clone() else { return };
        let id = t.id;
        t.busy = true;
        let tx = self.job_tx.clone();
        let ctx = ctx.clone();
        let printer = self.printer.clone();
        let range = self.print_range.clone();
        let copies = self.copies;
        self.print_dialog = false;
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<()> {
                let bytes = model.serialize()?;
                let mut tmp = tempfile::Builder::new()
                    .prefix("openpdf-print-")
                    .suffix(".pdf")
                    .tempfile()?;
                tmp.write_all(&bytes)?;
                tmp.as_file().sync_all()?;
                let mut command = std::process::Command::new("lp");
                command.args(["-n", &copies.to_string(), "-o", "fit-to-page"]);
                if !printer.is_empty() {
                    command.args(["-d", &printer]);
                }
                if !range.is_empty() {
                    command.args(["-o", &format!("page-ranges={range}")]);
                }
                let output = command.arg("--").arg(tmp.path()).output()?;
                anyhow::ensure!(
                    output.status.success(),
                    "Print service did not accept the job: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                Ok(())
            })();
            let job = match result {
                Ok(()) => Job::Printed { id },
                Err(e) => Job::Error {
                    id,
                    message: format!("{e:#}"),
                },
            };
            let _ = tx.send(job);
            ctx.request_repaint();
        });
    }
}

fn signature_preview(ui: &mut egui::Ui, a: &Annotation) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(500., 90.), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4., Color32::WHITE);
    let crop = Rect {
        left: 0.,
        bottom: 0.,
        right: 500.,
        top: 90.,
    };
    let mut preview = a.clone();
    preview.move_to(signature_layout::fit_in(
        a.rect,
        Rect {
            left: 12.,
            bottom: 12.,
            right: 488.,
            top: 78.,
        },
    ));
    canvas::paint_annotation(
        ui,
        &preview,
        openpdf_document::PageGeometry { crop, rotation: 0 },
        0,
        rect.min,
        1.,
    );
}
