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
title row carries the workspace name, right-aligned). Clicking any tab —
including tabs of other workspaces — focuses that tab and switches to its
workspace (upstream `TabFocus` already does both; no server changes). The
border is `overlay0` for the focused workspace's box, `surface_dim` for
others.

Each tab renders as a bordered block:

- Title row: tab title (left) + workspace label right-aligned, dim — this is
  where the agent badge used to sit; per-pane facts moved to the details row.
  The label hides before the title drops below 4 columns.
- Details row (unless `vertical_tabs_compact`): the primary pane's cwd leaf
  first, then `workspace.branch`, joined by ` • `; the primary pane's agent
  badge (name + status icon) right-aligned. Segments drop from the RIGHT
  when narrow (branch before leaf — the leaf is the identity); the cwd leaf
  is capped at 8 columns, tail ellipsized (`kartyb-…`). The client snapshot
  has no per-pane branch — `workspace.branch` is shared by all rows in the
  workspace; if upstream adds one, use it.
- Extra panes: one line per additional pane in the same details format (pane
  label if set, else cwd leaf) with that pane's own agent badge. Clicking a
  pane line sends `PaneFocus` for that pane; clicking the title/details
  region sends `TabFocus`.
- Compact mode (`vertical_tabs_compact`) drops the details row, so the agent
  badge stays on the title row there.
- Active tab: muted `surface0` background + bold text. Inactive tabs: no
  background, dim text. Blocks are wrapped in a rounded box (`╭─╮│╰─╯`),
  `overlay0` border for the active tab, `surface_dim` for inactive ones.
  The strip paints `sidebar_bg` only — no sidebar separator `│` (it read as
  a stray border line below the last box).
- Flush highlight + text padding: the active tab's background bleeds under
  the adjacent border cells (left/right on every highlighted row; top/bottom
  border rows when the block touches the group edge), so the block reads
  continuous with the box instead of floating a half-cell inside it —
  border glyphs stay drawn on top. Row text is inset one column per side
  (`STRIP_TEXT_INSET`) so it keeps breathing room from the borders; hit
  rects keep the full row width.
- First-pane click: the details row IS the primary pane's row — it routes
  `PaneFocus` for that pane (which switches tab + workspace too), so the
  first pane switches like every other pane row even when its tab is
  already focused (TabFocus there was a no-op). Without a primary pane the
  details row stays part of the tab hit. The title row alone sends
  `TabFocus`.
- Details-row color follows the primary pane's state (same convention as
  the pane rows): focused pane reads `overlay1`, everything else stays
  dim — the row no longer renders the inactive color when the first pane
  is the focused pane.
- Mouse resize: `render_tab_strip` arms the upstream `SidebarWidth` drag by
  setting `hits.sidebar_divider` to the strip's last column, content rows
  only (the chrome row's right corner owns the ▲▼ buttons — a full-height
  divider swallows their clicks). Tab hit rects stop at the inner edge, so
  no tab click is stolen. Drag resizes; double-click resets; width persists
  through the existing chrome-preferences path.

**Where:**

- `src/client/shell/state.rs` — `ClientShellConfig::{vertical_tabs,
  vertical_tabs_compact}`, `ClientShellLayout.tab_strip`.
- `src/client/shell/config.rs` — config threading (`from_config`,
  `apply_live_config`) and the layout branch in `ClientShellConfig::layout()`
  (strip replaces sidebar + tab bar; collapse hides the strip entirely —
  `prefix+b`/ToggleSidebar toggles it back, persisting through chrome
  preferences like upstream's sidebar).
- `src/client/shell/tabs.rs` — `render_tab_strip`, `render_tab_strip_group`
  (one box per workspace), `render_strip_line_with_badge` (segment-based,
  drops rightmost when narrow), `render_strip_title_row`, `strip_cwd_leaf` /
  `ellipsize_tail` / `detail_segments`, `draw_strip_box`, `strip_agent_name`,
  plus the vertical scroll helpers (`max_tab_strip_scroll`,
  `strip_scroll_revealing`, `last_visible_entry`, `overflow_tab_strip`) and
  `tab_drop_indicator_y`.
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
pane clicks, compact mode, cwd-leaf/branch details + workspace-on-title,
8-column leaf ellipsis, narrow-strip right-to-left dropping, no separator +
flush group stacking, overflow scrolling).

