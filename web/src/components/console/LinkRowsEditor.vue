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

let nextRowId = 0;
const draggingIndex = ref<number | null>(null);
const dropTargetIndex = ref<number | null>(null);
const rowIds = ref<number[]>([]);

function allocateRowId(): number {
  nextRowId += 1;
  return nextRowId;
}

function ensureRowIds(): void {
  if (rowIds.value.length === 0 && props.rows.length > 0) {
    rowIds.value = props.rows.map(() => allocateRowId());
    return;
  }

  if (rowIds.value.length < props.rows.length) {
    rowIds.value = [
      ...rowIds.value,
      ...Array.from({ length: props.rows.length - rowIds.value.length }, () => allocateRowId()),
    ];
    return;
  }

  if (rowIds.value.length > props.rows.length) {
    rowIds.value = rowIds.value.slice(0, props.rows.length);
  }
}

ensureRowIds();

watch(
  () => props.rows.length,
  (length) => {
    ensureRowIds();

    if (length === 0) {
      rowIds.value = [];
    }

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
  const nextIds = [...rowIds.value];
  if (nextRows.length === 1) {
    nextRows[0] = "";
  } else {
    nextRows.splice(index, 1);
    nextIds.splice(index, 1);
  }

  rowIds.value = nextIds.length > 0 ? nextIds : [allocateRowId()];
  emitRows(nextRows);
  resetDragState();
}

function moveRow(index: number, offset: number): void {
  const target = index + offset;
  if (target < 0 || target >= props.rows.length) {
    return;
  }

  const nextRows = [...props.rows];
  const nextIds = [...rowIds.value];
  const [moved] = nextRows.splice(index, 1);
  const [movedId] = nextIds.splice(index, 1);
  nextRows.splice(target, 0, moved);
  nextIds.splice(target, 0, movedId);
  rowIds.value = nextIds;
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
  const nextIds = [...rowIds.value];
  const [moved] = nextRows.splice(draggingIndex.value, 1);
  const [movedId] = nextIds.splice(draggingIndex.value, 1);
  nextRows.splice(index, 0, moved);
  nextIds.splice(index, 0, movedId);
  rowIds.value = nextIds;
  emitRows(nextRows);
  resetDragState();
}

</script>

<template>
  <div class="link-row-list">
    <LinkRowItem
      v-for="(row, index) in props.rows"
      :key="rowIds[index] ?? index"
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
