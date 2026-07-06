.PHONY: all build release test check clean fmt lint doc run install docker help

APP_NAME = tuba

all: build

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

check:
	cargo check

clean:
	cargo clean

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

doc:
	cargo doc --no-deps --open

run:
	cargo run -- $(ARGS)

install: release
	cp target/release/$(APP_NAME) ~/.local/bin/

docker:
	docker build -t $(APP_NAME) .

docker-run:
	docker run --rm -it $(APP_NAME) $(ARGS)

# Release helper: updates version and creates a git tag
# Usage: make release-version VERSION=1.1.0
release-version:
	@if [ -z "$(VERSION)" ]; then echo "Usage: make release-version VERSION=x.y.z"; exit 1; fi
	@echo "Updating version to $(VERSION)"
	@sed -i 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	@sed -i 's/version = ".*"/version = "$(VERSION)"/' src/main.rs
	@echo "Version updated to $(VERSION)"
	@echo "Run: git add -A && git commit -m 'chore: release v$(VERSION)' && git tag v$(VERSION)"

help:
	@echo "Usage:"
	@echo "  make build          - Build in debug mode"
	@echo "  make release        - Build in release mode (LTO optimized)"
	@echo "  make test           - Run all tests"
	@echo "  make check          - Check compilation without building"
	@echo "  make clean          - Remove build artifacts"
	@echo "  make fmt            - Format code with rustfmt"
	@echo "  make lint           - Run clippy lints"
	@echo "  make doc            - Build and open documentation"
	@echo "  make run ARGS=url   - Run tuba with optional URL"
	@echo "  make install        - Install binary to ~/.local/bin/"
	@echo "  make docker         - Build Docker image"
	@echo "  make docker-run ARGS=url - Run in Docker"
	@echo "  make release-version VERSION=x.y.z - Bump version and create tag"
