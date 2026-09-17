import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import ReactMarkdown from "react-markdown";
import { describe, expect, it } from "vitest";
import { markdownSanitize, markdownSchema } from "./markdownSanitize";

describe("markdownSchema", () => {
  it("does not allow img, picture, or source tags", () => {
    const tags = markdownSchema.tagNames ?? [];
    expect(tags).not.toContain("img");
    expect(tags).not.toContain("picture");
    expect(tags).not.toContain("source");
    expect(tags).toContain("p");
    expect(tags).toContain("a");
  });

  it("does not permit http(s) image protocols", () => {
    expect(markdownSchema.protocols?.src ?? []).not.toContain("http");
    expect(markdownSchema.protocols?.src ?? []).not.toContain("https");
    expect(markdownSchema.protocols?.href ?? []).toContain("https");
  });

  it("strips remote images from rendered markdown", () => {
    const html = renderToStaticMarkup(
      createElement(
        ReactMarkdown,
        { rehypePlugins: [markdownSanitize] },
        "hello ![x](https://evil.example/x.png) world"
      )
    );
    expect(html).not.toContain("evil.example");
    expect(html).not.toContain("<img");
    expect(html).toContain("hello");
    expect(html).toContain("world");
  });

  it("still renders links", () => {
    const html = renderToStaticMarkup(
      createElement(
        ReactMarkdown,
        { rehypePlugins: [markdownSanitize] },
        "see [docs](https://example.com/a)"
      )
    );
    expect(html).toContain("href=\"https://example.com/a\"");
  });
});


describe("markdownSchema", () => {
  it("does not allow img, picture, or source tags", () => {
    const tags = markdownSchema.tagNames ?? [];
    expect(tags).not.toContain("img");
    expect(tags).not.toContain("picture");
    expect(tags).not.toContain("source");
    expect(tags).toContain("p");
    expect(tags).toContain("a");
  });

  it("does not permit http(s) image protocols", () => {
    expect(markdownSchema.protocols?.src ?? []).not.toContain("http");
    expect(markdownSchema.protocols?.src ?? []).not.toContain("https");
    expect(markdownSchema.protocols?.href ?? []).toContain("https");
  });
});
