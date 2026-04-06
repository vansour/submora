import { useFeedback } from "@/composables";

describe("feedback", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useFeedback().clear();
  });

  afterEach(() => {
    useFeedback().clear();
    vi.useRealTimers();
  });

  it("auto clears status toasts", () => {
    const feedback = useFeedback();

    feedback.setStatus("保存成功");
    expect(feedback.statusMessage.value).toBe("保存成功");

    vi.advanceTimersByTime(2499);
    expect(feedback.statusMessage.value).toBe("保存成功");

    vi.advanceTimersByTime(1);
    expect(feedback.statusMessage.value).toBeNull();
  });

  it("auto clears error toasts", () => {
    const feedback = useFeedback();

    feedback.setError("保存失败");
    expect(feedback.errorMessage.value).toBe("保存失败");

    vi.advanceTimersByTime(3999);
    expect(feedback.errorMessage.value).toBe("保存失败");

    vi.advanceTimersByTime(1);
    expect(feedback.errorMessage.value).toBeNull();
  });
});
