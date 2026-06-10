# User Story: US-MON-16 — Static Binary & Alpine / Docker Support

**Status:** 🔵 Sprint 04
**Sprint:** [Sprint 04](../sprint-backlogs/sprint-04.md)
**Epic:** [Platform Support](../01-product-backlog.md)

---

## 📖 Description

**ในฐานะ** ผู้ดูแลระบบที่รัน Alpine Linux หรือ Docker container
**ฉันต้องการ** binary ที่ compile ด้วย musl libc (static linked)
**เพื่อให้** รัน VaultWatch ได้โดยไม่ต้องพึ่ง glibc ที่ Alpine ไม่มี

---

## ✅ Acceptance Criteria

1. [ ] `make build` → dynamic binary (glibc) สำหรับ Ubuntu/Fedora/Arch
2. [ ] `make build-static` → static musl binary รันได้บน Alpine 3.19+ โดยไม่ error
3. [ ] `make install` → copy binary ไปยัง `/usr/local/bin/vault-watch` + สร้าง systemd service file
4. [ ] Static binary file size ไม่เกิน 20 MB
5. [ ] `make install-service` → enable และ start systemd service

---

## 🛠 Technical Tasks

- [ ] เพิ่ม `Makefile` ที่ root:
  ```makefile
  build:
      cargo build --release

  build-static:
      cargo build --release --target x86_64-unknown-linux-musl

  install: build
      sudo install -m 755 target/release/vault-watch /usr/local/bin/

  install-static: build-static
      sudo install -m 755 target/x86_64-unknown-linux-musl/release/vault-watch /usr/local/bin/

  install-service: install
      sudo install -m 644 contrib/vault-watch.service /etc/systemd/system/
      sudo systemctl daemon-reload
      sudo systemctl enable --now vault-watch
  ```
- [ ] เพิ่ม `.cargo/config.toml`:
  ```toml
  [target.x86_64-unknown-linux-musl]
  linker = "x86_64-linux-musl-gcc"
  ```
- [ ] สร้าง `contrib/vault-watch.service` — systemd unit file สำหรับ run as monitoring service
- [ ] สร้าง `contrib/vault-watch.openrc` — OpenRC init script สำหรับ Alpine
- [ ] ทดสอบ: `docker run --rm -it alpine:3.19 sh` แล้วรัน static binary ได้จริง
- [ ] อัปเดต README: ขั้นตอน install musl toolchain (`rustup target add x86_64-unknown-linux-musl`)

---

## 📋 musl Build Prerequisites

```bash
# Ubuntu/Debian — install musl toolchain
sudo apt install musl-tools
rustup target add x86_64-unknown-linux-musl

# Arch Linux
sudo pacman -S musl
rustup target add x86_64-unknown-linux-musl

# Build static binary
make build-static
```

---

## 🔗 Related Files

- Backlog: [01-product-backlog.md](../01-product-backlog.md)
- Sprint: [../sprint-backlogs/sprint-04.md](../sprint-backlogs/sprint-04.md)
- Related: [US-MON-17](./US-MON-17.md) (README มี build instructions)
