# Electron Playwright Validator

Persistent Electron UI validation through a bundled Playwright/CDP CLI.

The skill separates launch, interaction, and product success; discovers actual
renderer windows and accessibility state; bounds visual context; and turns
important findings into normal project E2E tests.

The bundled `scripts/e-cli` requires Node.js 18+, Electron, and Playwright from
the target project. The `electron-playwright-validator` skill owns the complete
validation workflow.
