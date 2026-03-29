---
description: Quick-navigate to a URL and summarize the page
argument-hint: <url>
allowed-tools:
  - Bash(web-interact *)
  - Read
---

# /browse — Navigate and Read a Webpage

**First:** Load the browser-automation skill for full API reference and patterns.

## Instructions

1. If no URL is provided via `$1`, ask the user for one.

2. Ensure the URL has a protocol prefix. If `$1` does not start with `http://` or
   `https://`, prepend `https://`.

3. Navigate and snapshot:

```bash
web-interact --headless --timeout 10 <<'EOF'
const page = await browser.getPage("browse");
await page.goto("URL_HERE");
await page.waitForLoadState("networkidle");
const snap = await page.snapshotForAI();
console.log(JSON.stringify({
  url: page.url(),
  title: await page.title(),
}, null, 2));
console.log("---SNAPSHOT---");
console.log(snap.full);
EOF
```

Replace `URL_HERE` with the actual URL from `$1` (with protocol prefix added if needed).

4. If the snapshot output exceeds ~200 lines, re-run with a depth limit:

```bash
web-interact --headless --timeout 10 <<'EOF'
const page = await browser.getPage("browse");
const snap = await page.snapshotForAI({ depth: 3 });
console.log(snap.full);
EOF
```

5. Present:
   - Page title and final URL (note any redirects)
   - Concise summary of content and structure
   - Key interactive elements (forms, buttons, navigation)

6. The named page `"browse"` persists — offer to interact further
   (click links, fill forms, take screenshots, extract data).

## If web-interact is unavailable or not ready

If `WEB_INTERACT_AVAILABLE=false`, install or build the `web-interact` CLI and make
sure it resolves on `PATH` (or set `WEB_INTERACT_BIN`).

If `WEB_INTERACT_INSTALL_NEEDED=true` or the CLI says embedded dependencies are
missing, run:

```bash
web-interact install
```
