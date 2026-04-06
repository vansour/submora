import { computed } from "vue";

import * as usersApi from "@/api/users";
import type {
  UserCacheStatusResponse,
  UserDiagnosticsResponse,
  UserLinksResponse,
} from "@/api/types";
import { createAsyncDataState, errorMessage } from "@/composables/state";
import { useEditorDrafts } from "@/composables/useEditorDrafts";
import { useFeedback } from "@/composables/useFeedback";
import { usePendingMap } from "@/composables/usePendingMap";
import { analyzeLinks, parseLinks } from "@/utils/links";
import { extractFieldValidationError } from "@/utils/messages";

const linksState = createAsyncDataState<UserLinksResponse | null>(null);
const diagnosticsState = createAsyncDataState<UserDiagnosticsResponse | null>(null);
const cacheState = createAsyncDataState<UserCacheStatusResponse | null>(null);

function cacheRefreshMessage(username: string, state: string): string {
  return state === "empty"
    ? `${username} 当前没有可缓存的已保存链接`
    : `已刷新 ${username} 的缓存`;
}

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
    if (username === null) {
      return linksState.setData(null);
    }

    linksState.setLoading();

    try {
      const payload = await usersApi.getLinks(username);
      drafts.applyLoadedLinks(payload);
      return linksState.setData(payload);
    } catch (error) {
      throw linksState.setError(errorMessage(error));
    }
  }

  async function loadDiagnostics(
    username = drafts.selectedUsername.value,
  ): Promise<UserDiagnosticsResponse | null> {
    if (username === null) {
      return diagnosticsState.setData(null);
    }

    diagnosticsState.setLoading();

    try {
      const payload = await usersApi.getDiagnostics(username);
      return diagnosticsState.setData(payload);
    } catch (error) {
      throw diagnosticsState.setError(errorMessage(error));
    }
  }

  async function loadCacheStatus(
    username = drafts.selectedUsername.value,
  ): Promise<UserCacheStatusResponse | null> {
    if (username === null) {
      return cacheState.setData(null);
    }

    cacheState.setLoading();

    try {
      const payload = await usersApi.getCacheStatus(username);
      return cacheState.setData(payload);
    } catch (error) {
      throw cacheState.setError(errorMessage(error));
    }
  }

  async function loadSelectedData(): Promise<void> {
    const username = drafts.selectedUsername.value;
    if (username === null) {
      linksState.setData(null);
      diagnosticsState.setData(null);
      cacheState.setData(null);
      return;
    }

    await Promise.all([
      loadLinks(username),
      loadDiagnostics(username),
      loadCacheStatus(username),
    ]);
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
        await Promise.all([
          loadDiagnostics(response.username),
          loadCacheStatus(response.username),
        ]);
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

  async function refreshCache(): Promise<UserCacheStatusResponse | null> {
    const username = drafts.selectedUsername.value;
    if (username === null) {
      return null;
    }

    feedback.clear();

    return pending.runWithPending("refreshCache", async () => {
      try {
        const status = await usersApi.refreshCache(username);
        cacheState.setData(status);
        await loadDiagnostics(username);
        feedback.setStatus(cacheRefreshMessage(username, status.state));
        return status;
      } catch (error) {
        const message = errorMessage(error);
        feedback.setError(message);
        throw message;
      }
    });
  }

  async function clearCache(): Promise<void> {
    const username = drafts.selectedUsername.value;
    if (username === null) {
      return;
    }

    feedback.clear();

    return pending.runWithPending("clearCache", async () => {
      try {
        await usersApi.clearCache(username);
        cacheState.setData({
          username,
          state: "empty",
          line_count: 0,
          body_bytes: 0,
          generated_at: null,
          expires_at: null,
        });
        feedback.setStatus(`已清空 ${username} 的缓存`);
      } catch (error) {
        const message = errorMessage(error);
        feedback.setError(message);
        throw message;
      }
    });
  }

  function reset(): void {
    linksState.reset();
    diagnosticsState.reset();
    cacheState.reset();
  }

  return {
    links: linksState.data,
    linksStatus: linksState.status,
    linksError: linksState.error,
    linksLoading: linksState.loading,
    diagnostics: diagnosticsState.data,
    diagnosticsStatus: diagnosticsState.status,
    diagnosticsError: diagnosticsState.error,
    diagnosticsLoading: diagnosticsState.loading,
    cacheStatus: cacheState.data,
    cacheStatusState: cacheState.status,
    cacheError: cacheState.error,
    cacheLoading: cacheState.loading,
    savedLinkCount: computed(() => linksState.data.value?.links.length ?? 0),
    loadLinks,
    loadDiagnostics,
    loadCacheStatus,
    loadSelectedData,
    saveSelectedLinks,
    refreshCache,
    clearCache,
    reset,
  };
}
