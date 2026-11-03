import { reactive } from 'vue'

export const toasts = reactive([])
let seq = 0

export function toast(message, { type = 'success', timeout = type === 'error' ? 7000 : 4000 } = {}) {
  const id = ++seq
  toasts.push({ id, message, type })
  if (timeout > 0) setTimeout(() => dismissToast(id), timeout)
  return id
}

export function dismissToast(id) {
  const i = toasts.findIndex((t) => t.id === id)
  if (i !== -1) toasts.splice(i, 1)
}
