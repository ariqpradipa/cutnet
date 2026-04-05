import { useToastStore } from "@/hooks/useToast";
import { cn } from "@/lib/utils";
import { XIcon, AlertCircle, CheckCircle2 } from "lucide-react";

export function ToastContainer() {
  const toasts = useToastStore((state) => state.toasts);
  const removeToast = useToastStore((state) => state.removeToast);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 w-80">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={cn(
            "flex items-start gap-3 rounded-lg border p-4 shadow-lg bg-background ring-1 ring-foreground/10 animate-in fade-in slide-in-from-bottom-2",
            toast.variant === "destructive"
              ? "border-destructive/50 bg-destructive/10"
              : "border-border"
          )}
        >
          {toast.variant === "destructive" ? (
            <AlertCircle className="size-4 text-destructive shrink-0 mt-0.5" />
          ) : (
            <CheckCircle2 className="size-4 text-primary shrink-0 mt-0.5" />
          )}
          <div className="flex-1 min-w-0">
            {toast.title && (
              <p className="text-sm font-medium">{toast.title}</p>
            )}
            <p className="text-xs text-muted-foreground mt-0.5">
              {toast.description}
            </p>
          </div>
          <button
            onClick={() => removeToast(toast.id)}
            className="shrink-0 text-muted-foreground hover:text-foreground transition-colors"
          >
            <XIcon className="size-3.5" />
          </button>
        </div>
      ))}
    </div>
  );
}
