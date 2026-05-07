<script setup lang="ts">
import { computed } from 'vue'
import { useNotificationStore } from '../stores/notificationStore'

const store = useNotificationStore()

const currentNotification = computed(() => store.notifications[0])
const hasNotification = computed(() => store.notifications.length > 0)

const snackbarColor = computed(() => {
  if (!currentNotification.value) return 'info'
  return currentNotification.value.type
})

const snackbarIcon = computed(() => {
  if (!currentNotification.value) return 'mdi-information'
  const icons = {
    success: 'mdi-check-circle',
    error: 'mdi-alert-circle',
    warning: 'mdi-alert',
    info: 'mdi-information'
  }
  return icons[currentNotification.value.type]
})

const snackbarTimeout = computed(() => {
  if (!currentNotification.value) return 4000
  if (currentNotification.value.type === 'error') return 6000
  return 4000
})

function closeSnackbar() {
  if (currentNotification.value) {
    store.removeNotification(currentNotification.value.id)
  }
}

function onSnackbarUpdate(value: boolean) {
  if (!value && currentNotification.value) {
    store.removeNotification(currentNotification.value.id)
  }
}

const dialogIcon = computed(() => {
  const icons = {
    success: 'mdi-check-circle',
    error: 'mdi-alert-circle',
    warning: 'mdi-alert',
    info: 'mdi-information'
  }
  return icons[store.dialogType]
})

const dialogColor = computed(() => store.dialogType)

const confirmIcon = computed(() => {
  const icons = {
    success: 'mdi-check-circle',
    error: 'mdi-alert-circle',
    warning: 'mdi-alert',
    info: 'mdi-information'
  }
  return icons[store.confirmType]
})

const confirmColor = computed(() => store.confirmType)

const choiceIcon = computed(() => {
  const icons = {
    success: 'mdi-check-circle',
    error: 'mdi-alert-circle',
    warning: 'mdi-alert',
    info: 'mdi-information'
  }
  return icons[store.choiceType]
})

const choiceColor = computed(() => store.choiceType)
</script>

<template>
  <!-- Snackbar 通知 -->
  <v-snackbar
    :model-value="hasNotification"
    :color="snackbarColor"
    :timeout="snackbarTimeout"
    location="top right"
    elevation="2"
    @update:model-value="onSnackbarUpdate"
  >
    <div class="d-flex align-center">
      <v-icon class="mr-2">{{ snackbarIcon }}</v-icon>
      <div>
        <div class="font-weight-medium">{{ currentNotification?.title }}</div>
        <div v-if="currentNotification?.message" class="text-caption">
          {{ currentNotification.message }}
        </div>
      </div>
    </div>
    <template #actions>
      <v-btn variant="text" @click="closeSnackbar">关闭</v-btn>
    </template>
  </v-snackbar>

  <!-- 详情 Dialog -->
  <v-dialog v-model="store.dialogVisible" max-width="520">
    <v-card>
      <v-card-title :class="`text-${dialogColor}`">
        <v-icon start :color="dialogColor">{{ dialogIcon }}</v-icon>
        {{ store.dialogTitle }}
      </v-card-title>
      <v-card-text>
        <pre class="dialog-content">{{ store.dialogContent }}</pre>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn color="primary" @click="store.closeDialog">确定</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

  <!-- 确认 Dialog -->
  <v-dialog v-model="store.confirmVisible" max-width="420" persistent>
    <v-card>
      <v-card-title :class="`text-${confirmColor}`">
        <v-icon start :color="confirmColor">{{ confirmIcon }}</v-icon>
        {{ store.confirmTitle }}
      </v-card-title>
      <v-card-text>
        {{ store.confirmContent }}
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="store.handleConfirm(false)">取消</v-btn>
        <v-btn color="primary" variant="elevated" @click="store.handleConfirm(true)">确定</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

  <!-- 选择 Dialog -->
  <v-dialog v-model="store.choiceVisible" max-width="560" persistent>
    <v-card>
      <v-card-title class="d-flex align-center justify-space-between py-3 px-4">
        <span :class="`text-${choiceColor} d-flex align-center`">
          <v-icon start :color="choiceColor" class="mr-2">{{ choiceIcon }}</v-icon>
          {{ store.choiceTitle }}
        </span>
        <v-btn
          icon="mdi-close"
          variant="text"
          density="comfortable"
          size="small"
          @click="store.handleChoice(null)"
        />
      </v-card-title>
      <v-divider />
      <v-card-text class="pt-4">
        {{ store.choiceContent }}
      </v-card-text>
      <v-card-actions class="d-flex flex-wrap ga-2 px-4 pb-4">
        <v-spacer class="d-none d-sm-flex" />
        <v-btn
          v-for="opt in store.choiceOptions"
          :key="opt.id"
          :color="opt.color || 'primary'"
          :variant="opt.variant || 'elevated'"
          class="text-none flex-grow-1 flex-sm-grow-0"
          @click="store.handleChoice(opt.id)"
        >
          {{ opt.label }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.dialog-content {
  white-space: pre-wrap;
  word-wrap: break-word;
  font-family: inherit;
  margin: 0;
}
</style>
