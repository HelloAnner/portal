#!/usr/bin/env bash
set -euo pipefail

DEPLOY_HOST="${DEPLOY_HOST:-northline}"
DEPLOY_PORT="${DEPLOY_PORT:-7777}"
DEPLOY_PUBLIC_BASE_URL="${DEPLOY_PUBLIC_BASE_URL:-http://47.94.2.8:6688}"
REMOTE_ROOT="${REMOTE_ROOT:-/opt/portal}"
DEPLOY_TARGET="${DEPLOY_TARGET:-x86_64-unknown-linux-musl}"
LOCAL_BINARY="${LOCAL_BINARY:-target/deploy-linux-x86_64-musl/$DEPLOY_TARGET/release/portal-api}"
LOCAL_SEED_BINARY="${LOCAL_SEED_BINARY:-target/deploy-linux-x86_64-musl/$DEPLOY_TARGET/release/portal-seed}"
RELEASE_ID="${RELEASE_ID:-$(date +%Y%m%d-%H%M%S)}"

if [[ "$DEPLOY_HOST" != "northline" && "${ALLOW_CUSTOM_DEPLOY_HOST:-0}" != "1" ]]; then
  echo "Refusing non-default deploy host: $DEPLOY_HOST"
  echo "Set ALLOW_CUSTOM_DEPLOY_HOST=1 if you really want to override ssh northline."
  exit 1
fi

if [[ ! -x "$LOCAL_BINARY" ]]; then
  echo "Missing deploy binary: $LOCAL_BINARY"
  echo "Run: make deploy-build"
  exit 1
fi
if [[ ! -x "$LOCAL_SEED_BINARY" ]]; then
  echo "Missing deploy seed binary: $LOCAL_SEED_BINARY"
  echo "Run: make deploy-build"
  exit 1
fi

binary_info="$(file "$LOCAL_BINARY")"
if ! echo "$binary_info" | grep -qi 'ELF.*x86-64'; then
  echo "Deploy binary must be a Linux x86_64 ELF executable:"
  echo "$binary_info"
  exit 1
fi
if echo "$binary_info" | grep -qi 'interpreter '; then
  echo "Deploy binary must be fully static and must not require a dynamic loader:"
  echo "$binary_info"
  exit 1
fi
seed_binary_info="$(file "$LOCAL_SEED_BINARY")"
if ! echo "$seed_binary_info" | grep -qi 'ELF.*x86-64'; then
  echo "Deploy seed binary must be a Linux x86_64 ELF executable:"
  echo "$seed_binary_info"
  exit 1
fi
if echo "$seed_binary_info" | grep -qi 'interpreter '; then
  echo "Deploy seed binary must be fully static and must not require a dynamic loader:"
  echo "$seed_binary_info"
  exit 1
fi

REMOTE_RELEASE="$REMOTE_ROOT/releases/$RELEASE_ID"
REMOTE_CURRENT="$REMOTE_ROOT/current"
REMOTE_SHARED="$REMOTE_ROOT/shared"
REMOTE_ENV="$REMOTE_SHARED/.env"
REMOTE_LOG="$REMOTE_SHARED/logs/portal-$DEPLOY_PORT.log"
REMOTE_PID="$REMOTE_SHARED/runtime/portal-$DEPLOY_PORT.pid"
TMP_ENV="$(mktemp)"
trap 'rm -f "$TMP_ENV"' EXIT

remote_env_content="$(ssh "$DEPLOY_HOST" "test -f '$REMOTE_ENV' && cat '$REMOTE_ENV' || true" 2>/dev/null || true)"
existing_session_cookie="$(printf '%s\n' "$remote_env_content" | grep -E '^SESSION_COOKIE_NAME=' | tail -1 | cut -d= -f2- || true)"

cat > "$TMP_ENV" <<ENV
# Managed by scripts/deploy.sh. Edit on server only when changing runtime config.
APP_ENV=production
APP_PORT=$DEPLOY_PORT
APP_BASE_URL=$DEPLOY_PUBLIC_BASE_URL
PORTAL_ISSUER=$DEPLOY_PUBLIC_BASE_URL
PORTAL_TOKEN_TTL_SECONDS=300

PG_HOST=127.0.0.1
PG_PORT=8100
PG_DATABASE=northline_dev
PG_USER=northline
PG_PASSWORD=northline
PG_SCHEMA=portal
PG_SSL=false

SESSION_COOKIE_NAME=${existing_session_cookie:-portal_session}
SESSION_TTL_SECONDS=28800
REMEMBER_ME_TTL_SECONDS=2592000
AUDIT_RETENTION_DAYS=365
ALLOW_PERMISSION_REQUEST=true
RUST_LOG=portal_api=info,tower_http=info
ENV

