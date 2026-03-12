# Agent Instructions

## Serena for context, changes

- Make use of Serena MCP for file searching and editing.

## HTMX-First Policy

- Implement UI behavior with pure HTMX by default.
- Do not add client-side JavaScript for UI state or interaction flows.
- The only pre-approved JavaScript exception is browser timezone detection/hinting.
- Any other JavaScript exception requires explicit user approval before implementation.
- Validate this policy in every related change and review.

## Debug Protocol (Live Local Target Only)

- Debug locally.
- Test only against a live local target by calling real HTTP endpoints.
- For debugging sessions, explicitly set the app port to `5656` and then run `just dev`.
- Never rely on default port bindings for debug runs.
- Start and stop the debug server as needed during testing.
- Only stop the server process started by the agent for port `5656`.
- Never kill the user's dev server.
- If port `5656` is already in use, do not terminate that process; report it and stop.
- Monitor server logs while reproducing and validating issues.
- Do not run code-level checks for debugging (`cargo test`, `cargo check`, `cargo clippy`) unless the user explicitly requests them.
- Do not use JavaScript for debugging or testing flows; only explicitly approved JavaScript exceptions apply.

## Secrets Handling (SOPS)

When working with secrets (for example `secrets.yaml` encrypted by SOPS), follow these rules:

- Secrets must be handled transiently only (in-process environment variables or pipes).
- Never print secret values to console, logs, traces, or command output.
- Never write decrypted secrets to disk (including temp files, `.env` files, cached artifacts, or committed files).
- Never commit plaintext secret material to git.
- Prefer process-scoped loading patterns like `sops -d ... | yq ...` piped directly into environment export for the executing command.

## Beads Interop (No GSD Framework Changes)

- Bind one `bead_id` (format `B-####`) before any `/gsd-*` workflow begins.
- Use exactly one active bead per `/gsd-*` run.
- Prefix these artifacts with `[bead:B-####]`: todo title, plan task `<name>`, checkpoint output, summary header.
- On completion transitions, append a ledger line to `.planning/beads-log.md`:
  `YYYY-MM-DD | bead:B-#### | from:<state> | to:<state> | evidence:<path>`
- Example ledger line:
  `2026-02-24 | bead:B-1042 | from:in_progress | to:completed | evidence:.planning/phases/phase-01/verification.md`
- If no `bead_id` is provided, stop and request one before continuing.

## Beads Issue Tracking

This project uses **bd** (beads) for issue tracking. Issues are stored in `.beads/` and tracked in git.

### Essential Commands

```bash
bd ready              # Show issues ready to work (no blockers)
bd list --status=open # All open issues
bd show <id>          # Full issue details with dependencies
bd create --title="..." --type=task --priority=2
bd update <id> --status=in_progress
bd close <id> --reason="Completed"
bd close <id1> <id2>  # Close multiple issues at once
bd sync               # Commit and push beads changes
```

### Workflow Pattern

1. **Start**: Run `bd ready` to find actionable work
2. **Claim**: `bd update <id> --status=in_progress` immediately followed `bd sync --flush-only`
3. **Work**: Implement the task
4. **Complete**: `bd close <id>`, followed by `bd update --append-notes` with a complete description of what was done, followed by `bd sync --flush-only`
5. **Sync**: Always run `bd sync` at session end

### Key Concepts

- **Dependencies**: Issues can block other issues. `bd ready` shows only unblocked work.
- **Priority**: P0=critical, P1=high, P2=medium, P3=low, P4=backlog (use numbers, not words)
- **Types**: task, bug, feature, epic, question, docs
- **Blocking**: `bd dep add <issue> <depends-on>` to add dependencies

### Best Practices

- Check `bd ready` at session start to find available work
- Update status as you work (`in_progress` -> closed)
- Create new issues with `bd create` when you discover tasks during work
- Use descriptive titles and set appropriate priority/type
- Always `bd sync` before ending a session

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Use `bd create` for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - `bd close <id>` finished work, `bd update <id> --status=in_progress` for in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

## Serena Project State

- Serena config lives in `.serena/project.yml` and `~/.serena/serena_config.yml`
- Onboarding writes memory files under `.serena/memories/`
- Treat `.serena/` as local agent state unless explicitly intended for version control

<!-- bv-agent-instructions-v1 -->

<!-- BEGIN BEADS INTEGRATION -->
## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Dolt-powered version control with native sync
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json
bd create "Issue title" --description="What this issue is about" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update <id> --claim --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task atomically**: `bd update <id> --claim`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details about what was found" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Auto-Sync

bd automatically syncs via Dolt:

- Each write auto-commits to Dolt history
- Use `bd dolt push`/`bd dolt pull` for remote sync
- No manual export/import needed!

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and docs/QUICKSTART.md.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

<!-- END BEADS INTEGRATION -->
