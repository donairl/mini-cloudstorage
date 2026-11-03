import { ref } from 'vue'
import { defineStore } from 'pinia'
import * as api from '../api'

export const useAuth = defineStore('auth', () => {
  const user = ref(null)
  let me = null

  function fetchMe() {
    me ??= api.get('/api/auth/me').then(
      (u) => (user.value = u),
      () => (user.value = null),
    )
    return me
  }

  async function login(username, password) {
    user.value = await api.post('/api/auth/login', { username, password })
  }

  async function register(username, password) {
    await api.post('/api/auth/register', { username, password })
    await login(username, password)
  }

  async function logout() {
    try {
      await api.post('/api/auth/logout')
    } finally {
      user.value = null
    }
  }

  return { user, fetchMe, login, register, logout }
})