### 2. Makefile (fork helper)

`make build` / `make run` (debug, sandboxed) / `make run-release` (real
config + sessions) / `make stop-dev` / `make test` / `make check` / `make
install`. The repo's canonical runner is still `just`; the Makefile exists
because debug-vs-stable toolchain quirks are easy to get wrong (see gotchas
below). Build targets resolve the toolchain pins themselves — the rustup
toolchain on PATH and `ZIG` (prefers `~/zig-0.16.0/zig`, env/command-line
override wins) — so make-driven builds track the vendored libghostty-vt's
current Zig requirement. There is deliberately no stop-release: stopping
the live server is `herdr server stop` (kills all panes/agents) and should
not sit one tab-completion away.

`make install` symlinks `target/release/herdr` to
`$XDG_BIN_HOME/herdr` (default `~/.local/bin`; fish PATH precedence puts
`$XDG_BIN_HOME` ahead of `/opt/homebrew/bin`), so plain `herdr` runs the
fork; the brew install stays for rollback — remove the symlink to fall
back. If PATH precedence changes, `herdr` silently reverts to brew
(`which -a herdr` shows both). The fork binary is the full herdr CLI (help output is
byte-identical to stable): server, agent, pane, workspace, tab, session,
machine, api, config, worktree, notification, integration subcommands all
work against the running server.

### 3. Upstream sync tooling — `/sync-upstream` + drift test

A project-local pi prompt (`.pi/prompts/sync-upstream.md`, run as
`/sync-upstream [--dry-run]`) that systematizes upstream pulls:
preconditions (clean tree, master, force-push guard), impact preview
(carried diff × incoming commits), merge with a `pre-sync/<date>` rollback
anchor, map-driven conflict resolution (fork-owned files keep the fork
side; shared files take upstream structure + re-applied fork behavior;
protocol/wire conflicts STOP and ask), the phase-3 validation battery,
and a FORK-CHANGELOG.md Log update + single sync commit. Merge, never
rebase.

`scripts/test_fork_changelog_check.py` (registered in
`just maintenance-test`) guards the map itself: every `src/`/`docs/` path
referenced by FORK-CHANGELOG.md must exist, every load-bearing symbol in
its `MAPPED_SYMBOLS` list must still exist in its mapped file, and
AGENTS.md must keep requiring changelog tracking. After an upstream pull
that renames or moves fork integration points, this fails first and
points at the stale map entry.

**Where:**

- `.pi/prompts/sync-upstream.md` — the sync runbook prompt.
- `scripts/test_fork_changelog_check.py` — map-drift maintenance test
  (`MAPPED_SYMBOLS` mirrors the Where sections; keep both in sync).
- `justfile` — maintenance-test registration.
- `AGENTS.md` — fork rules referencing the test + the prompt.

**Re-check after upstream pulls:**

- `MAPPED_SYMBOLS` in the test vs the changelog's Where sections — update
  together when symbols move.
- The pinned-toolchain PATH in the prompt's clippy step tracks
  `rust-toolchain.toml` (1.96.1 today).

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
- **Builds need Zig 0.16.0.** Upstream's libghostty-vt upgrade (425c8617)
  hard-fails older Zig in build.rs. System zig is 0.15.2 (brew zig@0.15,
  kept for other work); 0.16.0 is installed at ~/zig-0.16.0. Make targets
  export `ZIG` themselves (env override wins); plain `cargo` invocations
  still need `ZIG=$HOME/zig-0.16.0/zig` exported by hand. The
  `/sync-upstream` build step verifies the make path on every sync.
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

