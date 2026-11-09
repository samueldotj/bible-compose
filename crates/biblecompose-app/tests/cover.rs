//! The cover: `book_cover.pdf`, sized from the page, the paper and the
//! binding, and lettered from the `[cover]` settings.
//!
//! No typesetter is needed — the cover is drawn by the application — so every
//! test here runs on every machine. The font is one the repository already
//! ships for the window, copied into the project's `assets/fonts` the way a
//! publisher would ship one with a book.

mod common;

use biblecompose_app::cover::{
    self, Geometry, PageSource, BOARD_SQUARE, BOARD_THICKNESS, HINGE, WRAP,
};
use biblecompose_app::project;
use biblecompose_config::value::{CoverBinding, CoverBorder};
use biblecompose_diagnostics::code;
use camino::Utf8PathBuf;
use common::settings;
use lopdf::{dictionary, Document, Object};

const INCH: f64 = 72.0;

const GENESIS: &str = concat!(
    "\\id GEN\n\\h Genesis\n\\c 1\n\\p\n",
    "\\v 1 In the beginning God created the heavens and the earth.\n",
    "\\v 2 Now the earth was formless and void.\n",
);

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

/// A folder with Genesis 1, the repository's Newsreader in its fonts, and
/// these settings — opened as the CLI opens it.
fn project_with(cover_settings: &str) -> (tempfile::TempDir, project::Opened) {
    let guard = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(guard.path().to_path_buf()).expect("UTF-8 temp path");
    std::fs::write(root.join("GEN.usfm"), GENESIS).unwrap();
    let fonts = root.join("assets/fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::copy(
        biblecompose_testkit::repo_root().join("src/assets/fonts/Newsreader-Variable.ttf"),
        fonts.join("Newsreader-Variable.ttf"),
    )
    .unwrap();
    // The repository's Newsreader, unless the test names a family of its own.
    let font = if cover_settings.contains("font_family") {
        ""
    } else {
        "font_family = \"Newsreader\"\n"
    };
    std::fs::write(
        root.join("biblecompose.toml"),
        format!(
            "schema_version = 1\n[project]\nname = \"Test Bible\"\n[cover]\n{font}{cover_settings}\n"
        ),
    )
    .unwrap();
    let opened = project::open(&root);
    assert!(!opened.blocked(), "the project should open");
    (guard, opened)
}

/// Every stream in the file, inflated where it was compressed.
fn streams(doc: &Document) -> Vec<Vec<u8>> {
    doc.objects
        .values()
        .filter_map(|o| match o {
            Object::Stream(s) => Some(
                s.decompressed_content()
                    .unwrap_or_else(|_| s.content.clone()),
            ),
            _ => None,
        })
        .collect()
}

fn page_box(doc: &Document, name: &str) -> [f64; 4] {
    let (_, page) = doc.get_pages().into_iter().next().expect("one page");
    let dict = doc.get_dictionary(page).unwrap();
    let arr = dict.get(name.as_bytes()).unwrap().as_array().unwrap();
    let mut out = [0.0; 4];
    for (i, v) in arr.iter().enumerate() {
        out[i] = v
            .as_float()
            .map(f64::from)
            .or_else(|_| v.as_i64().map(|n| n as f64))
            .unwrap();
    }
    out
}

// ---- the sheet's measurements --------------------------------------------

#[test]
fn a_paperback_is_back_spine_and_front_with_a_bleed() {
    let s = settings("[page]\nsize = \"6x9in\"\n[cover]\npaper = \"bible\"\nbleed = \"0.125in\"");
    let g = Geometry::new(&s, 1000);
    // 1,000 pages of Bible paper at 0.0008in a page.
    assert!(close(g.spine, 0.8 * INCH), "spine {}", g.spine);
    assert!(close(g.width, (0.25 + 12.0 + 0.8) * INCH));
    assert!(close(g.height, 9.25 * INCH));
    assert_eq!(g.hinge, 0.0);
    // The front is the right-hand face, and the back the left.
    assert!(close(g.front().x, (0.125 + 6.0 + 0.8) * INCH));
    assert!(close(g.back().x, 0.125 * INCH));
    let trim = g.trim_box();
    assert!(close(trim.w, 12.8 * INCH) && close(trim.h, 9.0 * INCH));
}

