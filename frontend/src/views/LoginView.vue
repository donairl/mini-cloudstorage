<script setup>
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuth } from '../stores/auth'

const auth = useAuth()
const router = useRouter()
const isRegister = ref(false)
const username = ref('')
const password = ref('')
const showPassword = ref(false)
const error = ref('')
const busy = ref(false)
const title = computed(() => (isRegister.value ? 'Create your account' : 'Welcome back'))
const cta = computed(() => (isRegister.value ? 'Create account' : 'Sign in'))

const features = [
  { title: 'Private by design', text: 'Files never leave your server. No third parties, no tracking.' },
  { title: 'Organized your way', text: 'Nested folders, breadcrumbs and one-click rename.' },
  { title: 'Deploy in minutes', text: 'One binary or one Docker container. Back up a single volume.' },
]

async function submit() {
  error.value = ''
  busy.value = true
  try {
    if (isRegister.value) await auth.register(username.value, password.value)
    else await auth.login(username.value, password.value)
    router.push('/')
  } catch (e) {
    error.value = e.message
  } finally {
    busy.value = false
  }
}

function toggle() {
  isRegister.value = !isRegister.value
  error.value = ''
}
</script>

<template>
  <main class="login">
    <section class="hero" aria-label="About Mini-Drive">
      <div class="brand">
        <svg class="logo" viewBox="0 0 32 32" aria-hidden="true">
          <rect width="32" height="32" rx="8" fill="currentColor" />
          <path d="M10 21.5h12.2a4.3 4.3 0 0 0 .5-8.6 6 6 0 0 0-11.5 1.6A3.5 3.5 0 0 0 10 21.5Z" fill="#fff" />
        </svg>
        Mini-Drive
      </div>

      <div class="pitch">
        <p class="eyebrow">Self-hosted cloud storage</p>
        <h1>Your files.<br />Your server.<br /><span>Zero compromise.</span></h1>
        <p class="lead">
          Upload, organize and download from any browser, while every byte stays on hardware you control.
        </p>
        <ul class="features">
          <li v-for="f in features" :key="f.title">
            <svg viewBox="0 0 20 20" aria-hidden="true"><path d="m5 10.5 3 3 7-7" /></svg>
            <div>
              <strong>{{ f.title }}</strong>
              <span>{{ f.text }}</span>
            </div>
          </li>
        </ul>
      </div>

      <p class="stack">Built with Rust · SQLite · Vue</p>
    </section>

    <section class="panel">
      <form class="card" @submit.prevent="submit">
        <header>
          <h2>{{ title }}</h2>
          <p class="sub">
            {{ isRegister ? 'Start storing your files in under a minute.' : 'Sign in to access your files.' }}
          </p>
        </header>

        <label class="field">
          <span>Username</span>
          <input
            v-model="username"
            autocomplete="username"
            placeholder="e.g. alex"
            required
            :minlength="isRegister ? 3 : null"
            :maxlength="isRegister ? 32 : null"
          />
        </label>

        <label class="field">
          <span>Password</span>
          <div class="password">
            <input
              v-model="password"
              :type="showPassword ? 'text' : 'password'"
              :autocomplete="isRegister ? 'new-password' : 'current-password'"
              placeholder="••••••••"
              required
              :minlength="isRegister ? 8 : null"
            />
            <button
              type="button"
              class="reveal"
              :aria-label="showPassword ? 'Hide password' : 'Show password'"
              :aria-pressed="showPassword"
              @click="showPassword = !showPassword"
            >
              {{ showPassword ? 'Hide' : 'Show' }}
            </button>
          </div>
          <small v-if="isRegister" class="hint">At least 8 characters. Username 3–32 characters.</small>
        </label>

        <p v-if="error" class="error" role="alert">{{ error }}</p>

        <button class="primary submit" type="submit" :disabled="busy">
          <span v-if="busy" class="spinner" aria-hidden="true" />
          {{ busy ? 'Please wait…' : cta }}
        </button>

        <p class="switch">
          {{ isRegister ? 'Already have an account?' : 'New to Mini-Drive?' }}
          <button type="button" class="link" @click="toggle">
            {{ isRegister ? 'Sign in' : 'Create an account' }}
          </button>
        </p>
      </form>

      <p class="foot">Your data stays on this server.</p>
    </section>
  </main>
</template>

<style scoped>
.login {
  min-height: 100vh;
  display: grid;
  grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr);
}

