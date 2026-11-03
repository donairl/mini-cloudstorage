<script setup>
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import FileIcon from '../components/FileIcon.vue'
import { formatSize, formatDate } from '../format'

const token = encodeURIComponent(useRoute().params.token)
const dl = `/api/s/${token}/download`
const state = ref('loading') // loading | ok | missing | error
const file = ref(null)
const password = ref('')
const wrong = ref('')
const previewFailed = ref(false)
const frame = ref(null)

const preview = computed(
  () => file.value && !file.value.password_required && file.value.mime_type.startsWith('image/') && !previewFailed.value,
)

onMounted(async () => {
  try {
    const res = await fetch(`/api/s/${token}`)
    if (res.status === 404) return (state.value = 'missing')
    if (!res.ok) throw new Error()
    file.value = await res.json()
    document.title = `${file.value.name} · Mini-Drive`
    state.value = 'ok'
  } catch {
    state.value = 'error'
  }
})

// The password form posts into a hidden iframe: a correct password downloads (no load event),
// a wrong one loads the JSON error there, which we read and show inline.
function onFrameLoad() {
  let msg = ''
  try {
    msg = JSON.parse(frame.value.contentDocument.body.textContent).error
  } catch {
    return
  }
  if (msg) wrong.value = msg === 'wrong password' ? 'Incorrect password. Please try again.' : msg
}
</script>

<template>
  <main class="page">
    <div class="glow" aria-hidden="true" />
    <a class="brand" href="/">
      <svg class="logo" viewBox="0 0 32 32" aria-hidden="true">
        <rect width="32" height="32" rx="8" fill="currentColor" />
        <path d="M10 21.5h12.2a4.3 4.3 0 0 0 .5-8.6 6 6 0 0 0-11.5 1.6A3.5 3.5 0 0 0 10 21.5Z" fill="#fff" />
      </svg>
      Mini-Drive
    </a>

    <section class="card" aria-live="polite">
      <div v-if="state === 'loading'" class="center">
        <span class="spinner" aria-hidden="true" />
        <p class="muted">Loading shared file…</p>
      </div>

      <div v-else-if="state !== 'ok'" class="center">
        <div class="broken" aria-hidden="true">
          <svg viewBox="0 0 24 24"><path d="M10 14a4 4 0 0 0 5.66 0l3-3a4 4 0 0 0-5.66-5.66l-1 1M14 10a4 4 0 0 0-5.66 0l-3 3a4 4 0 0 0 5.66 5.66l1-1M4 4l16 16" /></svg>
        </div>
        <h1>{{ state === 'missing' ? 'This link is invalid or has expired' : 'Something went wrong' }}</h1>
        <p class="muted">
          {{ state === 'missing'
            ? 'Ask the person who shared it with you to send a new link.'
            : 'We couldn’t load this file. Check your connection and try again.' }}
        </p>
      </div>

      <template v-else>
        <div class="visual" :class="{ preview }">
          <img v-if="preview" :src="dl + '?preview'" :alt="file.name" @error="previewFailed = true" />
          <FileIcon v-else :item="file" :size="88" />
        </div>
        <p class="eyebrow">Someone shared a file with you</p>
        <h1 class="name">{{ file.name }}</h1>
        <p class="meta">
          <span>{{ formatSize(file.size_bytes) }}</span>
          <template v-if="file.expires_at">
            <span class="dot" aria-hidden="true">·</span>
            <span>Expires {{ formatDate(file.expires_at) }}</span>
          </template>
        </p>

        <a v-if="!file.password_required" class="btn" :href="dl" download>
          <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M10 3v10m0 0-4-4m4 4 4-4M4 16h12" /></svg>
          Download
        </a>
        <form v-else class="unlock" method="post" :action="dl" target="share-dl" @submit="wrong = ''">
          <label class="field">
            <span>
              <svg viewBox="0 0 20 20" aria-hidden="true"><rect x="4" y="9" width="12" height="8" rx="2" /><path d="M7 9V6.5a3 3 0 0 1 6 0V9" /></svg>
              This file is password protected
            </span>
            <input
              v-model="password"
              name="password"
              type="password"
              autocomplete="current-password"
              placeholder="Enter password"
              required
              :aria-invalid="!!wrong"
              aria-describedby="pw-err"
            />
          </label>
          <p v-if="wrong" id="pw-err" class="err" role="alert">{{ wrong }}</p>
          <button class="btn" type="submit">
            <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M10 3v10m0 0-4-4m4 4 4-4M4 16h12" /></svg>
            Download
          </button>
          <iframe ref="frame" name="share-dl" title="Download" hidden @load="onFrameLoad" />
        </form>
      </template>
    </section>

    <p class="foot">
      <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M10 2.5 4 5v4.5c0 3.7 2.6 6.9 6 8 3.4-1.1 6-4.3 6-8V5z" /></svg>
      Shared securely with Mini-Drive
    </p>
  </main>
