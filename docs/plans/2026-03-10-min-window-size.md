# Minimum Window Size Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prevent the window from being resized to an unusably small size (closes #43)

**Architecture:** Add `minWidth` and `minHeight` to Tauri window config. Values derived from layout: sensor pane (300px) + fan pane columns (100+280+~100px) = ~780px wide; header + 2 fan rows + footer = ~440px tall.

**Tech Stack:** Tauri v2 config (JSON)

---

### Task 1: Add minimum window dimensions

**Files:**
- Modify: `src-tauri/tauri.conf.json:14-19`

**Step 1: Add minWidth and minHeight to window config**

```json
"windows": [
  {
    "title": "Mac Fan Control",
    "width": 960,
    "height": 540,
    "minWidth": 780,
    "minHeight": 440
  }
]
```

**Step 2: Verify with `pnpm tauri dev`**

- Try resizing window — it should stop at 780x440
- Both fan and sensor panes should remain usable at minimum size

**Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "fix: set minimum window size to prevent unusable resize (closes #43)"
```
