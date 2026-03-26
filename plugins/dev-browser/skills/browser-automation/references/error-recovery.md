# Error Recovery Guide

Systematic catalog of common failures when using dev-browser, with recovery strategies.

## General Recovery Principle

When a script fails, the named page usually stays where it stopped. Reconnect to the same
page name, take a screenshot, and log the URL/title to understand what happened:

```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("SAME_NAME");
const path = await saveScreenshot(await page.screenshot(), "debug.png");
console.log(JSON.stringify({
  screenshot: path,
  url: page.url(),
  title: await page.title(),
}));
EOF
```

Then use the `Read` tool on the screenshot path to visually inspect the page state.

---

## Failure Catalog

### F-01: Element Not Found (Timeout)

**Error:** `waiting for selector "button.submit" failed: timeout 30000ms exceeded`

**Causes:**
- Selector does not match any element on the page
- Element has not loaded yet (SPA, lazy loading)
- Element is inside an iframe
- Page navigated away before the element appeared

**Recovery:**
1. Take a snapshot to see what is actually on the page:
   ```bash
   dev-browser --headless <<'EOF'
   const page = await browser.getPage("SAME_NAME");
   const snap = await page.snapshotForAI();
   console.log(snap.full);
   EOF
   ```
2. Use the snapshot to find the correct selector or ARIA role.
3. If the element is in an iframe, use `page.frameLocator()` or `page.frame()`.

**Prevention:** Use `page.waitForSelector()` before interacting. Use short timeouts
(`--timeout 10`) for fast failure instead of waiting the full 30 seconds.

### F-02: Navigation Timeout

**Error:** `page.goto: timeout 30000ms exceeded`

**Causes:**
- The URL is unreachable or very slow
- DNS resolution failure
- SSL certificate error
- The page loads indefinitely (streaming response, SSE)

**Recovery:**
1. Check URL correctness.
2. Try with `networkidle` wait instead:
   ```javascript
   await page.goto(url, { waitUntil: "domcontentloaded", timeout: 60000 });
   ```
3. If the page streams data, use `domcontentloaded` instead of the default `load`.

### F-03: Click Intercepted / Element Not Clickable

**Error:** `element is not visible` or `element is intercepted by another element`

**Causes:**
- A modal, overlay, or cookie banner is covering the element
- The element is scrolled out of view
- The element is hidden via CSS

**Recovery:**
1. Snapshot to see overlays:
   ```javascript
   const snap = await page.snapshotForAI();
   console.log(snap.full);
   ```
2. Dismiss the overlay first (close modal, accept cookies).
3. Use `{ force: true }` to click even when covered:
   ```javascript
   await page.click("button.submit", { force: true });
   ```
4. Scroll the element into view:
   ```javascript
   await page.locator("button.submit").scrollIntoViewIfNeeded();
   await page.click("button.submit");
   ```

### F-04: Stale Element Reference

**Error:** `element is not attached to the DOM`

**Causes:**
- The page re-rendered (SPA state change, React re-render)
- The element was removed and re-added

**Recovery:**
1. Re-query the element using a locator (locators auto-retry):
   ```javascript
   await page.locator("button.submit").click(); // auto-retries
   ```
2. Avoid storing ElementHandle references across awaits. Prefer locators.

### F-05: Script Timeout

**Error:** `Script execution timed out after 30 seconds`

**Causes:**
- The script does too much (violates one-action-per-script rule)
- A wait operation is hanging on a condition that never resolves
- Infinite loop in the script

**Recovery:**
1. Break the script into smaller steps (preferred — fix the root cause).
2. Only increase timeout if the operation is genuinely slow (large data extraction).
3. Add short timeouts to individual operations:
   ```javascript
   await page.waitForSelector(".result", { timeout: 5000 });
   ```

### F-06: Daemon Not Running

**Error:** `Connection refused` or `Failed to connect to daemon`

**Causes:**
- dev-browser daemon crashed or was stopped
- Socket file is stale

**Recovery:**
1. Stop and restart:
   ```bash
   dev-browser stop
   dev-browser status
   ```
2. The daemon auto-starts on the next command, so just re-run the script.

### F-07: Connect Mode — Chrome Not Found

**Error:** `Could not find a Chrome instance with remote debugging enabled`

**Causes:**
- Chrome is not running with `--remote-debugging-port`
- Chrome is using a different port than 9222-9229
- Chrome crashed or closed

**Recovery:**
1. Launch Chrome with debugging:
   ```bash
   /Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222
   ```
