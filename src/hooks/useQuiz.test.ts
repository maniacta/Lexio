import { describe, expect, it } from "vitest";
import { quizStep } from "./useQuiz";

describe("quizStep", () => {
  it("advances while questions remain", () => {
    expect(quizStep(0, 3)).toBe(1);
    expect(quizStep(1, 3)).toBe(2);
  });

  it("completes on the last question instead of no-opping", () => {
    expect(quizStep(2, 3)).toBe("complete");
    expect(quizStep(0, 1)).toBe("complete");
  });

  it("completes when the quiz is empty", () => {
    expect(quizStep(0, 0)).toBe("complete");
  });
});
