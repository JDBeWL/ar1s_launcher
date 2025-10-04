<script setup lang="ts">
import { computed } from 'vue';
import { useDownloadStore } from '../stores/downloadStore';
import NotificationDownload from './NotificationDownload.vue';

const downloadStore = useDownloadStore();
const isDownloading = computed(() => downloadStore.isDownloading);
const selectedVersion = computed(() => downloadStore.selectedVersion);
const downloadProgress = computed(() => downloadStore.downloadProgress);
const showNotification = computed({
  get: () => downloadStore.showNotification,
  set: (value) => {
    if (value) {
      downloadStore.showDownloadNotification();
    } else {
      downloadStore.hideDownloadNotification();
    }
  }
});

// 取消下载
function cancelDownload() {
  downloadStore.cancelDownload();
}
</script>

<template>
  <div>
    <!-- 下载通知组件 -->
    <NotificationDownload
      v-if="isDownloading || downloadProgress.status === 'completed'"
      v-model="showNotification"
      :version="selectedVersion"
      :progress="downloadProgress.progress"
      :total="downloadProgress.total"
      :speed="downloadProgress.speed"
      :status="downloadProgress.status"
      @cancel="cancelDownload"
    />
    
    <!-- 悬浮按钮，用于重新显示已隐藏的下载通知 -->
    <v-btn
      v-if="(isDownloading || downloadProgress.status === 'completed') && !showNotification"
      icon
      color="primary"
      size="large"
      class="download-fab"
      @click="showNotification = true"
      title="显示下载状态"
    >
      <v-icon size="large">mdi-download</v-icon>
    </v-btn>
  </div>
</template>

<style scoped>
.download-fab {
  position: fixed;
  bottom: 20px;
  right: 20px;
  z-index: 999;
  width: 56px !important;
  height: 56px !important;
  min-width: 56px !important;
}
</style>