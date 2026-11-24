//! The preview: pages of the built PDF, drawn here and shown in the window
//! (GUI-008).
//!
//! # Why pages arrive as pictures
//!
//! The webview has no PDF renderer it can be relied on to have — WebKitGTK has
//! none at all — so the shell draws each page with hayro and hands the window a
//! PNG. The frontend sees nothing but `<img>` elements, which keeps every PDF
//! byte on this side of the bridge and leaves the frontend with no parser to
//! trust.
//!
//! # How a page is asked for
//!
//! [`open`] reads one PDF and says how many pages it has and how big each is.
//! The window then asks for pages through the `bcpdf` scheme
//! (see [`SCHEME`]), one address per page per width:
//!
//! ```text
//! bcpdf://localhost/<generation>_<page>_<width>
//! ```
//!
//! `generation` changes every time a document is opened, so an address names
//! one drawing of one file and the webview can keep it forever; a rebuild is a
//! new generation rather than a cache to clear.
//!
//! # What it will not draw
//!
//! Only the document [`open`] was given, and only the generation it was given
//! as. The scheme takes no path, so nothing the webview can type reaches a
//! file; the command that does take a path is checked against the open project
//! by its caller.
//!
//! # Nothing is drawn on the caller's thread
//!
//! A page takes a few milliseconds and the window asks for several at once
//! while scrolling. They go to a small pool of workers, and a request for a
//! generation that has since been replaced is dropped before it is drawn, so a
//! rebuild does not leave the pool finishing pages nobody will see.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};

use biblecompose_diagnostics::Diagnostic as AppDiagnostic;
use camino::Utf8Path;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{render, PixmapSettings, RenderCache, RenderSettings};
use serde::Serialize;

/// The URI scheme pages are served under.
pub const SCHEME: &str = "bcpdf";

/// The narrowest and widest page the window may ask for, in device pixels.
///
/// The ceiling is what keeps a request from asking for a gigapixel drawing: a
/// 6-inch page at this width is about 650 dpi, which is past what any screen
/// shows and past where proofreading a hairline stops improving.
pub const MIN_WIDTH: u32 = 64;
pub const MAX_WIDTH: u32 = 4096;

/// What the window learns about a document when it is opened.
#[derive(Debug, Clone, Serialize)]
pub struct WirePreview {
    /// The address prefix of this drawing. Ask for pages under this and no
    /// other.
    pub generation: u64,
    /// Width and height of every page in points, rotation applied — enough to
    /// lay out the whole document before a single page has been drawn.
    pub pages: Vec<[f32; 2]>,
}

/// The document being previewed.
struct Loaded {
    generation: u64,
    pdf: Pdf,
}

/// One page the window has asked for.
pub struct Job {
    generation: u64,
    page: usize,
    width: u32,
    reply: Box<dyn FnOnce(Result<Vec<u8>, Refusal>) + Send>,
}

/// Why a page was not drawn. Each is a status the webview understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The address is not one this scheme makes.
    BadRequest,
    /// No such page, or the document it named has been replaced.
    NotFound,
    /// The page could not be drawn or encoded.
    Failed,
}

impl Refusal {
    pub const fn status(self) -> u16 {
        match self {
            Refusal::BadRequest => 400,
            Refusal::NotFound => 404,
            Refusal::Failed => 500,
        }
    }
}

/// The preview's share of the session.
#[derive(Default)]
pub struct Preview {
    current: Mutex<Option<Arc<Loaded>>>,
    generations: AtomicU64,
    pool: OnceLock<Mutex<Sender<Job>>>,
}

impl Preview {
    /// Read `path` and make it the document pages are drawn from.
    ///
    /// The caller has already decided `path` may be read; this decides only
    /// whether it is a PDF.
    pub fn open(&self, path: &Utf8Path) -> Result<WirePreview, AppDiagnostic> {
        let bytes = std::fs::read(path).map_err(|e| {
            AppDiagnostic::error(
                biblecompose_diagnostics::code::NOT_FOUND,
                format!("could not read {path} to preview it"),
            )
            .help("build again, or the file was moved or renamed since")
            .detail(e.to_string())
        })?;
        let pdf = Pdf::new(Arc::new(bytes)).map_err(|e| {
            AppDiagnostic::error(
                biblecompose_diagnostics::code::NOT_FOUND,
                format!("{path} could not be read as a PDF"),
            )
            .help("Open PDF shows it in the platform's own viewer, which may say more")
            .detail(format!("{e:?}"))
        })?;

        let pages = pdf
            .pages()
            .iter()
            .map(|p| {
                let (w, h) = p.render_dimensions();
                [w, h]
            })
            .collect();
        let generation = self.generations.fetch_add(1, Ordering::Relaxed) + 1;
        *self
            .current
            .lock()
            .expect("the preview lock is not poisoned") =
            Some(Arc::new(Loaded { generation, pdf }));
        Ok(WirePreview { generation, pages })
    }

