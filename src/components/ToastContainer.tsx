import { useRef, useCallback } from "react";
import { useToastStore } from "@/hooks/useToast";
import { cn } from "@/lib/utils";
import { XIcon, AlertCircle, CheckCircle2, Info, AlertTriangle } from "lucide-react";

const iconMap = {
  default: CheckCircle2,
  destructive: AlertCircle,
  info: Info,
  warning: AlertTriangle,
};

export function ToastContainer() {
  const toasts = useToastStore((state) => state.toasts);
  const removeToast = useToastStore((state) => state.removeToast);
  const pauseToast = useToastStore((state) => state.pauseToast);
  const resumeToast = useToastStore((state) => state.resumeToast);
  const toastRefs = useRef<Map<string, HTMLDivElement>>(new Map());

  const handleMouseEnter = useCallback((id: string) => {
    pauseToast(id);
  }, [pauseToast]);

  const handleMouseLeave = useCallback((id: string) => {
    resumeToast(id);
  }, [resumeToast]);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 w-80">
      {toasts.map((toast) => {
        const Icon = iconMap[toast.variant as keyof typeof iconMap] || CheckCircle2;
        const isDestructive = toast.variant === "destructive";

        return (
          <div
            key={toast.id}
            ref={(el) => {
              if (el) {
                toastRefs.current.set(toast.id, el);
              } else {
                toastRefs.current.delete(toast.id);
              }
            }}
            onMouseEnter={() => handleMouseEnter(toast.id)}
            onMouseLeave={() => handleMouseLeave(toast.id)}
            className={cn(
              "flex items-start gap-3 rounded-lg border p-4 shadow-lg bg-background ring-1 ring-foreground/10 animate-in fade-in slide-in-from-bottom-2 cursor-default",
              isDestructive
                ? "border-destructive/50 bg-destructive/10"
                : "border-border"
            )}
          >
            <Icon
              className={cn(
                "size-4 shrink-0 mt-0.5",
                isDestructive ? "text-destructive" : "text-primary"
              )}
            />
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
        );
      })}
    </div>
  );
}
