import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import vuetify from './plugins/vuetify'

// 设置应用标题
document.title = 'Ar1s Launcher'

// 在生产环境中禁用右键菜单
if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault()
  })
}

// 创建并挂载应用
const app = createApp(App)
app.use(vuetify)
app.use(createPinia())
app.use(router)
app.mount('#root')
