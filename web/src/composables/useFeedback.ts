import { computed, ref } from "vue";

const statusMessage = ref<string | null>(null);
const errorMessage = ref<string | null>(null);
const STATUS_TOAST_DURATION_MS = 2500;
const ERROR_TOAST_DURATION_MS = 4000;

let statusTimer: ReturnType<typeof setTimeout> | null = null;
let errorTimer: ReturnType<typeof setTimeout> | null = null;

function clearTimer(timer: ReturnType<typeof setTimeout> | null): void {
  if (timer !== null) {
    clearTimeout(timer);
  }
}

export function useFeedback() {
  function clearStatus(): void {
    clearTimer(statusTimer);
    statusTimer = null;
    statusMessage.value = null;
  }

  function clearError(): void {
    clearTimer(errorTimer);
    errorTimer = null;
    errorMessage.value = null;
  }

  function clear(): void {
    clearStatus();
    clearError();
  }

  function setStatus(message: string): void {
    statusMessage.value = message;
    clearTimer(statusTimer);
    statusTimer = setTimeout(() => {
      statusTimer = null;
      statusMessage.value = null;
    }, STATUS_TOAST_DURATION_MS);
  }

  function setError(message: string): void {
    errorMessage.value = message;
    clearTimer(errorTimer);
    errorTimer = setTimeout(() => {
      errorTimer = null;
      errorMessage.value = null;
    }, ERROR_TOAST_DURATION_MS);
  }

  return {
    statusMessage,
    errorMessage,
    hasStatus: computed(() => statusMessage.value !== null),
    hasError: computed(() => errorMessage.value !== null),
    clearStatus,
    clearError,
    clear,
    setStatus,
    setError,
  };
}
