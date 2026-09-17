import { Component, type ErrorInfo, type ReactNode } from "react";
import { logger } from "../utils/logger";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Catches render-time errors so a single bad message payload degrades to an
 * inline notice instead of unmounting the whole React tree (which previously
 * caused a white screen that recurred on every reload of the same session).
 */
export default class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    logger.log({
      level: "error",
      category: "ui",
      action: "render_error",
      user_action: "界面渲染出错",
      error_message: error.message,
      params_summary: { componentStack: info.componentStack?.slice(0, 500) },
    });
  }

  private reset = () => this.setState({ error: null });

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="error-boundary" role="alert">
        <h3>界面出错了</h3>
        <p className="error-boundary-detail">{error.message}</p>
        <div className="error-boundary-actions">
          <button type="button" className="btn-secondary" onClick={this.reset}>
            重试
          </button>
          <button
            type="button"
            className="btn-secondary"
            onClick={() => window.location.reload()}
          >
            重新加载
          </button>
        </div>
      </div>
    );
  }
}
