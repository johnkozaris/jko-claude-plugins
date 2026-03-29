# Workflow Patterns

Concrete script examples for common browser automation scenarios. Each pattern follows
the one-action-per-script rule: small focused scripts that log state for the next decision.

---

## Login Flows

### Pattern 1: Known Form Structure

When selectors are already known, interact directly:

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("login");
await page.goto("https://app.example.com/login");
await page.fill('input[name="email"]', 'user@example.com');
await page.fill('input[name="password"]', 'secretpass');
await page.click('button[type="submit"]');
await page.waitForURL("**/dashboard");
console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
EOF
```

### Pattern 2: Discover-Then-Interact (Unknown Form)

Step 1 — snapshot the login page:
```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("login");
await page.goto("https://app.example.com/login");
const snap = await page.snapshotForAI();
console.log(snap.full);
EOF
```

Step 2 — fill based on discovered ARIA roles (page persists):
```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("login");
await page.getByRole("textbox", { name: "Email" }).fill("user@example.com");
await page.getByLabel("Password").fill("secretpass");
await page.getByRole("button", { name: "Sign in" }).click();
await page.waitForURL("**/dashboard");
console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
EOF
```

### Pattern 3: Use Existing Chrome Session (Already Logged In)

When the user is already logged in via their browser:

Step 1 — list tabs to find the target:
```bash
web-interact --connect <<'EOF'
const tabs = await browser.listPages();
console.log(JSON.stringify(tabs, null, 2));
EOF
```

Step 2 — connect to the specific tab by targetId:
```bash
web-interact --connect <<'EOF'
const page = await browser.getPage("TARGET_ID_FROM_STEP1");
console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
EOF
```

### Pattern 4: OAuth / Multi-Step Login

For multi-step auth flows, handle each step separately:

```bash
# Step 1: Enter email
web-interact --headless <<'EOF'
const page = await browser.getPage("auth");
await page.goto("https://accounts.google.com");
await page.fill('input[type="email"]', 'user@gmail.com');
await page.click('#identifierNext');
await page.waitForSelector('input[type="password"]', { timeout: 10000 });
console.log("Email submitted, password field visible");
EOF

# Step 2: Enter password
web-interact --headless <<'EOF'
const page = await browser.getPage("auth");
await page.fill('input[type="password"]', 'password123');
await page.click('#passwordNext');
await page.waitForURL("**/myaccount**", { timeout: 15000 });
console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
EOF
```

Caveat: Google and similar providers actively detect headless automation and may block
login attempts. For real OAuth flows, prefer `--connect` mode to use the user's
authenticated browser session.

---

## Form Filling

### Text Inputs, Dropdowns, Checkboxes, Radios

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("form");
await page.goto("https://app.example.com/settings");

// Text inputs
await page.fill('input[name="firstName"]', 'John');
await page.fill('input[name="lastName"]', 'Doe');

// Dropdown / select
await page.selectOption('select[name="country"]', 'US');

// Checkboxes
await page.check('input[name="terms"]');
await page.uncheck('input[name="newsletter"]');

// Radio buttons
await page.check('input[value="premium"]');

// Date picker
await page.fill('input[type="date"]', '2026-03-26');

// Textarea
await page.fill('textarea[name="bio"]', 'Hello world');

// Submit
await page.click('button[type="submit"]');
await page.waitForSelector('.success-message');
console.log("Form submitted");
EOF
```

### Using getByRole (More Robust for Unknown Forms)

After using `snapshotForAI()` to discover the form structure:

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("form");
await page.getByRole("textbox", { name: "First Name" }).fill("John");
await page.getByRole("textbox", { name: "Last Name" }).fill("Doe");
await page.getByRole("combobox", { name: "Country" }).selectOption("US");
await page.getByRole("checkbox", { name: "I agree to the terms" }).check();
await page.getByRole("radio", { name: "Premium" }).check();
await page.getByRole("button", { name: "Submit" }).click();
console.log("Form submitted via ARIA roles");
EOF
```

### File Upload

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("upload");
await page.goto("https://app.example.com/upload");
const fileInput = page.locator('input[type="file"]');
await fileInput.setInputFiles("/path/to/file.pdf");
await page.click('button:text("Upload")');
await page.waitForSelector('.upload-complete');
console.log("File uploaded");
EOF
```

---

## Tab Management

### Open Multiple Named Pages

