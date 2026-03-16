# Grep/Ripgrep Patterns for Dead Code Detection

Practical patterns for detecting dead code using ripgrep (rg) across any codebase. These are heuristics -- not 100% accurate, but fast first-pass detection.

## Core Technique: Definition vs. Reference Count

The fundamental approach: find all symbol definitions, then count references across the project. A symbol appearing only at its definition (count = 1) is a dead code candidate.

## Unused Function Detection

### Python
```bash
# List all function definitions
rg -o 'def (\w+)' -r '$1' --no-filename -t py | sort -u > /tmp/py_funcs.txt

# For each, count references (exclude definition line itself needs context)
while IFS= read -r func; do
  count=$(rg -w "$func" -t py -c --stats 2>/dev/null | tail -1 | grep -oP '\d+' | head -1)
  [ "${count:-0}" -le 1 ] && echo "Possibly unused: $func (refs: ${count:-0})"
done < /tmp/py_funcs.txt
```

### JavaScript / TypeScript
```bash
# Find exported functions/consts that may be unused
rg 'export (function|const|class|type|interface|enum) (\w+)' -t ts -t js -o -r '$2' --no-filename | sort -u | while read sym; do
  count=$(rg -w "\b${sym}\b" -t ts -t js -c 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
  [ "$count" -le 1 ] && echo "Possibly unused export: $sym (refs: $count)"
done
```

### Rust
```bash
# Find pub functions not referenced elsewhere
rg 'pub fn (\w+)' -t rust -o -r '$1' --no-filename | sort -u | while read func; do
  count=$(rg -w "$func" -t rust -c 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
  [ "$count" -le 1 ] && echo "Possibly unused pub fn: $func (refs: $count)"
done
```

## Unused Import Detection

### Python
```bash
# Find imports and check if the imported name is used in the file
rg -n '^(from \S+ )?import (.+)' -t py | while IFS=: read -r file line match; do
  # Extract imported names
  echo "$match" | grep -oP '\b\w+\b' | tail -n +2 | while read name; do
    uses=$(rg -c "\b${name}\b" "$file" 2>/dev/null)
    [ "${uses:-0}" -le 1 ] && echo "$file:$line Possibly unused import: $name"
  done
done
```

### JavaScript / TypeScript (simple check)
```bash
# Find named imports and check usage in same file
rg -n 'import \{([^}]+)\}' -t ts -t js | while IFS=: read -r file line match; do
  echo "$match" | grep -oP '\w+' | while read name; do
    [ "$name" = "import" ] && continue
    [ "$name" = "from" ] && continue
    uses=$(rg -c "\b${name}\b" "$file" 2>/dev/null)
    [ "${uses:-0}" -le 1 ] && echo "$file:$line Possibly unused import: $name"
  done
done
```

## Commented-Out Code Detection

```bash
# Multi-line blocks of commented code (5+ consecutive // lines with code-like content)
rg -n '^\s*//' -t ts -t js -t java -t rust | \
  awk -F: '{file=$1; line=$2} prev_file==file && line==prev_line+1 {count++} prev_file!=file || line!=prev_line+1 {if(count>=4) print prev_file":"(prev_line-count)": "count+1" consecutive commented lines"; count=0} {prev_file=file; prev_line=line}'

# Commented-out code patterns (assignments, function calls, control flow)
rg -n '^\s*(//|#)\s*(const|let|var|function|class|import|from|if|else|for|while|return|def |fn |pub |async |await )\b' --type-add 'src:*.{py,js,ts,jsx,tsx,rs,go,swift,cs,java}'  -t src
```

## Debug Artifact Detection

```bash
# JavaScript/TypeScript
rg -n 'console\.(log|debug|trace)\(' -t js -t ts --glob '!*test*' --glob '!*spec*'
# Note: console.error and console.warn are often intentional -- review separately
rg -n '\bdebugger\b' -t js -t ts

# Python
rg -n '(^|\s)(print\(|breakpoint\(\)|pdb\.set_trace|import pdb)' -t py --glob '!*test*'

# Rust
rg -n '(dbg!\(|println!\(|eprintln!\()' -t rust --glob '!*test*'
rg -n '(todo!\(|unimplemented!\()' -t rust

# Swift
rg -n '(print\(|dump\()' -t swift --glob '!*Test*' --glob '!*Preview*'

# C#
rg -n 'Console\.Write(Line)?\(' -t cs --glob '!*Test*'

# Go
rg -n 'fmt\.Print(ln|f)?\(' -t go --glob '!*_test.go'
```

