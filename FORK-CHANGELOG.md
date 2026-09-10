# Fork changelog — customizations on top of native herdr

This fork tracks [ogulcancelik/herdr](https://github.com/ogulcancelik/herdr)
(upstream) and carries a small set of UI customizations on top. This file is
the single source of truth for what the fork changes, where each change
lives, and how to re-verify it after pulling upstream.

Upstream owns the root `CHANGELOG.md` — never edit it here. This file is the
fork's; add one entry per fork change, newest first.

## How to maintain this file

1. Every fork change gets a numbered entry under "Customization map" with:
   what it does, which files/functions it touches, the upstream integration
   points to re-check after a pull, and the tests that cover it.
2. After every upstream pull, walk the map top to bottom, fix what broke,
   run the validation checklist, and add a dated line under "Log".
3. If a customization is removed or replaced, delete its entry and record
   the removal in the Log.

## Customization map

### 1. Vertical tab strip — `ui.vertical_tabs` (bool, default false)

Replaces the left sidebar (spaces + agents panel) and the horizontal top tab
row with a single left-hand vertical strip listing **every workspace's tabs**,
grouped into one rounded box per workspace (no workspace header rows — the
context line carries the workspace name). Clicking any tab — including tabs
of other workspaces — focuses that tab and switches to its workspace
(upstream `TabFocus` already does both; no server changes). The border is
`overlay0` for the focused workspace's box, `surface_dim` for others.

Each tab renders as a bordered block:

- Line 1: tab title (left) + agent badge (right) — agent name shown only when
  the title keeps 4+ columns, status icon always right-aligned.
- Line 2 (unless `vertical_tabs_compact`): `repo • branch • workspace`,
  dropping leftmost segments when the strip is too narrow. "repo" comes from
  the attached worktree's label (the client snapshot has no repo-name field —
  if upstream adds one, use it); branch from `workspace.branch`; workspace
  label always last.
- Extra panes: one line per additional pane below (pane label, else cwd
  basename) with that pane's own agent badge. Clicking a pane line sends
  `PaneFocus` for that pane; clicking the title/context region sends
  `TabFocus`.
- Active tab: muted `surface0` background + bold text. Inactive tabs: no
  background, dim text. Blocks are wrapped in a rounded box
  (`╭─╮│╰─╯`), `overlay0` border for the active tab, `surface_dim` for
  inactive ones.

**Where:**

- `src/client/shell/state.rs` — `ClientShellConfig::{vertical_tabs,
  vertical_tabs_compact}`, `ClientShellLayout.tab_strip`.
- `src/client/shell/config.rs` — config threading (`from_config`,
  `apply_live_config`) and the layout branch in `ClientShellConfig::layout()`
  (strip replaces sidebar + tab bar; ignores sidebar collapse).
- `src/client/shell/tabs.rs` — `render_tab_strip`, `render_tab_strip_group`
  (one box per workspace), `render_strip_line_with_badge`,
  `render_strip_context_line`, `draw_strip_box`, `strip_agent_name`,
  `strip_pane_title`, plus the vertical scroll helpers
  (`max_tab_strip_scroll`, `strip_scroll_revealing`, `last_visible_entry`,
  `overflow_tab_strip`) and `tab_drop_indicator_y`.
- `src/client/shell/render.rs` — `render_shell` dispatch (strip branch before
  the sidebar/tab-bar branch).
- `src/client/shell/mouse.rs` — `tab_drop_index_at` vertical branch for
  tab-drag drop indices; `cycle_strip_tab` — wheel over the strip cycles
  through ALL tabs across workspaces (upstream wheel = `NextTab`/`PrevTab`,
  workspace-scoped; kept for non-strip mode). The strip auto-reveals the
  newly focused tab; clamps at the ends. `tab_scroll_right` clamps by
  workspace-group count in strip mode.
- `src/config/model.rs` — `UiConfig::{vertical_tabs, vertical_tabs_compact}`.
- `docs/next/website/src/data/config-reference.json` — entries for both keys
  (the `config_reference_check` maintenance test enforces this file stays in
  lockstep with the Rust config model).

**Re-check after upstream pulls:**

- `ClientShellConfig::layout()` in `shell/config.rs` — upstream may change
  layout math; re-apply the `vertical_tabs` early-return branch.