/* Marketing side */
.hero {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 2.5rem;
  padding: 2.5rem 3.5rem;
  color: #e6edf6;
  background:
    radial-gradient(60rem 30rem at 0% 0%, rgba(56, 139, 253, 0.35), transparent 60%),
    radial-gradient(40rem 30rem at 100% 100%, rgba(130, 80, 223, 0.3), transparent 60%),
    #0b1220;
  overflow: hidden;
}
.hero::before {
  content: '';
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(255, 255, 255, 0.04) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.04) 1px, transparent 1px);
  background-size: 40px 40px;
  mask-image: linear-gradient(to bottom, #000, transparent 85%);
  pointer-events: none;
}
.hero > * { position: relative; }
.brand { display: flex; align-items: center; gap: 0.6rem; font-weight: 700; font-size: 1.15rem; }
.logo { width: 32px; height: 32px; color: #388bfd; }
.pitch { max-width: 32rem; }
.eyebrow {
  margin: 0 0 1rem;
  font-size: 0.8rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #79b8ff;
}
h1 { margin: 0; font-size: clamp(2rem, 3.4vw, 3rem); line-height: 1.1; letter-spacing: -0.02em; }
h1 span {
  background: linear-gradient(90deg, #79b8ff, #b392f0);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
.lead { margin: 1.25rem 0 2rem; font-size: 1.05rem; line-height: 1.6; color: #a9b6c8; }
.features { list-style: none; margin: 0; padding: 0; display: grid; gap: 1.1rem; }
.features li { display: flex; gap: 0.8rem; align-items: flex-start; }
.features svg {
  flex: none;
  width: 24px;
  height: 24px;
  padding: 3px;
  border-radius: 50%;
  background: rgba(56, 139, 253, 0.18);
  fill: none;
  stroke: #79b8ff;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.features strong { display: block; color: #fff; font-weight: 600; }
.features span { font-size: 0.92rem; color: #a9b6c8; }
.stack { margin: 0; font-size: 0.82rem; color: #6e7d91; }

/* Form side */
.panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1.5rem;
  padding: 2.5rem 1.5rem;
  background: var(--surface);
}
.card { width: 100%; max-width: 380px; display: flex; flex-direction: column; gap: 1.1rem; }
h2 { margin: 0; font-size: 1.6rem; letter-spacing: -0.01em; }
.sub { margin: 0.35rem 0 0.5rem; color: var(--muted); }
.field { display: flex; flex-direction: column; gap: 0.4rem; font-size: 0.9rem; font-weight: 500; }
.field input {
  width: 100%;
  padding: 0.7rem 0.8rem;
  border-radius: 8px;
  background: #fff;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.field input:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px rgba(9, 105, 218, 0.18); }
.password { position: relative; }
.password input { padding-right: 4rem; }
.reveal {
  position: absolute;
  top: 50%;
  right: 0.4rem;
  transform: translateY(-50%);
  padding: 0.25rem 0.5rem;
  border: 0;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--muted);
  background: none;
}
.reveal:hover { color: var(--text); background: var(--bg); }
.hint { font-weight: 400; color: var(--muted); }
.error {
  margin: 0;
  padding: 0.6rem 0.8rem;
  border: 1px solid rgba(207, 34, 46, 0.3);
  border-radius: 8px;
  background: rgba(207, 34, 46, 0.06);
  font-size: 0.9rem;
}
.submit {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.75rem;
  border-radius: 8px;
  font-weight: 600;
  box-shadow: 0 1px 2px rgba(9, 105, 218, 0.3), 0 6px 16px -6px rgba(9, 105, 218, 0.6);
}
.spinner {
  width: 1rem;
  height: 1rem;
  border: 2px solid rgba(255, 255, 255, 0.4);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.switch { margin: 0.25rem 0 0; text-align: center; font-size: 0.9rem; color: var(--muted); }
.link { border: 0; padding: 0; background: none; color: var(--accent); font-weight: 600; }
.link:hover { background: none; text-decoration: underline; }
.foot { margin: 0; font-size: 0.8rem; color: var(--muted); }

@media (max-width: 860px) {
  .login { grid-template-columns: 1fr; }
  .hero { padding: 1.5rem 1rem 2rem; gap: 1.5rem; }
  .features, .stack { display: none; }
  .lead { margin-bottom: 0; }
  .panel { padding: 2rem 1rem; justify-content: flex-start; }
}
@media (prefers-reduced-motion: reduce) {
  .spinner { animation-duration: 2s; }
}
</style>
