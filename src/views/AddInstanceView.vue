<template>
    <v-container fluid>
        <v-row justify="center">
            <v-col cols="12" md="5" lg="8">
                <v-card>
                    <v-card-title class="mt-2">添加新实例</v-card-title>
                    <v-card-text>
                        <!-- 安装方式选择 -->
                        <v-row class="mb-4">
                            <v-col cols="12" class="d-flex align-center">
                                <div class="d-flex align-center" style="width: 100%;">
                                    <div class="install-type-tab flex-grow-1 text-center py-3 cursor-pointer"
                                        :class="{ 'install-type-active': installType === 'custom' }"
                                        @click="installType = 'custom'">
                                        自定义安装
                                    </div>
                                    <div class="install-type-divider"></div>
                                    <div class="install-type-tab flex-grow-1 text-center py-3 cursor-pointer"
                                        :class="{ 'install-type-active': installType === 'online' }"
                                        @click="installType = 'online'">
                                        从互联网安装
                                    </div>
                                </div>
                            </v-col>
                        </v-row>

                        <!-- 自定义安装内容 -->
                        <div v-if="installType === 'custom'">
                            <v-row>
                                <v-col cols="12">
                                    <v-text-field v-model="instanceName" label="实例名称" :placeholder="defaultInstanceName"
                                        hide-details></v-text-field>
                                </v-col>
                            </v-row>

                            <!-- 搜索游戏版本 -->
                            <v-row no-gutters class="align-center mt-4 mb-4">
                                <v-col class="flex-grow-1 pr-2">
                                    <v-text-field v-model="searchVersion" label="搜索版本" prepend-inner-icon="mdi-magnify"
                                        clearable hide-details></v-text-field>
                                </v-col>
                                <v-col class="shrink" style="max-width: 150px;">
                                    <v-select v-model="versionTypeFilter" label="版本类型" :items="versionTypes"
                                        hide-details></v-select>
                                </v-col>
                            </v-row>

                            <!-- 游戏版本和Mod加载器选择 -->
                            <v-row no-gutters class="align-center mb-4">
                                <v-col class="shrink pr-2" style="max-width: 200px;">
                                    <v-select v-model="selectedVersion" :items="filteredVersions" item-title="id"
                                        item-value="id" label="游戏版本" :loading="loadingVersions" hide-details
                                        return-object></v-select>
                                </v-col>
                                <v-col class="shrink pr-2" style="max-width: 200px;">
                                    <v-select v-model="selectedModLoaderType" :items="modLoaderTypes" label="Mod加载器"
                                        :disabled="!selectedVersion" hide-details></v-select>
                                </v-col>
                                <v-col class="shrink" style="max-width: 700px;">
                                    <v-select v-model="selectedModLoaderVersion" :items="modLoaderVersions"
                                        item-title="version" item-value="version" label="Mod加载器版本"
                                        :loading="loadingModLoaderVersions"
                                        :disabled="!selectedModLoaderType || selectedModLoaderType === 'None'"
                                        placeholder="请先选择Mod加载器" hide-details return-object></v-select>
                                </v-col>
                            </v-row>

                            <!-- 进度条 -->
                            <v-row v-if="showProgress" class="mt-4">
                                <v-col cols="12">
                                    <div class="d-flex align-center justify-space-between mb-2">
                                        <span class="text-caption">{{ progressText }}</span>
                                        <span class="text-caption font-weight-medium">{{ progressValue }}%</span>
                                    </div>
                                    <v-progress-linear v-model="progressValue" color="primary" height="8"
                                        :indeterminate="progressIndeterminate"></v-progress-linear>
                                </v-col>
                            </v-row>

                            <!-- 开始安装按钮 -->
                            <v-row class="mt-4">
                                <v-col cols="12" class="text-right">
                                    <v-btn color="primary" size="large" @click="createInstance"
                                        :disabled="!selectedVersion || installing" :loading="installing">
                                        开始安装
                                    </v-btn>
                                </v-col>
                            </v-row>
                        </div>

                        <!-- 从互联网安装内容 -->
                        <div v-if="installType === 'online'">
                            <v-alert type="info" class="mb-4">
                                从互联网安装功能正在开发中...
                            </v-alert>
                        </div>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>
    </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const installType = ref('custom');

interface MinecraftVersion {
    id: string;
    type: string;
    url: string;
    time: string;
    releaseTime: string;
}

const versions = ref<MinecraftVersion[]>([]);
const loadingVersions = ref(false);
const selectedVersion = ref<MinecraftVersion | null>(null);
const searchVersion = ref('');
const versionTypeFilter = ref('release');

const versionTypes = [
    { title: '正式版', value: 'release' },
    { title: '快照版', value: 'snapshot' },
    { title: '全部', value: 'all' }
];

const instanceName = ref('');
const installing = ref(false);
const showProgress = ref(false);
const progressValue = ref(0);
const progressIndeterminate = ref(false);
const progressText = ref('');

