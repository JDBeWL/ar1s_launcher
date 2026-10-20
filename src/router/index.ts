/**
 * 前端路由
 * HTML5 history 模式下的懒加载路由
 */
import { createRouter, createWebHistory } from 'vue-router';
import type { RouteRecordRaw } from 'vue-router';
import { isVanillaDownloadEnabled } from '../utils/vanillaGate';

const routes: RouteRecordRaw[] = [
  { path: '/', component: () => import('../views/HomeView.vue') },
  { path: '/download', component: () => import('../views/DownloadView.vue') },
  { path: '/settings', component: () => import('../views/SettingsView.vue') },
  {
    path: '/instance-manager',
    name: 'instance-manager',
    component: () => import('../views/InstanceManagerView.vue'),
  },
  {
    path: '/instance/:name',
    name: 'instance-detail',
    component: () => import('../views/InstanceDetailView.vue'),
  },
  { path: '/add-instance', component: () => import('../views/AddInstanceView.vue') },
  { path: '/install-modpack', component: () => import('../views/InstallModpackView.vue') },
];

/** 依赖原版游戏文件下载的页面：入口已从界面上拿掉，这里再挡一次直达链接 */
const VANILLA_DEPENDENT_PATHS = ['/download', '/add-instance', '/install-modpack'];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

router.beforeEach((to) => {
  if (isVanillaDownloadEnabled()) return true;
  return VANILLA_DEPENDENT_PATHS.includes(to.path) ? { path: '/' } : true;
});

export default router;
