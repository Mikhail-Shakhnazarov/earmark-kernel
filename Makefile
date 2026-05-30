# standard checks for earmark-kernel

.PHONY: default check lint test fmt-check verify help

default: help

help:
	@echo "Available targets:"
	@echo "  check      - Run basic build checks"
	@echo "  lint       - Run clippy with strict warnings"
	@echo "  test       - Run all workspace tests"
	@echo "  fmt-check  - Check formatting"
	@echo "  audit      - Run dependency and license audit"
	@echo "  verify     - Run all standard checks (check, lint, test, fmt-check, audit)"

check:
	cargo check --all-targets --all-features

lint:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all-targets --all-features

fmt-check:
	cargo fmt --all -- --check

bench:
	cargo bench

verify: check lint test fmt-check audit
	@echo "all checks passed"

audit:
	./scripts/audit_licenses.sh
