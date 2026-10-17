// 必须最先导入：vuetify/styles（main.css）里的 @layer 声明建立层级顺序
// （components > core.reset）。若 App.vue 的组件样式先注入，层顺序会被颠倒，
// reset 层的 button { font: inherit } 就会覆盖按钮的 font-size/font-weight。
import vuetify from './plugins/vuetify';
import { createApp } from 'vue';
import { createPinia } from 'pinia';
import App from './App.vue';
import router from './router';
import { logError } from './utils/logger';

// 字体全部自托管，@font-face 声明集中在 style.css：
//   · Roboto（拉丁/数字）—— 走 @fontsource 的 woff2 文件，但用自己的 @font-face 声明，
//     因为必须带度量覆盖（ascent/descent-override）才能让基线停在改动前的位置，详见 style.css；
//   · Noto Sans SC 子集（中文）—— public/fonts/，由 scripts/build-cjk-subset.py 生成。
// 这里不再导入 @fontsource 自带的 CSS 入口：那会再产生一组没有度量覆盖的同名 @font-face，
// 两组规则同名同字重时会互相干扰，且会额外打进用不到的 .woff 文件。
import './style.css';

document.title = 'Ar1s Launcher';

if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
  });
}

const app = createApp(App);

app.config.errorHandler = (err, _instance, info) => {
  logError('Unhandled Vue error', err, info);
};

app.use(vuetify);
app.use(createPinia());
app.use(router);
app.mount('#root');
