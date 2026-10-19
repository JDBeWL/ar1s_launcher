<script setup lang="ts">
import { onMounted, computed } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { useInstanceCreation } from '../../composables/useInstanceCreation';
import { getLoaderIcon } from '../../utils/format';
import type { ForgeVersion, LoaderVersionInfo } from '../../types/events';

const router = useRouter();
const { t } = useI18n();

const {
  loadingVersions,
  selectedVersion,
  searchVersion,
  versionTypeFilter,
  filteredVersions,
  instanceName,
  instanceNameError,
  defaultInstanceName,
  installing,
  showProgress,
  progressValue,
  progressIndeterminate,
  progressText,
  loadingAvailableLoaders,
  modLoaderTypes,
  selectedModLoaderType,
  modLoaderVersions,
  loadingModLoaderVersions,
  selectedModLoaderVersion,
  fetchVersions,
  createInstance,
} = useInstanceCreation();

// 类型守卫：有 stable 属性即为 LoaderVersionInfo
function hasStableProperty(item: ForgeVersion | LoaderVersionInfo): item is LoaderVersionInfo {
  return 'stable' in item;
}

const effectiveInstanceName = computed(() => {
  return instanceName.value.trim() || defaultInstanceName.value;
});

const canCreate = computed(() => {
  return (
    selectedVersion.value &&
    !installing.value &&
    !instanceNameError.value &&
    (selectedModLoaderType.value === 'None' || selectedModLoaderVersion.value)
  );
});

function getLoaderVersionText(item: ForgeVersion | LoaderVersionInfo | null | undefined): string {
  if (!item) return '';
  if ('version' in item) {
    // ForgeVersion 有 mcversion 字段
    if ('mcversion' in item) {
      return item.version;
    }
    const stableText =
      'stable' in item && item.stable === true
        ? t('customInstall.stableSuffix')
        : 'stable' in item && item.stable === false
          ? t('customInstall.betaSuffix')
          : '';
    return item.version + stableText;
  }
  return String(item);
}

async function handleCreate() {
  await createInstance();
  if (!installing.value && !showProgress.value) {
    router.push('/instance-manager');
  }
}

onMounted(() => {
  fetchVersions();
});
</script>

