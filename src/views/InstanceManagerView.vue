<template>
  <v-container>
    <v-row justify="center">
      <v-col cols="12" md="10" lg="8">
        <v-card>
          <v-card-title class="mt-2">实例管理</v-card-title>
          <v-card-text>
            <p class="text-center">这里将显示已创建的实例列表。</p>
            <p class="text-center text-grey">此功能正在开发中。</p>
            <!-- Placeholder for instance list -->
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>
  </v-container>
</template>

<script setup lang="ts">
// 实例管理页面的逻辑
// 这里将包含实例列表的获取和管理功能
</script>

<style scoped>
.v-card {
  transition: all 0.2s ease-in-out;
}

.v-card:hover {
  box-shadow: 0 8px 16px rgba(0,0,0,0.15);
}
</style>
