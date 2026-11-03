<script setup>
import { toasts, dismissToast } from '../ui/toast'

const icons = { success: '✓', error: '!', info: 'i' }
</script>

<template>
  <TransitionGroup tag="div" name="toast" class="host" aria-live="polite">
    <div
      v-for="t in toasts"
      :key="t.id"
      class="toast"
      :class="t.type"
      :role="t.type === 'error' ? 'alert' : 'status'"
    >
      <span class="icon" aria-hidden="true">{{ icons[t.type] || icons.info }}</span>
      <span class="msg">{{ t.message }}</span>
      <button type="button" class="close" aria-label="Dismiss" @click="dismissToast(t.id)">×</button>
    </div>
  </TransitionGroup>
</template>

<style scoped>
.host {
  position: fixed;
  right: 1rem;
  bottom: 1rem;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: min(360px, calc(100vw - 2rem));
  pointer-events: none;
}
.toast {
  --c: var(--accent);
  display: flex;
  align-items: flex-start;
  gap: 0.6rem;
  padding: 0.7rem 0.6rem 0.7rem 0.8rem;
  border: 1px solid var(--border);
  border-left: 4px solid var(--c);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: 0 6px 20px rgb(0 0 0 / 0.12);
  pointer-events: auto;
}
.toast.success { --c: #1a7f37; }
.toast.error { --c: var(--danger); }
.icon {
  flex: none;
  display: grid;
  place-items: center;
  width: 1.25rem;
  height: 1.25rem;
  border-radius: 50%;
  background: var(--c);
  color: #fff;
  font-size: 0.75rem;
  font-weight: 700;
}
.msg { flex: 1; line-height: 1.35; overflow-wrap: anywhere; }
.close {
  flex: none;
  padding: 0 0.35rem;
  border: none;
  background: none;
  color: var(--muted);
  font-size: 1.2rem;
  line-height: 1;
}
.close:hover { color: var(--text); background: var(--bg); }

.toast-enter-active, .toast-leave-active { transition: opacity 0.2s, transform 0.2s ease-out; }
.toast-move { transition: transform 0.2s ease-out; }
.toast-enter-from { opacity: 0; transform: translateX(110%); }
.toast-leave-to { opacity: 0; transform: translateX(30%); }

@media (max-width: 480px) {
  .host { left: 1rem; right: 1rem; width: auto; }
  .toast-enter-from { transform: translateY(100%); }
  .toast-leave-to { transform: translateY(30%); }
}
@media (prefers-reduced-motion: reduce) {
  .toast-enter-active, .toast-leave-active, .toast-move { transition: opacity 0.2s; transform: none; }
}
</style>
