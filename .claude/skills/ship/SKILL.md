---
name: ship
description: Stage all changes, create a comprehensive conventional commit, and push to the current branch. Use this skill when the user says "/ship" to commit and push their work.
---

# Ship — Stage, Commit & Push

Automate the full git shipping workflow: stage everything, write a meaningful conventional commit message, and push to the current remote branch. No confirmation prompts — just ship it.

## Workflow

### 1. Gather context

Run these in parallel to understand the current state:

- `git status` — see what's changed (staged, unstaged, untracked)
- `git diff` — see the actual unstaged code changes
- `git diff --cached` — see any already-staged changes
- `git log --oneline -5` — recent commits for context
- `git branch --show-current` — confirm the current branch

If there are no changes at all (nothing staged, unstaged, or untracked), tell the user there's nothing to ship and stop.

### 2. Stage all changes

```bash
git add -A
```

### 3. Write the commit message

Analyze the full diff (`git diff --cached` after staging) and write a conventional commit message.

#### Format

```
<type>(<scope>): <short summary>

<body — what changed and why, in enough detail that someone reading
the git log months from now can understand the change without opening
the diff>

Co-Authored-By: Claude <noreply@anthropic.com>
```

#### Types

Use the conventional commit type that best fits the **primary intent** of the changes:

- `feat` — new functionality for the user
- `fix` — bug fix
- `refactor` — restructuring without behavior change
- `style` — formatting, whitespace, missing semicolons
- `docs` — documentation only
- `test` — adding or updating tests
- `chore` — build scripts, dependencies, config
- `perf` — performance improvement

#### Scope

Use a short scope that identifies the area of the codebase (e.g., `auth`, `ui`, `api`, `pomodoro`, `notes`). Omit the scope if the change is truly cross-cutting.

#### Writing a good summary

- Keep the first line under 72 characters
- Use imperative mood ("add", "fix", "update" — not "added", "fixes")
- The summary should be specific enough to be useful in `git log --oneline`

#### Writing a good body

- Explain **what** changed and **why**, not just which files were touched
- If multiple things changed, use bullet points grouped by area
- Mention any non-obvious side effects or trade-offs
- Don't list every single file — focus on the meaningful changes
- If there are many unrelated changes, that's fine — summarize the main themes

### 4. Commit

Use a heredoc to pass the message so multi-line formatting is preserved:

```bash
git commit -m "$(cat <<'EOF'
<the commit message>
EOF
)"
```

### 5. Push

Push to the current branch's remote tracking branch. If no upstream is set, push with `-u`:

```bash
# If upstream exists:
git push

# If no upstream:
git push -u origin <current-branch>
```

### 6. Confirm

Tell the user what happened — the commit hash, a brief summary, and which branch it was pushed to. Keep it short, something like:

```
Shipped `a1b2c3d` to `main` — feat(pomodoro): add break skip button
```

## Important

- Never skip the `Co-Authored-By` trailer.
- Never use `--force` or `--force-with-lease` unless the user explicitly asks.
- Never use `--no-verify` — if a hook fails, report the error and stop.
- If the push fails (e.g., remote is ahead), tell the user and suggest pulling first. Don't auto-rebase or force push.
