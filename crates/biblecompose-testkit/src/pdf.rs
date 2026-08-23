//! Structural facts about a PDF, for DET-002's assertions.
//!
//! **The PDF is never byte-compared**, and the reason is narrower than the
//! usual one. SILE already writes an all-zero document `/ID` and no creation
//! date — the obvious sources of variance are gone. What remains is the
//! **font subset tag**, generated randomly per run: four builds of identical
//! input gave four different hashes and two different file sizes, differing
//! only in prefixes like `AYABNL+DejaVuSerif` against `HQTCEM+DejaVuSerif`
//! (spike/NOTES.md F-15).
//!
//! So [`Pdf::fonts`] strips that six-letter prefix. Without it every run fails
//! on a difference that carries no information, and somebody spends an
//! afternoon looking for a `SOURCE_DATE_EPOCH` that does not exist.
//!
//! **These assertions do not detect missing glyphs**, and must not be relied
//! on for that. A page rendered entirely as `.notdef` boxes passes every one
//! of them — right page count, right geometry, fonts embedded, text extracts
//! fine, because the codepoints are present even where no glyph is
//! (spike/NOTES.md F-12). FONT-002's coverage pre-flight is the only defence.

use std::collections::BTreeMap;

use camino::Utf8Path;

#[derive(Debug, Clone, PartialEq)]
pub struct Pdf {
    pub bytes: usize,
    pub version: String,
    pub pages: usize,
    /// Page sizes in points, deduplicated, in first-seen order.
    pub page_sizes: Vec<(f64, f64)>,
    /// Font names with the random subset prefix stripped, sorted.
    pub fonts: Vec<String>,
    pub embedded_font_files: usize,
}

impl Pdf {
    pub fn read(path: &Utf8Path) -> std::io::Result<Pdf> {
        Ok(Pdf::parse(&std::fs::read(path.as_std_path())?))
    }

    /// Parses the little that DET-002 asserts, with no PDF library.
    ///
    /// SILE's libtexpdf backend writes cross-reference and object streams, so
    /// the objects recording page count and fonts are Flate-compressed and
    /// invisible to a plain byte search. Everything is inflated first.
    ///
    /// **Through the object table, not by walking for `stream` in the bytes.**
    /// The blind walk finds the first `stream` keyword and the next `endstream`
    /// after it, which lines up on a large file and does not on a small one: on
    /// a one-page document it skipped the object stream entirely, `pages` came
    /// back as 0, and `every_fixture_typesets_to_a_pdf` failed on the smallest
    /// fixture in the set while passing on the rest.
    pub fn parse(raw: &[u8]) -> Pdf {
        let mut blob = raw.to_vec();
        for body in objects(raw).values() {
            blob.extend_from_slice(body);
        }

        Pdf {
            bytes: raw.len(),
            version: String::from_utf8_lossy(&raw[..raw.len().min(8)])
                .trim_start_matches('%')
                .trim()
                .to_owned(),
            pages: count_pages(&blob),
            page_sizes: page_sizes(&blob),
            fonts: fonts(&blob),
            embedded_font_files: count(&blob, b"/FontFile"),
        }
    }

    /// The page size in inches, if every page is the same.
    pub fn uniform_page_size_inches(&self) -> Option<(f64, f64)> {
        match self.page_sizes.as_slice() {
            [(w, h)] => Some((w / 72.0, h / 72.0)),
            _ => None,
        }
    }