- `render_shell` in `shell/render.rs` — re-hook the `layout.tab_strip` branch
  if the render flow moves.
- `ClientShellTab`/`ClientShellPane`/`ClientShellAgent` in
  `src/protocol/wire.rs` — the strip reads `tab_id`, `workspace_id`, `label`,
  `custom_label`, `zoomed`, `focused`, `agent_status`, pane `cwd`/
  `foreground_cwd`/`label`, agent `display_agent`/`name`/`agent`/`title`.
- `tab_drop_index_at` + the wheel handlers + `tab_scroll_right` clamp in
  `shell/mouse.rs` — keep the vertical/strip branches in sync with
  upstream's tab-bar logic.

**Known limitations:** tab drag-reorder drop indices are computed against
the focused workspace's tabs (upstream semantics); dragging a tab of a
non-focused workspace may drop at an unexpected index. Scroll granularity is
the workspace group (one box per notch) — a single workspace with more tabs
than fit the viewport cannot scroll them all. Both low priority.

**Tests:** `src/client/shell/tests/vertical_tabs.rs` (grouping, click-to-focus,
pane clicks, compact mode, narrow-strip name hiding, overflow scrolling).

### 2. Makefile (fork helper)

`make build` / `make run` (debug, sandboxed) / `make run-release` (real
config + sessions) / `make run-dev` (isolated fresh server) / `make
stop-dev` / `make test` / `make check`. The repo's canonical runner is still
`just`; the Makefile exists because debug-vs-stable toolchain quirks are easy
to get wrong (see gotchas below).

## Gotchas (learned the hard way)

- **Debug builds are sandboxed.** `cfg!(debug_assertions)` redirects the
  config/state root from `~/.config/herdr` to `~/.config/herdr-dev`
  (`src/config/io.rs::app_dir_name`). A `make run` session therefore looks
  empty and ignores your real config — copy your config into
  `~/.config/herdr-dev/config.toml` or use `make run-release`.
- **Nesting guard.** Running any `herdr` binary inside a Herdr pane
  (`HERDR_ENV=1`) exits with "nested herdr is disabled by default"
  (`src/main.rs::exit_if_nested_disabled`). Launch clients from a plain
  terminal tab.
- **Clippy needs the pinned toolchain.** rust-toolchain.toml pins 1.96.1;
  Homebrew's newer Rust fails `clippy -- -D warnings` on untouched upstream
  files. The Makefile prepends the matching rustup toolchain to PATH.
- **Unknown config keys are ignored** by stock herdr (`#[serde(default)]`,
  no `deny_unknown_fields`), so sharing one `~/.config/herdr/config.toml`
  between stable and fork is safe.
- **Probes:** to capture what a client renders, use a pty with an explicit
  winsize (rows come first in `TIOCSWINSZ`) — see the pyte-based probe used
  during development; a zero-sized grid makes the client exit immediately.

## Validation checklist (run after every upstream pull)

```sh
make check                 # fmt + clippy + nextest (pinned toolchain via PATH)
just maintenance-test      # config-reference check, changelog scripts, …
just ui-hot-path-architecture-test
make run-release           # eyeball the strip against the live session
```

Known failures on this machine that are NOT the fork's fault (confirmed on
clean upstream master): `live_handoff_preserves_pane_process_io`,
`live_handoff_keeps_agent_started_pane_after_agent_exits`,
`live_handoff_keeps_unmanaged_agent_name_bound_to_saved_session`,
`pane_info_and_subscriptions_expose_done_agent_status` (all spawn real PTY
agents), plus an occasional timing flake in
`federated_launch_opens_local_directly_while_saved_ssh_is_unavailable` that
passes in isolation.

## Log

### 2026-09-10

- Added the vertical tab strip behind `ui.vertical_tabs`: left-hand strip,
  all workspaces' tabs in one rounded box per workspace (no headers), per-tab
  agent badge inline (name hidden when tight), `repo • branch • workspace`
  context line, per-extra-pane lines with pane-focus clicks, `TabFocus` on
  click, group-level scrolling + drag-drop indicator.
- Added `ui.vertical_tabs_compact` to drop the context line.
- Mouse wheel over the strip cycles through all tabs across workspaces
  (clamped); the strip auto-reveals the focused tab.
- Added the fork Makefile.
- Fork point: upstream `120c6820` ("fix: restore connected session navigator
  tree glyphs (#3861)").
