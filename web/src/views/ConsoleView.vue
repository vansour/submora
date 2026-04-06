<script setup lang="ts">
import { computed, watch } from "vue";
import { useRoute, useRouter } from "vue-router";

import EditorWorkspace from "@/components/console/EditorWorkspace.vue";
import Topbar from "@/components/console/Topbar.vue";
import UserListPane from "@/components/console/UserListPane.vue";
import AppShell from "@/components/shell/AppShell.vue";
import {
  moveUsernameBefore,
  reorderedUsernames,
  useEditorDrafts,
  usePendingMap,
  useSession,
  useUserRuntime,
  useUsers,
} from "@/composables";

const route = useRoute();
const router = useRouter();
const session = useSession();
const { currentUser, sessionLoading } = session;
const pending = usePendingMap();
const users = useUsers();
const {
  users: usersList,
  usersStatus,
  usersError,
} = users;
const drafts = useEditorDrafts();
const runtime = useUserRuntime();

const selectedUsername = computed(() => {
  const value = route.params.username;
  return typeof value === "string" ? value : null;
});

watch(
  () => currentUser.value?.username ?? null,
  async (username) => {
    if (username === null) {
      users.reset();
      drafts.reset();
      runtime.reset();
      return;
    }

    try {
      await users.loadUsers();
    } catch {
      // Error state is already stored in the users composable.
    }
  },
  { immediate: true },
);

watch(
  [
    selectedUsername,
    () => users.usersStatus.value,
    () => users.users.value.map((user) => user.username).join("\0"),
  ],
  async ([username, status]) => {
    drafts.setSelectedUsername(username);

    if (username === null) {
      runtime.reset();
      return;
    }

    if (status !== "ready") {
      return;
    }

    if (!users.users.value.some((user) => user.username === username)) {
      await router.replace("/console");
      return;
    }

    try {
      await runtime.loadSelectedData();
    } catch {
      // Error state is already stored in the runtime composable.
    }
  },
  { immediate: true },
);

async function logout(): Promise<void> {
  try {
    await session.logout();
    await router.replace("/login");
  } catch {
    // Feedback is already surfaced through the shared toast viewport.
  }
}

async function openUser(username: string): Promise<void> {
  drafts.setSelectedUsername(username);
  await router.push(`/users/${username}`);
}

async function createUser(username: string): Promise<void> {
  try {
    const user = await users.createUser(username, false);
    await openUser(user.username);
  } catch {
    // Feedback is already surfaced through the shared toast viewport.
  }
}

async function deleteUser(username: string): Promise<void> {
  try {
    await users.deleteUser(username);

    if (selectedUsername.value === username) {
      await router.replace("/console");
    }
  } catch {
    // Feedback is already surfaced through the shared toast viewport.
  }
}

async function moveUser(username: string, offset: number): Promise<void> {
  const order = reorderedUsernames(users.users.value, username, offset);
  if (order === null) {
    return;
  }

  try {
    await users.updateOrder(order);
  } catch {
    // Feedback is already surfaced through the shared toast viewport.
  }
}

async function dropUserBefore(payload: {
  draggedUsername: string;
  beforeUsername: string;
}): Promise<void> {
  const order = moveUsernameBefore(
    users.users.value,
    payload.draggedUsername,
    payload.beforeUsername,
  );
  if (order === null) {
    return;
  }

  try {
    await users.updateOrder(order);
  } catch {
    // Feedback is already surfaced through the shared toast viewport.
  }
}
</script>

<template>
  <AppShell>
    <main v-if="currentUser" class="console-page">
      <Topbar
        title="submora"
        :username="currentUser.username"
        :logout-pending="pending.pending.logout"
        @logout="logout"
      />

      <div class="console-frame">
        <UserListPane
          :users="usersList"
          :status="usersStatus"
          :error="usersError"
          :selected-username="selectedUsername"
          :create-pending="pending.pending.createUser"
          :reorder-pending="pending.pending.reorderUsers"
          :delete-pending="pending.pending.deleteUser"
          @create="createUser"
          @select="openUser"
          @move-up="moveUser($event, -1)"
          @move-down="moveUser($event, 1)"
          @delete="deleteUser"
          @reload="users.loadUsers()"
          @drop-before="dropUserBefore"
        />

        <section class="console-main">
          <EditorWorkspace :username="selectedUsername" />
        </section>
      </div>
    </main>
    <main v-else class="console-page console-page--empty">
      <section class="panel-surface workspace-pane">
        <p class="eyebrow">session</p>
        <h2 class="section-title">
          {{ sessionLoading ? "恢复会话中" : "当前未登录" }}
        </h2>
        <p class="section-copy">
          {{ sessionLoading ? "正在检查当前登录状态。" : "请先登录后再访问控制台。" }}
        </p>
        <div class="button-row">
          <RouterLink class="button button--primary" to="/login">前往登录</RouterLink>
        </div>
      </section>
    </main>

  </AppShell>
</template>
