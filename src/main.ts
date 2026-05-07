import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import vuetify from './plugins/vuetify'
import { logError } from './utils/logger'
import './style.css'

document.title = 'Ar1s Launcher'

if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault()
  })
}

const app = createApp(App)

app.config.errorHandler = (err, _instance, info) => {
  logError('Unhandled Vue error', err, info)
}

app.use(vuetify)
app.use(createPinia())
app.use(router)
app.mount('#root')