#[test]
fn the_paper_decides_how_thick_the_spine_is() {
    let thick =
        |paper: &str| Geometry::new(&settings(&format!("[cover]\npaper = \"{paper}\"")), 800).spine;
    assert!(thick("bible") < thick("thin"));
    assert!(thick("thin") < thick("white"));
    assert!(thick("white") < thick("cream"));
}

#[test]
fn a_hardcover_is_a_case_wrap() {
    let s =
        settings("[page]\nsize = \"6x9in\"\n[cover]\nbinding = \"hardcover\"\npaper = \"white\"");
    let g = Geometry::new(&s, 400);
    let text_block = 400.0 * 0.002252 * INCH;
    assert_eq!(g.binding, CoverBinding::Hardcover);
    assert!(close(g.spine, text_block + 2.0 * BOARD_THICKNESS));
    assert!(close(g.face_width, 6.0 * INCH + BOARD_SQUARE));
    assert!(close(g.face_height, 9.0 * INCH + 2.0 * BOARD_SQUARE));
    assert!(close(g.margin, WRAP));
    assert!(close(
        g.width,
        2.0 * WRAP + 2.0 * g.face_width + 2.0 * HINGE + g.spine
    ));
    // The wrap is folded, not trimmed: the printer cuts at the sheet's edge.
    let trim = g.trim_box();
    assert!(close(trim.w, g.width) && close(trim.h, g.height));
}

#[test]
fn a_spine_width_from_the_printer_wins_over_the_paper() {
    let s = settings("[cover]\nspine_width = \"1.5in\"");
    assert!(close(Geometry::new(&s, 10).spine, 1.5 * INCH));
}

#[test]
fn the_page_size_is_the_face() {
    let s = settings("[page]\nsize = \"a5\"");
    let g = Geometry::new(&s, 100);
    assert!(close(g.face_width, s.page.size.width.points()));
    assert!(close(g.face_height, s.page.size.height.points()));
}

// ---- the file ---------------------------------------------------------------

#[test]
fn a_cover_is_one_page_with_its_boxes_and_its_font_embedded() {
    let (guard, opened) = project_with("pages = 600\nsubtitle = \"Berean Standard Bible\"");
    let out = cover::default_output(&opened);
    assert!(out.ends_with("output/book_cover.pdf"), "{out}");

    let report = cover::make(&opened, &out, &[]);
    assert!(
        report.succeeded(),
        "{:?}",
        report.diagnostics.iter().collect::<Vec<_>>()
    );
    assert_eq!(report.pages_from, Some(PageSource::Setting));
    let g = report.geometry.expect("geometry");

    let doc = Document::load(out.as_std_path()).expect("a PDF");
    assert_eq!(doc.get_pages().len(), 1);
    let media = page_box(&doc, "MediaBox");
    assert!(close(media[2], g.width) && close(media[3], g.height));
    let trim = page_box(&doc, "TrimBox");
    assert!(close(trim[0], 0.125 * INCH) && close(trim[3], g.height - 0.125 * INCH));

    let all = streams(&doc);
    // The whole TrueType file, not a reference to a font the printer may not
    // have.
    let font_file =
        std::fs::read(opened.root.join("assets/fonts/Newsreader-Variable.ttf")).unwrap();
    assert!(all.contains(&font_file), "the font is embedded whole");
    // The lettering maps back to text, so the cover can be searched.
    let cmap = all
        .iter()
        .find(|s| s.windows(11).any(|w| w == b"beginbfchar"))
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .expect("a ToUnicode map");
    for letter in ["0048", "0042", "0049", "004C", "0045"] {
        assert!(cmap.contains(&format!("<{letter}>")), "{letter} in the map");
    }
    drop(guard);
}

