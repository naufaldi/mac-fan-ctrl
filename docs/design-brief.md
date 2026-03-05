# Design Brief: mac-fan-ctrl

## 1. Design Philosophy

### Core Principle
mac-fan-ctrl follows the **Strict Native macOS Utility** design approach. The UI must look and feel indistinguishable from a first-party macOS system utility (like Activity Monitor) or a high-quality native app (like Macs Fan Control). We avoid web-centric "card" designs, floating rounded boxes, or loose spacing in favor of edge-to-edge tables, standard system controls, and high data density.

### Key Characteristics

| Characteristic | Implementation |
|---------------|----------------|
| **Native Replication** | UI components must perfectly mimic macOS native controls (segmented controls, dropdowns, table headers). |
| **Data-first Density** | Small, crisp typography (11px/12px) with tight spacing to maximize visible information. |
| **Zebra Striping** | Tables and lists must use alternating row background colors (`odd:` / `even:`) for readability. |
| **System Alignment** | Strict use of system fonts (SF Pro, SF Mono), semantic system colors, and native border styles. |

### macOS Alignment
- Uses **SF Pro Text** for UI text (11px-13px) and **SF Mono** for numeric displays.
- Follows standard **macOS table layouts** (gray headers with vertical dividers, top/bottom borders).
- Adapts to **light/dark mode** automatically via system colors, matching Apple's exact HIG hex values.
- Uses **SF Symbols** for iconography (e.g., wifi, battery, cpu) instead of custom SVGs where possible.

---

## 2. Visual Identity System

### 2.1 Iconography

#### Menu Bar Icon
- **Format**: PNG template (black with transparency)
- **Sizes**: 16x16px @1x, 32x32px @2x (Retina)
- **Style**: 3-blade fan silhouette, 45-degree rotation
- **Geometry**: Rounded blade tips, visible counterweight, 1px padding
- **Location**: `src-tauri/icons/menu-icon-template.png`

#### App Icon (ICNS)
- **Format**: PNG source, converted to ICNS
- **Size**: 1024x1024px source
- **Style**: macOS Big Sur rounded rectangle (22% corner radius)
- **Safe Area**: 20% padding from edges
- **Same geometry** as menu bar icon, centered

### 2.2 Typography

| Purpose | Font | Usage |
|---------|------|-------|
| UI Labels | SF Pro Text | Sensor labels, section headers |
| Numeric Values | SF Mono | Temperatures, RPM, all metrics |
| Tabular Figures | `font-variant-numeric: tabular-nums` | Aligned number columns |

**CSS Declaration:**
```css
--font-ui: -apple-system, BlinkMacSystemFont, "SF Pro Text", sans-serif;
--font-mono: "SF Mono", SFMono-Regular, Monaco, monospace;
```

### 2.3 Color System

#### Status Colors (Semantic)
Temperature states use system colors for automatic light/dark adaptation:

| Status | Temperature | CSS Variable | Light Mode | Dark Mode |
|--------|-------------|--------------|------------|-----------|
| Normal | < 70°C | `--color-status-normal` | System Green | System Green |
| Warm | 70-85°C | `--color-status-warm` | System Yellow | System Yellow |
| Hot | > 85°C | `--color-status-hot` | System Red | System Red |
| Unknown | N/A | `--color-status-unknown` | System Gray | System Gray |

**Technical Values:**
```css
--color-status-normal: oklch(0.7 0.2 145);
--color-status-warm: oklch(0.8 0.15 85);
--color-status-hot: oklch(0.6 0.25 25);
--color-status-unknown: oklch(0.6 0 0);
```

#### Surface Colors
| Purpose | CSS Variable | Value |
|---------|--------------|-------|
| Card Background | `--color-surface-card` | rgba(120, 120, 128, 0.08) |
| Hover State | `--color-surface-hover` | rgba(120, 120, 128, 0.12) |

### 2.4 Spacing Scale (8pt Grid)