    pub fn has_font(&self, name: &str) -> bool {
        self.fonts.iter().any(|f| f == name)
    }
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

fn count_pages(blob: &[u8]) -> usize {
    // `/Type /Page` but not `/Pages`.
    let needle = b"/Type";
    let mut n = 0;
    let mut i = 0;
    while let Some(pos) = find(&blob[i..], needle) {
        let at = i + pos + needle.len();
        let rest = &blob[at..blob.len().min(at + 16)];
        let s = String::from_utf8_lossy(rest);
        let s = s.trim_start();
        if s.starts_with("/Page") && !s.starts_with("/Pages") {
            n += 1;
        }
        i = at;
    }
    n
}

fn page_sizes(blob: &[u8]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    let needle = b"/MediaBox";
    let mut i = 0;
    while let Some(pos) = find(&blob[i..], needle) {
        let at = i + pos + needle.len();
        let rest = &blob[at..blob.len().min(at + 80)];
        let s = String::from_utf8_lossy(rest);
        if let Some(open) = s.find('[') {
            if let Some(close) = s[open..].find(']') {
                let nums: Vec<f64> = s[open + 1..open + close]
                    .split_whitespace()
                    .filter_map(|t| t.parse().ok())
                    .collect();
                if let [x0, y0, x1, y1] = nums.as_slice() {
                    let size = (x1 - x0, y1 - y0);
                    if !out.contains(&size) {
                        out.push(size);
                    }
                }
            }
        }
        i = at;
    }
    out
}

fn fonts(blob: &[u8]) -> Vec<String> {
    let needle = b"/BaseFont";
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while let Some(pos) = find(&blob[i..], needle) {
        let at = i + pos + needle.len();
        let rest = &blob[at..blob.len().min(at + 128)];
        let s = String::from_utf8_lossy(rest);
        let s = s.trim_start();
        if let Some(stripped) = s.strip_prefix('/') {
            let name: String = stripped
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || "+-_.,".contains(*c))
                .collect();
            let name = strip_subset_prefix(&name);
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
        i = at;
    }
    out.sort();
    out
}

/// `AYABNL+DejaVuSerif` → `DejaVuSerif`.
///
/// The prefix is six uppercase letters and a plus, and it is different on
/// every run. See the module note.
pub fn strip_subset_prefix(name: &str) -> String {
    if let Some((prefix, rest)) = name.split_once('+') {
        if prefix.len() == 6 && prefix.chars().all(|c| c.is_ascii_uppercase()) {
            return rest.to_owned();
        }
    }
    name.to_owned()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// A minimal raw-DEFLATE/zlib reader.
///
/// Only the stored and fixed-Huffman cases plus dynamic Huffman are needed to
/// read what libtexpdf writes; rather than carry a compression dependency into
/// the test kit, this returns `None` on anything it cannot handle and the
/// caller simply learns less from that stream.
fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    // zlib header: 0x78 followed by a byte making the pair a multiple of 31.
    if data.len() < 2 || data[0] & 0x0f != 8 {
        return None;
    }
    let check = (u16::from(data[0]) << 8) | u16::from(data[1]);
    if check % 31 != 0 {
        return None;
    }
    miniz::inflate(&data[2..])
}

/// A small inflate implementation, sufficient for PDF object streams.
mod miniz {
    struct Bits<'a> {
        data: &'a [u8],
        pos: usize,
        bit: u32,
        acc: u32,
    }

    impl<'a> Bits<'a> {
        fn new(data: &'a [u8]) -> Self {
            Bits {
                data,
                pos: 0,
                bit: 0,
                acc: 0,
            }
        }

        fn need(&mut self, n: u32) -> Option<()> {
            while self.bit < n {
                let byte = *self.data.get(self.pos)?;
                self.pos += 1;
                self.acc |= u32::from(byte) << self.bit;
                self.bit += 8;
            }
            Some(())
        }

        fn take(&mut self, n: u32) -> Option<u32> {
            if n == 0 {
                return Some(0);
            }
            self.need(n)?;
            let v = self.acc & ((1 << n) - 1);
            self.acc >>= n;
            self.bit -= n;
            Some(v)
        }

        fn align(&mut self) {
            let drop = self.bit % 8;
            self.acc >>= drop;
            self.bit -= drop;
        }
    }

    struct Huffman {
        counts: [u16; 16],
        symbols: Vec<u16>,
    }

