.PHONY: dev build

dev:
	~/go/bin/wails dev -tags webkit2_41

build:
	~/go/bin/wails build -tags webkit2_41