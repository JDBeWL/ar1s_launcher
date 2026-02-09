<script setup lang="ts">
import { Window } from '@tauri-apps/api/window'
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useTheme } from 'vuetify'
import { useDownloadStore } from './stores/downloadStore'
import { useLauncherStore } from './stores/launcherStore'
import GlobalDownloadStatus from './components/GlobalDownloadStatus.vue'
import GlobalNotification from './components/GlobalNotification.vue'

// 窗口控制
const appWindow = Window.getCurrent()
const windowControls = {
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
const themePalette = ref(localStorage.getItem('themePalette') || 'indigo')

const router = useRouter()

function isTypingTarget(target: EventTarget | null) {
  if (!target || !(target instanceof HTMLElement)) return false
  const tag = target.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || target.isContentEditable
}

function handleGlobalShortcut(event: KeyboardEvent) {
  if (isTypingTarget(event.target)) return
  const isCommand = event.ctrlKey || event.metaKey
  if (!isCommand || event.altKey) return

  const key = event.key.toLowerCase()
  if (key === ',') {
    event.preventDefault()
    router.push('/settings')
  } else if (key === 'd') {
    event.preventDefault()
    router.push('/download')
  } else if (key === 'n') {
    event.preventDefault()
    router.push('/add-instance')
  } else if (key === 'i') {
    event.preventDefault()
    router.push('/instance-manager')
  }
}

function getThemeName() {
  return `${isDarkMode.value ? 'dark' : 'light'}-${themePalette.value}`
}

function applyTheme() {
  theme.global.name.value = getThemeName()
  localStorage.setItem('theme', isDarkMode.value ? 'dark' : 'light')
}

// 切换主题模式
function toggleTheme() {
  // 临时禁用所有过渡效果
  const html = document.documentElement
  html.classList.add('no-transition')
  document.body.classList.add('no-transition')
  
  isDarkMode.value = !isDarkMode.value
  applyTheme()
  
  // 强制重绘后移除禁用类
  // 使用 setTimeout 确保浏览器有足够时间应用样式
  setTimeout(() => {
    html.classList.remove('no-transition')
    document.body.classList.remove('no-transition')
  }, 50)
}

function handlePaletteChange(event: Event) {
  const nextPalette = (event as CustomEvent<string>).detail
  if (nextPalette) {
    themePalette.value = nextPalette
    applyTheme()
  }
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
  applyTheme()

  window.addEventListener('keydown', handleGlobalShortcut)
  window.addEventListener('theme-palette-changed', handlePaletteChange)
})

// 清理监听器防止内存泄漏
onUnmounted(() => {
  downloadStore.unsubscribe()
  launcherStore.unsubscribe()
  window.removeEventListener('keydown', handleGlobalShortcut)
  window.removeEventListener('theme-palette-changed', handlePaletteChange)
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
      <v-btn icon data-tauri-no-drag @click="windowControls.minimize()" variant="text">
        <v-icon size="20">mdi-minus</v-icon>
      </v-btn>
      <v-btn icon data-tauri-no-drag @click="windowControls.toggleMaximize()" variant="text">
        <v-icon size="18">mdi-square-outline</v-icon>
      </v-btn>
      <v-btn icon data-tauri-no-drag @click="windowControls.close()" variant="text" class="close-btn">
        <v-icon size="20">mdi-close</v-icon>
      </v-btn>
    </v-app-bar>

    <v-main>
      <router-view v-slot="{ Component }">
        <transition name="fade-slide" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
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

/* 让 v-main 成为滚动容器，滚动条紧贴内容区域 */
.v-main {
  height: 100vh;
  overflow-y: auto;
}

/* Scrollbar styling */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(var(--v-theme-on-surface), 0.2);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(var(--v-theme-on-surface), 0.35);
}

/* Titlebar styles */
.titlebar .v-toolbar__content {
  pointer-events: none;
}

.titlebar .v-btn,
.titlebar .v-app-bar-nav-icon {
  pointer-events: auto;
}

/* Navigation list */
.nav-list {
  padding: 8px;
}

/* Navigation item
 * padding 0 12px → 清除 Vuetify 默认 4px 垂直 padding，水平 12px
 * icon center = 12px padding + 12px half-icon = 24px
 * Rail item = 64 - 8*2 = 48px → center = 24px ✓
 * 两种模式布局完全一致，过渡无跳变 */
.v-list-item.nav-item {
  --v-list-prepend-gap: 0;
  padding: 0 12px;
  min-height: 48px;
  margin-bottom: 4px;
  overflow: hidden;
}

.nav-item > .v-list-item__prepend {
  margin-inline-end: 12px;
}

.nav-item.v-list-item--active {
  background: rgb(var(--v-theme-secondary-container));
  color: rgb(var(--v-theme-on-secondary-container));
}

.nav-item.v-list-item--active .v-icon {
  color: rgb(var(--v-theme-on-secondary-container));
}

/* Rail mode: 只需隐藏文字，布局与展开模式完全一致 */
.v-navigation-drawer--rail .nav-item > .v-list-item__content {
  display: none;
}

/* Theme toggle animation */
.theme-toggle-btn {
  transform: rotate(0deg);
}

.theme-toggle-btn:hover {
  transform: rotate(30deg);
}

/* Close button hover */
.titlebar .close-btn:hover {
  background-color: rgb(var(--v-theme-error));
  color: rgb(var(--v-theme-on-error));
}

/* MD3 Surface tones */
.v-app .surface-container {
  background-color: rgb(var(--v-theme-surface-container));
}

.v-app .surface-container-high {
  background-color: rgb(var(--v-theme-surface-container-high));
}

.v-app .surface-container-highest {
  background-color: rgb(var(--v-theme-surface-container-highest));
}

/* Route transition */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
