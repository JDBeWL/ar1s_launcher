// 必须最先导入：vuetify/styles 的 @layer 声明靠导入先后建立层序，组件样式先注入会颠倒成 components < core.reset
// 层序颠倒后 reset 的 button { font: inherit } 会盖掉按钮的字号与字重
import vuetify from './plugins/vuetify';
import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import router from './router';
import { i18n } from './i18n';
import { installGlobalErrorHandlers } from './utils/globalErrors';

// 字体全部自托管，@font-face 与度量覆盖见 style.css；别导入 @fontsource 的 CSS 入口，那组同名规则没有度量覆盖会互相干扰
import './style.css';

document.title = 'Ar1s Launcher';

if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
  });
}

const app = createApp(App);
const pinia = createPinia();

app.use(vuetify);
app.use(i18n);
app.use(pinia);
app.use(router);
installGlobalErrorHandlers(app);
app.mount('#root');
