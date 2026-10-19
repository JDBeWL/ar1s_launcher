<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { configApi } from '../../services';
import { getErrorMessage } from '../../utils/format';
import type { ProxyMode, ProxySettings } from '../../types/events';

const { t } = useI18n();

const mode = ref<ProxyMode>('system');
const url = ref('');
const saved = ref<ProxySettings>({ mode: 'system', url: null });
const loading = ref(true);
const applying = ref(false);
const testing = ref(false);
const applyError = ref('');
const testResult = ref<{ type: 'success' | 'error'; text: string } | null>(null);

const isCustom = computed(() => mode.value === 'custom');
const trimmedUrl = computed(() => url.value.trim());
const dirty = computed(
  () => mode.value !== saved.value.mode || (isCustom.value && trimmedUrl.value !== (saved.value.url ?? '')),
);
const canApply = computed(() => isCustom.value && trimmedUrl.value.length > 0 && dirty.value && !applying.value);
const canTest = computed(
  () => !testing.value && !loading.value && !applying.value && (!isCustom.value || trimmedUrl.value.length > 0),
);
const modeDescription = computed(() => {
  switch (mode.value) {
    case 'direct':
      return t('settings.proxy.descDirect');
    case 'custom':
      return t('settings.proxy.descCustom');
    default:
      return t('settings.proxy.descSystem');
  }
});

// 改了地址以后旧的测试结论和报错都不再对应当前输入
watch(url, () => {
  testResult.value = null;
  applyError.value = '';
});

async function load() {
  try {
    const settings = await configApi.getProxySettings();
    saved.value = settings;
    mode.value = settings.mode;
    url.value = settings.url ?? '';
  } catch (err) {
    applyError.value = getErrorMessage(err);
  } finally {
    loading.value = false;
  }
}

async function persist(settings: ProxySettings, resetOnError: boolean): Promise<boolean> {
  applying.value = true;
  applyError.value = '';
  try {
    const applied = await configApi.setProxySettings(settings);
    saved.value = applied;
    mode.value = applied.mode;
    url.value = applied.url ?? '';
    return true;
  } catch (err) {
    applyError.value = getErrorMessage(err);
    if (resetOnError) {
      mode.value = saved.value.mode;
      url.value = saved.value.url ?? '';
    }
    return false;
  } finally {
    applying.value = false;
  }
}

async function handleModeChange(value: unknown) {
  if (value !== 'system' && value !== 'direct' && value !== 'custom') return;
  const next = value;
  if (next === mode.value) return;

  mode.value = next;
  applyError.value = '';
  testResult.value = null;

  if (next === 'custom') {
    url.value = saved.value.url ?? url.value;
    return;
  }

  // 切到系统/直连时保留上次的自定义地址，方便切回来继续用
  await persist({ mode: next, url: saved.value.url }, true);
}

async function applyCustom() {
  if (!canApply.value) return;
  await persist({ mode: 'custom', url: url.value }, false);
}

async function testConnection() {
  testing.value = true;
  testResult.value = null;
  try {
    const result = await configApi.testProxy({
      mode: mode.value,
      url: isCustom.value ? url.value : saved.value.url,
    });
    testResult.value = {
      type: 'success',
      text: t('settings.proxy.testSuccess', { latency: result.latencyMs, status: result.status }),
    };
  } catch (err) {
    testResult.value = { type: 'error', text: getErrorMessage(err) };
  } finally {
    testing.value = false;
  }
}

onMounted(load);
</script>

<template>
  <v-card variant="flat" color="surface-container" class="mb-4">
    <v-card-text class="pa-4">
      <div class="d-flex align-center mb-3">
        <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-lan-connect</v-icon>
        <span class="text-title-medium">{{ t('settings.proxy.title') }}</span>
        <v-chip v-if="isCustom && dirty" size="small" color="warning" variant="tonal" class="ml-2">
          {{ t('settings.proxy.notApplied') }}
        </v-chip>
      </div>

      <v-btn-toggle
        :model-value="mode"
        mandatory
        border
        divided
        rounded="pill"
        density="comfortable"
        variant="text"
        color="primary"
        :disabled="loading || applying"
        @update:model-value="handleModeChange"
      >
        <v-btn value="system" class="px-4">
          <v-icon start size="18">mdi-microsoft-windows</v-icon>
          {{ t('settings.proxy.modeSystem') }}
        </v-btn>
        <v-btn value="direct" class="px-4">
          <v-icon start size="18">mdi-lan-disconnect</v-icon>
          {{ t('settings.proxy.modeDirect') }}
        </v-btn>
        <v-btn value="custom" class="px-4">
          <v-icon start size="18">mdi-tune-variant</v-icon>
          {{ t('settings.proxy.modeCustom') }}
        </v-btn>
      </v-btn-toggle>

      <p class="text-body-medium text-on-surface-variant mt-3 mb-0">{{ modeDescription }}</p>

      <v-expand-transition>
        <div v-if="isCustom" class="mt-4">
          <v-text-field
            v-model="url"
            :label="t('settings.proxy.urlLabel')"
            :placeholder="t('settings.proxy.urlPlaceholder')"
            density="comfortable"
            hide-details
            autocomplete="off"
            :disabled="applying"
            @keyup.enter="applyCustom"
          />
          <div class="text-body-small text-on-surface-variant mt-2">
            {{ t('settings.proxy.urlHint') }}
          </div>
          <div class="d-flex flex-wrap align-center ga-2 mt-3">
            <v-btn
              color="primary"
              variant="flat"
              prepend-icon="mdi-check"
              :loading="applying"
              :disabled="!canApply"
              @click="applyCustom"
            >
              {{ t('settings.proxy.apply') }}
            </v-btn>
            <v-btn
              variant="outlined"
              prepend-icon="mdi-access-point"
              :loading="testing"
              :disabled="!canTest"
              @click="testConnection"
            >
              {{ t('settings.proxy.test') }}
            </v-btn>
          </div>
        </div>
      </v-expand-transition>

      <div v-if="!isCustom" class="mt-3">
        <v-btn
          variant="outlined"
          prepend-icon="mdi-access-point"
          :loading="testing"
          :disabled="!canTest"
          @click="testConnection"
        >
          {{ t('settings.proxy.test') }}
        </v-btn>
      </div>

      <v-alert v-if="applyError" type="error" variant="tonal" density="compact" class="mt-3 text-body-small">
        {{ applyError }}
      </v-alert>
      <v-alert v-if="testResult" :type="testResult.type" variant="tonal" density="compact" class="mt-3 text-body-small">
        {{ testResult.text }}
      </v-alert>

      <p class="text-body-small text-on-surface-variant mt-3 mb-0">
        {{ t('settings.proxy.scopeHint') }}
      </p>
    </v-card-text>
  </v-card>
</template>