```bash
web-interact --headless <<'EOF'
const search = await browser.getPage("search");
await search.goto("https://google.com");

const docs = await browser.getPage("docs");
await docs.goto("https://playwright.dev");

const tabs = await browser.listPages();
console.log(JSON.stringify(tabs, null, 2));
EOF
```

### Switch Between Existing Tabs (They Persist)

```bash
web-interact --headless <<'EOF'
const search = await browser.getPage("search");
console.log("Search:", search.url());

const docs = await browser.getPage("docs");
console.log("Docs:", docs.url());
EOF
```

### Close a Tab

```bash
web-interact --headless <<'EOF'
await browser.closePage("search");
const remaining = await browser.listPages();
console.log(JSON.stringify(remaining, null, 2));
EOF
```

### Connect to Existing Chrome Tab by targetId

```bash
web-interact --connect <<'EOF'
const tabs = await browser.listPages();
const gmailTab = tabs.find(t => t.url.includes("mail.google.com"));
if (gmailTab) {
  const page = await browser.getPage(gmailTab.id);
  console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
} else {
  console.log("Gmail tab not found");
  console.log(JSON.stringify(tabs.map(t => ({ title: t.title, url: t.url }))));
}
EOF
```

---

## Data Extraction

### Extract Table Data

```bash
web-interact --headless --timeout 60 <<'EOF'
const page = await browser.getPage("data");
await page.goto("https://example.com/products");
await page.waitForSelector("table");

const rows = await page.$$eval("table tbody tr", trs =>
  trs.map(tr => {
    const cells = tr.querySelectorAll("td");
    return {
      name: cells[0]?.textContent?.trim(),
      price: cells[1]?.textContent?.trim(),
      stock: cells[2]?.textContent?.trim(),
    };
  })
);

console.log(JSON.stringify(rows, null, 2));
EOF
```

### Extract List Items

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("articles");
await page.goto("https://example.com/blog");

const items = await page.$$eval("article.post", posts =>
  posts.map(p => ({
    title: p.querySelector("h2")?.textContent?.trim(),
    summary: p.querySelector(".summary")?.textContent?.trim(),
    link: p.querySelector("a")?.href,
  }))
);

console.log(JSON.stringify(items, null, 2));
EOF
```

### Extract All Links

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("links");
await page.goto("https://example.com");

const links = await page.$$eval("a[href]", anchors =>
  anchors.map(a => ({ text: a.textContent?.trim(), href: a.href }))
    .filter(l => l.href.startsWith("http"))
);

console.log(JSON.stringify(links, null, 2));
EOF
```

---

## Pagination

### Click-Based Pagination

```bash
web-interact --headless --timeout 120 <<'EOF'
const page = await browser.getPage("paginated");
await page.goto("https://example.com/results");
const allResults = [];

for (let i = 0; i < 5; i++) {
  await page.waitForSelector(".result-item");
  const pageResults = await page.$$eval(".result-item", items =>
    items.map(el => ({
      title: el.querySelector("h3")?.textContent?.trim(),
      url: el.querySelector("a")?.href,
    }))
  );
  allResults.push(...pageResults);
  console.log(`Page ${i + 1}: ${pageResults.length} items`);

  const nextBtn = page.locator('a:text("Next")');
  if (await nextBtn.count() === 0) break;
  await nextBtn.click();
  await page.waitForLoadState("networkidle");
}

const path = await writeFile("all-results.json", JSON.stringify(allResults, null, 2));
console.log(JSON.stringify({ total: allResults.length, file: path }));
EOF
```

### URL-Based Pagination

```bash
web-interact --headless --timeout 120 <<'EOF'
const page = await browser.getPage("paginated");
const allData = [];

for (let p = 1; p <= 10; p++) {
  await page.goto(`https://example.com/api/items?page=${p}`);
  const text = await page.textContent("body");
  const json = JSON.parse(text);
  if (json.items.length === 0) break;
  allData.push(...json.items);
  console.log(`Page ${p}: ${json.items.length} items`);
}

const path = await writeFile("paginated-data.json", JSON.stringify(allData, null, 2));
console.log(JSON.stringify({ total: allData.length, file: path }));
EOF
```

### Infinite Scroll

```bash
web-interact --headless --timeout 60 <<'EOF'
const page = await browser.getPage("scroll");
await page.goto("https://example.com/feed");