| Token | Value | Usage |
|-------|-------|-------|
| `--spacing-1` | 4px | Micro gaps, tight padding |
| `--spacing-2` | 8px | Tight padding, icon gaps |
| `--spacing-3` | 12px | Small component padding |
| `--spacing-4` | 16px | Standard padding, card gaps |
| `--spacing-6` | 24px | Section spacing |
| `--spacing-8` | 32px | Major section breaks |

### 2.5 Border Radius

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-card` | 8px | Card corners |
| `--radius-dot` | 50% | Status indicator circles |

---

## 3. Dashboard Layout

### 3.1 Layout Structure

The application uses a classic macOS split-pane window layout, avoiding floating cards in favor of edge-to-edge content.

```
┌───────────────────────────────────────────────────────────┐
│  Active preset: [ Automatic ▼ ]                [ ... ]    │  ← Header (Control Bar)
├────────────────────────────────┬──────────────────────────┤
│  Fan      Min/Current/Max RPM  │  Sensor         Value °C │  ← Native Table Headers
├────────────────────────────────┼──────────────────────────┤
│ ❖ Left    1200 - 2329 - 5779   │  [icon] Wi-Fi      49    │  ← Left: Fans (Flexible width)
│  [Auto] [Custom...]            │  [icon] Battery    32    │  ← Right: Sensors (Fixed width)
│                                │  [icon] CPU Core   69    │  ← Zebra striped rows
│ ❖ Right   1200 - 2499 - 6241   │  [icon] GPU        68    │
│  [Auto] [Custom...]            │                          │
├────────────────────────────────┴──────────────────────────┤
│  [Hide to menu bar]   [Preferences...]                [?] │  ← Footer
└───────────────────────────────────────────────────────────┘
```

### 3.2 Header & Footer
- **Header**: Contains the "Active preset" dropdown and standard window controls. Subtle bottom border.
- **Footer**: Contains utility actions ("Hide to menu bar", "Preferences...", "Help"). Subtle top border.
- **Buttons**: Must mimic native macOS push buttons (rounded rectangles with specific gradients/shadows or flat native styles depending on OS version).

### 3.3 Fan Control Pane (Left)
- **Layout**: Flexible width (`1fr`).
- **Table Structure**: Edge-to-edge list.
- **Columns**: "Fan", "Min/Current/Max RPM", "Control".
- **Controls**: Must use native-looking Segmented Controls for the "Auto" / "Custom..." toggles. Do not use standalone rounded buttons.
- **Row Styling**: Alternating background colors (zebra striping) or clear dividers.

### 3.4 Sensor List Pane (Right)
- **Layout**: Fixed width (e.g., `300px`). Separated from the left pane by a standard 1px vertical divider.
- **Table Structure**: Edge-to-edge list, scrollable.
- **Columns**: "Sensor", "Value °C".
- **Icons**: Use SF Symbols to represent sensor types (e.g., Wi-Fi, Battery, CPU).
- **Row Styling**: Alternating background colors (zebra striping). No "Read More" expandable sections; just a standard scrollable list.

### 3.5 Component Specifications

#### Table Headers
- Background: Native macOS table header gray.
- Typography: 11px SF Pro Text, regular weight, dark gray text.
- Borders: 1px bottom border, 1px vertical dividers between columns.

#### Segmented Controls (Fan Controls)
- Must perfectly mimic macOS segmented controls.
- Connected buttons with a shared border.
- Selected state uses native accent color or native selected gray.
- Unselected state has a transparent/subtle background.

#### Sensor Icons
- Size: 14px - 16px.
- Color: Semantic colors (e.g., blue for Wi-Fi, green for CPU) or standard macOS monochrome, matching native utility conventions.

---

## 4. Menu Bar Integration

### 4.1 Display Modes

User-toggleable modes for menu bar display:

#### Mode A: Fan RPM (Default)
```
[fan-icon] 2.4K
```
- Format: K notation above 1000 (2.4K = 2400)
- Updates: Every 2-3 seconds (window closed)
- Tooltip: "Left: 2450 RPM | Right: 2300 RPM"

#### Mode B: Temperature
```
[fan-icon] 72°C
```
- Shows primary sensor (CPU Package by default)
- Updates: Every 2-3 seconds
- Tooltip: "CPU: 72°C | GPU: 68°C | RAM: 45°C"

#### Mode C: Icon Only
```
[fan-icon]
```
- Minimal mode for users who prefer clean menu bar
- Click reveals dropdown with full status

### 4.2 Behavior Specification

| Action | Response |
|--------|----------|
| **Click** | Open main application window |
| **Right-click** | Context menu: Mode toggle, Preferences, Quit |
| **Hover** | Tooltip with expanded status |
| **Polling (closed)** | 2-3 second interval |
| **Polling (open)** | 1 second interval |

### 4.3 Menu Bar Icon States

| State | Appearance |
|-------|------------|
| Normal | Standard template icon (black) |
| Warning | Yellow tint when any sensor is "warm" |
| Critical | Red tint when any sensor is "hot" |

---

## 5. Icon Asset Specifications

### 5.1 Menu Bar Icon Template

**File:** `src-tauri/icons/menu-icon-template.png`

**Requirements:**
- Format: PNG with alpha transparency
- Size: 16x16 pixels @1x
- Color: Pure black (#000000)
- macOS automatically inverts for dark mode
- Geometry: 3-blade fan, 45° rotation, rounded tips

**Retina:**
- File: `src-tauri/icons/menu-icon-template@2x.png`
- Size: 32x32 pixels @2x

### 5.2 App Icon ICNS

**Source File:** `src-tauri/icons/icon.png`

**Requirements:**
- Format: PNG
- Size: 1024x1024 pixels
- Style: macOS Big Sur rounded rectangle
- Corner radius: 22% (180px for 1024px canvas)
- Safe area: 20% padding (184px margin)
- Same fan geometry as menu bar icon, scaled up
- Background: Subtle gradient or solid fill (neutral gray)

**Generated Assets:**
Tauri will generate ICNS from source, but source must be high quality.

### 5.3 Icon Design Principles

1. **Template Style**: Menu bar icons are pure black; macOS handles color
2. **Clarity at 16px**: Simple geometry, no fine details
3. **Recognizable Shape**: Fan silhouette distinct from other menu bar icons
4. **Consistency**: App icon and menu bar icon share same geometry

---

## 6. Accessibility Considerations

### 6.1 Color Independence
- Status conveyed through both color AND position (hot items may appear first)
- Text labels always visible, not just color coding
- Tooltips provide full context

### 6.2 Dynamic Type Support
- All text scales with system font size preferences
- Layouts use relative units (rem, not px)
- Minimum touch targets: 44x44pt

### 6.3 Reduced Motion
- Sparklines are static SVG, no animation
- No pulsing or flashing indicators
- Status changes are instant, not animated

---

## 7. Design Token Reference

### CSS Variables Summary

```css
/* Status Colors */
--color-status-normal: oklch(0.7 0.2 145);
--color-status-warm: oklch(0.8 0.15 85);
--color-status-hot: oklch(0.6 0.25 25);
--color-status-unknown: oklch(0.6 0 0);

/* Surface Colors */
--color-surface-card: rgba(120, 120, 128, 0.08);
--color-surface-hover: rgba(120, 120, 128, 0.12);

/* Spacing */
--spacing-1: 4px;
--spacing-2: 8px;
--spacing-4: 16px;
--spacing-6: 24px;
--spacing-8: 32px;

/* Typography */
--font-mono: "SF Mono", SFMono-Regular, Monaco, monospace;

/* Border Radius */
--radius-card: 8px;
```

---

*Document Version: 1.0*
*Last Updated: 2026-03-04*
