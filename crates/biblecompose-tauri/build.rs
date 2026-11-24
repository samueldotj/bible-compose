//! Generates the Tauri context, and the permission for each of this
//! application's own commands.
//!
//! **Why the commands are listed here.** Tauri lets every window call an
//! application's commands unless the application declares them, and then a
//! window may call only what a capability grants it. There are two kinds of
//! window now (ADR-007): `main`, which is BibleCompose, and the `editor-*`
//! windows easy-usfm runs in, which render a book's text and have no business
//! starting a build or fetching a font. So the commands are declared, `main`
//! is granted all of them (`capabilities/default.json`), and an editor window
//! is granted none.
//!
//! A command added to `generate_handler!` and not here cannot be granted, and
//! the window is refused it at run time — `COMMANDS` and the handler list are
//! checked against each other by `the_declared_commands_are_the_handled_ones`.

const COMMANDS: &[&str] = &[
    "versions",
    "defaults",
    "open_project",
    "changed_files",
    "set_setting",
    "reset_setting",
    "set_style",
    "reset_style",
    "fonts",
    "font_suggestions",
    "install_font",
    "recent_projects",
    "forget_project",
    "create_project",
    "check_project_folder",
    "add_front_matter",
    "make_cover",
    "cover_preview",
    "close_project",
    "open_folder",
    "open_pdf",
    "open_preview",
    "close_preview",
    "open_in_editor",
    "open_url",
    "presets",
    "head_fields",
    "apply_preset",
    "start_build",
    "cancel_build",
    "is_building",
    "check_for_update",
    "install_update",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("the Tauri context could not be generated");
}
