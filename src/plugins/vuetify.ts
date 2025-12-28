import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'
import { createVuetify } from 'vuetify'

// Material Design 3 主题配置
const vuetify = createVuetify({
  theme: {
    defaultTheme: 'dark',
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
    VList: {
      rounded: 'xl',
    },
    VListItem: {
      rounded: 'lg',
    },
  }
})

export default vuetify
