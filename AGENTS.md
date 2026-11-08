# Agent Guidelines

## Agent skills

### Issue tracker

Local markdown issues tracked under `.scratch/<feature-slug>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Canonical 5-role triage vocabulary. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout with root `GLOSSARY.md` and `docs/adr/`. See `docs/agents/domain.md`.

## Verification & Guardrails

- Test suite: `cargo test`
- Linter: `cargo clippy --all-targets -- -D warnings`
- CI workflow: `.github/workflows/ci.yml` (Windows runner)


