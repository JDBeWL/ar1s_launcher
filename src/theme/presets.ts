/**
 * 主题色预设
 *
 * 每条只描述一颗种子色，整套 MD3 令牌由 `buildTheme` 推导。
 * `id` 会拼进 Vuetify 主题名（`dark-${id}`），改名会让用户已保存的选择失效。
 */
import type { ThemePreset } from './md3';

export const THEME_PRESETS: ThemePreset[] = [
  { id: 'indigo', label: '经典紫', seed: '#6750A4' },
  { id: 'ocean', label: '深海蓝', seed: '#0B57D0' },
  { id: 'sky', label: '天青', seed: '#00639B' },
  { id: 'emerald', label: '青碧', seed: '#006A60' },
  { id: 'forest', label: '松林', seed: '#386A20' },
  { id: 'amber', label: '琥珀', seed: '#8B5000' },
  { id: 'rose', label: '玫瑰', seed: '#9A4050' },
  { id: 'plum', label: '梅紫', seed: '#7B3F6B' },
  { id: 'slate', label: '岩灰', seed: '#465A6B' },
  { id: 'graphite', label: '石墨', seed: '#44464F' },
];

export const DEFAULT_THEME_PRESET = THEME_PRESETS[0].id;

export const THEME_PRESET_IDS = new Set(THEME_PRESETS.map((preset) => preset.id));