    impl Huffman {
        fn new(lengths: &[u8]) -> Huffman {
            let mut counts = [0u16; 16];
            for &l in lengths {
                counts[l as usize] += 1;
            }
            counts[0] = 0;
            let mut offs = [0u16; 16];
            for i in 1..16 {
                offs[i] = offs[i - 1] + counts[i - 1];
            }
            let mut symbols = vec![0u16; lengths.len()];
            for (sym, &l) in lengths.iter().enumerate() {
                if l != 0 {
                    symbols[offs[l as usize] as usize] = sym as u16;
                    offs[l as usize] += 1;
                }
            }
            Huffman { counts, symbols }
        }

        fn decode(&self, b: &mut Bits<'_>) -> Option<u16> {
            let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
            for len in 1..16 {
                code |= b.take(1)? as i32;
                let count = i32::from(self.counts[len]);
                if code - count < first {
                    return self.symbols.get((index + (code - first)) as usize).copied();
                }
                index += count;
                first = (first + count) << 1;
                code <<= 1;
            }
            None
        }
    }

    const LEN_BASE: [u16; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LEN_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DIST_BASE: [u16; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DIST_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    pub fn inflate(data: &[u8]) -> Option<Vec<u8>> {
        let mut b = Bits::new(data);
        let mut out: Vec<u8> = Vec::new();

        loop {
            let last = b.take(1)?;
            match b.take(2)? {
                0 => {
                    b.align();
                    let len = b.take(16)? as usize;
                    let _nlen = b.take(16)?;
                    for _ in 0..len {
                        out.push(b.take(8)? as u8);
                    }
                }
                1 => {
                    let mut lengths = [0u8; 288];
                    for (i, l) in lengths.iter_mut().enumerate() {
                        *l = match i {
                            0..=143 => 8,
                            144..=255 => 9,
                            256..=279 => 7,
                            _ => 8,
                        };
                    }
                    let lit = Huffman::new(&lengths);
                    let dist = Huffman::new(&[5u8; 30]);
                    block(&mut b, &lit, &dist, &mut out)?;
                }
                2 => {
                    let hlit = b.take(5)? as usize + 257;
                    let hdist = b.take(5)? as usize + 1;
                    let hclen = b.take(4)? as usize + 4;
                    const ORDER: [usize; 19] = [
                        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
                    ];
                    let mut cl = [0u8; 19];
                    for &o in ORDER.iter().take(hclen) {
                        cl[o] = b.take(3)? as u8;
                    }
                    let clh = Huffman::new(&cl);
                    let mut lengths = vec![0u8; hlit + hdist];
                    let mut i = 0;
                    while i < lengths.len() {
                        let sym = clh.decode(&mut b)?;
                        match sym {
                            0..=15 => {
                                lengths[i] = sym as u8;
                                i += 1;
                            }
                            16 => {
                                let prev = *lengths.get(i.checked_sub(1)?)?;
                                for _ in 0..3 + b.take(2)? {
                                    *lengths.get_mut(i)? = prev;
                                    i += 1;
                                }
                            }
                            17 => i += 3 + b.take(3)? as usize,
                            18 => i += 11 + b.take(7)? as usize,
                            _ => return None,
                        }
                    }
                    if i > lengths.len() {
                        return None;
                    }
                    let lit = Huffman::new(&lengths[..hlit]);
                    let dist = Huffman::new(&lengths[hlit..]);
                    block(&mut b, &lit, &dist, &mut out)?;
                }
                _ => return None,
            }
            if last == 1 {
                break;
            }
            if out.len() > 64 * 1024 * 1024 {
                return None;
            }
        }
        Some(out)
    }

    fn block(b: &mut Bits<'_>, lit: &Huffman, dist: &Huffman, out: &mut Vec<u8>) -> Option<()> {
        loop {
            let sym = lit.decode(b)?;
            match sym {
                0..=255 => out.push(sym as u8),
                256 => return Some(()),
                257..=285 => {
                    let i = sym as usize - 257;
                    let len = LEN_BASE[i] as usize + b.take(u32::from(LEN_EXTRA[i]))? as usize;
                    let ds = dist.decode(b)? as usize;
                    if ds >= 30 {
                        return None;
                    }
                    let d = DIST_BASE[ds] as usize + b.take(u32::from(DIST_EXTRA[ds]))? as usize;
                    if d > out.len() {
                        return None;
                    }
                    let start = out.len() - d;
                    for k in 0..len {
                        let byte = out[start + k];
                        out.push(byte);
                    }
                }
                _ => return None,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Where the ink is
//
// [`Pdf`] above answers questions about the file. What follows answers
// questions about the *page*: which glyphs were set, at what size, and where.
//
// It exists because the defects this project has actually shipped were all of
// that kind and none of them were visible from the file's structure. Lines that
// ran past the measure, and notes typeset over the last lines of a column, both
// produced a PDF with the right page count, the right page size and every font
// embedded. Only the coordinates said anything was wrong.
//
// This is a reader for what SILE writes and not a PDF library. It handles the
// operators libtexpdf emits — `Tf`, `Td`, `TD`, `Tm`, `BT`, and hex strings
// inside `Tj` and `TJ` — and ignores the rest of the imaging model, including
// `T*`, `'`, `"`, and text rendering matrices other than the translation.
// ---------------------------------------------------------------------------

/// One run of text, as it was placed.
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    /// 1-based, in reading order.
    pub page: usize,
    pub x: f64,
    /// **Top-down, and negative.** SILE flips the page transform, so the top of
    /// the page is 0 and further down is further negative. Comparisons read the
    /// other way round from PDF's own convention: `a.y < b.y` means a is lower.
    pub y: f64,
    /// Points, as the `Tf` operator gave it. This is how the note area is told
    /// apart from the body without knowing the frame geometry.
    pub size: f64,
    /// The characters, through the font's `/ToUnicode` map. A glyph the map
    /// does not cover becomes `U+FFFD`, so a broken subset is visible rather
    /// than silently short.
    pub text: String,
}

/// One baseline's worth of marks, in the order they were placed.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub page: usize,
    pub y: f64,
    pub marks: Vec<Mark>,
}

impl Line {
    /// The line's text, with nothing between the runs.
    ///
    /// SILE places each word as its own run and spaces them by moving the pen,
    /// so there are no space glyphs to recover — `"in the beginning"` comes back
    /// as `"inthebeginning"`. Assert with [`str::contains`] on a word, or on a
    /// run of words spelled the same way.
    pub fn text(&self) -> String {
        self.marks.iter().map(|m| m.text.as_str()).collect()
    }

    /// The sizes on this line, so a note line can be told from a body line.
    pub fn sizes(&self) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        for m in &self.marks {
            if !out.contains(&m.size) {
                out.push(m.size);
            }
        }
        out
    }

    pub fn left(&self) -> f64 {
        self.marks.iter().map(|m| m.x).fold(f64::MAX, f64::min)
    }
}

impl Pdf {
    /// Every mark in the file, in page order and then in placement order.
    pub fn marks(raw: &[u8]) -> Vec<Mark> {
        let objects = objects(raw);
        let mut out = Vec::new();
        for (index, page) in page_order(&objects).into_iter().enumerate() {
            let Some(body) = objects.get(&page) else {
                continue;
            };
            let fonts = page_fonts(&objects, body);
            let Some(content) = reference(body, b"/Contents").and_then(|n| stream(&objects, n))
            else {
                continue;
            };
            read_content(&content, &fonts, index + 1, &mut out);
        }
        out
    }

    /// The same, gathered into baselines.
    ///
    /// Two marks are on the same line when they share a `y` exactly, which is
    /// what SILE produces for a line of type: every run on it is placed from the
    /// same baseline.
    pub fn lines(raw: &[u8]) -> Vec<Line> {
        let mut out: Vec<Line> = Vec::new();
        for mark in Pdf::marks(raw) {
            match out
                .iter_mut()
                .find(|l| l.page == mark.page && l.y.to_bits() == mark.y.to_bits())
            {
                Some(line) => line.marks.push(mark),
                None => out.push(Line {
                    page: mark.page,
                    y: mark.y,
                    marks: vec![mark],
                }),
            }
        }
        // Down the page within a page, which is what a reader would expect from
        // something called `lines` and is not the order the operators arrive in.
        out.sort_by(|a, b| {
            a.page
                .cmp(&b.page)
                .then(b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal))
        });
        out
    }
}

/// Every indirect object, including the ones inside object streams.
fn objects(raw: &[u8]) -> BTreeMap<u32, Vec<u8>> {
    let mut out: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    let mut i = 0;
    while let Some(pos) = find(&raw[i..], b" obj") {
        let at = i + pos;
        // Back up over `N G ` to the object number.
        let head = &raw[..at];
        let start = head
            .iter()
            .rposition(|c| *c == b'\n' || *c == b'\r')
            .map_or(0, |p| p + 1);
        let words: Vec<&[u8]> = head[start..].split(|c| *c == b' ').collect();
        // Both the number and the generation, or a `" obj"` that happened to
        // land inside a compressed stream would overwrite a real object.
        let number = match words.as_slice() {
            [n, g] if ascii_u32(g).is_some() => ascii_u32(n),
            _ => None,
        };
        if let Some(number) = number {
            if let Some(end) = find(&raw[at..], b"endobj") {
                out.insert(number, raw[at + b" obj".len()..at + end].to_vec());
            }
        }
        i = at + b" obj".len();
    }

    // Object streams hold the page and font dictionaries, so nothing above is
    // visible until they are unpacked.
    for body in out.clone().values() {
        if find(body, b"/ObjStm").is_none() {
            continue;
        }
        let (Some(count), Some(first), Some(data)) = (
            integer(body, b"/N"),
            integer(body, b"/First"),
            inflate_body(body),
        ) else {
            continue;
        };
        let header: Vec<u32> = String::from_utf8_lossy(&data[..first.min(data.len())])
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        for pair in 0..count {
            let Some(&number) = header.get(pair * 2) else {
                break;
            };
            let Some(&offset) = header.get(pair * 2 + 1) else {
                break;
            };
            let start = first + offset as usize;
            let end = header
                .get(pair * 2 + 3)
                .map_or(data.len(), |o| first + *o as usize);
            if start <= end && end <= data.len() {
                out.insert(number, data[start..end].to_vec());
            }
        }
    }
    out
}

/// The pages, in reading order, by walking the page tree from the catalog.
///
/// Not by sorting object numbers, which happens to work for a SILE document
/// written straight through and would quietly stop working for one that is not.
fn page_order(objects: &BTreeMap<u32, Vec<u8>>) -> Vec<u32> {
    let root = objects
        .iter()
        .find(|(_, b)| find(b, b"/Type/Catalog").is_some() || find(b, b"/Type /Catalog").is_some())
        .and_then(|(_, b)| reference(b, b"/Pages"));
    let mut out = Vec::new();
    if let Some(root) = root {
        walk_pages(objects, root, &mut out, 0);
    }
    out
}

fn walk_pages(objects: &BTreeMap<u32, Vec<u8>>, node: u32, out: &mut Vec<u32>, depth: usize) {
    // A malformed file could make the tree a cycle; a page tree deeper than
    // this is not one this reader is going to make sense of anyway.
    if depth > 32 {
        return;
    }
    let Some(body) = objects.get(&node) else {
        return;
    };
    if let Some(kids) = between(body, b"/Kids", b'[', b']') {
        for child in references_in(&kids) {
            walk_pages(objects, child, out, depth + 1);
        }
    } else {
        out.push(node);
    }
}

/// `/Font << /F1 5 0 R >>` on a page's resources, each with its decoded map.
fn page_fonts(objects: &BTreeMap<u32, Vec<u8>>, page: &[u8]) -> BTreeMap<String, ToUnicode> {
    let mut out = BTreeMap::new();
    let Some(resources) = reference(page, b"/Resources").and_then(|n| objects.get(&n)) else {
        return out;
    };
    let Some(fonts) = between(resources, b"/Font", b'<', b'>') else {
        return out;
    };
    let text = String::from_utf8_lossy(&fonts).into_owned();
    let mut rest = text.as_str();
    while let Some(at) = rest.find('/') {
        rest = &rest[at + 1..];
        let name: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
        let after = &rest[name.len()..];
        let number: Option<u32> = after.split_whitespace().next().and_then(|w| w.parse().ok());
        if let Some(number) = number {
            let map = objects
                .get(&number)
                .and_then(|f| reference(f, b"/ToUnicode"))
                .and_then(|n| stream(objects, n))
                .map(|cmap| to_unicode(&cmap))
                .unwrap_or_default();
            out.insert(name, map);
        }
    }
    out
}

type ToUnicode = BTreeMap<u32, String>;

/// A `/ToUnicode` CMap, as far as `bfchar` and `bfrange` go.
fn to_unicode(cmap: &[u8]) -> ToUnicode {
    let text = String::from_utf8_lossy(cmap);
    let mut out = BTreeMap::new();

    for block in sections(&text, "beginbfchar", "endbfchar") {
        let codes = hex_groups(block);
        for pair in codes.chunks(2) {
            if let [src, dst] = pair {
                if let Some(code) = hex_u32(src) {
                    out.insert(code, utf16_be(dst));
                }
            }
        }
    }
    for block in sections(&text, "beginbfrange", "endbfrange") {
        let codes = hex_groups(block);
        for triple in codes.chunks(3) {
            if let [lo, hi, dst] = triple {
                let (Some(lo), Some(hi), Some(base)) = (hex_u32(lo), hex_u32(hi), hex_u32(dst))
                else {
                    continue;
                };
                for (step, code) in (lo..=hi).enumerate() {
                    if let Some(ch) = char::from_u32(base + step as u32) {
                        out.insert(code, ch.to_string());
                    }
                }
            }
        }
    }
    out
}

fn sections<'a>(text: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        let after = &rest[start + open.len()..];
        let Some(end) = after.find(close) else { break };
        out.push(&after[..end]);
        rest = &after[end..];
    }
    out
}

