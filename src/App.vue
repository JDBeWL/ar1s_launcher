<script setup lang="ts">
import { Window } from '@tauri-apps/api/window'
import { ref, onMounted, onUnmounted } from 'vue'
import { useTheme } from 'vuetify'
import { useDownloadStore } from './stores/downloadStore'
import { useLauncherStore } from './stores/launcherStore'
import GlobalDownloadStatus from './components/GlobalDownloadStatus.vue'
import GlobalNotification from './components/GlobalNotification.vue'

// 窗口控制
const appWindow = Window.getCurrent()
const window = {
  minimize: () => appWindow.minimize(),
  toggleMaximize: async () => {
    const isMaximized = await appWindow.isMaximized()
    isMaximized ? appWindow.unmaximize() : appWindow.maximize()
  },
  close: () => appWindow.close()
}

// 导航栏控制
const rail = ref(true)

// 主题控制
const theme = useTheme()
const isDarkMode = ref(true)

// 切换主题模式
function toggleTheme() {
  isDarkMode.value = !isDarkMode.value
  const newTheme = isDarkMode.value ? 'dark' : 'light'
  theme.change(newTheme)
  localStorage.setItem('theme', newTheme)
}

const downloadStore = useDownloadStore()
const launcherStore = useLauncherStore()

// 初始化下载监听器和主题
onMounted(async () => {
  // 初始化监听器
  await downloadStore.subscribe()
  await launcherStore.subscribe()
  
  // 初始化主题
  const savedTheme = localStorage.getItem('theme')
  if (savedTheme) {
    isDarkMode.value = savedTheme === 'dark'
  }
  const themeName = isDarkMode.value ? 'dark' : 'light'
  theme.change(themeName)
})

// 清理监听器防止内存泄漏
onUnmounted(() => {
  downloadStore.unsubscribe()
  launcherStore.unsubscribe()
})
</script>

<template>
  <v-app>
    <v-navigation-drawer 
      :rail="rail" 
      :mobile-breakpoint="0" 
      rail-width="64"
      width="220"
      color="surface-container"
    >
      <v-list nav class="nav-list">
        <v-list-item 
          prepend-icon="mdi-minecraft" 
          title="启动" 
          to="/" 
          class="nav-item mb-1"
        />
        <v-list-item 
          prepend-icon="mdi-download" 
          title="下载" 
          to="/download" 
          class="nav-item mb-1"
        />
        <v-list-item 
          prepend-icon="mdi-plus-circle-outline" 
          title="添加实例" 
          to="/add-instance" 
          class="nav-item mb-1"
        />
        <v-list-item 
          prepend-icon="mdi-folder-multiple-outline" 
          title="实例管理" 
          to="/instance-manager" 
          class="nav-item"
        />
      </v-list>

      <template v-slot:append>
        <v-list nav class="nav-list">
          <v-list-item 
            prepend-icon="mdi-cog-outline" 
            title="设置" 
            to="/settings" 
            class="nav-item"
          />
        </v-list>
      </template>
    </v-navigation-drawer>

    <v-app-bar class="titlebar" data-tauri-drag-region elevation="0" color="surface">
      <template v-slot:prepend>
        <v-app-bar-nav-icon @click="rail = !rail" data-tauri-no-drag />
      </template>
      <v-app-bar-title class="font-weight-bold">Ar1s Launcher</v-app-bar-title>
      <v-spacer />
      
      <!-- 主题切换按钮 -->
      <v-btn 
        icon 
        data-tauri-no-drag 
        @click="toggleTheme" 
        class="theme-toggle-btn mr-1"
        variant="text"
      >
        <v-icon>{{ isDarkMode ? 'mdi-weather-sunny' : 'mdi-weather-night' }}</v-icon>
        <v-tooltip activator="parent" location="bottom">
          {{ isDarkMode ? '切换到浅色模式' : '切换到深色模式' }}
        </v-tooltip>
      </v-btn>
      
      <!-- 窗口控制按钮 -->
      <v-btn icon data-tauri-no-drag @click="window.minimize()" variant="text">
        <v-icon size="20">mdi-minus</v-icon>
      </v-btn>
      <v-btn icon data-tauri-no-drag @click="window.toggleMaximize()" variant="text">
        <v-icon size="18">mdi-square-outline</v-icon>
      </v-btn>
      <v-btn icon data-tauri-no-drag @click="window.close()" variant="text" class="close-btn">
        <v-icon size="20">mdi-close</v-icon>
      </v-btn>
    </v-app-bar>

    <v-main>
      <router-view />
    </v-main>
    
    <!-- 全局下载状态组件 -->
    <GlobalDownloadStatus />
    <!-- 全局通知组件 -->
    <GlobalNotification />
  </v-app>
</template>

<style>
:root {
  color-scheme: light dark;
}

/* Hide scrollbar while keeping scroll functionality */
::-webkit-scrollbar {
  display: none;
}

/* Titlebar styles */
.titlebar .v-toolbar__content {
  pointer-events: none;
}

.titlebar .v-btn,
.titlebar .v-app-bar-nav-icon {
  pointer-events: auto;
}

/* Navigation list padding */
.nav-list {
  padding: 8px;
}

/* Theme toggle animation */
.theme-toggle-btn {
  transition: transform 0.3s ease;
}

.theme-toggle-btn:hover {
  transform: rotate(30deg);
}

/* Close button hover */
.close-btn:hover {
  background-color: rgb(var(--v-theme-error)) !important;
  color: rgb(var(--v-theme-on-error)) !important;
}

/* Navigation item styles - MD3 */
.nav-item {
  margin-bottom: 4px;
}

.nav-item.v-list-item--active {
  background: rgb(var(--v-theme-secondary-container));
  color: rgb(var(--v-theme-on-secondary-container));
}

.nav-item.v-list-item--active .v-icon {
  color: rgb(var(--v-theme-on-secondary-container));
}

/* MD3 elevation and transitions */
.v-btn {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.v-card {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

/* MD3 Surface tones */
.surface-container {
  background-color: rgb(var(--v-theme-surface-container)) !important;
}

.surface-container-high {
  background-color: rgb(var(--v-theme-surface-container-high)) !important;
}

.surface-container-highest {
  background-color: rgb(var(--v-theme-surface-container-highest)) !important;
}

/* Navigation drawer rail mode - center icons */
.v-navigation-drawer--rail .nav-list {
  padding: 8px;
}

.v-navigation-drawer--rail .nav-list .v-list-item {
  padding: 0 !important;
  min-height: 48px;
}

.v-navigation-drawer--rail .nav-list .v-list-item > .v-list-item__prepend {
  margin-left: 12px !important;
}

.v-navigation-drawer--rail .nav-list .v-list-item .v-list-item-title,
.v-navigation-drawer--rail .nav-list .v-list-item .v-list-item__content {
  display: none !important;
}

/* Smooth theme transition */
.v-application {
  transition: background-color 0.3s ease, color 0.3s ease;
}
</style>
