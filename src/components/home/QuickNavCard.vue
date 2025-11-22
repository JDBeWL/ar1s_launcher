<script setup lang="ts">
</script>

<template>
  <v-card>
    <v-card-title>快速导航</v-card-title>
    <v-card-text>
      <v-list density="compact">
        <v-list-item to="/add-instance" prepend-icon="mdi-plus">
          <v-list-item-title>添加实例</v-list-item-title>
        </v-list-item>
        <v-list-item to="/instance-manager" prepend-icon="mdi-layers-outline">
          <v-list-item-title>实例管理</v-list-item-title>
        </v-list-item>
        <v-list-item to="/download" prepend-icon="mdi-download">
          <v-list-item-title>下载中心</v-list-item-title>
        </v-list-item>
      </v-list>
    </v-card-text>
  </v-card>
</template>
