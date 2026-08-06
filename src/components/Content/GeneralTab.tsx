import { useState } from "react";
import type { SettingsData } from "../../types";
import { api } from "../../api/client";
import { applyLanguage, applyTheme } from "../../utils/theme";

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

  const handleThemeChange = (value: string) => {
    setTheme(value);
    applyTheme(value);
  };

  const handleLanguageChange = (value: string) => {
    setLanguage(value);
    applyLanguage(value);
  };

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
      applyTheme(theme);
      applyLanguage(language);
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
                checked={theme === o.value} onChange={e => handleThemeChange(e.target.value)} />
              {o.label}
            </label>
          ))}
        </div>
        <p className="setting-hint">切换后立即生效，点保存可持久化。</p>
      </div>

      <div className="setting-group">
        <label className="setting-label">语言</label>
        <select value={language} onChange={e => handleLanguageChange(e.target.value)}>
          <option value="zh">中文</option>
          <option value="en">English（界面文案暂仍为中文）</option>
        </select>
        <p className="setting-hint">完整英文界面尚未完成，目前仅更新页面语言标记。</p>
      </div>

      <div className="setting-group">
        <label className="setting-label">数据存储路径</label>
        <input type="text" value={dataPath}
          onChange={e => setDataPath(e.target.value)}
          placeholder="留空使用默认路径" />
        <p className="setting-hint">自定义路径即将支持，当前仍使用应用默认目录。</p>
      </div>

      <div className="setting-group">
        <label className="setting-label">网络搜索</label>
        <label className="switch-label">
          <input type="checkbox" checked={searchEnabled}
            onChange={e => setSearchEnabled(e.target.checked)} />
          {searchEnabled ? "已启用" : "已禁用"}
        </label>
        <p className="setting-hint">真实联网搜索尚未接入；研究主题时由 AI 模拟整理资料。</p>
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
