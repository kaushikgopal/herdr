from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CHANGELOG = REPO_ROOT / "FORK-CHANGELOG.md"

# Load-bearing fork symbols mapped by FORK-CHANGELOG.md's "Where" sections.
# Keep in sync with that file: when the changelog map gains a symbol, add it
# here; when this test fails after an upstream pull, a symbol moved or died
# and the map needs updating.
MAPPED_SYMBOLS: dict[str, tuple[str, ...]] = {
    "Makefile": ("ZIG_016",),
    "src/client/shell/tabs.rs": (
        "render_tab_strip",
        "render_tab_strip_group",
        "render_strip_line_with_badge",
        "render_strip_title_row",
        "strip_cwd_leaf",
        "ellipsize_tail",
        "detail_segments",
        "draw_strip_box",
        "strip_agent_name",
        "max_tab_strip_scroll",
        "strip_scroll_revealing",
        "overflow_tab_strip",
        "tab_drop_indicator_y",
    ),
    "src/client/shell/config.rs": ("vertical_tabs", "vertical_tabs_compact"),
    "src/client/shell/state.rs": ("vertical_tabs", "vertical_tabs_compact"),
    "src/client/shell/render.rs": ("tab_strip",),
    "src/client/shell/mouse.rs": ("tab_drop_index_at", "cycle_strip_tab"),
    "src/config/model.rs": ("vertical_tabs", "vertical_tabs_compact"),
}


def changelog_text() -> str:
    return CHANGELOG.read_text(encoding="utf-8")


class ForkChangelogCheck(unittest.TestCase):
    """Guards the fork customization map against upstream drift.

    After pulling upstream, refactors rename or move fork integration
    points. These tests fail fast and point at the stale map entry instead
    of letting the fork's customizations silently rot. Run via
    `just maintenance-test` or `/sync-upstream` (phase 3).
    """

    def test_changelog_exists_with_required_sections(self) -> None:
        text = changelog_text()
        for section in ("Customization map", "Re-check after upstream pulls", "Log"):
            self.assertIn(section, text, "FORK-CHANGELOG.md is missing a required section")
        self.assertRegex(text.lower(), r"fork point")

    def test_mapped_files_exist(self) -> None:
        paths = set(re.findall(r"`((?:src|docs)/[\w/.\-]+)`", changelog_text()))
        self.assertTrue(paths, "no mapped paths found — is FORK-CHANGELOG.md empty?")
        missing = sorted(
            path for path in paths if not (REPO_ROOT / path).exists()
        )
        self.assertEqual(
            missing,
            [],
            "paths referenced by FORK-CHANGELOG.md no longer exist — update the map",
        )

    def test_mapped_symbols_still_exist(self) -> None:
        for rel_path, symbols in MAPPED_SYMBOLS.items():
            source = (REPO_ROOT / rel_path).read_text(encoding="utf-8")
            for symbol in symbols:
                self.assertIn(
                    symbol,
                    source,
                    f"{rel_path} no longer defines {symbol!r} — "
                    "upstream drifted; update FORK-CHANGELOG.md and re-apply the fork behavior",
                )

    def test_agents_md_requires_changelog_tracking(self) -> None:
        agents = (REPO_ROOT / "AGENTS.md").read_text(encoding="utf-8")
        self.assertIn(
            "FORK-CHANGELOG.md",
            agents,
            "AGENTS.md fork rules must keep requiring changelog tracking",
        )


class MakefileZigSelection(unittest.TestCase):
    def test_compiler_selection(self) -> None:
        make = shutil.which("make")
        self.assertIsNotNone(make)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            formula = root / "formula"
            (formula / "bin").mkdir(parents=True)
            (formula / "bin/zig").touch()
            # A former manual install must not override the managed compiler.
            (root / "zig-0.16.0").mkdir()
            (root / "zig-0.16.0/zig").touch()
            scripts = {
                "brew": 'test "$*" = "--prefix zig@0.16" || exit 2\n'
                        'test -n "$TEST_BREW_PREFIX" || exit 1\n'
                        'printf "%s\\n" "$TEST_BREW_PREFIX"\n',
                "cargo": 'printf "%s\\n" "$ZIG"\n',
                "zig": "exit 0\n",
            }
            for name, body in scripts.items():
                script = bin_dir / name
                script.write_text("#!/bin/sh\n" + body, encoding="utf-8")
                script.chmod(0o755)
            env = {
                "HOME": str(root),
                "PATH": f"{bin_dir}{os.pathsep}/usr/bin{os.pathsep}/bin",
            }
            cases = (
                ("homebrew", str(formula), None, [], str(formula / "bin/zig")),
                ("path fallback", "", None, [], str(bin_dir / "zig")),
                ("environment override", str(formula), "/explicit/zig", [], "/explicit/zig"),
                ("command-line override", str(formula), "/env/zig", ["ZIG=/cli/zig"], "/cli/zig"),
            )
            for name, prefix, override, args, expected in cases:
                with self.subTest(name=name):
                    case_env = {**env, "TEST_BREW_PREFIX": prefix}
                    if override is not None:
                        case_env["ZIG"] = override
                    result = subprocess.run(
                        [make, "--no-print-directory", "-s", "-f", str(REPO_ROOT / "Makefile"), "build", *args],
                        env=case_env,
                        cwd=root,
                        capture_output=True,
                        text=True,
                        check=True,
                    )
                    self.assertEqual(result.stdout.strip(), expected)



if __name__ == "__main__":
    unittest.main()
