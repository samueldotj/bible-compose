//! The canon table — book codes, canonical order, testament.
//!
//! **Data, not code.** SRS §19 left open whether deuterocanonical books belong
//! in the built-in ordering; [SRS-REVIEW] closed it as yes, on the grounds
//! that the table is rows and the alternative makes the canon a schema
//! property. That decision is what this file looks like: adding a book is a
//! line, and BOOK-002's project-configured ordering is a permutation of it
//! rather than a special case.
//!
//! [SRS-REVIEW]: ../../../docs/SRS-REVIEW.md

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Testament {
    Old,
    New,
    Deuterocanon,
}

/// A canonical three-character Scripture identifier.
///
/// Constructed only through [`BookCode::parse`], so an arbitrary string cannot
/// masquerade as a book code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BookCode(&'static str);

impl Serialize for BookCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.0)
    }
}

/// Deserialization goes through [`BookCode::parse`], so a code absent from the
/// table cannot enter the model by the back door.
impl<'de> Deserialize<'de> for BookCode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<BookCode, D::Error> {
        let s = String::deserialize(d)?;
        BookCode::parse(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown book code {s:?}")))
    }
}

impl BookCode {
    pub fn parse(s: &str) -> Option<BookCode> {
        let upper = s.trim().to_ascii_uppercase();
        TABLE
            .iter()
            .find(|e| e.code == upper)
            .map(|e| BookCode(e.code))
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }

    fn entry(self) -> &'static Entry {
        TABLE
            .iter()
            .find(|e| e.code == self.0)
            .expect("BookCode is only constructed from the table")
    }

    /// Position in canonical order (BOOK-001).
    pub fn order(self) -> u16 {
        self.entry().order
    }

    pub fn testament(self) -> Testament {
        self.entry().testament
    }

    /// The English name, used only as a fallback when a project supplies no
    /// `\h` or `\toc` — never in place of one.
    pub fn english_name(self) -> &'static str {
        self.entry().name
    }

    pub fn all() -> impl Iterator<Item = BookCode> {
        TABLE.iter().map(|e| BookCode(e.code))
    }
}

impl PartialOrd for BookCode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BookCode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order().cmp(&other.order())
    }
}

impl std::fmt::Display for BookCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

struct Entry {
    order: u16,
    code: &'static str,
    name: &'static str,
    testament: Testament,
}

macro_rules! table {
    ($( $order:literal $code:literal $name:literal $t:ident );* $(;)?) => {
        static TABLE: &[Entry] = &[
            $( Entry { order: $order, code: $code, name: $name, testament: Testament::$t } ),*
        ];
    };
}

use Testament::{Deuterocanon as D, New as N, Old as O};
#[allow(clippy::enum_glob_use)]
const _: () = {
    let _ = (O, N, D);
};

table! {
    1 "GEN" "Genesis" Old;              2 "EXO" "Exodus" Old;
    3 "LEV" "Leviticus" Old;            4 "NUM" "Numbers" Old;
    5 "DEU" "Deuteronomy" Old;          6 "JOS" "Joshua" Old;
    7 "JDG" "Judges" Old;               8 "RUT" "Ruth" Old;
    9 "1SA" "1 Samuel" Old;            10 "2SA" "2 Samuel" Old;
   11 "1KI" "1 Kings" Old;             12 "2KI" "2 Kings" Old;
   13 "1CH" "1 Chronicles" Old;        14 "2CH" "2 Chronicles" Old;
   15 "EZR" "Ezra" Old;                16 "NEH" "Nehemiah" Old;
   17 "EST" "Esther" Old;              18 "JOB" "Job" Old;
   19 "PSA" "Psalms" Old;              20 "PRO" "Proverbs" Old;
   21 "ECC" "Ecclesiastes" Old;        22 "SNG" "Song of Songs" Old;
   23 "ISA" "Isaiah" Old;              24 "JER" "Jeremiah" Old;
   25 "LAM" "Lamentations" Old;        26 "EZK" "Ezekiel" Old;
   27 "DAN" "Daniel" Old;              28 "HOS" "Hosea" Old;
   29 "JOL" "Joel" Old;                30 "AMO" "Amos" Old;
   31 "OBA" "Obadiah" Old;             32 "JON" "Jonah" Old;
   33 "MIC" "Micah" Old;               34 "NAM" "Nahum" Old;
   35 "HAB" "Habakkuk" Old;            36 "ZEP" "Zephaniah" Old;
   37 "HAG" "Haggai" Old;              38 "ZEC" "Zechariah" Old;
   39 "MAL" "Malachi" Old;

   40 "MAT" "Matthew" New;             41 "MRK" "Mark" New;
   42 "LUK" "Luke" New;                43 "JHN" "John" New;
   44 "ACT" "Acts" New;                45 "ROM" "Romans" New;
   46 "1CO" "1 Corinthians" New;       47 "2CO" "2 Corinthians" New;
   48 "GAL" "Galatians" New;           49 "EPH" "Ephesians" New;
   50 "PHP" "Philippians" New;         51 "COL" "Colossians" New;
   52 "1TH" "1 Thessalonians" New;     53 "2TH" "2 Thessalonians" New;
   54 "1TI" "1 Timothy" New;           55 "2TI" "2 Timothy" New;
   56 "TIT" "Titus" New;               57 "PHM" "Philemon" New;
   58 "HEB" "Hebrews" New;             59 "JAS" "James" New;
   60 "1PE" "1 Peter" New;             61 "2PE" "2 Peter" New;
   62 "1JN" "1 John" New;              63 "2JN" "2 John" New;
   64 "3JN" "3 John" New;              65 "JUD" "Jude" New;
   66 "REV" "Revelation" New;

   67 "TOB" "Tobit" Deuterocanon;      68 "JDT" "Judith" Deuterocanon;
   69 "ESG" "Esther Greek" Deuterocanon;
   70 "WIS" "Wisdom of Solomon" Deuterocanon;
   71 "SIR" "Sirach" Deuterocanon;     72 "BAR" "Baruch" Deuterocanon;
   73 "LJE" "Letter of Jeremiah" Deuterocanon;
   74 "S3Y" "Song of the Three Young Men" Deuterocanon;
   75 "SUS" "Susanna" Deuterocanon;    76 "BEL" "Bel and the Dragon" Deuterocanon;
   77 "1MA" "1 Maccabees" Deuterocanon;
   78 "2MA" "2 Maccabees" Deuterocanon;
   79 "3MA" "3 Maccabees" Deuterocanon;
   80 "4MA" "4 Maccabees" Deuterocanon;
   81 "1ES" "1 Esdras" Deuterocanon;   82 "2ES" "2 Esdras" Deuterocanon;
   83 "MAN" "Prayer of Manasseh" Deuterocanon;
   84 "PS2" "Psalm 151" Deuterocanon;
}

