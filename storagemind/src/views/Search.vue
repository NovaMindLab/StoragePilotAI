<template>
  <div class="card flex-col" style="height: 100%;">
    <div class="card-header flex items-center justify-between">
      <div>
        <div class="card-title">Search Results</div>
        <div class="card-subtitle" v-if="searchQuery">Showing results for "{{ searchQuery }}"</div>
        <div class="card-subtitle" v-else>Enter a query in the top bar</div>
      </div>
      <div class="flex items-center gap-2">
        <label class="font-sm text-secondary flex items-center gap-2" style="cursor: pointer;">
          <input type="checkbox" v-model="useSemantic" @change="reSearch" />
          ✨ AI Semantic Search
        </label>
      </div>
    </div>
    
    <div style="flex: 1; overflow-y: auto;">
      <table class="data-table" v-if="results.length > 0">
        <thead>
          <tr>
            <th>Name</th>
            <th>Path</th>
            <th>Type</th>
            <th>Size</th>
            <th v-if="useSemantic">Relevance</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="file in results" :key="file.id">
            <td class="flex items-center gap-2">
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" class="text-tertiary">
                <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path>
                <polyline points="13 2 13 9 20 9"></polyline>
              </svg>
              <span>{{ file.name }}</span>
            </td>
            <td class="text-tertiary font-sm truncate" style="max-width: 300px;" :title="file.path">{{ file.path }}</td>
            <td><span :class="['badge', 'badge-' + (file.category || 'other').toLowerCase()]">{{ file.category || 'Other' }}</span></td>
            <td class="font-mono text-tertiary">{{ formatSize(file.size) }}</td>
            <td v-if="useSemantic" class="font-mono text-tertiary">
              {{ (1.0 - (file.distance || 0)).toFixed(2) }}
            </td>
          </tr>
        </tbody>
      </table>
      
      <div v-else class="empty-state">
        <div class="empty-state-icon">🔍</div>
        <div class="empty-state-title" v-if="searchQuery">No results found</div>
        <div class="empty-state-title" v-else>Search for files</div>
        <div class="empty-state-desc">Try using different keywords or filters.</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

const route = useRoute()
const searchQuery = ref('')
const results = ref<any[]>([])
const useSemantic = ref(false)

const reSearch = () => {
  performSearch(searchQuery.value)
}

const performSearch = async (query: string) => {
  if (!query) {
    results.value = []
    return
  }
  
  try {
    if (useSemantic.value) {
      const response: any = await invoke('cmd_search_semantic', {
        query
      })
      // Map response to flatten file and distance
      results.value = response.map((r: any) => ({
        ...r.file,
        distance: r.distance
      }))
    } else {
      const response: any = await invoke('cmd_search_files', {
        params: {
          query,
          limit: 100,
          offset: 0
        }
      })
      results.value = response.files
    }
  } catch (e) {
    console.error('Search failed:', e)
  }
}

onMounted(() => {
  const q = route.query.q as string
  if (q) {
    searchQuery.value = q
    performSearch(q)
  }
})

watch(() => route.query.q, (newQ) => {
  const q = newQ as string
  searchQuery.value = q || ''
  performSearch(q || '')
})

const formatSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}
</script>
