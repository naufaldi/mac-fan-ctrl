# GitHub Issues Workflow

This project uses **GitHub Issues** as the primary task tracking system. Claude (AI agent) manages issues autonomously during work sessions.

## Session Start

1. Run `gh issue list --state open` to see current open issues
2. Ask the user which issue to work on, or create a new one if needed

## Creating Issues

```bash
gh issue create --title "<title>" --label "<label1>,<label2>" --body "<body>"
```

- Always include at least one **phase label** and one **area label**
- Body should include: description, acceptance criteria, definition of done

## Working on Issues

- Reference the issue number in progress updates:
  ```bash
  gh issue comment <N> --body "Progress: <update>"
  ```

## Completing Issues

- Use `closes #N` in the commit message — GitHub auto-closes the issue when the commit lands on `main`
- Commit format: `<type>: <description> (closes #N)`
- Examples:
  - `feat: add menu bar tray icon (closes #3)`
  - `fix: correct fan speed calculation (closes #7)`
  - `refactor: extract sensor polling module (closes #12)`

## Labels

| Label         | Color     | Purpose                      |
|---------------|-----------|------------------------------|
| `phase-a`     | `#0075ca` | Phase A: Read-only monitoring |
| `phase-b`     | `#e99695` | Phase B: Fan control + safety |
| `phase-c`     | `#0e8a16` | Phase C: Polish + hardening   |
| `frontend`    | `#7057ff` | Svelte/TypeScript frontend    |
| `backend`     | `#d73a4a` | Rust backend                  |
| `smc`         | `#fbca04` | SMC hardware layer            |
| `ui`          | `#f9d0c4` | Design and styling            |
| `bug`         | (default) | Bug fix                       |
| `enhancement` | (default) | New feature                   |

## Git Convention

- All work commits directly to `main` (no feature branches)
- Every commit referencing an issue must include `closes #N` or `fixes #N`
- Follow conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`

## Definition of Done

Before closing an issue, verify:

- [ ] Acceptance criteria implemented and verified
- [ ] Unit + integration tests (including failure path)
- [ ] Error and fallback strategy documented
- [ ] Rollback note included
- [ ] No known crashes introduced
