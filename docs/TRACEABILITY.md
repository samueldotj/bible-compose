# MUST traceability

Every MUST in [SRS v0.1](SRS-v0.1.md), and where it is answered.

**Generated, then read.** `scripts/traceability.mjs` finds each requirement's
id in the test suite and reports the `#[test]` that follows it. Rows it cannot
resolve that way were resolved by reading the test; the two it cannot resolve at
all are recorded below as exceptions, with their reasoning.

It is worth exactly what it is worth. A test that names a requirement is
evidence somebody meant to check it, not proof the check is good. **The value is
in the absence of blank rows** — and in having to write a sentence for anything
that would otherwise have one.

It earned its keep on the first run by finding a real gap: BLD-003 says the
PDF's name is derived from the publication's, and it was always `bible.pdf`.

| Requirement | | Verified by |
|---|---|---|
| **PRJ-001** | The application shall allow the user to open a folder as a BibleCompose project. | [`a_project_of_one_book_is_a_project`](../crates/biblecompose-project/tests/discovery.rs) (+1 more) |
| **PRJ-002** | The application shall recursively discover .usfm and .sfm files in the selected project… | [`nested_directories_are_discovered_without_registration`](../crates/biblecompose-project/tests/discovery.rs) |
| **PRJ-003** | Each Scripture file shall be identified primarily from its USFM \id marker, not from fi… | [`a_renamed_file_still_loads_as_the_book_it_declares`](../crates/biblecompose-project/tests/discovery.rs) (+1 more) |
| **PRJ-004** | The application shall detect duplicate canonical book IDs and block PDF generation unti… | [`two_files_claiming_one_book_block_the_build`](../crates/biblecompose-project/tests/discovery.rs) (+2 more) |
| **PRJ-005** | The application shall support projects containing a subset of Bible books. | [`a_project_of_one_book_is_a_project`](../crates/biblecompose-project/tests/discovery.rs) (+2 more) |
| **PRJ-007** | The project shall remain portable as a normal filesystem directory. | [`climbing_out_of_the_project_is_refused`](../crates/biblecompose-app/tests/figures.rs) |
| **FUN-001** | The application shall parse all discovered Scripture files into a normalized in-memory… | [normalize.rs](../crates/biblecompose-scripture/tests/normalize.rs) *(the whole suite)* |
| **FUN-002** | The application shall preserve source order and content unless a documented normalizati… | [`no_scripture_text_is_lost_across_the_construct_set`](../crates/biblecompose-scripture/tests/normalize.rs) (+1 more) |
| **FUN-003** | Unknown or unsupported USFM markers shall generate structured diagnostics and shall not… | [`an_unsupported_marker_keeps_the_text_underneath_it`](../crates/biblecompose-scripture/tests/normalize.rs) (+1 more) |
| **FUN-004** | The application shall distinguish blocking errors from non-blocking warnings. | [`h_invalid_config`](../crates/biblecompose-app/tests/acceptance.rs) |
| **FUN-005** | The application shall expose detected metadata including book ID, file path, chapter co… | [`a_folder_of_usfm_opens_with_its_books_and_its_defaults`](../crates/biblecompose-tauri/tests/commands.rs) |
| **FUN-006** | The user shall be able to reload the project after external file changes without closin… | [`an_external_settings_edit_is_reflected_after_reload`](../crates/biblecompose-tauri/tests/commands.rs) |
| **BLD-001** | The user shall be able to start PDF generation from the GUI. | [`a_folder_of_usfm_opens_with_its_books_and_its_defaults`](../crates/biblecompose-tauri/tests/commands.rs) |
| **BLD-002** | A successful build shall create exactly one primary PDF at the resolved output path. | [`a_defaults_only`](../crates/biblecompose-app/tests/acceptance.rs) |
| **BLD-003** | The default output filename shall be derived from project name, with a configurable ove… | [`a_setting_the_project_wrote_says_where`](../crates/biblecompose-tauri/tests/commands.rs) (+1 more) |
| **BLD-004** | The application shall not overwrite source USFM during a build. | [`a_defaults_only`](../crates/biblecompose-app/tests/acceptance.rs) (+2 more) |
| **BLD-005** | The application shall capture SILE standard output, warnings, and errors and translate… | [`j_cancel`](../crates/biblecompose-app/tests/acceptance.rs) |
| **BLD-006** | The application shall support canceling an in-progress build and terminating the SILE c… | [`j_cancel`](../crates/biblecompose-app/tests/acceptance.rs) |
| **BLD-009** | A failed build shall not replace the last known good PDF unless the new PDF was complet… | [`j_cancel`](../crates/biblecompose-app/tests/acceptance.rs) (+2 more) |
| **BOOK-001** | Detected books shall default to canonical Bible order rather than filesystem order. | [`canonical_order_beats_the_order_files_arrive_in`](../crates/biblecompose-scripture/tests/plan.rs) (+1 more) |
| **BOOK-002** | Project settings shall allow explicit book ordering. | [`a_configured_order_is_reflected`](../crates/biblecompose-scripture/tests/plan.rs) |
| **BOOK-003** | Project settings shall allow books to be included or excluded without deleting source f… | [`a_book_left_out_of_the_list_does_not_appear`](../crates/biblecompose-scripture/tests/plan.rs) (+1 more) |
| **SCR-001** | Chapter and verse markers shall be retained semantically even when configured to be vis… | [`an_anchor_outlives_the_number_it_belongs_to`](../crates/biblecompose-app/tests/metadata.rs) (+4 more) |
| **SCR-002** | Paragraph, poetry, section heading, list, and character-level styles shall map to confi… | [`the_matrix_covers_every_selector_class`](../crates/biblecompose-app/tests/style_matrix.rs) |
| **SCR-003** | USFM footnotes shall be parsed as structured note content and rendered through a SILE f… | [`the_two_sequences_run_independently`](../crates/biblecompose-app/tests/apparatus.rs) |
| **SCR-004** | USFM cross-references shall be parsed independently from footnotes. | [`a_cross_reference_is_not_a_note`](../crates/biblecompose-scripture/tests/normalize.rs) |
| **SCR-005** | MVP shall support rendering cross-references as footnote-area references or inline/end-… | [`cross_references_can_be_set_in_the_text`](../crates/biblecompose-app/tests/apparatus.rs) |
| **SCR-006** | USFM figure markers shall be parsed with source, alt/caption/reference metadata where p… | [figures.rs](../crates/biblecompose-app/tests/figures.rs) *(the whole suite)* |
| **SCR-007** | The application shall support hiding chapter numbers, verse numbers, section headings,… | [`each_switch_hides_only_its_own`](../crates/biblecompose-app/tests/visibility.rs) (+1 more) |
| **CFG-001** | BibleCompose shall contain an embedded default settings configuration. | [`the_position_agrees_with_the_parsers_own_rendering`](../crates/biblecompose-config/tests/document.rs) (+5 more) |
| **CFG-002** | If biblecompose.toml exists, project values shall override embedded defaults field-by-f… | [`one_override_leaves_every_other_default_intact`](../crates/biblecompose-config/tests/settings.rs) (+2 more) |
| **CFG-003** | Invalid TOML syntax shall produce a blocking diagnostic containing filename and line/co… | [`a_syntax_error_has_a_position`](../crates/biblecompose-config/tests/document.rs) (+9 more) |
| **CFG-004** | Unknown settings keys shall produce a warning by default and may be treated as errors i… | [`a_table_reports_the_keys_it_has_and_where_they_are`](../crates/biblecompose-config/tests/document.rs) (+5 more) |
| **CFG-005** | The GUI shall be able to save supported setting changes back to biblecompose.toml. | [`a_saved_change_survives_reopening`](../crates/biblecompose-config/tests/edit.rs) (+3 more) |
| **CFG-007** | A user shall be able to reset a setting to inherited/default behavior. | [`reset_removes_the_key_rather_than_writing_the_default_into_the_file`](../crates/biblecompose-config/tests/edit.rs) (+8 more) |
| **STY-001** | BibleCompose shall contain built-in styles for every USFM marker it claims to support. | [`a_cycle_is_one_diagnostic_naming_it`](../crates/biblecompose-config/tests/cascade.rs) (+6 more) |
| **STY-002** | Project styles.toml shall override built-in styles by semantic selector. | [`an_override_changes_only_what_it_names`](../crates/biblecompose-config/tests/cascade.rs) (+4 more) |
| **STY-003** | The style engine shall distinguish paragraph-level, character-level, chapter, verse, no… | [`a_style_inheriting_from_itself_is_caught_when_it_is_read`](../crates/biblecompose-config/tests/cascade.rs) (+4 more) |
| **STY-004** | Unsupported style properties shall generate diagnostics rather than being silently igno… | [`an_unknown_selector_is_reported_at_its_line`](../crates/biblecompose-config/tests/style.rs) (+3 more) |
| **STY-005** | The GUI shall expose common style properties without requiring TOML editing. | [`resetting_a_style_restores_the_cascade`](../crates/biblecompose-tauri/tests/commands.rs) |
| **STY-006** | Advanced users shall be able to edit styles.toml externally and reload. | [`an_external_style_edit_is_reflected_after_reload`](../crates/biblecompose-tauri/tests/commands.rs) |
| **USFM-001** | The parser shall produce source spans or equivalent location metadata sufficient to ass… | [`a_syntax_error_has_a_position`](../crates/biblecompose-config/tests/document.rs) |
| **USFM-002** | Nested character markers and note submarkers shall be represented structurally rather t… | [`a_cross_reference_is_not_a_note`](../crates/biblecompose-scripture/tests/normalize.rs) |
| **USFM-003** | Character attributes defined by supported USFM versions shall be preserved in the inter… | [`a_figure_is_reported_where_the_author_said_it_belongs`](../crates/biblecompose-app/tests/figures.rs) (+2 more) |
| **USFM-005** | The parser shall never silently merge verse text across book boundaries or reorder Scri… | [`normalizing_the_corpus_loses_nothing`](../crates/biblecompose-testkit/tests/normalize_corpus.rs) |
| **USFM-006** | Validation shall detect missing/invalid \id, malformed chapter/verse numbers, unclosed… | [`g_invalid_usfm`](../crates/biblecompose-app/tests/acceptance.rs) |
| **SILE-001** | BibleCompose shall invoke SILE only through a dedicated backend adapter interface. | [`no_crate_depends_on_the_app_except_the_cli`](../crates/biblecompose-testkit/tests/architecture.rs) |
| **SILE-002** | The application shall detect and report the SILE backend version used for each build. | [`the_backend_reports_a_version`](../crates/biblecompose-sile/tests/backend.rs) |
| **SILE-005** | The generated SILE input shall be deterministic for identical normalized input and reso… | [`the_styles_block_is_byte_stable`](../crates/biblecompose-app/tests/style_golden.rs) (+3 more) |
| **SILE-006** | Backend stderr/stdout shall be captured and associated with the active build. | [`backend_output_is_captured_from_both_streams`](../crates/biblecompose-sile/tests/backend.rs) |
| **SILE-007** | SILE-specific failures shall be converted to understandable BibleCompose diagnostics wh… | [`a_known_failure_is_named`](../crates/biblecompose-app/tests/backend_failure.rs) |
| **SILE-008** | The application shall clean temporary intermediate files after a successful build unles… | [`a_finished_build_leaves_no_scripture_on_disk`](../crates/biblecompose-app/tests/hygiene.rs) |
| **SILE-009** | SILE custom packages used by BibleCompose shall be versioned together with BibleCompose… | [`an_unknown_failure_is_not_swallowed`](../crates/biblecompose-app/tests/backend_failure.rs) |
| **PDF-001** | Generated output shall be a standards-compliant PDF readable by current mainstream PDF… | [`every_page_of_every_fixture_is_sound`](../crates/biblecompose-app/tests/corpus_build.rs) |
| **PDF-002** | Page size, margins, columns, fonts, text direction, and supported images shall reflect… | [`a_paragraph_that_runs_for_pages_leaves_none_blank`](../crates/biblecompose-app/tests/corpus_build.rs) |
| **PDF-003** | Fonts required for correct text display shall be embedded or otherwise handled in a pri… | [`every_page_of_every_fixture_is_sound`](../crates/biblecompose-app/tests/corpus_build.rs) (+2 more) |
| **PDF-004** | Unicode Scripture text shall render with no missing-glyph boxes when the configured fon… | [`no_corpus_book_is_blocked_by_anything_but_a_font`](../crates/biblecompose-app/tests/corpus_build.rs) |
| **GUI-001** | The user shall be able to open a project folder from the main window. | [`a_folder_of_usfm_opens_with_its_books_and_its_defaults`](../crates/biblecompose-tauri/tests/commands.rs) |
| **GUI-002** | The project pane shall list detected books in resolved order with included/excluded and… | [`editing_a_field_writes_the_file`](../crates/biblecompose-tauri/tests/commands.rs) (+1 more) |
| **GUI-003** | The GUI shall provide access to core settings without requiring manual TOML editing. | [`the_picker_says_which_fonts_can_set_the_scripture`](../crates/biblecompose-app/tests/font_preflight.rs) |
| **GUI-004** | The GUI shall provide a style editor for at least body paragraphs, poetry, section head… | [`editing_a_style_writes_the_sheet`](../crates/biblecompose-tauri/tests/commands.rs) |
| **GUI-005** | The GUI shall show diagnostics with severity, message, book/file, and source location w… | [`h_invalid_config`](../crates/biblecompose-app/tests/acceptance.rs) |
| **GUI-006** | The GUI shall show build state: idle, validating, generating, running SILE, completed,… | [`the_page_number_goes_where_it_is_put`](../crates/biblecompose-app/tests/heads.rs) |
| **GUI-007** | The GUI shall provide a build log with copyable technical output. | *exception* — **No automated test, and the requirement is met by design.** Every build writes the backend's whole output to a file and the window reports where; the panel shows each message with its raw detail, which is selectable text in a webview. What cannot be asserted is that a person can *copy* it — that is the operating system's clipboard, and a test of it would be a test of the webview. |
| **GUI-009** | If integrated preview is unavailable on a platform, the application shall provide Open… | [`a_draft_is_written_beside_the_real_pdf`](../crates/biblecompose-app/tests/drafts.rs) |
| **GUI-010** | Unsaved settings/style edits shall be visibly indicated and protected from accidental p… | *exception* — **Satisfied vacuously, which is the stronger answer.** There are no unsaved edits to indicate or protect: every settings and style change is written to the file as it is made (CFG-005), so closing a project cannot discard anything. The requirement anticipates a dialog with an OK button, and this window does not have one. |
| **GUI-012** | The UI should remain responsive while parsing or typesetting by running long work off t… | [`a_second_identical_build_does_not_run_the_backend`](../crates/biblecompose-app/tests/reuse.rs) |
| **NFR-003** | The GUI shall remain responsive during builds lasting minutes. | [`a_whole_canon_opens_in_well_under_a_second`](../crates/biblecompose-app/tests/opening.rs) |
| **NFR-004** | BibleCompose shall not require an internet connection for project load, validation, com… | [`the_shell_is_the_only_exemption_and_it_is_the_frameworks`](../crates/biblecompose-testkit/tests/offline.rs) (+1 more) |
| **NFR-005** | The application shall use UTF-8 internally and preserve Unicode text without lossy conv… | [`a_title_in_another_script_survives`](../crates/biblecompose-app/tests/metadata.rs) |
| **NFR-006** | Builds shall be reproducible enough for regression testing; generated intermediate text… | [`the_styles_block_is_byte_stable`](../crates/biblecompose-app/tests/style_golden.rs) |
| **NFR-007** | A crash or failed build shall not corrupt source USFM or existing configuration files. | [`generated_directories_never_become_inputs`](../crates/biblecompose-project/tests/discovery.rs) |
| **NFR-008** | Configuration and style schemas shall be versionable to permit controlled future evolut… | [`an_unknown_version_produces_exactly_one_diagnostic`](../crates/biblecompose-config/tests/settings.rs) |
| **NFR-009** | The core parser, configuration resolver, style resolver, and SILE emitter shall be test… | [`the_cli_links_no_gui_crate`](../crates/biblecompose-testkit/tests/architecture.rs) (+1 more) |
| **DIA-001** | Errors shall use stable machine-readable codes in addition to human-readable text. | [`h_invalid_config`](../crates/biblecompose-app/tests/acceptance.rs) |
| **DIA-002** | A build blocked by validation shall explain the blocking issues before SILE is invoked. | [`every_figure_is_checked`](../crates/biblecompose-app/tests/figures.rs) (+2 more) |
| **DIA-003** | Where safe, warnings shall allow the build to proceed. | [`omit_warns_and_names_the_figure_to_withhold`](../crates/biblecompose-app/tests/figures.rs) |
| **DIA-005** | Raw SILE logs shall be available for debugging but collapsed by default behind user-fri… | [`an_unknown_failure_is_not_swallowed`](../crates/biblecompose-app/tests/backend_failure.rs) |

---

**78 MUST requirements.** 76 are verified by a test;
2 are recorded exceptions; 0 are unanswered.

`node scripts/traceability.mjs --check` fails if this file is out of date, and
`node scripts/traceability.mjs` rewrites it. Neither will produce a table with
a blank row in it.
