import { useEffect } from "react";
import Layout from "./components/Layout";
import { api } from "./api/client";
import { applyLanguage, applyTheme } from "./utils/theme";
import "./App.css";

function App() {
  useEffect(() => {
    api.settings
      .getAll()
      .then((data) => {
        applyTheme(data.general.theme || "system");
        applyLanguage(data.general.language || "zh");
      })
      .catch(() => {
        applyTheme("system");
        applyLanguage("zh");
      });
  }, []);

  return <Layout />;
}

export default App;
