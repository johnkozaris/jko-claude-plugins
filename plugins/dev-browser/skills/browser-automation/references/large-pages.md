# Handling Large Pages

Strategies for working with pages too large for the context window.

## The Problem

`snapshotForAI()` returns the full ARIA accessibility tree. Complex pages (dashboards,
large tables, SPAs) can produce thousands of lines — consuming context and degrading
reasoning quality.

## Strategy Selection

| Scenario | Strategy | Context Cost |
|----------|----------|-------------|
| Simple page, few elements | `snapshotForAI()` directly | Low |
| Complex page, need structure | `snapshotForAI({ depth: 3 })` | Medium |
| Need specific section only | Locator-scoped snapshot | Low-Medium |
| Large table, need all data | `$$eval` + `writeFile()` + Read/Grep | Zero |
| Need raw text only | `textContent("main")` | Varies |
| Visual layout matters | `screenshot()` + Read (image) | Fixed |

---

## Depth-Limited Snapshots

Limit tree traversal to reduce output:

```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://complex-dashboard.com");
const snap = await page.snapshotForAI({ depth: 3 });
console.log(snap.full);
EOF
```

`depth: 2` gives top-level layout. `depth: 3-4` balances detail and size.

---

## Section-Scoped Snapshots

Snapshot only the relevant part of the page:

```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com");
const mainSnap = await page.locator("main").snapshotForAI();
console.log(mainSnap.full);
EOF
```

Good candidates: `main`, `article`, `[role="main"]`, `table.results`, `nav`, `aside`.

---

## Extract to File, Then Read/Grep

The most important pattern for large data. Save to a temp file, then use Claude's
native `Read` and `Grep` tools on the file:

```bash
dev-browser --headless --timeout 60 <<'EOF'
const page = await browser.getPage("data");
await page.goto("https://example.com/reports");
await page.waitForSelector("table");

const rows = await page.$$eval("table tbody tr", trs =>
  trs.map(tr => {
    const cells = tr.querySelectorAll("td");
    return {
      date: cells[0]?.textContent?.trim(),
      name: cells[1]?.textContent?.trim(),
      amount: cells[2]?.textContent?.trim(),
      status: cells[3]?.textContent?.trim(),
    };
  })
);

const path = await writeFile("report.json", JSON.stringify(rows, null, 2));
console.log(JSON.stringify({ file: path, rows: rows.length }));
EOF
```

After this script completes:
- Use `Read` on the returned file path to view the data
- Use `Grep` to search for patterns (e.g., find all rows with status "failed")

---

## Filtered Extraction

Extract only specific data, not everything:

```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com/dashboard");

// Only extract the summary cards, not the full dashboard
const metrics = await page.$$eval(".metric-card", cards =>
  cards.map(c => ({
    label: c.querySelector(".label")?.textContent?.trim(),
    value: c.querySelector(".value")?.textContent?.trim(),
  }))
);

console.log(JSON.stringify(metrics, null, 2));
EOF
```

---

## Incremental Snapshots with Tracking

After interacting with a page, get only what changed:

```bash
# First: full snapshot
dev-browser --headless <<'EOF'
const page = await browser.getPage("dash");
await page.goto("https://example.com/dashboard");
const snap = await page.snapshotForAI({ track: "dash" });
console.log(snap.full);
EOF

# After interaction: incremental diff only
dev-browser --headless <<'EOF'
const page = await browser.getPage("dash");
await page.click('button:text("Refresh")');
await page.waitForSelector(".updated");
const snap = await page.snapshotForAI({ track: "dash" });
console.log(snap.incremental || "No changes");
EOF
```

---

## Text-Only Extraction

When structure does not matter, just get the text:

```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com/article");
const text = await page.textContent("article");
console.log(text);
EOF
```

For very long text, save to file:

```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("main");
const text = await page.textContent("body");
const path = await writeFile("page-text.txt", text);
console.log(path);
EOF
```

---

## Paginated Extraction to File

Collect data across multiple pages:

```bash
dev-browser --headless --timeout 120 <<'EOF'
const page = await browser.getPage("paginated");
const allData = [];

for (let p = 1; p <= 20; p++) {
  await page.goto("https://example.com/items?page=" + p);
  await page.waitForSelector(".item");
  const items = await page.$$eval(".item", els =>
    els.map(el => ({
      title: el.querySelector("h3")?.textContent?.trim(),
      price: el.querySelector(".price")?.textContent?.trim(),
    }))
  );
  if (items.length === 0) break;
  allData.push(...items);
  console.log("Page " + p + ": " + items.length + " items");
}

const path = await writeFile("all-items.json", JSON.stringify(allData, null, 2));
console.log(JSON.stringify({ total: allData.length, file: path }));
EOF
```

---

## Save HTML for Grep

Save raw HTML and search it offline:

```bash
dev-browser --headless --timeout 60 <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com/catalog");
const html = await page.innerHTML("body");
const path = await writeFile("catalog.html", html);
console.log(path);
EOF
```

Then use `Grep` on the saved HTML file for specific patterns.

---

## Decision Guide

| Need | Method | Context Impact |
|------|--------|----------------|
| Discover structure for interaction | `snapshotForAI()` | Uses context |
| Extract specific structured data | `$$eval` + `writeFile` | Zero |
| Get readable text | `textContent(selector)` | Uses context |
| Compare states after action | `snapshotForAI({ track })` | Minimal |
| Visual layout / styling | `screenshot()` | Fixed |
| Debug page state | `screenshot()` + `snapshotForAI()` | Both |

**Rule of thumb:** To *interact* → `snapshotForAI()`. To *extract data* → `$$eval` +
`writeFile()`. To *read content* → `textContent()`.