    /// Stop previewing. Pages already asked for are refused.
    pub fn close(&self) {
        *self
            .current
            .lock()
            .expect("the preview lock is not poisoned") = None;
    }

    /// Hand one request to the pool. `reply` is called exactly once, on a
    /// worker thread or — for an address that names nothing — on this one.
    pub fn request(
        self: &Arc<Self>,
        address: &str,
        reply: impl FnOnce(Result<Vec<u8>, Refusal>) + Send + 'static,
    ) {
        let Some((generation, page, width)) = parse(address) else {
            return reply(Err(Refusal::BadRequest));
        };
        let job = Job {
            generation,
            page,
            width,
            reply: Box::new(reply),
        };
        let sender = self.pool.get_or_init(|| Mutex::new(self.start_pool()));
        let sent = sender
            .lock()
            .expect("the preview lock is not poisoned")
            .send(job);
        if let Err(std::sync::mpsc::SendError(job)) = sent {
            (job.reply)(Err(Refusal::Failed));
        }
    }

    /// Draw one page now, on this thread. What a worker does with a job, and
    /// what the tests call.
    pub fn draw(&self, generation: u64, page: usize, width: u32) -> Result<Vec<u8>, Refusal> {
        let loaded = self
            .current
            .lock()
            .expect("the preview lock is not poisoned")
            .clone()
            .filter(|l| l.generation == generation)
            .ok_or(Refusal::NotFound)?;
        draw_page(&loaded.pdf, page, width)
    }

    fn start_pool(self: &Arc<Self>) -> Sender<Job> {
        let (tx, rx) = channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        // Two to four: enough that a screenful of pages arrives together, few
        // enough that a preview never competes with a build for every core.
        let workers = std::thread::available_parallelism()
            .map_or(2, |n| n.get() / 2)
            .clamp(2, 4);
        for _ in 0..workers {
            let rx = Arc::clone(&rx);
            let preview = Arc::downgrade(self);
            std::thread::spawn(move || work(&rx, &preview));
        }
        tx
    }
}

fn work(rx: &Mutex<Receiver<Job>>, preview: &std::sync::Weak<Preview>) {
    loop {
        let job = {
            let rx = rx.lock().expect("the preview lock is not poisoned");
            match rx.recv() {
                Ok(job) => job,
                Err(_) => return,
            }
        };
        let Some(preview) = preview.upgrade() else {
            (job.reply)(Err(Refusal::NotFound));
            return;
        };
        let result = preview.draw(job.generation, job.page, job.width);
        (job.reply)(result);
    }
}

/// `<generation>_<page>_<width>`, with or without the leading slash a URI path
/// carries. `page` counts from one, as the window shows it.
pub fn parse(address: &str) -> Option<(u64, usize, u32)> {
    let mut parts = address.trim_start_matches('/').split('_');
    let generation = parts.next()?.parse().ok()?;
    let page: usize = parts.next()?.parse().ok()?;
    let width = parts.next()?.parse().ok()?;
    if parts.next().is_some() || page == 0 {
        return None;
    }
    Some((generation, page, width))
}

/// One page, `width` device pixels wide, as an opaque RGB PNG.
fn draw_page(pdf: &Pdf, page: usize, width: u32) -> Result<Vec<u8>, Refusal> {
    let page = pdf.pages().get(page - 1).ok_or(Refusal::NotFound)?;
    let (w, h) = page.render_dimensions();
    if w <= 0.0 || h <= 0.0 {
        return Err(Refusal::Failed);
    }
    let width = width.clamp(MIN_WIDTH, MAX_WIDTH);
    let scale = width as f32 / w;
    // The height follows from the width; a page far taller than it is wide
    // must not turn a sensible width into a pixmap past what u16 can hold.
    if h * scale > f32::from(u16::MAX) {
        return Err(Refusal::BadRequest);
    }

    let cache = RenderCache::new();
    let pixmap = render(
        page,
        &cache,
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &PixmapSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
        },
    );
    encode(
        u32::from(pixmap.width()),
        u32::from(pixmap.height()),
        pixmap.data_as_u8_slice(),
    )
}

