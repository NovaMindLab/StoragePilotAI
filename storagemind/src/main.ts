import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { createRouter, createWebHashHistory } from 'vue-router'
import './style.css'
import App from './App.vue'

// Router
import Dashboard from './views/Dashboard.vue'
import Explorer from './views/Explorer.vue'
import Analyzer from './views/Analyzer.vue'
import Search from './views/Search.vue'
import LargeFiles from './views/LargeFiles.vue'
import Duplicates from './views/Duplicates.vue'
import Cleaner from './views/Cleaner.vue'
import Settings from './views/Settings.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/dashboard' },
    { path: '/dashboard', component: Dashboard, meta: { title: 'Dashboard' } },
    { path: '/explorer', component: Explorer, meta: { title: 'Explorer' } },
    { path: '/analyzer', component: Analyzer, meta: { title: 'Disk Analyzer' } },
    { path: '/search', component: Search, meta: { title: 'Search' } },
    { path: '/large-files', component: LargeFiles, meta: { title: 'Large Files' } },
    { path: '/duplicates', component: Duplicates, meta: { title: 'Duplicates' } },
    { path: '/cleaner', component: Cleaner, meta: { title: 'Cleaner' } },
    { path: '/settings', component: Settings, meta: { title: 'Settings' } },
  ]
})

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.mount('#app')
