import { useState } from "react";
import type { SettingsData, ProviderWithModels, UpdateProviderRequest } from "../../types";
import { api } from "../../api/client";

interface Props {
  settings: SettingsData;
  onSaved: () => void;
}

export default function ProvidersTab({ settings, onSaved }: Props) {
  const [providers, setProviders] = useState(settings.providers);
  const [editId, setEditId] = useState<string | null>(null);
  const [addNew, setAddNew] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<string>("");

  // Form state for editing/adding
  const [formName, setFormName] = useState("");
  const [formUrl, setFormUrl] = useState("");
  const [formKey, setFormKey] = useState("");

  const startEdit = (p: ProviderWithModels) => {
    setEditId(p.id);
    setAddNew(false);
    setFormName(p.name);
    setFormUrl(p.base_url);
    setFormKey("");
    setTestResult("");
  };

  const startAdd = () => {
    setEditId(null);
    setAddNew(true);
    setFormName("");
    setFormUrl("");
    setFormKey("");
    setTestResult("");
  };

  const cancelEdit = () => {
    setEditId(null);
    setAddNew(false);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      if (addNew) {
        await api.settings.createProvider({
          name: formName, base_url: formUrl, api_key: formKey,
        });
      } else if (editId) {
        await api.settings.updateProvider(editId, {
          name: formName, base_url: formUrl, api_key: formKey || undefined,
        } as UpdateProviderRequest);
      }
      cancelEdit();
      onSaved();
      // Reload
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string, isPreset: boolean) => {
    if (isPreset) {
      setTestResult("❌ 预设厂商不可删除");
      return;
    }
    if (!confirm("确定删除此厂商？所有关联模型也将被删除。")) return;
    try {
      await api.settings.deleteProvider(id);
      onSaved();
      const data = await api.settings.getAll();
      setProviders(data.providers);
    } catch (e: any) {
      setTestResult(`❌ ${e.message}`);
    }
  };

  const handleSetDefault = async (id: string) => {
    try {
      await api.settings.updateProvider(id, {
        name: providers.find(p => p.id === id)!.name,
        base_url: providers.find(p => p.id === id)!.base_url,
        api_key: undefined as unknown as string,
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

  // Model operations within the edit form
  const editProvider = providers.find(p => p.id === editId);
  const [modelName, setModelName] = useState("");
  const [modelTemp, setModelTemp] = useState(0.7);
  const [modelTokens, setModelTokens] = useState(4096);

  const handleAddModel = async () => {
    if (!editId || !modelName) return;
    try {
      await api.settings.createModel(editId, {
        model_name: modelName, temperature: modelTemp, max_tokens: modelTokens,
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

  return (
    <div className="settings-tab providers-tab">
      <ul className="provider-list">
        {providers.map(p => (
          <li key={p.id} className={`provider-item ${p.is_default ? "default" : ""}`}>
            <span className="provider-name">
              {p.is_default && "● "}{p.name}
              {p.is_preset && <span className="badge-preset">预设</span>}
            </span>
            <span className="provider-url">{p.base_url}</span>
            <div className="provider-actions">
              {!p.is_default && (
                <button className="btn-sm" onClick={() => handleSetDefault(p.id)}>设为默认</button>
              )}
              <button className="btn-sm" onClick={() => startEdit(p)}>编辑</button>
              <button className="btn-sm btn-danger" onClick={() => handleDelete(p.id, p.is_preset)}
                disabled={p.is_preset}>删除</button>
            </div>

            {(editId === p.id || addNew) && (
              <div className="provider-edit-form">
                <h4>{addNew ? "添加厂商" : `编辑 ${p.name}`}</h4>
                <label>名称</label>
                <input value={formName} onChange={e => setFormName(e.target.value)} />
                <label>Base URL</label>
                <input value={formUrl} onChange={e => setFormUrl(e.target.value)} />
                <label>API Key</label>
                <input type="password" value={formKey} onChange={e => setFormKey(e.target.value)}
                  placeholder={!addNew ? "留空则不修改" : ""} />

                {editId && editProvider && (
                  <div className="models-section">
                    <h4>模型列表</h4>
                    <ul>
                      {editProvider.models.map(m => (
                        <li key={m.id} className="model-item">
                          <span>{m.model_name}</span>
                          <span>Temp: {m.temperature}</span>
                          <span>Tokens: {m.max_tokens}</span>
                          <button className="btn-sm btn-danger"
                            onClick={() => handleDeleteModel(editId!, m.id)}
                            disabled={editProvider.models.length <= 1}>
                            删除
                          </button>
                          <button className="btn-sm"
                            onClick={() => handleTest(editId!, m.model_name)}
                            disabled={testing === m.model_name}>
                            {testing === m.model_name ? "测试中..." : "测试"}
                          </button>
                        </li>
                      ))}
                    </ul>
                    <div className="add-model-row">
                      <input placeholder="模型名" value={modelName}
                        onChange={e => setModelName(e.target.value)} />
                      <input type="number" value={modelTemp} step="0.1" min="0" max="2"
                        onChange={e => setModelTemp(+e.target.value)} title="Temperature" />
                      <input type="number" value={modelTokens}
                        onChange={e => setModelTokens(+e.target.value)} title="Max Tokens" />
                      <button className="btn-sm" onClick={handleAddModel}>+ 添加</button>
                    </div>
                  </div>
                )}

                <div className="form-actions">
                  {testResult && <span className="setting-msg">{testResult}</span>}
                  <button className="btn-primary" onClick={handleSave} disabled={saving}>
                    {saving ? "保存中..." : "保存"}
                  </button>
                  <button className="btn-secondary" onClick={cancelEdit}>取消</button>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>
      {!addNew && <button className="btn-secondary" onClick={startAdd}>+ 添加厂商</button>}
    </div>
  );
}
