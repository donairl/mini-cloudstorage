<script setup>
import { ref, onMounted } from 'vue'
import * as api from '../api'
import { toast } from '../ui/toast'
import { formatDate } from '../format'
import FileIcon from './FileIcon.vue'

// kind is the API path segment: 'files' or 'folders'.
const props = defineProps({ file: { type: Object, required: true }, kind: { type: String, default: 'files' } })
const what = props.kind === 'folders' ? 'folder' : 'file'
const emit = defineEmits(['close'])

const el = ref(null)
const days = ref(7)
const password = ref('')
const shares = ref([])
const loading = ref(true)
const busy = ref(false)
let downTarget = null

const base = `${location.origin}/s/`
const sharesUrl = `/api/${props.kind}/${props.file.id}/shares`
const isExpired = (s) => s.expires_at && new Date(s.expires_at.replace(' ', 'T') + 'Z') <= Date.now()

onMounted(async () => {
  el.value.showModal()
  try {
    shares.value = await api.get(sharesUrl)
  } catch (e) {
    toast(e.message, { type: 'error' })
  } finally {
    loading.value = false
  }
})

async function copy(s, msg = 'Link copied') {
  try {
    await navigator.clipboard.writeText(base + s.token)
    toast(msg)
  } catch {
    toast('Could not copy automatically. Select the link and copy it.', { type: 'error' })
  }
}

async function create() {
  busy.value = true
  try {
    const s = await api.post(sharesUrl, { expires_in_days: days.value, password: password.value || null })
    shares.value.unshift(s)
    password.value = ''
    await copy(s, 'Link created and copied')
  } catch (e) {
    toast(e.message, { type: 'error' })
  } finally {
    busy.value = false
  }
}

async function revoke(s) {
  try {
    await api.del(`/api/shares/${s.id}`)
    shares.value = shares.value.filter((x) => x.id !== s.id)
    toast('Link revoked')
  } catch (e) {
    toast(e.message, { type: 'error' })
  }
}

function onClick(e) {
  if (e.target === el.value && downTarget === el.value) el.value.close()
}
</script>

<template>
  <dialog
    ref="el"
    class="dlg"
    aria-labelledby="share-title"
    @close="emit('close')"
    @pointerdown="(e) => (downTarget = e.target)"
    @click="onClick"
  >
    <div class="body">
      <header>
        <FileIcon :item="file" :folder="kind === 'folders'" :size="36" />
        <div class="head-text">
          <h2 id="share-title">Share “{{ file.name }}”</h2>
          <p>Anyone with the link can {{ what === 'folder' ? 'browse this folder and download its files' : 'download this file' }}.</p>
        </div>
        <button type="button" class="x" aria-label="Close" @click="el.close()">×</button>
      </header>

      <form class="create" @submit.prevent="create">
        <label class="field">
          <span>Expires</span>
          <select v-model="days">
            <option :value="null">Never</option>
            <option :value="1">1 day</option>
            <option :value="7">7 days</option>
            <option :value="30">30 days</option>
          </select>
        </label>
        <label class="field">
          <span>Password <small>(optional)</small></span>
          <input
            v-model="password"
            type="password"
            autocomplete="new-password"
            placeholder="No password"
            minlength="4"
          />
        </label>
        <button type="submit" class="primary" :disabled="busy">{{ busy ? 'Creating…' : 'Create link' }}</button>
      </form>

      <section class="links" aria-label="Existing links">
        <h3>Active links <span v-if="shares.length" class="count">{{ shares.length }}</span></h3>
        <p v-if="loading" class="empty">Loading…</p>
        <p v-else-if="!shares.length" class="empty">No links yet. Create one above to share this {{ what }}.</p>
        <ul v-else>
          <li v-for="s in shares" :key="s.id" :class="{ expired: isExpired(s) }">
            <input class="url" :value="base + s.token" readonly aria-label="Share link" @focus="$event.target.select()" />
            <div class="row">
              <div class="badges">
                <span class="badge muted">{{ s.download_count }} download{{ s.download_count === 1 ? '' : 's' }}</span>
                <span v-if="s.has_password" class="badge">Password</span>
                <span v-if="isExpired(s)" class="badge red">Expired</span>
                <span v-else-if="s.expires_at" class="badge">Expires {{ formatDate(s.expires_at) }}</span>
                <span v-else class="badge muted">No expiry</span>
              </div>
              <div class="acts">
                <button type="button" :disabled="isExpired(s)" @click="copy(s)">Copy</button>
                <button type="button" class="danger" @click="revoke(s)">Revoke</button>
              </div>
            </div>
          </li>
        </ul>
      </section>

      <div class="foot">
        <button type="button" @click="el.close()">Close</button>
      </div>
    </div>
  </dialog>
