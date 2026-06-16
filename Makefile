SHELL := /bin/bash

PORT ?= 7777
HOST ?= 127.0.0.1
APP_BASE_URL ?= http://$(HOST):$(PORT)
PG_HOST ?= 127.0.0.1
PG_PORT ?= 8100
PG_DATABASE ?= northline_dev
PG_USER ?= moss
PG_PASSWORD ?= moss
PG_SCHEMA ?= portal
NORTHLINE_URL ?= http://127.0.0.1:6688
DOCUMIND_URL ?= http://127.0.0.1:5555
DEPLOY_HOST ?= northline
DEPLOY_PORT ?= $(PORT)
DEPLOY_PUBLIC_BASE_URL ?= http://47.94.2.8:6688
DEPLOY_TARGET ?= x86_64-unknown-linux-musl
DEPLOY_TARGET_DIR ?= target/deploy-linux-x86_64-musl
DEPLOY_BINARY ?= $(DEPLOY_TARGET_DIR)/$(DEPLOY_TARGET)/release/portal-api
DEPLOY_SEED_BINARY ?= $(DEPLOY_TARGET_DIR)/$(DEPLOY_TARGET)/release/portal-seed

.PHONY: install web-build dev-seed dev build deploy-build deploy status health logs clean

define check_port
	@if lsof -ti tcp:$(1) >/dev/null 2>&1; then \
		echo ""; \
		echo "  Port $(1) is already in use."; \
		lsof -nP -iTCP:$(1) -sTCP:LISTEN || true; \
		echo ""; \
		echo "  Stop that process, or run with custom port:"; \
		echo "    make dev PORT=7799"; \
		echo ""; \
		exit 1; \
	fi
endef

define portal_env
APP_ENV=development APP_PORT=$(PORT) APP_BASE_URL=$(APP_BASE_URL) PORTAL_ISSUER=$(APP_BASE_URL) \
PG_HOST=$(PG_HOST) PG_PORT=$(PG_PORT) PG_DATABASE=$(PG_DATABASE) PG_USER=$(PG_USER) PG_PASSWORD=$(PG_PASSWORD) PG_SCHEMA=$(PG_SCHEMA) PG_SSL=false \
NORTHLINE_ENTRY_URL=$(NORTHLINE_URL)/ NORTHLINE_CALLBACK_URL=$(NORTHLINE_URL)/auth/portal/callback \
DOCUMIND_ENTRY_URL=$(DOCUMIND_URL)/ DOCUMIND_CALLBACK_URL=$(DOCUMIND_URL)/auth/portal/callback \
RUST_LOG=portal_api=info,tower_http=info
endef

install:
	pnpm install

web-build: install
	pnpm --filter @portal/web build

dev-seed:
	$(portal_env) cargo run --bin portal-seed

dev: web-build dev-seed
	$(call check_port,$(PORT))
	@echo ""
	@echo "  Portal dev → $(APP_BASE_URL)"
	@echo "  PG namespace → $(PG_DATABASE).$(PG_SCHEMA)"
	@echo "  Northline → $(NORTHLINE_URL)"
	@echo "  DocuMind → $(DOCUMIND_URL)"
	@echo ""
	$(portal_env) cargo run --bin portal-api

build: web-build
	cargo build --release --bin portal-api --bin portal-seed

deploy-build: web-build
	DEPLOY_TARGET=$(DEPLOY_TARGET) DEPLOY_TARGET_DIR=$(DEPLOY_TARGET_DIR) scripts/build-linux.sh

deploy: deploy-build
	DEPLOY_HOST=$(DEPLOY_HOST) DEPLOY_PORT=$(DEPLOY_PORT) DEPLOY_PUBLIC_BASE_URL=$(DEPLOY_PUBLIC_BASE_URL) LOCAL_BINARY=$(DEPLOY_BINARY) LOCAL_SEED_BINARY=$(DEPLOY_SEED_BINARY) scripts/deploy.sh

status:
	ssh $(DEPLOY_HOST) 'bash -lc '"'"'set -euo pipefail; \
		echo "== portal process =="; pgrep -af "/opt/portal/.*/portal-api|portal-api" || true; \
		echo; echo "== portal port =="; (command -v ss >/dev/null && ss -ltnp | grep ":$(DEPLOY_PORT) " || true); \
		echo; echo "== nginx 6688 =="; (command -v ss >/dev/null && ss -ltnp | grep ":6688 " || true); \
		echo; echo "== logs =="; ls -lh /opt/portal/shared/logs 2>/dev/null || true'"'"''

health:
	ssh $(DEPLOY_HOST) 'bash -lc '"'"'set -euo pipefail; \
		curl -fsS http://127.0.0.1:$(DEPLOY_PORT)/api/health; echo; \
		curl -fsS -o /dev/null -w "%{http_code}\n" http://127.0.0.1:6688/'"'"''

logs:
	ssh $(DEPLOY_HOST) 'bash -lc '"'"'tail -n $${LINES:-300} -f /opt/portal/shared/logs/portal-$(DEPLOY_PORT).log'"'"''

clean:
	cargo clean
	rm -rf apps/web/out apps/web/.next