#[test]
fn uppercase_off_keeps_the_title_as_written() {
    let (_guard, opened) = project_with("pages = 600\nuppercase = false");
    let out = cover::default_output(&opened);
    assert!(cover::make(&opened, &out, &[]).succeeded());
    let doc = Document::load(out.as_std_path()).unwrap();
    let cmap = streams(&doc)
        .into_iter()
        .map(|s| String::from_utf8_lossy(&s).into_owned())
        .find(|s| s.contains("beginbfchar"))
        .unwrap();
    // "Holy" keeps its lowercase o.
    assert!(cmap.contains("<006F>"));
}

#[test]
fn every_border_style_makes_a_cover() {
    for style in CoverBorder::SPELLINGS {
        let (_guard, opened) = project_with(&format!("pages = 900\nborder = \"{style}\""));
        let out = cover::default_output(&opened);
        let report = cover::make(&opened, &out, &[]);
        assert!(report.succeeded(), "{style}");
        assert_eq!(
            *opened.settings.cover.border,
            CoverBorder::NAMES
                .iter()
                .find(|(n, _)| n == style)
                .unwrap()
                .1
        );
    }
}

#[test]
fn every_binding_and_paper_makes_a_cover() {
    for binding in CoverBinding::SPELLINGS {
        for paper in ["bible", "thin", "white", "cream"] {
            let (_guard, opened) = project_with(&format!(
                "pages = 700\nbinding = \"{binding}\"\npaper = \"{paper}\"\nshow_guides = true"
            ));
            let out = cover::default_output(&opened);
            assert!(
                cover::make(&opened, &out, &[]).succeeded(),
                "{binding} on {paper}"
            );
        }
    }
}

// ---- where the page count comes from ----------------------------------------

#[test]
fn a_built_book_is_counted() {
    let (_guard, opened) = project_with("");
    // A three-page stand-in for the book, where the build would put it.
    let book = opened.output();
    std::fs::create_dir_all(book.parent().unwrap()).unwrap();
    let mut doc = lopdf::Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let kids: Vec<Object> = (0..3)
        .map(|_| {
            Object::Reference(doc.add_object(dictionary! {
                "Type" => Object::Name(b"Page".to_vec()),
                "Parent" => Object::Reference(pages_id),
                "MediaBox" => vec![0.into(), 0.into(), 432.into(), 648.into()],
            }))
        })
        .collect();
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => Object::Name(b"Pages".to_vec()),
            "Kids" => kids,
            "Count" => 3,
        }),
    );
    let catalog = doc.add_object(dictionary! {
        "Type" => Object::Name(b"Catalog".to_vec()),
        "Pages" => Object::Reference(pages_id),
    });
    doc.trailer.set("Root", catalog);
    doc.save(book.as_std_path()).unwrap();

    let (pages, from) = cover::page_count(&opened, &book).expect("counted");
    assert_eq!(pages, 3);
    assert_eq!(from, PageSource::Book(book));
}

#[test]
fn an_unbuilt_book_is_estimated_and_says_so() {
    let (_guard, opened) = project_with("");
    let out = cover::default_output(&opened);
    let report = cover::make(&opened, &out, &[]);
    assert!(report.succeeded());
    assert_eq!(report.pages_from, Some(PageSource::Estimate));
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == code::COVER_PAGES_ESTIMATED));
    // One chapter is a thin book: the spine is too narrow to letter.
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == code::COVER_SPINE_NARROW));
}

#[test]
fn a_font_that_is_not_there_writes_nothing() {
    let (_guard, opened) = project_with("pages = 600\nfont_family = \"No Such Family 9\"");
    let out = cover::default_output(&opened);
    let report = cover::make(&opened, &out, &[]);
    assert!(!report.succeeded());
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.code == code::UNRESOLVED));
    assert!(!out.exists());
}
