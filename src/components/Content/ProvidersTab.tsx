import { useEffect, useRef, useState } from "react";
import type { SettingsData, ProviderWithModels, ProviderKindInfo } from "../../types";
import { api } from "../../api/client";

interface Props {
  settings: SettingsData;
  onSaved: () => void;
}

function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export default function ProvidersTab({ settings, onSaved }: Props) {
  const [providers, setProviders] = useState(settings.providers);
  const [kinds, setKinds] = useState<ProviderKindInfo[]>([]);
  const [editId, setEditId] = useState<string | null>(null);
  const [addNew, setAddNew] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<string | null>(null);
  const [status, setStatus] = useState("");
  const [addKind, setAddKind] = useState("deepseek");
  const keyInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    api.settings.listProviderKinds().then(setKinds).catch(() => {});
  }, []);

  useEffect(() => {
    setProviders(settings.providers);
  }, [settings.providers]);

  const editProvider = providers.find((p) => p.id === editId);
  const selectedKind = kinds.find((k) => k.kind === addKind);
  const existingKinds = new Set(providers.map((p) => p.api_format));
  const availableKinds = kinds.filter((k) => !existingKinds.has(k.kind));

  const refresh = async () => {
    onSaved();
    const data = await api.settings.getAll();
    setProviders(data.providers);
  };

  const readApiKey = () => {
    // Prefer DOM value so browser autofill is not lost when React state stays empty.
    const fromDom = keyInputRef.current?.value ?? "";
    return fromDom.trim();
  };

  const startEdit = (p: ProviderWithModels) => {
    setEditId(p.id);
    setAddNew(false);
    setStatus("");
    if (keyInputRef.current) keyInputRef.current.value = "";
  };

  const startAdd = () => {
    setEditId(null);
    setAddNew(true);
    setAddKind(availableKinds[0]?.kind ?? "deepseek");
    setStatus("");
    if (keyInputRef.current) keyInputRef.current.value = "";
  };

  const cancelEdit = (clearStatus = true) => {
    setEditId(null);
    setAddNew(false);
    if (clearStatus) setStatus("");
  };

  const kindLabel = (p: ProviderWithModels) =>
    kinds.find((x) => x.kind === p.api_format)?.display_name ?? p.name;

  const catalogFor = (p: ProviderWithModels) =>
    kinds.find((x) => x.kind === p.api_format)?.models ?? [];

  const handleSave = async () => {
    setSaving(true);
    setStatus("");
    try {
      const apiKey = readApiKey();
      if (addNew) {
        if (!apiKey) {
          setStatus("❌ 请填写 API Key");
          return;
        }
        await api.settings.createProvider({
          kind: addKind,
          api_key: apiKey,
        });
        await refresh();
        cancelEdit(false);
        setStatus("✅ 已添加并保存 API Key");
      } else if (editId) {
        const p = providers.find((x) => x.id === editId);
        if (!p) return;
        if (!apiKey) {
          setStatus("❌ 请粘贴 API Key 后再保存");
          return;
        }
        await api.settings.updateProvider(editId, {
          base_url: p.base_url,
          api_key: apiKey,
        });
        await refresh();
        cancelEdit(false);
        setStatus("✅ API Key 已保存");
      }
    } catch (e: unknown) {
      setStatus(`❌ ${errMsg(e)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("确定删除此厂商？其下所有模型也会被删除。")) return;
    try {
      await api.settings.deleteProvider(id);
      if (editId === id) cancelEdit();
      setStatus("");
      await refresh();
    } catch (e: unknown) {
      setStatus(`❌ ${errMsg(e)}`);
    }
  };

  const handleSetDefaultProvider = async (id: string) => {
    const p = providers.find((x) => x.id === id);
    if (!p) return;
    try {
      await api.settings.updateProvider(id, {
        base_url: p.base_url,
        is_default: true,
      });
      await refresh();
    } catch (e: unknown) {
      setStatus(`❌ ${errMsg(e)}`);
    }
  };

  const handleTest = async (providerId: string, modelName: string) => {
    setTesting(modelName);
    setStatus("");
    try {
      const res = await api.settings.testConnection(providerId, modelName);
      setStatus(res.ok ? `✅ ${res.message}` : `❌ ${res.message}`);
    } catch (e: unknown) {
      setStatus(`❌ ${errMsg(e)}`);
    } finally {
      setTesting(null);
    }
  };

  const handleEnableModel = async (providerId: string, modelName: string) => {
    try {
      await api.settings.createModel(providerId, { model_name: modelName });
      await refresh();
    } catch (e: unknown) {
      setStatus(`❌ ${errMsg(e)}`);
    }
  };

  const handleDisableModel = async (providerId: string, modelId: string) => {
    try {
      await api.settings.deleteModel(providerId, modelId);
      await refresh();
    } catch (e: unknown) {
      setStatus(`❌ ${errMsg(e)}`);
    }
  };

  const handleSetDefaultModel = async (providerId: string, modelId: string) => {
    try {
      await api.settings.setModelDefault(providerId, modelId);
      await refresh();
    } catch (e: unknown) {
      setStatus(`❌ ${errMsg(e)}`);
    }
  };

  const keyPlaceholder = (p?: ProviderWithModels) => {
    if (!p) return "粘贴 DeepSeek API Key";
    if (p.api_key) return `当前 ${p.api_key}，粘贴新密钥以更新`;
    return "尚未配置，请粘贴 API Key";
  };

  return (
    <div className="settings-tab providers-tab">
      <p className="tab-desc">
        配置 DeepSeek API Key，并从官方模型目录启用模型。温度、max_tokens 等采样参数由程序按任务决定，无需手动填写。
      </p>

      {status && !editId && !addNew && <p className="setting-msg provider-status">{status}</p>}

      <ul className="provider-list">
        {providers.map((p) => (
          <li key={p.id} className={`provider-item ${p.is_default ? "default" : ""}`}>
            <div className="provider-item-head">
              <div>
                <span className="provider-name">
                  {p.is_default && <span className="default-dot" aria-hidden />}
                  {p.name}
                  <span className="badge-preset">{kindLabel(p)}</span>
                </span>
                <span className="provider-url">{p.base_url}</span>
                <span className="provider-key-hint">Key：{p.api_key || "未设置"}</span>
              </div>
              <div className="provider-actions">
                {!p.is_default && (
                  <button type="button" className="btn-sm" onClick={() => handleSetDefaultProvider(p.id)}>
                    设为默认
                  </button>
                )}
                <button type="button" className="btn-sm" onClick={() => startEdit(p)}>
                  配置
                </button>
                <button type="button" className="btn-sm btn-danger" onClick={() => handleDelete(p.id)}>
                  删除
                </button>
              </div>
            </div>

            {editId === p.id && (
              <div className="provider-edit-form">
                <h4>配置 {kindLabel(p)}</h4>

                <label htmlFor="provider-api-key">API Key</label>
                <input
                  id="provider-api-key"
                  ref={keyInputRef}
                  type="password"
                  name="api_key"
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={keyPlaceholder(p)}
                />
                <p className="resolved-hint">密钥本地加密存储。浏览器自动填充时也会正确读取。</p>

                <div className="models-section">
                  <h4>官方模型</h4>
                  <p className="resolved-hint">启用后可用于对话与任务；默认模型用于未单独指定任务时。</p>
                  <ul className="model-catalog">
                    {catalogFor(p).map((cat) => {
                      const existing = editProvider?.models.find((m) => m.model_name === cat.model_name);
                      return (
                        <li key={cat.model_name} className="model-catalog-item">
                          <div className="model-catalog-info">
                            <span className="model-catalog-name">{cat.model_name}</span>
                            {existing?.is_default && <span className="badge-preset">默认</span>}
                          </div>
                          <div className="model-catalog-actions">
                            {existing ? (
                              <>
                                {!existing.is_default && (
                                  <button
                                    type="button"
                                    className="btn-sm"
                                    onClick={() => handleSetDefaultModel(p.id, existing.id)}
                                  >
                                    设为默认
                                  </button>
                                )}
                                <button
                                  type="button"
                                  className="btn-sm"
                                  onClick={() => handleTest(p.id, existing.model_name)}
                                  disabled={testing === existing.model_name}
                                >
                                  {testing === existing.model_name ? "测试中…" : "测试"}
                                </button>
                                <button
                                  type="button"
                                  className="btn-sm btn-danger"
                                  onClick={() => handleDisableModel(p.id, existing.id)}
                                  disabled={editProvider != null && editProvider.models.length <= 1}
                                  title={
                                    editProvider != null && editProvider.models.length <= 1
                                      ? "至少保留一个模型"
                                      : "停用"
                                  }
                                >
                                  停用
                                </button>
                              </>
                            ) : (
                              <button
                                type="button"
                                className="btn-sm"
                                onClick={() => handleEnableModel(p.id, cat.model_name)}
                              >
                                启用
                              </button>
                            )}
                          </div>
                        </li>
                      );
                    })}
                  </ul>

                  {editProvider &&
                    editProvider.models.some(
                      (m) => !catalogFor(p).some((c) => c.model_name === m.model_name)
                    ) && (
                      <div className="legacy-models">
                        <h4>其他已保存模型</h4>
                        <ul className="model-catalog">
                          {editProvider.models
                            .filter((m) => !catalogFor(p).some((c) => c.model_name === m.model_name))
                            .map((m) => (
                              <li key={m.id} className="model-catalog-item">
                                <span className="model-catalog-name">{m.model_name}</span>
                                <button
                                  type="button"
                                  className="btn-sm btn-danger"
                                  onClick={() => handleDisableModel(p.id, m.id)}
                                >
                                  删除
                                </button>
                              </li>
                            ))}
                        </ul>
                      </div>
                    )}
                </div>

                <div className="form-actions">
                  {status && <span className="setting-msg">{status}</span>}
                  <button type="button" className="btn-primary" onClick={handleSave} disabled={saving}>
                    {saving ? "保存中…" : "保存密钥"}
                  </button>
                  <button type="button" className="btn-secondary" onClick={() => cancelEdit()}>
                    完成
                  </button>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>

      {addNew && (
        <div className="provider-edit-form provider-add-form">
          <h4>添加厂商</h4>
          <label htmlFor="add-kind">厂商</label>
          <div className="select-wrap">
            <select
              id="add-kind"
              value={addKind}
              onChange={(e) => setAddKind(e.target.value)}
              disabled={availableKinds.length === 0}
            >
              {availableKinds.map((k) => (
                <option key={k.kind} value={k.kind}>
                  {k.display_name}
                </option>
              ))}
            </select>
          </div>
          {selectedKind && (
            <p className="resolved-hint">
              官方地址 {selectedKind.default_base_url} · 将预置{" "}
              {selectedKind.models.map((m) => m.model_name).join("、")}
            </p>
          )}
          <label htmlFor="add-api-key">API Key</label>
          <input
            id="add-api-key"
            ref={keyInputRef}
            type="password"
            name="api_key"
            autoComplete="off"
            spellCheck={false}
            placeholder="粘贴 API Key（必填）"
          />
          <div className="form-actions">
            {status && <span className="setting-msg">{status}</span>}
            <button
              type="button"
              className="btn-primary"
              onClick={handleSave}
              disabled={saving || availableKinds.length === 0}
            >
              {saving ? "保存中…" : "保存"}
            </button>
            <button type="button" className="btn-secondary" onClick={() => cancelEdit()}>
              取消
            </button>
          </div>
        </div>
      )}

      {!addNew && availableKinds.length > 0 && (
        <button type="button" className="btn-secondary" onClick={startAdd}>
          + 添加厂商
        </button>
      )}
      {!addNew && providers.length === 0 && availableKinds.length === 0 && (
        <p className="resolved-hint">暂无可用厂商类型。</p>
      )}
    </div>
  );
}
