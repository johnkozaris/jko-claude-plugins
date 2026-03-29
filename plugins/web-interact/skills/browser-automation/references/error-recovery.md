# Error Recovery Guide

Systematic catalog of common failures when using web-interact, with recovery strategies.

## General Recovery Principle

When a script fails, the named page usually stays where it stopped. Reconnect to the same
page name, take a screenshot, and log the URL/title to understand what happened:

```bash
web-interact --headless <<'EOF'
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

## Primary Defensive Patterns

These are not optional niceties — they prevent the most common class of failures.

### Action Timeout vs Script Timeout (Critical)

Playwright actions (click, fill, type, getByRole().click()) use a **retry loop**: they
repeatedly poll the DOM for the element and check actionability (visible, stable, enabled,
not obscured). If the selector is wrong or the element doesn't exist, the action hangs
for its full timeout before failing. The default action timeout can exceed the script
timeout, so a single bad selector can kill the entire script with a generic "Script timed
out" error — with no indication of which action failed or why.

Always use per-action timeouts for uncertain interactions:

```javascript
// Good: fails in 2s with specific error naming the selector
await page.click('.uncertain', { timeout: 2000 });

// Bad: hangs in retry loop until script timeout kills everything
await page.click('.uncertain');
```

### Check Before Click

`locator.count()` returns immediately with the current count — no retry loop, no waiting.
Use it before any interaction with an element that might not exist:

```javascript
const button = page.locator("button.submit");
if (await button.count() > 0) {
  await button.click({ timeout: 3000 });
  console.log("Clicked");
} else {
  console.log("Button not found — exploring DOM");
  const info = await page.evaluate(() => ({
    buttons: Array.from(document.querySelectorAll('button')).map(b => b.textContent?.trim()),
  }));
  console.log(JSON.stringify(info));
}
```

### Safe Popup/Banner Dismissal

```javascript
// WRONG: .catch() waits full action timeout (retry loop) before catching
await page.getByRole('button', { name: 'Got it' }).click().catch(() => {});

// RIGHT: count() returns immediately, no retry loop
const btn = page.getByRole('button', { name: 'Got it' });
if (await btn.count() > 0) await btn.click({ timeout: 2000 });
```

### Actionability Bypass

When an element exists in the DOM but click() hangs because Playwright's actionability
checks fail (obscured by overlay, needs activation, not stable):

```javascript
// Option 1: native DOM click via evaluate (bypasses Playwright entirely)
await page.evaluate(() => document.querySelector('.target')?.click());

// Option 2: force click (skips Playwright's actionability checks)
await page.click('.target', { force: true, timeout: 2000 });
```

### Try-Catch with Debug Screenshot

```javascript
try {
  await page.click("button.submit", { timeout: 3000 });
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
   web-interact --headless <<'EOF'
   const page = await browser.getPage("SAME_NAME");
   const snap = await page.snapshotForAI();
   console.log(snap.full);
   EOF
   ```
2. Use the snapshot to find the correct selector or ARIA role.
3. If the element is in an iframe, use `page.frameLocator()` or `page.frame()`.

**Prevention:** Use `locator.count()` before interacting — it returns immediately and
tells you if the element exists without entering a retry loop:
```javascript
const el = page.locator('button.submit');
if (await el.count() === 0) {
  console.log('Element not found — exploring DOM');
  const info = await page.evaluate(() => ({
    buttons: Array.from(document.querySelectorAll('button')).map(b => b.textContent?.trim()),
  }));
  console.log(JSON.stringify(info));
} else {
  await el.click({ timeout: 3000 });
}
```
Also use per-action `{ timeout: 2000 }` on uncertain selectors so failures are fast and
specific instead of killing the whole script.

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

**Error:** `Script timed out after 10s and was terminated.`

**Causes:**
- A single Playwright action (click, fill, waitForSelector) entered its retry loop on a
  wrong selector and hung until the script wall-clock timeout killed everything
- The script does too much (violates one-action-per-script rule)
- A wait operation is hanging on a condition that never resolves
- Infinite loop in the script

**The hidden trap:** Playwright actions use a retry loop that polls the DOM repeatedly.
When a selector is wrong, the action hangs retrying for its full action timeout. If the
action timeout exceeds the script timeout, the script dies with a generic "Script timed
out" — you never see which action failed or why. You lose the full timeout duration and
gain zero diagnostic information.

**Recovery:**
1. **Add per-action timeouts** to every speculative action:
   ```javascript
   await page.click('.might-exist', { timeout: 2000 });
   ```
   This way, if the selector is wrong, you get `page.click: Timeout 2000ms exceeded`
   in 2s — a specific error naming the selector — instead of "Script timed out" in 10s.
2. **Use count() before clicking** elements that may not exist:
   ```javascript
   const btn = page.locator('.might-exist');
   if (await btn.count() > 0) await btn.click({ timeout: 2000 });
   ```
3. Break the script into smaller steps (fix the root cause).
4. Only increase the script timeout if the operation is genuinely slow (large data extraction).

### F-06: Daemon Not Running

**Error:** `Connection refused` or `Failed to connect to daemon`

**Causes:**
- web-interact daemon crashed or was stopped
- Socket file is stale

**Recovery:**
1. Stop and restart:
   ```bash
   web-interact stop
   web-interact status
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
   web-interact --connect http://localhost:9222 <<'EOF'
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
web-interact --headless <<'EOF'
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
Pass `--ignore-https-errors` when launching a daemon-managed browser against a
self-signed localhost or staging certificate. In `--connect` mode, configure the
external Chrome session yourself (for example by launching it with
`--ignore-certificate-errors` if that is acceptable for the test).

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
2. web-interact still navigates and controls the page — the user does not need to
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

