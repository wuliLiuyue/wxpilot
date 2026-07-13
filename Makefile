.PHONY: build test lint rust-cli-build rust-cli-help rust-test local-build local-install local-install-test local-install-all public-release-package public-distribution-test
build:
	pnpm build

test:
	pnpm test

lint:
	pnpm lint

rust-cli-build:
	cd rust && cargo build -p wxp-cli --bin wxpilot

rust-cli-help:
	cd rust && cargo run -p wxp-cli --bin wxpilot -- --help

rust-test:
	cd rust && cargo test --workspace

local-build:
	bash scripts/local/build-macos-binaries.sh --all

local-install:
	bash scripts/local/install-macos-binaries.sh

local-install-test:
	cd rust && cargo test -p wxp-cli locate_daemon_binary_ -- --nocapture
	cd rust && cargo test -p wxp-cli --test readme_migration
	bash scripts/verify/local-build-smoke.sh
	bash scripts/verify/local-install-smoke.sh

local-install-all:
	$(MAKE) local-build
	$(MAKE) local-install

public-release-package:
	@if [ -z "$(VERSION)" ]; then \
		echo "usage: make public-release-package VERSION=v0.1.0 [OUT_DIR=dist/public-release/v0.1.0]" >&2; \
		exit 1; \
	fi
	bash scripts/release/package-public-release.sh --version "$(VERSION)" $(if $(OUT_DIR),--output-dir "$(OUT_DIR)",)

public-distribution-test:
	bash scripts/verify/public-release-package-smoke.sh
	bash scripts/verify/install-script-smoke.sh