2. Or specify the exact endpoint:
   ```bash
   dev-browser --connect http://localhost:9222 <<'EOF'
   // ...
   EOF
   ```

### F-08: Page Has Dialog (alert/confirm/prompt)

**Error:** Script hangs because a dialog is blocking interaction.

**Causes:**
- JavaScript `alert()`, `confirm()`, or `prompt()` is open
- The page shows a browser-native dialog

**Recovery:**
Set up a dialog handler before the triggering action:
```javascript
page.on("dialog", async dialog => {
  console.log("Dialog:", dialog.type(), dialog.message());
  await dialog.accept();
});
```

Or if already stuck, reconnect and handle:
```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("SAME_NAME");
page.on("dialog", async d => await d.accept());
// Now take the snapshot or continue
const snap = await page.snapshotForAI();
console.log(snap.full);
EOF
```

### F-09: SSL / Certificate Error

**Error:** `net::ERR_CERT_AUTHORITY_INVALID` or similar

**Causes:**
- Self-signed certificate on localhost/dev server
- Expired certificate

**Recovery:**
The daemon-managed browser uses `ignoreHTTPSErrors: true` by default for headless mode.
For connect mode, launch Chrome with `--ignore-certificate-errors` flag.

### F-10: Empty Snapshot

**Error:** `snapshotForAI()` returns empty or minimal content.

**Causes:**
- Page is still loading (JS hasn't rendered yet)
- Content is loaded via JavaScript that hasn't executed yet
- Page uses Shadow DOM extensively

**Recovery:**
1. Wait for the content to load:
   ```javascript
   await page.waitForSelector(".main-content");
   const snap = await page.snapshotForAI();
   ```
2. Wait for network idle:
   ```javascript
   await page.waitForLoadState("networkidle");
   const snap = await page.snapshotForAI();
   ```
3. Use `page.textContent("body")` as fallback for simple text extraction.

### F-11: Memory Limit Exceeded

**Error:** Script crashes with memory error.

**Causes:**
- Extracting huge amounts of data in a single `$$eval`
- Processing very large pages

**Recovery:**
1. Extract data in chunks (paginate rows, limit results).
2. Use `writeFile()` to save partial results between operations.
3. Limit snapshot depth: `page.snapshotForAI({ depth: 3 })`.

### F-12: Cloudflare / Bot Detection Page

**Symptom:** Page shows "Checking your browser..." or a challenge page instead of content.

**Causes:**
- Cloudflare, Akamai, or similar bot detection service
- Headless Chromium detected by fingerprinting

**Recovery:**
1. Switch to `--connect` mode. This uses the user's real Chrome, which has a genuine
   fingerprint and passes most bot detection automatically.
2. dev-browser still navigates and controls the page — the user does not need to
   manually browse. Just switch the flag from `--headless` to `--connect`.
3. If a CAPTCHA appears, ask the user to solve it in their Chrome window, then continue.

**Prevention:** Prefer `--connect` over `--headless` for all external websites. Reserve
headless for localhost, internal sites, and known-safe URLs.

### F-13: Rate Limiting (HTTP 429)

**Symptom:** Pages return "Too Many Requests" or empty/error content after several requests.

**Recovery:**
1. Add delays between requests using `page.waitForTimeout(2000)`.
2. Reduce the number of pages fetched per session.
3. If scraping, respect the site's `robots.txt` and terms of service.

### F-14: Cookie Consent Banner Blocking Interaction

**Symptom:** Clicks land on the cookie banner overlay instead of the target element.

**Recovery:**
Dismiss the banner first — see the Cookie Consent section in `workflow-patterns.md`.

---

## Defensive Patterns

### Check Before Click

```javascript
const button = page.locator("button.submit");
const count = await button.count();
if (count > 0) {
  await button.click();
  console.log("Clicked");
} else {
  console.log("Button not found — snapshotting for diagnosis");
  const snap = await page.snapshotForAI();
  console.log(snap.full);
}
```

### Try-Catch with Debug Screenshot

```javascript
try {
  await page.click("button.submit", { timeout: 5000 });
  console.log("Success");
} catch (e) {
  const path = await saveScreenshot(await page.screenshot(), "error.png");
  console.error(JSON.stringify({
    error: e.message,
    screenshot: path,
    url: page.url(),
  }));
}
```

### Wait for Stable State

```javascript
// Wait until element count stops changing (content fully loaded)
await page.waitForFunction(() => {
  return document.querySelectorAll(".item").length >= 10;
});
```
