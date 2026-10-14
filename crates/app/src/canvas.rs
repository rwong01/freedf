use super::*;
use egui::{Pos2, Sense, Stroke, StrokeKind};
use openpdf_document::{FieldKind, FieldValue, PageGeometry};

fn screen(g: PageGeometry, p: Point, rotation: u16, origin: Pos2, zoom: f32) -> Pos2 {
    let p = g.to_display(p, rotation);
    origin + egui::vec2(p.x * zoom, p.y * zoom)
}
fn screen_rect(g: PageGeometry, r: Rect, rotation: u16, origin: Pos2, zoom: f32) -> egui::Rect {
    egui::Rect::from_two_pos(
        screen(
            g,
            Point {
                x: r.left,
                y: r.bottom,
            },
            rotation,
            origin,
            zoom,
        ),
        screen(
            g,
            Point {
                x: r.right,
                y: r.top,
            },
            rotation,
            origin,
            zoom,
        ),
    )
}
fn intersects(a: Rect, b: Rect) -> bool {
    a.left <= b.right && a.right >= b.left && a.bottom <= b.top && a.top >= b.bottom
}
fn color(a: [f32; 3], opacity: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (a[0] * 255.) as u8,
        (a[1] * 255.) as u8,
        (a[2] * 255.) as u8,
        (opacity * 255.) as u8,
    )
}

