BINARY     := vault-watch
INSTALL    := /usr/local/bin
SERVICE    := /etc/systemd/system
OPENRC_DIR := /etc/init.d

.PHONY: build build-static install install-static install-service uninstall clean

# Build steps — run as normal user (requires cargo/rustup in PATH)
build:
	cargo build --release

build-static:
	cargo build --release --target x86_64-unknown-linux-musl

# Install steps — run with sudo (no cargo needed, binary must already be built)
install:
	install -Dm755 target/release/$(BINARY) $(INSTALL)/$(BINARY)

install-static:
	install -Dm755 target/x86_64-unknown-linux-musl/release/$(BINARY) $(INSTALL)/$(BINARY)

install-service:
	@if command -v systemctl >/dev/null 2>&1; then \
		sed 's#__BINARY_PATH__#$(INSTALL)/$(BINARY)#g' contrib/vault-watch.service > /tmp/$(BINARY).service; \
		install -Dm644 /tmp/$(BINARY).service $(SERVICE)/$(BINARY).service; \
		rm -f /tmp/$(BINARY).service; \
		systemctl daemon-reload; \
		echo "Installed systemd service. Enable with: systemctl enable --now $(BINARY)"; \
	elif [ -d /etc/init.d ]; then \
		sed 's#__BINARY_PATH__#$(INSTALL)/$(BINARY)#g' contrib/vault-watch.openrc > /tmp/$(BINARY).openrc; \
		install -Dm755 /tmp/$(BINARY).openrc $(OPENRC_DIR)/$(BINARY); \
		rm -f /tmp/$(BINARY).openrc; \
		echo "Installed OpenRC service. Enable with: rc-update add $(BINARY) default"; \
	else \
		echo "No supported init system found (systemd or OpenRC)"; \
	fi

uninstall:
	rm -f $(INSTALL)/$(BINARY)
	rm -f $(SERVICE)/$(BINARY).service
	rm -f $(OPENRC_DIR)/$(BINARY)

clean:
	cargo clean
