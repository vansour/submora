<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{
  open: boolean;
  username?: string | null;
  pending?: boolean;
}>();

const emit = defineEmits<{
  close: [];
  submit: [payload: { accountUsername: string; currentPassword: string; newPassword: string }];
}>();

const accountUsername = ref("");
const currentPassword = ref("");
const newPassword = ref("");

function resetForm(): void {
  accountUsername.value = "";
  currentPassword.value = "";
  newPassword.value = "";
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      resetForm();
    }
  },
);

function submit(): void {
  emit("submit", {
    accountUsername: accountUsername.value,
    currentPassword: currentPassword.value,
    newPassword: newPassword.value,
  });
}
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="dialog-backdrop" data-testid="account-dialog" @click="emit('close')">
      <section class="dialog panel-surface" @click.stop>
        <div class="dialog__head">
          <div>
            <p class="eyebrow">account</p>
            <h2 class="dialog__title">管理员账户</h2>
            <p class="dialog__subtitle">
              支持仅改用户名或仅改密码，但每次更新都必须填写当前密码。提交成功后会立即要求重新登录。
            </p>
          </div>
          <button class="button button--ghost" type="button" @click="emit('close')">关闭</button>
        </div>

        <form class="dialog__form" @submit.prevent="submit">
          <label class="field">
            <span>当前用户名</span>
            <input :value="props.username ?? 'admin'" disabled />
          </label>
          <label class="field">
            <span>新用户名</span>
            <input
              v-model="accountUsername"
              data-testid="account-new-username"
              :disabled="props.pending"
              :placeholder="props.username ?? '留空则保持当前用户名'"
            />
          </label>
          <label class="field">
            <span>当前密码</span>
            <input
              v-model="currentPassword"
              data-testid="account-current-password"
              :disabled="props.pending"
              type="password"
              placeholder="必填"
              autocomplete="current-password"
            />
          </label>
          <label class="field">
            <span>新密码</span>
            <input
              v-model="newPassword"
              data-testid="account-new-password"
              :disabled="props.pending"
              type="password"
              placeholder="留空则不修改密码"
              autocomplete="new-password"
            />
          </label>
          <div class="button-row">
            <button
              class="button button--primary"
              data-testid="account-submit"
              type="submit"
              :disabled="props.pending"
            >
              {{ props.pending ? "更新中…" : "更新账户" }}
            </button>
          </div>
        </form>
      </section>
    </div>
  </Teleport>
</template>
