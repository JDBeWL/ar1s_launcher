/**
 * localStorage 偏好读取
 * 主题、视图模式与排序键的读取，全部按白名单校验、非法值回退默认
 */
import { DEFAULT_THEME_PRESET, THEME_PRESET_IDS } from '../theme/presets';

const VALID_VIEW_MODES = new Set(['grid', 'list']);
// 必须与 InstanceManagerView 的 sortOptions 及排序 switch 的分支一致，否则选了就存不住
const VALID_SORT_BY = new Set(['lastPlayed', 'name', 'version']);

export function isValidThemePalette(value: string): boolean {
  return THEME_PRESET_IDS.has(value);
}

export function getThemePalette(defaultValue: string = DEFAULT_THEME_PRESET): string {
  const stored = localStorage.getItem('themePalette');
  if (stored && THEME_PRESET_IDS.has(stored)) {
    return stored;
  }
  return defaultValue;
}

export function getTheme(defaultValue: 'dark' | 'light' = 'dark'): 'dark' | 'light' {
  const stored = localStorage.getItem('theme');
  if (stored === 'dark' || stored === 'light') {
    return stored;
  }
  return defaultValue;
}

export function getInstanceViewMode(defaultValue: 'grid' | 'list' = 'grid'): 'grid' | 'list' {
  const stored = localStorage.getItem('instanceViewMode');
  if (stored && VALID_VIEW_MODES.has(stored)) {
    return stored as 'grid' | 'list';
  }
  return defaultValue;
}

export function getInstanceSortBy(defaultValue: string = 'lastPlayed'): string {
  const stored = localStorage.getItem('instanceSortBy');
  if (stored && VALID_SORT_BY.has(stored)) {
    return stored;
  }
  return defaultValue;
}
