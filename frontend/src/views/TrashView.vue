<script setup>
import { ref, computed, onMounted } from 'vue'
import * as api from '../api'
import { confirmDialog } from '../ui/dialog'
import { toast } from '../ui/toast'
import FileIcon from '../components/FileIcon.vue'
import { formatSize } from '../format'

const folders = ref([])
const files = ref([])
const error = ref('')
const loading = ref(true)
// One list: [kind, item] with kind matching the API path segment.
const items = computed(() => [...folders.value.map((f) => ['folders', f]), ...files.value.map((f) => ['files', f])])

async function load() {
  error.value = ''
  try {
    const data = await api.get('/api/trash')
    folders.value = data.folders
    files.value = data.files
  } catch (e) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}
onMounted(load)

async function run(fn, success) {
  try {
    await fn()
    toast(success)
  } catch (e) {
    toast(e.message, { type: 'error' })
  }
  await load()
}

const restore = (kind, item) => run(() => api.post(`/api/trash/${kind}/${item.id}`), `"${item.name}" restored`)

async function purge(kind, item) {
  const ok = await confirmDialog({
    title: 'Delete forever?',
    message: `"${item.name}"${kind === 'folders' ? ' and everything inside it' : ''} will be permanently deleted. This can’t be undone.`,
    confirmText: 'Delete forever',
  })
  if (ok) run(() => api.del(`/api/trash/${kind}/${item.id}`), `"${item.name}" deleted`)
}

async function empty() {
  const ok = await confirmDialog({
    title: 'Empty trash?',
    message: 'Everything in the trash will be permanently deleted. This can’t be undone.',
    confirmText: 'Empty trash',
  })
  if (ok) run(() => api.del('/api/trash'), 'Trash emptied')
}
</script>

<template>
  <div class="page">
    <header class="topbar">
      <strong class="brand">Mini-Drive</strong>
      <RouterLink to="/">Back to files</RouterLink>
    </header>

    <main class="content">
      <div class="toolbar">
        <h1>Trash</h1>
        <button type="button" class="danger" :disabled="!items.length" @click="empty">Empty trash</button>
      </div>
      <p class="hint">Items in the trash still count toward your storage until they are deleted forever.</p>

      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <table v-if="items.length" class="list">
        <thead>
          <tr>
            <th>Name</th>
            <th class="num">Size</th>
            <th><span class="sr-only">Actions</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="[kind, f] in items" :key="kind + f.id">
            <td class="name">
              <FileIcon :folder="kind === 'folders'" :item="f" class="icon" />
              {{ f.name }}
            </td>
            <td class="num">{{ kind === 'folders' ? '—' : formatSize(f.size_bytes) }}</td>
            <td class="row-actions">
              <button type="button" @click="restore(kind, f)" :aria-label="`Restore ${f.name}`">Restore</button>
              <button type="button" class="danger" @click="purge(kind, f)" :aria-label="`Delete ${f.name} forever`">Delete forever</button>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-else-if="!loading && !error" class="empty">The trash is empty.</p>
    </main>
  </div>
</template>

<style scoped>
.topbar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}
.brand { margin-right: auto; }
.content { max-width: 1000px; margin: 0 auto; padding: 1rem; }
.toolbar { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
h1 { margin: 0; font-size: 1.25rem; }
.hint { margin: 0.5rem 0 1rem; color: var(--muted); font-size: 0.9rem; }
.empty { padding: 2rem 0; text-align: center; color: var(--muted); }
.list {
  width: 100%;
  border-collapse: collapse;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
}
th, td { padding: 0.6rem 0.75rem; text-align: left; border-bottom: 1px solid var(--border); }
th { font-size: 0.85rem; font-weight: 600; color: var(--muted); }
tbody tr:last-child td { border-bottom: 0; }
td.name { word-break: break-all; }
td.name .icon { display: inline-block; margin-right: 0.6rem; vertical-align: middle; }
.num { text-align: right; white-space: nowrap; }
.row-actions { text-align: right; white-space: nowrap; }
.row-actions button { margin-left: 0.35rem; padding: 0.25rem 0.6rem; font-size: 0.85rem; }
.danger { color: var(--danger); }
.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
@media (max-width: 640px) {
  .row-actions { white-space: normal; }
  .row-actions button { margin: 0.15rem 0 0.15rem 0.35rem; }
}
</style>
