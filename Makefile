.PHONY: build test lint rust-cli-build rust-cli-help rust-test install-test

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

install-test:
	sh tests/install/package_macos_smoke.sh
	sh tests/install/install_smoke.sh
	sh tests/install/e2e_release_install_smoke.sh
	sh tests/install/readme_migration_check.sh
