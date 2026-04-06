<script setup lang="ts">
const props = defineProps<{
  title: string;
  subtitle: string;
  username?: string | null;
  selectedLabel?: string | null;
  logoutPending?: boolean;
}>();

const emit = defineEmits<{
  logout: [];
}>();
</script>

<template>
  <header class="topbar">
    <div class="topbar__brand">
      <span class="topbar__mark" aria-hidden="true"></span>
      <strong class="topbar__logo">{{ props.title }}</strong>
      <span class="topbar__badge">{{ props.subtitle }}</span>
    </div>
    <div class="topbar__meta">
      <span v-if="props.selectedLabel" class="chip chip--soft">{{ props.selectedLabel }}</span>
      <span class="chip chip--solid">{{ props.username ?? "匿名会话" }}</span>
      <button
        class="button button--danger button--icon topbar__icon-button"
        data-testid="topbar-logout"
        type="button"
        :disabled="props.logoutPending"
        aria-label="退出登录"
        @click="emit('logout')"
      >
        <span v-if="props.logoutPending" class="topbar__icon-text">…</span>
        <svg v-else class="topbar__icon-svg" viewBox="0 0 20 20" aria-hidden="true">
          <path
            d="M8 4.5H5.5A1.5 1.5 0 0 0 4 6v8a1.5 1.5 0 0 0 1.5 1.5H8"
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.6"
          />
          <path
            d="M11 6.5 15 10l-4 3.5M15 10H8"
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.6"
          />
        </svg>
      </button>
    </div>
  </header>
</template>