Added 2026-09-16 after the machine's macOS/Xcode update (each confirmed
failing identically on the pre-merge fork master and/or clean upstream
master `b88e8116`, so none are fork regressions):
`events_subscribe_streams_output_and_agent_status_events`,
`live_server_holds_one_pty_master_fd_per_pane`,
`same_tab_geometry_follows_meaningful_client_activity`,
`api_pane_output_is_fanned_out_as_pane_surface_updates`,
`live_handoff_carries_more_panes_than_one_scm_rights_message` (ptmx fd
count off by one on macOS), and upstream's `scripts.test_release`
hotfix test (its temp-repo `git apply --3way` reports "No valid
patches" under local git 2.55.0; fails on clean upstream too).

Observed 2026-09-21 on the merged `b88e8116..28360107` tree: the six
Xcode-update failures above no longer reproduce. The full suite ran 3557
tests, 3556 passed, 1 failed
(`pane_info_and_subscriptions_expose_done_agent_status`), 6 skipped. The
current baseline is that single test plus the `scripts.test_release` git
2.55.0 hotfix failure; a reappearance of the others is now a regression
signal again, not background noise.

## Log

### 2026-09-09 (sync tooling)

- Added `/sync-upstream` (.pi/prompts/sync-upstream.md): four-phase runbook
  (preconditions → impact preview → merge with map-driven conflict
  playbook → validation + paper trail), --dry-run support, merge-never-
  rebase, stop-and-flag on protocol conflicts.
- Added scripts/test_fork_changelog_check.py (maintenance-test): fails when
  mapped files/symbols drift after an upstream pull.

### 2026-09-09 (details-row color)

- The details row text follows the primary pane's state (focused pane =
  `overlay1`, else dim) instead of always rendering the inactive color.

### 2026-09-09 (first-pane click)

- The details row routes PaneFocus for the primary pane, so clicking the
  first pane's row switches to it like every other pane row (TabFocus on
  an already-focused tab was a no-op); the title row alone sends TabFocus.

### 2026-09-09 (strip mouse resize)

- The strip is mouse-resizable again (it never was: the strip commit never
  set `hits.sidebar_divider`): the box border column arms the upstream
  `SidebarWidth` drag on content rows only, so the ▲▼ chrome stays clickable.

### 2026-09-09 (flush highlight + text padding)

- Active-tab highlight now bleeds under the adjacent border cells (the
  border glyphs float centered in their own cells, which read as padding
  between the box and the rows); row text is inset one column per side.

### 2026-09-09 (details-row rework)

- Strip tab blocks reworked: title row now carries the workspace label
  (right-aligned, replaces the agent badge); details row is now `cwd leaf •
  branch` + agent badge (was `repo • branch • workspace`); extra pane rows
  use the same format; cwd leaf capped at 8 columns with a trailing
  ellipsis; details drop right-to-left when narrow.
- Removed the sidebar separator `│` from the strip path (rendered as a stray
  full-height line at the panel edge); the strip paints background only. The
  blue active-pane edge indicator is separate and untouched.
- Added `## Fork rules` to `AGENTS.md`: every fork change must be tracked in
  `FORK-CHANGELOG.md`; work notes in `.agents/dox/`.

### 2026-09-10

- `make install` symlinks the release binary over the daily `herdr` command
  (into `$XDG_BIN_HOME`, which precedes /opt/homebrew/bin on PATH); brew
  stable remains the fallback.
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

### 2026-09-10 (upstream sync)

- Merged upstream/master `120c6820..90e947a6` (11 commits). Clean merge,
  zero conflicts: upstream's mouse-selection work (repaint cadence #3901,
  pane selection highlights #3894) touched `shell/mouse.rs` and
  `shell/state.rs` in regions disjoint from the strip's handlers/config,
  so both mapped shared files auto-merged with no rework.
