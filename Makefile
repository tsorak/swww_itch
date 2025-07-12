INSTALL_DIR = /usr/bin

build_itchd:
	@cargo build --release --bin swww-itchd

build_itch:
	@bun run tauri build -b deb

build: build_itchd build_itch


install_itchd: build_itchd
	@sudo install -Dvm755 ./target/release/swww-itchd $(INSTALL_DIR)/swww-itchd

install_itch: build_itch
	@sudo install -Dvm755 ./target/release/itch $(INSTALL_DIR)/itch

install: install_itchd install_itch


clean:
	@cargo clean
