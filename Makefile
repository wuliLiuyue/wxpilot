.PHONY: build test lint rust-cli-build rust-cli-help rust-test

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