impl App {
    pub(super) fn sidebar_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (i, label) in ["Pages", "Outline", "Comments", "Forms", "Signatures"]
                .iter()
                .enumerate()
            {
                ui.selectable_value(&mut self.sidebar, i, *label);
            }
        });
        ui.separator();
        let mut requests = vec![];
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        let Some(m) = &mut t.model else {
            ui.spinner();
            return;
        };
        match self.sidebar {
            0 => {
                egui::ScrollArea::vertical().show_rows(ui, 175., m.pages.len(), |ui, range| {
                    for page in range {
                        let key = RenderKey {
                            document: t.id,
                            page,
                            width: 120,
                            rotation: 0,
                        };
                        let size = egui::vec2(120., 145.);
                        if let Some(c) = self.cache.get_mut(&key) {
                            c.last_used = self.frame;
                            let response = ui.add(
                                egui::Image::new((c.texture.id(), c.texture.size_vec2()))
                                    .fit_to_exact_size(size)
                                    .sense(Sense::click()),
                            );
                            if response.clicked() {
                                t.jump = Some(page);
                            }
                        } else {
                            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                            ui.painter().rect_filled(rect, 2., Color32::WHITE);
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "…",
                                egui::FontId::proportional(18.),
                                Color32::GRAY,
                            );
                            if response.clicked() {
                                t.jump = Some(page);
                            }
                            requests.push(key);
                        }
                        ui.label(format!("Page {}", page + 1));
                    }
                });
            }
            1 => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if m.bookmarks.is_empty() {
                        ui.weak("No supported page bookmarks.");
                    }
                    for (title, page) in &m.bookmarks {
                        if ui.link(title).clicked() {
                            t.jump = Some(*page);
                        }
                    }
                });
            }
            2 => {
                ui.label("Annotation text");
                ui.text_edit_multiline(&mut self.annotation_text);
                ui.color_edit_button_rgb(&mut self.color);
                ui.add(egui::Slider::new(&mut self.opacity, 0.1..=1.).text("Opacity"));
                ui.add(egui::Slider::new(&mut self.stroke_width, 0.5..=12.).text("Stroke"));
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for a in &m.edits.annotations {
                        if ui
                            .selectable_label(
                                t.selected == Some(a.id),
                                format!(
                                    "p.{} · {:?} {}",
                                    a.page + 1,
                                    a.kind,
                                    a.text.chars().take(28).collect::<String>()
                                ),
                            )
                            .clicked()
                        {
                            t.selected = Some(a.id);
                            t.jump = Some(a.page);
                        }
                    }
                });
                if let Some(id) = t.selected
                    && let Some(a) = m.edits.annotations.iter().find(|a| a.id == id)
                {
                    let mut edited = a.clone();
                    ui.separator();
                    ui.label("Selected annotation");
                    let mut changed = ui.text_edit_multiline(&mut edited.text).changed();
                    changed |= ui.color_edit_button_rgb(&mut edited.color).changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut edited.opacity, 0.1..=1.).text("Opacity"))
                        .changed();
                    let mut w = edited.rect.width();
                    let mut h = edited.rect.height();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut w)
                                .range(5. ..=2000.)
                                .prefix("Width "),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut h)
                                .range(5. ..=2000.)
                                .prefix("Height "),
                        )
                        .changed();
                    let mut rect = edited.rect;
                    rect.right = rect.left + w;
                    rect.top = rect.bottom + h;
                    edited.move_to(rect);
                    if changed && !t.busy {
                        let _ = m.change(|e| {
                            if let Some(a) = e.annotations.iter_mut().find(|a| a.id == id) {
                                *a = edited;
                            }
                        });
                    }
                    if ui
                        .add_enabled(!t.busy, egui::Button::new("Delete annotation"))
                        .clicked()
                    {
                        let _ = m.change(|e| e.annotations.retain(|a| a.id != id));
                        t.selected = None;
                    }
                }
            }
            3 => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if m.fields.is_empty() {
                        ui.label("No AcroForm fields found.");
                    }
                    let fields = m.fields.clone();
                    for f in fields {
                        ui.push_id(f.widget, |ui| {
                            ui.horizontal(|ui| {
                                if ui.small_button(format!("p.{}", f.page + 1)).clicked() {
                                    t.jump = Some(f.page);
                                }
                                ui.label(&f.name);
                            });
                            let mut value = m.edits.fields.get(&f.id).unwrap_or(&f.value).clone();
                            let mut changed = false;
                            ui.add_enabled_ui(m.editable() && !t.busy && !f.readonly, |ui| {
                                changed = field_widget(ui, &f, &mut value, false);
                            });
                            if changed {
                                let _ = m.change(|e| {
                                    e.fields.insert(f.id, value);
                                });
                            }
                            ui.separator();
                        });
                    }
                });
            }
            _ => {
                egui::ScrollArea::vertical().show(ui,|ui|{ui.label(&t.signature_report);ui.separator();ui.weak("Cryptographic integrity and signer identity trust are separate. A local self-signed certificate does not independently prove a person's identity.");});
            }
        }
        for key in requests {
            self.request(key);
        }
    }

    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        let Some(m) = &mut t.model else {
            ui.vertical_centered(|ui| {
                if t.busy {
                    ui.spinner();
                    ui.label("Opening PDF…");
                } else {
                    ui.label("The document could not be opened.");
                    if ui.button("Enter password and retry…").clicked() {
                        self.password_path = Some(t.path.clone());
                    }
                }
            });
            return;
        };
        let avail = ui.available_size();
        let (pw, ph) = m.pages[t.page.min(m.pages.len() - 1)].size(t.rotation);
        match t.fit {
            Fit::Width => t.zoom = ((avail.x - 32.) / pw).clamp(0.1, 5.),
            Fit::Page => {
                t.zoom = ((avail.x - 32.) / pw)
                    .min((avail.y - 32.) / ph)
                    .clamp(0.1, 5.)
            }
            _ => {}
        }
        let zoom = t.zoom;
        let rotation = t.rotation;
        let id = t.id;
        let mut requests = vec![];
        let mut add = None;
        let mut changed_fields = vec![];
        let mut move_annotation = None;
        let mut pages = vec![];
        let mut total = 0f32;
        let mut max_width = avail.x - 20.;
        let target = t.jump.take();
        if t.single
            && let Some(page) = target
        {
            t.page = page;
        }
        for (page, g) in m.pages.iter().enumerate() {
            if t.single && page != t.page {
                continue;
            }
            let (w, h) = g.size(rotation);
            let (w, h) = (w * zoom, h * zoom);
            pages.push((page, *g, total, w, h));
            total += h + 22.;
            max_width = max_width.max(w + 24.);
        }
        let mut scroll = egui::ScrollArea::both()
            .id_salt(("document", id))
            .auto_shrink([false, false])
            .scroll_source(egui::scroll_area::ScrollSource {
                drag: if self.tool == Tool::Hand {
                    egui::scroll_area::DragScroll::Always
                } else {
                    egui::scroll_area::DragScroll::Never
                },
                ..Default::default()
            });
        if let Some(target) = target
            && let Some(p) = pages.iter().find(|p| p.0 == target)
        {
            scroll = scroll.vertical_scroll_offset(p.2);
        }
        let busy = t.busy;
        let editable = m.editable() && !busy;
        let dpi = ui.ctx().pixels_per_point();
        scroll.show_viewport(ui, |ui, viewport| {
            ui.set_min_size(egui::vec2(max_width, total));
            let base = ui.min_rect().min;
            let mut most_visible = 0.;
            for (page, g, y, w, h) in pages {
                if y + h < viewport.min.y - 50. || y > viewport.max.y + 50. {
                    continue;
                }
                // Prefetch margins are not visible content. Select the page with
                // the largest actual viewport intersection for navigation/signing.
                let visible = (y + h).min(viewport.max.y) - y.max(viewport.min.y);
                if visible > most_visible {
                    t.page = page;
                    most_visible = visible;
                }
                let rect = egui::Rect::from_min_size(
                    base + egui::vec2((max_width - w) / 2., y),
                    egui::vec2(w, h),
                );
                ui.painter().rect_filled(
                    rect.translate(egui::vec2(3., 4.)),
                    1.,
                    Color32::from_black_alpha(35),
                );
                ui.painter().rect_filled(rect, 0., Color32::WHITE);
                let width = ((w * dpi).ceil() as u32).clamp(64, 4096);
                let key = RenderKey {
                    document: id,
                    page,
                    width,
                    rotation,
                };
                let glyphs = if let Some(c) = self.cache.get_mut(&key) {
                    c.last_used = self.frame;
                    ui.painter().image(
                        c.texture.id(),
                        rect,
                        egui::Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                        Color32::WHITE,
                    );
                    c.glyphs.clone()
                } else {
                    requests.push(key);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Rendering…",
                        egui::FontId::proportional(16.),
                        Color32::GRAY,
                    );
                    vec![]
                };
                let response =
                    ui.interact(rect, ui.id().with(("page", page)), Sense::click_and_drag());
                for (p, r) in &t.matches {
                    if *p == page {
                        ui.painter().rect_filled(
                            screen_rect(g, *r, rotation, rect.min, zoom),
                            0.,
                            Color32::from_rgba_unmultiplied(255, 175, 0, 95),
                        );
                    }
                }
                for (p, r, _) in &t.selection {
                    if *p == page {
                        ui.painter().rect_filled(
                            screen_rect(g, *r, rotation, rect.min, zoom),
                            0.,
                            Color32::from_rgba_unmultiplied(45, 130, 245, 80),
                        );
                    }
                }
                if m.editable() {
                    for a in &m.edits.annotations {
                        if a.page == page {
                            paint_annotation(ui, a, g, rotation, rect.min, zoom);
                            if t.selected == Some(a.id) {
                                ui.painter().rect_stroke(
                                    screen_rect(g, a.rect, rotation, rect.min, zoom).expand(3.),
                                    0.,
                                    Stroke::new(1.5, Color32::from_rgb(30, 110, 230)),
                                    StrokeKind::Outside,
                                );
                            }
                        }
                    }
                }
                let point = response
                    .interact_pointer_pos()
                    .or_else(|| ui.ctx().pointer_hover_pos().filter(|p| rect.contains(*p)))
                    .map(|p| {
                        g.from_display(
                            Point {
                                x: (p.x - rect.left()) / zoom,
                                y: (p.y - rect.top()) / zoom,
                            },
                            rotation,
                        )
                    });
                if let Some(p) = point {
                    if response.drag_started() {
                        let anchor = ui
                            .ctx()
                            .input(|i| i.pointer.press_origin())
                            .map(|position| {
                                g.from_display(
                                    Point {
                                        x: (position.x - rect.left()) / zoom,
                                        y: (position.y - rect.top()) / zoom,
                                    },
                                    rotation,
                                )
                            })
                            .unwrap_or(p);
                        self.drag_start = Some(anchor);
                        self.stroke = vec![anchor];
                        if self.tool == Tool::Select {
                            t.selected = m
                                .edits
                                .annotations
                                .iter()
                                .rev()
                                .find(|a| a.page == page && a.rect.contains(anchor))
                                .map(|a| a.id);
                        }
                    }
                    if response.dragged() && self.tool == Tool::Ink && editable {
                        self.stroke.push(p);
                    }
                    if response.dragged()
                        && matches!(
                            self.tool,
                            Tool::Select | Tool::Highlight | Tool::Underline | Tool::StrikeOut
                        )
                        && t.selected.is_none()
                        && let Some(start) = self.drag_start
                    {
                        let region = Rect::new(start, p);
                        t.selection = glyphs
                            .iter()
                            .filter(|g| intersects(g.rect, region))
                            .map(|g| (page, g.rect, g.text.clone()))
                            .collect();
                    }
                    if response.dragged() && self.tool == Tool::Ink {
                        for pair in self.stroke.windows(2) {
                            ui.painter().line_segment(
                                [
                                    screen(g, pair[0], rotation, rect.min, zoom),
                                    screen(g, pair[1], rotation, rect.min, zoom),
                                ],
                                Stroke::new(
                                    self.stroke_width * zoom,
                                    color(self.color, self.opacity),
                                ),
                            );
                        }
                    }
                    if response.clicked() && self.tool == Tool::Select {
                        t.selected = m
                            .edits
                            .annotations
                            .iter()
                            .rev()
                            .find(|a| a.page == page && a.rect.contains(p))
                            .map(|a| a.id);
                        if t.selected.is_some() {
                            self.sidebar = 2;
                        }
                    }
                    if response.clicked() && editable {
                        if self.tool == Tool::Signature
                            && let Some(mut a) = self.placement.clone()
                        {
                            a.id = self.next_id;
                            self.next_id += 1;
                            a.page = page;
                            let r = Rect {
                                left: p.x,
                                bottom: p.y,
                                right: p.x + a.rect.width(),
                                top: p.y + a.rect.height(),
                            };
                            a.move_to(r);
                            add = Some(a);
                            self.tool = Tool::Select;
                        } else if matches!(self.tool, Tool::Note | Tool::Text) {
                            let (w, h) = if self.tool == Tool::Note {
                                (22., 22.)
                            } else {
                                (200., 40.)
                            };
                            let mut a = Annotation::new(
                                self.next_id,
                                page,
                                if self.tool == Tool::Note {
                                    AnnotationKind::Note
                                } else {
                                    AnnotationKind::FreeText
                                },
                                Rect {
                                    left: p.x,
                                    bottom: p.y,
                                    right: p.x + w,
                                    top: p.y + h,
                                },
                            );
                            self.next_id += 1;
                            a.text = self.annotation_text.clone();
                            a.color = self.color;
                            add = Some(a);
                            self.sidebar = 2;
                        }
                    }
                    if response.drag_stopped()
                        && let Some(start) = self.drag_start.take()
                    {
                        let region = Rect::new(start, p);
                        if editable
                            && self.tool == Tool::Select
                            && let Some(selected) = t.selected
                            && let Some(a) = m.edits.annotations.iter().find(|a| a.id == selected)
                        {
                            let dx = p.x - start.x;
                            let dy = p.y - start.y;
                            let mut moved = a.clone();
                            moved.move_to(Rect {
                                left: a.rect.left + dx,
                                right: a.rect.right + dx,
                                bottom: a.rect.bottom + dy,
                                top: a.rect.top + dy,
                            });
                            move_annotation = Some(moved);
                        } else if editable && region.width() > 1. && region.height() > 1. {
                            let kind = match self.tool {
                                Tool::Highlight => Some(AnnotationKind::Highlight),
                                Tool::Underline => Some(AnnotationKind::Underline),
                                Tool::StrikeOut => Some(AnnotationKind::StrikeOut),
                                Tool::Ink => Some(AnnotationKind::Ink),
                                Tool::Rectangle => Some(AnnotationKind::Rectangle),
                                Tool::Ellipse => Some(AnnotationKind::Ellipse),
                                Tool::Line => Some(AnnotationKind::Line),
                                _ => None,
                            };
                            if let Some(kind) = kind {
                                let mut a = Annotation::new(self.next_id, page, kind, region);
                                self.next_id += 1;
                                a.color = self.color;
                                a.opacity = self.opacity;
                                a.width = self.stroke_width;
                                if kind == AnnotationKind::Line {
                                    a.strokes = vec![vec![start, p]];
                                }
                                a.text = self.annotation_text.clone();
                                if matches!(
                                    kind,
                                    AnnotationKind::Highlight
                                        | AnnotationKind::Underline
                                        | AnnotationKind::StrikeOut
                                ) {
                                    a.quads = t.selection.iter().map(|(_, r, _)| *r).collect();
                                    if kind == AnnotationKind::Highlight {
                                        a.color = [1., 0.85, 0.];
                                        a.opacity = 0.35;
                                    }
                                    if !a.quads.is_empty() {
                                        let mut r = a.quads[0];
                                        for q in &a.quads {
                                            r.left = r.left.min(q.left);
                                            r.right = r.right.max(q.right);
                                            r.bottom = r.bottom.min(q.bottom);
                                            r.top = r.top.max(q.top);
                                        }
                                        a.rect = r;
                                    }
                                }
                                if kind == AnnotationKind::Ink {
                                    a.strokes = vec![self.stroke.clone()];
                                    let mut r = region;
                                    for p in &self.stroke {
                                        r.left = r.left.min(p.x - 2.);
                                        r.right = r.right.max(p.x + 2.);
                                        r.bottom = r.bottom.min(p.y - 2.);
                                        r.top = r.top.max(p.y + 2.);
                                    }
                                    a.rect = r;
                                }
                                if !matches!(
                                    kind,
                                    AnnotationKind::Highlight
                                        | AnnotationKind::Underline
                                        | AnnotationKind::StrikeOut
                                ) || !a.quads.is_empty()
                                {
                                    add = Some(a);
                                    t.selection.clear();
                                }
                            }
                        }
                    }
                    if self.tool == Tool::Signature
                        && let Some(mut a) = self.placement.clone()
                    {
                        a.move_to(Rect {
                            left: p.x,
                            bottom: p.y,
                            right: p.x + a.rect.width(),
                            top: p.y + a.rect.height(),
                        });
                        paint_annotation(ui, &a, g, rotation, rect.min, zoom);
                    }
                }
                response.context_menu(|ui| {
                    if ui.button("Copy selected text").clicked() {
                        ui.ctx()
                            .copy_text(t.selection.iter().map(|(_, _, s)| s.as_str()).collect());
                        ui.close();
                    }
                    if ui.button("Form fields").clicked() {
                        self.sidebar = 3;
                        ui.close();
                    }
                    if ui.button("Signature details").clicked() {
                        self.sidebar = 4;
                        ui.close();
                    }
                });
                if editable && (g.rotation + rotation) % 360 == 0 {
                    for f in &m.fields {
                        if f.page != page || f.readonly || matches!(f.kind, FieldKind::Signature) {
                            continue;
                        }
                        let field_rect = screen_rect(g, f.rect, rotation, rect.min, zoom);
                        let mut value = m.edits.fields.get(&f.id).unwrap_or(&f.value).clone();
                        let mut changed = false;
                        ui.scope_builder(egui::UiBuilder::new().max_rect(field_rect), |ui| {
                            ui.set_clip_rect(field_rect);
                            // Editable controls replace the saved widget appearance on screen.
                            // Cover the entire widget so old values never show behind pending input.
                            ui.painter().rect_filled(field_rect, 0., Color32::WHITE);
                            ui.painter().rect_stroke(
                                field_rect,
                                0.,
                                Stroke::new(1., Color32::LIGHT_GRAY),
                                StrokeKind::Inside,
                            );
                            ui.style_mut().override_font_id =
                                Some(egui::FontId::proportional((12. * zoom).clamp(9., 72.)));
                            ui.spacing_mut().interact_size.y = field_rect.height().min(24. * zoom);
                            ui.add_enabled_ui(!f.readonly, |ui| {
                                changed = field_widget(ui, f, &mut value, true);
                            });
                        });
                        if changed {
                            changed_fields.push((f.id, value));
                        }
                    }
                }
            }
        });
        if let Some(a) = move_annotation {
            let _ = m.change(|e| {
                if let Some(old) = e.annotations.iter_mut().find(|old| old.id == a.id) {
                    *old = a;
                }
            });
        }
        for (id, value) in changed_fields {
            let _ = m.change(|e| {
                e.fields.insert(id, value);
            });
        }
        if let Some(a) = add {
            self.add_annotation(a);
        }
        for key in requests {
            self.request(key);
        }
    }
}