/// RGBA to an RGB PNG. The page is drawn over white, so alpha is always full
/// and premultiplied and straight are the same bytes; dropping it is a quarter
/// off every page sent.
fn encode(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, Refusal> {
    let rgb: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|px| [px[0], px[1], px[2]])
        .collect();
    let mut out = Vec::with_capacity(rgb.len() / 4);
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    // Fast rather than small: the image crosses a process boundary, not a
    // network, and the trial measured the default encoder at three times the
    // cost of drawing the page.
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header().map_err(|_| Refusal::Failed)?;
    writer.write_image_data(&rgb).map_err(|_| Refusal::Failed)?;
    writer.finish().map_err(|_| Refusal::Failed)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

    fn fixture() -> Utf8PathBuf {
        // Committed output of the S0 spike: one page, raster and vector art.
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spike/out/figures.pdf")
    }

    fn png_width(png: &[u8]) -> u32 {
        u32::from_be_bytes(png[16..20].try_into().unwrap())
    }

    #[test]
    fn an_address_names_generation_page_and_width() {
        assert_eq!(parse("/3_12_900"), Some((3, 12, 900)));
        assert_eq!(parse("3_12_900"), Some((3, 12, 900)));
        assert_eq!(parse("/3_0_900"), None, "pages count from one");
        assert_eq!(parse("/3_12"), None);
        assert_eq!(parse("/3_12_900_1"), None);
        assert_eq!(parse("/../etc/passwd"), None);
        assert_eq!(parse("/a_b_c"), None);
    }

    #[test]
    fn opening_reports_every_page_and_its_size() {
        let preview = Preview::default();
        let doc = preview.open(&fixture()).unwrap();
        assert_eq!(doc.pages.len(), 1);
        let [w, h] = doc.pages[0];
        assert!(w > 100.0 && h > w, "a portrait page in points, got {w}×{h}");
    }

    #[test]
    fn a_page_is_drawn_at_the_width_asked_for() {
        let preview = Preview::default();
        let doc = preview.open(&fixture()).unwrap();
        let png = preview.draw(doc.generation, 1, 600).unwrap();
        assert!(png.starts_with(PNG));
        assert_eq!(png_width(&png), 600);
    }

    #[test]
    fn the_width_is_kept_within_bounds() {
        let preview = Preview::default();
        let doc = preview.open(&fixture()).unwrap();
        let png = preview.draw(doc.generation, 1, 1_000_000).unwrap();
        assert_eq!(png_width(&png), MAX_WIDTH);
    }

    #[test]
    fn a_page_past_the_end_is_not_found() {
        let preview = Preview::default();
        let doc = preview.open(&fixture()).unwrap();
        assert_eq!(preview.draw(doc.generation, 2, 600), Err(Refusal::NotFound));
    }

    #[test]
    fn reopening_retires_the_old_addresses() {
        let preview = Preview::default();
        let first = preview.open(&fixture()).unwrap();
        let second = preview.open(&fixture()).unwrap();
        assert_ne!(first.generation, second.generation);
        assert_eq!(
            preview.draw(first.generation, 1, 600),
            Err(Refusal::NotFound)
        );
        assert!(preview.draw(second.generation, 1, 600).is_ok());
    }

    #[test]
    fn closing_refuses_everything() {
        let preview = Preview::default();
        let doc = preview.open(&fixture()).unwrap();
        preview.close();
        assert_eq!(preview.draw(doc.generation, 1, 600), Err(Refusal::NotFound));
    }

    #[test]
    fn a_file_that_is_not_a_pdf_is_refused_with_a_reason() {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("x.pdf")).unwrap();
        std::fs::write(&path, b"not a pdf").unwrap();
        let err = Preview::default().open(&path).unwrap_err();
        assert!(
            err.message.contains("could not be read as a PDF"),
            "{}",
            err.message
        );
    }

    #[test]
    fn the_pool_answers_every_request_once() {
        let preview = Arc::new(Preview::default());
        let doc = preview.open(&fixture()).unwrap();
        let (tx, rx) = channel();
        for address in [
            format!("/{}_1_300", doc.generation),
            format!("/{}_9_300", doc.generation),
            "/nonsense".to_owned(),
        ] {
            let tx = tx.clone();
            preview.request(&address, move |r| tx.send(r.map(|_| ())).unwrap());
        }
        drop(tx);
        let mut answers: Vec<_> = rx.iter().collect();
        answers.sort_by_key(|r| format!("{r:?}"));
        assert_eq!(
            answers,
            vec![Err(Refusal::BadRequest), Err(Refusal::NotFound), Ok(())]
        );
    }
}
