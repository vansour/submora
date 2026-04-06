import {
  clearCsrfTokenCache,
  requestJson,
  requestJsonWithBody,
  requestWithoutBody,
} from "@/api/client";

function jsonResponse(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: {
      "content-type": "application/json",
    },
  });
}

describe("api client", () => {
  beforeEach(() => {
    clearCsrfTokenCache();
  });

  it("sends GET requests with credentials and without csrf token", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(jsonResponse({ ok: true }));
    vi.stubGlobal("fetch", fetchMock);

    const payload = await requestWithoutBody<{ ok: boolean }>("GET", "/api/users");

    expect(payload).toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/users",
      expect.objectContaining({
        credentials: "include",
        method: "GET",
      }),
    );
  });

  it("fetches csrf token before state-changing requests", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ token: "csrf-1" }))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    vi.stubGlobal("fetch", fetchMock);

    const payload = await requestJsonWithBody<{ ok: boolean }>("POST", "/api/users", {
      username: "alpha",
    });

    expect(payload).toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/auth/csrf",
      expect.objectContaining({ credentials: "include" }),
    );

    const secondCall = fetchMock.mock.calls[1];
    expect(secondCall?.[0]).toBe("/api/users");
    expect(secondCall?.[1]).toEqual(
      expect.objectContaining({
        credentials: "include",
        method: "POST",
      }),
    );

    const headers = new Headers(secondCall?.[1]?.headers);
    expect(headers.get("content-type")).toBe("application/json");
    expect(headers.get("x-csrf-token")).toBe("csrf-1");
  });

  it("clears cached csrf token and retries once after a 403", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ token: "stale-token" }))
      .mockResolvedValueOnce(jsonResponse({ message: "invalid csrf token" }, 403))
      .mockResolvedValueOnce(jsonResponse({ token: "fresh-token" }))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    vi.stubGlobal("fetch", fetchMock);

    const payload = await requestJson<{ ok: boolean }>("/api/users/order", {
      method: "PUT",
      body: JSON.stringify({ order: ["beta", "alpha"] }),
      headers: {
        "content-type": "application/json",
      },
    });

    expect(payload).toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(4);

    const retryCall = fetchMock.mock.calls[3];
    const headers = new Headers(retryCall?.[1]?.headers);
    expect(headers.get("x-csrf-token")).toBe("fresh-token");
  });
});
