.FORCE: ;

target/debug/libmy_jay_config.so: .FORCE
	cargo build

install: target/debug/libmy_jay_config.so
	mkdir -p ~/.config/jay
	rm -f ~/.config/jay/config.so
	cp target/debug/libmy_jay_config.so ~/.config/jay/config.so
