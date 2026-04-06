<script setup lang="ts">
import { useFeedback } from "@/composables";

const feedback = useFeedback();
const {
  hasStatus,
  hasError,
  statusMessage,
  errorMessage,
  clearStatus,
  clearError,
} = feedback;
</script>

<template>
  <div v-if="hasStatus || hasError" class="toast-viewport">
    <article v-if="statusMessage" class="toast toast--success">
      <div>
        <strong>操作完成</strong>
        <p>{{ statusMessage }}</p>
      </div>
      <button
        class="button button--ghost button--compact"
        data-testid="toast-close"
        type="button"
        @click="clearStatus()"
      >
        关闭
      </button>
    </article>
    <article v-if="errorMessage" class="toast toast--error">
      <div>
        <strong>操作失败</strong>
        <p>{{ errorMessage }}</p>
      </div>
      <button
        class="button button--ghost button--compact"
        data-testid="toast-close"
        type="button"
        @click="clearError()"
      >
        关闭
      </button>
    </article>
  </div>
</template>
