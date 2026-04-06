<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";

import { useSession } from "@/composables";

const route = useRoute();
const router = useRouter();
const session = useSession();
const sessionReady = ref(false);

const shouldRedirectToConsole = computed(
  () => sessionReady.value && session.isAuthenticated.value && route.path === "/login",
);

const shouldRedirectToLogin = computed(
  () => sessionReady.value && !session.isAuthenticated.value && route.path !== "/login",
);

onMounted(async () => {
  try {
    await session.restoreSession();
  } finally {
    sessionReady.value = true;
  }
});

watch(
  [shouldRedirectToConsole, shouldRedirectToLogin],
  async ([toConsole, toLogin]) => {
    if (toConsole) {
      await router.replace("/console");
      return;
    }

    if (toLogin) {
      await router.replace("/login");
    }
  },
  { immediate: true },
);
</script>

<template>
  <RouterView v-if="sessionReady" />
  <main v-else class="boot-screen">
    <section class="panel-surface boot-card">
      <p class="eyebrow">session</p>
      <h1 class="section-title">恢复会话中</h1>
      <p class="section-copy">正在读取当前登录状态并准备路由。</p>
    </section>
  </main>
</template>
