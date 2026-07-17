<template>
  <header class="topbar">
    <div class="topbar-title">{{ route.meta.title || 'StorageMind' }}</div>
    <div class="topbar-spacer"></div>
    
    <div class="search-input-wrapper" style="width: 240px;">
      <svg class="search-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8"></circle>
        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
      </svg>
      <input 
        type="text" 
        class="search-input" 
        placeholder="Search files (Ctrl+K)..."
        v-model="searchQuery"
        @keyup.enter="performSearch"
      />
    </div>
    
    <button class="btn btn-primary btn-sm" @click="startScan" :disabled="isScanning">
      <svg v-if="!isScanning" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="23 4 23 10 17 10"></polyline>
        <polyline points="1 20 1 14 7 14"></polyline>
        <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
      </svg>
      <span v-if="isScanning" class="spinner"></span>
      {{ isScanning ? 'Scanning...' : 'Scan Now' }}
    </button>
  </header>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { documentDir, downloadDir, pictureDir } from '@tauri-apps/api/path'

const route = useRoute()
const router = useRouter()
const searchQuery = ref('')
const isScanning = ref(false)

const performSearch = () => {
  if (searchQuery.value.trim()) {
    router.push({ path: '/search', query: { q: searchQuery.value } })
  }
}

const startScan = async () => {
  if (isScanning.value) return;
  isScanning.value = true;
  try {
    // Determine a sensible default path for testing (Pictures directory)
    const scanTarget = await pictureDir().catch(() => '/Users/hou/Pictures')
    
    await invoke('cmd_start_scan', { 
      params: { path: scanTarget, includeHidden: false } 
    })
    
    // Poll for status
    const interval = setInterval(async () => {
      try {
        const status: any = await invoke('cmd_get_scan_status')
        if (!status.isScanning && status.activeTasks === 0) {
          isScanning.value = false;
          clearInterval(interval);
          // Reload the page to refresh dashboard stats
          window.location.reload();
        }
      } catch (e) {
        clearInterval(interval);
        isScanning.value = false;
      }
    }, 2000);
    
  } catch (e) {
    console.error('Failed to start scan:', e)
    isScanning.value = false;
  }
}
</script>

<style scoped>
.spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255,255,255,0.3);
  border-radius: 50%;
  border-top-color: #fff;
  animation: spin 1s ease-in-out infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
