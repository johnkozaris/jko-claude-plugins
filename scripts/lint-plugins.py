#!/usr/bin/env python3
"""Lint the plugin marketplace for the defect classes that have actually shipped.

Every check here corresponds to a bug found in a real review of this repo:
  1. SKILL.md frontmatter description over the 1024-char limit
  2. Trigger-time skill or command bodies large enough to become handbooks
  3. Commands without routing frontmatter
  4. Pattern-ID citations (CODE-xx / ARCH-xx / AP-xx / DN-xx) that don't exist
     in the plugin's own catalog (drift between commands and references)
  5. Marketplace version/description out of sync with plugin.json, or the two
     marketplace.json files diverging
  6. Reference files never routed from their SKILL.md (orphans)
  7. Private project names leaking into plugin content
  8. Empty hooks.json stubs
  9. Slash-command references to command files that don't exist (e.g. a
     deleted command still cited in the root README or a plugin reference)

Exit code 0 = clean, 1 = findings. Output is the evidence.
"""

import json
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    yaml = None

ROOT = Path(__file__).resolve().parent.parent
PLUGINS = ROOT / "plugins"
DESCRIPTION_LIMIT = 1024
SKILL_BODY_WORD_LIMIT = 1600
COMMAND_BODY_WORD_LIMIT = 1200
PRIVATE_NAMES = ["kodosi"]  # lowercase; add private project names here
ID_PREFIXES = ("CODE", "ARCH", "AP", "DN")

findings: list[str] = []


def finding(msg: str) -> None:
    findings.append(msg)


def frontmatter_description(skill_md: Path) -> str | None:
    text = skill_md.read_text(encoding="utf-8")
    m = re.match(r"---\n(.*?)\n---", text, re.S)
    if not m:
        return None
    fm = m.group(1)
    dm = re.search(r"^description:\s*(.*)$", fm, re.M)
    if not dm:
        return None
    desc = dm.group(1)
    if desc.strip() in (">", ">-", "|"):  # folded/literal block scalar
        lines = []
        for line in fm[dm.end():].splitlines():
            if line.startswith((" ", "\t")):
                lines.append(line.strip())
            elif line.strip():
                break
        desc = " ".join(lines)
    return desc.strip().strip('"')


def check_descriptions() -> None:
    for skill_md in PLUGINS.glob("*/skills/*/SKILL.md"):
        desc = frontmatter_description(skill_md)
        rel = skill_md.relative_to(ROOT)
        if desc is None:
            finding(f"{rel}: no frontmatter description found")
        elif len(desc) > DESCRIPTION_LIMIT:
            finding(f"{rel}: description is {len(desc)} chars (limit {DESCRIPTION_LIMIT})")
        else:
            text = skill_md.read_text(encoding="utf-8")
            match = re.match(r"---\n(.*?)\n---", text, re.S)
            line = re.search(r"^description:\s*(.*)$", match.group(1), re.M) if match else None
            raw = line.group(1).strip() if line else ""
            if raw and raw not in (">", ">-", "|") and not raw.startswith(("'", '"')):
                if re.search(r":\s", raw):
                    finding(
                        f"{rel}: unquoted ':' in description can break YAML; "
                        "quote it or use a folded block"
                    )


