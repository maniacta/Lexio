import { useState } from "react";
import type { SettingsData } from "../../types";
import { api } from "../../api/client";

interface Props {
  settings: SettingsData;
  onSaved: () => void;
}

const TASK_LABELS: Record<string, string> = {
  chat: "对话 / 主题研究",
  quiz_gen: "测验生成",
};

export default function TaskModelsTab({ settings, onSaved }: Props) {
  const [taskModels, setTaskModels] = useState(settings.task_models);
  const [saving, setSaving] = useState<string | null>(null);
  const [message, setMessage] = useState("");

  const buildOptions = () => {
    const opts: { label: string; modelId: string | null }[] = [
      { label: "跟随默认", modelId: null },
    ];
    for (const p of settings.providers) {
      for (const m of p.models) {
        opts.push({ label: `${p.name} / ${m.model_name}`, modelId: m.id });
      }
    }
    return opts;
  };

  const options = buildOptions();

  const handleChange = async (taskName: string, modelId: string | null) => {
    setSaving(taskName);
    setMessage("");
    try {
      await api.settings.setTaskModel(taskName, modelId);
      setTaskModels(prev => ({
        ...prev,
        [taskName]: { model_id: modelId, resolved: options.find(o => o.modelId === modelId)?.label || null },
      }));
      setMessage("✅ 已保存");
      onSaved();
    } catch (e: any) {
      setMessage(`❌ ${e.message}`);
    } finally {
      setSaving(null);
    }
  };

  return (
    <div className="settings-tab taskmodels-tab">
      <p className="tab-desc">
        为已接线的 AI 任务指定模型。未指定时使用默认厂商。主题研究与知识点提取共用「对话 / 主题研究」。
      </p>
      {Object.entries(TASK_LABELS).map(([taskName, label]) => {
        const current = taskModels[taskName];
        return (
          <div key={taskName} className="setting-group">
            <label className="setting-label">{label}</label>
            <select
              value={current?.model_id || ""}
              onChange={e => handleChange(taskName, e.target.value || null)}
              disabled={saving === taskName}
            >
              {options.map(o => (
                <option key={o.modelId || "__default"} value={o.modelId || ""}>
                  {o.label}
                </option>
              ))}
            </select>
            {current?.resolved && (
              <span className="resolved-hint">当前生效：{current.resolved}</span>
            )}
          </div>
        );
      })}
      {message && <p className="setting-msg">{message}</p>}
    </div>
  );
}