const modLoaderTypes = ['None', 'Forge', 'Fabric', 'Quilt'];
const selectedModLoaderType = ref('None');

const modLoaderVersions = ref<string[]>([]);
const loadingModLoaderVersions = ref(false);
const selectedModLoaderVersion = ref<string | null>(null);

const filteredVersions = computed(() => {
    return versions.value.filter(version => {
        const typeMatch = versionTypeFilter.value === 'all' || version.type === versionTypeFilter.value;
        const searchMatch = !searchVersion.value || version.id.toLowerCase().includes(searchVersion.value.toLowerCase());
        return typeMatch && searchMatch;
    });
});

const defaultInstanceName = computed(() => {
    if (selectedVersion.value) {
        return selectedVersion.value.id;
    }
    return '';
});

async function fetchVersions() {
    loadingVersions.value = true;
    try {
        const manifest = await invoke('get_versions');
        versions.value = (manifest as any).versions.map((v: any) => ({
            ...v,
            releaseTime: new Date(v.releaseTime).toLocaleString(),
        }));
    } catch (error) {
        console.error("Failed to fetch versions:", error);
    } finally {
        loadingVersions.value = false;
    }
}

async function fetchModLoaderVersions() {
    if (!selectedVersion.value || !selectedModLoaderType.value || selectedModLoaderType.value === 'None') {
        modLoaderVersions.value = [];
        return;
    }

    loadingModLoaderVersions.value = true;
    selectedModLoaderVersion.value = null;

    try {
        if (selectedModLoaderType.value === 'Forge') {
            const result = await invoke('get_forge_versions', { minecraftVersion: selectedVersion.value.id });
            modLoaderVersions.value = result as any[];
        } else {
            // Placeholder for other loaders like Fabric
            modLoaderVersions.value = [];
        }

        if (modLoaderVersions.value.length > 0) {
            selectedModLoaderVersion.value = modLoaderVersions.value[0];
        }

    } catch (error) {
        console.error(`Failed to fetch ${selectedModLoaderType.value} versions:`, error);
        modLoaderVersions.value = []; // Clear on error
    } finally {
        loadingModLoaderVersions.value = false;
    }
}

// 监听进度事件
let unlistenProgress: (() => void) | null = null;

async function createInstance() {
    if (!selectedVersion.value) {
        alert('请先选择一个Minecraft版本');
        return;
    }

    const finalInstanceName = instanceName.value || defaultInstanceName.value;
    if (!finalInstanceName) {
        alert('实例名称不能为空');
        return;
    }

    installing.value = true;
    showProgress.value = true;
    progressValue.value = 0;
    progressIndeterminate.value = true;
    progressText.value = '准备安装...';

    try {
        // 监听进度事件
        unlistenProgress = await listen('instance-install-progress', (event: any) => {
            const progressData = event.payload;
            progressValue.value = progressData.progress;
            progressText.value = progressData.message;
            progressIndeterminate.value = progressData.indeterminate;
        });

        let payload: any = {
            newInstanceName: finalInstanceName,
            baseVersionId: selectedVersion.value.id,
        };

        if (selectedModLoaderType.value === 'Forge' && selectedModLoaderVersion.value) {
            payload.forgeVersion = selectedModLoaderVersion.value;
        }

        await invoke('create_instance', payload);

        alert(`实例 '${finalInstanceName}' 创建成功!`);

        // 重置进度状态
        showProgress.value = false;
        installing.value = false;

    } catch (error) {
        console.error("Failed to create instance:", error);
        progressText.value = '安装失败！';
        progressIndeterminate.value = false;
        installing.value = false;

        await new Promise(resolve => setTimeout(resolve, 1000));
        showProgress.value = false;

        alert(`创建实例失败: ${error}`);
    } finally {
        // 取消监听
        if (unlistenProgress) {
            unlistenProgress();
            unlistenProgress = null;
        }
    }
}

onUnmounted(() => {
    // 组件卸载时取消监听
    if (unlistenProgress) {
        unlistenProgress();
    }
});

onMounted(() => {
    fetchVersions();
});

watch(selectedVersion, () => {
    selectedModLoaderType.value = 'None';
    selectedModLoaderVersion.value = null;
    modLoaderVersions.value = [];
});

watch(selectedModLoaderType, () => {
    selectedModLoaderVersion.value = null;
    fetchModLoaderVersions();
});

</script>

<style scoped>
.install-type-tab {
    border-bottom: 2px solid transparent;
    transition: all 0.3s ease;
    color: rgba(var(--v-theme-on-surface), var(--v-medium-emphasis-opacity));
    font-weight: 500;
}

.install-type-tab:hover {
    box-shadow: 0 2px 4px -1px rgba(0, 0, 0, 0.1);
}

.install-type-active {
    border-bottom-color: rgb(var(--v-theme-primary));
    color: rgb(var(--v-theme-primary));
    font-weight: 600;
}

.install-type-divider {
    width: 1px;
    height: 24px;
    background-color: rgba(var(--v-theme-on-surface), 0.12);
    margin: 0 8px;
}
</style>