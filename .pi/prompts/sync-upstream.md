---
description: Sync the fork with upstream herdr master — merge, resolve via FORK-CHANGELOG.md, validate, log
argument-hint: "[--dry-run]"
---

# Upstream sync for the kaushikgopal/herdr fork

Merge the latest `upstream/master` (https://github.com/ogulcancelik/herdr.git,
remote `upstream`) into this fork with zero loss of fork customizations.
`FORK-CHANGELOG.md` is the authoritative map of everything the fork carries —
read it fully before touching anything. Conflicts get resolved from that map
or flagged to the user, never guessed away. Merge, never rebase: fork
commits are already pushed, and the map is written to keep merges shallow.

## Phase 0 — Preconditions (stop when unmet)

1. `git status --porcelain` must be empty (locally-ignored `.agents/` state
   is fine). Dirty tree: stop and tell the user.
2. Current branch must be `master`; otherwise stop and ask.
3. `git fetch upstream --prune`, then record:
   - fork point: `git merge-base upstream/master master`
   - incoming: `git log --oneline <fork-point>..upstream/master`
4. No incoming commits: report "already up to date" and stop.
5. Force-push guard: if the fork point recorded in FORK-CHANGELOG.md's Log
   is not an ancestor of upstream/master
   (`git merge-base --is-ancestor <recorded-sha> upstream/master`),
   upstream rewrote history — flag it and ask before proceeding.

## Phase 1 — Impact preview

- Carried diff: `git diff --name-only <fork-point>..master`.
- Overlap: intersect that with
  `git diff --name-only <fork-point>..upstream/master` — intersections are
  conflict candidates.
- For each overlapping mapped file, read its FORK-CHANGELOG.md entry and
  note the "Re-check after upstream pulls" items.
- Present the plan: commit count, candidate files, and the expected
  strategy per file. Then:
  - `--dry-run` in `$ARGUMENTS`: stop here and report. Touch nothing.
  - Otherwise continue automatically; ambiguous conflicts still stop.

## Phase 2 — Merge

1. Rollback anchor: `git branch pre-sync/<date>` (if the name exists, use
   `pre-sync/<date>-<time>`). Report the anchor.
2. `git merge upstream/master --no-edit`. Clean merge: go to Phase 3.
3. Conflicts — resolve file by file, consulting the map entry first:
   - **Fork-owned files** (FORK-CHANGELOG.md, Makefile,
     `src/client/shell/tests/vertical_tabs.rs`): keep the fork side.
   - **Mapped shared files** (`shell/config.rs`, `shell/render.rs`,
     `shell/mouse.rs`, `shell/state.rs`, `config/model.rs`,
     `docs/next/website/src/data/config-reference.json`): take upstream's
     structure, then re-apply the fork's config-gated behavior exactly as
     the map describes it. Never delete upstream code paths.
   - **Protocol/wire/endpoint surfaces** (`src/protocol/wire.rs`,
     `PROTOCOL_VERSION`): STOP. Wire changes carry compatibility rules (see
     AGENTS.md). Show the conflict and ask.
   - **Anything unclear after reading the map**: STOP, show both sides next
     to the relevant map entry, ask the user. One file at a time.
4. Mid-merge disaster: `git merge --abort`, report, ask.
5. `cargo fmt` before staging resolutions.

## Phase 3 — Validate (in order; stop on the first real failure)

1. Map drift: `python3 -m unittest scripts.test_fork_changelog_check` —
   fails when upstream moved or renamed anything the map references. Update
   FORK-CHANGELOG.md (and the test's `MAPPED_SYMBOLS`) until green.
2. Fast fork signal: `cargo test --bin herdr vertical_tabs`.
3. Clippy on the pinned toolchain (plain `cargo clippy` uses Homebrew's
   newer clippy, which fails untouched upstream files):
   `PATH="$HOME/.rustup/toolchains/1.96.1-aarch64-apple-darwin/bin:$PATH" cargo clippy --all-targets --locked -- -D warnings`
4. Full suite: `cargo nextest run --locked --no-fail-fast`. Baseline
   failures documented in FORK-CHANGELOG.md (`live_handoff_*` ×3,
   `pane_info_and_subscriptions_expose_done_agent_status`, occasional
   `federated_launch_*` flake that passes isolated) are NOT regressions.
   Any other failure is: root-cause it before continuing.
5. `just maintenance-test` and `just ui-hot-path-architecture-test`.
6. Build through the user path so the Makefile's toolchain wiring gets
   exercised too: `make build` (debug profile; the Makefile resolves the
   pinned Rust toolchain and Zig itself), then `make install` (release
   build + refresh the daily `herdr` symlink). Plain `cargo` invocations
   in steps 2-4 need `ZIG=$HOME/zig-0.16.0/zig` exported — the vendored
   libghostty-vt pins Zig and build.rs hard-fails older versions; check
   FORK-CHANGELOG.md Gotchas for the current pins before assuming paths.

## Phase 4 — Paper trail + commit

- FORK-CHANGELOG.md: update the fork point in the Log; fix any Where/
  symbol references that moved; add a dated Log line — what was pulled and
  how each conflict was resolved.
- `.agents/dox/2026-09-09-herdr-vertical-split.md`: append a sync session
  note.
- One commit: `chore: sync upstream <upstream-short-sha> (<n> commits)` —
  body lists the notable upstream fixes absorbed. Do not push unless asked.

## Report

End with: merged range + notable upstream fixes; conflict table (file →
strategy, or "flagged for user"); validation results vs baseline; new fork
point; rollback anchor; anything left for human review.
