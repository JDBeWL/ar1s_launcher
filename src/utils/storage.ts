const VALID_THEME_PALETTES = new Set([
  'indigo', 'blue', 'purple', 'teal', 'orange', 'red', 'green', 'pink'
]);

const VALID_VIEW_MODES = new Set(['grid', 'list']);
const VALID_SORT_BY = new Set(['lastPlayed', 'name', 'created']);

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