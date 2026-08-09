//! P2.2 — one parse, a typed view derived from it, spans retained.

use biblecompose_config::ConfigDocument;
use biblecompose_diagnostics::Severity;

const SETTINGS: &str = "\
# BibleCompose settings.
schema_version = 1

[page]
size = \"6x9in\"     # trade paperback
width = \"6in\"
margin.inner = \"0.875in\"

[body]
font = \"Noto Serif\"
size = 10.5
leading = 13

[books]
order = [\"MAT\", \"MRK\", \"LUK\", \"JHN\"]
exclude = []
";

fn parse(source: &str) -> ConfigDocument {
    ConfigDocument::parse("biblecompose.toml", source.to_owned()).expect("the fixture parses")
}

// ---------------------------------------------------------------- CFG-003

/// A syntax error reports the file, the line and the column.
#[test]
fn a_syntax_error_has_a_position() {
    let bad = "[page]\nwidth = \"6in\"\nheight \"9in\"\n";
    let err = ConfigDocument::parse("biblecompose.toml", bad.to_owned())
        .expect_err("a missing `=` is a syntax error");

    assert_eq!(err.severity, Severity::Error);
    assert_eq!(err.code.as_str(), "CFG-001");

    let loc = err.location.as_ref().expect("CFG-003 requires a location");
    assert_eq!(loc.path, "biblecompose.toml");
    assert_eq!(loc.line, Some(3), "the error is on the third line");
    assert!(loc.column.is_some(), "CFG-003 requires a column too");
}

/// Our line and column are computed from the span rather than scraped out of
/// `toml_edit`'s rendering — so they are worth checking *against* that
/// rendering, which is the only other authority on where the error is.
#[test]
fn the_position_agrees_with_the_parsers_own_rendering() {
    let bad = "[page]\nwidth = \"6in\"\nheight \"9in\"\n";
    let err = ConfigDocument::parse("biblecompose.toml", bad.to_owned()).unwrap_err();
    let loc = err.location.as_ref().unwrap();
    let detail = err.detail.as_ref().expect("the rendered error is kept");

    assert!(
        detail.contains(&format!(
            "line {}, column {}",
            loc.line.unwrap(),
            loc.column.unwrap()
        )),
        "our position and the parser's disagree.\n  ours: {loc}\n  {detail}"
    );
}

/// An error on the first line must not underflow the line index.
#[test]
fn a_syntax_error_on_the_first_line_is_line_one() {
    let err = ConfigDocument::parse("s.toml", "= 1\n".to_owned()).unwrap_err();
    assert_eq!(err.location.unwrap().line, Some(1));
}

/// Windows line endings must not shift every line number by the count of
/// carriage returns above it.
#[test]
fn crlf_does_not_disturb_line_numbers() {
    let doc = parse(&SETTINGS.replace('\n', "\r\n"));
    let width = doc.find("page.width").expect("page.width is present");
    assert_eq!(width.loc().line, Some(6));
}

// ------------------------------------------------------- spans, retained

#[test]
fn every_read_knows_where_it_came_from() {
    let doc = parse(SETTINGS);

    let width = doc.find("page.width").unwrap().string().unwrap();
    assert_eq!(*width, "6in");
    // The key, not the value: it is what a person searches the file for.
    assert_eq!(width.loc.line, Some(6));
    assert_eq!(width.loc.column, Some(1));

    let size = doc.find("body.size").unwrap().number().unwrap();
    assert_eq!(*size, 10.5);
    assert_eq!(size.loc.line, Some(11));

    // A trailing comment on the line above must not drag the position with it.
    let font = doc.find("body.font").unwrap().string().unwrap();
    assert_eq!(font.loc.line, Some(10));
}

/// `margin.inner = …` inside `[page]` is a dotted key, and the position that
/// matters is the leaf's, not the table's.
#[test]
fn a_dotted_key_is_located_at_its_leaf() {
    let doc = parse(SETTINGS);
    let inner = doc.find("page.margin.inner").unwrap().string().unwrap();
    assert_eq!(*inner, "0.875in");
    assert_eq!(inner.loc.line, Some(7));
    assert_eq!(inner.loc.column, Some(8), "`inner` begins after `margin.`");
}

/// A bad element of an array is reported at the element, not at the array.
#[test]
fn array_elements_are_located_individually() {
    let doc = parse(SETTINGS);
    let order = doc.find("books.order").unwrap().array().unwrap();
    assert_eq!(order.len(), 4);

    assert_eq!(*order[0].string().unwrap(), "MAT");
    assert_eq!(order[0].dotted_path(), "books.order[0]");

    let luk = order[2].string().unwrap();
    assert_eq!(*luk, "LUK");
    assert_eq!(luk.loc.line, Some(15));
    assert_eq!(luk.loc.column, Some(24), "the third element, not the array");
}

/// Where the position is genuinely unknown it is absent, not `:0:0` — which
/// reads as a defect in the file rather than a gap in the tooling (ADR-005).
#[test]
fn the_root_has_a_file_but_no_line() {
    let doc = parse(SETTINGS);
    let loc = doc.root().loc();
    assert_eq!(loc.path, "biblecompose.toml");
    assert_eq!(loc.line, None);
    assert_eq!(loc.to_string(), "biblecompose.toml");
}

// ------------------------------------------------------------ typed view

#[test]
fn a_value_of_the_wrong_type_is_reported_where_it_was_written() {
    let doc = parse("[page]\ncolumns = true\n");
    let err = doc.find("page.columns").unwrap().integer().unwrap_err();

    assert_eq!(err.code.as_str(), "CFG-006");
    assert_eq!(err.severity, Severity::Error);
    assert_eq!(
        err.message,
        "`page.columns` is a boolean; expected an integer"
    );
    let loc = err.location.unwrap();
    assert_eq!((loc.line, loc.column), (Some(2), Some(1)));
}

/// `leading = 13` where a decimal is wanted. Rejecting it would be pedantry
/// with a diagnostic attached.
#[test]
fn an_integer_is_a_number() {
    let doc = parse(SETTINGS);
    assert_eq!(*doc.find("body.leading").unwrap().number().unwrap(), 13.0);
    // Not the reverse: 10.5 is not an integer, and silently truncating a
    // point size is worse than saying so.
    assert!(doc.find("body.size").unwrap().integer().is_err());
}

#[test]
fn a_missing_key_is_absent_rather_than_an_error() {
    let doc = parse(SETTINGS);
    assert!(doc.find("page.wdith").is_none());
    assert!(doc.find("nothing.here.at.all").is_none());
}

/// The one thing the typed view must not do is care how the author spelled a
/// table. `[page.margin]` and `margin = { … }` are the same document.
#[test]
fn a_standard_table_and_an_inline_table_read_alike() {
    let standard = parse("[page.margin]\ninner = \"0.875in\"\n");
    let inline = parse("[page]\nmargin = { inner = \"0.875in\" }\n");
    let dotted = parse("[page]\nmargin.inner = \"0.875in\"\n");

    for doc in [&standard, &inline, &dotted] {
        assert_eq!(
            *doc.find("page.margin.inner").unwrap().string().unwrap(),
            "0.875in"
        );
    }
}

#[test]
fn a_bad_element_does_not_hide_the_rest_of_the_array() {
    let doc = parse("[books]\norder = [\"MAT\", 3, \"LUK\", true]\n");
    let (values, errors) = doc.find("books.order").unwrap().string_array();

    assert_eq!(
        values.iter().map(|v| v.value.as_str()).collect::<Vec<_>>(),
        ["MAT", "LUK"]
    );
    assert_eq!(errors.len(), 2, "DIA-002: report both, not the first");
    assert!(errors.iter().all(|e| e.code.as_str() == "CFG-006"));
    assert!(errors[0].message.contains("books.order[1]"));
}

/// The raw material for CFG-004 at P2.5: stray keys, each at its own key.
#[test]
fn unknown_keys_come_back_with_their_positions() {
    let doc = parse(SETTINGS);
    let page = doc.find("page").unwrap().table().unwrap();

    assert_eq!(page.names(), ["size", "width", "margin"], "in file order");

    let stray = page.unknown_keys(&["size", "margin"]);
    assert_eq!(stray.len(), 1);
    assert_eq!(stray[0].dotted_path(), "page.width");
    assert_eq!(stray[0].loc().line, Some(6));
}

#[test]
fn a_table_read_as_a_value_says_so() {
    let doc = parse(SETTINGS);
    let err = doc.find("page").unwrap().string().unwrap_err();
    assert_eq!(err.message, "`page` is a table; expected a string");
}

// -------------------------------------------------- one parse, not two

/// CFG-006's foundation: what was read is what gets written. A file with
/// comments, alignment and an author's own key order survives the round trip
/// byte for byte — so a GUI edit at P2.7 can only disturb the key it edits.
#[test]
fn a_commented_file_round_trips_byte_for_byte() {
    let awkward = "\
# A heading comment.

  # An indented comment, in a file whose keys are not sorted.
[body]
size     = 10.5   # aligned on purpose
font     = 'Noto Serif'
leading  = 13

[page]   # the page table comes second, deliberately
width = \"6in\"


# Two blank lines above.
[books]
order = [
  \"MAT\",   # Matthew
  \"JHN\",
]
";

    let doc = parse(awkward);
    assert_eq!(doc.source(), awkward, "the source is kept verbatim");
    assert_eq!(
        doc.into_editable().to_string(),
        awkward,
        "a parse and a serialize must be the identity on an untouched file"
    );
}

/// The one divergence found, recorded rather than discovered later: a file
/// that does not end in a newline gains one.
///
/// Harmless — most editors add it and every diff tool complains about its
/// absence — but CFG-006 says "avoid rewriting unrelated formatting", so it is
/// pinned here. If P2.7's writer starts trimming it back, this test says so
/// rather than a user's `git diff`.
#[test]
fn a_file_without_a_trailing_newline_gains_one() {
    let doc = parse("[page]\nwidth = \"6in\"");
    assert_eq!(doc.into_editable().to_string(), "[page]\nwidth = \"6in\"\n");
}

/// The typed view and the format-preserving document are the same tree, so a
/// value the reader reports and a value the writer would emit cannot differ.
#[test]
fn the_typed_view_and_the_document_are_one_tree() {
    let doc = parse(SETTINGS);
    let read = doc.find("body.font").unwrap().string().unwrap().value;

    let mut editable = doc.into_editable();
    // Ask the writer for the same key, through the mutation API rather than
    // the read API.
    let written = editable["body"]["font"].as_str().unwrap().to_owned();
    assert_eq!(read, written);

    editable["body"]["font"] = toml_edit::value("Gentium Plus");
    assert!(editable.to_string().contains("font = \"Gentium Plus\""));
}
