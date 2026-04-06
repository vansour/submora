import { computed, ref } from "vue";

const statusMessage = ref<string | null>(null);
const errorMessage = ref<string | null>(null);

export function useFeedback() {
  function clearStatus(): void {
    statusMessage.value = null;
  }

  function clearError(): void {
    errorMessage.value = null;
  }

  function clear(): void {
    clearStatus();
    clearError();
  }

  function setStatus(message: string): void {
    statusMessage.value = message;
  }

  function setError(message: string): void {
    errorMessage.value = message;
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
