# Roadmap: shortcuts_tui Improvements

This roadmap lists the development phases and detailed tasks to refactor, optimize, and enhance the Rust-based hotkey visualizer.

## 📋 Development Phases

- [ ] **Phase 1: DRY Refactoring & Dynamism**
  - [ ] Extract duplicated `system_shortcuts.json` loading from `App::new` and `App::reload` into `App::load_system_shortcuts`.
  - [ ] Modify `parser.rs` to read `app_aliases.json` dynamically instead of using hardcoded rules in `get_app_slug`.
  - [ ] Consolidate lists of special keys (e.g. esc, enter, space) into global constants in `ui.rs`/`app.rs`.

- [ ] **Phase 2: Render Loop Caching**
  - [ ] Add a `filtered_cache: Vec<usize>` (or `Vec<Shortcut>`) to `App` to store the active filtered items.
  - [ ] Update cache only when active filter inputs (search query, app queries, space filter, modifiers) change.
  - [ ] Replace costly inline `.filtered_items()` invocations in `ui.rs` with references to the cached list.

- [ ] **Phase 3: Parsing Improvements**
  - [ ] Enhance `parser.rs` complex modifications logic to support `to_if_alone`, `to_if_held_down`, and `to_after_key_up`.
  - [ ] Properly parse and format optional modifiers (`"optional": ["any"]`).
  - [ ] Provide descriptive outputs for combined taps/holds.

- [ ] **Phase 4: UI/UX & Polish**
  - [ ] **Terminal Size Guard**: Hide ASCII keyboard and show a compact warning if width is `< 100` characters.
  - [ ] **Scrollbar Integration**: Add Ratatui's interactive `Scrollbar` component to the table.
  - [ ] Improve focus visual states (changing border styles or colors when searching).
