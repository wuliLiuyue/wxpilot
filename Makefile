.PHONY: build test lint rust-cli-build rust-cli-help rust-test local-build local-install local-install-test local-install-all

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
	cd rust && cargo test -p wxp-cli --test help_version --test command_parse --test project_detect --test daemon_client --test rpc_client --test output_parity --test runner_mapping --test status_smoke --test readme_migration
	cd rust && cargo test -p wxp-common --test contract

local-build:
	bash scripts/local/build-macos-binaries.sh --all

local-install:
	bash scripts/local/install-macos-binaries.sh

local-install-test:
	cd rust && cargo test -p wxp-cli locate_daemon_binary_ -- --nocapture
	cd rust && cargo test -p wxp-cli --test readme_migration
	bash tests/install/local_build_smoke.sh
	bash tests/install/local_install_smoke.sh
	bash tests/install/local_doc_smoke.sh

local-install-all:
	$(MAKE) local-build
	$(MAKE) local-install
