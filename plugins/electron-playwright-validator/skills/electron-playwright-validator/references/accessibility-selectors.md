# Accessibility Selectors Reference

## Reading the Accessibility Tree

`e-cli snapshot` outputs the Chromium accessibility tree as indented text. Each line represents an accessible node with this format:

```
<indent><role> "<name>" [attribute=value ...]
```

### Roles

Common roles you'll encounter in Electron apps:

| Role | HTML source | Examples |
|------|-------------|----------|
| `WebArea` | Root `<html>` | Always the tree root |
| `navigation` | `<nav>`, `role="navigation"` | Sidebar, tab bars |
| `main` | `<main>`, `role="main"` | Primary content area |
| `tab` | `role="tab"` | Tab bar items |
| `tabpanel` | `role="tabpanel"` | Content panel for a tab |
| `button` | `<button>`, `role="button"` | Clickable actions |
| `link` | `<a>` | Navigation links |
| `heading` | `<h1>`–`<h6>` | Section headings (has `level` attr) |
| `list` | `<ul>`, `<ol>` | Lists |
| `listitem` | `<li>` | List items |
| `textbox` | `<input>`, `<textarea>` | Text inputs |
| `checkbox` | `<input type="checkbox">` | Checkboxes |
| `combobox` | `<select>`, combobox pattern | Dropdowns |
| `dialog` | `<dialog>`, `role="dialog"` | Modal dialogs |
| `group` | `<fieldset>`, `role="group"` | Grouped elements |
| `img` | `<img>` with alt text | Images |
| `generic` | `<div>`, `<span>` (no semantic role) | Unstyled containers |

### Attributes

| Attribute | Meaning | Appears on |
|-----------|---------|------------|
| `pressed=true` | Currently active/selected | `tab`, `button` |
| `checked=true/false/mixed` | Checkbox/toggle state | `checkbox`, `switch` |
| `expanded=true/false` | Collapsible state | `button`, `treeitem` |
| `disabled` | Not interactive | Any interactive element |
| `value="..."` | Current value | `textbox`, `combobox`, `slider` |
| `level=N` | Heading level | `heading` |

## Selector Formats

Playwright supports multiple selector engines. Use these with `e-cli click`, `e-cli wait`, `e-cli text`, etc.

### Role Selectors (Preferred)

Match elements by their ARIA role and accessible name:

```
role=tab[name="Sessions"]
role=button[name="Create"]
role=heading[name="Settings"][level=1]
role=textbox[name="Search"]
role=dialog[name="Confirm"]
role=listitem[name="dev-server"]
```

**Syntax:** `role=<role>[name="<accessible-name>"]`

Role selectors are the most robust because they match the semantic structure, not the visual presentation. They survive CSS class renames, DOM restructuring, and theme changes.

### Text Selectors

Match elements by visible text content:

```
text=Sessions
text=Create new session
text=Are you sure?
```

**Matching rules (Playwright `text=` engine):**
- `text=Sessions` (without inner quotes) = **case-insensitive substring** match, whitespace-trimmed
- `text="Sessions"` (with quotes inside) = **case-sensitive exact** match
- When in doubt, verify against the snapshot output rather than assuming

### Test ID Selectors

Match elements by `data-testid` attribute:

```
[data-testid=session-list]
[data-testid=new-session-btn]
[data-testid=settings-dialog]
```

Good for elements that lack meaningful accessible names but have test attributes.

### CSS Selectors

Standard CSS selectors as a fallback:

```
.sidebar-nav button
#main-content h1
.session-item:first-child
input[type="text"]
```

Use CSS selectors only when role/text/testid selectors are insufficient.

### Combining Selectors

Chain selectors with `>>` to scope within a parent:

```
role=navigation >> role=tab[name="Sessions"]
role=dialog >> role=button[name="Confirm"]
.sidebar >> text=Settings
```

## Translating Snapshot to Selector

Given this snapshot:
```
WebArea "MyApp"
  navigation "Main"
    tab "Sessions" pressed=true
    tab "Rooms"
    tab "Friends"
  main
    heading "Sessions" level=1
    list "Session list"
      listitem "dev-server"
```

To click the "Rooms" tab:
```bash
e-cli click 'role=tab[name="Rooms"]'
```

To verify the heading:
```bash
e-cli text 'role=heading[name="Sessions"]'
```

To click a specific session:
```bash
e-cli click 'role=listitem[name="dev-server"]'
```

To scope within navigation:
```bash
e-cli click 'role=navigation[name="Main"] >> role=tab[name="Rooms"]'
```

## Common Patterns

### Finding interactive elements

Scan the snapshot for these roles: `button`, `tab`, `link`, `textbox`, `checkbox`, `combobox`, `menuitem`. These are the elements you can click, fill, or toggle.

### Finding the active tab

Look for `pressed=true` on tab elements:
```
tab "Sessions" pressed=true    ← this is the active tab
tab "Rooms"                    ← inactive
```

### Finding expanded/collapsed sections

Look for `expanded=true/false`:
```
button "Advanced" expanded=false   ← collapsed, click to expand
```

### Counting list items

Use `eval` to count:
```bash
e-cli eval "document.querySelectorAll('[role=listitem]').length"
```

### Waiting for dynamic content

After clicking a tab, wait for the new content to appear:
```bash
e-cli click 'role=tab[name="Rooms"]'
e-cli wait 'role=heading[name="Rooms"]'
e-cli snapshot
```