</template>

<style scoped>
.dlg {
  width: min(540px, calc(100vw - 2rem));
  max-width: none;
  max-height: calc(100dvh - 2rem);
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface);
  color: var(--text);
  box-shadow: 0 12px 40px rgb(0 0 0 / 0.18);
}
.dlg::backdrop {
  background: rgb(15 20 25 / 0.35);
  backdrop-filter: blur(3px);
}
.dlg[open] { animation: pop 0.16s ease-out; }
.dlg[open]::backdrop { animation: fade 0.16s ease-out; }
@keyframes pop {
  from { opacity: 0; transform: translateY(8px) scale(0.97); }
}
@keyframes fade {
  from { opacity: 0; }
}
@media (prefers-reduced-motion: reduce) {
  .dlg[open], .dlg[open]::backdrop { animation: none; }
}
.body { padding: 1.25rem; display: grid; gap: 1.1rem; }

header { display: flex; gap: 0.75rem; align-items: flex-start; }
.head-text { flex: 1; min-width: 0; }
h2 { margin: 0; font-size: 1.1rem; overflow-wrap: anywhere; }
header p { margin: 0.2rem 0 0; font-size: 0.88rem; color: var(--muted); }
.x {
  flex: none;
  width: 2rem;
  height: 2rem;
  padding: 0;
  border: 0;
  font-size: 1.4rem;
  line-height: 1;
  color: var(--muted);
  background: none;
}
.x:hover { color: var(--text); background: var(--bg); }

.create {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.3fr) auto;
  gap: 0.6rem;
  align-items: end;
  padding: 0.9rem;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
}
.field { display: grid; gap: 0.3rem; font-size: 0.82rem; font-weight: 500; color: var(--muted); }
.field small { font-weight: 400; }
.field input, .field select { width: 100%; min-width: 0; color: var(--text); background: var(--surface); }
select {
  font: inherit;
  padding: 0.5rem 0.6rem;
  border: 1px solid var(--border);
  border-radius: 6px;
}
.create .primary { padding: 0.5rem 0.9rem; font-weight: 600; white-space: nowrap; }

h3 { display: flex; align-items: center; gap: 0.4rem; margin: 0 0 0.6rem; font-size: 0.9rem; }
.count {
  padding: 0 0.45rem;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--muted);
  background: var(--bg);
  border: 1px solid var(--border);
}
.empty { margin: 0; padding: 1rem; text-align: center; font-size: 0.88rem; color: var(--muted); border: 1px dashed var(--border); border-radius: 10px; }
ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.5rem; max-height: 40dvh; overflow-y: auto; }
li { display: grid; gap: 0.5rem; padding: 0.7rem; border: 1px solid var(--border); border-radius: 10px; }
li.expired .url { color: var(--muted); text-decoration: line-through; }
.url {
  width: 100%;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.8rem;
  color: var(--text);
  background: var(--bg);
  text-overflow: ellipsis;
}
.row { display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: center; justify-content: space-between; }
.badges { display: flex; flex-wrap: wrap; gap: 0.35rem; }
.badge {
  padding: 0.12rem 0.5rem;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--accent);
  background: rgb(9 105 218 / 0.08);
  border: 1px solid rgb(9 105 218 / 0.2);
}
.badge.muted { color: var(--muted); background: var(--bg); border-color: var(--border); }
.badge.red { color: var(--danger); background: rgb(207 34 46 / 0.08); border-color: rgb(207 34 46 / 0.25); }
.acts { display: flex; gap: 0.4rem; margin-left: auto; }
.acts button { padding: 0.3rem 0.7rem; font-size: 0.85rem; }
.acts .danger { color: var(--danger); }
.acts .danger:hover { background: rgb(207 34 46 / 0.08); border-color: rgb(207 34 46 / 0.35); }

.foot { display: flex; justify-content: flex-end; }

@media (max-width: 520px) {
  .body { padding: 1rem; }
  .create { grid-template-columns: 1fr; }
  .create .primary { padding: 0.6rem; }
}
</style>
