# Roadmap: shortcuts_tui Improvements

This roadmap lists the development phases and detailed tasks to refactor, optimize, and enhance the Rust-based hotkey visualizer.

## 📋 Development Phases

- [x] **Phase 1: DRY Refactoring & Dynamism**
  - [x] Extract duplicated `system_shortcuts.json` loading from `App::new` and `App::reload` into `App::load_system_shortcuts`.
  - [x] Modify `parser.rs` to read `app_aliases.json` dynamically instead of using hardcoded rules in `get_app_slug`.
  - [x] Consolidate lists of special keys (e.g. esc, enter, space) into global constants in `ui.rs`/`app.rs`.

- [x] **Phase 2: Render Loop Caching**
  - [x] Add a `filtered_cache: Vec<usize>` (or `Vec<Shortcut>`) to `App` to store the active filtered items.
  - [x] Update cache only when active filter inputs (search query, app queries, space filter, modifiers) change.
  - [x] Replace costly inline `.filtered_items()` invocations in `ui.rs` with references to the cached list.

- [x] **Phase 3: Parsing Improvements**
  - [x] Enhance `parser.rs` complex modifications logic to support `to_if_alone`, `to_if_held_down`, and `to_after_key_up`.
  - [x] Properly parse and format optional modifiers (`"optional": ["any"]`).
  - [x] Provide descriptive outputs for combined taps/holds.

- [x] **Phase 4: UI/UX & Polish**
  - [x] **Terminal Size Guard**: Hide ASCII keyboard and show a compact warning if width is `< 100` characters.
  - [x] **Scrollbar Integration**: Add Ratatui's interactive `Scrollbar` component to the table.
  - [x] Improve focus visual states (changing border styles or colors when searching).
