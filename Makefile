# RNF Makefile

BINARY   := target/release/rnf
MUSL_TARGET := x86_64-unknown-linux-musl

.PHONY: all build release musl test fmt lint clean install demo

all: build

build:
	cargo build

release:
	cargo build --release

musl:
	rustup target add $(MUSL_TARGET)
	RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target $(MUSL_TARGET)
	@echo "Static musl binary: target/$(MUSL_TARGET)/release/rnf"

test:
	cargo test
	$(BINARY) --run examples/hello.rnf
	$(BINARY) --run examples/system.rnf

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

clean:
	cargo clean

install: release
	install -m755 $(BINARY) /usr/local/bin/rnf
	@echo "rnf installed to /usr/local/bin/rnf"

demo: build
	@echo "=== Hello World ==="
	$(BINARY) --run examples/hello.rnf
	@echo ""
	@echo "=== System Automation ==="
	$(BINARY) --run examples/system.rnf
	@echo ""
	@echo "=== Hardware Demo ==="
	$(BINARY) --run examples/hardware.rnf
