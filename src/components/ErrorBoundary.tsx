import { Component, ErrorInfo, ReactNode } from "react";
import { Alert, AlertTitle, AlertDescription } from "@/components/ui/alert";
import { AlertCircle, RotateCcw, ChevronDown, ChevronUp, Terminal } from "lucide-react";
import { cn } from "@/lib/utils";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  showDetails: boolean;
}

class ErrorBoundary extends Component<Props, State> {
  state: State = {
    hasError: false,
    error: null,
    showDetails: false,
  };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error, showDetails: false };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("ErrorBoundary caught an error:", error, errorInfo);
  }

  handleReload = () => {
    window.location.reload();
  };

  toggleDetails = () => {
    this.setState((prev) => ({ showDetails: !prev.showDetails }));
  };

  render() {
    if (this.state.hasError) {
      const isDev = import.meta.env.DEV;

      if (isDev) {
        return this.props.children;
      }

      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="min-h-screen flex items-center justify-center p-6 bg-background">
          <div className="w-full max-w-lg animate-in fade-in slide-in-from-bottom-4 duration-500">
            <Alert
              variant="destructive"
              className={cn(
                "border-destructive/50 bg-destructive/5",
                "shadow-2xl shadow-destructive/10"
              )}
            >
              <div className="flex flex-col gap-4">
                <div className="flex items-start gap-3">
                  <div className="shrink-0 p-2 rounded-lg bg-destructive/10">
                    <AlertCircle className="size-6 text-destructive" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <AlertTitle className="text-lg font-semibold text-destructive">
                      Something went wrong
                    </AlertTitle>
                    <AlertDescription className="mt-1 text-sm text-muted-foreground">
                      The application encountered an unexpected error. We&apos;ve noted this issue and are working to fix it.
                    </AlertDescription>
                  </div>
                </div>

                <button
                  onClick={this.handleReload}
                  className={cn(
                    "flex items-center justify-center gap-2",
                    "px-4 py-2.5 rounded-md",
                    "bg-destructive text-destructive-foreground",
                    "hover:bg-destructive/90 transition-colors",
                    "font-medium text-sm"
                  )}
                >
                  <RotateCcw className="size-4" />
                  Reload Application
                </button>

                {this.state.error && (
                  <div className="border-t border-destructive/20 pt-4">
                    <button
                      onClick={this.toggleDetails}
                      className={cn(
                        "flex items-center gap-2",
                        "text-xs text-muted-foreground",
                        "hover:text-foreground transition-colors"
                      )}
                    >
                      <Terminal className="size-3.5" />
                      <span>Technical Details</span>
                      {this.state.showDetails ? (
                        <ChevronUp className="size-3.5" />
                      ) : (
                        <ChevronDown className="size-3.5" />
                      )}
                    </button>

                    {this.state.showDetails && (
                      <div className="mt-3 space-y-2 animate-in fade-in slide-in-from-top-2">
                        <div className="rounded-md bg-destructive/10 p-3 font-mono text-xs overflow-auto max-h-40">
                          <p className="text-destructive font-semibold">
                            {this.state.error.name}: {this.state.error.message}
                          </p>
                          {this.state.error.stack && (
                            <pre className="mt-2 text-muted-foreground whitespace-pre-wrap break-all">
                              {this.state.error.stack}
                            </pre>
                          )}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            </Alert>

            <p className="mt-4 text-center text-xs text-muted-foreground">
              If this problem persists, please try clearing your browser cache or contact support.
            </p>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
