#!/usr/bin/env python3
"""Generate nexus.ico — a simple "N" icon for Nexus.
Uses the Python Imaging Library (PIL/Pillow) to create a multi-resolution .ico file.
Install: pip install Pillow
"""

import struct
import os

def create_ico_png():
    """Create a 256x256 PNG icon and embed it as a BMP-masked .ico."""
    from PIL import Image, ImageDraw, ImageFont

    sizes = [16, 24, 32, 48, 64, 128, 256]
    images = []

    # Nexus brand colors
    BG_COLOR = (88, 86, 214)    # Purple/Indigo
    FG_COLOR = (255, 255, 255)  # White text

    for size in sizes:
        img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)

        # Rounded rect background (simple circle approximation for small sizes)
        margin = max(1, size // 10)
        draw.rounded_rectangle(
            [margin, margin, size - margin, size - margin],
            radius=size // 5,
            fill=BG_COLOR
        )

        # Draw "N" letter
        try:
            # Try to use system font, fall back to default
            font_size = int(size * 0.55)
            try:
                font = ImageFont.truetype("segoeui.ttf", font_size)
            except (IOError, OSError):
                try:
                    font = ImageFont.truetype("arial.ttf", font_size)
                except (IOError, OSError):
                    try:
                        font = ImageFont.truetype("C:\\Windows\\Fonts\\segoeui.ttf", font_size)
                    except (IOError, OSError):
                        font = ImageFont.load_default()
        except Exception:
            font = ImageFont.load_default()

        # Center the "N" text
        bbox = draw.textbbox((0, 0), "N", font=font)
        text_w = bbox[2] - bbox[0]
        text_h = bbox[3] - bbox[1]
        x = (size - text_w) // 2
        y = (size - text_h) // 2 - size // 20

        draw.text((x, y), "N", fill=FG_COLOR, font=font)
        images.append(img)

    # Save as .ico
    script_dir = os.path.dirname(os.path.abspath(__file__))
    ico_path = os.path.join(script_dir, "nexus.ico")
    images[0].save(
        ico_path,
        format='ICO',
        sizes=[(s, s) for s in sizes],
        append_images=images[1:]
    )
    print(f"Generated: {ico_path}")

if __name__ == "__main__":
    try:
        from PIL import Image
        create_ico_png()
    except ImportError:
        print("Pillow not installed. Install with: pip install Pillow")
        print("Generating minimal .ico fallback...")
        # Fallback: create minimal valid .ico
        create_minimal_ico()

def create_minimal_ico():
    """Minimal valid .ico file without PIL dependency."""
    # A 32x32 icon: simple BMP data with purple background
    width, height = 32, 32
    # Create XOR mask (BGRA pixels) + AND mask (1bpp)
    pixels = []
    for y in range(height):
        for x in range(width):
            # Purple background with white "N" shape
            is_letter = False
            # Simple "N" shape (two vertical lines + diagonal)
            if 4 <= x <= 8 or 23 <= x <= 27:  # vertical bars
                if 6 <= y <= 25:
                    is_letter = True
            if 8 <= x <= 23 and 6 <= y <= 25:  # diagonal area
                diag_pos = (x - 8) * 20 // 16
                if abs(y - 6 - diag_pos) <= 2:
                    is_letter = True

            if is_letter:
                pixels.append((255, 255, 255, 255))  # White
            else:
                pixels.append((88, 86, 214, 255))    # Purple

    # Build ICO header
    script_dir = os.path.dirname(os.path.abspath(__file__))
    ico_path = os.path.join(script_dir, "nexus.ico")

    with open(ico_path, 'wb') as f:
        # ICO header
        f.write(struct.pack('<HHH', 0, 1, 1))  # Reserved, type=1(ICO), count=1

        # BMP data for 32x32 32bpp
        bmp_size = 40 + width * height * 4  # BITMAPINFOHEADER + pixels
        and_size = ((width + 31) // 32 * 4) * height
        total_size = 40 + width * height * 4 + and_size

        # ICO directory entry
        f.write(struct.pack('<BBBBHHIH',
            32, 32,           # width, height
            0,                # no palette
            0,                # reserved
            1, 32,            # color planes, bpp
            total_size,       # size of image data
            22                # offset to image data (6 + 16)
        ))

        # BITMAPINFOHEADER
        f.write(struct.pack('<IiiHHIIiiII',
            40,               # header size
            width, height * 2, # width, height (doubled for ICO)
            1,                # planes
            32,               # bpp
            0,                # BI_RGB
            width * height * 4, # image size
            0, 0, 0, 0        # unused
        ))

        # Pixel data (bottom-up for BMP)
        for y in range(height - 1, -1, -1):
            for x in range(width):
                b, g, r, a = pixels[y * width + x]
                f.write(struct.pack('BBBB', b, g, r, a))

        # AND mask (all transparent)
        f.write(b'\x00' * and_size)

    print(f"Generated (minimal): {ico_path}")
