<script setup lang="ts">
defineProps<{
  compact?: boolean;
}>();
</script>

<template>
  <div :class="['app-shell', compact ? 'app-shell--compact' : 'app-shell--console']">
    <div class="app-shell__chrome"></div>
    <div class="app-shell__body">
      <slot />
    </div>
  </div>
</template>
