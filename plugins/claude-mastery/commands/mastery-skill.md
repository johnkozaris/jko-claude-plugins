Help me create a new Claude Code skill following best practices.

Interview me using the AskUserQuestion tool to understand:
1. What category does this skill fall into? (Library/API Reference, Product Verification, Data Fetching, Business Process, Code Scaffolding, Code Quality, CI/CD, Runbook, Infrastructure Operations)
2. What problem does this skill solve?
3. What triggers should activate this skill? (specific phrases, situations, contexts)
4. What tools or commands does the skill need?
5. What are the known gotchas or failure modes?
6. Should it include verification scripts?

Then create the full skill folder structure:
```
skills/<skill-name>/
  SKILL.md              # Description as trigger conditions, instructions (WHAT not HOW), gotchas section
  references/           # Detailed docs for progressive disclosure
  scripts/              # Helper scripts (verification, data fetching)
  assets/               # Templates, configs
```

Follow these rules:
- Description field = trigger conditions, NOT a summary
- Don't over-specify steps — tell WHAT, not HOW
- Include a gotchas section (even if empty, to be filled over time)
- Include a verification step
- Include helper scripts where useful
- Reference other files for detailed content (progressive disclosure)
