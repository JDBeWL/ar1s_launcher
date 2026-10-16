// 必须是 plugins/vuetify.ts 里真实注册过的配色，否则会被回退成默认值
const VALID_THEME_PALETTES = new Set(['indigo', 'emerald', 'rose']);

const VALID_VIEW_MODES = new Set(['grid', 'list']);
const VALID_SORT_BY = new Set(['lastPlayed', 'name', 'created']);

export function isValidThemePalette(value: string): boolean {
  return VALID_THEME_PALETTES.has(value);
}

export function getThemePalette(defaultValue: string = 'indigo'): string {
  const stored = localStorage.getItem('themePalette');
  if (stored && VALID_THEME_PALETTES.has(stored)) {
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