fn hex_groups(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('>') else { break };
        out.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    out
}

fn hex_u32(text: &str) -> Option<u32> {
    u32::from_str_radix(text.trim(), 16).ok()
}

/// A CMap destination is UTF-16BE, and may be a surrogate pair or a sequence.
fn utf16_be(hex: &str) -> String {
    let units: Vec<u16> = hex
        .as_bytes()
        .chunks(4)
        .filter_map(|c| u16::from_str_radix(&String::from_utf8_lossy(c), 16).ok())
        .collect();
    String::from_utf16_lossy(&units)
}

fn read_content(
    data: &[u8],
    fonts: &BTreeMap<String, ToUnicode>,
    page: usize,
    out: &mut Vec<Mark>,
) {
    let text = String::from_utf8_lossy(data);
    let words = Tokens { rest: &text };
    let mut stack: Vec<&str> = Vec::new();
    let empty = ToUnicode::new();
    let mut map = &empty;
    let (mut size, mut x, mut y) = (0.0f64, 0.0f64, 0.0f64);

    for token in words {
        match token {
            "Tf" => {
                if let [name, points] = tail(&stack, 2) {
                    size = points.parse().unwrap_or(size);
                    map = fonts.get(name.trim_start_matches('/')).unwrap_or(&empty);
                }
            }
            // Both are a translation of the text line matrix; SILE emits one
            // per `BT`, so treating them as absolute and relative alike is the
            // same answer here and the relative reading is the safer one.
            "Td" | "TD" => {
                if let [dx, dy] = tail(&stack, 2) {
                    x += dx.parse().unwrap_or(0.0);
                    y += dy.parse().unwrap_or(0.0);
                }
            }
            "Tm" => {
                if let [.., ex, ey] = tail(&stack, 6) {
                    x = ex.parse().unwrap_or(x);
                    y = ey.parse().unwrap_or(y);
                }
            }
            "BT" => {
                x = 0.0;
                y = 0.0;
            }
            _ => {
                if let Some(hex) = token.strip_prefix('<').and_then(|t| t.strip_suffix('>')) {
                    let decoded: String = hex
                        .as_bytes()
                        .chunks(4)
                        .filter_map(|c| hex_u32(&String::from_utf8_lossy(c)))
                        .map(|code| map.get(&code).cloned().unwrap_or('\u{fffd}'.to_string()))
                        .collect();
                    out.push(Mark {
                        page,
                        x,
                        y,
                        size,
                        text: decoded,
                    });
                }
            }
        }
        stack.push(token);
        if stack.len() > 8 {
            stack.remove(0);
        }
    }
}

