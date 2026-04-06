<script setup lang="ts">
const props = defineProps<{
  title: string;
  username?: string | null;
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
    </div>
    <div class="topbar__meta">
      <span class="topbar__user">{{ props.username ?? "匿名会话" }}</span>
      <button
        class="button button--ghost topbar__logout"
        data-testid="topbar-logout"
        type="button"
        :disabled="props.logoutPending"
        aria-label="退出登录"
        @click="emit('logout')"
      >
        {{ props.logoutPending ? "退出中…" : "退出" }}
      </button>
    </div>
  </header>
</template>
