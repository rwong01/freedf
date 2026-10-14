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
        let mut open = true;
        egui::Window::new("Create a visual signature").open(&mut open).default_width(600.).show(ctx,|ui|{
            ui.label("A visual signature is an appearance. Use Digitally sign for cryptographic document integrity.");
            ui.horizontal(|ui|{ui.selectable_value(&mut self.signature_mode,0,"Draw");ui.selectable_value(&mut self.signature_mode,1,"Type");ui.selectable_value(&mut self.signature_mode,2,"Import image");ui.selectable_value(&mut self.signature_mode,3,"Saved appearances");});
            match self.signature_mode{
                0=>{let(rect,response)=ui.allocate_exact_size(egui::vec2(540.,180.),egui::Sense::drag());ui.painter().rect_filled(rect,4.,Color32::WHITE);ui.painter().line_segment([rect.left_bottom()-egui::vec2(0.,30.),rect.right_bottom()-egui::vec2(0.,30.)],egui::Stroke::new(1.,Color32::LIGHT_GRAY));
                    if response.drag_started(){self.signature_strokes.push(vec![]);self.signature_drawing=true;}
                    if response.dragged()&&let Some(p)=response.interact_pointer_pos()&&let Some(stroke)=self.signature_strokes.last_mut(){stroke.push(Point{x:(p.x-rect.left()).clamp(0.,540.),y:(180.-(p.y-rect.top())).clamp(0.,180.)});}
                    if response.drag_stopped(){self.signature_drawing=false;}
                    for stroke in&self.signature_strokes{for pair in stroke.windows(2){ui.painter().line_segment([rect.left_top()+egui::vec2(pair[0].x,180.-pair[0].y),rect.left_top()+egui::vec2(pair[1].x,180.-pair[1].y)],egui::Stroke::new(2.,Color32::from_rgb(20,50,110)));}}
                    if ui.button("Clear drawing").clicked(){self.signature_strokes.clear();}
                }
                1=>{ui.label("Name or initials");ui.text_edit_singleline(&mut self.signature_name);ui.horizontal(|ui|{ui.selectable_value(&mut self.signature_style,0,"Caveat");ui.selectable_value(&mut self.signature_style,1,"Dancing Script");});let family=if self.signature_style==0{"Caveat"}else{"Dancing Script"};ui.label(egui::RichText::new(&self.signature_name).font(egui::FontId::new(36.,egui::FontFamily::Name(family.into()))));ui.weak("Bundled handwritten fonts · SIL Open Font License");}
                2=>{if ui.button("Choose PNG or JPEG…").clicked()&& let Some(path)=rfd::FileDialog::new().add_filter("Signature images",&["png","jpg","jpeg"]).pick_file(){match std::fs::read(path){Ok(bytes)if bytes.len()<=20*1024*1024=>{
                    let result=(||->anyhow::Result<_>{let mut reader=image::ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format()?;let mut limits=image::Limits::default();limits.max_image_width=Some(4096);limits.max_image_height=Some(4096);limits.max_alloc=Some(80*1024*1024);reader.limits(limits);let im=reader.decode()?;anyhow::ensure!(im.width()>0&&im.height()>0,"Empty image");Ok(())})();match result{Ok(())=>self.signature_image=Some(bytes),Err(e)=>self.error=Some(e.to_string())}
                },Ok(_)=>self.error=Some("Signature image exceeds 20 MiB".into()),Err(e)=>self.error=Some(e.to_string())}}
if self.signature_image.is_some(){ui.label("Image loaded. Transparency is preserved.");}ui.weak("Crop and background removal are not yet supported.");}
                _=>{let dir=self.profile.join("appearances");if let Ok(files)=std::fs::read_dir(&dir){for file in files.flatten(){if file.path().extension().is_some_and(|e|e=="json")&&let Ok(bytes)=std::fs::read(file.path())&&let Ok(a)=serde_json::from_slice::<Annotation>(&bytes){ui.horizontal(|ui|{if ui.button(file.file_name().to_string_lossy()).clicked(){self.placement=Some(a);self.tool=Tool::Signature;self.signature_dialog=false;}
if ui.small_button("Delete").clicked()&& let Err(e)=std::fs::remove_file(file.path()){self.error=Some(e.to_string());}});}}}else{ui.label("No saved appearances.");}}
            }
            if self.signature_mode<3 {
                ui.separator();ui.label("Click Place, then click the page. Use Select to move it; resize in Comments.");
                let ready=match self.signature_mode{0=>self.signature_strokes.iter().any(|s|s.len()>1),1=>!self.signature_name.trim().is_empty(),_=>self.signature_image.is_some()};
                ui.horizontal(|ui|{
                    if ui.add_enabled(ready,egui::Button::new("Place signature")).clicked(){self.placement=self.make_appearance();self.tool=Tool::Signature;self.signature_dialog=false;}
                    if ui.add_enabled(ready,egui::Button::new("Save reusable appearance locally")).clicked()&& let Some(a)=self.make_appearance(){let dir=self.profile.join("appearances");let result=(||->anyhow::Result<()>{std::fs::create_dir_all(&dir)?;#[cfg(unix)]{use std::os::unix::fs::PermissionsExt;std::fs::set_permissions(&dir,std::fs::Permissions::from_mode(0o700))?;}let mut tmp=tempfile::NamedTempFile::new_in(&dir)?;tmp.write_all(&serde_json::to_vec(&a)?)?;tmp.as_file().sync_all()?;let name=format!("signature-{}.json",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis());tmp.persist_noclobber(dir.join(name)).map_err(|e|e.error)?;Ok(())})();match result{Ok(())=>self.status="Signature appearance saved locally".into(),Err(e)=>self.error=Some(e.to_string())}}
                });
            }
        });
        if !open {
            self.signature_dialog = false;
        }
    }
    fn make_appearance(&mut self) -> Option<Annotation> {
        if self.signature_mode == 1 {
            return match signature_appearance::typed(&self.signature_name, self.signature_style) {
                Ok((bytes, aspect)) => {
                    let mut a = Annotation::new(
                        0,
                        0,
                        AnnotationKind::Image,
                        Rect {
                            left: 0.,
                            bottom: 0.,
                            right: 270.,
                            top: 270. * aspect,
                        },
                    );
                    a.image = Some(bytes);
                    Some(a)
                }
                Err(e) => {
                    self.error = Some(e.to_string());
                    None
                }
            };
        }
        let kind = match self.signature_mode {
            0 => AnnotationKind::Ink,
            1 => AnnotationKind::FreeText,
            2 => AnnotationKind::Image,
            _ => return None,
        };
        let mut a = Annotation::new(
            0,
            0,
            kind,
            Rect {
                left: 0.,
                bottom: 0.,
                right: 270.,
                top: 90.,
            },
        );
        a.color = [0.08, 0.18, 0.4];
        match kind {
            AnnotationKind::Ink => {
                a.strokes = self
                    .signature_strokes
                    .iter()
                    .map(|s| {
                        s.iter()
                            .map(|p| Point {
                                x: p.x / 2.,
                                y: p.y / 2.,
                            })
                            .collect()
                    })
                    .collect();
                a.width = 1.5;
            }
            AnnotationKind::FreeText => {
                a.text = self.signature_name.clone();
                a.rect.top = 45.;
            }
            AnnotationKind::Image => {
                a.image = self.signature_image.clone();
            }
            _ => {}
        }
        Some(a)
    }
    fn digital_signature_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut sign = false;
        egui::Window::new("Digitally sign & save a copy").open(&mut open).default_width(560.).show(ctx,|ui|{
            ui.label("This signs the complete saved PDF with your local identity. It detects changes to signed bytes. A self-signed identity does not independently prove your legal identity.");
            if let Some(info)=&self.identity_info{ui.label(format!("Signing as {}",info.display_name));ui.label(format!("Certificate expires {}",info.expires));ui.monospace(&info.fingerprint);}else{ui.label("Create your private local identity once. Future signatures reuse it.");ui.label("Display name");ui.text_edit_singleline(&mut self.identity_name);ui.label("Email (optional)");ui.text_edit_singleline(&mut self.identity_email);ui.weak("Protected by your OS credential store. No password setup or online service is required. If secure storage is unavailable, signing will stop.");}
            ui.separator();ui.label("Placement: select a visual signature or annotation to use its position. Otherwise a visible signature box is placed near the bottom of the current page.");ui.label("The output is a new file. The original remains available.");
            if ui.add_enabled(self.identity_info.is_some()||!self.identity_name.trim().is_empty(),egui::Button::new("Sign & save copy…")).clicked(){sign=true;}
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
        let rect = t
            .selected
            .and_then(|id| model.edits.annotations.iter().find(|a| a.id == id))
            .map(|a| a.rect)
            .unwrap_or(Rect {
                left: g.crop.left + 30.,
                bottom: g.crop.bottom + 30.,
                right: (g.crop.left + 300.).min(g.crop.right - 10.),
                top: g.crop.bottom + 100.,
            });
        let appearance = openpdf_signatures::Appearance {
            page: t.page,
            x: rect.left as f64,
            y: rect.bottom as f64,
            width: rect.width() as f64,
            height: rect.height() as f64,
            text: "Digitally signed with OpenPDF\nLocal self-signed identity".into(),
            ..Default::default()
        };
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
                let bytes = OfflineSigner.sign(&input, &identity, Some(appearance))?;
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
