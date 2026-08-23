# Code Cleanup

Product-aware cleanup for semantically dead code, dead feature islands, split
brain, agent residue, and unused dependencies, settings, and artifacts. It
starts from the product's users, goals, and boundaries, then reviews every
relevant file rather than only obvious unused symbols.

## Use

Claude Code exposes the installed plugin skill as
`/code-cleanup:code-cleanup`. In Copilot CLI, ask Copilot to use the
`/code-cleanup` skill.

For exhaustive reviews, the command can use the bundled hash-bound inventory:

```bash
python3 "${CLAUDE_PLUGIN_ROOT}/scripts/code-cleanup-audit.py" init \
  --repo . --ledger .code-cleanup/audit.tsv

python3 "${CLAUDE_PLUGIN_ROOT}/scripts/code-cleanup-audit.py" check \
  --repo . --ledger .code-cleanup/audit.tsv --require-resolved
```

The ledger proves that every file was considered and that evidence still
matches the reviewed bytes. It does not decide whether a file is live.

## Skill

The `code-cleanup` skill guides product-wide semantic cleanup without imposing a
large checklist or language-specific tool catalog.

## License

MIT
