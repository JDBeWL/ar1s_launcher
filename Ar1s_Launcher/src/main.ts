import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import 'vuetify/styles'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import '@mdi/font/css/materialdesignicons.css'

// 尝试修改WebView进程名称
try {
  // 修改用户代理
  Object.defineProperty(navigator, 'userAgent', {
    get: function() { return 'Ar1s Launcher WebView'; }
  });
  
  // 修改应用名称
  document.title = 'Ar1s Launcher';
  
  // 如果是Tauri环境，尝试使用Tauri API
  if (typeof window !== 'undefined' && '__TAURI__' in window) {
    console.log('在Tauri环境中设置WebView名称');
    // 这里可以添加Tauri特定的API调用
  }
} catch (e) {
  console.error('修改WebView进程名称失败:', e);
}

// 创建 Vuetify 实例
const vuetify = createVuetify({
  components,
  directives,
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
          'on-primary': '#000000',  // 在主色调背景上的文字颜色（黑色）
          'primary-darken-1': '#a58fe9', // 稍深的主色调，用于悬停效果
          secondary: '#ccc2dc',
          'on-secondary': '#000000', // 在次要色调背景上的文字颜色
          error: '#CF6679',
          info: '#2196F3',
          success: '#4CAF50',
          warning: '#FB8C00',
        }
      }
    }
  },
})

// 创建 Pinia 实例
const pinia = createPinia()

// 创建路由
const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: () => import('./views/HomeView.vue') },
    { path: '/download', component: () => import('./views/DownloadView.vue') },
    { path: '/settings', component: () => import('./views/SettingsView.vue') },
  ],
})

// 创建并挂载应用
const app = createApp(App)
app.use(vuetify)
app.use(pinia)
app.use(router)
app.mount('#root')