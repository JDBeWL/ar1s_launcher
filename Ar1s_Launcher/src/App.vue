<script setup lang="ts">
import { Window } from '@tauri-apps/api/window'
import { ref, onMounted } from 'vue'
import { useTheme } from 'vuetify'

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
  theme.global.name.value = isDarkMode.value ? 'dark' : 'light'
  localStorage.setItem('theme', isDarkMode.value ? 'dark' : 'light')
}

// 初始化主题
onMounted(() => {
  const savedTheme = localStorage.getItem('theme')
  if (savedTheme) {
    isDarkMode.value = savedTheme === 'dark'
    theme.global.name.value = isDarkMode.value ? 'dark' : 'light'
  } else {
    // 默认使用暗色主题
    theme.global.name.value = 'dark'
  }
})
</script>

<template>
  <v-app>
    <v-navigation-drawer :rail="rail" :mobile-breakpoint="0" rail-width="64">
      <v-list nav>
        <v-list-item prepend-icon="mdi-rocket-launch" title="启动" value="home" to="/" rounded="lg"></v-list-item>
        <v-list-item prepend-icon="mdi-download" title="下载" value="download" to="/download" rounded="lg"></v-list-item>
      </v-list>

      <template v-slot:append>
        <v-list nav>
          <v-list-item prepend-icon="mdi-cog" title="设置" value="settings" to="/settings" rounded="lg"></v-list-item>
        </v-list>
      </template>
    </v-navigation-drawer>

    <v-app-bar class="titlebar" data-tauri-drag-region elevation="0">
      <v-app-bar-nav-icon @click="rail = !rail" data-tauri-no-drag></v-app-bar-nav-icon>
      <v-toolbar-title class="font-weight-bold">Ar1s Launcher</v-toolbar-title>
      <v-spacer></v-spacer>
      <v-btn icon data-tauri-no-drag @click="toggleTheme" class="theme-toggle-btn" :color="isDarkMode ? 'amber' : 'indigo'">
        <v-icon>{{ isDarkMode ? 'mdi-weather-sunny' : 'mdi-weather-night' }}</v-icon>
      </v-btn>
      <v-btn icon data-tauri-no-drag @click="window.minimize()" class="window-control-btn">
        <v-icon>mdi-window-minimize</v-icon>
      </v-btn>
      <v-btn icon data-tauri-no-drag @click="window.toggleMaximize()" class="window-control-btn">
        <v-icon>mdi-window-maximize</v-icon>
      </v-btn>
      <v-btn icon data-tauri-no-drag @click="window.close()" class="window-control-btn">
        <v-icon>mdi-close</v-icon>
      </v-btn>
    </v-app-bar>

    <v-main>
      <router-view></router-view>
    </v-main>
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

.titlebar .v-toolbar__content {
  pointer-events: none;
}

.titlebar .v-btn,
.titlebar .v-app-bar-nav-icon {
  pointer-events: auto;
}

.theme-toggle-btn {
  margin: 0 4px;
  transition: transform 0.3s ease, color 0.3s ease;
}

.theme-toggle-btn:hover {
  transform: rotate(30deg);
}

.window-control-btn {
  transition: background-color 0.2s ease;
}

/* MD3 风格自定义 */
.v-theme--light {
  --v-theme-primary: #6750a4;
  --v-theme-secondary: #625b71;
  --v-theme-surface: #f5f5f8;
  --v-theme-surface-variant: #e7e0ec;
  --v-theme-on-surface: #1c1b1f;
  --v-theme-on-surface-variant: #49454f;
  --v-theme-background: #f0f0f4;
}

.v-theme--dark {
  --v-theme-primary: #d0bcff;
  --v-theme-secondary: #ccc2dc;
  --v-theme-surface: #1c1b1f;
  --v-theme-surface-variant: #49454f;
  --v-theme-on-surface: #e6e1e5;
  --v-theme-on-surface-variant: #cac4d0;
  --v-theme-background: #1c1b1f;
}

/* MD3 圆角和阴影 */
.v-btn {
  transition: all 0.2s ease;
}

.v-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

/* 动画过渡效果 */
.v-application {
  transition: background-color 0.3s ease;
}

/* 动画过渡效果 */
.v-application {
  transition: background-color 0.3s ease;
}

/* Center icon using user's padding suggestion */
.v-navigation-drawer--rail .v-list-item {
  padding-inline: 12px !important;
}

.v-navigation-drawer--rail .v-list-item .v-list-item-title {
  display: none;
}
</style>