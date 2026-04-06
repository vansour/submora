import { computed } from "vue";

import * as usersApi from "@/api/users";
import type { UserLinksResponse } from "@/api/types";
import { createAsyncDataState, errorMessage } from "@/composables/state";
import { useEditorDrafts } from "@/composables/useEditorDrafts";
import { useFeedback } from "@/composables/useFeedback";
import { usePendingMap } from "@/composables/usePendingMap";
import { analyzeLinks, parseLinks } from "@/utils/links";
import { extractFieldValidationError } from "@/utils/messages";

const linksState = createAsyncDataState<UserLinksResponse | null>(null);
let activeLoadToken = 0;

function savedLinksMessage(
  username: string,
  savedCount: number,
  draftStats: ReturnType<typeof analyzeLinks>,
): string {
  if (savedCount === 0) {
    return `已清空 ${username} 的源链接`;
  }

  const normalizedChanges: string[] = [];
  if (draftStats.duplicate_count > 0) {
    normalizedChanges.push(`合并 ${draftStats.duplicate_count} 条重复`);
  }
  if (draftStats.blank_count > 0) {
    normalizedChanges.push(`忽略 ${draftStats.blank_count} 行空白`);
  }

  if (normalizedChanges.length === 0) {
    return `已保存 ${username} 的源链接，共 ${savedCount} 条`;
  }

  return `已保存 ${username} 的源链接，保留 ${savedCount} 条，${normalizedChanges.join("，")}`;
}

export function useUserRuntime() {
  const drafts = useEditorDrafts();
  const feedback = useFeedback();
  const pending = usePendingMap();

  async function loadLinks(username = drafts.selectedUsername.value): Promise<UserLinksResponse | null> {
    const loadToken = ++activeLoadToken;

    if (username === null) {
      return linksState.setData(null);
    }

    linksState.setLoading();

    try {
      const payload = await usersApi.getLinks(username);
      if (loadToken !== activeLoadToken || drafts.selectedUsername.value !== payload.username) {
        return payload;
      }

      drafts.applyLoadedLinks(payload);
      return linksState.setData(payload);
    } catch (error) {
      if (loadToken !== activeLoadToken || drafts.selectedUsername.value !== username) {
        throw errorMessage(error);
      }

      throw linksState.setError(errorMessage(error));
    }
  }

  async function loadSelectedData(): Promise<void> {
    const username = drafts.selectedUsername.value;
    if (username === null) {
      ++activeLoadToken;
      linksState.setData(null);
      return;
    }

    await loadLinks(username);
  }

  async function saveSelectedLinks(): Promise<UserLinksResponse | null> {
    const username = drafts.selectedUsername.value;
    if (username === null) {
      return null;
    }

    feedback.clear();
    drafts.setLinksError(null);

    const draftStats = analyzeLinks(drafts.linksText.value, 0);

    return pending.runWithPending("saveLinks", async () => {
      try {
        const response = await usersApi.setLinks(username, {
          links: parseLinks(drafts.linksText.value),
        });
        const normalizedLinks = response.links.join("\n");
        drafts.markLinksSaved(response.username, normalizedLinks);
        linksState.setData(response);
        feedback.setStatus(savedLinksMessage(response.username, response.links.length, draftStats));
        return response;
      } catch (error) {
        const message = errorMessage(error);
        const fieldError = extractFieldValidationError(message, "links", "链接");
        if (fieldError !== null) {
          drafts.setLinksError(fieldError);
        } else {
          feedback.setError(message);
        }
        throw message;
      }
    });
  }

  function reset(): void {
    ++activeLoadToken;
    linksState.reset();
  }

  return {
    links: linksState.data,
    linksStatus: linksState.status,
    linksError: linksState.error,
    linksLoading: linksState.loading,
    savedLinkCount: computed(() => linksState.data.value?.links.length ?? 0),
    loadLinks,
    loadSelectedData,
    saveSelectedLinks,
    reset,
  };
}