fn field_widget(
    ui: &mut egui::Ui,
    f: &openpdf_document::Field,
    value: &mut FieldValue,
    overlay: bool,
) -> bool {
    match (&f.kind, value) {
        (FieldKind::Text { multiline }, FieldValue::Text(s)) => {
            let size = if overlay {
                ui.available_size()
            } else {
                egui::vec2(ui.available_width(), if *multiline { 48. } else { 24. })
            };
            if *multiline {
                ui.add_sized(
                    size,
                    egui::TextEdit::multiline(s)
                        .desired_width(f32::INFINITY)
                        .desired_rows(2),
                )
                .changed()
            } else {
                ui.add_sized(
                    size,
                    egui::TextEdit::singleline(s)
                        .desired_width(f32::INFINITY)
                        .hint_text(&f.name),
                )
                .changed()
            }
        }
        (FieldKind::Checkbox, FieldValue::Button(s)) => {
            let mut checked = s == &f.on_value;
            let changed = ui
                .checkbox(&mut checked, "")
                .on_hover_text(&f.name)
                .changed();
            if changed {
                *s = if checked {
                    f.on_value.clone()
                } else {
                    "Off".into()
                };
            }
            changed
        }
        (FieldKind::Radio, FieldValue::Button(s)) => {
            if ui
                .radio(s == &f.on_value, if overlay { "" } else { &f.on_value })
                .on_hover_text(&f.name)
                .clicked()
            {
                *s = f.on_value.clone();
                true
            } else {
                false
            }
        }
        (FieldKind::Combo | FieldKind::List { .. }, FieldValue::Choice(v)) => {
            let mut changed = false;
            egui::ComboBox::from_id_salt(f.widget)
                .selected_text(v.join(", "))
                .show_ui(ui, |ui| {
                    for (export, label) in &f.options {
                        if matches!(f.kind, FieldKind::List { multiple: true }) {
                            let mut selected = v.contains(export);
                            if ui.checkbox(&mut selected, label).changed() {
                                v.retain(|s| s != export);
                                if selected {
                                    v.push(export.clone());
                                }
                                changed = true;
                            }
                        } else if ui.selectable_label(v.contains(export), label).clicked() {
                            *v = vec![export.clone()];
                            changed = true;
                        }
                    }
                });
            changed
        }
        (FieldKind::Signature, _) => {
            ui.weak("Existing signature field: inspect Signatures tab");
            false
        }
        _ => {
            ui.weak("Unsupported field");
            false
        }
    }
}

