import { ref, computed, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useNotificationStore } from '../stores/notificationStore';
import type { MinecraftVersion, VersionManifest, CreateInstancePayload, InstallProgressPayload } from '../types/events';

export function useInstanceCreation() {
    const versions = ref<MinecraftVersion[]>([]);
    const loadingVersions = ref(false);
    const selectedVersion = ref<MinecraftVersion | null>(null);
    const searchVersion = ref("");
    const versionTypeFilter = ref("release");
    const sortOrder = ref("newest");

    const instanceName = ref("");
    const installing = ref(false);
    const showProgress = ref(false);
    const progressValue = ref(0);
    const progressIndeterminate = ref(false);
    const progressText = ref("");

    const modLoaderTypes = ["None", "Forge", "Fabric", "Quilt"];
    const selectedModLoaderType = ref("None");
    const modLoaderVersions = ref<string[]>([]);
    const loadingModLoaderVersions = ref(false);
    const selectedModLoaderVersion = ref<string | null>(null);

    const filteredVersions = computed(() => {
        let filtered = versions.value.filter((version) => {
            const typeMatch =
                versionTypeFilter.value === "all" ||
                version.type === versionTypeFilter.value;
            const searchMatch =
                !searchVersion.value ||
                version.id.toLowerCase().includes(searchVersion.value.toLowerCase());
            return typeMatch && searchMatch;
        });

        if (sortOrder.value === "newest") {
            filtered.sort(
                (a, b) =>
                    new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime()
            );
        } else if (sortOrder.value === "oldest") {
            filtered.sort(
                (a, b) =>
                    new Date(a.releaseTime).getTime() - new Date(b.releaseTime).getTime()
            );
        } else if (sortOrder.value === "az") {
            filtered.sort((a, b) => a.id.localeCompare(b.id));
        } else if (sortOrder.value === "za") {
            filtered.sort((a, b) => b.id.localeCompare(a.id));
        }

        return filtered;
    });

    const defaultInstanceName = computed(() => {
        if (selectedVersion.value) {
            return selectedVersion.value.id;
        }
        return "";
    });

    async function fetchVersions() {
        loadingVersions.value = true;
        try {
            const manifest = await invoke<VersionManifest>("get_versions");
            versions.value = manifest.versions.map((v) => ({
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
        if (
            !selectedVersion.value ||
            !selectedModLoaderType.value ||
            selectedModLoaderType.value === "None"
        ) {
            modLoaderVersions.value = [];
            return;
        }

        loadingModLoaderVersions.value = true;
        selectedModLoaderVersion.value = null;

        try {
            if (selectedModLoaderType.value === "Forge") {
                const result = await invoke<string[]>("get_forge_versions", {
                    minecraftVersion: selectedVersion.value.id,
                });
                modLoaderVersions.value = result;
            } else {
                modLoaderVersions.value = [];
            }

            if (modLoaderVersions.value.length > 0) {
                selectedModLoaderVersion.value = modLoaderVersions.value[0];
            }
        } catch (error) {
            console.error(
                `Failed to fetch ${selectedModLoaderType.value} versions:`,
                error
            );
            modLoaderVersions.value = [];
        } finally {
            loadingModLoaderVersions.value = false;
        }
    }

    let unlistenProgress: UnlistenFn | null = null;

    function cleanup() {
        if (unlistenProgress) {
            unlistenProgress();
            unlistenProgress = null;
        }
    }

    // 组件卸载时自动清理
    onUnmounted(cleanup);

    async function createInstance() {
        const notificationStore = useNotificationStore();
        
        if (!selectedVersion.value) {
            notificationStore.warning('请先选择一个Minecraft版本');
            return;
        }

        const finalInstanceName = instanceName.value || defaultInstanceName.value;
        if (!finalInstanceName) {
            notificationStore.warning('实例名称不能为空');
            return;
        }

        installing.value = true;
        showProgress.value = true;
        progressValue.value = 0;
        progressIndeterminate.value = true;
        progressText.value = "准备安装...";

        try {
            unlistenProgress = await listen<InstallProgressPayload>(
                "instance-install-progress",
                (event) => {
                    const progressData = event.payload;
                    progressValue.value = progressData.progress;
                    progressText.value = progressData.message;
                    progressIndeterminate.value = progressData.indeterminate;
                }
            );

            const payload: CreateInstancePayload = {
                newInstanceName: finalInstanceName,
                baseVersionId: selectedVersion.value.id,
            };

            if (
                selectedModLoaderType.value === "Forge" &&
                selectedModLoaderVersion.value
            ) {
                payload.forgeVersion = selectedModLoaderVersion.value;
            }

            await invoke("create_instance", { ...payload });

            notificationStore.success('创建成功', `实例 '${finalInstanceName}' 已创建`);

            showProgress.value = false;
            installing.value = false;
        } catch (error) {
            console.error("Failed to create instance:", error);
            progressText.value = "安装失败！";
            progressIndeterminate.value = false;
            installing.value = false;

            await new Promise((resolve) => setTimeout(resolve, 1000));
            showProgress.value = false;

            const errorMessage = error instanceof Error ? error.message : String(error);
            notificationStore.error('创建实例失败', errorMessage, true);
        } finally {
            cleanup();
        }
    }

    return {
        versions,
        loadingVersions,
        selectedVersion,
        searchVersion,
        versionTypeFilter,
        sortOrder,
        filteredVersions,
        instanceName,
        defaultInstanceName,
        installing,
        showProgress,
        progressValue,
        progressIndeterminate,
        progressText,
        modLoaderTypes,
        selectedModLoaderType,
        modLoaderVersions,
        loadingModLoaderVersions,
        selectedModLoaderVersion,
        fetchVersions,
        fetchModLoaderVersions,
        createInstance
    };
}
