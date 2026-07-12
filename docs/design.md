# FanGuard Design System - macOS 27 Liquid Glass

FanGuard follows a native macOS 27 utility direction based on the community Figma file:

- File: `macOS 27 (Community)`
- Link: https://www.figma.com/design/57pdOQpJuHYSqIFLpmL8kO/macOS-27--Community-?node-id=684-19522
- Verified node: `684:19522`, named `List`, size `1512 x 982`
- Relevant pages in the file: `Colors`, `Materials`, `Text Styles`, `Windows`, `Buttons`, `Lists and Tables`, `Sidebars`, `Toolbars`, `Segmented Controls`, `Search Fields`, `Sheets`, `Popovers`, `Toggles`

This document is the product-specific translation of that Figma system for FanGuard. Treat it as the design source for future frontend work, not as a generic macOS reference.

## Canonical V1 Screen

The canonical v1 screen is the generated FanGuard dashboard image selected in the design discussion. The Figma macOS 27 system provides the material language, but the generated image defines the actual FanGuard composition.

Implementation must match this screen before introducing alternate layouts:

- floating rounded macOS window over a blurred desktop background,
- traffic-light controls in the top-left,
- toolbar with `FanGuard`, centered `Preset: Automatic`, search field, and overflow menu,
- no sidebar and no back/forward navigation,
- one framed `FAN CONTROL` section above the table,
- one global `Auto / Manual` segmented control,
- `Manual` selected in the preview state,
- target slider with `1200`, `3200 RPM`, `6550`, and current average under the track,
- `Apply` primary button before `Restore Auto`,
- info row reading `Applies to all fans.`,
- framed dense table with columns `Name`, `Value`, `Range`, `Mode`, `Status`,
- group rows for `FANS`, `TEMPERATURE SENSORS`, and `DISK DRIVES`,
- preview data with `System Fan 1`, `System Fan 2`, `System Fan 3`, and `Synced` fan modes,
- preview disk rows `APPLE SSD AP0512Q` and `SanDisk Extreme Pro`,
- bottom status bar with `SMC Connected`, model id, `Fan control enabled`, and last update.

If this document and the generated image disagree, the generated image wins and this document should be corrected.

## Design Intent

FanGuard should feel like a first-party macOS utility for thermal monitoring and fan control. The interface is quiet, dense, and system-native. It should look closer to Finder, Activity Monitor, Console, or System Settings than to a web dashboard.

The Liquid Glass direction comes from layering, translucency, subtle material separation, rounded window geometry, and native control styling. It is not decorative glassmorphism. Blur and transparency should support hierarchy, not become the subject of the UI.

### Principles

- Prefer native macOS structure over custom web layout.
- Use Liquid Glass materials for window, toolbar, fan control, sheets, and popovers.
- Keep the main data area dense, scannable, and table-driven.
- Use system typography: SF Pro Text for UI, SF Mono for metrics.
- Use accent blue for active selection and primary confirmation only.
- Use semantic temperature colors only for thermal state.
- Avoid marketing-page spacing, decorative cards, custom pill controls, and large display typography.

## Source Observations

The verified Figma `List` frame shows:

- macOS menu bar above the window.
- rounded app window with large continuous corner radius.
- traffic-light controls in the top-left.
- translucent window and toolbar materials.
- unified toolbar with grouped icon controls and a native search field.
- main content table with dense rows, column headers, separators, group labels, active blue row selection, and a thin scrollbar.
- light and dark variants for many window examples.

The selected frame inspector exposed these relevant variables:

| Figma variable | Observed value | FanGuard role |
| --- | --- | --- |
| `Window Background` | `#FFF` at `100%` | Main content surface |
| `Labels/Primary` | `#000` at `85%` | Primary text |
| `Labels/Secondary` | `#000` at `50%` | Secondary text |
| `Labels - Vibrant (Use Plus Lighter | Darker)/Primary` | `#1A1A1A` at `100%` | Text on translucent material |
| `Labels - Vibrant (Use Plus Lighter | Darker)/Secondary` | `#727272` at `100%` | Secondary text on translucent material |
| `Labels - Vibrant (Use Plus Lighter | Darker)/Quaternary` | `#D9D9D9` at `100%` | Faint dividers and inactive affordances |
| `Accents - Vibrant/Blue` | `#0078F0` at `100%` | Accent and selected controls |
| `Miscellaneous/Tables/BG - Selected + Active` | `#0165E2` at `100%` | Active selected table row |
| `Miscellaneous/Tables/Alternating Row Color` | `#000` at `5%` | Alternating table row |
| `Miscellaneous/Tables/Column Separator` | `#000` at `10%` | Table column divider |
| `Miscellaneous/Scrollbar` | `#000` at `50%` | Scrollbar thumb |

