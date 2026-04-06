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
    <main class="login-layout">
      <section class="login-hero">
        <p class="eyebrow">submora</p>
        <h1 class="hero-title">管理台入口</h1>
        <p class="hero-copy">
          Vue 3 管理台已经接通会话、订阅组、编辑器与运行状态。这里保留最直接的管理员入口。
        </p>
        <div class="hero-tags">
          <span class="chip chip--soft">Vue 3</span>
          <span class="chip chip--soft">Vite</span>
          <span class="chip chip--soft">TypeScript</span>
        </div>
      </section>

      <section class="panel-surface login-panel">
        <div class="login-panel__head">
          <p class="eyebrow">login</p>
          <h2 class="section-title">管理员登录</h2>
          <p class="section-copy">
            会话恢复、登录和 CSRF 已全部接通。登录成功后会直接进入控制台。
          </p>
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
              class="button button--primary"
              data-testid="login-submit"
              type="submit"
              :disabled="pending.pending.login || sessionLoading"
            >
              {{ pending.pending.login ? "登录中…" : "登录" }}
            </button>
            <RouterLink class="button button--ghost" to="/console">控制台</RouterLink>
          </div>
        </form>
      </section>
    </main>
  </AppShell>
</template>