<template>
  <div>
    <v-card color="surface-container" variant="flat" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-4">
          <div class="d-flex align-center">
            <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-minecraft</v-icon>
            <span class="text-title-medium">{{ t('customInstall.gameVersionTitle') }}</span>
          </div>
          <v-btn-toggle
            v-model="versionTypeFilter"
            mandatory
            border
            divided
            rounded="pill"
            density="comfortable"
            variant="text"
            color="primary"
          >
            <v-btn value="release">{{ t('customInstall.release') }}</v-btn>
            <v-btn value="snapshot">{{ t('customInstall.snapshot') }}</v-btn>
            <v-btn value="all">{{ t('customInstall.all') }}</v-btn>
          </v-btn-toggle>
        </div>

        <v-row density="compact">
          <v-col cols="12" sm="8">
            <v-select
              v-model="selectedVersion"
              :items="filteredVersions"
              item-title="id"
              item-value="id"
              :label="t('customInstall.versionLabel')"
              density="comfortable"
              variant="outlined"
              hide-details
              return-object
            >
              <template #prepend-inner>
                <v-icon size="18" color="on-surface-variant">mdi-gamepad-variant</v-icon>
              </template>
              <template #item="{ item, props }">
                <v-list-item v-bind="props">
                  <template #append>
                    <v-chip size="x-small" :color="item.type === 'release' ? 'success' : 'warning'" variant="tonal">
                      {{ item.type === 'release' ? t('customInstall.release') : t('customInstall.snapshot') }}
                    </v-chip>
                  </template>
                </v-list-item>
              </template>
              <template #no-data>
                <v-list-item>
                  <v-list-item-title class="text-on-surface-variant">
                    {{ loadingVersions ? t('customInstall.loading') : t('customInstall.noVersionFound') }}
                  </v-list-item-title>
                </v-list-item>
              </template>
            </v-select>
          </v-col>
          <v-col cols="12" sm="4">
            <v-text-field
              v-model="searchVersion"
              :label="t('customInstall.searchLabel')"
              density="comfortable"
              variant="outlined"
              hide-details
              clearable
            >
              <template #prepend-inner>
                <v-icon size="18" color="on-surface-variant">mdi-magnify</v-icon>
              </template>
            </v-text-field>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <v-card color="surface-container" variant="flat" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-puzzle</v-icon>
          <span class="text-title-medium">{{ t('customInstall.loaderTitle') }}</span>
          <v-chip size="x-small" class="ml-2" color="secondary" variant="tonal">
            {{ t('customInstall.optional') }}
          </v-chip>
          <v-spacer />
          <v-fade-transition>
            <v-progress-circular v-if="loadingAvailableLoaders" indeterminate size="16" width="2" color="primary" />
          </v-fade-transition>
        </div>

        <v-btn-toggle
          v-model="selectedModLoaderType"
          mandatory
          border
          divided
          rounded="pill"
          density="comfortable"
          variant="text"
          color="primary"
          class="mb-4"
        >
          <v-btn
            v-for="loader in modLoaderTypes"
            :key="loader.value"
            :value="loader.value"
            :disabled="loader.disabled || !selectedVersion || loadingAvailableLoaders"
          >
            <v-icon start size="16">{{ getLoaderIcon(loader.value, 'select') }}</v-icon>
            {{ loader.title }}
            <v-tooltip v-if="loader.disabled && selectedVersion" activator="parent" location="top">
              {{ t('customInstall.loaderUnsupported', { loader: loader.title }) }}
            </v-tooltip>
          </v-btn>
        </v-btn-toggle>

        <v-expand-transition>
          <div v-if="selectedModLoaderType !== 'None'">
            <v-select
              v-model="selectedModLoaderVersion"
              :items="modLoaderVersions"
              :item-title="getLoaderVersionText"
              item-value="version"
              :label="t('customInstall.loaderVersionLabel')"
              :placeholder="t('customInstall.loaderVersionPlaceholder', { loader: selectedModLoaderType })"
              density="comfortable"
              variant="outlined"
              :disabled="loadingModLoaderVersions"
              hide-details
              return-object
            >
              <template #prepend-inner>
                <v-progress-circular
                  v-if="loadingModLoaderVersions"
                  indeterminate
                  size="16"
                  width="2"
                  color="primary"
                  class="mr-1"
                />
                <v-icon v-else size="18" color="on-surface-variant">mdi-tag</v-icon>
              </template>
              <template #item="{ item, props }">
                <v-list-item v-bind="props">
                  <template #append>
                    <v-chip
                      v-if="hasStableProperty(item) && item.stable === true"
                      size="x-small"
                      color="success"
                      variant="tonal"
                    >
                      {{ t('customInstall.stable') }}
                    </v-chip>
                    <v-chip
                      v-else-if="hasStableProperty(item) && item.stable === false"
                      size="x-small"
                      color="warning"
                      variant="tonal"
                    >
                      {{ t('customInstall.beta') }}
                    </v-chip>
                  </template>
                </v-list-item>
              </template>
              <template #no-data>
                <v-list-item>
                  <v-list-item-title class="text-on-surface-variant">
                    {{ loadingModLoaderVersions ? t('customInstall.loading') : t('customInstall.noLoaderVersion') }}
                  </v-list-item-title>
                </v-list-item>
              </template>
            </v-select>
          </div>
        </v-expand-transition>

        <v-alert v-if="!selectedVersion" type="info" variant="tonal" density="compact" class="mt-2">
          {{ t('customInstall.selectVersionFirst') }}
        </v-alert>
      </v-card-text>
    </v-card>

    <v-card color="surface-container" variant="flat" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-rename</v-icon>
          <span class="text-title-medium">{{ t('customInstall.nameTitle') }}</span>
        </div>

        <v-text-field
          v-model="instanceName"
          :label="t('customInstall.nameLabel')"
          :placeholder="defaultInstanceName || undefined"
          :error-messages="instanceNameError || undefined"
          density="comfortable"
          variant="outlined"
          hide-details="auto"
          counter="64"
          maxlength="64"
        >
          <template #append-inner>
            <v-fade-transition>
              <v-icon v-if="effectiveInstanceName && !instanceNameError" size="18" color="success">
                mdi-check-circle
              </v-icon>
            </v-fade-transition>
          </template>
        </v-text-field>

        <div class="text-body-small text-on-surface-variant mt-2">
          {{ t('customInstall.nameHintPrefix') }}
          <span class="font-weight-medium">{{ defaultInstanceName || t('customInstall.versionIdFallback') }}</span>
          {{ t('customInstall.nameHintSuffix') }}
        </div>
      </v-card-text>
    </v-card>

    <v-expand-transition>
      <v-card v-if="showProgress" color="surface-container-high" variant="flat" class="mb-4">
        <v-card-text class="pa-4">
          <div class="d-flex align-center justify-space-between mb-2 num-mono">
            <div class="d-flex align-center">
              <v-progress-circular
                v-if="progressIndeterminate"
                indeterminate
                size="18"
                width="2"
                color="primary"
                class="mr-2"
              />
              <v-icon v-else size="18" color="primary" class="mr-2">mdi-download</v-icon>
              <span class="text-body-medium">{{ progressText }}</span>
            </div>
            <span class="text-body-medium font-weight-medium text-primary">{{ progressValue }}%</span>
          </div>
          <v-progress-linear
            :model-value="progressValue"
            :indeterminate="progressIndeterminate"
            height="6"
            rounded
            color="primary"
            bg-color="surface-container"
          />
        </v-card-text>
      </v-card>
    </v-expand-transition>

    <v-card v-if="selectedVersion && !showProgress" color="primary-container" variant="flat" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between">
          <div>
            <div class="text-body-medium text-on-primary-container opacity-70">
              {{ t('customInstall.aboutToCreate') }}
            </div>
            <div class="text-body-large font-weight-medium text-on-primary-container">
              {{ effectiveInstanceName }}
            </div>
            <div class="d-flex align-center ga-1 mt-1">
              <v-chip size="x-small" color="on-primary-container" variant="outlined">
                {{ selectedVersion.id }}
              </v-chip>
              <v-chip
                v-if="
                  selectedModLoaderType !== 'None' && selectedModLoaderVersion && 'version' in selectedModLoaderVersion
                "
                size="x-small"
                color="on-primary-container"
                variant="outlined"
              >
                {{ selectedModLoaderType }} {{ selectedModLoaderVersion.version }}
              </v-chip>
            </div>
          </div>
          <v-btn
            color="on-primary-container"
            variant="flat"
            :disabled="!canCreate"
            :loading="installing"
            @click="handleCreate"
          >
            <v-icon start>mdi-download</v-icon>
            {{ t('customInstall.createInstance') }}
          </v-btn>
        </div>
      </v-card-text>
    </v-card>

    <div v-else-if="!showProgress" class="d-flex justify-end">
      <v-btn color="primary" size="large" disabled>
        <v-icon start>mdi-download</v-icon>
        {{ t('customInstall.createInstance') }}
      </v-btn>
    </div>
  </div>
</template>
