import { computed } from "vue";

import * as usersApi from "@/api/users";
import type { UserSummary } from "@/api/types";
import { createAsyncDataState, errorMessage } from "@/composables/state";
import { useEditorDrafts } from "@/composables/useEditorDrafts";
import { useFeedback } from "@/composables/useFeedback";
import { usePendingMap } from "@/composables/usePendingMap";
import { useUserRuntime } from "@/composables/useUserRuntime";

const usersState = createAsyncDataState<UserSummary[]>([]);

export function reorderedUsernames(
  users: UserSummary[],
  username: string,
  offset: number,
): string[] | null {
  const position = users.findIndex((item) => item.username === username);
  if (position === -1) {
    return null;
  }

  const target = position + offset;
  if (target < 0 || target >= users.length) {
    return null;
  }

  const order = users.map((item) => item.username);
  [order[position], order[target]] = [order[target], order[position]];
  return order;
}

export function moveUsernameBefore(
  users: UserSummary[],
  username: string,
  beforeUsername: string,
): string[] | null {
  const position = users.findIndex((item) => item.username === username);
  const target = users.findIndex((item) => item.username === beforeUsername);
  if (position === -1 || target === -1 || position === target) {
    return null;
  }

  if (target === users.length - 1) {
    return moveUsernameToEdge(users, username, false);
  }

  const order = users.map((item) => item.username);
  const [moved] = order.splice(position, 1);
  const adjustedTarget = position < target ? target - 1 : target;
  order.splice(adjustedTarget, 0, moved);
  return order;
}

export function moveUsernameToEdge(
  users: UserSummary[],
  username: string,
  toStart: boolean,
): string[] | null {
  const position = users.findIndex((item) => item.username === username);
  if (position === -1) {
    return null;
  }

  const target = toStart ? 0 : users.length - 1;
  if (position === target) {
    return null;
  }

  const order = users.map((item) => item.username);
  const [moved] = order.splice(position, 1);
  order.splice(target, 0, moved);
  return order;
}

export function useUsers() {
  const feedback = useFeedback();
  const pending = usePendingMap();
  const drafts = useEditorDrafts();
  const runtime = useUserRuntime();

  async function loadUsers(): Promise<UserSummary[]> {
    usersState.setLoading();

    try {
      const users = await usersApi.listUsers();
      usersState.setData(users);

      if (
        drafts.selectedUsername.value !== null &&
        !users.some((user) => user.username === drafts.selectedUsername.value)
      ) {
        drafts.setSelectedUsername(null);
      }

      return users;
    } catch (error) {
      throw usersState.setError(errorMessage(error));
    }
  }

  async function createUser(username: string, selectCreated = true): Promise<UserSummary> {
    feedback.clear();

    return pending.runWithPending("createUser", async () => {
      try {
        const user = await usersApi.createUser({ username });
        usersState.setData([...usersState.data.value, user]);
        feedback.setStatus(`已创建订阅组 ${user.username}`);

        if (selectCreated) {
          drafts.setSelectedUsername(user.username);
        }

        return user;
      } catch (error) {
        const message = errorMessage(error);
        feedback.setError(message);
        throw message;
      }
    });
  }

  async function deleteUser(username: string): Promise<void> {
    feedback.clear();

    return pending.runWithPending("deleteUser", async () => {
      try {
        await usersApi.deleteUser(username);
        usersState.setData(usersState.data.value.filter((user) => user.username !== username));
        drafts.clearUserState(username);
        runtime.reset();
        feedback.setStatus("已删除");
      } catch (error) {
        const message = errorMessage(error);
        feedback.setError(message);
        throw message;
      }
    });
  }

  async function updateOrder(order: string[]): Promise<string[]> {
    feedback.clear();

    return pending.runWithPending("reorderUsers", async () => {
      try {
        const confirmedOrder = await usersApi.setOrder({ order });
        const userMap = new Map(usersState.data.value.map((user) => [user.username, user]));
        usersState.setData(
          confirmedOrder
            .map((username) => userMap.get(username))
            .filter((user): user is UserSummary => user !== undefined),
        );
        feedback.setStatus("已更新订阅组顺序");
        return confirmedOrder;
      } catch (error) {
        const message = errorMessage(error);
        feedback.setError(message);
        throw message;
      }
    });
  }

  function reset(): void {
    usersState.reset();
  }

  return {
    users: usersState.data,
    usersStatus: usersState.status,
    usersError: usersState.error,
    usersLoading: usersState.loading,
    isEmpty: computed(() => usersState.data.value.length === 0),
    loadUsers,
    createUser,
    deleteUser,
    updateOrder,
    reset,
  };
}