/// The last `n` tokens, or fewer — an operator given too few operands simply
/// fails to match the pattern the caller destructures with.
fn tail<'s, 'a>(stack: &'s [&'a str], n: usize) -> &'s [&'a str] {
    &stack[stack.len().saturating_sub(n)..]
}

/// Content-stream tokens: hex strings whole, everything else on whitespace and
/// on the array brackets that wrap a `TJ`.
struct Tokens<'a> {
    rest: &'a str,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        self.rest = self
            .rest
            .trim_start_matches(|c: char| c.is_whitespace() || c == '[' || c == ']');
        if self.rest.is_empty() {
            return None;
        }
        let end = if self.rest.starts_with('<') {
            self.rest.find('>').map_or(self.rest.len(), |i| i + 1)
        } else {
            self.rest
                .find(|c: char| c.is_whitespace() || c == '[' || c == ']' || c == '<')
                .unwrap_or(self.rest.len())
        };
        let (token, rest) = self.rest.split_at(end);
        self.rest = rest;
        Some(token)
    }
}

/// `/Key 12 0 R` → `12`, and `/Key[12 0 R]` too.
fn reference(body: &[u8], key: &[u8]) -> Option<u32> {
    let at = find(body, key)? + key.len();
    let text = String::from_utf8_lossy(&body[at..body.len().min(at + 64)]).into_owned();
    digits(text.trim_start().trim_start_matches('['))
}

