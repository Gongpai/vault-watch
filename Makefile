BINARY     := vault-watch
INSTALL    := /usr/local/bin
SERVICE    := /etc/systemd/system
OPENRC_DIR := /etc/init.d

.PHONY: build build-static install install-service uninstall clean

build:
	cargo build --release

build-static:
	cargo build --release --target x86_64-unknown-linux-musl

install: build
	install -Dm755 target/release/$(BINARY) $(INSTALL)/$(BINARY)

install-static: build-static
	install -Dm755 target/x86_64-unknown-linux-musl/release/$(BINARY) $(INSTALL)/$(BINARY)

install-service: install
	@if command -v systemctl >/dev/null 2>&1; then \
		install -Dm644 contrib/vault-watch.service $(SERVICE)/$(BINARY).service; \
		systemctl daemon-reload; \
		echo "Installed systemd service. Enable with: systemctl enable --now $(BINARY)"; \
	elif [ -d /etc/init.d ]; then \
		install -Dm755 contrib/vault-watch.openrc $(OPENRC_DIR)/$(BINARY); \
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