## Tokens

### Typography

| Token | Value | Usage |
| --- | --- | --- |
| `--font-ui` | `-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif` | Labels, buttons, rows, toolbar, dialogs |
| `--font-mono` | `"SF Mono", SFMono-Regular, ui-monospace, Menlo, monospace` | Temperatures, RPM, timestamps, numeric columns |

Use `font-variant-numeric: tabular-nums` for all thermal values, RPM values, percentages, dates, and aligned metrics.

| Role | Size | Weight | Line height | Notes |
| --- | --- | --- | --- | --- |
| Table row label | `13px` | `400` | `18px` | Primary scanner text |
| Table value | `12px` | `400` | `18px` | Use mono and tabular nums |
| Table header | `11px` | `500` | `16px` | Muted, compact |
| Section label | `11px` | `600` | `16px` | Upper/lowercase follows Figma page conventions |
| Toolbar title | `13px` | `600` | `18px` | Window or active view title |
| Dialog title | `13px` | `600` | `18px` | Native sheet/dialog title |
| Helper text | `11px` | `400` | `16px` | Muted explanatory text |

### Color

Use semantic tokens instead of hard-coded colors in components.

```css
:root {
  color-scheme: light dark;

  --font-ui: -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
  --font-mono: "SF Mono", SFMono-Regular, ui-monospace, Menlo, monospace;

  --color-window-background: rgb(255 255 255);
  --color-label-primary: rgb(0 0 0 / 85%);
  --color-label-secondary: rgb(0 0 0 / 50%);
  --color-label-vibrant-primary: rgb(26 26 26);
  --color-label-vibrant-secondary: rgb(114 114 114);
  --color-label-quaternary: rgb(217 217 217);

  --color-accent-blue: rgb(0 120 240);
  --color-selection-active: rgb(1 101 226);
  --color-table-row-alternating: rgb(0 0 0 / 5%);
  --color-table-separator: rgb(0 0 0 / 10%);
  --color-scrollbar: rgb(0 0 0 / 50%);

  --color-status-normal: rgb(52 199 89);
  --color-status-warm: rgb(255 204 0);
  --color-status-hot: rgb(255 69 58);
  --color-status-unknown: rgb(142 142 147);
}
```

Dark mode should mirror the same token names with inverted label colors and darker materials. Do not fork component structure for dark mode; only token values should change.

### Materials

Liquid Glass is represented with layered materials:

| Token | Role | Implementation guidance |
| --- | --- | --- |
| `--material-window` | App window body | Mostly opaque surface with subtle translucency and system blur |
| `--material-toolbar` | Unified top toolbar | Translucent material over the window, with subtle bottom separator |
| `--material-control` | Buttons, search field, segmented controls | Small translucent surface with hairline border and inner highlight |
| `--material-sheet` | Dialogs and sheets | Floating material above window, stronger shadow and backdrop separation |
| `--material-popover` | Menus and compact overlays | Compact floating material with native corner radius and shadow |

For CSS, use `background-color`, `backdrop-filter: blur(...) saturate(...)`, and subtle inset borders. Avoid heavy shadows. The material should read as native glass, not as a blurred card.

### Spacing and Density

Use a 4px base unit. FanGuard is a dense utility, so most layout spacing should stay in the `4px`, `6px`, `8px`, `12px`, and `16px` range.

| Token | Value | Usage |
| --- | --- | --- |
| `--space-1` | `4px` | Icon gaps, separator offsets |
| `--space-1-5` | `6px` | Compact row internals |
| `--space-2` | `8px` | Toolbar and table cell padding |
| `--space-3` | `12px` | Toolbar padding, control groups |
| `--space-4` | `16px` | Window edge and dialog padding |
| `--space-6` | `24px` | Dialog section separation |

| Element | Target size |
| --- | --- |
| Table row height | `22px` to `26px` |
| Toolbar height | `52px` to `64px` depending on title layout |
| Compact icon button | `28px` square |
| Search field height | `30px` to `32px` |
| Segmented control height | `28px` to `30px` |

### Radius

| Element | Radius |
| --- | --- |
| App window | `22px` to `26px` |
| Sheets/dialogs | `18px` to `22px` |
| Popovers | `14px` to `18px` |
| Toolbar controls | `8px` to `12px` |
| Table selected rows | `6px` |
| Status dots | `50%` |