let prevHeight = 0;
for (let i = 0; i < 10; i++) {
  const height = await page.evaluate(() => {
    window.scrollTo(0, document.body.scrollHeight);
    return document.body.scrollHeight;
  });
  if (height === prevHeight) break;
  prevHeight = height;
  await page.waitForTimeout(1500);
  console.log(`Scroll ${i + 1}: height=${height}`);
}

const items = await page.$$eval(".feed-item", els =>
  els.map(el => el.textContent?.trim())
);
console.log(JSON.stringify({ count: items.length }));
EOF
```

---

## Navigation Patterns

### Click Links and Buttons

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("nav");
await page.goto("https://example.com");

// Click a link
await page.click('a[href="/about"]');
await page.waitForURL("**/about");
console.log("Navigated to:", page.url());
EOF
```

### Back / Forward

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("nav");
await page.goBack();
console.log("Back to:", page.url());
await page.goForward();
console.log("Forward to:", page.url());
EOF
```

### Wait for SPA Navigation

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("spa");
await page.goto("https://spa-app.com");
await page.click('a[href="/dashboard"]');
// SPA: URL changes but no full page load
await page.waitForURL("**/dashboard");
await page.waitForSelector(".dashboard-loaded");
console.log("SPA navigated:", page.url());
EOF
```

---

## Screenshots

### Basic Screenshot

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com");
const path = await saveScreenshot(await page.screenshot(), "homepage.png");
console.log(path);
EOF
```

### Full-Page Screenshot

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
const path = await saveScreenshot(
  await page.screenshot({ fullPage: true }),
  "full-page.png"
);
console.log(path);
EOF
```

### Element Screenshot

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
const path = await saveScreenshot(
  await page.locator("header").screenshot(),
  "header.png"
);
console.log(path);
EOF
```

---

## Connect Mode (Authenticated Sessions)

### Launch Chrome with Remote Debugging

On macOS:
```bash
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222
```

### Auto-Discover Running Chrome

```bash
web-interact --connect <<'EOF'
const tabs = await browser.listPages();
console.log(JSON.stringify(tabs, null, 2));
EOF
```

The daemon probes ports 9222-9229 and reads `DevToolsActivePort` files from Chrome
profile directories.

### Use an Authenticated Tab

```bash
web-interact --connect <<'EOF'
const tabs = await browser.listPages();
const target = tabs.find(t => t.url.includes("github.com"));
if (target) {
  const page = await browser.getPage(target.id);
  const snap = await page.snapshotForAI();
  console.log(snap.full);
}
EOF
```

---

## Keyboard and Mouse

### Type Text Character by Character

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
await page.click("#search-input");
await page.keyboard.type("hello world", { delay: 50 });
await page.keyboard.press("Enter");
EOF
```

### Key Combinations

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
// Ctrl+A (select all)
await page.keyboard.down("Control");
await page.keyboard.press("a");
await page.keyboard.up("Control");
// Then type to replace
await page.keyboard.type("replacement text");
EOF
```

### Scroll via Mouse Wheel

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
await page.mouse.wheel(0, 500); // scroll down 500px
EOF
```

---

## Working with Iframes

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com/with-iframe");

// Get iframe by name or URL
const frame = page.frame("iframe-name");
if (frame) {
  const text = await frame.textContent("h1");
  console.log("Inside iframe:", text);
  await frame.click("button.action");
}
EOF
```

Or use locator-based frame access:

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
const frameLocator = page.frameLocator("#my-iframe");
await frameLocator.locator("button.submit").click();
EOF
```

---

## Dialog Handling

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");

// Set up dialog handler BEFORE triggering the dialog
page.on("dialog", async dialog => {
  console.log("Dialog:", dialog.type(), dialog.message());
  await dialog.accept(); // or dialog.dismiss()
});

await page.click("button.delete");
console.log("Dialog handled");
EOF
```

---

## Cookie Consent / Banner Dismissal

Most websites show cookie consent banners that block interaction. Dismiss them first:

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com");

// Try common cookie banner selectors (sites vary widely)
try {
  const bannerBtn = page.locator(
    '[class*="cookie"] button, [id*="consent"] button, ' +
    'button:text("Accept"), button:text("Accept All"), ' +
    'button:text("Got it"), button:text("OK"), ' +
    '[aria-label*="cookie"] button, [aria-label*="consent"] button'
  );
  if (await bannerBtn.count() > 0) {
    await bannerBtn.first().click({ timeout: 3000 });
    console.log("Cookie banner dismissed");
  }
} catch (e) {
  // No banner or already dismissed
}

const snap = await page.snapshotForAI();
console.log(snap.full);
EOF
```

