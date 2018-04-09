all: build

dev:
	cargo build

build:
	cargo build --release

install: build
	sudo install -o root -g root -m 0755 target/release/throttle-cpu-temp \
		/usr/local/bin/throttle-cpu-temp
	sudo install -o root -g root -m 0644 init/throttle-cpu-temp.service \
		/etc/systemd/system/throttle-cpu-temp.service
	sudo systemctl daemon-reload
	sudo systemctl enable --now throttle-cpu-temp

check:
	cargo check

clean:
	cargo clean

.PHONY: all check build install dev clean
