<script setup>
import { ref, reactive, computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuth } from '../stores/auth'
import * as api from '../api'
import { promptDialog, confirmDialog } from '../ui/dialog'
import { toast } from '../ui/toast'
import DropZone from '../components/DropZone.vue'
import EmptyState from '../components/EmptyState.vue'
import FileGrid from '../components/FileGrid.vue'
import FileIcon from '../components/FileIcon.vue'
import FilePreview, { previewKind } from '../components/FilePreview.vue'
import ShareDialog from '../components/ShareDialog.vue'
import StorageMeter from '../components/StorageMeter.vue'
import StatsCards from '../components/StatsCards.vue'
import { formatSize, formatDate } from '../format'

const route = useRoute()
const router = useRouter()
const auth = useAuth()

const folderId = computed(() => route.query.folder || null)
// Search is global (all folders); the folder stays in the URL so clearing the box returns to it.
const search = computed(() => (route.query.q || '').trim())
const term = ref(route.query.q || '')
watch(() => route.query.q, (q) => {
  if ((q || '') !== term.value.trim()) term.value = q || ''
})
let searchTimer
watch(term, (t) => {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    const q = t.trim()
    if (q !== search.value) router.replace({ query: { ...route.query, q: q || undefined } })
  }, 250)
})
const folders = ref([])
const files = ref([])
const breadcrumbs = ref([])
const uploads = ref([])
const error = ref('')
const loading = ref(false)
const fileInput = ref(null)
const preview = ref(null)
const sharing = ref(null)
const meter = ref(null)
const stats = ref(null)
const refreshUsage = () => {
  meter.value?.refresh()
  stats.value?.refresh()
}

const isImage = (f) => f.mime_type.startsWith('image/')

const view = ref(readView())
watch(view, (v) => {
  try { localStorage.setItem('view', v) } catch {}
})
function readView() {
  try { return localStorage.getItem('view') === 'grid' ? 'grid' : 'list' } catch { return 'list' }
}

async function load() {
  const id = folderId.value
  const q = search.value
  const stale = () => id !== folderId.value || q !== search.value
  loading.value = true
  error.value = ''
  try {
    const params = new URLSearchParams()
    if (q) params.set('q', q)
    else if (id) params.set('folder_id', id)
    const data = await api.get('/api/files?' + params)
    if (stale()) return // user navigated away; a newer load owns the view
    folders.value = data.folders
    files.value = data.files
    breadcrumbs.value = data.breadcrumbs
  } catch (e) {
    if (stale()) return
    folders.value = []
    files.value = []
    breadcrumbs.value = []
    error.value = e.message
  } finally {
    loading.value = false
  }
}
watch([folderId, search], load, { immediate: true })

// Runs an action, toasts the outcome, reloads the listing.
async function run(fn, success) {
  try {
    await fn()
    toast(success)
  } catch (e) {
    toast(e.message, { type: 'error' })
  }
  refreshUsage()
  await load()
}

function openFolder(id) {
  router.push({ query: { folder: id } })
}

async function newFolder() {
  const name = await promptDialog({ title: 'New folder', label: 'Folder name', confirmText: 'Create' })
  if (!name) return
  run(() => api.post('/api/folders', { name, parent_id: folderId.value ? Number(folderId.value) : null }), `Folder "${name}" created`)
}

async function rename(kind, item) {
  const name = await promptDialog({ title: `Rename ${kind === 'folders' ? 'folder' : 'file'}`, label: 'New name', value: item.name, confirmText: 'Rename' })
  if (!name || name === item.name) return
  run(() => api.patch(`/api/${kind}/${item.id}`, { name }), `Renamed to "${name}"`)
}

async function remove(kind, item) {
  const isFolder = kind === 'folders'
  const ok = await confirmDialog({
    title: isFolder ? 'Move folder to trash?' : 'Move file to trash?',
    message: `"${item.name}"${isFolder ? ' and everything inside it' : ''} will be moved to the trash. You can restore it from there.`,
    confirmText: 'Move to trash',
  })
  if (!ok) return
  run(() => api.del(`/api/${kind}/${item.id}`), `"${item.name}" moved to trash`)
}

function onFilesPicked(e) {
  const picked = [...e.target.files]
  e.target.value = ''
  uploadFiles(picked)
}

async function uploadFiles(picked) {
  const target = folderId.value
  let done = 0
  await Promise.all(
    picked.map(async (file) => {
      const u = reactive({ name: file.name, progress: 0, error: '' })
      uploads.value.push(u)
      try {
        await api.upload(file, target, (p) => (u.progress = p))
        uploads.value.splice(uploads.value.indexOf(u), 1)
        done++
      } catch (err) {
        u.error = err.message
      }
    }),
  )
  if (done) toast(done === 1 ? `Uploaded "${picked[0].name}"` : `Uploaded ${done} files`)
  refreshUsage()
  await load()
}

