<script setup lang="ts">
import { computed, ref } from "vue";

import type { AsyncStatus } from "@/composables";
import type { UserSummary } from "@/api/types";

import UserListItem from "@/components/console/UserListItem.vue";

const props = defineProps<{
  users: UserSummary[];
  status: AsyncStatus;
  error: string | null;
  selectedUsername: string | null;
  createPending: boolean;
  reorderPending: boolean;
  deletePending: boolean;
}>();

const emit = defineEmits<{
  create: [username: string];
  select: [username: string];
  moveUp: [username: string];
  moveDown: [username: string];
  delete: [username: string];
  reload: [];
  dropBefore: [payload: { draggedUsername: string; beforeUsername: string }];
}>();

const createUsername = ref("");
const draggingUsername = ref<string | null>(null);

const isLoading = computed(() => props.status === "loading");
const isError = computed(() => props.status === "error");
const isEmpty = computed(() => props.status === "ready" && props.users.length === 0);

function submitCreate(): void {
  const username = createUsername.value.trim();
  if (username === "") {
    return;
  }

  emit("create", username);
  createUsername.value = "";
}

function onDelete(username: string): void {
  if (window.confirm(`确认删除订阅组 ${username}？`)) {
    emit("delete", username);
  }
}

function onDropBefore(beforeUsername: string): void {
  if (draggingUsername.value === null) {
    return;
  }

  emit("dropBefore", {
    draggedUsername: draggingUsername.value,
    beforeUsername,
  });
  draggingUsername.value = null;
}
</script>

<template>
  <aside class="sidebar-pane">
    <div class="pane-head sidebar-pane__head">
      <div class="sidebar-pane__head-copy">
        <h2 class="section-title">订阅组</h2>
        <p class="sidebar-pane__count">{{ props.users.length }} 个</p>
      </div>
    </div>

    <form id="create-user-form" class="create-user-form" @submit.prevent="submitCreate">
      <input
        v-model="createUsername"
        data-testid="create-user-input"
        class="create-user-form__input"
        :disabled="props.createPending"
        placeholder="新建订阅组"
      />
      <button
        class="button button--primary sidebar-pane__create"
        data-testid="create-user-submit"
        type="submit"
        :disabled="props.createPending"
      >
        <span class="sidebar-pane__create-icon" aria-hidden="true">+</span>
        <span>{{ props.createPending ? "新建中…" : "新建" }}</span>
      </button>
    </form>

    <div v-if="isLoading" class="empty-copy">
      <p class="section-copy">正在加载订阅组…</p>
    </div>

    <div v-else-if="isError" class="empty-copy">
      <p class="inline-error">{{ props.error }}</p>
      <div class="button-row">
        <button class="button button--ghost" type="button" @click="emit('reload')">重试</button>
      </div>
    </div>

    <div v-else-if="isEmpty" class="empty-copy">
      <p class="section-copy">还没有订阅组，先创建一个入口。</p>
    </div>

    <div v-else class="list-stack">
      <UserListItem
        v-for="(user, index) in props.users"
        :key="user.username"
        :user="user"
        :selected="props.selectedUsername === user.username"
        :reorder-pending="props.reorderPending"
        :delete-pending="props.deletePending"
        :can-move-up="index > 0"
        :can-move-down="index < props.users.length - 1"
        @select="emit('select', $event)"
        @move-up="emit('moveUp', $event)"
        @move-down="emit('moveDown', $event)"
        @delete="onDelete"
        @drag-start="draggingUsername = $event"
        @drag-end="draggingUsername = null"
        @drop-before="onDropBefore"
      />
    </div>

  </aside>
</template>
