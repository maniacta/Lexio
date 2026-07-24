import { useState } from "react";
import type { SettingsData } from "../../types";
import { api } from "../../api/client";

interface Props {
  settings: SettingsData;
  onSaved: () => void;
}

export default function GeneralTab({ settings, onSaved }: Props) {
  const [theme, setTheme] = useState(settings.general.theme || "system");
  const [language, setLanguage] = useState(settings.general.language || "zh");
  const [dataPath, setDataPath] = useState(settings.general.data_path || "");
  const [searchEnabled, setSearchEnabled] = useState(
    settings.general.search_enabled === "true"
  );
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  const handleSave = async () => {
    setSaving(true);
    setMessage("");
    try {
      await api.settings.updateGeneral({
        theme,
        language,
        data_path: dataPath,
        search_enabled: searchEnabled,
      });
      setMessage("✅ 已保存");
      onSaved();
    } catch (e: any) {
      setMessage(`❌ ${e.message}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="settings-tab general-tab">
      <div className="setting-group">
        <label className="setting-label">主题</label>
        <div className="setting-options">
          {[
            { value: "system", label: "跟随系统" },
            { value: "light", label: "亮色" },
            { value: "dark", label: "暗色" },
          ].map(o => (
            <label key={o.value} className="radio-label">
              <input type="radio" name="theme" value={o.value}
                checked={theme === o.value} onChange={e => setTheme(e.target.value)} />
              {o.label}
            </label>
          ))}
        </div>
      </div>

      <div className="setting-group">
        <label className="setting-label">语言</label>
        <select value={language} onChange={e => setLanguage(e.target.value)}>
          <option value="zh">中文</option>
          <option value="en">English</option>
        </select>
      </div>

      <div className="setting-group">
        <label className="setting-label">数据存储路径</label>
        <input type="text" value={dataPath}
          onChange={e => setDataPath(e.target.value)}
          placeholder="留空使用默认路径" />
      </div>

      <div className="setting-group">
        <label className="setting-label">网络搜索</label>
        <label className="switch-label">
          <input type="checkbox" checked={searchEnabled}
            onChange={e => setSearchEnabled(e.target.checked)} />
          {searchEnabled ? "已启用" : "已禁用"}
        </label>
      </div>

      <div className="setting-actions">
        <span className="setting-msg">{message}</span>
        <button className="btn-primary" onClick={handleSave} disabled={saving}>
          {saving ? "保存中..." : "保存"}
        </button>
      </div>
    </div>
  );
}
