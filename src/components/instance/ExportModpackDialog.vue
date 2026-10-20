<script setup lang="ts">
/**
 * 导出整合包对话框
 * 用原生保存对话框选目标，后端把实例内容整理成 Modrinth 的 .mrpack
 */
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { save } from '@tauri-apps/plugin-dialog';
import { modpackApi } from '../../services';
import { useNotificationStore } from '../../stores/notificationStore';
import { formatFileSize, getErrorMessage } from '../../utils/format';
import { logError } from '../../utils/logger';
import type { GameInstance } from '../../types/events';

const props = defineProps<{
  modelValue: boolean;
  instance: GameInstance | null;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
}>();

const notificationStore = useNotificationStore();
const { t } = useI18n();

const name = ref('');
const version = ref('1.0.0');
const summary = ref('');
const includeSaves = ref(false);
const exporting = ref(false);
const error = ref('');

const canExport = computed(() => {
  return Boolean(props.instance) && name.value.trim().length > 0 && version.value.trim().length > 0;
});

/** 默认文件名只取安全字符，避免对话框直接给出非法路径 */
function defaultFileName(): string {
  const safe = (props.instance?.name || 'modpack').replace(/[\\/:*?"<>|]/g, '_');
  return `${safe}-${version.value.trim() || '1.0.0'}.mrpack`;
}

watch(
  () => props.modelValue,
  (open) => {
    if (!open) return;
    error.value = '';
    exporting.value = false;
    name.value = props.instance?.name || '';
    version.value = '1.0.0';
    summary.value = '';
    includeSaves.value = false;
  },
);

async function exportModpack() {
  if (!props.instance || exporting.value) return;

  if (!name.value.trim()) {
    error.value = t('exportModpack.nameRequired');
    return;
  }
  if (!version.value.trim()) {
    error.value = t('exportModpack.versionRequired');
    return;
  }

  error.value = '';
  exporting.value = true;

  try {
    const selected = await save({
      title: t('exportModpack.saveTitle'),
      defaultPath: defaultFileName(),
      filters: [{ name: t('exportModpack.saveFilter'), extensions: ['mrpack'] }],
    });
    if (!selected) {
      exporting.value = false;
      return;
    }

    const target = selected.toLowerCase().endsWith('.mrpack') ? selected : `${selected}.mrpack`;
    const result = await modpackApi.exportModpack(props.instance.name, {
      name: name.value.trim(),
      version: version.value.trim(),
      summary: summary.value.trim() || undefined,
      targetPath: target,
      includeSaves: includeSaves.value,
    });

    notificationStore.success(
      t('exportModpack.success'),
      t('exportModpack.successHint', {
        linked: result.linkedFiles,
        overrides: result.overrideFiles,
        size: formatFileSize(result.totalBytes),
      }),
    );
    emit('update:modelValue', false);
  } catch (e) {
    logError('Failed to export modpack', e, 'ExportModpackDialog');
    error.value = getErrorMessage(e);
    notificationStore.error(t('exportModpack.failed'), getErrorMessage(e), true);
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="560"
    :persistent="exporting"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card color="surface-container-high">
      <v-card-title class="pa-4 pb-2">
        <div class="text-title-large font-weight-bold">{{ t('exportModpack.title') }}</div>
        <div class="text-body-small text-on-surface-variant mt-1">{{ instance?.name }}</div>
      </v-card-title>

      <v-card-text class="pa-4 pt-2">
        <div class="text-body-small text-on-surface-variant mb-4">{{ t('exportModpack.hint') }}</div>

        <v-text-field
          v-model="name"
          :label="t('exportModpack.nameLabel')"
          density="comfortable"
          variant="outlined"
          hide-details
          :disabled="exporting"
          class="mb-3"
        />

        <v-text-field
          v-model="version"
          :label="t('exportModpack.versionLabel')"
          :hint="t('exportModpack.versionHint')"
          persistent-hint
          density="comfortable"
          variant="outlined"
          :disabled="exporting"
          class="mb-5"
        />

        <v-text-field
          v-model="summary"
          :label="t('exportModpack.summaryLabel')"
          :placeholder="t('exportModpack.summaryPlaceholder')"
          density="comfortable"
          variant="outlined"
          hide-details
          :disabled="exporting"
          class="mb-3"
        />

        <v-switch
          v-model="includeSaves"
          :label="t('exportModpack.includeSaves')"
          :hint="t('exportModpack.includeSavesHint')"
          persistent-hint
          color="primary"
          density="compact"
          hide-details
          :disabled="exporting"
        />

        <div v-if="error" class="text-body-small text-error mt-4">{{ error }}</div>
      </v-card-text>

      <v-card-actions class="pa-4 pt-0">
        <v-spacer />
        <v-btn variant="text" :disabled="exporting" @click="emit('update:modelValue', false)">
          {{ t('common.cancel') }}
        </v-btn>
        <v-btn color="primary" :disabled="!canExport" :loading="exporting" @click="exportModpack">
          {{ exporting ? t('exportModpack.exporting') : t('exportModpack.start') }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
