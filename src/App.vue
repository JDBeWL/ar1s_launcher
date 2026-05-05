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
  } else if (key === '/') {
    event.preventDefault()
    shortcutHelpVisible.value = !shortcutHelpVisible.value
  }
}

function getThemeName() {
  return `${isDarkMode.value ? 'dark' : 'light'}-${themePalette.value}`
}

function applyTheme() {
  theme.change(getThemeName())
  localStorage.setItem('theme', isDarkMode.value ? 'dark' : 'light')
}

// 切换主题模式
function toggleTheme() {
  isDarkMode.value = !isDarkMode.value
  applyTheme()
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

const shortcutHelpVisible = ref(false)
const shortcuts = [
  { keys: 'Ctrl + ,', description: '打开设置' },
  { keys: 'Ctrl + D', description: '打开下载' },
  { keys: 'Ctrl + N', description: '添加实例' },
  { keys: 'Ctrl + I', description: '实例管理' },
  { keys: 'Ctrl + /', description: '快捷键帮助' },
]

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
        >
          <v-tooltip activator="parent" location="right" :disabled="!rail">启动</v-tooltip>
        </v-list-item>
        <v-list-item 
          prepend-icon="mdi-download" 
          title="下载" 
          to="/download" 
          class="nav-item mb-1"
        >
          <v-tooltip activator="parent" location="right" :disabled="!rail">
            下载 <kbd class="shortcut-key">Ctrl+D</kbd>
          </v-tooltip>
        </v-list-item>
        <v-list-item 
          prepend-icon="mdi-plus-circle-outline" 
          title="添加实例" 
          to="/add-instance" 
          class="nav-item mb-1"
        >
          <v-tooltip activator="parent" location="right" :disabled="!rail">
            添加实例 <kbd class="shortcut-key">Ctrl+N</kbd>
          </v-tooltip>
        </v-list-item>
        <v-list-item 
          prepend-icon="mdi-folder-multiple-outline" 
          title="实例管理" 
          to="/instance-manager" 
          class="nav-item"
        >
          <v-tooltip activator="parent" location="right" :disabled="!rail">
            实例管理 <kbd class="shortcut-key">Ctrl+I</kbd>
          </v-tooltip>
        </v-list-item>
      </v-list>

      <template v-slot:append>
        <v-list nav class="nav-list">
          <v-list-item 
            prepend-icon="mdi-cog-outline" 
            title="设置" 
            to="/settings" 
            class="nav-item"
          >
            <v-tooltip activator="parent" location="right" :disabled="!rail">
              设置 <kbd class="shortcut-key">Ctrl+,</kbd>
            </v-tooltip>
          </v-list-item>
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
      <v-btn icon data-tauri-no-drag @click="windowControls.minimize()" variant="text" class="window-control-btn">
        <v-icon size="20">mdi-minus</v-icon>
      </v-btn>
      <v-btn icon data-tauri-no-drag @click="windowControls.toggleMaximize()" variant="text" class="window-control-btn">
        <v-icon size="18">mdi-square-outline</v-icon>
      </v-btn>
      <v-btn icon data-tauri-no-drag @click="windowControls.close()" variant="text" class="close-btn">
        <v-icon size="20">mdi-close</v-icon>
      </v-btn>
    </v-app-bar>

    <v-main>
      <router-view v-slot="{ Component }">
        <component :is="Component" />
      </router-view>
    </v-main>
    
    <!-- 全局下载状态组件 -->
    <GlobalDownloadStatus />
    <!-- 全局通知组件 -->
    <GlobalNotification />

    <!-- 快捷键帮助面板 -->
    <v-dialog v-model="shortcutHelpVisible" max-width="380">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-h6 font-weight-bold mb-4">快捷键</div>
          <div class="shortcut-list">
            <div v-for="s in shortcuts" :key="s.keys" class="shortcut-row">
              <span class="text-body-2">{{ s.description }}</span>
              <kbd class="shortcut-key-lg">{{ s.keys }}</kbd>
            </div>
          </div>
        </v-card-text>
        <v-card-actions class="pa-4 pt-0">
          <v-spacer />
          <v-btn variant="text" @click="shortcutHelpVisible = false">关闭</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-app>
</template>

<style>
:root {
  color-scheme: light dark;
}

html, body {
  margin: 0;
  padding: 0;
  overflow: hidden;
  background-color: rgb(var(--v-theme-background));
}

.v-application {
  background-color: rgb(var(--v-theme-background)) !important;
}

.v-main {
  --v-layout-top: 0px !important;
  height: calc(100vh - 64px);
  margin-top: 64px;
  overflow-y: auto;
  overflow-x: hidden;
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

/* Navigation item */
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

/* Window control button hover */
.titlebar .window-control-btn:hover {
  background-color: rgba(var(--v-theme-on-surface), 0.08);
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

/* Shortcut key styles */
.shortcut-key {
  display: inline-block;
  padding: 1px 5px;
  margin-left: 6px;
  font-size: 11px;
  font-family: inherit;
  line-height: 1.4;
  background: rgba(var(--v-theme-on-surface), 0.12);
  border-radius: 4px;
  color: rgba(var(--v-theme-on-surface), 0.7);
}

.shortcut-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.shortcut-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.shortcut-key-lg {
  display: inline-block;
  padding: 2px 8px;
  font-size: 12px;
  font-family: inherit;
  line-height: 1.5;
  background: rgba(var(--v-theme-on-surface), 0.08);
  border: 1px solid rgba(var(--v-theme-on-surface), 0.15);
  border-radius: 6px;
  color: rgba(var(--v-theme-on-surface), 0.8);
}
</style>