# Icon Assets

## Menu Bar Icon

The menu bar icon is a **template icon** (pure black with transparency) that macOS automatically inverts for dark mode.

### Files

| File | Size | Description |
|------|------|-------------|
| `menu-icon-template.png` | 16x16px @1x | Standard menu bar icon |
| `menu-icon-template@2x.png` | 32x32px @2x | Retina display icon |
| `menu-icon-source.svg` | Vector | Source for regeneration |

### Design Spec

- **Format:** PNG with alpha transparency
- **Color:** Pure black (#000000)
- **Style:** 3-blade fan silhouette
- **Padding:** 1px on all sides

### Regenerating

Using macOS `sips`:
```bash
cd src-tauri/icons
sips -s format png menu-icon-source.svg --out menu-icon-template.png -z 16 16
sips -s format png menu-icon-source.svg --out menu-icon-template@2x.png -z 32 32
```

## App Icon

The main app icon (`icon.png`) is a 1024x1024px PNG that Tauri uses to generate the ICNS file for the application bundle.
