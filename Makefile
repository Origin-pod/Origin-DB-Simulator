# Makefile for DB-Simulator project

.PHONY: help build test check lint format clean bench doc run install

# Default target
help:
	@echo "Available targets:"
	@echo "  build    - Build the project"
	@echo "  test     - Run all tests"
	@echo "  check    - Check code without building"
	@echo "  lint     - Run clippy linter"
	@echo "  format   - Format code with rustfmt"
	@echo "  clean    - Clean build artifacts"
	@echo "  bench    - Run benchmarks"
	@echo "  doc      - Generate documentation"
	@echo "  run      - Run the simulator"
	@echo "  install  - Install required tools"

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test --all

# Check code without building
check:
	cargo check --all

# Run clippy linter
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Format code
format:
	cargo fmt --all

# Check if code is formatted
format-check:
	cargo fmt --all -- --check

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/
	rm -f flamegraph.svg
	rm -f perf.data*

# Run benchmarks
bench:
	cargo bench --all

# Generate documentation
doc:
	cargo doc --no-deps --open

# Run the simulator
run:
	cargo run --release

# Run with specific example
run-example:
	@if [ -z "$(EXAMPLE)" ]; then \
		echo "Usage: make run-example EXAMPLE=<example_name>"; \
	else \
		cargo run --example $(EXAMPLE); \
	fi

# Install required tools
install:
	rustup component add rustfmt
	rustup component add clippy
	cargo install cargo-watch
	cargo install cargo-expand
	cargo install flamegraph
	cargo install cargo-criterion

# Watch and rebuild on changes
watch:
	cargo watch -x build

# Watch and run tests on changes
watch-test:
	cargo watch -x test

# Run security audit
audit:
	cargo audit

# Generate flamegraph
flamegraph:
	cargo flamegraph --bench block_benchmarks

# Coverage report
coverage:
	cargo tarpaulin --out Html --output-dir coverage

# All checks (used in CI)
ci: format-check lint test

# Development setup
dev-setup: install
	@echo "Development environment setup complete!"
