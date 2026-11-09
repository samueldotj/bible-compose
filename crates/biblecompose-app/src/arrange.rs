//! Putting the front matter at the front.
//!
//! The class sets the front matter *last*, after every book, because its
//! table of contents names the page each book starts on and those pages are
//! known only once the books are set. The alternative is a second run of the
//! typesetter over the whole publication, which on a Bible is minutes, to
//! learn numbers the first run already knew by the time it reached the end.
//! So the first run sets the front matter at the end of the file and says how
//! many pages it took, and this moves them.
//!
//! What moving them is: a page tree whose leaves are in a different order.
//! Everything else in a PDF that refers to a page — a bookmark, a link, a
//! destination — refers to the page *object*, not to its position, so nothing
//! else has to change for the pages to be right. The outline is the one
//! thing that is about order rather than identity: its entries are a list
//! the viewer shows top to bottom, and the front matter's entries, made last,
//! are moved to the head of it so the outline reads as the book does.
//!
//! The tree comes back flat — every page a direct child of the root — because
//! a flat tree is a correct tree and the only kind this can write without
//! knowing how the typesetter balanced its own. The attributes a page may
//! inherit from the nodes above it (`Resources`, `MediaBox`, `CropBox`,
//! `Rotate`) are copied down first, so a page that was relying on its parent
//! for its size still has one.

use biblecompose_diagnostics::{code, Diagnostic};
use camino::Utf8Path;
use lopdf::{Dictionary, Document, Object, ObjectId};

/// What the class reported at the end of its run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrontMatter {
    /// Pages at the end of the file that belong at its front.
    pub pages: u32,
    /// Top-level outline entries at the end that belong at its head.
    pub bookmarks: u32,
}

impl FrontMatter {
    /// The line the class writes to its log: `biblecompose-front-pages 4 3`.
    pub fn parse_log_line(text: &str) -> Option<FrontMatter> {
        let rest = text.trim().strip_prefix("biblecompose-front-pages ")?;
        let mut parts = rest.split_whitespace();
        let pages = parts.next()?.parse().ok()?;
        let bookmarks = parts.next()?.parse().ok()?;
        Some(FrontMatter { pages, bookmarks })
    }
}

/// Move the last `front.pages` pages of the PDF at `pdf` to its front, in
/// place, and the last `front.bookmarks` top-level outline entries with them.
pub fn front_first(pdf: &Utf8Path, front: FrontMatter) -> Result<(), Diagnostic> {
    if front.pages == 0 {
        return Ok(());
    }
    let fail = |what: &str, detail: String| {
        Diagnostic::error(
            code::PUBLISH_FAILED,
            format!("could not put the front matter first: {what}"),
        )
        .help("the typeset pages are in the build folder; report this with the backend log")
        .detail(detail)
    };

    let raw = std::fs::read(pdf.as_std_path())
        .map_err(|e| fail("the PDF could not be read", e.to_string()))?;
    let mut doc =
        Document::load_mem(&raw).map_err(|e| fail("the PDF could not be parsed", e.to_string()))?;

    reorder_pages(&mut doc, front.pages as usize).map_err(|e| fail("the page tree", e))?;
    if front.bookmarks > 0 {
        reorder_outline(&mut doc, front.bookmarks as usize).map_err(|e| fail("the outline", e))?;
    }

    // Only what the document still reaches. The typesetter packs its
    // dictionaries into object streams, which the parser unpacks into
    // objects of their own and keeps the packed copies of beside them; the
    // page tree's old inner nodes are unreferenced once it is flat; and the
    // old cross-reference stream describes a file that no longer exists.
    // Written out as they stand, the pages would be in the file twice — once
    // live and once inside a stream nothing points at — and a reader that
    // counts `/Type /Page` would count every page twice. Measured: a
    // three-page document reported six.
    doc.prune_objects();
    let mut out = Vec::with_capacity(raw.len());
    doc.save_to(&mut out)
        .map_err(|e| fail("the PDF could not be written", e.to_string()))?;
    std::fs::write(pdf.as_std_path(), out)
        .map_err(|e| fail("the PDF could not be written", e.to_string()))?;
    Ok(())
}

/// The keys a page inherits from its ancestors when it does not carry them.
const INHERITED: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

