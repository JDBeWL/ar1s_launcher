<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, defineAsyncComponent } from 'vue';

const GeneralSettings = defineAsyncComponent(() => import('../components/settings/GeneralSettings.vue'));
const AuthSettings = defineAsyncComponent(() => import('../components/settings/AuthSettings.vue'));
const JavaSettings = defineAsyncComponent(() => import('../components/settings/JavaSettings.vue'));
const MemorySettings = defineAsyncComponent(() => import('../components/settings/MemorySettings.vue'));
const WindowSettings = defineAsyncComponent(() => import('../components/settings/WindowSettings.vue'));
const AdvancedSettings = defineAsyncComponent(() => import('../components/settings/AdvancedSettings.vue'));

const activeSection = ref('general');
const isSmallScreen = ref(false);

const sections = [
  { id: 'general', title: '常规设置', icon: 'mdi-cog-outline' },
  { id: 'auth', title: '账户与认证', icon: 'mdi-account-circle-outline' },
  { id: 'java', title: 'Java 配置', icon: 'mdi-language-java' },
  { id: 'memory', title: '内存管理', icon: 'mdi-memory' },
  { id: 'window', title: '窗口设置', icon: 'mdi-monitor' },
  { id: 'advanced', title: '高级选项', icon: 'mdi-tune-variant' },
];

const activeTab = computed({
  get: () => activeSection.value,
  set: (val: string) => {
    activeSection.value = val;
  },
});

function checkScreenSize() {
  isSmallScreen.value = window.innerWidth < 768;
}

onMounted(() => {
  checkScreenSize();
  window.addEventListener('resize', checkScreenSize);
});

onUnmounted(() => {
  window.removeEventListener('resize', checkScreenSize);
});
</script>

<template>
  <div class="settings-container">
    <!-- 小屏幕：顶部标签页 -->
    <template v-if="isSmallScreen">
      <div class="settings-mobile">
        <div class="pa-4 pb-0">
          <h1 class="text-headline-small font-weight-bold">设置</h1>
          <p class="text-body-medium text-on-surface-variant mt-1">管理启动器配置</p>
        </div>
        <v-tabs v-model="activeTab" color="primary" density="comfortable" class="px-2" show-arrows>
          <v-tab
            v-for="section in sections"
            :key="section.id"
            :value="section.id"
            :prepend-icon="section.icon"
            size="small"
          >
            {{ section.title }}
          </v-tab>
        </v-tabs>

        <div class="settings-mobile-content pa-4">
          <GeneralSettings v-if="activeSection === 'general'" />
          <AuthSettings v-else-if="activeSection === 'auth'" />
          <JavaSettings v-else-if="activeSection === 'java'" />
          <MemorySettings v-else-if="activeSection === 'memory'" />
          <WindowSettings v-else-if="activeSection === 'window'" />
          <AdvancedSettings v-else-if="activeSection === 'advanced'" />
        </div>
      </div>
    </template>

    <!-- 大屏幕：左侧导航 + 右侧内容 -->
    <template v-else>
      <div class="settings-nav">
        <div class="nav-header pa-4">
          <h1 class="text-headline-small font-weight-bold">设置</h1>
          <p class="text-body-medium text-on-surface-variant mt-1">管理启动器配置</p>
        </div>

        <v-list nav density="comfortable" class="px-2" bg-color="transparent">
          <v-list-item
            v-for="section in sections"
            :key="section.id"
            :active="activeSection === section.id"
            :prepend-icon="section.icon"
            :title="section.title"
            class="mb-1 settings-nav-item"
            @click="activeSection = section.id"
          />
        </v-list>
      </div>

      <div class="settings-content">
        <div class="content-wrapper pa-6">
          <GeneralSettings v-if="activeSection === 'general'" />
          <AuthSettings v-else-if="activeSection === 'auth'" />
          <JavaSettings v-else-if="activeSection === 'java'" />
          <MemorySettings v-else-if="activeSection === 'memory'" />
          <WindowSettings v-else-if="activeSection === 'window'" />
          <AdvancedSettings v-else-if="activeSection === 'advanced'" />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.settings-container {
  display: flex;
  height: calc(100vh - 64px);
  overflow: hidden;
}

.settings-mobile {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.settings-mobile-content {
  flex: 1;
  overflow-y: auto;
}

.settings-nav {
  width: 220px;
  min-width: 220px;
  background-color: rgb(var(--v-theme-surface-container));
  overflow-y: auto;
  margin: 12px 0 12px 12px;
  border-radius: 16px;
  max-height: calc(100vh - 64px - 24px);
}

.nav-header {
  border-bottom: 1px solid rgb(var(--v-theme-outline-variant));
  border-radius: 16px 16px 0 0;
}

.settings-nav-item.v-list-item--active {
  background: rgb(var(--v-theme-secondary-container));
  color: rgb(var(--v-theme-on-secondary-container));
}

.settings-content {
  flex: 1;
  overflow-y: auto;
}

.content-wrapper {
  max-width: 900px;
}
</style>

<style>
.settings-group {
  margin-bottom: 32px;
}

/* v-alert 默认把 prepend 图标垂直居中于整块内容，
   多行提示里图标会飘到中间，改成贴住第一行标题 */
.tip-alert .v-alert__prepend {
  align-self: flex-start;
}
</style>
