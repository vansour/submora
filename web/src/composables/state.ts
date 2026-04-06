import { computed, ref, type ComputedRef, type Ref } from "vue";

export type AsyncStatus = "idle" | "loading" | "ready" | "error";

export interface AsyncDataState<T> {
  status: Ref<AsyncStatus>;
  data: Ref<T>;
  error: Ref<string | null>;
  loading: ComputedRef<boolean>;
  ready: ComputedRef<boolean>;
  hasError: ComputedRef<boolean>;
  setLoading: () => void;
  setData: (value: T) => T;
  setError: (message: string) => string;
  reset: () => void;
}

export function createAsyncDataState<T>(initialValue: T): AsyncDataState<T> {
  const status = ref<AsyncStatus>("idle");
  const data = ref<T>(initialValue) as Ref<T>;
  const error = ref<string | null>(null);

  function setLoading(): void {
    status.value = "loading";
    error.value = null;
  }

  function setData(value: T): T {
    data.value = value;
    status.value = "ready";
    error.value = null;
    return value;
  }

  function setError(message: string): string {
    status.value = "error";
    error.value = message;
    return message;
  }

  function reset(): void {
    status.value = "idle";
    data.value = initialValue;
    error.value = null;
  }

  return {
    status,
    data,
    error,
    loading: computed(() => status.value === "loading"),
    ready: computed(() => status.value === "ready"),
    hasError: computed(() => status.value === "error"),
    setLoading,
    setData,
    setError,
    reset,
  };
}

export function errorMessage(error: unknown): string {
  if (typeof error === "string" && error.trim() !== "") {
    return error;
  }

  if (error instanceof Error && error.message.trim() !== "") {
    return error.message;
  }

  return "Unexpected error";
}
