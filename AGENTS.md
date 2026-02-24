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

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Validate against a live local target by calling real endpoints on port `5656` and confirming expected logs/results
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
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
