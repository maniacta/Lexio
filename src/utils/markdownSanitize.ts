import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import type { Options } from "rehype-sanitize";

/**
 * Markdown from the model (and stored knowledge-point bodies) must not fetch
 * the network. The default rehype-sanitize schema allows `img` with http(s)
 * `src`, which would leak the user's IP and let a prompt injection beacon
 * out. Desktop CSP already blocks that; this schema is the web-mode control
 * and a second line on desktop.
 */
const MEDIA_TAGS = new Set(["img", "picture", "source"]);

export const markdownSchema: Options = {
  ...defaultSchema,
  tagNames: (defaultSchema.tagNames ?? []).filter((tag) => !MEDIA_TAGS.has(tag)),
  attributes: Object.fromEntries(
    Object.entries(defaultSchema.attributes ?? {}).filter(([tag]) => !MEDIA_TAGS.has(tag))
  ),
  protocols: {
    ...defaultSchema.protocols,
    src: [],
    longDesc: [],
  },
};

/** Plugin tuple for `ReactMarkdown`: sanitize with {@link markdownSchema}. */
export const markdownSanitize: [typeof rehypeSanitize, Options] = [
  rehypeSanitize,
  markdownSchema,
];
