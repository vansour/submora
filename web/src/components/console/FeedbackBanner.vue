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
  <div v-if="hasStatus || hasError" class="feedback-banner-wrap">
    <section v-if="statusMessage" class="feedback-banner feedback-banner--success">
      <p>{{ statusMessage }}</p>
      <button class="button button--ghost button--compact" type="button" @click="clearStatus()">
        关闭
      </button>
    </section>
    <section v-if="errorMessage" class="feedback-banner feedback-banner--error">
      <p>{{ errorMessage }}</p>
      <button class="button button--ghost button--compact" type="button" @click="clearError()">
        关闭
      </button>
    </section>
  </div>
</template>
