/**
 * Colour conversion for the picker.
 *
 * Kept dependency-free and total: every function either returns a value or
 * `null`, never a half-parsed colour. A form that renders a broken swatch is
 * worse than one that renders none, because the user cannot tell the value is
 * wrong until it reaches whatever consumes it.
 */

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export interface Hsv {
  h: number;
  s: number;
  v: number;
}

const HEX_PATTERN = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i;

/** Parse `#rgb`, `#rrggbb` or either without the hash. */
export function parseHex(input: string): Rgb | null {
  const match = HEX_PATTERN.exec(input.trim());
  if (!match) return null;

  let body = match[1].toLowerCase();
  if (body.length === 3) {
    body = body
      .split("")
      .map((char) => char + char)
      .join("");
  }
  return {
    r: parseInt(body.slice(0, 2), 16),
    g: parseInt(body.slice(2, 4), 16),
    b: parseInt(body.slice(4, 6), 16),
  };
}

export function toHex({ r, g, b }: Rgb): string {
  const channel = (value: number) =>
    Math.round(Math.min(255, Math.max(0, value)))
      .toString(16)
      .padStart(2, "0");
  return `#${channel(r)}${channel(g)}${channel(b)}`;
}

export function rgbToHsv({ r, g, b }: Rgb): Hsv {
  const red = r / 255;
  const green = g / 255;
  const blue = b / 255;
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const delta = max - min;

  let hue = 0;
  if (delta !== 0) {
    if (max === red) hue = ((green - blue) / delta) % 6;
    else if (max === green) hue = (blue - red) / delta + 2;
    else hue = (red - green) / delta + 4;
  }

  return {
    h: Math.round(hue * 60 + (hue < 0 ? 360 : 0)),
    s: max === 0 ? 0 : delta / max,
    v: max,
  };
}

export function hsvToRgb({ h, s, v }: Hsv): Rgb {
  const chroma = v * s;
  const hue = ((h % 360) + 360) % 360 / 60;
  const second = chroma * (1 - Math.abs((hue % 2) - 1));
  const match = v - chroma;

  const [red, green, blue] =
    hue < 1 ? [chroma, second, 0]
    : hue < 2 ? [second, chroma, 0]
    : hue < 3 ? [0, chroma, second]
    : hue < 4 ? [0, second, chroma]
    : hue < 5 ? [second, 0, chroma]
    : [chroma, 0, second];

  return {
    r: Math.round((red + match) * 255),
    g: Math.round((green + match) * 255),
    b: Math.round((blue + match) * 255),
  };
}

export function hexToHsv(hex: string): Hsv | null {
  const rgb = parseHex(hex);
  return rgb ? rgbToHsv(rgb) : null;
}

export function hsvToHex(hsv: Hsv): string {
  return toHex(hsvToRgb(hsv));
}

/**
 * Pick black or white text for a swatch, by perceived luminance.
 *
 * Uses the sRGB coefficients rather than a plain average: mid-greens look
 * light to the eye and mid-blues look dark, and an average gets both wrong.
 */
export function readableInk(hex: string): string {
  const rgb = parseHex(hex);
  if (!rgb) return "#000000";
  const luminance =
    (0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b) / 255;
  return luminance > 0.6 ? "#000000" : "#ffffff";
}

/** Whether a string is a colour this picker can round-trip. */
export function isColor(value: string): boolean {
  return parseHex(value) !== null;
}
