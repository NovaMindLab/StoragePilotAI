<template>
  <AppShell />
</template>

<script setup lang="ts">
import AppShell from './components/layout/AppShell.vue'
import { onMounted } from 'vue'
import { listen } from '@tauri-apps/api/event'

onMounted(async () => {
  // Listen for global events from Rust
  await listen('scan:started', (event) => {
    console.log('Scan started:', event.payload)
  })
  
  await listen('scan:progress', (event) => {
    console.log('Scan progress:', event.payload)
  })
  
  await listen('scan:completed', (event) => {
    console.log('Scan completed:', event.payload)
  })
})
</script>