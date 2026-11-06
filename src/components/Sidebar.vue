<script setup lang="ts">
import { useApp, type Route } from '../store'

const store = useApp()

const items: { route: Route; label: string; icon: string }[] = [
  {
    route: 'single',
    label: 'Single',
    icon: 'M3 10.5 12 3l9 7.5V21h-6v-7h-6v7H3z',
  },
  {
    route: 'batch',
    label: 'Batch',
    icon: 'M12 3 3 8l9 5 9-5-9-5zM3 13l9 5 9-5M3 18l9 5 9-5',
  },
  {
    route: 'settings',
    label: 'Settings',
    icon: 'M12 8.8a3.2 3.2 0 1 0 0 6.4 3.2 3.2 0 0 0 0-6.4zM12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M19.1 4.9 17 7M7 17l-2.1 2.1',
  },
]
</script>

<template>
  <nav class="sidebar">
    <div class="rail">
      <button
        v-for="item in items"
        :key="item.route"
        class="rail-btn"
        :class="{ active: store.route === item.route }"
        :title="item.label"
        :aria-label="item.label"
        @click="store.route = item.route"
      >
        <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor"
             stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path :d="item.icon" />
        </svg>
      </button>
    </div>
    <div class="version muted">v{{ store.version }}</div>
  </nav>
</template>

<style scoped>
.sidebar {
  width: 64px;
  background: var(--panel);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 0;
  gap: 8px;
}

.rail {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rail-btn {
  width: 44px;
  height: 44px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 8px;
  color: var(--muted);
}

.rail-btn:hover {
  background: var(--panel-2);
  color: var(--text);
}

.rail-btn.active {
  background: var(--panel-2);
  color: var(--accent);
}

.version {
  margin-top: auto;
  font-size: 10px;
  writing-mode: horizontal-tb;
}
</style>
