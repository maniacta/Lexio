export type ThemeMode = "system" | "light" | "dark";

export function applyTheme(theme: string) {
  const root = document.documentElement;
  root.classList.remove("theme-light", "theme-dark");
  if (theme === "light") root.classList.add("theme-light");
  else if (theme === "dark") root.classList.add("theme-dark");
  // "system" → no class; CSS media query applies
}

export function applyLanguage(language: string) {
  document.documentElement.lang = language === "en" ? "en" : "zh-CN";
}
