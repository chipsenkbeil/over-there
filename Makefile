.PHONY: all build clean release

all: clean build

build:
	@cargo build

release:
	@cargo build --release
	@strip target/release/over-there

clean:
	@rm -rf target/
