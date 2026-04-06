<script setup lang="ts">
import { ref, watch } from "vue";

import LinkRowItem from "@/components/console/LinkRowItem.vue";

const props = defineProps<{
  rows: string[];
  disabled: boolean;
}>();

const emit = defineEmits<{
  changeRows: [rows: string[]];
}>();

const draggingIndex = ref<number | null>(null);
const dropTargetIndex = ref<number | null>(null);

watch(
  () => props.rows.length,
  (length) => {
    if (draggingIndex.value !== null && draggingIndex.value >= length) {
      draggingIndex.value = null;
    }

    if (dropTargetIndex.value !== null && dropTargetIndex.value >= length) {
      dropTargetIndex.value = null;
    }
  },
);

function emitRows(nextRows: string[]): void {
  emit("changeRows", nextRows);
}

function updateRow(index: number, value: string): void {
  const nextRows = [...props.rows];
  nextRows[index] = value;
  emitRows(nextRows);
}

function removeRow(index: number): void {
  const nextRows = [...props.rows];
  if (nextRows.length === 1) {
    nextRows[0] = "";
  } else {
    nextRows.splice(index, 1);
  }

  emitRows(nextRows);
  resetDragState();
}

function moveRow(index: number, offset: number): void {
  const target = index + offset;
  if (target < 0 || target >= props.rows.length) {
    return;
  }

  const nextRows = [...props.rows];
  const [moved] = nextRows.splice(index, 1);
  nextRows.splice(target, 0, moved);
  emitRows(nextRows);
  draggingIndex.value = target;
  dropTargetIndex.value = target;
}

function resetDragState(): void {
  draggingIndex.value = null;
  dropTargetIndex.value = null;
}

function dropAt(index: number): void {
  if (draggingIndex.value === null || draggingIndex.value === index) {
    resetDragState();
    return;
  }

  const nextRows = [...props.rows];
  const [moved] = nextRows.splice(draggingIndex.value, 1);
  nextRows.splice(index, 0, moved);
  emitRows(nextRows);
  resetDragState();
}

</script>

<template>
  <div class="link-row-list">
    <LinkRowItem
      v-for="(row, index) in props.rows"
      :key="index"
      :index="index"
      :value="row"
      :disabled="props.disabled"
      :dragging="draggingIndex === index"
      :drop-target="dropTargetIndex === index"
      :can-move-up="index > 0"
      :can-move-down="index < props.rows.length - 1"
      @update-value="updateRow(index, $event)"
      @remove="removeRow(index)"
      @move-up="moveRow(index, -1)"
      @move-down="moveRow(index, 1)"
      @drag-start="
        draggingIndex = index;
        dropTargetIndex = index;
      "
      @drag-end="resetDragState"
      @drag-enter="dropTargetIndex = index"
      @drag-leave="
        if (dropTargetIndex === index) {
          dropTargetIndex = null;
        }
      "
      @drop-at="dropAt(index)"
    />
  </div>
</template>
