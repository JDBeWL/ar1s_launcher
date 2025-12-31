<script setup lang="ts">
import { ref } from 'vue';
import GeneralSettings from "../components/settings/GeneralSettings.vue";
import JavaSettings from "../components/settings/JavaSettings.vue";
import MemorySettings from "../components/settings/MemorySettings.vue";
import WindowSettings from "../components/settings/WindowSettings.vue";

const activeSection = ref('general');

const sections = [
  { id: 'general', title: '常规设置', icon: 'mdi-cog-outline' },
  { id: 'java', title: 'Java 配置', icon: 'mdi-language-java' },
  { id: 'memory', title: '内存管理', icon: 'mdi-memory' },
  { id: 'window', title: '窗口设置', icon: 'mdi-monitor' },
];

function scrollToSection(sectionId: string) {
  activeSection.value = sectionId;
  const element = document.getElementById(`section-${sectionId}`);
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
}
</script>

<template>
  <div class="settings-container">
    <!-- 左侧导航 -->
    <div class="settings-nav">
      <div class="nav-header pa-4">
        <h1 class="text-h5 font-weight-bold">设置</h1>
        <p class="text-body-2 text-on-surface-variant mt-1">管理启动器配置</p>
      </div>
      
      <v-list nav density="comfortable" class="px-2" bg-color="transparent">
        <v-list-item
          v-for="section in sections"
          :key="section.id"
          :active="activeSection === section.id"
          :prepend-icon="section.icon"
          :title="section.title"
          class="mb-1 settings-nav-item"
          @click="scrollToSection(section.id)"
        />
      </v-list>
    </div>

    <!-- 右侧内容 -->
    <div class="settings-content">
      <div class="content-wrapper pa-6">
        <!-- 常规设置 -->
        <section id="section-general" class="settings-section mb-8">
          <GeneralSettings />
        </section>

        <!-- Java 配置 -->
        <section id="section-java" class="settings-section mb-8">
          <JavaSettings />
        </section>

        <!-- 内存管理 -->
        <section id="section-memory" class="settings-section mb-8">
          <MemorySettings />
        </section>

        <!-- 窗口设置 -->
        <section id="section-window" class="settings-section mb-8">
          <WindowSettings />
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-container {
  display: flex;
  height: calc(100vh - 64px); /* 减去顶部标题栏高度 */
  overflow: hidden;
  position: fixed;
  top: 64px;
  left: 64px; /* 左侧导航栏宽度 */
  right: 0;
}

.settings-nav {
  width: 200px;
  min-width: 200px;
  background-color: rgb(var(--v-theme-surface-container));
  height: 100%;
  flex-shrink: 0;
  overflow-y: auto;
}

.nav-header {
  border-bottom: 1px solid rgb(var(--v-theme-outline-variant));
}

.settings-nav-item.v-list-item--active {
  background: rgb(var(--v-theme-secondary-container));
  color: rgb(var(--v-theme-on-secondary-container));
}

.settings-content {
  flex: 1;
  height: 100%;
  overflow-y: auto;
}

.content-wrapper {
  max-width: 900px;
}

.settings-section {
  scroll-margin-top: 24px;
}
</style>
