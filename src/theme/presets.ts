/**
 * 主题色预设
 *
 * 每条只描述一颗种子色，整套 MD3 令牌由 `buildTheme` 推导。
 * `id` 会拼进 Vuetify 主题名（`dark-${id}`），改名会让用户已保存的选择失效。
 * 展示名不放这里，按 id 去 i18n 的 `theme.presets.<id>` 取。
 */
import type { ThemePreset } from './md3';

export const THEME_PRESETS: ThemePreset[] = [
  { id: 'indigo', seed: '#6750A4' },
  { id: 'ocean', seed: '#0B57D0' },
  { id: 'sky', seed: '#00639B' },
  { id: 'emerald', seed: '#006A60' },
  { id: 'forest', seed: '#386A20' },
  { id: 'amber', seed: '#8B5000' },
  { id: 'rose', seed: '#9A4050' },
  { id: 'plum', seed: '#7B3F6B' },
  { id: 'slate', seed: '#465A6B' },
  { id: 'graphite', seed: '#44464F' },
];

export const DEFAULT_THEME_PRESET = THEME_PRESETS[0].id;

export const THEME_PRESET_IDS = new Set(THEME_PRESETS.map((preset) => preset.id));