fn reorder_pages(doc: &mut Document, tail: usize) -> Result<(), String> {
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
    if tail > pages.len() {
        return Err(format!(
            "{tail} pages of front matter were reported in a document of {}",
            pages.len()
        ));
    }
    let root = doc
        .catalog()
        .map_err(|e| e.to_string())?
        .get(b"Pages")
        .and_then(Object::as_reference)
        .map_err(|e| e.to_string())?;

    // Inheritance first, read before any parent is rewritten.
    for page in &pages {
        for key in INHERITED {
            if doc
                .get_dictionary(*page)
                .map_err(|e| e.to_string())?
                .has(key)
            {
                continue;
            }
            if let Some(value) = inherited(doc, *page, key) {
                doc.get_dictionary_mut(*page)
                    .map_err(|e| e.to_string())?
                    .set(key, value);
            }
        }
    }

    let (body, front) = pages.split_at(pages.len() - tail);
    let order: Vec<ObjectId> = front.iter().chain(body.iter()).copied().collect();
    for page in &order {
        doc.get_dictionary_mut(*page)
            .map_err(|e| e.to_string())?
            .set("Parent", Object::Reference(root));
    }
    let kids: Vec<Object> = order.iter().map(|id| Object::Reference(*id)).collect();
    let root_dict = doc.get_dictionary_mut(root).map_err(|e| e.to_string())?;
    root_dict.set("Kids", Object::Array(kids));
    root_dict.set("Count", Object::Integer(order.len() as i64));
    Ok(())
}

