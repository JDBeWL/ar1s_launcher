<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import GeneralSettings from "../components/settings/GeneralSettings.vue";
import JavaSettings from "../components/settings/JavaSettings.vue";
import MemorySettings from "../components/settings/MemorySettings.vue";
import WindowSettings from "../components/settings/WindowSettings.vue";

const activeSection = ref('general');
const contentRef = ref<HTMLElement | null>(null);
const isScrollingByClick = ref(false);

const sections = [
  { id: 'general', title: '常规设置', icon: 'mdi-cog-outline' },
  { id: 'java', title: 'Java 配置', icon: 'mdi-language-java' },
  { id: 'memory', title: '内存管理', icon: 'mdi-memory' },
  { id: 'window', title: '窗口设置', icon: 'mdi-monitor' },
];

function scrollToSection(sectionId: string) {
  isScrollingByClick.value = true;
  activeSection.value = sectionId;
  const element = document.getElementById(`section-${sectionId}`);
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' });
    // 等待平滑滚动完成后再恢复 scroll spy
    setTimeout(() => {
      isScrollingByClick.value = false;
    }, 600);
  }
}

// Scroll spy：监听右侧内容滚动，自动高亮对应的左侧导航项
function handleScroll() {
  if (isScrollingByClick.value) return;

  const container = contentRef.value;
  if (!container) return;

  const scrollTop = container.scrollTop;
  const containerHeight = container.clientHeight;

  // 如果滚到底部，高亮最后一个
  if (scrollTop + containerHeight >= container.scrollHeight - 10) {
    activeSection.value = sections[sections.length - 1].id;
    return;
  }

  // 找到当前可见区域中最靠上的 section
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
  nextTick(() => {
    contentRef.value?.addEventListener('scroll', handleScroll, { passive: true });
  });
});

onUnmounted(() => {
  contentRef.value?.removeEventListener('scroll', handleScroll);
});
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
    <div ref="contentRef" class="settings-content">
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
  height: calc(100vh - 64px); /* 减去顶部 app bar 高度 */
  overflow: hidden;
}

.settings-nav {
  width: 220px;
  min-width: 220px;
  background-color: rgb(var(--v-theme-surface-container));
  flex-shrink: 0;
  overflow-y: auto;
  margin: 12px;
  margin-right: 0;
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
</style>
