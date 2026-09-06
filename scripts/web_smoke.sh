#!/usr/bin/env bash
# Prove the WebGPU WASM client at http://127.0.0.1:8080 boots in Chrome.
# Requires trunk serve (./scripts/web.sh) and a local Chrome. Does not start a
# second native Unbound; uses its own Chrome profile, not identity.token.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
eval "$(mise activate bash)" 2>/dev/null || true
export PATH="$HOME/.local/share/mise/shims:$HOME/.cargo/bin:$PATH"

url="${UNBOUND_WEB_URL:-http://127.0.0.1:8080}"
budget_ms="${UNBOUND_WEB_SMOKE_MS:-45000}"
cdp_port="${UNBOUND_CDP_PORT:-$((9200 + RANDOM % 500))}"
profile="${TMPDIR:-/tmp}/unbound-web-smoke-profile"
chrome_bin="${CHROME_BIN:-}"
if [ -z "$chrome_bin" ]; then
  for c in google-chrome-stable google-chrome chromium chromium-browser; do
    if command -v "$c" >/dev/null 2>&1; then
      chrome_bin=$(command -v "$c")
      break
    fi
  done
fi
if [ -z "$chrome_bin" ]; then
  echo "FAIL web_smoke: no Chrome/Chromium on PATH"
  exit 1
fi
if ! command -v node >/dev/null 2>&1; then
  echo "FAIL web_smoke: node is required for CDP"
  exit 1
fi

need_http() {
  local path="$1" min="$2"
  local code size
  code=$(curl -sS -o /dev/null -w '%{http_code}' "$url$path")
  size=$(curl -sS -o /dev/null -w '%{size_download}' "$url$path")
  if [ "$code" != "200" ]; then
    echo "FAIL web_smoke $path: HTTP $code (is trunk on :8080?)"
    exit 1
  fi
  if [ "$size" -lt "$min" ]; then
    echo "FAIL web_smoke $path: ${size}B < ${min}B"
    exit 1
  fi
  echo "ok web_smoke $path: HTTP $code ${size}B"
}

echo "== http =="
need_http "/" 500
need_http "/unbound.js" 1000
need_http "/unbound_bg.wasm" 1000000
need_http "/assets/sfx/hit.wav" 100

echo
echo "== chrome WebGPU =="
rm -rf "$profile"
mkdir -p "$profile"
chrome_log="${TMPDIR:-/tmp}/unbound-web-smoke.chrome.log"
# Unique profile so this never shares ~/.local/share/unbound/identity.token.
"$chrome_bin" \
  --user-data-dir="$profile" \
  --no-first-run --no-default-browser-check \
  --disable-sync --disable-extensions \
  --remote-debugging-port="$cdp_port" \
  --remote-allow-origins=* \
  --autoplay-policy=no-user-gesture-required \
  --window-size=1280,720 \
  --enable-unsafe-webgpu \
  about:blank \
  >"$chrome_log" 2>&1 &
chrome_pid=$!
cleanup() {
  kill "$chrome_pid" 2>/dev/null || true
  sleep 0.2
  kill -9 "$chrome_pid" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 40); do
  if curl -sf "http://127.0.0.1:${cdp_port}/json/version" >/dev/null; then
    break
  fi
  if ! kill -0 "$chrome_pid" 2>/dev/null; then
    echo "FAIL web_smoke: Chrome exited"
    tail -20 "$chrome_log" || true
    exit 1
  fi
  sleep 0.15
done
if ! curl -sf "http://127.0.0.1:${cdp_port}/json/version" >/dev/null; then
  echo "FAIL web_smoke: Chrome CDP did not come up on :$cdp_port"
  tail -20 "$chrome_log" || true
  exit 1
fi

UNBOUND_WEB_URL="$url" UNBOUND_WEB_SMOKE_MS="$budget_ms" UNBOUND_CDP_PORT="$cdp_port" \
  node --input-type=module <<'JS'
const port = process.env.UNBOUND_CDP_PORT || '9229';
const target = (process.env.UNBOUND_WEB_URL || 'http://127.0.0.1:8080').replace(/\/$/, '') + '/';
const budget = Number(process.env.UNBOUND_WEB_SMOKE_MS || '45000');

const version = await fetch(`http://127.0.0.1:${port}/json/version`).then((r) => r.json());
const browser = new WebSocket(version.webSocketDebuggerUrl);
await new Promise((res, rej) => {
  browser.onopen = res;
  browser.onerror = () => rej(new Error('cdp websocket failed'));
});

let id = 0;
const pending = new Map();
function onMessage(ev) {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    const { res, rej } = pending.get(msg.id);
    pending.delete(msg.id);
    if (msg.error) rej(new Error(JSON.stringify(msg.error)));
    else res(msg.result);
  }
}
browser.addEventListener('message', onMessage);

function call(method, params, sessionId) {
  const mid = ++id;
  const payload = { id: mid, method, params };
  if (sessionId) payload.sessionId = sessionId;
  browser.send(JSON.stringify(payload));
  return new Promise((res, rej) => pending.set(mid, { res, rej }));
}

const { targetId } = await call('Target.createTarget', { url: 'about:blank' });
const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true });
const sess = (method, params) => call(method, params, sessionId);

const logs = [];
const exceptions = [];
browser.addEventListener('message', (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.sessionId !== sessionId) return;
  if (msg.method === 'Runtime.consoleAPICalled') {
    const args = (msg.params.args || [])
      .map((a) => a.value ?? a.description ?? a.type)
      .join(' ');
    logs.push({ type: msg.params.type, args });
  }
  if (msg.method === 'Runtime.exceptionThrown') {
    const t =
      msg.params.exceptionDetails?.exception?.description ||
      msg.params.exceptionDetails?.text ||
      JSON.stringify(msg.params.exceptionDetails);
    exceptions.push(t);
  }
});

await sess('Runtime.enable');
await sess('Page.enable');
await sess('Page.navigate', { url: target });

const start = Date.now();
let started = false;
let canvas = 0;
let title = '';
while (Date.now() - start < budget) {
  await new Promise((r) => setTimeout(r, 1000));
  const ev = await sess('Runtime.evaluate', {
    expression: `({
      title: document.title,
      started: !!window.wasmBindings,
      canvas: document.querySelectorAll('canvas').length
    })`,
    returnByValue: true,
  });
  const v = ev.result?.value || {};
  title = v.title || '';
  started = !!v.started;
  canvas = v.canvas || 0;
  if (started && canvas > 0) break;
}

const errors = logs.filter((l) => l.type === 'error').map((l) => l.args);
const panic = [...exceptions, ...errors].find((t) =>
  /panic|WebGPU|adapter|failed to|fatal/i.test(String(t)),
);

await call('Target.closeTarget', { targetId }).catch(() => {});
browser.close();

if (!started || canvas < 1) {
  console.error(
    `FAIL web_smoke: wasm window did not boot (started=${started} canvas=${canvas} title=${title})`,
  );
  if (exceptions[0]) console.error(exceptions[0]);
  if (errors[0]) console.error(errors[0]);
  process.exit(1);
}
if (panic) {
  console.error(`FAIL web_smoke: ${panic}`);
  process.exit(1);
}
console.log(`ok web_smoke chrome: canvas ${canvas} wasmBindings title=${title}`);
process.exit(0);
JS

echo
echo "web_smoke: chrome WebGPU client booted"
