<script setup lang="ts">
import type { UserSummary } from "@/api/types";

const props = defineProps<{
  user: UserSummary;
  selected: boolean;
  reorderPending: boolean;
  deletePending: boolean;
  canMoveUp: boolean;
  canMoveDown: boolean;
}>();

const emit = defineEmits<{
  select: [username: string];
  moveUp: [username: string];
  moveDown: [username: string];
  delete: [username: string];
  dragStart: [username: string];
  dragEnd: [];
  dropBefore: [username: string];
}>();

function onKeydown(event: KeyboardEvent): void {
  if (props.reorderPending) {
    return;
  }

  if (event.key === "ArrowUp" && props.canMoveUp) {
    event.preventDefault();
    emit("moveUp", props.user.username);
  }

  if (event.key === "ArrowDown" && props.canMoveDown) {
    event.preventDefault();
    emit("moveDown", props.user.username);
  }
}
</script>

<template>
  <article
    :data-testid="`user-item-${props.user.username}`"
    :class="['user-list-item', selected && 'user-list-item--selected']"
    @dragover.prevent
    @drop="emit('dropBefore', props.user.username)"
  >
    <button
      class="drag-handle drag-handle--button user-list-item__drag"
      :data-testid="`user-item-${props.user.username}-drag`"
      type="button"
      :disabled="props.reorderPending"
      draggable="true"
      tabindex="0"
      aria-label="拖拽或使用方向键调整顺序"
      @dragstart="emit('dragStart', props.user.username)"
      @dragend="emit('dragEnd')"
      @keydown="onKeydown"
    >
      ⋮⋮
    </button>

    <button
      :data-testid="`user-item-${props.user.username}-select`"
      class="user-list-item__main"
      type="button"
      @click="emit('select', props.user.username)"
    >
      <span class="user-list-item__name">{{ props.user.username }}</span>
    </button>

    <div class="user-list-item__actions">
      <button
        class="button button--danger button--compact user-list-item__delete"
        :data-testid="`user-item-${props.user.username}-delete`"
        type="button"
        :disabled="props.deletePending"
        @click="emit('delete', props.user.username)"
        aria-label="删除"
      >
        删除
      </button>
    </div>
  </article>
</template>
