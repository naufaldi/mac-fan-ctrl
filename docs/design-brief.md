# Design Brief: mac-fan-ctrl

## 1. Design Philosophy

### Core Principle
mac-fan-ctrl follows the **Professional Utility** design approach - information-dense monitoring tools that respect macOS conventions while providing pro-level visibility. The design aligns with utilities like iStat Menus and Bartender: distinctly third-party but native-feeling, data-first, and immediately scannable.

### Key Characteristics

| Characteristic | Implementation |
|---------------|----------------|
| **Data-first** | Numbers are the visual heroes; typography emphasizes readability |
| **At-a-glance** | Status conveyed through color coding without cognitive load |
| **Native respect** | System fonts (SF Pro, SF Mono), semantic colors, standard spacing |
| **Predictable** | Users understand the interface instantly; no learning curve |

### macOS Alignment
- Uses **SF Pro** for UI text and **SF Mono** for numeric displays
- Follows **8pt grid system** (4px, 8px, 16px, 24px, 32px)
- Adapts to **light/dark mode** automatically via system colors
- Respects **accessibility settings** (dynamic type, reduced motion)

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

```
┌─────────────────────────────────────────────┐
│  CPU Package        GPU         RAM        │  ← Hero Row (3 cols)
│   72°C ●            68°C ●      45°C ●     │
│  [sparkline]                                  │
├─────────────────────────────────────────────┤
│  Fans                                       │
│  Left: 2.4K ●     Right: 2.3K ●            │  ← Fan Section (2 cols)
│  [sparkline]      [sparkline]              │
├─────────────────────────────────────────────┤
│  Battery    SSD    HDD                     │  ← Other Sensors (3 cols)
│  35°C ●     42°C ●   N/A ●                 │
└─────────────────────────────────────────────┘
```

### 3.2 Hero Row
- **Sensors**: CPU Package, GPU, RAM (primary metrics)
- **Layout**: 3-column grid, equal width
- **Card Style**: Surface background, 16px padding, 8px radius
- **Content**: Label + status dot, large numeric value, optional sparkline

### 3.3 Fan Section
- **Layout**: 2-column grid
- **Content**: Fan name, RPM value with K notation (e.g., "2.4K"), 60-second sparkline
- **Expandable**: Section can collapse/expand

### 3.4 Secondary Sensors
- **Sensors**: Battery, SSD, HDD, additional thermals
- **Layout**: 3-column grid
- **Fallback**: "N/A" for unavailable sensors with gray status

### 3.5 Card Component Specification

```
┌─────────────────────────────┐
│ Label               ●       │  ← 16px padding
│                             │
│ 72°C                        │  ← 24px font, tabular nums
│                             │
│ ~～～～～～～～～            │  ← Sparkline (optional)
└─────────────────────────────┘
     8px radius
```

**Card Properties:**
- Background: `var(--color-surface-card)`
- Border-radius: `var(--radius-card)` (8px)
- Padding: `var(--spacing-4)` (16px)
- No shadow (flat design)
- Hover: `var(--color-surface-hover)`

### 3.6 Sparkline Specification

- **Height**: 24px
- **Width**: 60px (scales with container)
- **Stroke**: 2px, semantic color matching status
- **Fill**: None (stroke only for minimal visual weight)
- **Data Points**: 60 (one per second)
- **Smoothing**: Simple line, no curves (performance)
- **Animation**: None (static SVG path)

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
