/** MD3 取色测试 / 校验 tone 推导、令牌集合完整性和配色对比度 */
import { describe, expect, it } from 'vitest';
import { buildTheme, hexToLch, toneColor } from './md3';
import { DEFAULT_THEME_PRESET, THEME_PRESET_IDS, THEME_PRESETS } from './presets';

/** 全部令牌：新增配色必须一次给全，缺一个 Vuetify 就会静默回退成默认色 */
const EXPECTED_TOKENS = [
  'primary',
  'on-primary',
  'primary-container',
  'on-primary-container',
  'secondary',
  'on-secondary',
  'secondary-container',
  'on-secondary-container',
  'tertiary',
  'on-tertiary',
  'tertiary-container',
  'on-tertiary-container',
  'error',
  'on-error',
  'error-container',
  'on-error-container',
  'background',
  'on-background',
  'surface',
  'on-surface',
  'surface-variant',
  'on-surface-variant',
  'surface-container',
  'surface-container-high',
  'surface-container-highest',
  'surface-container-low',
  'surface-container-lowest',
  'outline',
  'outline-variant',
  'info',
  'success',
  'warning',
].sort();

function channels(hex: string): [number, number, number] {
  const value = Number.parseInt(hex.slice(1), 16);
  return [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];
}

function luminance(hex: string): number {
  const [r, g, b] = channels(hex).map((channel) => {
    const v = channel / 255;
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

function channelDistance(a: string, b: string): number {
  const [ar, ag, ab] = channels(a);
  const [br, bg, bb] = channels(b);
  return Math.max(Math.abs(ar - br), Math.abs(ag - bg), Math.abs(ab - bb));
}

describe('MD3 配色推导', () => {
  it('预设 id 唯一，默认值在白名单里', () => {
    expect(THEME_PRESET_IDS.size).toBe(THEME_PRESETS.length);
    expect(THEME_PRESET_IDS.has(DEFAULT_THEME_PRESET)).toBe(true);
    expect(THEME_PRESET_IDS.size).toBeGreaterThanOrEqual(10);
  });

  it('每套配色都给全了令牌，且都是合法的十六进制', () => {
    for (const preset of THEME_PRESETS) {
      const palette = buildTheme(preset.seed);
      for (const mode of ['light', 'dark'] as const) {
        expect(Object.keys(palette[mode]).sort(), `${preset.id}/${mode}`).toEqual(EXPECTED_TOKENS);
        for (const [token, value] of Object.entries(palette[mode])) {
          expect(value, `${preset.id}/${mode}/${token}`).toMatch(/^#[0-9A-F]{6}$/);
        }
      }
    }
  });

  it('原有三套的种子色仍落在主色上', () => {
    // 种子在 tone 40 本来就等于主色，差一两个色阶是色域取整
    expect(buildTheme('#6750A4').light.primary).toBe('#6750A4');
    expect(channelDistance(buildTheme('#006A60').light.primary, '#006A60')).toBeLessThanOrEqual(2);
    expect(channelDistance(buildTheme('#9A4050').light.primary, '#9A4050')).toBeLessThanOrEqual(2);
  });

  it('主色容器与手工调校值同量级', () => {
    // 浅色容器是手调过的，取色规则改动不该把它们带偏
    expect(channelDistance(buildTheme('#6750A4').light['primary-container'], '#EADDFF')).toBeLessThanOrEqual(6);
    expect(channelDistance(buildTheme('#9A4050').light['primary-container'], '#FFD9DE')).toBeLessThanOrEqual(6);
  });

  it('所有预设、深浅两套都满足对比度', () => {
    for (const preset of THEME_PRESETS) {
      const palette = buildTheme(preset.seed);

      for (const mode of ['light', 'dark'] as const) {
        const c = palette[mode];
        const pairs: [string, string, string, number][] = [
          ['on-primary', c['on-primary'], c.primary, 4.5],
          ['on-primary-container', c['on-primary-container'], c['primary-container'], 4.5],
          ['on-secondary', c['on-secondary'], c.secondary, 4.5],
          ['on-secondary-container', c['on-secondary-container'], c['secondary-container'], 4.5],
          ['on-tertiary', c['on-tertiary'], c.tertiary, 4.5],
          ['on-tertiary-container', c['on-tertiary-container'], c['tertiary-container'], 4.5],
          ['on-surface', c['on-surface'], c.surface, 7],
          ['on-surface-variant', c['on-surface-variant'], c['surface-container'], 4.5],
          ['on-surface-variant-high', c['on-surface-variant'], c['surface-container-high'], 4.5],
          ['outline', c.outline, c.surface, 3],
          ['primary-on-surface', c.primary, c.surface, 4.5],
        ];

        for (const [name, fg, bg, min] of pairs) {
          expect(contrast(fg, bg), `${preset.id}/${mode}/${name}`).toBeGreaterThanOrEqual(min);
        }
      }
    }
  });

  it('浅色模式的表面比主色浅，深色模式反过来', () => {
    for (const preset of THEME_PRESETS) {
      const { light, dark } = buildTheme(preset.seed);
      expect(luminance(light.surface)).toBeGreaterThan(luminance(light.primary));
      expect(luminance(dark.surface)).toBeLessThan(luminance(dark.primary));
    }
  });

  it('饱和度取自种子：灰种子不该冒出彩色主色', () => {
    expect(hexToLch(buildTheme('#44464F').light.primary).c).toBeLessThan(12);
    expect(hexToLch(buildTheme('#0B57D0').light.primary).c).toBeGreaterThan(45);
  });

  it('同一种子结果稳定，不同种子主色不同', () => {
    expect(buildTheme('#0B57D0')).toEqual(buildTheme('#0B57D0'));
    const primaries = new Set(THEME_PRESETS.map((preset) => buildTheme(preset.seed).light.primary));
    expect(primaries.size).toBe(THEME_PRESETS.length);
  });

  it('超出色域的彩度会被收敛而不是裁坏通道', () => {
    for (const tone of [0, 10, 40, 55, 90, 100]) {
      expect(toneColor(tone, 120, 30)).toMatch(/^#[0-9A-F]{6}$/);
    }
    expect(toneColor(0, 80, 120)).toBe('#000000');
    expect(toneColor(100, 80, 120)).toBe('#FFFFFF');
  });
});
