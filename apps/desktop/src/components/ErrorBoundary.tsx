import { Component, type ErrorInfo, type ReactNode } from "react";
import { useI18n } from "../lib/i18n";

interface InnerProps {
  children: ReactNode;
  labels: { title: string; details: string; reload: string };
}

interface State {
  error: Error | null;
  info: string;
}

class InnerBoundary extends Component<InnerProps, State> {
  state: State = { error: null, info: "" };

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    this.setState({ info: info.componentStack ?? "" });
  }

  render() {
    if (this.state.error) {
      const { labels } = this.props;
      return (
        <div className="flex h-screen w-screen flex-col items-center justify-center gap-4 bg-void-0 p-8">
          <p className="font-display text-sm font-bold uppercase tracking-[0.2em] text-alerte">
            {labels.title}
          </p>
          <p className="max-w-xl break-words text-center font-mono text-[13px] text-mist-1">
            {this.state.error.message}
          </p>
          <details className="max-w-xl text-left">
            <summary className="cursor-pointer text-[12px] text-mist-3">
              {labels.details}
            </summary>
            <pre className="mt-2 max-h-64 overflow-auto rounded-xl bg-void-4 p-3 font-mono text-[11px] leading-relaxed text-mist-2">
              {this.state.error.stack}
              {"\n"}
              {this.state.info}
            </pre>
          </details>
          <button
            onClick={() => window.location.reload()}
            className="btn-press focus-glow mt-2 rounded-xl bg-nebula px-5 py-2.5 font-display text-[13px] font-bold uppercase tracking-wider text-void-0 shadow-lg shadow-nebula/20 hover:bg-nebula-hi"
          >
            {labels.reload}
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

export default function ErrorBoundary({ children }: { children: ReactNode }) {
  const { t } = useI18n();
  return (
    <InnerBoundary
      labels={{
        title: t("error.title"),
        details: t("error.details"),
        reload: t("error.reload"),
      }}
    >
      {children}
    </InnerBoundary>
  );
}
