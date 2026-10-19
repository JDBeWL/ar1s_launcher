/**
 * MD3 主题色生成
 *
 * 预设只给一颗种子色，这里推导整套 light/dark 令牌：色相取自种子（tertiary 偏移 60°），
 * 彩度按角色比例派生，tone 就是 CIELab 的 L*，所以「容器 90 / 容器文字 10」的对比是算出来的。
 * 超出 sRGB 色域时二分降彩度收敛，不硬裁通道（硬裁会让色相跑偏）。
 */

export type ThemeTokens = Record<string, string>;

export interface ThemePalette {
  light: ThemeTokens;
  dark: ThemeTokens;
}

export interface ThemePreset {
  /** 主题名后缀，与存储值一致：`light-${id}` / `dark-${id}` */
  id: string;
  /** 种子色：既决定色相，也决定整体饱和度（灰种子出灰配色） */
  seed: string;
}

interface Lch {
  l: number;
  c: number;
  h: number;
}

/** 各角色相对主色的彩度比例，按现有三套手调配色反推；主色沿用种子自身的彩度 */
const CHROMA_RATIO = {
  secondary: 0.3,
  tertiary: 0.45,
  neutral: 0.1,
  neutralVariant: 0.16,
} as const;

/** 中性色只染一丝种子色相，彩度上限避免背景泛色 */
const NEUTRAL_MAX = 5.5;
const NEUTRAL_VARIANT_MAX = 8;

/** D65 白点 */
const WHITE: [number, number, number] = [0.95047, 1, 1.08883];

/** 深/浅色共用的语义色，与主题色无关 */
const SHARED = {
  light: {
    error: '#B3261E',
    'on-error': '#FFFFFF',
    'error-container': '#F9DEDC',
    'on-error-container': '#410E0B',
    info: '#0288D1',
    success: '#2E7D32',
    warning: '#ED6C02',
  },
  dark: {
    error: '#F2B8B5',
    'on-error': '#601410',
    'error-container': '#8C1D18',
    'on-error-container': '#F9DEDC',
    info: '#29B6F6',
    success: '#66BB6A',
    warning: '#FFA726',
  },
} as const;

