import { computed, ref } from "vue";

import type { UserLinksResponse } from "@/api/types";

const selectedUsername = ref<string | null>(null);
const linksText = ref("");
const linksError = ref<string | null>(null);
const savedByUser = ref<Record<string, string>>({});
const draftByUser = ref<Record<string, string>>({});

function savedTextForUser(username: string): string {
  return savedByUser.value[username] ?? "";
}

function displayLinksForUser(username: string): string {
  return draftByUser.value[username] ?? savedTextForUser(username);
}

function updateDraftMap(username: string, nextText: string): void {
  const savedText = savedTextForUser(username);

  if (nextText === savedText) {
    delete draftByUser.value[username];
    return;
  }

  draftByUser.value = {
    ...draftByUser.value,
    [username]: nextText,
  };
}

export function useEditorDrafts() {
  function setSelectedUsername(username: string | null): void {
    selectedUsername.value = username;
    linksError.value = null;
    linksText.value = username === null ? "" : displayLinksForUser(username);
  }

  function applyLoadedLinks(payload: UserLinksResponse | null): void {
    if (payload === null) {
      return;
    }

    const savedText = payload.links.join("\n");
    savedByUser.value = {
      ...savedByUser.value,
      [payload.username]: savedText,
    };

    if (draftByUser.value[payload.username] === savedText) {
      const nextDrafts = { ...draftByUser.value };
      delete nextDrafts[payload.username];
      draftByUser.value = nextDrafts;
    }

    if (selectedUsername.value === payload.username) {
      linksText.value = displayLinksForUser(payload.username);
    }
  }

  function rememberLinksInput(username: string, nextText: string): void {
    updateDraftMap(username, nextText);

    if (selectedUsername.value === username) {
      linksText.value = nextText;
    }
  }

  function updateSelectedLinksText(nextText: string): void {
    if (selectedUsername.value === null) {
      linksText.value = nextText;
      return;
    }

    linksText.value = nextText;
    updateDraftMap(selectedUsername.value, nextText);
  }

  function markLinksSaved(username: string, savedText: string): void {
    savedByUser.value = {
      ...savedByUser.value,
      [username]: savedText,
    };

    const nextDrafts = { ...draftByUser.value };
    delete nextDrafts[username];
    draftByUser.value = nextDrafts;
    linksError.value = null;

    if (selectedUsername.value === username) {
      linksText.value = savedText;
    }
  }

  function clearUserState(username: string): void {
    const nextSaved = { ...savedByUser.value };
    delete nextSaved[username];
    savedByUser.value = nextSaved;

    const nextDrafts = { ...draftByUser.value };
    delete nextDrafts[username];
    draftByUser.value = nextDrafts;

    if (selectedUsername.value === username) {
      selectedUsername.value = null;
      linksText.value = "";
      linksError.value = null;
    }
  }

  function setLinksError(message: string | null): void {
    linksError.value = message;
  }

  function reset(): void {
    selectedUsername.value = null;
    linksText.value = "";
    linksError.value = null;
    savedByUser.value = {};
    draftByUser.value = {};
  }

  return {
    selectedUsername,
    linksText,
    linksError,
    savedByUser,
    draftByUser,
    hasUnsavedChanges: computed(() => {
      if (selectedUsername.value === null) {
        return false;
      }

      return linksText.value !== savedTextForUser(selectedUsername.value);
    }),
    currentSavedText: computed(() =>
      selectedUsername.value === null ? "" : savedTextForUser(selectedUsername.value),
    ),
    setSelectedUsername,
    applyLoadedLinks,
    rememberLinksInput,
    updateSelectedLinksText,
    markLinksSaved,
    clearUserState,
    setLinksError,
    reset,
  };
}