</template>

<style scoped>
.page {
  position: relative;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1.5rem;
  padding: 2rem 1rem;
  overflow: hidden;
  background: var(--bg);
}
.glow {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(40rem 22rem at 50% -10%, rgb(56 139 253 / 0.18), transparent 70%),
    radial-gradient(30rem 20rem at 100% 100%, rgb(130 80 223 / 0.1), transparent 70%);
  pointer-events: none;
}
.page > :not(.glow) { position: relative; }
.brand {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  font-weight: 700;
  font-size: 1.1rem;
  color: var(--text);
  text-decoration: none;
}
.logo { width: 30px; height: 30px; color: var(--accent); }

.card {
  width: 100%;
  max-width: 420px;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 2rem 1.75rem;
  border: 1px solid var(--border);
  border-radius: 16px;
  background: var(--surface);
  box-shadow: 0 1px 2px rgb(0 0 0 / 0.04), 0 20px 50px -20px rgb(15 30 60 / 0.25);
}
.center { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 1rem 0; }
.visual {
  display: grid;
  place-items: center;
  width: 128px;
  height: 128px;
  margin-bottom: 1.25rem;
  border-radius: 20px;
  background: var(--bg);
  border: 1px solid var(--border);
}
.visual.preview { width: 100%; height: auto; max-height: 260px; padding: 0; overflow: hidden; }
.visual img { display: block; max-width: 100%; max-height: 260px; object-fit: contain; }
.eyebrow {
  margin: 0 0 0.35rem;
  font-size: 0.75rem;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--accent);
}
h1 { margin: 0; font-size: 1.25rem; line-height: 1.3; letter-spacing: -0.01em; }
.name { overflow-wrap: anywhere; }
.meta { display: flex; flex-wrap: wrap; justify-content: center; gap: 0.4rem; margin: 0.4rem 0 1.5rem; font-size: 0.9rem; color: var(--muted); }
.muted { margin: 0; color: var(--muted); line-height: 1.5; }

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.75rem;
  border: 1px solid var(--accent);
  border-radius: 8px;
  font: inherit;
  font-weight: 600;
  color: #fff;
  background: var(--accent);
  text-decoration: none;
  cursor: pointer;
  box-shadow: 0 1px 2px rgb(9 105 218 / 0.3), 0 6px 16px -6px rgb(9 105 218 / 0.6);
}
.btn:hover { filter: brightness(1.1); background: var(--accent); }
.btn svg, .field svg, .foot svg {
  width: 18px;
  height: 18px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.unlock { width: 100%; display: grid; gap: 0.75rem; text-align: left; }
.field { display: grid; gap: 0.45rem; font-size: 0.88rem; font-weight: 500; }
.field span { display: flex; align-items: center; gap: 0.4rem; color: var(--muted); }
.field input {
  width: 100%;
  padding: 0.7rem 0.8rem;
  border-radius: 8px;
  background: #fff;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.field input:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px rgb(9 105 218 / 0.18); }
.field input[aria-invalid='true'] { border-color: var(--danger); }
.err {
  margin: 0;
  padding: 0.55rem 0.75rem;
  border: 1px solid rgb(207 34 46 / 0.3);
  border-radius: 8px;
  background: rgb(207 34 46 / 0.06);
  font-size: 0.88rem;
  color: var(--danger);
}

.broken {
  display: grid;
  place-items: center;
  width: 64px;
  height: 64px;
  margin-bottom: 0.5rem;
  border-radius: 50%;
  color: var(--muted);
  background: var(--bg);
  border: 1px solid var(--border);
}
.broken svg { width: 28px; height: 28px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; }

.spinner {
  width: 28px;
  height: 28px;
  border: 3px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

.foot { display: flex; align-items: center; gap: 0.35rem; margin: 0; font-size: 0.8rem; color: var(--muted); }
.foot svg { width: 14px; height: 14px; }

@media (max-width: 420px) {
  .card { padding: 1.5rem 1.1rem; }
}
@media (prefers-reduced-motion: reduce) {
  .spinner { animation-duration: 2s; }
}
</style>