If the banner uses an iframe (common with CMP providers like OneTrust, Cookiebot):

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
try {
  const frame = page.frameLocator('[id*="consent"], [title*="cookie"]');
  await frame.locator('button:text("Accept")').click({ timeout: 3000 });
} catch (e) { /* no consent iframe */ }
EOF
```

---

## Popup / New Window Handling

When clicking a link opens a new tab (via `target="_blank"` or `window.open()`):

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("main");

// Wait for popup while clicking
const [popup] = await Promise.all([
  page.waitForEvent("popup"),
  page.click('a[target="_blank"]'),
]);

// popup is a full Page object
await popup.waitForLoadState("networkidle");
console.log(JSON.stringify({
  parentUrl: page.url(),
  popupUrl: popup.url(),
  popupTitle: await popup.title(),
}));

// Interact with the popup
const snap = await popup.snapshotForAI();
console.log(snap.full);
EOF
```

---

## Responsive Testing / Viewport Control

```bash
# Test at mobile viewport
web-interact --headless <<'EOF'
const page = await browser.getPage("responsive");
await page.setViewportSize({ width: 375, height: 812 });
await page.goto("https://example.com");
await saveScreenshot(await page.screenshot(), "mobile.png");
console.log("Mobile screenshot taken");
EOF

# Test at desktop viewport
web-interact --headless <<'EOF'
const page = await browser.getPage("responsive");
await page.setViewportSize({ width: 1920, height: 1080 });
await page.goto("https://example.com");
await saveScreenshot(await page.screenshot(), "desktop.png");
console.log("Desktop screenshot taken");
EOF
```

---

## Capturing Browser Console Errors

Check a page for JavaScript errors:

```bash
web-interact --headless --timeout 15 <<'EOF'
const page = await browser.getPage("debug");
const errors = [];
const warnings = [];

page.on("console", msg => {
  if (msg.type() === "error") errors.push(msg.text());
  if (msg.type() === "warning") warnings.push(msg.text());
});

page.on("pageerror", err => {
  errors.push("Uncaught: " + err.message);
});

await page.goto("https://myapp.com");
await page.waitForLoadState("networkidle");

console.log(JSON.stringify({ errors, warnings }, null, 2));
EOF
```

---

## Waiting for API Responses

Wait for a specific network request to complete after an action:

```bash
web-interact --headless <<'EOF'
const page = await browser.getPage("app");

// Click button and wait for the API call to return
const [response] = await Promise.all([
  page.waitForResponse(resp => resp.url().includes("/api/data") && resp.status() === 200),
  page.click('button:text("Load Data")'),
]);

console.log("API responded:", response.status());
const snap = await page.snapshotForAI();
console.log(snap.full);
EOF
```

---

## Bot Detection, Cloudflare, and CAPTCHAs

**Always prefer `--connect` mode over `--headless` for external websites.** Connect mode
uses the user's real Chrome browser, which has a genuine fingerprint, real browsing history,
cookies, and extensions. This bypasses most bot detection (Cloudflare, Akamai, etc.)
automatically — web-interact still navigates and controls the page, but through a real
browser that does not look like automation.

Headless managed Chromium has a recognizable fingerprint that many sites detect and
block. Reserve `--headless` for local development servers, known-safe internal sites,
CI-style scripted jobs, and sites that do not use bot detection.

### Connect mode handles most cases automatically

```bash
# Connect to user's Chrome and navigate — no manual intervention needed
web-interact --connect <<'EOF'
const page = await browser.getPage("site");
await page.goto("https://protected-site.com");
// Real Chrome fingerprint → Cloudflare passes automatically
await page.waitForLoadState("networkidle");
const snap = await page.snapshotForAI();
console.log(snap.full);
EOF
```

### CAPTCHAs require user intervention

If a CAPTCHA appears (reCAPTCHA, hCaptcha, etc.), it cannot be solved automatically.
In this case:

1. Tell the user a CAPTCHA appeared
2. The user solves it in their visible Chrome window
3. After solving, continue automation on the same named page:

```bash
web-interact --connect <<'EOF'
const page = await browser.getPage("site");
// User has solved the CAPTCHA — page is now past it
const snap = await page.snapshotForAI();
console.log(snap.full);
EOF
```

### When headless is fine

- `localhost` and local dev servers
- Internal/intranet sites
- Sites with no bot detection
- Generating screenshots for testing
- Any site where the user explicitly asks for headless mode