function srgbToLinear(v: number): number {
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

function linearToSrgb(v: number): number {
  return v <= 0.0031308 ? v * 12.92 : 1.055 * v ** (1 / 2.4) - 0.055;
}

function pivot(t: number): number {
  return t > 216 / 24389 ? Math.cbrt(t) : ((24389 / 27) * t + 16) / 116;
}

function inversePivot(t: number): number {
  const cubed = t ** 3;
  return cubed > 216 / 24389 ? cubed : (116 * t - 16) / (24389 / 27);
}

function parseHex(hex: string): [number, number, number] {
  const text = hex.replace('#', '');
  if (!/^[0-9a-fA-F]{6}$/.test(text)) {
    throw new Error(`不是合法的 6 位颜色：${hex}`);
  }
  const value = Number.parseInt(text, 16);
  return [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];
}

/** 十六进制 → CIELCh，用于取出种子的色相与彩度 */
export function hexToLch(hex: string): Lch {
  const [r, g, b] = parseHex(hex).map((channel) => srgbToLinear(channel / 255));
  const x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / WHITE[0];
  const y = (0.2126729 * r + 0.7151522 * g + 0.072175 * b) / WHITE[1];
  const z = (0.0193339 * r + 0.119192 * g + 0.9503041 * b) / WHITE[2];

  const fx = pivot(x);
  const fy = pivot(y);
  const fz = pivot(z);
  const l = 116 * fy - 16;
  const a = 500 * (fx - fy);
  const bb = 200 * (fy - fz);

  return {
    l,
    c: Math.hypot(a, bb),
    h: ((Math.atan2(bb, a) * 180) / Math.PI + 360) % 360,
  };
}

/** CIELCh → 线性 sRGB；结果可能落在色域外 */
function lchToLinearRgb(l: number, c: number, h: number): [number, number, number] {
  const rad = (h * Math.PI) / 180;
  const a = Math.cos(rad) * c;
  const bb = Math.sin(rad) * c;

  const fy = (l + 16) / 116;
  const fx = fy + a / 500;
  const fz = fy - bb / 200;

  const x = WHITE[0] * inversePivot(fx);
  const y = WHITE[1] * inversePivot(fy);
  const z = WHITE[2] * inversePivot(fz);

  return [
    3.2404542 * x - 1.5371385 * y - 0.4985314 * z,
    -0.969266 * x + 1.8760108 * y + 0.041556 * z,
    0.0556434 * x - 0.2040259 * y + 1.0572252 * z,
  ];
}

function inGamut(rgb: [number, number, number]): boolean {
  const epsilon = 0.0005;
  return rgb.every((channel) => channel >= -epsilon && channel <= 1 + epsilon);
}

function toHex(channel: number): string {
  const clamped = Math.min(255, Math.max(0, Math.round(channel)));
  return clamped.toString(16).padStart(2, '0');
}

/** 取色调板在指定 tone 上的颜色；tone 越界或彩度超域时收敛到能显示的最大彩度 */
export function toneColor(tone: number, chromaCap: number, hue: number): string {
  const l = Math.min(100, Math.max(0, tone));
  let low = 0;
  let high = Math.max(0, chromaCap);

  if (inGamut(lchToLinearRgb(l, high, hue))) {
    low = high;
  } else {
    for (let i = 0; i < 20; i += 1) {
      const mid = (low + high) / 2;
      if (inGamut(lchToLinearRgb(l, mid, hue))) {
        low = mid;
      } else {
        high = mid;
      }
    }
  }

  const rgb = lchToLinearRgb(l, low, hue).map((channel) =>
    Math.min(255, Math.max(0, Math.round(linearToSrgb(Math.min(1, Math.max(0, channel))) * 255))),
  );

  return `#${rgb.map(toHex).join('').toUpperCase()}`;
}

/** 从种子色推导一套 light/dark 令牌 */
export function buildTheme(seed: string): ThemePalette {
  const { c: seedChroma, h: hue } = hexToLch(seed);

  const role =
    (chroma: number, hueOffset = 0) =>
    (tone: number) =>
      toneColor(tone, chroma, hue + hueOffset);

  const primary = role(seedChroma);
  const secondary = role(seedChroma * CHROMA_RATIO.secondary);
  const tertiary = role(seedChroma * CHROMA_RATIO.tertiary, 60);
  const neutral = role(Math.min(NEUTRAL_MAX, seedChroma * CHROMA_RATIO.neutral));
  const neutralVariant = role(Math.min(NEUTRAL_VARIANT_MAX, seedChroma * CHROMA_RATIO.neutralVariant));

  return {
    light: {
      primary: primary(40),
      'on-primary': primary(100),
      'primary-container': primary(90),
      'on-primary-container': primary(10),

      secondary: secondary(40),
      'on-secondary': secondary(100),
      'secondary-container': secondary(90),
      'on-secondary-container': secondary(10),

      tertiary: tertiary(40),
      'on-tertiary': tertiary(100),
      'tertiary-container': tertiary(90),
      'on-tertiary-container': tertiary(10),

      ...SHARED.light,

      background: neutral(98),
      'on-background': neutral(10),
      surface: neutral(98),
      'on-surface': neutral(10),
      'surface-variant': neutralVariant(90),
      'on-surface-variant': neutralVariant(30),
      'surface-container': neutral(94),
      'surface-container-high': neutral(92),
      'surface-container-highest': neutral(90),
      'surface-container-low': neutral(96),
      'surface-container-lowest': neutral(100),
      outline: neutralVariant(50),
      'outline-variant': neutralVariant(80),
    },
    dark: {
      primary: primary(80),
      'on-primary': primary(20),
      'primary-container': primary(30),
      'on-primary-container': primary(90),

      secondary: secondary(80),
      'on-secondary': secondary(20),
      'secondary-container': secondary(30),
      'on-secondary-container': secondary(90),

      tertiary: tertiary(80),
      'on-tertiary': tertiary(20),
      'tertiary-container': tertiary(30),
      'on-tertiary-container': tertiary(90),

      ...SHARED.dark,

      background: neutral(6),
      'on-background': neutral(90),
      surface: neutral(6),
      'on-surface': neutral(90),
      'surface-variant': neutralVariant(30),
      'on-surface-variant': neutralVariant(80),
      'surface-container': neutral(12),
      'surface-container-high': neutral(17),
      'surface-container-highest': neutral(22),
      'surface-container-low': neutral(10),
      'surface-container-lowest': neutral(4),
      outline: neutralVariant(60),
      'outline-variant': neutralVariant(30),
    },
  };
}
