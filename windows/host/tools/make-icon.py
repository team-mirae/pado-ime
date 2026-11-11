# Writes icons/bada-windows.ico: a sea-blue rounded square with a white wave, 16/32/48 px, no
# dependencies. The DLL's icon (index 0), used for the language profile unless the manifest names one.
import math, os, struct

def pixels(n):
    rows = []
    r = n * 0.22
    for y in range(n):
        row = []
        for x in range(n):
            cx, cy = min(max(x + 0.5, r), n - r), min(max(y + 0.5, r), n - r)
            inside = math.hypot(x + 0.5 - cx, y + 0.5 - cy) <= r
            if not inside:
                row.append((0, 0, 0, 0)); continue
            wave = n * 0.55 + math.sin((x + 0.5) / n * 2 * math.pi) * n * 0.1
            if abs((y + 0.5) - wave) < max(1.0, n * 0.07):
                row.append((255, 255, 255, 255))
            else:
                row.append((0x1E, 0x6F, 0xD9, 255))  # RGB
        rows.append(row)
    return rows

def image(n):
    data = b''
    for row in reversed(pixels(n)):  # bottom-up
        for (r, g, b, a) in row:
            data += bytes((b, g, r, a))
    mask = b'\x00' * (((n + 31) // 32) * 4 * n)
    header = struct.pack('<IiiHHIIiiII', 40, n, 2 * n, 1, 32, 0, len(data) + len(mask), 0, 0, 0, 0)
    return header + data + mask

sizes = [16, 32, 48]
images = [image(n) for n in sizes]
out = struct.pack('<HHH', 0, 1, len(sizes))
offset = 6 + 16 * len(sizes)
for n, img in zip(sizes, images):
    out += struct.pack('<BBBBHHII', n, n, 0, 0, 1, 32, len(img), offset)
    offset += len(img)
out += b''.join(images)
path = os.path.join(os.path.dirname(__file__), '..', 'icons', 'bada-windows.ico')
open(path, 'wb').write(out)
print(path, len(out))
