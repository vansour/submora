import { computed } from "vue";

import * as authApi from "@/api/auth";
import type { CurrentUserResponse } from "@/api/types";
import { createAsyncDataState, errorMessage } from "@/composables/state";
import { useEditorDrafts } from "@/composables/useEditorDrafts";
import { useFeedback } from "@/composables/useFeedback";
import { usePendingMap } from "@/composables/usePendingMap";
import { useUserRuntime } from "@/composables/useUserRuntime";
import { useUsers } from "@/composables/useUsers";

const sessionState = createAsyncDataState<CurrentUserResponse | null>(null);

export function useSession() {
  const feedback = useFeedback();
  const pending = usePendingMap();
  const drafts = useEditorDrafts();
  const users = useUsers();
  const runtime = useUserRuntime();

  function resetAuthDependentState(): void {
    sessionState.setData(null);
    users.reset();
    drafts.reset();
    runtime.reset();
  }

  async function restoreSession(): Promise<CurrentUserResponse | null> {
    sessionState.setLoading();

    try {
      const currentUser = await authApi.getCurrentUser();
      return sessionState.setData(currentUser);
    } catch (error) {
      throw sessionState.setError(errorMessage(error));
    }
  }

  async function login(username: string, password: string): Promise<void> {
    feedback.clear();

    return pending.runWithPending("login", async () => {
      try {
        await authApi.login({ username, password });
        await restoreSession();
        feedback.setStatus("登录成功");
      } catch (error) {
        const message = errorMessage(error);
        sessionState.setError(message);
        feedback.setError(message);
        throw message;
      }
    });
  }

  async function logout(): Promise<void> {
    feedback.clear();

    return pending.runWithPending("logout", async () => {
      try {
        await authApi.logout();
        resetAuthDependentState();
        feedback.setStatus("已退出登录");
      } catch (error) {
        const message = errorMessage(error);
        feedback.setError(message);
        throw message;
      }
    });
  }

  return {
    currentUser: sessionState.data,
    sessionStatus: sessionState.status,
    sessionError: sessionState.error,
    sessionLoading: sessionState.loading,
    isAuthenticated: computed(() => sessionState.data.value !== null),
    restoreSession,
    login,
    logout,
    resetAuthDependentState,
  };
}
