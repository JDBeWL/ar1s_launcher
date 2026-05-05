<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed, defineAsyncComponent } from 'vue';

const GeneralSettings = defineAsyncComponent(() => import('../components/settings/GeneralSettings.vue'));
const AuthSettings = defineAsyncComponent(() => import('../components/settings/AuthSettings.vue'));
const JavaSettings = defineAsyncComponent(() => import('../components/settings/JavaSettings.vue'));
const MemorySettings = defineAsyncComponent(() => import('../components/settings/MemorySettings.vue'));
const WindowSettings = defineAsyncComponent(() => import('../components/settings/WindowSettings.vue'));

const activeSection = ref('general');
const contentRef = ref<HTMLElement | null>(null);
const isScrollingByClick = ref(false);
const isSmallScreen = ref(false);

const sections = [
  { id: 'general', title: '常规设置', icon: 'mdi-cog-outline' },
  { id: 'auth', title: '账户与认证', icon: 'mdi-account-circle-outline' },
  { id: 'java', title: 'Java 配置', icon: 'mdi-language-java' },
  { id: 'memory', title: '内存管理', icon: 'mdi-memory' },
  { id: 'window', title: '窗口设置', icon: 'mdi-monitor' },
];

const activeTab = computed({
  get: () => activeSection.value,
  set: (val: string) => scrollToSection(val)
});

function checkScreenSize() {
  isSmallScreen.value = window.innerWidth < 768;
}

function scrollToSection(sectionId: string) {
  isScrollingByClick.value = true;
  activeSection.value = sectionId;

  if (isSmallScreen.value) return;

  const element = document.getElementById(`section-${sectionId}`);
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    setTimeout(() => {
      isScrollingByClick.value = false;
    }, 600);
  } else {
    isScrollingByClick.value = false;
  }
}

function handleScroll() {
  if (isScrollingByClick.value || isSmallScreen.value) return;

  const container = contentRef.value;
  if (!container) return;

  const scrollTop = container.scrollTop;
  const containerHeight = container.clientHeight;

  if (scrollTop + containerHeight >= container.scrollHeight - 10) {
    activeSection.value = sections[sections.length - 1].id;
    return;
  }

  for (let i = sections.length - 1; i >= 0; i--) {
    const el = document.getElementById(`section-${sections[i].id}`);
    if (el) {
      const offsetTop = el.offsetTop - container.offsetTop;
      if (scrollTop >= offsetTop - 60) {
        activeSection.value = sections[i].id;
        return;
      }
    }
  }

  activeSection.value = sections[0].id;
}

onMounted(() => {
  checkScreenSize();
  window.addEventListener('resize', checkScreenSize);
  nextTick(() => {
    contentRef.value?.addEventListener('scroll', handleScroll, { passive: true });
  });
});

onUnmounted(() => {
  window.removeEventListener('resize', checkScreenSize);
  contentRef.value?.removeEventListener('scroll', handleScroll);
});
</script>

<template>
  <div class="settings-container">
    <!-- 小屏幕：顶部标签页 -->
    <template v-if="isSmallScreen">
      <div class="settings-mobile">
        <div class="pa-4 pb-0">
          <h1 class="text-h5 font-weight-bold">设置</h1>
          <p class="text-body-2 text-on-surface-variant mt-1">管理启动器配置</p>
        </div>
        <v-tabs
          v-model="activeTab"
          color="primary"
          density="comfortable"
          class="px-2"
          show-arrows
        >
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
        </div>
      </div>
    </template>

    <!-- 大屏幕：左侧导航 + 右侧内容 -->
    <template v-else>
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

      <div ref="contentRef" class="settings-content">
        <div class="content-wrapper pa-6">
          <section id="section-general" class="settings-section mb-8">
            <GeneralSettings />
          </section>

          <section id="section-auth" class="settings-section mb-8">
            <AuthSettings />
          </section>

          <section id="section-java" class="settings-section mb-8">
            <JavaSettings />
          </section>

          <section id="section-memory" class="settings-section mb-8">
            <MemorySettings />
          </section>

          <section id="section-window" class="settings-section mb-8">
            <WindowSettings />
          </section>
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
  scroll-behavior: smooth;
}

.content-wrapper {
  max-width: 900px;
}

.settings-section {
  scroll-margin-top: 24px;
}

@media (max-width: 767px) {
  .settings-container {
    flex-direction: column;
  }
}
</style>

<style>
.settings-group {
  margin-bottom: 32px;
}

.group-header {
  padding-bottom: 16px;
  border-bottom: 1px solid rgb(var(--v-theme-outline-variant));
}
</style>
