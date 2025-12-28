<script setup lang="ts">
import { ref } from 'vue';
import GeneralSettings from "../components/settings/GeneralSettings.vue";
import JavaSettings from "../components/settings/JavaSettings.vue";
import MemorySettings from "../components/settings/MemorySettings.vue";

const activeSection = ref('general');

const sections = [
  { id: 'general', title: '常规设置', icon: 'mdi-cog-outline' },
  { id: 'java', title: 'Java 配置', icon: 'mdi-language-java' },
  { id: 'memory', title: '内存管理', icon: 'mdi-memory' },
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
  <v-container fluid class="settings-container pa-0">
    <v-row no-gutters class="fill-height">
      <!-- 左侧导航 -->
      <v-col cols="12" md="3" lg="2" class="settings-nav">
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
      </v-col>

      <!-- 右侧内容 -->
      <v-col cols="12" md="9" lg="10" class="settings-content">
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
        </div>
      </v-col>
    </v-row>
  </v-container>
</template>

<style scoped>
.settings-container {
  height: 100%;
  overflow: hidden;
}

.settings-nav {
  background-color: rgb(var(--v-theme-surface-container));
  height: 100%;
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
