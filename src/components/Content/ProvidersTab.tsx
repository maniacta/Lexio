import { useEffect, useState } from "react";
import type { SettingsData, ProviderWithModels, ProviderKindInfo } from "../../types";
import { api } from "../../api/client";

interface Props {
  settings: SettingsData;
  onSaved: () => void;
}

export default function ProvidersTab({ settings, onSaved }: Props) {
  const [providers, setProviders] = useState(settings.providers);
  const [kinds, setKinds] = useState<ProviderKindInfo[]>([]);
  const [editId, setEditId] = useState<string | null>(null);
  const [addNew, setAddNew] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState("");

  const [formUrl, setFormUrl] = useState("");
  const [formKey, setFormKey] = useState("");
  const [addKind, setAddKind] = useState("deepseek");

  const [modelName, setModelName] = useState("");
  const [modelTemp, setModelTemp] = useState(0.7);
  const [modelTokens, setModelTokens] = useState(4096);

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

  const startEdit = (p: ProviderWithModels) => {
    setEditId(p.id);
    setAddNew(false);
    setFormUrl(p.base_url);
    setFormKey("");
    setTestResult("");
  };

  const startAdd = () => {
    setEditId(null);
    setAddNew(true);
    const first = availableKinds[0];
    setAddKind(first?.kind ?? "deepseek");
    setFormUrl(first?.default_base_url ?? "");
    setFormKey("");
    setTestResult("");
  };

  const cancelEdit = () => {
    setEditId(null);
    setAddNew(false);
  };

  const handleKindChange = (kind: string) => {
    setAddKind(kind);
    const info = kinds.find((k) => k.kind === kind);
    if (info) setFormUrl(info.default_base_url);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      if (addNew) {
        await api.settings.createProvider({
          kind: addKind,
          api_key: formKey,
          base_url: formUrl || undefined,
        });
      } else if (editId) {
        await api.settings.updateProvider(editId, {
          base_url: formUrl,
          api_key: formKey || undefined,
        });
      }
      cancelEdit();
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("确定删除此厂商？其下所有模型也会被删除。")) return;
    try {
      await api.settings.deleteProvider(id);
      if (editId === id) cancelEdit();
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
      setTestResult("");
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    }
  };

  const handleSetDefault = async (id: string) => {
    const p = providers.find((x) => x.id === id);
    if (!p) return;
    try {
      await api.settings.updateProvider(id, {
        base_url: p.base_url,
        is_default: true,
      });
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    }
  };

  const handleTest = async (providerId: string, modelName: string) => {
    setTesting(modelName);
    setTestResult("");
    try {
      const res = await api.settings.testConnection(providerId, modelName);
      setTestResult(res.ok ? `✅ ${res.message}` : `❌ ${res.message}`);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    } finally {
      setTesting(null);
    }
  };

  const handleAddModel = async () => {
    if (!editId || !modelName) return;
    try {
      await api.settings.createModel(editId, {
        model_name: modelName,
        temperature: modelTemp,
        max_tokens: modelTokens,
      });
      setModelName("");
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    }
  };

  const handleDeleteModel = async (providerId: string, modelId: string) => {
    try {
      await api.settings.deleteModel(providerId, modelId);
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    }
  };

  const kindLabel = (p: ProviderWithModels) => {
    const k = kinds.find((x) => x.kind === p.api_format);
    return k?.display_name ?? p.api_format;
  };

  const kindImplemented = (p: ProviderWithModels) =>
    kinds.find((x) => x.kind === p.api_format)?.implemented ?? p.api_format === "deepseek";

  const suggestedModels = (p: ProviderWithModels) =>
    kinds.find((x) => x.kind === p.api_format)?.models.map((m) => m.model_name) ?? [];

  return (
    <div className="settings-tab providers-tab">
      <p className="tab-desc">
        每个厂商独立适配。当前已接入 DeepSeek；OpenAI / Anthropic 可先配置，调用能力后续单独接入。
      </p>

      <ul className="provider-list">
        {providers.map((p) => (
          <li key={p.id} className={`provider-item ${p.is_default ? "default" : ""}`}>
            <span className="provider-name">
              {p.is_default && "● "}
              {p.name}
              <span className="badge-preset">{kindLabel(p)}</span>
              {!kindImplemented(p) && <span className="badge-preset">未接入调用</span>}
            </span>
            <span className="provider-url">{p.base_url}</span>
            <div className="provider-actions">
              {!p.is_default && (
                <button className="btn-sm" onClick={() => handleSetDefault(p.id)}>
                  设为默认
                </button>
              )}
              <button className="btn-sm" onClick={() => startEdit(p)}>
                编辑
              </button>
              <button className="btn-sm btn-danger" onClick={() => handleDelete(p.id)}>
                删除
              </button>
            </div>

            {editId === p.id && (
              <div className="provider-edit-form">
                <h4>编辑 {kindLabel(p)}</h4>
                <label>厂商类型</label>
                <input value={kindLabel(p)} disabled />
                <label>Base URL</label>
                <input value={formUrl} onChange={(e) => setFormUrl(e.target.value)} />
                <label>API Key</label>
                <input
                  type="password"
                  value={formKey}
                  onChange={(e) => setFormKey(e.target.value)}
                  placeholder="留空则不修改"
                />

                {editProvider && (
                  <div className="models-section">
                    <h4>模型列表</h4>
                    <ul>
                      {editProvider.models.map((m) => (
                        <li key={m.id} className="model-item">
                          <span>{m.model_name}</span>
                          <span>Temp: {m.temperature}</span>
                          <span>Tokens: {m.max_tokens}</span>
                          <button
                            className="btn-sm btn-danger"
                            onClick={() => handleDeleteModel(editId!, m.id)}
                          >
                            删除
                          </button>
                          <button
                            className="btn-sm"
                            onClick={() => handleTest(editId!, m.model_name)}
                            disabled={testing === m.model_name || !kindImplemented(p)}
                            title={!kindImplemented(p) ? "该厂商调用尚未接入" : "测试连接"}
                          >
                            {testing === m.model_name ? "测试中..." : "测试"}
                          </button>
                        </li>
                      ))}
                    </ul>
                    <div className="add-model-row">
                      {suggestedModels(p).length > 0 ? (
                        <select
                          value={modelName}
                          onChange={(e) => setModelName(e.target.value)}
                        >
                          <option value="">选择模型…</option>
                          {suggestedModels(p).map((name) => (
                            <option key={name} value={name}>
                              {name}
                            </option>
                          ))}
                        </select>
                      ) : (
                        <input
                          placeholder="模型名"
                          value={modelName}
                          onChange={(e) => setModelName(e.target.value)}
                        />
                      )}
                      <input
                        type="number"
                        value={modelTemp}
                        step="0.1"
                        min="0"
                        max="2"
                        onChange={(e) => setModelTemp(+e.target.value)}
                        title="Temperature"
                      />
                      <input
                        type="number"
                        value={modelTokens}
                        onChange={(e) => setModelTokens(+e.target.value)}
                        title="Max Tokens"
                      />
                      <button className="btn-sm" onClick={handleAddModel}>
                        + 添加
                      </button>
                    </div>
                  </div>
                )}

                <div className="form-actions">
                  {testResult && <span className="setting-msg">{testResult}</span>}
                  <button className="btn-primary" onClick={handleSave} disabled={saving}>
                    {saving ? "保存中..." : "保存"}
                  </button>
                  <button className="btn-secondary" onClick={cancelEdit}>
                    取消
                  </button>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>

      {addNew && (
        <div className="provider-edit-form">
          <h4>添加厂商</h4>
          <label>厂商类型</label>
          <select value={addKind} onChange={(e) => handleKindChange(e.target.value)}>
            {availableKinds.map((k) => (
              <option key={k.kind} value={k.kind}>
                {k.display_name}
                {!k.implemented ? "（配置可用，调用未接入）" : ""}
              </option>
            ))}
          </select>
          {selectedKind && (
            <p className="resolved-hint">
              将预置模型：{selectedKind.models.map((m) => m.model_name).join("、")}
            </p>
          )}
          <label>Base URL</label>
          <input value={formUrl} onChange={(e) => setFormUrl(e.target.value)} />
          <label>API Key</label>
          <input
            type="password"
            value={formKey}
            onChange={(e) => setFormKey(e.target.value)}
          />

          <div className="form-actions">
            {testResult && <span className="setting-msg">{testResult}</span>}
            <button
              className="btn-primary"
              onClick={handleSave}
              disabled={saving || availableKinds.length === 0}
            >
              {saving ? "保存中..." : "保存"}
            </button>
            <button className="btn-secondary" onClick={cancelEdit}>
              取消
            </button>
          </div>
        </div>
      )}

      {!addNew && availableKinds.length > 0 && (
        <button className="btn-secondary" onClick={startAdd}>
          + 添加厂商
        </button>
      )}
      {!addNew && availableKinds.length === 0 && (
        <p className="resolved-hint">已添加全部支持的厂商类型。</p>
      )}
    </div>
  );
}
