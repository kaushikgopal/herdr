# Personal-fork helpers. The repo's canonical task runner is `just`; this
# Makefile only wraps the everyday build/run loop for this checkout.

# rust-toolchain.toml pins 1.96.1, but Homebrew's rustc is newer and fails
# `clippy -- -D warnings` on untouched upstream code. Prefer the matching
# rustup toolchain when it exists.
RUST_BIN := $(HOME)/.rustup/toolchains/1.96.1-aarch64-apple-darwin/bin
ifeq ($(wildcard $(RUST_BIN)/cargo),)
RUST_BIN :=
endif
export PATH := $(RUST_BIN):$(PATH)

# The vendored libghostty-vt builds through Zig 0.15, which Homebrew installs
# keg-only, so plain `zig` may not resolve.
export ZIG := $(shell command -v zig 2>/dev/null || echo /opt/homebrew/opt/zig@0.15/bin/zig)

GRAY := \033[1;30m
NC := \033[0m

.PHONY: help build build-release install run run-release run-dev stop-dev test check

help:
	@printf "%b\n" "$(GRAY)Targets:$(NC)"
	@printf "%b\n" "$(GRAY)  make build         # debug build (sandboxed binary in target/debug)$(NC)"
	@printf "%b\n" "$(GRAY)  make run           # debug build + attach (herdr-dev sandbox: own config/state)$(NC)"
	@printf "%b\n" "$(GRAY)  make build-release # release build (real config/sessions binary)$(NC)"
	@printf "%b\n" "$(GRAY)  make run-release   # release build + attach to the live stable server$(NC)"
	@printf "%b\n" "$(GRAY)  make run-dev       # isolated: fresh herdr-dev server, no inherited sockets$(NC)"
	@printf "%b\n" "$(GRAY)  make stop-dev      # quit the herdr-dev sandbox server$(NC)"
	@printf "%b\n" "$(GRAY)  make install       # symlink release binary as daily herdr ($$XDG_BIN_HOME)$(NC)"
	@printf "%b\n" "$(GRAY)  make test          # cargo nextest$(NC)"
	@printf "%b\n" "$(GRAY)  make check         # fmt + clippy -D warnings + nextest$(NC)"

build:
	cargo build

build-release:
	cargo build --release

# Symlink the release binary as the daily `herdr` command, into the XDG bin
# directory ($XDG_BIN_HOME, default ~/.local/bin). Needs to precede
# /opt/homebrew/bin on PATH to shadow the brew install; remove the symlink
# to fall back to it.
install: build-release
	@dest="$${XDG_BIN_HOME:-$$HOME/.local/bin}/herdr"; \
	if [ -e "$$dest" ] && [ ! -L "$$dest" ]; then \
		echo "refusing to overwrite real file at $$dest"; exit 1; \
	fi; \
	mkdir -p "$${XDG_BIN_HOME:-$$HOME/.local/bin}"; \
	ln -sfn $(CURDIR)/target/release/herdr $$dest && echo "installed: $$dest -> $(CURDIR)/target/release/herdr"

# Debug builds are sandboxed into ~/.config/herdr-dev (own config + state),
# so this never touches the real sessions.
run: build
	cargo run

# Release builds use the real ~/.config/herdr root, so run-release from a
# plain terminal tab attaches to the live server and sessions with this
# fork's UI.
run-release: build-release
	cargo run --release

# Isolated playground: clear inherited socket overrides so the debug binary
# spawns its own herdr-dev server instead of attaching to a running one.
run-dev: build
	env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo run

# Quit the debug-sandbox server (the make run / run-dev one).
stop-dev:
	env HERDR_SOCKET_PATH=$(HOME)/.config/herdr-dev/herdr.sock ./target/debug/herdr server stop

test:
	cargo nextest run --locked

check:
	cargo fmt --check
	cargo clippy --all-targets --locked -- -D warnings
	cargo nextest run --locked
