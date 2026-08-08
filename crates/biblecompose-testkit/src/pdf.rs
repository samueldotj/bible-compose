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
    pub fn parse(raw: &[u8]) -> Pdf {
        let mut blob = raw.to_vec();
        for chunk in inflate_streams(raw) {
            blob.extend_from_slice(&chunk);
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

/// Inflate every zlib stream in the file, skipping the ones that are not.
fn inflate_streams(raw: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(pos) = find(&raw[i..], b"stream") {
        let mut start = i + pos + b"stream".len();
        if raw.get(start) == Some(&b'\r') {
            start += 1;
        }
        if raw.get(start) == Some(&b'\n') {
            start += 1;
        }
        let Some(end_rel) = find(&raw[start..], b"endstream") else {
            break;
        };
        let end = start + end_rel;
        if let Some(data) = inflate(&raw[start..end]) {
            out.push(data);
        }
        i = end;
    }
    out
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

        let mut raw = b"%PDF-1.5\nstream\n".to_vec();
        raw.extend_from_slice(&z);
        raw.extend_from_slice(b"endstream\n");

        let streams = inflate_streams(&raw);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0], payload);
    }
}
