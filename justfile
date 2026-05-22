all: build install

build:
    cargo build --release

install: build
    sudo cp target/release/dotty $HOME/.local/bin/dotty

uninstall:
    sudo rm /usr/local/bin/dotty

test:
    cargo test
