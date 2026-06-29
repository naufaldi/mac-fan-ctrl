# FanGuard — Design System

> Parchment command terminal meets Apple HIG — warm paper surfaces beneath monochrome native controls, with a single ember signal reserved for heat.

FanGuard adopts the **ElevenLabs** warm-parchment visual language ([refero.design style](https://styles.refero.design/style/031056ff-7af1-46db-8daa-115f731c5d26)) and delivers it through **Apple Human Interface Guidelines** geometry so the app feels native to macOS rather than like a web marketing page dropped into a utility window.

The core idea: an off-white parchment canvas (`#fdfcfc`) layered with warm-sand surfaces (`#f5f3f1`) separated by hairline `#e5e5e5` dividers. All interactive chrome is monochrome ink (`#000000`) on parchment — color is never used as a call to action. The one sanctioned exception is **Ember Orange `#ff4704`**, used exclusively as a restrained safety signal for hot temperatures (≥85°C). Elevation comes from surface contrast and 1px hairlines, never from soft blurs.

---

## Apple HIG Adaptation

The ElevenLabs reference is a web-marketing system (9999px pill buttons, 20–24px rounded cards, Inter/Geist Mono, decorative gradient orbs). FanGuard is a macOS desktop utility, so the parchment *palette* is mapped onto native macOS *geometry*:

| ElevenLabs (web) | FanGuard (macOS HIG) |
| --- | --- |
| 9999px pill buttons | 7px rectangular rounded push buttons (macOS standard) |
| Pill toggle tabs | HIG segmented control (connected 6px segments) |
| Text-link preset menu | HIG pop-up button (rectangular rounded + chevron) |
| 20–24px floating rounded cards | Edge-to-edge panes with 1px hairline dividers |
| Inter + Geist Mono | SF Pro Text (UI) + SF Mono (data) — Apple system fonts |
| Decorative violet/orange gradient orbs | Omitted (not appropriate for a utility) |
| 4px flush editorial inputs | 5px native macOS text fields |

Native window chrome (traffic-light close/minimize/zoom buttons, title bar) stays OS-provided via Tauri `decorations: true`. Motion is calm and restrained — no cinematic physics, consistent with macOS system utilities. `9999px` capsules are reserved only for HIG capsule contexts (e.g. small status badges); they are not the default button shape.

---

## Tokens — Colors

| Name | Value | Token | Role |
| --- | --- | --- | --- |
| Parchment White | `#fdfcfc` | `--color-parchment-white` | Window canvas — the dominant background |
| Warm Sand | `#f5f3f1` | `--color-warm-sand` | Secondary surface — table headers, inset panes, hover base |
| Sand Hover | `#efedea` | `--color-sand-hover` | Row/control hover (one step darker than Warm Sand) |
| Ash Border | `#e5e5e5` | `--color-ash-border` | All hairline borders and dividers |
| Ash Strong | `#d4d4d4` | `--color-ash-strong` | Emphasized borders (dialog outlines, pressed states) |
| Midnight Ink | `#000000` | `--color-midnight-ink` | Primary text, headline text, filled control background, icon fills |
| Driftwood | `#777169` | `--color-driftwood` | Secondary text, muted labels, icon strokes |
| Fog | `#a59f97` | `--color-fog` | Tertiary helper text, placeholder text |
| Silver Mist | `#b1b0b0` | `--color-silver-mist` | Unknown/null status, disabled chrome |
| Ember Orange | `#ff4704` | `--color-ember-orange` | **Safety only** — hot temperature (≥85°C) signal |
| Void Violet | `#0447ff` | `--color-void-violet` | **Decorative only** — unused in utility chrome (reserved) |

### Semantic status mapping

Temperature status is monochrome by default; ember appears only for hot:

| Status | Condition | Token |
| --- | --- | --- |
| normal | `< 70°C` | `--color-driftwood` (monochrome dot) |
| warm | `70–84°C` | `--color-driftwood` (monochrome dot — no yellow) |
| hot | `≥ 85°C` | `--color-ember-orange` (the single safety signal) |
| unknown | `null` / N/A | `--color-silver-mist` |

---

## Tokens — Typography

Type is set in Apple's system fonts for UI/data, with DM Sans weight 300 reserved for display-size headings (the ElevenLabs "Waldenburg weight 300" personality, via its documented DM Sans 300 substitute).

### Font families

| Role | Family | Token | Weights |
| --- | --- | --- | --- |
| UI text | SF Pro Text (system) | `--font-ui` | 400, 500 |
| Data / metrics | SF Mono (system) | `--font-mono` | 400, tabular-nums |
| Display headings | DM Sans | `--font-display` | 300 (Waldenburg substitute) |

Font stacks:
```
--font-ui: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", system-ui, sans-serif;
--font-mono: "SF Mono", SFMono-Regular, ui-monospace, Menlo, Monaco, Consolas, monospace;
--font-display: "DM Sans", -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
```

> **Implementation note:** `--font-display` prefers DM Sans 300 (the documented Waldenburg substitute) when bundled, but falls back to the Apple system font at weight 300 with `-0.02em` tracking. On macOS this renders as SF Pro Display Light — which carries the same light, tightly-tracked headline personality natively and keeps the app fully offline. DM Sans is optional to bundle; the fallback is the default rendering.

### Type scale (utility-dense)

| Role | Size | Line height | Letter spacing | Weight | Font |
| --- | --- | --- | --- | --- | --- |
| caption | 10px | 1.4 | 0.1px | 400 | SF Pro Text |
| micro | 11px | 1.45 | 0.11px | 400/500 | SF Pro Text |
| label | 12px | 1.45 | 0.12px | 400/500 | SF Pro Text |
| body | 13px | 1.5 | 0.13px | 400 | SF Pro Text |
| data | 13px | 1.4 | 0 | 400 | SF Mono (tabular-nums) |
| subheading | 15px | 1.45 | 0.15px | 500 | SF Pro Text |
| heading-sm | 20px | 1.3 | -0.2px | 300 | DM Sans |
| heading | 28px | 1.2 | -0.56px | 300 | DM Sans |
| display | 36px | 1.13 | -0.72px | 300 | DM Sans |

All numeric metrics (RPM, °C, percentages) use `font-variant-numeric: tabular-nums` via SF Mono so columns align and values don't jitter.

---

## Tokens — Spacing, Radii, Shadows

### Spacing

Base unit `4px`. Common steps: `4, 8, 12, 16, 20, 24, 32`. Pane padding `12–16px`; dialog padding `20–24px`; element gap `8px`.

### Border radius (HIG)

| Element | Value | Token |
| --- | --- | --- |
| push button | 7px | `--radius-button` |
| segmented control | 6px | `--radius-segmented` |
| input / text field | 5px | `--radius-input` |
| dialog / panel | 10px | `--radius-dialog` |
| list card | 12px | `--radius-card` |
| capsule badge | 9999px | `--radius-capsule` (reserved) |

### Shadows (hairline only)

| Name | Value | Token |
| --- | --- | --- |
| hairline ring | `rgba(0,0,0,0.06) 0 0 0 1px, rgba(0,0,0,0.04) 0 1px 2px 0` | `--shadow-hairline` |
| elevated panel | `rgba(0,0,0,0.4) 0 0 1px 0, rgba(0,0,0,0.04) 0 1px 1px 0, rgba(0,0,0,0.04) 0 2px 4px 0` | `--shadow-elevated` |
| inset | `rgba(0,0,0,0.075) 0 0 0 0.5px inset` | `--shadow-inset` |

Blur never exceeds 4px and opacity never exceeds 0.04. Elevation is primarily signaled by surface contrast (`#fdfcfc` → `#f5f3f1` → `#ffffff`) and 1px `#e5e5e5` dividers.

---

## Surfaces

| Level | Name | Value | Purpose |
| --- | --- | --- | --- |
| 1 | Canvas | `#fdfcfc` | Window background, pane/row base |
| 2 | Warm Sand | `#f5f3f1` | Table headers, inset panes, secondary tiles |
| 3 | Border | `#e5e5e5` | Hairline borders and dividers |
| 4 | Elevated | `#ffffff` | Dialog panels and floating menus (with `--shadow-elevated`) |

---

## Components

### HIG Push Button
**Role:** Window chrome (Pin on Top, Hide, Preferences, About), dialog actions.

Parchment `#fdfcfc` background, `#000000` text, 7px radius, `12px 16px` padding, 1px `#e5e5e5` border, `--shadow-hairline`. SF Pro Text 12px weight 400. Hover → `#f5f3f1`. Active/pressed → `--shadow-inset`. Focus → 2px `#000000` ring offset by 2px parchment.

### Filled Ink Primary Button
**Role:** Primary dialog action (OK, Save), affirmative single action.

`#000000` background, `#ffffff` text, 7px radius, 1px `#000000` border. SF Pro Text 12px weight 500. Hover → `#1a1a1a`. Disabled → Silver Mist bg + `#a59f97` text. Used sparingly — one primary per dialog.

### HIG Segmented Control
**Role:** Fan Auto/Custom mode toggle, preferences tray-mode toggle.

Container: `#f5f3f1` background, 6px radius, 1px `#e5e5e5` border, connected segments divided by 1px `#e5e5e5`. Inactive segment: transparent bg, `#000000` text. Active segment: `#000000` bg, `#ffffff` text, SF Pro Text 11–12px weight 500. The Auto/Custom fan toggle is a 2-segment control; the active mode is filled ink, the other is outlined.

### HIG Pop-up Button
**Role:** Active preset selector.

Parchment `#fdfcfc` background, `#000000` text, 7px radius, 1px `#e5e5e5` border, `--shadow-hairline`, SF Pro Text 12px weight 400, with a chevron glyph on the right. Opens a floating menu (see List Menu).

### List Menu (dropdown)
**Role:** Preset list, pop-up menus.

`#ffffff` elevated panel, 10px radius, `--shadow-elevated`, min-width matches trigger. Menu items: `8px 12px` padding, SF Pro Text 12px, hairline `#e5e5e5` dividers between groups. Hover/focus → `#f5f3f1` (no blue). Selected item → ink `Check` icon (lucide), label weight 500. Destructive (delete) → `X` icon in Driftwood, hover to Ember only if confirming a hot/unsafe preset. Never `bg-blue-500`.

### Edge-to-Edge List Pane
**Role:** Fan table (left) and sensor list (right).

Parchment `#fdfcfc` canvas. Column header row: `#f5f3f1` Warm Sand, SF Pro Text 11px weight 500 Driftwood, 1px `#e5e5e5` bottom border. Data rows separated by 1px `#e5e5e5` hairline `divide-y` (no zebra fill). Hover → `#efedea`. Values in SF Mono 12px tabular-nums. Panes divided by a 1px `#e5e5e5` vertical hairline. No outer rounded card — panes run edge-to-edge like Activity Monitor.

### Status Dot
**Role:** Sensor temperature state.

6px circle. `< 85°C` → Driftwood `#777169` (monochrome). `≥ 85°C` → Ember `#ff4704` (the single safety signal). `null` → Silver Mist `#b1b0b0`. No green/yellow.

### HIG Dialog Panel
**Role:** FanControlModal, Preferences, About, Update.

`#ffffff` Elevated surface, 10px radius, `--shadow-elevated`, `20px` padding, 1px `#e5e5e5` border. Title in SF Pro Text 13px weight 600 ink (or DM Sans 300 for large About headings). Body SF Pro Text 12–13px. Buttons right-aligned: Cancel = HIG Push Button, OK = Filled Ink Primary. Backdrop `rgba(0,0,0,0.18)`. No `shadow-2xl`.

### Text Field
**Role:** RPM input, preset name, temperature thresholds.

Parchment `#fdfcfc` (or `#ffffff` inside elevated dialogs) background, 5px radius, 1px `#e5e5e5` border, `4px 8px` padding, SF Pro Text 12px. Numeric inputs use SF Mono tabular-nums, right-aligned. Focus → 2px ink ring. Error → 1px Ember border + Ember helper text below.

### Temperature Range Bar
**Role:** FanControlModal sensor-based config visualization.

Track: `#f5f3f1` Warm Sand, full width, 8px height, 4px radius. Range fill: `#000000` ink (monochrome — no green→yellow→red gradient). Live-current-temp marker: 12px ink circle with 2px parchment border, positioned along the track; turns **Ember `#ff4704`** only when the live reading is `≥ 85°C`. Labels in SF Mono 10px Driftwood.

### Safety / Warning Callout
**Role:** Extreme fan-setting warning, privilege/safety errors.

Warm Sand `#f5f3f1` background, 8px radius, 1px **Ember** border (the sanctioned safety use), SF Pro Text 11px ink text with a lucide `AlertTriangle` icon in Ember. No amber fill. This is the only place Ember is used as a border/fill accent.

---

## FanGuard Deviations from ElevenLabs Dogma

These deviations are intentional and documented, in service of matching the Apple ecosystem and the safety needs of a fan-control utility:

1. **Ember `#ff4704` as a safety signal** — ElevenLabs reserves color for decorative orbs only. FanGuard reuses Ember exclusively for hot-temperature (≥85°C) indication (status dots, range-bar marker, warning callout border). It is never used for buttons, links, or hover states.
2. **Decorative gradient orbs omitted** — The violet/orange ambient orbs are a voice-category marketing device; they have no place in a utility. Void Violet is retained as a token but unused in chrome.
3. **HIG rectangular-rounded geometry instead of 9999px pills** — Buttons, pop-ups, and dialogs use macOS-native radii (5–10px). Capsules (9999px) appear only for small HIG badge contexts.
4. **SF Pro Text + SF Mono instead of Inter + Geist Mono** — Apple's system fonts keep the app native to macOS. DM Sans weight 300 is retained for display headings to preserve the ElevenLabs "Waldenburg 300" headline personality.
5. **Edge-to-edge hairline panes instead of floating rounded cards** — Matches Activity Monitor / System Settings information density.
6. **Light-only** — The parchment system is a warm-light theme; dark mode is dropped for design coherence.

---

## Do

- Use HIG push-button geometry (7px) for window chrome and dialog actions; reserve Filled Ink for one primary action per dialog.
- Use SF Mono with `tabular-nums` for every numeric metric (RPM, °C) so columns align.
- Separate rows and panes with 1px `#e5e5e5` hairlines; let surface contrast (`#fdfcfc` / `#f5f3f1` / `#ffffff`) carry elevation.
- Use the HIG segmented control for the Auto/Custom fan-mode toggle — active segment filled ink, inactive outlined.
- Reserve Ember `#ff4704` strictly for hot (≥85°C) status dots, the range-bar live marker, and warning-callout borders.
- Use lucide outlined icons at consistent stroke width, monochrome (Driftwood or Ink) — never multicolor.
- Apply DM Sans 300 only to display-size headings (About title, large empty states); everything else is SF Pro Text / SF Mono.

## Don't

- Never use Ember `#ff4704` or Void Violet `#0447ff` for button backgrounds, link colors, hover states, or any interactive affordance.
- Never use green/yellow/red semantic status colors — temperature state is monochrome except the Ember hot signal.
- Never apply soft-blur shadows (blur > 4px or opacity > 0.04) — elevation is hairlines and surface contrast.
- Never use 9999px pill buttons as the default chrome — HIG rectangular-rounded geometry is the standard.
- Never use DM Sans for body text, labels, buttons, or data — SF Pro Text / SF Mono handle all functional text.
- Never add `dark:` variants or hardcoded dark hex values — the system is light-only.
- Never use emojis in chrome — replace `⚠` / `✓` / `✕` with lucide `AlertTriangle` / `Check` / `X`.
