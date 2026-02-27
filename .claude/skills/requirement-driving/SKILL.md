---
name: requirement-driving
description: Requirement-document-driven development. Develop based on requirement docs, track status, and guard against regressions.
---

# Requirement-Driving - Document-Driven Development

Core principle: **Develop based on requirement documents, ensure changes don't break existing functionality.**

The user writes requirement documents themselves. Your responsibilities: read the document, auto-register it in the
tracker, enrich incomplete requirements by discussing with the user, analyze feasibility, execute development, and
manage status — all automatically.

## User Input

$ARGUMENTS

## Conventions

- **Requirements directory:** `docs/requirements/`
- **Tracker file:** `docs/requirements/TRACKER.md`
- **Numbering:** `REQ-XXX` format, auto-incrementing
- **Status flow:** `Todo` → `In Progress` → `Done`

### TRACKER.md Format

```markdown
# Requirement Tracker

Latest ID: N

| ID | Document | Status | Completed | Notes |
|----|----------|--------|-----------|-------|
| REQ-001 | xxx.md | Done | 2026-02-26 | - |
| REQ-002 | yyy.md | In Progress | - | - |
```

Auto-create and initialize TRACKER.md if it doesn't exist. Always read latest state before any operation.

## Modes

Automatically determine mode based on arguments:

---

### Mode 1: View Status

**Trigger:** Argument is `status`, empty, or user asks to view status.

1. Read TRACKER.md
2. Display all requirements with current status, grouped by status with counts
3. Highlight any `In Progress` requirements

---

### Mode 2: Receive Requirement Document

**Trigger:** Argument points to a requirement document (filename or path).

1. Read the specified requirement document
2. Check if the document is already tracked in TRACKER.md
    - Not tracked → auto-assign next ID, add to TRACKER.md with status `Todo`
    - Already tracked → read current status
3. **Enrich the requirement:** If the document is brief or incomplete, proactively help the user flesh it out through
   discussion:
    - Clarify ambiguous functionality descriptions
    - Suggest acceptance criteria if missing
    - Identify edge cases and error scenarios not covered
    - Point out missing details (API definitions, data formats, performance requirements, etc.)
    - Assess impact on existing features
    - Evaluate technical feasibility
4. After discussion, help the user update the document with agreed-upon additions
5. Ask the user: start execution now?

---

### Mode 3: Execute Requirement

**Trigger:** User explicitly asks to execute/develop a requirement, or confirms execution after Mode 2 analysis.

#### Pre-check

- `In Progress` → confirm whether to continue
- `Done` → confirm whether to redo

#### Impact Analysis

- Review all `Done` and `In Progress` requirement documents in `docs/requirements/`
- Assess whether this development could affect existing functionality
- **If there is impact, clearly inform the user which requirements may be affected and how**

#### Design Implementation Plan

1. Based on the requirement document and current codebase, design an implementation plan
2. Include: approach overview, files to modify/create, step-by-step plan
3. Present to user, wait for confirmation

#### Execute Development

1. Auto-update TRACKER.md status to `In Progress`
2. Implement step by step, briefly report progress at key milestones

#### Wrap Up

1. Auto-update TRACKER.md status to `Done`, record completion date
2. Completion summary: files modified, acceptance criteria checklist, items requiring manual verification

---

## Notes

- All TRACKER.md updates (adding records, status changes, ID increments) are handled automatically
- Impact analysis is mandatory when executing a requirement — never skip it
- Keep document paths consistent between the filesystem and TRACKER.md references