- Notable fixes absorbed: bounded mouse selection repaint cadence (#3901),
  host background for pane selection highlights (#3894), WezTerm control
  key reports (#3592), machine add options before the SSH target (#3897),
  focused viewer geometry during activation (#3746), legacy Windows mouse
  reports over ssh (#3742), Linux desktop notification identification
  (#3639), session names after option terminator (#3222), spaces sidebar
  resize stability (#3862), token selection stops at CJK punctuation
  (#3708), Windows session paths for pi (#3729).
- Validation green: fork-map drift test, vertical_tabs (17/17), clippy on
  the pinned toolchain, nextest 3179 passed (only the 4 documented
  baseline failures), maintenance-test (105), ui-hot-path (6); the
  release build refreshed the `make install` symlink.
- Fork point: upstream `90e947a6` ("fix: bound mouse selection repaint
  cadence (#3901)").

### 2026-09-11 (upstream sync)

- Merged upstream/master `90e947a6..c7a7cc45` (14 commits). One conflict:
  justfile `maintenance-test` line (fork added `test_fork_changelog_check`,
  upstream added `test_windows_cross`) — resolved by keeping both scripts
  in the list; upstream's new `setup-windows-cross` / `windows-lint` /
  `libghostty-bindings` recipes taken as-is. `AGENTS.md`,
  `shell/config.rs`, `shell/state.rs` auto-merged in disjoint regions
  (vertical_tabs config threading + layout branch and the Fork rules
  section intact).

- Notable fixes absorbed: libghostty-vt upgrade + snapshot consistency
  while scrolling (#3906, #3928), conditional sidebar token hiding
  (#3925), CLI routing to saved SSH machines (#3918), cline launcher/idle
  detection (#3959), claude unicode spinner (#3953), Windows Codex paste
  flush (#3961), Windows lint all targets (#3963), local kitty graphics
  transport (#3957), plugin registry symlinks (#3927), zig version error
  message (#3931).
- New build prerequisite: Zig 0.16.0 — see Gotchas.
- Validation green: fork-map drift test, vertical_tabs (20/20), clippy on
  the pinned toolchain, nextest 3206/3208 (only the documented baseline
  failures), maintenance-test (incl. new `test_windows_cross`),
  ui-hot-path (6); the release build refreshed the `make install` symlink.
- Fork point: upstream `c7a7cc45` ("fix: lint all targets on windows
  (#3963)").

### 2026-09-11 (make zig pin)

- `make build`/`make install` broke right after the sync: the Makefile
  exported `ZIG` resolved to brew's zig 0.15.2 while upstream now pins
  0.16.0 — the make path never saw the sync-session env export. Build
  targets now prefer `~/zig-0.16.0/zig` (env/command-line `ZIG` still
  wins) and fall back to PATH/brew; `/sync-upstream` Phase 3 step 6 now
  verifies the build through `make build` + `make install` so the user
  path is covered on every sync.

### 2026-09-11 (strip collapse toggle)

- `prefix+b` (ToggleSidebar) did nothing in strip mode: the layout branch
  ignored `sidebar_collapsed` (deliberate at strip time — the strip
  replaced the sidebar wholesale). The strip now honors collapse: hidden
  entirely (a compact 4-column strip would only clip the boxes), same key
  restores the prior width; state persists through the existing
  chrome-preferences path. Replaced the
  `vertical_tabs_ignores_sidebar_collapse_...` test with collapse/restore
  + tab-bar-hiding-ignored coverage.

### 2026-09-16 (upstream sync)

- Merged upstream/master `c7a7cc45..b88e8116` (106 commits, incl. the
  v0.9.1 release-metadata sync). Two conflicts, both per-map:
  justfile `maintenance-test` (upstream added `test_release` +
  `test_windows_input` + bun release tests + a new `test-windows-input`
  recipe; kept all plus the fork's `test_fork_changelog_check`) and
  tabs.rs EOF (upstream appended a `max_tab_scroll` unit test and
  reworked `max_tab_scroll`/`last_visible_tab` + muted tab-label style;
  took upstream's changes, kept all 736 strip lines). Everything else
  auto-merged disjoint (`shell/config.rs`, `shell/state.rs`,
  `shell/mouse.rs`, `shell/render.rs`, `config/model.rs`,
  `config-reference.json`, `AGENTS.md`, `tests/mod.rs`); map drift test
  confirmed all mapped symbols intact.
- Notable fixes absorbed: stable release promotion from protected
  previews (#4241), Windows VT input recovery + mouse/pwsh fixes (#4269,
  #4257, #4295, #4297), cursor-based text input editing (#3698),
  ctrl-hover highlighting + wrapped links (#4074), whole-word selection
  during drag (#3969) + live selections during pane output (#4193),
  client location reconciliation after pane moves (#4171, #4159), saved
  machine screen restore (#4126) + stale surface rejection (#4141),
  overflowing tab strips reveal the full last tab (#4152), muted
  sidebar/tab label readability (#4062), machine connection metadata
  caching (#4256), exact session-name deletion (#4233), unloaded session
  preservation (#4190, #4125), kitty CSI-u + CSI-tilde F-keys (#2378,
  #4067), super-modified keys (#4044), pixel-mouse integer cell pitch
  (#4140), SSH bridge cleanup/backoff (#4083), Letta Code detection
  (#3107), opencode tui.json + v2 lifecycle (#4250, #3757), worktree
  removal improvements (#4238, #3315).
- Validation: drift test green, vertical_tabs 21/21, pinned clippy
  clean, nextest 3489/3495 (6 failures — all confirmed on unmerged
  trees after the macOS/Xcode update; see the known-failures note above,
  plus the documented `pane_info_and_subscriptions_...` baseline),
  maintenance-test 144/145 (upstream `test_release` hotfix test, fails
  on clean upstream too), ui-hot-path 6/6, `make build` + `make install`
  refreshed the daily symlink. The Xcode license had to be re-accepted
  mid-sync (`sudo xcodebuild -license accept`) before anything compiled.
- Fork point: upstream `b88e8116` ("test: add local windows input
  gauntlet (#4298)").

### 2026-09-21 (upstream sync)

- Merged upstream/master `b88e8116..28360107` (29 commits). Clean merge,
  zero conflicts: every overlapping file split into disjoint regions
  (`shell/mouse.rs` navigator scrollbar + navigator row clicks vs the
  strip's wheel/drop-index/clamp hunks; `shell/state.rs` navigator
  fields + tab-close confirmation vs the strip's config/layout fields;
  `config/model.rs` `clear_pane` keybinding vs `ui.vertical_tabs*`;
  `config-reference.json` `keys.clear_pane` vs the `ui.*` entries;
  `AGENTS.md` detection-testing policy vs the fork-rules section;
  `tests/mod.rs` `mod close_tab;`/`restore_error` vs `mod
  vertical_tabs;`). Upstream did not touch `tabs.rs`, `config.rs`,
  `render.rs`, `justfile`, or the Makefile in this range, and no
  `src/protocol/wire.rs` change means no protocol conflict to flag.
- Notable fixes absorbed: agent completion vs startup/session changes
  (#4457), droid scrollback-clear requests (#4432), forwarded ssh agent
  refresh after reconnect (#4443), per-endpoint navigator tab/pane
  indexing (#4426), session layout preservation across shutdown and
  restore failures (#4400), last-tab close confirmation (#4409),
  terminal-less attach rejection (#4395), event-subscription draining +
  history-loss reporting (#4225), explicit worktree workspace membership
  (#4301), navigator visibility in light themes (#4408), delayed mouse
  reports with confirmed keyboard input (#4247), codex custom interrupt
  keys (#4196), kiro live-control/OSC detection (#4372), cursor recovery
  during animated redraws (#4404), Windows cursor redraw + settle
  deadlines (#4389), go-to picker lists every agent and terminal (#4384),
  configurable pane screen/scrollback clearing with a new vendored
  libghostty-vt patch (#4383), sidebar agent reveal on cycle (#4355),
  socket error request-id preservation (#4353), ssh compression (#4340),
  grok detection with custom/disabled OSC signals (#4337), Windows mouse
  capture during refresh (#4319).
- Validation: drift test green, vertical_tabs 21/21, pinned clippy clean,
  nextest 3556/3557 (only the documented
  `pane_info_and_subscriptions_...` baseline), maintenance-test 148 with
  1 error (upstream `scripts.test_release` hotfix test under local git
  2.55.0, still failing the same way), ui-hot-path 6/6, `cargo fmt
  --check` clean, `make build` + `make install` refreshed the daily
  symlink.
- Environment note: this machine had lost both `cargo-nextest` and
  `just` between syncs (no binary anywhere on disk). Both were
  reinstalled with `cargo install --locked`; if the sync runbook reports
  `no such command: nextest` or `just: command not found`, reinstall
  rather than reworking the validation steps.
- Fork point: upstream `28360107` ("fix: distinguish agent completion
  from startup and session changes (#4457)").
