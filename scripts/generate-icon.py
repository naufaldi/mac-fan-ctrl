#!/usr/bin/env python3
"""Generate menu bar icon for mac-fan-ctrl"""

try:
    from PIL import Image, ImageDraw
    import math

    # Create 16x16 icon
    img = Image.new('RGBA', (16, 16), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    cx, cy = 8, 8

    # Draw center hub
    draw.ellipse([cx-2, cy-2, cx+2, cy+2], fill=(0, 0, 0, 255))

    # Draw 3 blades at 0, 120, 240 degrees
    for angle in [0, 120, 240]:
        rad = math.radians(angle)
        # Start from edge of hub
        x1 = cx + 2.5 * math.cos(rad)
        y1 = cy + 2.5 * math.sin(rad)
        # Extend outward
        x2 = cx + 6.5 * math.cos(rad)
        y2 = cy + 6.5 * math.sin(rad)

        # Draw blade as small ellipse
        bx = (x1 + x2) / 2
        by = (y1 + y2) / 2
        draw.ellipse([bx-1.5, by-1.5, bx+1.5, by+1.5], fill=(0, 0, 0, 255))

    # Save 16x16
    img.save('/Users/mac/WebApps/oss/mac-fan-ctrl/src-tauri/icons/menu-icon-template.png')
    print('Created menu-icon-template.png (16x16)')

    # Create 32x32 @2x version
    img2x = Image.new('RGBA', (32, 32), (0, 0, 0, 0))
    draw2x = ImageDraw.Draw(img2x)

    cx, cy = 16, 16

    # Draw center hub (scaled)
    draw2x.ellipse([cx-4, cy-4, cx+4, cy+4], fill=(0, 0, 0, 255))

    # Draw 3 blades
    for angle in [0, 120, 240]:
        rad = math.radians(angle)
        x1 = cx + 5 * math.cos(rad)
        y1 = cy + 5 * math.sin(rad)
        x2 = cx + 13 * math.cos(rad)
        y2 = cy + 13 * math.sin(rad)

        bx = (x1 + x2) / 2
        by = (y1 + y2) / 2
        draw2x.ellipse([bx-3, by-3, bx+3, by+3], fill=(0, 0, 0, 255))

    img2x.save('/Users/mac/WebApps/oss/mac-fan-ctrl/src-tauri/icons/menu-icon-template@2x.png')
    print('Created menu-icon-template@2x.png (32x32)')

except ImportError:
    print("PIL/Pillow not available. Please install: pip install Pillow")
    exit(1)
