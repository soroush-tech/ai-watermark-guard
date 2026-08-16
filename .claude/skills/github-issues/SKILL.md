---
description: Drafting, referencing, and creating GitHub issues in this repo - templates, issue-reference style, and the show-before-create rule. Use when drafting or filing any bug report or feature request, or when referencing issues in issue/PR bodies.
---

# GitHub issues

## Use the issue templates

Any bug report or feature request must follow the matching template in
`.github/ISSUE_TEMPLATE/` (`1.bug_report.yml`, `2.feature_request.yml`). Read the
template first; use its exact section headings and order. Blank issues are
disabled (`config.yml`); questions and open-ended ideas go to Discussions.

This repo has no RFC/Epic/Task hierarchy and no milestones - issues are flat.
For rework discovered later, file a new issue and reference the issue it reworks
in the body.

## Reference issues bare - GitHub renders the title

GitHub auto-renders the title for a bare `#123`, so don't hand-write it.

✗ `- [ ] #136 - Bug report: --staged --fix loses the unstaged half of a partial stage`
✓ `- #136`

(Release-notes files are the exception: they are also browsed in-repo where bare
references don't autolink, so there a reference is an absolute URL - see the
[`release-notes`](../release-notes/SKILL.md) skill.)

## Show before creating

Always show the drafted issue to the user and wait for explicit verification
before creating it on GitHub.

## Mandatory metadata on every created issue

- a **label** - the templates apply `status: needs triage` on their own; when
  triaging, replace it rather than stacking on top.
- **assignee** set to the repo owner (self).
