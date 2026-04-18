.PHONY: build test lint rust-cli-build rust-cli-help rust-test local-build local-install local-install-test local-install-all license-compose-up license-compose-down license-compose-logs license-compose-ps license-seed license-backup license-init-test-env release-env-test

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
	bash scripts/verify/local-build-smoke.sh
	bash scripts/verify/release-env-smoke.sh
	bash scripts/verify/local-install-smoke.sh

local-install-all:
	$(MAKE) local-build
	$(MAKE) local-install

license-compose-up:
	bash scripts/license/deploy-compose.sh up

license-compose-down:
	bash scripts/license/deploy-compose.sh down

license-compose-logs:
	bash scripts/license/deploy-compose.sh logs

license-compose-ps:
	bash scripts/license/deploy-compose.sh ps

license-seed:
	@if [ -z "$(DB)" ] || [ -z "$(KEY)" ]; then \
		echo "usage: make license-seed DB=./data/license.db KEY=lic-e2e-001 [STATUS=active]" >&2; \
		exit 1; \
	fi
	bash scripts/license/seed-sqlite-license.sh "$(DB)" "$(KEY)" "$(or $(STATUS),active)"

license-backup:
	@if [ -z "$(DB)" ]; then \
		echo "usage: make license-backup DB=./data/license.db [ENV_FILE=./.env] [OUT_DIR=./backups]" >&2; \
		exit 1; \
	fi
	bash scripts/license/backup-sqlite-license.sh "$(DB)" "$(or $(ENV_FILE),./.env)" "$(or $(OUT_DIR),./backups)"

license-init-test-env:
	@if [ -z "$(KEY)" ]; then \
		echo "usage: make license-init-test-env KEY=lic-e2e-001 [DB=./data/license.db]" >&2; \
		exit 1; \
	fi
	bash scripts/license/init-test-env.sh "$(KEY)" "$(or $(DB),./data/license.db)"

release-env-test:
	bash scripts/verify/release-env-smoke.sh
