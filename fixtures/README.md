# Compatibility Fixtures

Fixtures capture the stable compatibility contract used while rewriting `quantex-cli` as Rust-native `agx`.

## Policy

- Keep reference fixtures small and reviewable.
- Prefer stable fields over environment-dependent paths, timestamps, versions, and installer availability.
- When a command intentionally changes shape, update the fixture in the same commit as the implementation and OpenSpec task update.
- Do not include generated `runId`, `timestamp`, local absolute binary paths, or machine-specific PATH results in golden fixtures.

## Fixture Groups

- `reference/commands.json` records the reference command catalog names and schema refs from `quantex-cli/src/commands/commands.ts`, translated to the canonical `agx` entrypoint.
- `reference/agents.json` records the supported agent catalog names and aliases expected by AGX.
- `reference/schema-summary.json` records the schema documents AGX must expose.
- `reference/error-envelope.json` records a representative structured error envelope shape with dynamic metadata removed.