echo "Deploying Portal to ssh $DEPLOY_HOST"
echo "Release: $RELEASE_ID"
echo "Port: $DEPLOY_PORT"
echo "Public base URL: $DEPLOY_PUBLIC_BASE_URL"

ssh "$DEPLOY_HOST" "bash -s" <<REMOTE
set -euo pipefail
mkdir -p '$REMOTE_RELEASE/bin' '$REMOTE_SHARED/logs' '$REMOTE_SHARED/runtime'
REMOTE

scp "$LOCAL_BINARY" "$DEPLOY_HOST:$REMOTE_RELEASE/bin/portal-api"
scp "$LOCAL_SEED_BINARY" "$DEPLOY_HOST:$REMOTE_RELEASE/bin/portal-seed"
scp "$TMP_ENV" "$DEPLOY_HOST:$REMOTE_ENV"

ssh "$DEPLOY_HOST" "bash -s" <<REMOTE
set -euo pipefail

remote_release='$REMOTE_RELEASE'
remote_current='$REMOTE_CURRENT'
remote_shared='$REMOTE_SHARED'
remote_env='$REMOTE_ENV'
remote_log='$REMOTE_LOG'
remote_pid='$REMOTE_PID'
deploy_port='$DEPLOY_PORT'
public_base='$DEPLOY_PUBLIC_BASE_URL'

chmod 600 "\$remote_env"
chmod +x "\$remote_release/bin/portal-api"
chmod +x "\$remote_release/bin/portal-seed"
ln -sfn "\$remote_release" "\$remote_current"

if ! docker ps --format '{{.Names}}' | grep -qx northline-postgres; then
  echo "northline-postgres container is required but not running."
  exit 1
fi
docker exec northline-postgres pg_isready -U northline -d northline_dev >/dev/null

cd "\$remote_shared"
set -a
source "\$remote_env"
set +a

"\$remote_current/bin/portal-seed"

if [[ -f "\$remote_pid" ]]; then
  old_pid="\$(cat "\$remote_pid" 2>/dev/null || true)"
  if [[ -n "\$old_pid" ]] && kill -0 "\$old_pid" 2>/dev/null; then
    kill "\$old_pid" || true
    for _ in 1 2 3 4 5; do
      kill -0 "\$old_pid" 2>/dev/null || break
      sleep 1
    done
    kill -9 "\$old_pid" 2>/dev/null || true
  fi
fi

if command -v fuser >/dev/null 2>&1; then
  fuser -k "\$deploy_port/tcp" >/dev/null 2>&1 || true
fi

if [[ -f "\$remote_log" ]] && [[ "\$(wc -c < "\$remote_log")" -gt 52428800 ]]; then
  mv "\$remote_log" "\$remote_log.\$(date +%Y%m%d-%H%M%S)"
fi

nohup "\$remote_current/bin/portal-api" >> "\$remote_log" 2>&1 &
echo "\$!" > "\$remote_pid"

for _ in \$(seq 1 45); do
  if curl -fsS "http://127.0.0.1:\$deploy_port/api/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -fsS "http://127.0.0.1:\$deploy_port/api/health" >/dev/null

if command -v nginx >/dev/null 2>&1; then
  cat > /etc/nginx/conf.d/portal-northline-documind.conf <<NGINX
server {
    listen 6688 default_server;
    server_name _;

    location /northline/ {
        proxy_pass http://127.0.0.1:6666/;
        proxy_set_header Host \\\$host;
        proxy_set_header X-Real-IP \\\$remote_addr;
        proxy_set_header X-Forwarded-For \\\$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \\\$scheme;
    }

    location /documind/ {
        proxy_pass http://127.0.0.1:5555/;
        proxy_set_header Host \\\$host;
        proxy_set_header X-Real-IP \\\$remote_addr;
        proxy_set_header X-Forwarded-For \\\$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \\\$scheme;
    }

    location /portal/ {
        proxy_pass http://127.0.0.1:$DEPLOY_PORT/;
        proxy_set_header Host \\\$host;
        proxy_set_header X-Real-IP \\\$remote_addr;
        proxy_set_header X-Forwarded-For \\\$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \\\$scheme;
    }

    location / {
        proxy_pass http://127.0.0.1:$DEPLOY_PORT/;
        proxy_set_header Host \\\$host;
        proxy_set_header X-Real-IP \\\$remote_addr;
        proxy_set_header X-Forwarded-For \\\$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \\\$scheme;
    }
}
NGINX
  nginx -t
  nginx -s reload || systemctl reload nginx || true
fi

echo "Portal is running on port \$deploy_port"
curl -fsS "http://127.0.0.1:\$deploy_port/api/health"
echo
REMOTE
