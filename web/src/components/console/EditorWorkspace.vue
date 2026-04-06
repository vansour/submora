<script setup lang="ts">
import { computed, ref } from "vue";

import {
  useEditorDrafts,
  useFeedback,
  usePendingMap,
  useUserRuntime,
} from "@/composables";
import {
  buildPublicRouteUrl,
  copyPublicRoute,
} from "@/utils/clipboard";

import LinkRowsEditor from "@/components/console/LinkRowsEditor.vue";

const props = defineProps<{
  username: string | null;
}>();

const drafts = useEditorDrafts();
const {
  hasUnsavedChanges,
  linksError,
} = drafts;
const runtime = useUserRuntime();
const {
  linksError: loadedLinksError,
} = runtime;
const pending = usePendingMap();
const feedback = useFeedback();
const copyPending = ref(false);

const linkRows = computed(() => {
  if (drafts.linksText.value === "") {
    return [""];
  }

  return drafts.linksText.value.split("\n");
});

const editorBusy = computed(() => pending.pending.saveLinks || pending.pending.deleteUser);
const saveDisabled = computed(
  () =>
    props.username === null ||
    editorBusy.value ||
    !hasUnsavedChanges.value,
);
const publicRouteUrl = computed(() =>
  props.username === null ? "" : buildPublicRouteUrl(props.username),
);

function commitRows(rows: string[]): void {
  drafts.setLinksError(null);
  drafts.updateSelectedLinksText(rows.join("\n"));
}

function addRow(): void {
  if (props.username === null) {
    return;
  }

  commitRows([...linkRows.value, ""]);
}

async function saveLinks(): Promise<void> {
  try {
    await runtime.saveSelectedLinks();
  } catch {
    // Feedback is already surfaced through the shared toast viewport.
  }
}

async function copyPublicEntry(): Promise<void> {
  if (props.username === null || copyPending.value) {
    return;
  }

  copyPending.value = true;
  feedback.clear();

  try {
    const url = await copyPublicRoute(props.username);
    feedback.setStatus(`已复制公共入口链接：${url}`);
  } catch (error) {
    feedback.setError(typeof error === "string" ? error : "复制公共入口链接失败");
  } finally {
    copyPending.value = false;
  }
}
</script>

<template>
  <section class="editor-pane">
    <template v-if="props.username">
      <header class="editor-form-head">
        <div class="editor-toolbar__title">
          <h2 class="section-title">{{ props.username }}</h2>
        </div>
        <p v-if="hasUnsavedChanges" class="editor-form__status">有未保存更改</p>
      </header>

      <section class="editor-public-route editor-public-route--inline">
        <label class="field editor-public-route__field">
          <span>公共入口</span>
          <input
            class="editor-public-route__input"
            data-testid="editor-public-route"
            :value="publicRouteUrl"
            readonly
          />
        </label>
        <button
          class="button button--ghost"
          data-testid="editor-copy-public"
          type="button"
          :disabled="editorBusy || copyPending"
          aria-label="复制公共入口"
          @click="copyPublicEntry"
        >
          {{ copyPending ? "复制中…" : "复制入口" }}
        </button>
      </section>

      <section class="editor-form-section">
        <div class="editor-form-section__head">
          <div class="editor-form-section__title">
            <h3 class="section-title">源链接</h3>
            <p class="section-copy">每行一条。</p>
          </div>
          <button
            class="button button--ghost"
            data-testid="editor-add"
            type="button"
            :disabled="editorBusy"
            aria-label="新增链接"
            @click="addRow"
          >
            新增一行
          </button>
        </div>

        <div v-if="loadedLinksError || linksError" class="editor-status-stack">
          <p v-if="loadedLinksError" class="inline-error">
            加载失败：{{ loadedLinksError }}
          </p>
          <p v-if="linksError" class="inline-error">{{ linksError }}</p>
        </div>

        <LinkRowsEditor :rows="linkRows" :disabled="editorBusy" @change-rows="commitRows" />
      </section>

      <div class="editor-form-actions">
        <button
          class="button button--primary"
          data-testid="editor-save"
          type="button"
          :disabled="saveDisabled"
          aria-label="保存链接"
          @click="saveLinks"
        >
          {{ pending.pending.saveLinks ? "保存中…" : "保存" }}
        </button>
      </div>
    </template>

    <section v-else class="editor-empty-state">
      <h3 class="section-title">选择订阅组</h3>
      <p class="section-copy">从左侧菜单打开订阅组后，即可直接编辑链接。</p>
    </section>
  </section>
</template>
