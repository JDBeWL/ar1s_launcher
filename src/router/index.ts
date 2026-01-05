import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  { path: '/', component: () => import('../views/HomeView.vue') },
  { path: '/download', component: () => import('../views/DownloadView.vue') },
  { path: '/settings', component: () => import('../views/SettingsView.vue') },
  { path: '/instance-manager', component: () => import('../views/InstanceManagerView.vue') },
  { path: '/add-instance', component: () => import('../views/AddInstanceView.vue') },
  { path: '/install-modpack', component: () => import('../views/InstallModpackView.vue') },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

export default router