fn paint_annotation(
    ui: &egui::Ui,
    a: &Annotation,
    g: PageGeometry,
    rotation: u16,
    origin: Pos2,
    zoom: f32,
) {
    let r = screen_rect(g, a.rect, rotation, origin, zoom);
    let c = color(a.color, a.opacity);
    let stroke = Stroke::new(a.width * zoom, c);
    let painter = ui.painter();
    match a.kind {
        AnnotationKind::Highlight | AnnotationKind::Underline | AnnotationKind::StrikeOut => {
            let quads = if a.quads.is_empty() {
                vec![a.rect]
            } else {
                a.quads.clone()
            };
            for q in quads {
                let r = screen_rect(g, q, rotation, origin, zoom);
                if a.kind == AnnotationKind::Highlight {
                    painter.rect_filled(r, 0., c);
                } else {
                    let y = if a.kind == AnnotationKind::Underline {
                        r.bottom()
                    } else {
                        r.center().y
                    };
                    painter
                        .line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], stroke);
                }
            }
        }
        AnnotationKind::Ink => {
            for s in &a.strokes {
                for p in s.windows(2) {
                    painter.line_segment(
                        [
                            screen(g, p[0], rotation, origin, zoom),
                            screen(g, p[1], rotation, origin, zoom),
                        ],
                        stroke,
                    );
                }
            }
        }
        AnnotationKind::Rectangle => {
            painter.rect_stroke(r, 0., stroke, StrokeKind::Inside);
        }
        AnnotationKind::Ellipse => {
            painter.add(egui::Shape::ellipse_stroke(
                r.center(),
                r.size() / 2.,
                stroke,
            ));
        }
        AnnotationKind::Line => {
            let [a, b] = a.line_points();
            painter.line_segment(
                [
                    screen(g, a, rotation, origin, zoom),
                    screen(g, b, rotation, origin, zoom),
                ],
                stroke,
            );
        }
        AnnotationKind::Note => {
            painter.rect_filled(r, 2., c);
            painter.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "…",
                egui::FontId::proportional(14. * zoom),
                Color32::WHITE,
            );
        }
        AnnotationKind::FreeText => {
            painter.text(
                r.left_top() + egui::vec2(3., 2.),
                egui::Align2::LEFT_TOP,
                &a.text,
                egui::FontId::proportional((a.rect.height() * 0.6).min(24.) * zoom),
                c,
            );
        }
        AnnotationKind::Image => {
            if let Some(bytes) = &a.image {
                let id = egui::Id::new(("signature-image", a.id, bytes.len()));
                let texture = ui.ctx().data_mut(|d| d.get_temp::<TextureHandle>(id));
                let texture = texture.or_else(|| {
                    let im = image::load_from_memory(bytes).ok()?.to_rgba8();
                    let texture = ui.ctx().load_texture(
                        "signature-image",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [im.width() as usize, im.height() as usize],
                            im.as_raw(),
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    ui.ctx().data_mut(|d| d.insert_temp(id, texture.clone()));
                    Some(texture)
                });
                if let Some(texture) = texture {
                    painter.image(
                        texture.id(),
                        r,
                        egui::Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                        Color32::WHITE,
                    );
                }
            }
        }
    }
}
