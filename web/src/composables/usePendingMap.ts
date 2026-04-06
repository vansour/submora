import { computed, reactive, readonly } from "vue";

export type PendingKey =
  | "login"
  | "logout"
  | "createUser"
  | "reorderUsers"
  | "saveLinks"
  | "deleteUser";

const pending = reactive<Record<PendingKey, boolean>>({
  login: false,
  logout: false,
  createUser: false,
  reorderUsers: false,
  saveLinks: false,
  deleteUser: false,
});

export function usePendingMap() {
  function isPending(key: PendingKey): boolean {
    return pending[key];
  }

  function setPending(key: PendingKey, value: boolean): void {
    pending[key] = value;
  }

  async function runWithPending<T>(key: PendingKey, task: () => Promise<T>): Promise<T> {
    if (pending[key]) {
      throw `Pending operation already running: ${key}`;
    }

    pending[key] = true;
    try {
      return await task();
    } finally {
      pending[key] = false;
    }
  }

  function clearAll(): void {
    for (const key of Object.keys(pending) as PendingKey[]) {
      pending[key] = false;
    }
  }

  return {
    pending: readonly(pending),
    anyPending: computed(() => Object.values(pending).some(Boolean)),
    isPending,
    setPending,
    runWithPending,
    clearAll,
  };
}
