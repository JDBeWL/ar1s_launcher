import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'
import { createVuetify } from 'vuetify'

// 使用 vite-plugin-vuetify 自动按需导入组件和指令
const vuetify = createVuetify({
  theme: {
    defaultTheme: 'dark',
    themes: {
      light: {
        dark: false,
        colors: {
          background: '#e0e0e6',
          surface: '#e8e8ec',
          primary: '#6750a4',
          secondary: '#625b71',
          error: '#B00020',
          info: '#2196F3',
          success: '#4CAF50',
          warning: '#FB8C00',
        }
      },
      dark: {
        dark: true,
        colors: {
          background: '#1c1b1f',
          surface: '#1c1b1f',
          primary: '#d0bcff',
          'on-primary': '#000000',
          'primary-darken-1': '#a58fe9',
          secondary: '#ccc2dc',
          'on-secondary': '#000000',
          error: '#CF6679',
          info: '#2196F3',
          success: '#4CAF50',
          warning: '#FB8C00',
        }
      }
    }
  },
})

export default vuetify