/// Sort into canonical order (BOOK-001), regardless of filesystem order.
pub fn sort_canonical(books: &mut [BookCode]) {
    books.sort_by_key(|b| b.order());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_codes_case_insensitively() {
        assert_eq!(BookCode::parse("MAT").unwrap().as_str(), "MAT");
        assert_eq!(BookCode::parse("mat").unwrap().as_str(), "MAT");
        assert_eq!(BookCode::parse(" jhn ").unwrap().as_str(), "JHN");
        assert!(BookCode::parse("XYZ").is_none());
    }

    #[test]
    fn protestant_canon_is_complete_and_in_order() {
        let protestant: Vec<_> = BookCode::all()
            .filter(|b| b.testament() != Testament::Deuterocanon)
            .collect();
        assert_eq!(protestant.len(), 66);
        assert_eq!(protestant.first().unwrap().as_str(), "GEN");
        assert_eq!(protestant.last().unwrap().as_str(), "REV");
    }

    #[test]
    fn deuterocanon_is_present_and_ordered_after() {
        let deutero: Vec<_> = BookCode::all()
            .filter(|b| b.testament() == Testament::Deuterocanon)
            .collect();
        assert!(
            !deutero.is_empty(),
            "the table carries deuterocanonical books"
        );
        assert!(deutero.iter().all(|d| d.order() > 66));
    }

    #[test]
    fn orders_and_codes_are_unique_and_contiguous() {
        let mut orders: Vec<u16> = BookCode::all().map(|b| b.order()).collect();
        orders.sort_unstable();
        for (i, o) in orders.iter().enumerate() {
            assert_eq!(
                *o as usize,
                i + 1,
                "canonical order must be contiguous from 1"
            );
        }
        let mut codes: Vec<&str> = BookCode::all().map(|b| b.as_str()).collect();
        codes.sort_unstable();
        let before = codes.len();
        codes.dedup();
        assert_eq!(before, codes.len(), "duplicate book code");
    }

    /// BOOK-001: canonical order, not filesystem order.
    #[test]
    fn sorts_canonically_not_alphabetically() {
        let mut books = vec![
            BookCode::parse("EXO").unwrap(),
            BookCode::parse("GEN").unwrap(),
            BookCode::parse("ACT").unwrap(),
        ];
        sort_canonical(&mut books);
        let order: Vec<_> = books.iter().map(|b| b.as_str()).collect();
        assert_eq!(order, ["GEN", "EXO", "ACT"]);
        // Alphabetically ACT would come first; canonically it does not.
        assert_ne!(order[0], "ACT");
    }

    #[test]
    fn every_code_is_three_characters() {
        for b in BookCode::all() {
            assert_eq!(
                b.as_str().len(),
                3,
                "{} is not a 3-character code",
                b.as_str()
            );
        }
    }
}
