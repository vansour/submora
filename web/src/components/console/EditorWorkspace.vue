<script setup lang="ts">
import { computed, ref } from "vue";

import {
  useEditorDrafts,
  useFeedback,
  usePendingMap,
  useUserRuntime,
} from "@/composables";
import { analyzeLinks } from "@/utils/links";
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
  linksStatus,
  linksError: loadedLinksError,
  savedLinkCount,
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

const draftStats = computed(() => analyzeLinks(drafts.linksText.value, 4));
const editorBusy = computed(() => pending.pending.saveLinks || pending.pending.deleteUser);
const localFormatIssue = computed(() => {
  if (draftStats.value.invalid_count === 0 || draftStats.value.first_invalid === null) {
    return null;
  }

  return `发现 ${draftStats.value.invalid_count} 条格式不正确的链接，保存前需要修正。首条问题：${draftStats.value.first_invalid}`;
});
const normalizationHint = computed(() => {
  const changes: string[] = [];

  if (draftStats.value.duplicate_count > 0) {
    changes.push(`合并 ${draftStats.value.duplicate_count} 条重复`);
  }
  if (draftStats.value.blank_count > 0) {
    changes.push(`忽略 ${draftStats.value.blank_count} 行空白`);
  }

  if (changes.length === 0) {
    return null;
  }

  return `保存后会自动归一化：${changes.join("，")}。`;
});
const saveDisabled = computed(
  () =>
    props.username === null ||
    editorBusy.value ||
    !hasUnsavedChanges.value ||
    draftStats.value.invalid_count > 0,
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
    <div class="editor-toolbar">
      <div class="editor-toolbar__title">
        <span class="editor-toolbar__eyebrow">editor</span>
        <h2 class="section-title">{{ props.username ? props.username : "选择订阅组" }}</h2>
      </div>
      <div class="button-row">
        <span v-if="hasUnsavedChanges" class="chip chip--soft">未保存</span>
        <span v-if="props.username" class="chip chip--soft">主工作区</span>
        <button
          class="button button--ghost button--icon button--toolbar"
          data-testid="editor-add"
          type="button"
          :disabled="props.username === null || editorBusy"
          aria-label="新增链接"
          @click="addRow"
        >
          +
        </button>
        <button
          class="button button--ghost button--icon button--toolbar"
          data-testid="editor-copy-public"
          type="button"
          :disabled="props.username === null || editorBusy || copyPending"
          aria-label="复制公共入口"
          @click="copyPublicEntry"
        >
          {{ copyPending ? "…" : "⎘" }}
        </button>
        <button
          class="button button--primary button--icon button--toolbar"
          data-testid="editor-save"
          type="button"
          :disabled="saveDisabled"
          aria-label="保存链接"
          @click="saveLinks"
        >
          {{ pending.pending.saveLinks ? "…" : "✓" }}
        </button>
      </div>
    </div>

    <template v-if="props.username">
      <div class="editor-strip">
        <span class="editor-strip__item">LINKS</span>
        <span class="editor-strip__divider"></span>
        <span class="editor-strip__item">{{ hasUnsavedChanges ? "DRAFT" : "SYNCED" }}</span>
        <span class="editor-strip__divider"></span>
        <span class="editor-strip__item">WEB</span>
        <span class="editor-strip__divider"></span>
        <span class="editor-strip__item">SAVED {{ savedLinkCount }}</span>
        <span class="editor-strip__divider"></span>
        <span class="editor-strip__item">PREVIEW {{ draftStats.normalized_count }}</span>
      </div>

      <section class="editor-public-route">
        <div>
          <p class="eyebrow">public route</p>
          <p class="section-copy">公开聚合入口</p>
        </div>
        <input class="editor-public-route__input" data-testid="editor-public-route" :value="publicRouteUrl" readonly />
      </section>

      <div class="editor-status-stack">
        <p v-if="linksStatus === 'loading'" class="section-copy">正在加载已保存链接…</p>
        <p v-else-if="linksStatus === 'error' && loadedLinksError" class="inline-error">
          已保存链接加载失败：{{ loadedLinksError }}
        </p>

        <p v-if="linksError" class="inline-error">{{ linksError }}</p>
        <p v-else-if="localFormatIssue" class="inline-error">{{ localFormatIssue }}</p>
        <p v-else-if="hasUnsavedChanges" class="section-copy">
          当前内容是本地草稿，尚未保存到服务器。
        </p>
        <p v-else class="section-copy">当前显示的是服务器上最新的已保存链接。</p>
      </div>

      <div class="editor-stats">
        <span v-if="draftStats.blank_count > 0" class="chip chip--soft">空白 {{ draftStats.blank_count }}</span>
        <span v-if="draftStats.duplicate_count > 0" class="chip chip--soft">重复 {{ draftStats.duplicate_count }}</span>
        <span v-if="draftStats.invalid_count > 0" class="chip chip--soft">非法 {{ draftStats.invalid_count }}</span>
      </div>

      <LinkRowsEditor :rows="linkRows" :disabled="editorBusy" @change-rows="commitRows" />

      <section
        v-if="draftStats.normalized_preview.length > 0"
        class="editor-preview"
      >
        <div class="editor-preview__head">
          <p class="eyebrow">normalized preview</p>
          <p class="section-copy">保存后将按这个顺序保留前几条有效链接。</p>
        </div>
        <ul class="editor-preview__list">
          <li v-for="link in draftStats.normalized_preview" :key="link" class="editor-preview__item">
            {{ link }}
          </li>
        </ul>
      </section>

      <p v-if="normalizationHint" class="section-copy">{{ normalizationHint }}</p>
    </template>

    <section v-else class="editor-empty-state">
      <p class="eyebrow">editor</p>
      <h3 class="section-title">选择订阅组</h3>
      <p class="section-copy">从左侧菜单打开订阅组后，即可直接编辑链接。</p>
    </section>
  </section>
</template>