## Lint Suppression Detection

```bash
# Find all dead-code-related lint suppressions
rg -n '#\[allow\((dead_code|unused' -t rust
rg -n '(eslint-disable|eslint-disable-next-line).*(no-unused|unused)' -t js -t ts
rg -n '# noqa: F(401|811|841)' -t py
rg -n '#pragma warning disable CS(0168|0219)' -t cs
rg -n '@SuppressWarnings\("unused' -t java
```

## Orphaned Test File Detection

```bash
# Find test files whose corresponding source doesn't exist
for test_file in $(fd -e test.ts -e test.js -e spec.ts -e spec.js); do
  src_file=$(echo "$test_file" | sed 's/\.test\./\./' | sed 's/\.spec\./\./' | sed 's/__tests__\///')
  [ ! -f "$src_file" ] && echo "Orphaned test: $test_file (no source: $src_file)"
done

# Python
for test_file in $(fd 'test_.*\.py$'); do
  src_name=$(basename "$test_file" | sed 's/^test_//')
  found=$(fd "$src_name" --exclude '*test*')
  [ -z "$found" ] && echo "Orphaned test: $test_file"
done
```

## Skipped Test Detection

```bash
# Find permanently skipped tests across languages
rg -n '@(skip|ignore|disabled|pytest\.mark\.skip)' --type-add 'test:*test*' -t test
rg -n '(xit|xdescribe|xcontext|it\.skip|describe\.skip)\(' -t js -t ts
rg -n '#\[ignore\]' -t rust
rg -n '\[Ignore\]' -t cs
rg -n '@Disabled' -t java
```

## TODO / FIXME / HACK Markers

```bash
# Count markers indicating incomplete or temporary code
rg -n '\b(TODO|FIXME|HACK|XXX|TEMP|TEMPORARY|WORKAROUND)\b' --stats 2>&1 | tail -5
```

## Dead Store Detection (Heuristic)

```bash
# Variables assigned on consecutive lines (second assignment overwrites first without read)
# This is a rough heuristic -- compilers do this better
rg -n '(\w+)\s*=' -t py | sort | uniq -d -f1
```

## Quick Full Scan Script

Combine the above into a single scan:
```bash
#!/bin/bash
# dead-code-scan.sh - Quick heuristic dead code scan
# Usage: dead-code-scan.sh [directory]

DIR="${1:-.}"
echo "=== Dead Code Scan: $DIR ==="

echo -e "\n--- Debug Artifacts ---"
rg -c '(console\.log|print\(|dbg!\(|println!\(|debugger|breakpoint\(\))' "$DIR" --glob '!*test*' --glob '!*spec*' --glob '!node_modules*' --glob '!target*' --glob '!.git*' 2>/dev/null | awk -F: '{s+=$2} END {print "Total:", s+0}'

echo -e "\n--- Lint Suppressions ---"
rg -c '(#\[allow\((dead_code|unused)|eslint-disable.*unused|# noqa: F4|@SuppressWarnings)' "$DIR" --glob '!node_modules*' --glob '!target*' 2>/dev/null | awk -F: '{s+=$2} END {print "Total:", s+0}'

echo -e "\n--- TODO/FIXME/HACK ---"
rg -c '\b(TODO|FIXME|HACK|XXX)\b' "$DIR" --glob '!node_modules*' --glob '!target*' 2>/dev/null | awk -F: '{s+=$2} END {print "Total:", s+0}'

echo -e "\n--- Commented Code Blocks (heuristic) ---"
rg -c '^\s*(//|#)\s*(const|let|var|function|class|import|def |fn |pub )\b' "$DIR" --glob '!node_modules*' --glob '!target*' 2>/dev/null | awk -F: '{s+=$2} END {print "Total:", s+0}'

echo -e "\n--- Skipped Tests ---"
rg -c '(@skip|@ignore|xit\(|xdescribe\(|\.skip\(|#\[ignore\]|\[Ignore\]|@Disabled|@pytest\.mark\.skip)' "$DIR" 2>/dev/null | awk -F: '{s+=$2} END {print "Total:", s+0}'

echo -e "\nScan complete. Run language-specific tools for deeper analysis."
```
