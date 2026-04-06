<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";

import AppShell from "@/components/shell/AppShell.vue";
import { usePendingMap, useSession } from "@/composables";

const router = useRouter();
const session = useSession();
const {
  login,
  sessionLoading,
  sessionStatus,
  sessionError,
} = session;
const pending = usePendingMap();

const loginUsername = ref("admin");
const loginPassword = ref("");

async function submitLogin(): Promise<void> {
  try {
    await login(loginUsername.value, loginPassword.value);
    loginPassword.value = "";
    await router.replace("/console");
  } catch {
    // Feedback is already surfaced through the shared toast viewport.
  }
}
</script>

<template>
  <AppShell compact>
    <main class="login-layout login-layout--single">
      <section class="panel-surface login-panel login-panel--compact">
        <div class="login-panel__head">
          <p class="eyebrow">submora</p>
          <h1 class="section-title">登录</h1>
          <p class="section-copy">使用管理员账号进入控制台。</p>
        </div>

        <form class="form-stack" @submit.prevent="submitLogin">
          <label class="field">
            <span>用户名</span>
            <input
              v-model="loginUsername"
              data-testid="login-username"
              autocomplete="username"
              placeholder="admin"
              :disabled="pending.pending.login || sessionLoading"
            />
          </label>
          <label class="field">
            <span>密码</span>
            <input
              v-model="loginPassword"
              data-testid="login-password"
              type="password"
              autocomplete="current-password"
              placeholder="••••••••"
              :disabled="pending.pending.login || sessionLoading"
            />
          </label>
          <p v-if="sessionStatus === 'loading'" class="section-copy">正在检查现有会话…</p>
          <p v-else-if="sessionStatus === 'error' && sessionError" class="inline-error">
            {{ sessionError }}
          </p>
          <div class="button-row">
            <button
              class="button button--primary button--block"
              data-testid="login-submit"
              type="submit"
              :disabled="pending.pending.login || sessionLoading"
            >
              {{ pending.pending.login ? "登录中…" : "登录" }}
            </button>
          </div>
        </form>
      </section>
    </main>
  </AppShell>
</template>