def body_after_frontmatter(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    match = re.match(r"---\n.*?\n---\n?", text, re.S)
    return text[match.end():] if match else text


def check_context_budgets() -> None:
    for skill_md in PLUGINS.glob("*/skills/*/SKILL.md"):
        words = len(body_after_frontmatter(skill_md).split())
        if words > SKILL_BODY_WORD_LIMIT:
            finding(
                f"{skill_md.relative_to(ROOT)}: trigger-time body is {words} words "
                f"(limit {SKILL_BODY_WORD_LIMIT}); route detail to references"
            )
    for command in PLUGINS.glob("*/commands/*.md"):
        words = len(body_after_frontmatter(command).split())
        if words > COMMAND_BODY_WORD_LIMIT:
            finding(
                f"{command.relative_to(ROOT)}: command body is {words} words "
                f"(limit {COMMAND_BODY_WORD_LIMIT}); keep commands as thin workflows"
            )


def check_command_frontmatter() -> None:
    for command in PLUGINS.glob("*/commands/*.md"):
        text = command.read_text(encoding="utf-8")
        match = re.match(r"---\n(.*?)\n---", text, re.S)
        rel = command.relative_to(ROOT)
        if not match:
            finding(f"{rel}: command has no frontmatter")
        elif not re.search(r"^description:\s*\S", match.group(1), re.M):
            finding(f"{rel}: command frontmatter has no description")


def check_frontmatter_yaml() -> None:
    if yaml is None:
        return
    for path in [
        *PLUGINS.glob("*/skills/*/SKILL.md"),
        *PLUGINS.glob("*/commands/*.md"),
    ]:
        text = path.read_text(encoding="utf-8")
        match = re.match(r"---\n(.*?)\n---", text, re.S)
        if not match:
            continue
        try:
            parsed = yaml.safe_load(match.group(1))
        except yaml.YAMLError as exc:
            finding(f"{path.relative_to(ROOT)}: invalid YAML frontmatter: {exc}")
            continue
        if not isinstance(parsed, dict):
            finding(f"{path.relative_to(ROOT)}: frontmatter must be a YAML mapping")


def check_pattern_ids() -> None:
    id_re = re.compile(r"\b(" + "|".join(ID_PREFIXES) + r")-(\d{2})\b")
    for plugin in sorted(PLUGINS.iterdir()):
        if not plugin.is_dir():
            continue
        md_files = list(plugin.rglob("*.md"))
        # catalog = IDs defined in a heading (### CODE-01 ...) or as the first
        # cell of a table row (| DN-01 | ...) anywhere in the plugin
        catalog: set[str] = set()
        for f in md_files:
            for line in f.read_text(encoding="utf-8").splitlines():
                stripped = line.lstrip()
                is_def = stripped.startswith("#") or re.match(
                    r"\|\s*(" + "|".join(ID_PREFIXES) + r")-\d{2}\s*\|", stripped
                )
                if is_def:
                    for m in id_re.finditer(line):
                        catalog.add(m.group(0))
        if not catalog:
            continue
        prefixes_with_catalog = {c.split("-")[0] for c in catalog}
        for f in md_files:
            for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
                for m in id_re.finditer(line):
                    cid = m.group(0)
                    if m.group(1) in prefixes_with_catalog and cid not in catalog:
                        finding(
                            f"{f.relative_to(ROOT)}:{n}: cites {cid} but no such "
                            f"heading exists in this plugin's catalog"
                        )


def check_manifests() -> None:
    m1 = ROOT / ".claude-plugin" / "marketplace.json"
    m2 = ROOT / ".github" / "plugin" / "marketplace.json"
    if m1.read_bytes() != m2.read_bytes():
        finding(f"{m1.relative_to(ROOT)} and {m2.relative_to(ROOT)} differ")
    market = {p["name"]: p for p in json.loads(m1.read_text())["plugins"]}
    for plugin in sorted(PLUGINS.iterdir()):
        pj = plugin / ".claude-plugin" / "plugin.json"
        if not pj.exists():
            continue
        meta = json.loads(pj.read_text())
        name = meta["name"]
        if name not in market:
            finding(f"{name}: present in plugins/ but missing from marketplace.json")
            continue
        if market[name].get("version") != meta.get("version"):
            finding(
                f"{name}: marketplace version {market[name].get('version')} != "
                f"plugin.json version {meta.get('version')}"
            )
        if market[name].get("description") != meta.get("description"):
            finding(f"{name}: marketplace description differs from plugin.json")
        gh = plugin / ".github" / "plugin" / "plugin.json"
        if gh.exists():
            gh_meta = json.loads(gh.read_text())
            if gh_meta.get("version") != meta.get("version"):
                finding(f"{name}: .github plugin.json version differs from .claude-plugin")
            if gh_meta.get("description") != meta.get("description"):
                finding(f"{name}: .github plugin.json description differs from .claude-plugin")
    for name in market:
        if not (PLUGINS / name).is_dir():
            finding(f"{name}: in marketplace.json but plugins/{name}/ does not exist")


def check_reference_orphans() -> None:
    for skill_dir in PLUGINS.glob("*/skills/*"):
        skill_md = skill_dir / "SKILL.md"
        refs = skill_dir / "references"
        if not (skill_md.exists() and refs.is_dir()):
            continue
        body = skill_md.read_text(encoding="utf-8")
        for ref in sorted(refs.glob("*.md")):
            if ref.name not in body:
                finding(
                    f"{ref.relative_to(ROOT)}: never referenced from its SKILL.md "
                    f"(orphan — unreachable via routing)"
                )


def check_private_names() -> None:
    for f in PLUGINS.rglob("*"):
        if not f.is_file() or f.suffix in {".png", ".gif", ".jpg"}:
            continue
        try:
            text = f.read_text(encoding="utf-8").lower()
        except (UnicodeDecodeError, PermissionError):
            continue
        for name in PRIVATE_NAMES:
            if name in text:
                finding(f"{f.relative_to(ROOT)}: contains private project name '{name}'")


def check_hooks_stubs() -> None:
    for hooks in PLUGINS.glob("*/hooks/hooks.json"):
        data = json.loads(hooks.read_text())
        if data.get("hooks") == {}:
            finding(f"{hooks.relative_to(ROOT)}: empty hooks stub — delete it")


def check_command_references() -> None:
    # command names are valid marketplace-wide (plugins legitimately point at
    # each other's commands), and the namespaced /plugin:command form is legal
    all_commands = {p.stem for p in PLUGINS.glob("*/commands/*.md")}
    prefixes = {name.split("-")[0] for name in all_commands}
    cmd_re = re.compile(
        r"(?<![\w./])/(?:[a-z0-9-]+:)?((?:"
        + "|".join(map(re.escape, prefixes))
        + r")-[a-z0-9-]+)\b"
    )
    docs = [ROOT / "README.md", *PLUGINS.rglob("*.md")]
    for f in sorted(docs):
        for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
            for m in cmd_re.finditer(line):
                name = m.group(1)
                if name not in all_commands and not any(
                    c.startswith(name) or name.startswith(c) for c in all_commands
                ):
                    finding(
                        f"{f.relative_to(ROOT)}:{n}: references /{name} but no "
                        f"such command file exists in any plugin"
                    )


def main() -> int:
    for check in (
        check_descriptions,
        check_context_budgets,
        check_command_frontmatter,
        check_frontmatter_yaml,
        check_pattern_ids,
        check_manifests,
        check_reference_orphans,
        check_private_names,
        check_hooks_stubs,
        check_command_references,
    ):
        check()
    if findings:
        print(f"FAIL — {len(findings)} finding(s):\n")
        for f in findings:
            print(f"  {f}")
        return 1
    print("PASS — all plugin lint checks clean")
    return 0


if __name__ == "__main__":
    sys.exit(main())