function dismiss(u) {
  uploads.value.splice(uploads.value.indexOf(u), 1)
}

async function logout() {
  await auth.logout().catch(() => {})
  router.push('/login')
}
</script>

<template>
  <DropZone class="page" :label="breadcrumbs.at(-1)?.name" @drop="uploadFiles">
    <header class="topbar">
      <strong class="brand">Mini-Drive</strong>
      <StorageMeter ref="meter" class="topbar-meter" />
      <RouterLink to="/trash">Trash</RouterLink>
      <RouterLink to="/about">About</RouterLink>
      <span class="user">{{ auth.user?.username }}</span>
      <button type="button" @click="logout">Logout</button>
    </header>

    <main class="content">
      <StatsCards v-if="!folderId" ref="stats" />
      <div class="toolbar">
        <nav class="crumbs" aria-label="Breadcrumb">
          <span v-if="search" aria-current="page">Search results for “{{ search }}”</span>
          <RouterLink v-else-if="breadcrumbs.length" :to="{ query: {} }">Home</RouterLink>
          <span v-else aria-current="page">Home</span>
          <template v-for="(b, i) in breadcrumbs" :key="b.id">
            <span class="sep" aria-hidden="true">&gt;</span>
            <span v-if="i === breadcrumbs.length - 1" aria-current="page">{{ b.name }}</span>
            <RouterLink v-else :to="{ query: { folder: b.id } }">{{ b.name }}</RouterLink>
          </template>
        </nav>
        <div class="actions">
          <input v-model="term" type="search" class="search" placeholder="Search all files" aria-label="Search all files" />
          <div class="views" role="group" aria-label="View">
            <button type="button" :aria-pressed="view === 'list'" title="List view" @click="view = 'list'">
              <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M3 5h14M3 10h14M3 15h14" /></svg>
              <span class="sr-only">List view</span>
            </button>
            <button type="button" :aria-pressed="view === 'grid'" title="Grid view" @click="view = 'grid'">
              <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M3 3h6v6H3zM11 3h6v6h-6zM3 11h6v6H3zM11 11h6v6h-6z" /></svg>
              <span class="sr-only">Grid view</span>
            </button>
          </div>
          <button type="button" @click="newFolder">New Folder</button>
          <button type="button" class="primary" @click="fileInput.click()">Upload</button>
          <input ref="fileInput" type="file" multiple hidden aria-label="Upload files" @change="onFilesPicked" />
        </div>
      </div>

      <p v-if="error" class="error" role="alert">{{ error }}</p>

      <ul v-if="uploads.length" class="uploads" aria-label="Uploads">
        <li v-for="(u, i) in uploads" :key="i">
          <span class="name">{{ u.name }}</span>
          <template v-if="u.error">
            <span class="error">{{ u.error }}</span>
            <button type="button" @click="dismiss(u)" :aria-label="`Dismiss ${u.name}`">Dismiss</button>
          </template>
          <progress v-else :value="u.progress" max="1" :aria-label="`Uploading ${u.name}`"></progress>
        </li>
      </ul>

      <FileGrid
        v-if="view === 'grid' && (folders.length || files.length)"
        :folders="folders"
        :files="files"
        @open-folder="openFolder"
        @preview="(f) => (preview = f)"
        @share="(kind, item) => (sharing = { kind, item })"
        @rename="rename"
        @remove="remove"
      />
      <table v-else-if="folders.length || files.length" class="list">
        <thead>
          <tr>
            <th>Name</th>
            <th class="num">Size</th>
            <th class="date">Date</th>
            <th><span class="sr-only">Actions</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="f in folders" :key="'d' + f.id">
            <td>
              <button type="button" class="link" @click="openFolder(f.id)"><FileIcon folder class="icon" />{{ f.name }}</button>
            </td>
            <td class="num">—</td>
            <td class="date">{{ formatDate(f.created_at) }}</td>
            <td class="row-actions">
              <a class="btn" :href="`/api/folders/${f.id}/zip`" download :aria-label="`Download ${f.name} as zip`">Download</a>
              <button type="button" @click="sharing = { kind: 'folders', item: f }" :aria-label="`Share ${f.name}`">Share</button>
              <button type="button" @click="rename('folders', f)" :aria-label="`Rename ${f.name}`">Rename</button>
              <button type="button" class="danger" @click="remove('folders', f)" :aria-label="`Delete ${f.name}`">Delete</button>
            </td>
          </tr>
          <tr v-for="f in files" :key="'f' + f.id">
            <td class="name">
              <button v-if="isImage(f)" type="button" class="thumb" @click="preview = f" :aria-label="`Preview ${f.name}`">
                <!-- ponytail: thumbnail is the full original, add server-side thumbnails if large photos make listings slow -->
                <img :src="'/api/files/download/' + f.id" alt="" loading="lazy" />
              </button>
              <FileIcon v-else :item="f" class="icon" />
              <button v-if="previewKind(f)" type="button" class="link" title="Preview" @click="preview = f">{{ f.name }}</button>
              <template v-else>{{ f.name }}</template>
            </td>
            <td class="num">{{ formatSize(f.size_bytes) }}</td>
            <td class="date">{{ formatDate(f.created_at) }}</td>
            <td class="row-actions">
              <a class="btn" :href="'/api/files/download/' + f.id" download :aria-label="`Download ${f.name}`">Download</a>
              <button type="button" @click="sharing = { kind: 'files', item: f }" :aria-label="`Share ${f.name}`">Share</button>
              <button type="button" @click="rename('files', f)" :aria-label="`Rename ${f.name}`">Rename</button>
              <button type="button" class="danger" @click="remove('files', f)" :aria-label="`Delete ${f.name}`">Delete</button>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-else-if="search && !loading && !error" class="no-results">No files or folders match “{{ search }}”.</p>
      <EmptyState v-else-if="!loading && !error" @upload="fileInput.click()" @new-folder="newFolder" />
    </main>

    <FilePreview v-if="preview" :file="preview" @close="preview = null" />
    <ShareDialog v-if="sharing" :file="sharing.item" :kind="sharing.kind" @close="sharing = null; refreshUsage()" />
  </DropZone>
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
.user { color: var(--muted); }
.content { max-width: 1000px; margin: 0 auto; padding: 1rem; }
.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 1rem;
}
.crumbs { display: flex; flex-wrap: wrap; align-items: center; gap: 0.4rem; font-size: 1.05rem; }
.crumbs a { text-decoration: none; }
.crumbs a:hover { text-decoration: underline; }
.sep { color: var(--muted); }
.actions { display: flex; flex-wrap: wrap; gap: 0.5rem; }
.search { width: 14rem; max-width: 100%; }
.no-results { padding: 2rem 0; text-align: center; color: var(--muted); }
.uploads {
  list-style: none;
  margin: 0 0 1rem;
  padding: 0.75rem;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
}
.uploads li { display: flex; align-items: center; gap: 0.75rem; padding: 0.25rem 0; }
.uploads .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.uploads .error { margin: 0; }
.uploads progress { width: 40%; }
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
tbody tr:hover { background: var(--bg); }
td.name { word-break: break-all; }
td.name .icon { display: inline-block; }
.icon { margin-right: 0.6rem; vertical-align: middle; }
.views { display: inline-flex; border: 1px solid var(--border); border-radius: 6px; overflow: hidden; }
.views button { display: grid; place-items: center; padding: 0.4rem 0.55rem; border: 0; border-radius: 0; color: var(--muted); }
.views button + button { border-left: 1px solid var(--border); }
.views button[aria-pressed='true'] { background: var(--bg); color: var(--accent); }
.views svg { width: 18px; height: 18px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linejoin: round; }
.thumb {
  width: 40px;
  height: 40px;
  margin-right: 0.6rem;
  padding: 0;
  vertical-align: middle;
  overflow: hidden;
  border-radius: 6px;
  background: var(--bg);
}
.thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
.num { text-align: right; white-space: nowrap; }
.date { white-space: nowrap; color: var(--muted); }
.row-actions { text-align: right; white-space: nowrap; }
.row-actions > * { margin-left: 0.35rem; }
.row-actions button, .btn { padding: 0.25rem 0.6rem; font-size: 0.85rem; }
.btn {
  display: inline-block;
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  text-decoration: none;
}
.btn:hover { background: var(--bg); }
.danger { color: var(--danger); }
.link {
  border: 0;
  padding: 0;
  background: none;
  color: var(--accent);
  text-align: left;
  word-break: break-all;
}
.link:hover { background: none; text-decoration: underline; }
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
}
@media (max-width: 480px) {
  .topbar-meter { display: none; }
}
@media (max-width: 640px) {
  .date { display: none; }
  .row-actions { white-space: normal; }
  .row-actions > * { margin: 0.15rem 0 0.15rem 0.35rem; }
}
</style>