## Application Layout

FanGuard uses the generated single-window dashboard layout. It does not use a persistent sidebar because the core workflow is one screen: set global fan behavior, then scan grouped telemetry.

```text
+--------------------------------------------------------------+
| traffic lights   FanGuard        preset/search/actions       |
+--------------------------------------------------------------+
| FAN CONTROL: Auto / Manual, target RPM slider                |
+--------------------------------------------------------------+
| Name                  Value        Range        Mode Status   |
| Fans                                                        v |
| System Fan 1          2532 RPM     1200-6550    Synced OK     |
| System Fan 2          2200 RPM     1200-6550    Synced OK     |
| System Fan 3          2000 RPM     1200-6550    Synced OK     |
| Temperature Sensors                                      v   |
| CPU Core Average      56 C         40-100       -    Normal   |
| Disk Drives                                               v   |
| APPLE SSD AP0512Q      38 C        20-70        -    OK       |
| SanDisk Extreme Pro    36 C        20-70        -    OK       |
+--------------------------------------------------------------+
| SMC Connected   Mac model   Fan control state   Last update   |
+--------------------------------------------------------------+
```

### Window

- Use a single rounded macOS window container inside the app preview.
- Traffic-light controls belong in the upper-left of the generated window shell.
- Content should feel edge-to-edge inside the native window, with only the fan-control and table frames inset.
- Prefer a unified toolbar over a separate web-style header.
- Minimum window sizing should preserve the toolbar, global fan control, full table frame, status bar, and several rows.

### Navigation Model

FanGuard does not need a sidebar for the main dashboard. Fans, temperature sensors, and disk drives are visible as disclosure groups inside the main table because they are data categories, not separate screens.

Use a sidebar only if the product later gains real multi-screen navigation such as logs, alerts, history, or advanced rules. Do not add navigation solely to mirror the Figma reference.

### Toolbar

The toolbar is unified with the window title area and must follow the generated image:

- traffic lights at far left,
- `FanGuard` title next to them,
- centered preset pop-up labeled `Preset: Automatic`,
- search field near the right edge,
- one compact overflow menu at far right.

Do not include fake back/forward navigation or toolbar icons without real behavior. Text labels are acceptable where macOS would normally show them, such as primary confirmation in a sheet.

### Main Content

The main content area is table-first:

- fan rows,
- temperature sensor rows,
- disk rows,
- derived or unavailable rows when useful.

Use native list/table styling:

- compact row heights,
- optional alternating row fill,
- muted column headers,
- subtle column separators,
- active row selection in macOS blue,
- right-aligned numeric values,
- sticky header when the table scrolls,
- thin scrollbar.

Do not use repeated floating cards for sensors. FanGuard is a monitoring tool; the user should scan rows quickly.

## Components

### Tables and Lists

Use this table model for all telemetry lists:

| Column | Width | Notes |
| --- | --- | --- |
| Icon | `24px` | SF Symbol-style, muted unless status needs emphasis |
| Name | flexible | Primary sensor/fan label |
| Value | `88px` to `120px` | Right-aligned mono text |
| Detail | `120px` to `180px` | RPM range, source, mode, or last update |
| Status | `72px` to `96px` | Text or compact dot plus label |

Rules:

- values use `--font-mono` and tabular numbers,
- unavailable values render as muted `N/A`,
- selected row uses `--color-selection-active` with white text,
- group headers use muted 11px text and should not look like cards,
- column separators use `--color-table-separator`.

### Fan Controls

Fan control is global in FanGuard v1. One control applies to every detected fan together. The fan-control section must visually match the generated image: uppercase label, framed glass panel, mode row, target slider row, action buttons to the right, and info copy at the bottom.

Fan control mode must use a native segmented control:

- `Auto`
- `Manual`

The selected segment uses accent blue or native selected material depending on context. Manual mode exposes one inline target RPM slider because the dashboard's primary job is fan control.

Global manual control rules:

- individual fan rows are telemetry only,
- no per-fan sliders or custom modal,
- the preview state is `Manual` with synced fan rows,
- the shared safe range is `max(fan.min)` to `min(fan.max)`,
- if the detected fans have no shared safe range, disable manual writes and explain why,
- applying manual mode writes the same target RPM to every detected fan,
- restoring auto restores every detected fan to automatic mode.

RPM controls:

- validate min and max boundaries before writes,
- show minimum, maximum, requested target, and current average,
- place the requested target above or visually tied to the slider thumb,
- always show numeric RPM beside visual controls.