/// Every `N 0 R` inside a slice, in order.
fn references_in(body: &[u8]) -> Vec<u32> {
    let text = String::from_utf8_lossy(body);
    let words: Vec<&str> = text.split_whitespace().collect();
    words
        .windows(3)
        .filter(|w| w[2] == "R")
        .filter_map(|w| w[0].parse().ok())
        .collect()
}

/// `/Key 42` → `42`.
fn integer(body: &[u8], key: &[u8]) -> Option<usize> {
    let at = find(body, key)? + key.len();
    let text = String::from_utf8_lossy(&body[at..body.len().min(at + 32)]).into_owned();
    digits(text.trim_start())
}

/// The leading run of digits, and nothing after it.
///
/// Not `split_whitespace`, because PDF puts no space before the next key:
/// `/N 111/First 907` gives `111/First` to anything that splits on space, and
/// that parses as nothing at all.
fn digits<T: std::str::FromStr>(text: &str) -> Option<T> {
    let run: String = text.chars().take_while(char::is_ascii_digit).collect();
    run.parse().ok()
}

/// The balanced `open`…`close` run that follows `key`.
fn between(body: &[u8], key: &[u8], open: u8, close: u8) -> Option<Vec<u8>> {
    let at = find(body, key)? + key.len();
    let start = body[at..].iter().position(|c| *c == open)? + at;
    let mut depth = 0i32;
    for (i, c) in body[start..].iter().enumerate() {
        if *c == open {
            depth += 1;
        } else if *c == close {
            depth -= 1;
            if depth == 0 {
                return Some(body[start + 1..start + i].to_vec());
            }
        }
    }
    None
}

