use super::*;
use self_devkits::business::pdf::{
    self as pdf_tools, ImageMode, PageImage, PdfDocument, PreparedImages, ReviewedImage,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
};

#[derive(Clone, Copy, PartialEq)]
enum View {
    Images,
    Regions,
    Text,
}
enum Completed {
    Imported(Arc<PdfDocument>),
    Text(String),
    Exported(PathBuf, usize),
    Images(Arc<PreparedImages>, Result<image::RgbaImage, String>),
    Regions(Vec<[u32; 4]>),
    Preview(usize, image::RgbaImage),
    Page(PageImage, image::RgbaImage),
    Languages(Vec<String>),
}
enum Update {
    Progress(usize, usize),
    Done(Result<Completed, String>),
}

pub struct PdfEditor {
    document: Option<Arc<PdfDocument>>,
    text: String,
    view: View,
    staged: Option<Arc<PreparedImages>>,
    items: Vec<ReviewedImage>,
    included: Vec<bool>,
    selected: Option<usize>,
    preview: Option<egui::TextureHandle>,
    page: usize,
    pixels: u32,
    page_image: Option<PageImage>,
    page_texture: Option<egui::TextureHandle>,
    region: Option<egui::Rect>,
    anchor: Option<egui::Pos2>,
    region_name: String,
    suggestions: Vec<[u32; 4]>,
    ocr_first: usize,
    ocr_last: usize,
    language: String,
    languages: Vec<String>,
    job: Option<mpsc::Receiver<Update>>,
    worker: Option<std::thread::JoinHandle<()>>,
    cancel: Arc<AtomicBool>,
    progress: Option<(usize, usize)>,
    message: String,
    error: bool,
    output: Option<PathBuf>,
}
impl Default for PdfEditor {
    fn default() -> Self {
        Self {
            document: None,
            text: String::new(),
            view: View::Images,
            staged: None,
            items: Vec::new(),
            included: Vec::new(),
            selected: None,
            preview: None,
            page: 1,
            pixels: 2400,
            page_image: None,
            page_texture: None,
            region: None,
            anchor: None,
            region_name: String::new(),
            suggestions: Vec::new(),
            ocr_first: 1,
            ocr_last: 1,
            language: "eng".into(),
            languages: Vec::new(),
            job: None,
            worker: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: None,
            message: String::new(),
            error: false,
            output: None,
        }
    }
}
impl Drop for PdfEditor {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn texture(ctx: &egui::Context, image: image::RgbaImage) -> egui::TextureHandle {
    ctx.load_texture(
        "pdf-preview",
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}
fn progress(tx: &mpsc::Sender<Update>, ctx: &egui::Context, done: usize, total: usize) {
    let _ = tx.send(Update::Progress(done, total));
    ctx.request_repaint();
}
impl PdfEditor {
    #[cfg(debug_assertions)]
    pub fn preview_file(&mut self, path: &std::path::Path, ctx: &egui::Context) {
        let result = pdf_tools::import(path, &self.cancel).and_then(|doc| {
            self.accept(Completed::Imported(doc.clone()), ctx);
            match std::env::var("DEVKITS_CAPTURE_PDF_VIEW").as_deref() {
                Ok("regions") => {
                    self.view = View::Regions;
                    let page =
                        pdf_tools::render_page(&doc, 3.min(doc.pages), self.pixels, &self.cancel)?;
                    let image = pdf_tools::image_preview(&ReviewedImage {
                        source: page.source.clone(),
                        page: page.page,
                        name: String::new(),
                        caption: None,
                        crop: None,
                    })?;
                    let regions = pdf_tools::suggest_regions(&page, &self.cancel)?;
                    eprintln!("Suggested regions on page {}: {regions:?}", page.page);
                    self.accept(Completed::Page(page, image), ctx);
                    self.accept(Completed::Regions(regions), ctx);
                }
                Ok("images") => {
                    let batch = Arc::new(pdf_tools::prepare_images(&doc, &self.cancel, |_, _| {})?);
                    let image = pdf_tools::image_preview(&batch.items[0]);
                    self.accept(Completed::Images(batch, image), ctx);
                }
                _ => {
                    self.view = View::Text;
                    self.text = pdf_tools::read_text(&doc, &self.cancel)?;
                }
            }
            Ok(())
        });
        if let Err(e) = result {
            self.message = e;
            self.error = true;
        }
    }
    fn start(
        &mut self,
        ctx: &egui::Context,
        message: &str,
        task: impl FnOnce(&AtomicBool, &mpsc::Sender<Update>, &egui::Context) -> Result<Completed, String>
            + Send
            + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let ctx = ctx.clone();
        self.job = Some(rx);
        self.progress = None;
        self.message = message.into();
        self.error = false;
        self.worker = Some(std::thread::spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task(&cancel, &tx, &ctx)))
                    .unwrap_or_else(|_| {
                        Err("PDF worker stopped unexpectedly. Please retry.".into())
                    });
            let _ = tx.send(Update::Done(result));
            ctx.request_repaint();
        }));
    }
    fn accept(&mut self, result: Completed, ctx: &egui::Context) {
        match result {
            Completed::Imported(doc) => {
                self.ocr_last = doc.pages;
                self.document = Some(doc);
                self.text.clear();
                self.output = None;
                self.staged = None;
                self.items.clear();
                self.included.clear();
                self.selected = None;
                self.preview = None;
                self.page = 1;
                self.ocr_first = 1;
                self.page_image = None;
                self.page_texture = None;
                self.region = None;
                self.anchor = None;
                self.region_name.clear();
                self.suggestions.clear();
                self.message = "Choose Images to review embedded photos, Page regions for scans/collages, or Text for extraction/OCR.".into();
            }
            Completed::Text(text) => {
                self.message = if text.trim().is_empty() {
                    "No text layer found. Try OCR for scanned pages."
                } else {
                    "Text ready. Check reading order and OCR accuracy; you can edit before saving."
                }
                .into();
                self.text = text;
            }
            Completed::Images(batch, image) => {
                let first = self.items.len();
                self.items.extend(batch.items.clone());
                self.included.resize(self.items.len(), true);
                self.selected = Some(first);
                self.preview = image.as_ref().ok().map(|img| texture(ctx, img.clone()));
                self.message = batch.warning.clone().unwrap_or_else(|| {
                    "Review suggested names and uncheck unwanted images before exporting.".into()
                });
                if let Err(error) = image {
                    self.message.push_str(&format!(" First preview unavailable: {error}. Other images can still be reviewed/exported."));
                }
                self.staged = Some(batch);
            }
            Completed::Regions(regions) => {
                self.message = if regions.is_empty() {
                    "No separate regions found. Draw a rectangle manually; touching images and busy backgrounds may not separate.".into()
                } else {
                    format!("{} suggested regions. Click a numbered box to select and name it, or add all then review. Suggestions may include text or miss photos.", regions.len())
                };
                self.suggestions = regions;
            }
            Completed::Preview(index, image) => {
                self.selected = Some(index);
                self.preview = Some(texture(ctx, image));
                self.message.clear();
            }
            Completed::Page(page, image) => {
                self.page = page.page;
                self.page_image = Some(page);
                self.suggestions.clear();
                self.page_texture = Some(texture(ctx, image));
                self.region = None;
                self.anchor = None;
                self.message = "Drag a rectangle around one photo, enter its name, then Add region. Repeat for other photos.".into();
            }
            Completed::Languages(languages) => {
                if !languages.iter().any(|l| l == &self.language) {
                    self.language = languages[0].clone();
                }
                self.message = format!("Installed OCR languages: {}", languages.join(", "));
                self.languages = languages;
            }
            Completed::Exported(path, count) => {
                self.message = format!("Exported {count} PNG files and manifest.json.");
                self.output = Some(path);
            }
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        let updates: Vec<_> = self
            .job
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for update in updates {
            match update {
                Update::Progress(done, total) => self.progress = Some((done, total)),
                Update::Done(result) => {
                    self.job = None;
                    if let Some(worker) = self.worker.take() {
                        let _ = worker.join();
                    }
                    self.progress = None;
                    match result {
                        Ok(result) => self.accept(result, ctx),
                        Err(e) => {
                            self.error = e != "Cancelled.";
                            self.message = e;
                        }
                    }
                }
            }
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.poll(ui.ctx());
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.job.is_none(),
                    egui::Button::new(format!("{} Import PDF", icons::FOLDER_OPEN)),
                )
                .clicked()
            {
                if let Some(path) = desktop::open_pdf() {
                    self.start(ui.ctx(), "Importing…", move |c, _, _| {
                        pdf_tools::import(&path, c).map(Completed::Imported)
                    });
                }
            }
            if let Some(doc) = &self.document {
                ui.label(format!("{} · {} pages", doc.name, doc.pages));
            }
            if self.job.is_some() {
                ui.spinner();
                if ui
                    .add_enabled(
                        !self.cancel.load(Ordering::Relaxed),
                        design::button("Cancel", design::colors(ui).secondary),
                    )
                    .clicked()
                {
                    self.cancel.store(true, Ordering::Relaxed);
                }
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
        });
        ui.add_space(8.);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, View::Images, "Images & filenames");
            ui.selectable_value(&mut self.view, View::Regions, "Page regions");
            ui.selectable_value(&mut self.view, View::Text, "Text & OCR");
        });
        if let Some((done, total)) = self.progress {
            ui.add(
                egui::ProgressBar::new(done as f32 / total as f32)
                    .text(format!("{done} / {total}")),
            );
        }
        if !self.message.is_empty() {
            design::notice(
                ui,
                if self.error {
                    design::colors(ui).danger
                } else {
                    design::colors(ui).info
                },
                &self.message,
            );
        }
        if let Some(path) = &self.output {
            ui.horizontal_wrapped(|ui| {
                ui.label(path.display().to_string());
                if ui.small_button("Copy path").clicked() {
                    ui.ctx().copy_text(path.display().to_string());
                }
            });
        }
        ui.separator();
        if self.document.is_none() {
            ui.heading("Images, scanned pages and text");
            ui.label("Import a PDF to preview and rename photos, select regions from scans or collages, and extract text with optional offline OCR.");
            ui.colored_label(
                crate::ui::design::colors(ui).muted,
                "Poppler required. OCR additionally uses Tesseract. No files are uploaded.",
            );
            return;
        }
        match self.view {
            View::Images => self.images_ui(ui),
            View::Regions => self.regions_ui(ui),
            View::Text => self.text_ui(ui),
        }
    }
    fn images_ui(&mut self, ui: &mut egui::Ui) {
        let busy = self.job.is_some();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !busy && self.staged.is_none(),
                    egui::Button::new("Find embedded images"),
                )
                .clicked()
            {
                let doc = self.document.as_ref().unwrap().clone();
                self.start(
                    ui.ctx(),
                    "Finding images and captions…",
                    move |c, tx, ctx| {
                        let batch = Arc::new(pdf_tools::prepare_images(&doc, c, |d, t| {
                            progress(tx, ctx, d, t)
                        })?);
                        let image = pdf_tools::image_preview(&batch.items[0]);
                        Ok(Completed::Images(batch, image))
                    },
                );
            }
            let count = self.included.iter().filter(|b| **b).count();
            if ui
                .add_enabled(
                    self.job.is_none() && count > 0,
                    design::button(
                        format!("Export selected ({count})…"),
                        design::colors(ui).primary,
                    ),
                )
                .clicked()
            {
                if let Some(parent) = desktop::folder() {
                    let items: Vec<_> = self
                        .items
                        .iter()
                        .zip(&self.included)
                        .filter(|(_, enabled)| **enabled)
                        .map(|(item, _)| item.clone())
                        .collect();
                    let staged = self.staged.clone();
                    let doc = self.document.clone();
                    self.start(
                        ui.ctx(),
                        "Exporting reviewed images…",
                        move |c, tx, ctx| {
                            let _keep_alive = (staged, doc);
                            pdf_tools::export_reviewed(&items, &parent, c, |d, t| {
                                progress(tx, ctx, d, t)
                            })
                            .map(|(p, n)| Completed::Exported(p, n))
                        },
                    );
                }
            }
            if ui.add_enabled(!busy, egui::Button::new("All")).clicked() {
                self.included.fill(true);
            }
            if ui.add_enabled(!busy, egui::Button::new("None")).clicked() {
                self.included.fill(false);
            }
            if ui
                .add_enabled(
                    !busy && self.selected.is_some(),
                    egui::Button::new("Remove selected entry"),
                )
                .clicked()
            {
                if let Some(index) = self.selected.take() {
                    self.items.remove(index);
                    self.included.remove(index);
                    self.preview = None;
                }
            }
        });
        if self.items.is_empty() {
            ui.label("Find embedded images, or use Page regions to select photos from scans, collages and vector pages.");
            return;
        }
        ui.colored_label(crate::ui::design::colors(ui).muted, "Names are suggestions. Edit them below; accents are removed and duplicate names get a numeric suffix on export.");
        let height = ui.available_height().max(100.);
        let mut picked = None;
        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .id_salt("pdf-review-list")
                .max_height(height)
                .show(&mut columns[0], |ui| {
                    ui.add_enabled_ui(self.job.is_none(), |ui| {
                        for (i, item) in self.items.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.included[i], "");
                                if ui
                                    .selectable_label(
                                        self.selected == Some(i),
                                        format!(
                                            "{} · Page {}{}",
                                            i + 1,
                                            item.page,
                                            if item.crop.is_some() {
                                                " · Region"
                                            } else {
                                                ""
                                            }
                                        ),
                                    )
                                    .clicked()
                                {
                                    picked = Some(i);
                                }
                            });
                            ui.add(
                                egui::TextEdit::singleline(&mut item.name)
                                    .desired_width(f32::INFINITY),
                            );
                            ui.colored_label(
                                crate::ui::design::colors(ui).muted,
                                item.caption.as_deref().unwrap_or(if item.crop.is_some() {
                                    "Selected page region"
                                } else {
                                    "No unique caption — check the name"
                                }),
                            );
                            ui.separator();
                        }
                    });
                });
            if let Some(preview) = &self.preview {
                columns[1].add(
                    egui::Image::new(preview)
                        .max_size(egui::vec2(columns[1].available_width(), height))
                        .maintain_aspect_ratio(true),
                );
            }
        });
        if let Some(index) = picked {
            self.selected = Some(index);
            self.preview = None;
            let item = self.items[index].clone();
            self.start(ui.ctx(), "Loading preview…", move |_, _, _| {
                pdf_tools::image_preview(&item).map(|img| Completed::Preview(index, img))
            });
        }
    }
    fn regions_ui(&mut self, ui: &mut egui::Ui) {
        let enabled = self.job.is_none();
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Page");
                ui.add(
                    egui::DragValue::new(&mut self.page)
                        .range(1..=self.document.as_ref().unwrap().pages),
                );
                egui::ComboBox::from_id_salt("pdf-region-size")
                    .selected_text(format!("{} px", self.pixels))
                    .show_ui(ui, |ui| {
                        for n in [1200, 2400, 3200, 4000] {
                            ui.selectable_value(&mut self.pixels, n, format!("{n} px"));
                        }
                    });
                if ui.button("Load page").clicked() {
                    let doc = self.document.as_ref().unwrap().clone();
                    let (page, pixels) = (self.page, self.pixels);
                    self.start(ui.ctx(), "Rendering page…", move |c, _, _| {
                        let page = pdf_tools::render_page(&doc, page, pixels, c)?;
                        let image = pdf_tools::image_preview(&ReviewedImage {
                            source: page.source.clone(),
                            page: page.page,
                            name: String::new(),
                            caption: None,
                            crop: None,
                        })?;
                        Ok(Completed::Page(page, image))
                    });
                }
                if ui
                    .add_enabled(self.job.is_none(), egui::Button::new("Export all pages…"))
                    .clicked()
                {
                    if let Some(parent) = desktop::folder() {
                        let doc = self.document.as_ref().unwrap().clone();
                        let pixels = self.pixels;
                        self.start(ui.ctx(), "Exporting pages…", move |c, tx, ctx| {
                            pdf_tools::export_images(
                                &doc,
                                &parent,
                                ImageMode::Pages,
                                pixels,
                                c,
                                |d, t| progress(tx, ctx, d, t),
                            )
                            .map(|(p, n)| Completed::Exported(p, n))
                        });
                    }
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(self.job.is_none() && self.page_image.is_some(), egui::Button::new("Suggest image regions")).clicked() {
                let page = self.page_image.as_ref().unwrap();
                let page = PageImage { source: page.source.clone(), page: page.page, width: page.width, height: page.height };
                let doc = self.document.clone();
                self.start(ui.ctx(), "Finding separate regions…", move |c, _, _| {
                    let _keep_alive = doc;
                    pdf_tools::suggest_regions(&page, c).map(Completed::Regions)
                });
            }
            if ui.add_enabled(self.job.is_none() && !self.suggestions.is_empty(), egui::Button::new("Add all suggestions")).clicked() {
                if let Some(page) = &self.page_image {
                    let mut count = 0;
                    for (i, crop) in self.suggestions.iter().enumerate() {
                        if self.items.iter().any(|item| item.source == page.source && item.crop == Some(*crop)) { continue; }
                        self.items.push(ReviewedImage { source: page.source.clone(), page: page.page, name: format!("page_{:04}_region_{:03}",page.page,i+1), caption: None, crop: Some(*crop) });
                        self.included.push(true);
                        count += 1;
                    }
                    self.message = format!("Added {count} regions. Open Images & filenames to check crops, edit names and remove unwanted entries.");
                }
            }
            ui.colored_label(crate::ui::design::colors(ui).muted, "Best for separate photos on a plain background.");
        });
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled(self.job.is_none(),egui::TextEdit::singleline(&mut self.region_name).hint_text("Image name, e.g. sup_bi_do").desired_width(230.));
            if ui.add_enabled(self.job.is_none() && self.region.is_some() && !self.region_name.trim().is_empty(),egui::Button::new("Add region")).clicked() {
                if let (Some(page),Some(rect))=(&self.page_image,self.region) {
                    if let Some(crop)=pixel_crop(rect,page.width,page.height) {
                        self.items.push(ReviewedImage { source:page.source.clone(),page:page.page,name:self.region_name.trim().into(),caption:None,crop:Some(crop) });
                        self.included.push(true); self.region=None; self.region_name.clear();
                        self.message="Region added. Select another region or open Images & filenames to review/export.".into();
                    }
                }
            }
            if ui.button("Clear selection").clicked() { self.region=None; self.anchor=None; }
        });
        if let (Some(texture), Some(page)) = (&self.page_texture, &self.page_image) {
            ui.colored_label(
                crate::ui::design::colors(ui).muted,
                format!(
                    "Viewing page {} · {} × {} px. Drag around the photo only.",
                    page.page, page.width, page.height
                ),
            );
            let available = egui::vec2(ui.available_width(), ui.available_height().max(1.));
            let size = texture.size_vec2();
            let scale = (available.x / size.x).min(available.y / size.y).min(1.);
            let response = ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(size * scale)
                    .sense(if enabled {
                        egui::Sense::click_and_drag()
                    } else {
                        egui::Sense::hover()
                    }),
            );
            let normalized = |pos: egui::Pos2| {
                egui::pos2(
                    ((pos.x - response.rect.left()) / response.rect.width()).clamp(0., 1.),
                    ((pos.y - response.rect.top()) / response.rect.height()).clamp(0., 1.),
                )
            };
            for (i, crop) in self.suggestions.iter().enumerate() {
                let a = egui::pos2(
                    crop[0] as f32 / page.width as f32,
                    crop[1] as f32 / page.height as f32,
                );
                let b = egui::pos2(
                    (crop[0] + crop[2]) as f32 / page.width as f32,
                    (crop[1] + crop[3]) as f32 / page.height as f32,
                );
                let rect = egui::Rect::from_min_max(
                    response.rect.min + a.to_vec2() * response.rect.size(),
                    response.rect.min + b.to_vec2() * response.rect.size(),
                );
                ui.painter().rect_stroke(
                    rect,
                    0.,
                    egui::Stroke::new(1.5_f32, design::colors(ui).success.text),
                    egui::StrokeKind::Inside,
                );
                let badge = egui::Rect::from_min_size(rect.min, egui::vec2(25., 22.));
                ui.painter()
                    .rect_filled(badge, 2., design::colors(ui).success.base);
                ui.painter().text(
                    badge.center(),
                    egui::Align2::CENTER_CENTER,
                    (i + 1).to_string(),
                    egui::FontId::proportional(14.),
                    design::colors(ui).success.on_base,
                );
                if response.clicked()
                    && response
                        .interact_pointer_pos()
                        .is_some_and(|p| rect.contains(p))
                {
                    self.region = Some(egui::Rect::from_min_max(a, b));
                    self.region_name = format!("page_{:04}_region_{:03}", page.page, i + 1);
                }
            }
            if response.drag_started() {
                self.anchor = ui.input(|i| i.pointer.press_origin()).map(normalized);
                self.region = None;
            }
            if response.dragged() || response.drag_stopped() {
                if let (Some(anchor), Some(pos)) =
                    (self.anchor, ui.input(|i| i.pointer.interact_pos()))
                {
                    self.region = Some(egui::Rect::from_two_pos(anchor, normalized(pos)));
                }
            }
            if let Some(region) = self.region {
                let a = response.rect.min + region.min.to_vec2() * response.rect.size();
                let b = response.rect.min + region.max.to_vec2() * response.rect.size();
                let rect = egui::Rect::from_min_max(a, b);
                ui.painter().rect_filled(
                    rect,
                    0.,
                    design::colors(ui).primary.base.gamma_multiply(0.14),
                );
                ui.painter().rect_stroke(
                    rect,
                    0.,
                    egui::Stroke::new(2_f32, design::colors(ui).primary.text),
                    egui::StrokeKind::Inside,
                );
            }
        } else {
            ui.label("Load a page, draw a rectangle around a photo and name it. This also works for scans and vector graphics; regions are rasterized PNGs.");
        }
    }
    fn text_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(self.job.is_none(), |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Read text layer").clicked() {
                    let doc = self.document.as_ref().unwrap().clone();
                    self.start(ui.ctx(), "Reading text…", move |c, _, _| {
                        pdf_tools::read_text(&doc, c).map(Completed::Text)
                    });
                }
                if ui
                    .add_enabled(self.job.is_none(), egui::Button::new("Check OCR languages"))
                    .clicked()
                {
                    self.start(ui.ctx(), "Checking Tesseract…", move |c, _, _| {
                        pdf_tools::ocr_languages(c).map(Completed::Languages)
                    });
                }
                if ui
                    .add_enabled(!self.text.is_empty(), egui::Button::new("Copy"))
                    .clicked()
                {
                    ui.ctx().copy_text(self.text.clone());
                }
                if ui
                    .add_enabled(!self.text.is_empty(), egui::Button::new("Save TXT…"))
                    .clicked()
                {
                    if let Some(path) = desktop::save_file("extracted.txt", "txt") {
                        match business::documents::save_source(&path, &self.text) {
                            Ok(()) => {
                                self.message = format!("Saved {}", path.display());
                                self.error = false;
                            }
                            Err(e) => {
                                self.message = e.to_string();
                                self.error = true;
                            }
                        }
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("OCR pages");
                let pages = self.document.as_ref().unwrap().pages;
                ui.add(egui::DragValue::new(&mut self.ocr_first).range(1..=pages));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut self.ocr_last).range(1..=pages));
                ui.label("Language");
                ui.add(
                    egui::TextEdit::singleline(&mut self.language)
                        .desired_width(90.)
                        .hint_text("vie+eng"),
                );
                if ui
                    .add_enabled(self.job.is_none(), egui::Button::new("Run OCR"))
                    .clicked()
                {
                    let doc = self.document.as_ref().unwrap().clone();
                    let (first, last, lang) = (
                        self.ocr_first,
                        self.ocr_last,
                        self.language.trim().to_string(),
                    );
                    self.start(
                        ui.ctx(),
                        "Recognizing scanned text…",
                        move |c, tx, ctx| {
                            pdf_tools::ocr_text(&doc, first, last, &lang, c, |d, t| {
                                progress(tx, ctx, d, t)
                            })
                            .map(Completed::Text)
                        },
                    );
                }
            });
        });
        ui.colored_label(crate::ui::design::colors(ui).muted, "Offline OCR requires Tesseract and language packs (eng, vie). Review recognition errors before saving.");
        egui::ScrollArea::both().id_salt("pdf-text").show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(15)
                    .hint_text("Extracted or recognized text appears here; you can correct it."),
            );
        });
    }
}
fn pixel_crop(rect: egui::Rect, width: u32, height: u32) -> Option<[u32; 4]> {
    if !rect.is_finite() {
        return None;
    }
    let x = (rect.left().clamp(0., 1.) * width as f32).floor() as u32;
    let y = (rect.top().clamp(0., 1.) * height as f32).floor() as u32;
    let right = (rect.right().clamp(0., 1.) * width as f32).ceil() as u32;
    let bottom = (rect.bottom().clamp(0., 1.) * height as f32).ceil() as u32;
    if right <= x || bottom <= y {
        None
    } else {
        Some([x, y, right - x, bottom - y])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_coordinates_preserve_resolution_and_clamp_edges() {
        assert_eq!(
            pixel_crop(
                egui::Rect::from_min_max(egui::pos2(0.25, 0.1), egui::pos2(0.75, 0.6)),
                2000,
                1000
            ),
            Some([500, 100, 1000, 500])
        );
        assert_eq!(
            pixel_crop(
                egui::Rect::from_min_max(egui::pos2(-0.1, 0.), egui::pos2(1.2, 1.)),
                2000,
                1000
            ),
            Some([0, 0, 2000, 1000])
        );
        assert_eq!(pixel_crop(egui::Rect::ZERO, 2000, 1000), None);
    }
}
