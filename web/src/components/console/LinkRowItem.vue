<script setup lang="ts">
const props = defineProps<{
  index: number;
  value: string;
  disabled: boolean;
  invalid: boolean;
  dragging: boolean;
  dropTarget: boolean;
  canMoveUp: boolean;
  canMoveDown: boolean;
}>();

const emit = defineEmits<{
  updateValue: [value: string];
  remove: [];
  moveUp: [];
  moveDown: [];
  dragStart: [];
  dragEnd: [];
  dragEnter: [];
  dragLeave: [];
  dropAt: [];
}>();

function onInput(event: Event): void {
  emit("updateValue", (event.target as HTMLInputElement).value);
}

function onKeydown(event: KeyboardEvent): void {
  if (props.disabled) {
    return;
  }

  if (event.key === "ArrowUp" && props.canMoveUp) {
    event.preventDefault();
    emit("moveUp");
  }

  if (event.key === "ArrowDown" && props.canMoveDown) {
    event.preventDefault();
    emit("moveDown");
  }
}
</script>

<template>
  <article
    :data-testid="`link-row-${props.index}`"
    :class="[
      'link-row-item',
      props.invalid && 'link-row-item--invalid',
      props.dragging && 'link-row-item--dragging',
      props.dropTarget && 'link-row-item--drop-target',
    ]"
    @dragover.prevent
    @dragenter.prevent="emit('dragEnter')"
    @dragleave="emit('dragLeave')"
    @drop.prevent="emit('dropAt')"
  >
    <button
      class="drag-handle drag-handle--button link-row-item__sort"
      :data-testid="`link-row-drag-${props.index}`"
      type="button"
      :disabled="props.disabled"
      draggable="true"
      tabindex="0"
      aria-label="拖拽或使用方向键调整链接顺序"
      @dragstart="emit('dragStart')"
      @dragend="emit('dragEnd')"
      @keydown="onKeydown"
    >
      ⋮⋮
    </button>

    <input
      :data-testid="`link-row-input-${props.index}`"
      :class="['link-row-item__input', props.invalid && 'link-row-item__input--invalid']"
      :disabled="props.disabled"
      :value="props.value"
      placeholder="https://example.com/feed"
      :aria-invalid="props.invalid"
      @input="onInput"
    />

    <div class="link-row-item__actions">
      <button
        class="button button--ghost button--compact"
        :data-testid="`link-row-move-up-${props.index}`"
        type="button"
        :disabled="props.disabled || !props.canMoveUp"
        @click="emit('moveUp')"
      >
        上移
      </button>
      <button
        class="button button--ghost button--compact"
        :data-testid="`link-row-move-down-${props.index}`"
        type="button"
        :disabled="props.disabled || !props.canMoveDown"
        @click="emit('moveDown')"
      >
        下移
      </button>
      <button
        class="button button--ghost button--compact"
        :data-testid="`link-row-remove-${props.index}`"
        type="button"
        :disabled="props.disabled"
        @click="emit('remove')"
      >
        删除
      </button>
    </div>
  </article>
</template>
