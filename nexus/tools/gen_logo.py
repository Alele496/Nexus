#!/usr/bin/env python3
"""Generate final Nexus braille art logos. Saves to assets/logo/ directory."""
import sys, os
sys.stdout.reconfigure(encoding='utf-8')

def make_braille(dots_2x4):
    code = 0x2800
    weights = [1, 2, 4, 64, 8, 16, 32, 128]
    for i, on in enumerate(dots_2x4):
        if on:
            code += weights[i]
    return chr(code)

def grid_to_text(grid):
    h, w = len(grid), len(grid[0])
    lines = []
    for row_start in range(0, h, 4):
        line = []
        for col_start in range(0, w, 2):
            dots = []
            for r in range(4):
                y, x = row_start + r, col_start
                dots.append(grid[y][x] if y < h and x < w else False)
            for r in range(4):
                y, x = row_start + r, col_start + 1
                dots.append(grid[y][x] if y < h and x < w else False)
            line.append(make_braille(dots))
        lines.append(''.join(line))
    return '\n'.join(lines) + '\n'

def parse_rows(design):
    """Parse a list of strings into a bool grid, padding as needed."""
    rows = [[c in '#X1' for c in row.strip()] for row in design]
    w = max(len(r) for r in rows) if rows else 0
    for r in rows:
        while len(r) < w:
            r.append(False)
    while len(rows) % 4:
        rows.append([False] * w)
    if w % 2:
        for r in rows:
            r.append(False)
        w += 1
    return rows

def show(name, design):
    grid = parse_rows(design)
    print(f'\n=== {name} ({len(grid[0])}x{len(grid)}) ===')
    for row in grid:
        print(''.join('#' if c else ' ' for c in row))
    print()
    text = grid_to_text(grid)
    for l in text.strip().split('\n'):
        print(l)
    return text

assets = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..',
                      'crates', 'codegen', 'nexus-pager', 'assets', 'logo'))
os.makedirs(assets, exist_ok=True)
print(f'Assets dir resolved to: {assets}')

# ================================================================
# FINAL DESIGNS
# ================================================================

# logo05.txt (small, 5 lines): Central diamond hub with radiating arms
# Width: 14 cols = 7 braille chars wide
logo05 = show('logo05 (small)', [
    "......##......",
    ".....####.....",
    "....######....",
    "...##....##...",
    "..##..##..##..",
    "..##.####..##.",
    ".##..####..##.",
    "##....##....##",
    ".##..####..##.",
    "..##.####..##.",
    "..##..##..##..",
    "...##....##...",
    "....######....",
    ".....####.....",
    "......##......",
    "......##......",
    ".....####.....",
    "....######....",
    "...##....##...",
    "..##......##..",
])

# logo07.txt (full, 7 lines): Diamond hub with radiating connections
# Width: 20 cols = 10 braille chars wide
logo07 = show('logo07 (full)', [
    ".........##.........",
    "........####........",
    ".......######.......",
    "......##....##......",
    ".....##......##.....",
    "....##........##....",
    "...##..........##...",
    "...##...####...##...",
    "..##...######...##..",
    "..##..########..##..",
    ".##....######....##.",
    "##......####......##",
    ".##....######....##.",
    "..##..########..##..",
    "..##...######...##..",
    "...##...####...##...",
    "...##..........##...",
    "....##........##....",
    ".....##......##.....",
    "......##....##......",
    ".......######.......",
    "........####........",
    ".........##.........",
    ".........##.........",
    "........####........",
    ".......######.......",
    "......##....##......",
    ".....##......##.....",
])

# Write the files
for name, text in [('logo05.txt', logo05), ('logo07.txt', logo07)]:
    path = os.path.join(assets, name)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(text)
    with open(path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    print(f'\nSaved {path} ({len(lines)} lines)')

# Also generate all other sizes for completeness (scaled versions)
all_sizes = {
    'logo04.txt': 10,   # 4 braille lines
    'logo06.txt': 16,   # 6 braille lines
    'logo08.txt': 22,   # 8 braille lines
    'logo09.txt': 24,   # 9 braille lines
    'logo10.txt': 26,   # 10 braille lines
    'logo12.txt': 30,   # 12 braille lines
    'logo16.txt': 36,   # 16 braille lines
    'logo20.txt': 42,   # 20 braille lines
    'logo24.txt': 48,   # 24 braille lines
}

def generate_scaled(width, braille_lines, name):
    """Generate a diamond pattern at given width and braille line count."""
    h = braille_lines * 4
    w = width
    grid = [[False]*w for _ in range(h)]
    cx, cy = w/2, h/2

    # Draw concentric diamond shapes
    for y in range(h):
        for x in range(w):
            # Distance from center in diamond metric
            dx = abs(x - cx) / (w/2)
            dy = abs(y - cy) / (h/2)
            d = dx + dy

            # Outer diamond shell
            if 0.75 < d < 0.92:
                grid[y][x] = True
            # Inner diamond (hub)
            elif 0.15 < d < 0.35:
                grid[y][x] = True
            # Radial connecting lines (every ~60 degrees)
            angle = (x - cx) / max(w/2, 1)
            radial = abs(angle * 0.5 + (y - cy) / max(h/2, 1) * 0.5)
            if 0.40 < d < 0.80 and abs(radial) < 0.08:
                grid[y][x] = True
            if 0.40 < d < 0.80 and abs(radial - 0.5) < 0.08:
                grid[y][x] = True

    print(f'\n=== {name} ({w}x{h}) ===')
    for row in grid:
        print(''.join('#' if c else ' ' for c in row))
    print()
    text = grid_to_text(grid)
    for l in text.strip().split('\n'):
        print(l)
    return text

for fname, width in all_sizes.items():
    lines = int(fname.replace('logo', '').replace('.txt', ''))
    text = generate_scaled(width, lines, fname)
    path = os.path.join(assets, fname)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(text)

print(f'\nAll logos saved to {assets}/')
