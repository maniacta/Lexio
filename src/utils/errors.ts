/** Map backend / network errors into short Chinese guidance for the chat UI. */
export function formatApiError(err: unknown): string {
  const raw = err instanceof Error ? err.message : String(err);

  if (/MISSING_API_KEY/i.test(raw) || /填写 API Key/i.test(raw)) {
    return "还没有配置 API Key。请点击左下角 ⚙ →「模型厂商」，填写密钥后再试。";
  }
  if (/AUTH_ERROR/i.test(raw) || /invalid api key/i.test(raw) || /unauthorized/i.test(raw)) {
    return "API Key 无效或权限不足，请到「设置 → 模型厂商」核对密钥。";
  }
  if (/QUOTA_ERROR|rate limit|quota/i.test(raw)) {
    return "请求过于频繁或额度不足，请稍后再试。";
  }
  if (/NETWORK_ERROR|Failed to fetch|NetworkError|ECONNREFUSED/i.test(raw)) {
    return "无法连接模型服务，请检查网络或 Base URL 是否正确。";
  }
  if (/PROVIDER_ERROR|HTTP 5\d\d/i.test(raw)) {
    return "模型服务暂时不可用，请稍后再试。";
  }
  if (/No model configured|No default provider|No default model/i.test(raw)) {
    return "尚未配置可用模型，请到「设置 → 模型厂商」完成配置。";
  }

  // Strip noisy "API error 500: " prefix when body already explains
  const cleaned = raw.replace(/^API error \d+:\s*/i, "").trim();
  return cleaned ? `出错了：${cleaned}` : "出错了，请稍后重试。";
}

/** True when the default (or any) provider has no API key configured. */
export function needsApiKeySetup(providers: { is_default: boolean; api_key: string }[]): boolean {
  const def = providers.find((p) => p.is_default) ?? providers[0];
  return !def || !def.api_key.trim();
}
