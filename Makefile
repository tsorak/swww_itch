INSTALL_DIR = /usr/bin
STATE_DIR = ${HOME}/.local/state/itch
DATABASE_URL = sqlite://${STATE_DIR}/state.db

devenv:
	@mkdir -p ${STATE_DIR}
	@sqlx database setup -D ${DATABASE_URL}

	@echo "DATABASE_URL=${DATABASE_URL}" > .env
	@echo DATABASE_URL=${DATABASE_URL}

build_itchd:
	@cargo build --release --bin swww-itchd

	# Setup persistent app state
	@mkdir -p -v ${STATE_DIR}
	@sqlx database setup -D ${DATABASE_URL}
	# Sqlite migration info:
	@sqlx migrate info -D ${DATABASE_URL}

build_itch:
	# Install deps
	@bun install
	# Build app
	@bun run tauri build -b deb

build: build_itchd build_itch


install_itchd: build_itchd
	@sudo install -Dvm755 ./target/release/swww-itchd $(INSTALL_DIR)/swww-itchd

install_itch: build_itch
	@sudo install -Dvm755 ./target/release/itch $(INSTALL_DIR)/itch

install: install_itchd install_itch

clean:
	@cargo clean