/// A value from the nearest ancestor that has `key`, walking `Parent` up.
fn inherited(doc: &Document, page: ObjectId, key: &[u8]) -> Option<Object> {
    let mut node = doc
        .get_dictionary(page)
        .ok()?
        .get(b"Parent")
        .ok()?
        .as_reference()
        .ok()?;
    for _ in 0..64 {
        let dict = doc.get_dictionary(node).ok()?;
        if let Ok(value) = dict.get(key) {
            return Some(value.clone());
        }
        node = dict.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

/// Move the last `tail` top-level outline entries to the head of the list.
///
/// Outline entries are a doubly linked list under the outline root: `First`
/// and `Last` on the root, `Prev` and `Next` on each entry. Moving a run from
/// the end to the start is six pointer changes and does not touch the entries
/// themselves, their children, or what they point at.
fn reorder_outline(doc: &mut Document, tail: usize) -> Result<(), String> {
    let Some(outlines) = doc
        .catalog()
        .map_err(|e| e.to_string())?
        .get(b"Outlines")
        .ok()
        .and_then(|o| o.as_reference().ok())
    else {
        // No outline at all — the project writes no destinations — so there
        // is nothing to arrange.
        return Ok(());
    };
    let link = |doc: &Document, id: ObjectId, key: &[u8]| -> Option<ObjectId> {
        doc.get_dictionary(id)
            .ok()?
            .get(key)
            .ok()?
            .as_reference()
            .ok()
    };
    let (Some(first), Some(last)) = (link(doc, outlines, b"First"), link(doc, outlines, b"Last"))
    else {
        return Ok(());
    };

    // Walk back from the end to the first entry that moves, and note the one
    // before it — the new last entry.
    let mut moved_first = last;
    for _ in 1..tail {
        match link(doc, moved_first, b"Prev") {
            Some(prev) => moved_first = prev,
            None => return Ok(()), // fewer entries than reported: leave it be
        }
    }
    let Some(new_last) = link(doc, moved_first, b"Prev") else {
        return Ok(()); // every entry is front matter; the order is right already
    };

    let set =
        |doc: &mut Document, id: ObjectId, key: &str, to: Option<ObjectId>| -> Result<(), String> {
            let dict: &mut Dictionary = doc.get_dictionary_mut(id).map_err(|e| e.to_string())?;
            match to {
                Some(target) => dict.set(key, Object::Reference(target)),
                None => {
                    dict.remove(key.as_bytes());
                }
            }
            Ok(())
        };
    set(doc, new_last, "Next", None)?;
    set(doc, moved_first, "Prev", None)?;
    set(doc, last, "Next", Some(first))?;
    set(doc, first, "Prev", Some(last))?;
    set(doc, outlines, "First", Some(moved_first))?;
    set(doc, outlines, "Last", Some(new_last))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    #[test]
    fn the_log_line_is_read_and_anything_else_is_not() {
        assert_eq!(
            FrontMatter::parse_log_line("biblecompose-front-pages 4 3"),
            Some(FrontMatter {
                pages: 4,
                bookmarks: 3
            })
        );
        assert_eq!(
            FrontMatter::parse_log_line("  biblecompose-front-pages 2 0 "),
            Some(FrontMatter {
                pages: 2,
                bookmarks: 0
            })
        );
        assert_eq!(FrontMatter::parse_log_line("[12] [13]"), None);
        assert_eq!(
            FrontMatter::parse_log_line("biblecompose-front-pages 4"),
            None
        );
        assert_eq!(
            FrontMatter::parse_log_line("biblecompose-front-pages x y"),
            None
        );
    }

    /// A page tree of three pages with the last two moved to the front reads
    /// back in the new order, and a page that relied on its parent for its
    /// size still has one.
    #[test]
    fn pages_move_to_the_front_and_keep_what_they_inherited() {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::new();
        for _ in 0..3 {
            let content = doc.add_object(lopdf::Stream::new(Dictionary::new(), Vec::new()));
            let page = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => Object::Reference(pages_id),
                "Contents" => Object::Reference(content),
            });
            kids.push(page);
        }
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => kids.iter().map(|k| Object::Reference(*k)).collect::<Vec<_>>(),
            "Count" => 3,
            "MediaBox" => vec![0.into(), 0.into(), 432.into(), 648.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        });
        doc.trailer.set("Root", Object::Reference(catalog));

        reorder_pages(&mut doc, 2).expect("reorder");

        let order: Vec<ObjectId> = doc.get_pages().into_values().collect();
        assert_eq!(order, vec![kids[1], kids[2], kids[0]]);
        for page in &order {
            let dict = doc.get_dictionary(*page).expect("a page");
            assert!(dict.has(b"MediaBox"), "the size came down from the parent");
            assert_eq!(
                dict.get(b"Parent").and_then(Object::as_reference).ok(),
                Some(pages_id)
            );
        }
        assert_eq!(
            doc.get_dictionary(pages_id)
                .unwrap()
                .get(b"Count")
                .and_then(Object::as_i64)
                .ok(),
            Some(3)
        );
    }

    /// Four outline entries with the last two moved to the head: the list
    /// reads back in the new order from both ends.
    #[test]
    fn outline_entries_move_to_the_head() {
        let mut doc = Document::with_version("1.5");
        let outlines = doc.new_object_id();
        let ids: Vec<ObjectId> = (0..4).map(|_| doc.new_object_id()).collect();
        for (i, id) in ids.iter().enumerate() {
            let mut d = dictionary! { "Title" => Object::string_literal(format!("e{i}")), "Parent" => Object::Reference(outlines) };
            if i > 0 {
                d.set("Prev", Object::Reference(ids[i - 1]));
            }
            if i + 1 < ids.len() {
                d.set("Next", Object::Reference(ids[i + 1]));
            }
            doc.objects.insert(*id, Object::Dictionary(d));
        }
        doc.objects.insert(
            outlines,
            Object::Dictionary(dictionary! {
                "Type" => "Outlines",
                "First" => Object::Reference(ids[0]),
                "Last" => Object::Reference(ids[3]),
            }),
        );
        let catalog = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Outlines" => Object::Reference(outlines),
        });
        doc.trailer.set("Root", Object::Reference(catalog));

        reorder_outline(&mut doc, 2).expect("reorder");

        let link = |id: ObjectId, key: &[u8]| -> Option<ObjectId> {
            doc.get_dictionary(id)
                .ok()?
                .get(key)
                .ok()?
                .as_reference()
                .ok()
        };
        let mut forward = Vec::new();
        let mut at = link(outlines, b"First");
        while let Some(id) = at {
            forward.push(id);
            at = link(id, b"Next");
        }
        assert_eq!(forward, vec![ids[2], ids[3], ids[0], ids[1]]);
        let mut backward = Vec::new();
        let mut at = link(outlines, b"Last");
        while let Some(id) = at {
            backward.push(id);
            at = link(id, b"Prev");
        }
        assert_eq!(backward, vec![ids[1], ids[0], ids[3], ids[2]]);
    }
}
