import Sidebar from "./Sidebar";
import Content from "./Content";
import "./Layout.css";

export default function Layout() {
  return (
    <div className="layout">
      <Sidebar />
      <Content />
    </div>
  );
}