### Preset Control

The active preset belongs in the toolbar. Use a native pop-up or pull-down button style. Preset choices should feel like system menu items, not large tiles.

Expected options:

- Automatic
- Quiet
- Balanced
- Performance
- Custom presets when implemented

### Search Field

Use a native rounded search field with search icon, compact height, and placeholder text. Search filters visible rows only; it should not change polling or backend state.

### Buttons

Buttons should match macOS controls:

- push buttons for dialog actions,
- toolbar icon buttons for compact commands,
- bordered or material-backed controls for secondary actions,
- destructive actions use system red only in confirmation contexts.

Avoid custom pill styling. Radius should come from the control size and native material, not from a brand rule.

### Sheets and Dialogs

Use sheets for workflows attached to the current window:

- preferences,
- update confirmation,
- helper access explanation.

Use dialogs for app-level information:

- About FanGuard,
- critical safety warnings.

Sheets should use `--material-sheet`, centered or attached to the parent window depending on implementation constraints in Tauri. Keep text short and controls native.

### Popovers and Menus

Use popovers for compact contextual controls:

- preset menu,
- sensor source details,
- menu bar display mode,
- row actions.

Popover content should be narrow, dense, and keyboard reachable.

### Temperature Status

Temperature status is the main allowed semantic color usage:

| Status | Meaning | Color |
| --- | --- | --- |
| Normal | below warning threshold | `--color-status-normal` |
| Warm | approaching warning threshold | `--color-status-warm` |
| Hot | above safe threshold | `--color-status-hot` |
| Unknown | unavailable or unsupported | `--color-status-unknown` |

Use a small dot plus optional label. Do not flood table rows with colored backgrounds.

### Menu Bar

The menu bar icon remains a macOS template asset:

- black source with alpha,
- automatically adapts to light and dark menu bars,
- supports compact telemetry text when enabled.

Menu bar display modes:

- fan RPM,
- primary temperature,
- icon only.

Menu bar popover should reuse the same Liquid Glass material language as app popovers.

## Implementation Notes

### Svelte and Tailwind

- Keep styling in Tailwind v4 utilities backed by CSS custom properties.
- Use the shared `cn()` helper for dynamic classes.
- Use Svelte 5 runes for state.
- Prefer native-looking controls over hand-built decorative widgets.
- Keep component boundaries aligned with product surfaces: `Window`, `Toolbar`, `FanControl`, `DataTable`, `Sheet`, `Popover`.

### Tauri Window

The Tauri window should support the native design direction:

- transparent or translucent window where supported,
- rounded corners,
- hidden or integrated titlebar if the implementation can preserve drag regions and accessibility,
- stable minimum dimensions for dense table scanning.

Do not sacrifice accessibility or window behavior just to simulate glass. Native window semantics win when there is a tradeoff.

### Accessibility

- All controls need keyboard focus states.
- Table rows need readable selected, hover, and inactive states.
- Color cannot be the only indicator for temperature state.
- Search and filter controls need accessible names.
- Dialogs and sheets need focus trapping and escape/close behavior.

### Performance

Liquid Glass effects can be expensive in a webview. Apply blur to major surfaces only:

- window,
- toolbar,
- sheets,
- popovers.

Do not apply backdrop blur to every row or cell. Data tables should stay visually crisp and cheap to render.

## Do and Do Not

### Do

- Use SF Pro Text and SF Mono.
- Use Liquid Glass materials for app-level surfaces.
- Keep telemetry rows compact and table-based.
- Use macOS blue for active selection.
- Keep temperature colors small and semantic.
- Preserve native controls, keyboard behavior, and system expectations.

### Do Not

- Build a marketing dashboard.
- Use large hero typography.
- Put every sensor in a separate decorative card.
- Invent brand-specific button shapes.
- Use decorative gradients as app surfaces.
- Hide important fan or temperature data behind oversized controls.
- Use blur effects inside dense tables.

## Verification Checklist

Use this checklist when implementing UI from this design document:

- The app reads as a macOS utility at first glance.
- The rendered dashboard visually matches the canonical generated image before alternate layout changes.
- The toolbar, fan control, sheets, and popovers use Liquid Glass materials.
- Main telemetry uses dense list/table structure.
- Numeric columns align with SF Mono tabular numbers.
- Active selection uses macOS blue.
- Temperature status uses small semantic indicators.
- All controls remain keyboard accessible.
- Dark mode changes tokens, not component structure.
- Performance remains smooth during sensor polling.
