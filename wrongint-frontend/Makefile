YARN := corepack yarn

.PHONY: ci
ci: install typecheck build

.PHONY: install
install:
	$(YARN) install --immutable

.PHONY: typecheck
typecheck:
	$(YARN) typecheck

.PHONY: build
build:
	$(YARN) build

.PHONY: dev
dev: install
	$(YARN) dev