/// The decoded stream of an object, if it has one.
fn stream(objects: &BTreeMap<u32, Vec<u8>>, number: u32) -> Option<Vec<u8>> {
    inflate_body(objects.get(&number)?)
}

fn inflate_body(body: &[u8]) -> Option<Vec<u8>> {
    let at = find(body, b"stream")? + b"stream".len();
    let mut start = at;
    if body.get(start) == Some(&b'\r') {
        start += 1;
    }
    if body.get(start) == Some(&b'\n') {
        start += 1;
    }
    let end = start + find(&body[start..], b"endstream")?;
    let data = &body[start..end];
    match find(&body[..at], b"/FlateDecode") {
        Some(_) => inflate(data),
        None => Some(data.to_vec()),
    }
}

fn ascii_u32(word: &[u8]) -> Option<u32> {
    let text = std::str::from_utf8(word).ok()?;
    text.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F-15, as a unit test: the subset prefix is what varies between runs and
    /// it must be stripped before comparison.
    #[test]
    fn subset_prefixes_are_stripped() {
        assert_eq!(strip_subset_prefix("AYABNL+DejaVuSerif"), "DejaVuSerif");
        assert_eq!(strip_subset_prefix("HQTCEM+DejaVuSerif"), "DejaVuSerif");
        // Not a subset prefix: wrong length, or lowercase.
        assert_eq!(strip_subset_prefix("ABC+Font"), "ABC+Font");
        assert_eq!(strip_subset_prefix("abcdef+Font"), "abcdef+Font");
        assert_eq!(strip_subset_prefix("DejaVuSerif"), "DejaVuSerif");
    }

    #[test]
    fn two_runs_differing_only_in_subset_tag_compare_equal() {
        let a = b"%PDF-1.5\n/BaseFont /AYABNL+DejaVuSerif\n/Type /Page\n";
        let b = b"%PDF-1.5\n/BaseFont /HQTCEM+DejaVuSerif\n/Type /Page\n";
        let (pa, pb) = (Pdf::parse(a), Pdf::parse(b));
        assert_eq!(pa.fonts, pb.fonts);
        assert_eq!(pa.fonts, vec!["DejaVuSerif".to_owned()]);
    }

    #[test]
    fn reads_page_count_and_geometry() {
        let raw = b"%PDF-1.5\n/Type /Page\n/MediaBox [0 0 432 648]\n/Type /Page\n/MediaBox [0 0 432 648]\n";
        let pdf = Pdf::parse(raw);
        assert_eq!(pdf.pages, 2);
        let (w, h) = pdf.uniform_page_size_inches().expect("one page size");
        assert!((w - 6.0).abs() < 0.01, "width was {w}");
        assert!((h - 9.0).abs() < 0.01, "height was {h}");
    }

    #[test]
    fn pages_does_not_count_the_page_tree_node() {
        let raw = b"%PDF-1.5\n/Type /Pages /Count 3\n/Type /Page\n";
        assert_eq!(Pdf::parse(raw).pages, 1);
    }

    #[test]
    fn inflates_a_deflated_stream() {
        // "hello hello hello" stored uncompressed inside a zlib wrapper.
        let payload = b"hello hello hello";
        let mut z = vec![0x78, 0x01];
        z.push(0x01); // final, stored
        z.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(payload.len() as u16)).to_le_bytes());
        z.extend_from_slice(payload);

        let mut raw = b"%PDF-1.5\n1 0 obj\n<</Filter/FlateDecode>>\nstream\n".to_vec();
        raw.extend_from_slice(&z);
        raw.extend_from_slice(b"\nendstream\nendobj\n");

        let objects = objects(&raw);
        assert_eq!(
            inflate_body(objects.get(&1).expect("object 1")),
            Some(payload.to_vec())
        );
    }

    /// `/N 111/First 907` — no space before the next key, which is legal and is
    /// what libtexpdf writes.
    #[test]
    fn a_number_ends_where_the_next_key_begins() {
        assert_eq!(
            integer(b"<</Type/ObjStm/N 111/First 907>>", b"/N"),
            Some(111)
        );
        assert_eq!(reference(b"<</Contents[8 0 R]>>", b"/Contents"), Some(8));
        assert_eq!(
            reference(b"<</Pages 158 0 R/Type/Catalog>>", b"/Pages"),
            Some(158)
        );
    }
}
