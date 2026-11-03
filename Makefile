PREFIX ?= /usr
APP_ID := io.github.hakergeniusz.Hashcheck

.PHONY: all build run install install-user uninstall clean

all: build

build:
	cargo build --release --locked

run:
	cargo run

install: build
	install -Dm0755 target/release/hashcheck $(DESTDIR)$(PREFIX)/bin/hashcheck
	install -Dm0644 data/$(APP_ID).desktop $(DESTDIR)$(PREFIX)/share/applications/$(APP_ID).desktop
	install -Dm0644 data/$(APP_ID).metainfo.xml $(DESTDIR)$(PREFIX)/share/metainfo/$(APP_ID).metainfo.xml
	install -Dm0644 data/$(APP_ID).gschema.xml $(DESTDIR)$(PREFIX)/share/glib-2.0/schemas/$(APP_ID).gschema.xml
	install -Dm0644 data/icons/$(APP_ID).svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/$(APP_ID).svg
	install -Dm0644 data/icons/$(APP_ID)-symbolic.svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps/$(APP_ID)-symbolic.svg
	-gtk-update-icon-cache -f $(DESTDIR)$(PREFIX)/share/icons/hicolor
	glib-compile-schemas $(DESTDIR)$(PREFIX)/share/glib-2.0/schemas/

# Installs to ~/.local — enough for the GSettings schema to be picked up.
install-user:
	$(MAKE) install DESTDIR= PREFIX=$(HOME)/.local

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/hashcheck
	rm -f $(DESTDIR)$(PREFIX)/share/applications/$(APP_ID).desktop
	rm -f $(DESTDIR)$(PREFIX)/share/metainfo/$(APP_ID).metainfo.xml
	rm -f $(DESTDIR)$(PREFIX)/share/glib-2.0/schemas/$(APP_ID).gschema.xml
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/$(APP_ID).svg
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/symbolic/apps/$(APP_ID)-symbolic.svg

clean:
	cargo clean
