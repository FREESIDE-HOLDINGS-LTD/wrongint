.PHONY: dev backend frontend install

# Bring up the whole stack for local testing:
#   backend  -> http://localhost:8080  (API + /metrics + /docs)
#   frontend -> http://localhost:5173  (Vue dev server, proxies /api)
dev: install
	$(MAKE) -j2 backend frontend

backend:
	cd wrongint-backend && RUST_LOG=info,wrongint_backend=debug cargo run -- run local_config.toml --sample-now

frontend:
	cd wrongint-frontend && corepack yarn dev

install:
	cd wrongint-frontend && [ -d node_modules ] || corepack yarn install
