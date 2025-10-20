<template>
  <v-container fluid>
    <!-- 标题卡片 -->
    <v-card class="mb-6" elevation="2">
      <v-card-title class="d-flex mt-2 justify-space-between align-center">
        实例管理
        <v-btn color="primary" variant="outlined">
          <v-icon start>mdi-cog</v-icon>
          批量管理
        </v-btn>
      </v-card-title>
      
      <!-- 实例卡片网格布局 -->
      <v-row class="pa-4">
      <v-col v-for="instance in instances" :key="instance.id" cols="12" sm="4" md="10" lg="3">
        <v-card class="instance-card" elevation="2">
          <div class="card-top">
            <!-- 左边图标占位符 -->
            <div class="icon-placeholder"></div>
            
            <!-- 右边信息区域 -->
            <div class="info-area">
              <div class="instance-name">{{ instance.name }}</div>
              <div class="version-info">{{ instance.version }}</div>
            </div>
          </div>
          <div class="card-bottom">
            <v-btn class="action-btn" icon variant="text" size="small" @click="editInstance(instance)">
              <v-icon>mdi-pencil</v-icon>
            </v-btn>
            
            <v-btn class="action-btn" icon variant="text" size="small" @click="openInstanceFolder(instance)">
              <v-icon>mdi-folder-open</v-icon>
            </v-btn>
            
            <v-btn class="action-btn" icon variant="text" size="small" @click="launchInstance(instance)">
              <v-icon>mdi-play</v-icon>
            </v-btn>
          </div>
        </v-card>
      </v-col>
    </v-row>
      </v-card>
  </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface Instance {
  id: string;
  name: string;
  version: string;
  path: string;
}

const instances = ref<Instance[]>([]);

// 模拟实例数据（后续需要从后端获取）
const mockInstances: Instance[] = [
  { id: '1', name: '生存服务器', version: '1.20.1-forge-47.2.0', path: '/instances/1' },
  { id: '2', name: '创造模式', version: '1.19.2', path: '/instances/2' },
  { id: '3', name: '模组测试', version: '1.18.2-fabric-0.14.22', path: '/instances/3' },
  { id: '4', name: '红石世界', version: '1.17.1', path: '/instances/4' },
  { id: '5', name: '极限生存', version: '1.16.5-forge-36.2.39', path: '/instances/5' },
  { id: '6', name: '建筑创作', version: '1.15.2', path: '/instances/6' },
  { id: '7', name: '冒险地图', version: '1.14.4', path: '/instances/7' },
  { id: '8', name: '服务器备份', version: '1.13.2', path: '/instances/8' },
  { id: '9', name: '测试实例', version: '1.12.2-forge-14.23.5.2859', path: '/instances/9' },
];

// 获取实例列表
async function fetchInstances() {
  try {
    // 这里需要调用后端API获取真实实例数据
    // const realInstances = await invoke('get_instances');
    // instances.value = realInstances;
    
    // 暂时使用模拟数据
    instances.value = mockInstances;
  } catch (error) {
    console.error('获取实例列表失败:', error);
  }
}

// 编辑实例名称
function editInstance(instance: Instance) {
  console.log('编辑实例:', instance);
  // 实现编辑逻辑
}

// 打开实例文件夹
function openInstanceFolder(instance: Instance) {
  console.log('打开实例文件夹:', instance);
  // 实现打开文件夹逻辑
}

// 启动实例
function launchInstance(instance: Instance) {
  console.log('启动实例:', instance);
  // 实现启动逻辑
}

onMounted(() => {
  fetchInstances();
});
</script>

<style scoped>
.instance-card {
  height: 150px; /* 长宽比1:2，宽度由网格系统控制 */
  border-radius: 12px;
  background-color: #f5f5f5; /* 灰色背景 */
  display: flex;
  flex-direction: column;
}

.card-top {
  flex: 2; /* 占2/3高度 */
  display: flex;
  padding: 12px;
}

.icon-placeholder {
  width: 54px;
  height: 54px;
  background-color: rgba(0, 0, 0, 0.1);
  border-radius: 8px;
  margin-right: 12px;
    margin-top: 10px;
}

.info-area {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.instance-name {
  flex: 2;
  font-size: 1.175rem;
  font-weight: 600;
  color: #000;
  display: flex;
  align-items: center;
  line-height: 1.2;
  margin-top: -4px;
}

.version-info {
  flex: 1;
  font-size: 0.825rem;
  color: #666;
  display: flex;
  align-items: flex-start;
  line-height: 1.2;
  margin-bottom: -4px;
}

.card-bottom {
  flex: 1;
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 8px;
  border-top: 1px solid rgba(0, 0, 0, 0.1);
}

.action-btn {
  margin: 0 8px;
  width: 36px;
  height: 36px;
  transition: all 0.2s ease;
}

.action-btn:hover {
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

/* 深色模式适配 */
:deep(.v-theme--dark) .instance-card {
  background-color: #424242;
}

:deep(.v-theme--dark) .instance-name {
  color: #fff;
}

:deep(.v-theme--dark) .version-info {
  color: #ccc;
}

:deep(.v-theme--dark) .card-bottom {
  border-top-color: rgba(255, 255, 255, 0.1);
}

:deep(.v-theme--dark) .icon-placeholder {
  background-color: rgba(255, 255, 255, 0.1);
}
</style>