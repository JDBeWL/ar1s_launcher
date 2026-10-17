/**
 * Vuetify 主题插件
 *
 * 把 md3 推导出的令牌拼成 Vuetify 认识的 light- / dark- 两套主题，
 * 并注入不随配色变化的容器色。
 */
import 'vuetify/styles';
import '@mdi/font/css/materialdesignicons.css';
import { createVuetify } from 'vuetify';
import { buildTheme } from '../theme/md3';
import { DEFAULT_THEME_PRESET, THEME_PRESETS } from '../theme/presets';

// 加载器品牌色与告警色阶的容器色不随主题方案变化，集中定义后注入每个主题
const toneLight = {
  'warning-container': '#FFDBC8',
  'on-warning-container': '#3E2600',
  'blue-container': '#D7E3FF',
  'red-container': '#FFDAD6',
  'purple-container': '#E9DDFF',
  'orange-container': '#FFDBC8',
  'teal-container': '#B6EDE3',
  'grey-container': '#E7E0EC',
};

const toneDark = {
  'warning-container': '#7A4F14',
  'on-warning-container': '#FFDBC8',
  'blue-container': '#1A4A73',
  'red-container': '#732B2B',
  'purple-container': '#4A3573',
  'orange-container': '#6E4A1A',
  'teal-container': '#1A5C55',
  'grey-container': '#49454F',
};

const fieldDefaults = {
  rounded: 'xl',
  variant: 'outlined',
  density: 'comfortable',
};

const menuProps = {
  contentClass: 'v-select-menu-content',
  offset: 4,
};

function buildThemes() {
  return Object.fromEntries(
    THEME_PRESETS.flatMap((preset) => {
      const palette = buildTheme(preset.seed);
      return [
        [`light-${preset.id}`, { dark: false, colors: { ...palette.light, ...toneLight } }],
        [`dark-${preset.id}`, { dark: true, colors: { ...palette.dark, ...toneDark } }],
      ];
    }),
  );
}

const cspNonce = [...document.querySelectorAll('style')].map((s) => s.nonce).find(Boolean);

const vuetify = createVuetify({
  theme: {
    defaultTheme: `dark-${DEFAULT_THEME_PRESET}`,
    cspNonce,
    themes: {
      ...buildThemes(),
    },
  },
  defaults: {
    VBtn: { rounded: 'xl', fontWeight: 500 },
    VCard: { rounded: 'xl' },
    VTextField: fieldDefaults,
    VSelect: { ...fieldDefaults, menuProps },
    VAutocomplete: { ...fieldDefaults, menuProps },
    VCombobox: { ...fieldDefaults, menuProps },
    VMenu: { rounded: 'lg', offset: 4, transition: false, location: 'bottom end' },
    VAlert: { rounded: 'xl' },
    VDialog: { rounded: 'xl', transition: false },
    VList: { rounded: 'lg' },
    VListItem: { rounded: 'lg' },
  },
});

export default vuetify;
