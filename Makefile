PREFIX ?= /usr/local
DESTDIR ?=
APP_ID = io.github.GreisMicha.downmark

build:
	cargo build --release --locked

install:
	install -Dm755 target/release/downmark $(DESTDIR)$(PREFIX)/bin/downmark
	sed 's|^Exec=downmark|Exec=$(PREFIX)/bin/downmark|' data/$(APP_ID).desktop > $(APP_ID).desktop.tmp
	install -Dm644 $(APP_ID).desktop.tmp $(DESTDIR)$(PREFIX)/share/applications/$(APP_ID).desktop
	rm $(APP_ID).desktop.tmp
	install -Dm644 data/$(APP_ID).svg $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/$(APP_ID).svg
	install -Dm644 data/$(APP_ID).metainfo.xml $(DESTDIR)$(PREFIX)/share/metainfo/$(APP_ID).metainfo.xml
	install -Dm644 LICENSE $(DESTDIR)$(PREFIX)/share/licenses/downmark/LICENSE

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/downmark
	rm -f $(DESTDIR)$(PREFIX)/share/applications/$(APP_ID).desktop
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/$(APP_ID).svg
	rm -f $(DESTDIR)$(PREFIX)/share/metainfo/$(APP_ID).metainfo.xml
	rm -f $(DESTDIR)$(PREFIX)/share/licenses/downmark/LICENSE

.PHONY: build install uninstall
