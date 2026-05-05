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

const progressPercentage = computed(() => {
  if (downloadProgress.value.total_bytes === 0) return 0
  return Math.min((downloadProgress.value.bytes_downloaded / downloadProgress.value.total_bytes) * 100, 100)
})

function cancelDownload() {
  downloadStore.cancelDownload();
}
</script>

<template>
  <div>
    <NotificationDownload
      v-if="isDownloading || downloadProgress.status === 'completed'"
      v-model="showNotification"
      :version="selectedVersion"
      :progress="downloadProgress.bytes_downloaded"
      :total="downloadProgress.total_bytes"
      :speed="downloadProgress.speed"
      :status="downloadProgress.status"
      @cancel="cancelDownload"
    />
    
    <v-btn
      v-if="(isDownloading || downloadProgress.status === 'completed') && !showNotification"
      icon
      color="primary"
      size="large"
      class="download-fab"
      @click="showNotification = true"
    >
      <v-icon size="large">mdi-download</v-icon>
      <v-progress-circular
        v-if="isDownloading"
        :model-value="progressPercentage"
        size="52"
        width="3"
        color="primary-container"
        class="download-fab-progress"
      />
      <v-tooltip activator="parent" location="left">
        {{ isDownloading ? `下载中 ${progressPercentage.toFixed(0)}%` : '下载完成' }}
      </v-tooltip>
    </v-btn>
  </div>
</template>

<style scoped>
.download-fab.v-btn {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 999;
  width: 56px;
  height: 56px;
  min-width: 56px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
}

.download-fab-progress {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  pointer-events: none;
}
</style>