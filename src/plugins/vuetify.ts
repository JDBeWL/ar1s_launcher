import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'
import { createVuetify } from 'vuetify'

// Material Design 3 主题配置
const vuetify = createVuetify({
  theme: {
    defaultTheme: 'dark-indigo',
    themes: {
      // 浅色主题 - MD3 风格
      light: {
        dark: false,
        colors: {
          // Primary
          primary: '#6750A4',
          'on-primary': '#FFFFFF',
          'primary-container': '#EADDFF',
          'on-primary-container': '#21005D',
          
          // Secondary
          secondary: '#625B71',
          'on-secondary': '#FFFFFF',
          'secondary-container': '#E8DEF8',
          'on-secondary-container': '#1D192B',
          
          // Tertiary
          tertiary: '#7D5260',
          'on-tertiary': '#FFFFFF',
          'tertiary-container': '#FFD8E4',
          'on-tertiary-container': '#31111D',
          
          // Error
          error: '#B3261E',
          'on-error': '#FFFFFF',
          'error-container': '#F9DEDC',
          'on-error-container': '#410E0B',
          
          // Surface
          background: '#FEF7FF',
          'on-background': '#1D1B20',
          surface: '#FEF7FF',
          'on-surface': '#1D1B20',
          'surface-variant': '#E7E0EC',
          'on-surface-variant': '#49454F',
          'surface-container': '#F3EDF7',
          'surface-container-high': '#ECE6F0',
          'surface-container-highest': '#E6E0E9',
          'surface-container-low': '#F7F2FA',
          'surface-container-lowest': '#FFFFFF',
          
          // Outline
          outline: '#79747E',
          'outline-variant': '#CAC4D0',
          
          // Others
          info: '#0288D1',
          success: '#2E7D32',
          warning: '#ED6C02',
        }
      },
      // 深色主题 - MD3 风格
      dark: {
        dark: true,
        colors: {
          // Primary
          primary: '#D0BCFF',
          'on-primary': '#381E72',
          'primary-container': '#4F378B',
          'on-primary-container': '#EADDFF',
          
          // Secondary
          secondary: '#CCC2DC',
          'on-secondary': '#332D41',
          'secondary-container': '#4A4458',
          'on-secondary-container': '#E8DEF8',
          
          // Tertiary
          tertiary: '#EFB8C8',
          'on-tertiary': '#492532',
          'tertiary-container': '#633B48',
          'on-tertiary-container': '#FFD8E4',
          
          // Error
          error: '#F2B8B5',
          'on-error': '#601410',
          'error-container': '#8C1D18',
          'on-error-container': '#F9DEDC',
          
          // Surface
          background: '#141218',
          'on-background': '#E6E0E9',
          surface: '#141218',
          'on-surface': '#E6E0E9',
          'surface-variant': '#49454F',
          'on-surface-variant': '#CAC4D0',
          'surface-container': '#211F26',
          'surface-container-high': '#2B2930',
          'surface-container-highest': '#36343B',
          'surface-container-low': '#1D1B20',
          'surface-container-lowest': '#0F0D13',
          
          // Outline
          outline: '#938F99',
          'outline-variant': '#49454F',
          
          // Others
          info: '#29B6F6',
          success: '#66BB6A',
          warning: '#FFA726',
        }
      },
      // MD3 经典紫（与默认保持一致）
      'light-indigo': {
        dark: false,
        colors: {
          primary: '#6750A4',
          'on-primary': '#FFFFFF',
          'primary-container': '#EADDFF',
          'on-primary-container': '#21005D',
          secondary: '#625B71',
          'on-secondary': '#FFFFFF',
          'secondary-container': '#E8DEF8',
          'on-secondary-container': '#1D192B',
          tertiary: '#7D5260',
          'on-tertiary': '#FFFFFF',
          'tertiary-container': '#FFD8E4',
          'on-tertiary-container': '#31111D',
          error: '#B3261E',
          'on-error': '#FFFFFF',
          'error-container': '#F9DEDC',
          'on-error-container': '#410E0B',
          background: '#FEF7FF',
          'on-background': '#1D1B20',
          surface: '#FEF7FF',
          'on-surface': '#1D1B20',
          'surface-variant': '#E7E0EC',
          'on-surface-variant': '#49454F',
          'surface-container': '#F3EDF7',
          'surface-container-high': '#ECE6F0',
          'surface-container-highest': '#E6E0E9',
          'surface-container-low': '#F7F2FA',
          'surface-container-lowest': '#FFFFFF',
          outline: '#79747E',
          'outline-variant': '#CAC4D0',
          info: '#0288D1',
          success: '#2E7D32',
          warning: '#ED6C02',
        }
      },
      'dark-indigo': {
        dark: true,
        colors: {
          primary: '#D0BCFF',
          'on-primary': '#381E72',
          'primary-container': '#4F378B',
          'on-primary-container': '#EADDFF',
          secondary: '#CCC2DC',
          'on-secondary': '#332D41',
          'secondary-container': '#4A4458',
          'on-secondary-container': '#E8DEF8',
          tertiary: '#EFB8C8',
          'on-tertiary': '#492532',
          'tertiary-container': '#633B48',
          'on-tertiary-container': '#FFD8E4',
          error: '#F2B8B5',
          'on-error': '#601410',
          'error-container': '#8C1D18',
          'on-error-container': '#F9DEDC',
          background: '#141218',
          'on-background': '#E6E0E9',
          surface: '#141218',
          'on-surface': '#E6E0E9',
          'surface-variant': '#49454F',
          'on-surface-variant': '#CAC4D0',
          'surface-container': '#211F26',
          'surface-container-high': '#2B2930',
          'surface-container-highest': '#36343B',
          'surface-container-low': '#1D1B20',
          'surface-container-lowest': '#0F0D13',
          outline: '#938F99',
          'outline-variant': '#49454F',
          info: '#29B6F6',
          success: '#66BB6A',
          warning: '#FFA726',
        }
      },
      // MD3 清新青
      'light-emerald': {
        dark: false,
        colors: {
          primary: '#006A60',
          'on-primary': '#FFFFFF',
          'primary-container': '#74F8E3',
          'on-primary-container': '#00201C',
          secondary: '#4A635E',
          'on-secondary': '#FFFFFF',
          'secondary-container': '#CCE8E1',
          'on-secondary-container': '#05201C',
          tertiary: '#446179',
          'on-tertiary': '#FFFFFF',
          'tertiary-container': '#CBE6FF',
          'on-tertiary-container': '#001E30',
          error: '#B3261E',
          'on-error': '#FFFFFF',
          'error-container': '#F9DEDC',
          'on-error-container': '#410E0B',
          background: '#FAFDFB',
          'on-background': '#191C1B',
          surface: '#FAFDFB',
          'on-surface': '#191C1B',
          'surface-variant': '#DAE5E1',
          'on-surface-variant': '#3F4946',
          'surface-container': '#EEF1EF',
          'surface-container-high': '#E8ECEA',
          'surface-container-highest': '#E2E6E4',
          'surface-container-low': '#F4F7F5',
          'surface-container-lowest': '#FFFFFF',
          outline: '#6F7976',
          'outline-variant': '#BEC9C5',
          info: '#0288D1',
          success: '#2E7D32',
          warning: '#ED6C02',
        }
      },
      'dark-emerald': {
        dark: true,
        colors: {
          primary: '#54DBC8',
          'on-primary': '#003731',
          'primary-container': '#005047',
          'on-primary-container': '#74F8E3',
          secondary: '#B1CCC5',
          'on-secondary': '#1C3531',
          'secondary-container': '#334B47',
          'on-secondary-container': '#CCE8E1',
          tertiary: '#ACCAE6',
          'on-tertiary': '#143348',
          'tertiary-container': '#2C4A60',
          'on-tertiary-container': '#CBE6FF',
          error: '#F2B8B5',
          'on-error': '#601410',
          'error-container': '#8C1D18',
          'on-error-container': '#F9DEDC',
          background: '#0F1412',
          'on-background': '#DFE3E1',
          surface: '#0F1412',
          'on-surface': '#DFE3E1',
          'surface-variant': '#3F4946',
          'on-surface-variant': '#BEC9C5',
          'surface-container': '#1B201E',
          'surface-container-high': '#252B29',
          'surface-container-highest': '#303634',
          'surface-container-low': '#171C1A',
          'surface-container-lowest': '#0A0F0D',
          outline: '#88938F',
          'outline-variant': '#3F4946',
          info: '#29B6F6',
          success: '#66BB6A',
          warning: '#FFA726',
        }
      },
      // MD3 玫瑰
      'light-rose': {
        dark: false,
        colors: {
          primary: '#9A4050',
          'on-primary': '#FFFFFF',
          'primary-container': '#FFD9DE',
          'on-primary-container': '#3E0017',
          secondary: '#74565F',
          'on-secondary': '#FFFFFF',
          'secondary-container': '#FFD9E2',
          'on-secondary-container': '#2B151C',
          tertiary: '#7A5833',
          'on-tertiary': '#FFFFFF',
          'tertiary-container': '#FFDDBA',
          'on-tertiary-container': '#2C1600',
          error: '#B3261E',
          'on-error': '#FFFFFF',
          'error-container': '#F9DEDC',
          'on-error-container': '#410E0B',
          background: '#FFFBFF',
          'on-background': '#201A1B',
          surface: '#FFFBFF',
          'on-surface': '#201A1B',
          'surface-variant': '#F3DDDF',
          'on-surface-variant': '#524344',
          'surface-container': '#F7F1F2',
          'surface-container-high': '#F2ECED',
          'surface-container-highest': '#ECE6E7',
          'surface-container-low': '#FDF8F8',
          'surface-container-lowest': '#FFFFFF',
          outline: '#847374',
          'outline-variant': '#D7C1C3',
          info: '#0288D1',
          success: '#2E7D32',
          warning: '#ED6C02',
        }
      },
      'dark-rose': {
        dark: true,
        colors: {
          primary: '#FFB2BD',
          'on-primary': '#5F1122',
          'primary-container': '#7B2938',
          'on-primary-container': '#FFD9DE',
          secondary: '#E3BDC5',
          'on-secondary': '#422931',
          'secondary-container': '#5B3F47',
          'on-secondary-container': '#FFD9E2',
          tertiary: '#EBC08F',
          'on-tertiary': '#442A0A',
          'tertiary-container': '#5D3F1F',
          'on-tertiary-container': '#FFDDBA',
          error: '#F2B8B5',
          'on-error': '#601410',
          'error-container': '#8C1D18',
          'on-error-container': '#F9DEDC',
          background: '#181213',
          'on-background': '#EAE0E1',
          surface: '#181213',
          'on-surface': '#EAE0E1',
          'surface-variant': '#524344',
          'on-surface-variant': '#D7C1C3',
          'surface-container': '#221C1D',
          'surface-container-high': '#2D2627',
          'surface-container-highest': '#382F30',
          'surface-container-low': '#1E1819',
          'surface-container-lowest': '#120D0E',
          outline: '#9F8C8E',
          'outline-variant': '#524344',
          info: '#29B6F6',
          success: '#66BB6A',
          warning: '#FFA726',
        }
      }
    }
  },
  defaults: {
    VBtn: {
      rounded: 'xl',
      fontWeight: 500,
    },
    VCard: {
      rounded: 'xl',
    },
    VTextField: {
      rounded: 'xl',
      variant: 'outlined',
      density: 'comfortable',
    },
    VSelect: {
      rounded: 'xl',
      variant: 'outlined',
      density: 'comfortable',
      menuProps: {
        contentClass: 'v-select-menu-content',
      },
    },
    VAutocomplete: {
      rounded: 'xl',
      variant: 'outlined',
      density: 'comfortable',
      menuProps: {
        contentClass: 'v-select-menu-content',
      },
    },
    VCombobox: {
      rounded: 'xl',
      variant: 'outlined',
      density: 'comfortable',
      menuProps: {
        contentClass: 'v-select-menu-content',
      },
    },
    VChip: {
      rounded: 'xl',
    },
    VAlert: {
      rounded: 'xl',
    },
    VDialog: {
      rounded: 'xl',
    },
    VMenu: {
      rounded: 'lg',
    },
    VList: {
      rounded: 'lg',
    },
    VListItem: {
      rounded: 'lg',
    },
  }
})

export default vuetify
