all: build install

build:
    cargo build --release

install: build
    sudo cp target/release/dotty-cli $HOME/.local/bin/dotty

uninstall:
    sudo rm /usr/local/bin/dotty

test:
    cargo test

check:
    cargo fmt && cargo clippy --tests --fix --allow-dirty
