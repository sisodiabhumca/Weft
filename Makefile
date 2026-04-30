# Weft Terminal Makefile

.PHONY: all build clean test install uninstall run dev docs format lint

# Default target
all: build

# Build the project
build:
	@echo "Building Weft Terminal..."
	cargo build --release

# Development build
dev:
	@echo "Building Weft Terminal (debug)..."
	cargo build

# Run tests
test:
	@echo "Running tests..."
	cargo test

# Run tests with coverage
test-coverage:
	@echo "Running tests with coverage..."
	cargo tarpaulin --out Html

# Install the binary
install: build
	@echo "Installing Weft Terminal..."
	sudo cp target/release/weft /usr/local/bin/
	sudo chmod +x /usr/local/bin/weft

# Uninstall the binary
uninstall:
	@echo "Uninstalling Weft Terminal..."
	sudo rm -f /usr/local/bin/weft

# Run the terminal
run: build
	@echo "Starting Weft Terminal..."
	./target/release/weft

# Run in development mode
dev-run: dev
	@echo "Starting Weft Terminal (development)..."
	./target/debug/weft

# Generate documentation
docs:
	@echo "Generating documentation..."
	cargo doc --no-deps

# Open documentation in browser
docs-open: docs
	cargo doc --no-deps --open

# Format code
format:
	@echo "Formatting code..."
	cargo fmt

# Lint code
lint:
	@echo "Linting code..."
	cargo clippy -- -D warnings

# Check for security vulnerabilities
audit:
	@echo "Auditing dependencies..."
	cargo audit

# Update dependencies
update:
	@echo "Updating dependencies..."
	cargo update

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# Create release package
package: build
	@echo "Creating release package..."
	mkdir -p dist
	cp target/release/weft dist/
	tar -czf weft-$(shell cargo metadata --no-deps --format-version 1 | grep -o '"version":"[^"]*"' | cut -d'"' -f4)-$(shell uname -s | tr '[:upper:]' '[:lower:]')-$(shell uname -m).tar.gz -C dist weft

# Install development dependencies
setup-dev:
	@echo "Setting up development environment..."
	rustup component add rustfmt clippy
	cargo install cargo-tarpaulin cargo-audit

# Benchmarks
bench:
	@echo "Running benchmarks..."
	cargo bench

# Check formatting
check-format:
	@echo "Checking code formatting..."
	cargo fmt -- --check

# Check linting
check-lint:
	@echo "Running clippy checks..."
	cargo clippy -- -D warnings

# Full CI pipeline
ci: check-format check-lint test audit
	@echo "CI pipeline completed successfully!"

# Development helpers
watch:
	@echo "Watching for changes..."
	cargo watch -x run

# Build with all features
build-full:
	@echo "Building with all features..."
	cargo build --release --all-features

# Install from local source
install-local: build
	@echo "Installing from local source..."
	mkdir -p ~/.local/bin
	cp target/release/weft ~/.local/bin/
	chmod +x ~/.local/bin/weft

# Create desktop entry
desktop-entry:
	@echo "Creating desktop entry..."
	mkdir -p ~/.local/share/applications
	cat > ~/.local/share/applications/weft.desktop << EOF
[Desktop Entry]
Name=Weft Terminal
Comment=Next-generation AI-powered terminal
Exec=$(HOME)/.local/bin/weft
Icon=$(HOME)/.local/share/weft/icon.png
Terminal=false
Type=Application
Categories=Development;System;TerminalEmulator;
EOF

# Install icon
install-icon:
	@echo "Installing icon..."
	mkdir -p ~/.local/share/weft
	cp assets/icon.png ~/.local/share/weft/icon.png

# Full installation from source
install-full: install-local install-icon desktop-entry
	@echo "Full installation completed!"

# Help
help:
	@echo "Available targets:"
	@echo "  build          - Build the project"
	@echo "  dev            - Development build"
	@echo "  test           - Run tests"
	@echo "  test-coverage  - Run tests with coverage"
	@echo "  install        - Install the binary"
	@echo "  uninstall      - Uninstall the binary"
	@echo "  run            - Run the terminal"
	@echo "  dev-run        - Run in development mode"
	@echo "  docs           - Generate documentation"
	@echo "  docs-open      - Open documentation in browser"
	@echo "  format         - Format code"
	@echo "  lint           - Lint code"
	@echo "  audit          - Check security vulnerabilities"
	@echo "  update         - Update dependencies"
	@echo "  clean          - Clean build artifacts"
	@echo "  package        - Create release package"
	@echo "  setup-dev      - Install development dependencies"
	@echo "  bench          - Run benchmarks"
	@echo "  check-format   - Check code formatting"
	@echo "  check-lint     - Run clippy checks"
	@echo "  ci             - Full CI pipeline"
	@echo "  watch          - Watch for changes"
	@echo "  build-full     - Build with all features"
	@echo "  install-local  - Install from local source"
	@echo "  desktop-entry  - Create desktop entry"
	@echo "  install-icon   - Install icon"
	@echo "  install-full   - Full installation from source"
	@echo "  help           - Show this help message"
