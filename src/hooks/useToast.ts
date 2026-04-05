import { create } from "zustand";

export interface Toast {
  id: string;
  title?: string;
  description: string;
  variant?: "default" | "destructive";
  duration?: number;
}

interface ToastStore {
  toasts: Toast[];
  addToast: (toast: Omit<Toast, "id">) => void;
  removeToast: (id: string) => void;
}

export const useToastStore = create<ToastStore>((set) => ({
  toasts: [],
  addToast: (toast) => {
    const id = Math.random().toString(36).slice(2, 9);
    set((state) => ({
      toasts: [...state.toasts, { ...toast, id, duration: toast.duration ?? 4000 }],
    }));

    // Auto-dismiss
    const duration = toast.duration ?? 4000;
    if (duration > 0) {
      setTimeout(() => {
        set((state) => ({
          toasts: state.toasts.filter((t) => t.id !== id),
        }));
      }, duration);
    }
  },
  removeToast: (id) => {
    set((state) => ({
      toasts: state.toasts.filter((t) => t.id !== id),
    }));
  },
}));

export function useToast() {
  const addToast = useToastStore((state) => state.addToast);
  const removeToast = useToastStore((state) => state.removeToast);

  return {
    toast: (message: string | { title?: string; description: string; variant?: "default" | "destructive"; duration?: number }) => {
      if (typeof message === "string") {
        addToast({ description: message });
      } else {
        addToast(message);
      }
    },
    dismiss: removeToast,
  };
}
