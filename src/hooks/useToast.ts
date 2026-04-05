import { create } from "zustand";

export interface Toast {
  id: string;
  title?: string;
  description: string;
  variant?: "default" | "destructive";
  duration?: number;
}

interface ToastTimer {
  timeoutId: number | null;
  remainingTime: number;
  startTime: number;
  isPaused: boolean;
}

interface ToastStore {
  toasts: Toast[];
  timers: Map<string, ToastTimer>;
  addToast: (toast: Omit<Toast, "id">) => string;
  removeToast: (id: string) => void;
  pauseToast: (id: string) => void;
  resumeToast: (id: string) => void;
}

export const useToastStore = create<ToastStore>((set, get) => ({
  toasts: [],
  timers: new Map(),

  addToast: (toast) => {
    const id = Math.random().toString(36).slice(2, 9);
    const duration = toast.duration ?? 4000;

    set((state) => ({
      toasts: [...state.toasts, { ...toast, id, duration }],
    }));

    if (duration > 0) {
      const timeoutId = window.setTimeout(() => {
        get().removeToast(id);
      }, duration);

      set((state) => {
        const newTimers = new Map(state.timers);
        newTimers.set(id, {
          timeoutId,
          remainingTime: duration,
          startTime: Date.now(),
          isPaused: false,
        });
        return { timers: newTimers };
      });
    }

    return id;
  },

  removeToast: (id) => {
    set((state) => {
      const timer = state.timers.get(id);
      if (timer?.timeoutId) {
        window.clearTimeout(timer.timeoutId);
      }
      const newTimers = new Map(state.timers);
      newTimers.delete(id);
      return {
        toasts: state.toasts.filter((t) => t.id !== id),
        timers: newTimers,
      };
    });
  },

  pauseToast: (id) => {
    set((state) => {
      const timer = state.timers.get(id);
      if (!timer || timer.isPaused || !timer.timeoutId) return state;

      window.clearTimeout(timer.timeoutId);

      const elapsed = Date.now() - timer.startTime;
      const remainingTime = Math.max(0, timer.remainingTime - elapsed);

      const newTimers = new Map(state.timers);
      newTimers.set(id, {
        ...timer,
        timeoutId: null,
        remainingTime,
        isPaused: true,
      });

      return { timers: newTimers };
    });
  },

  resumeToast: (id) => {
    set((state) => {
      const timer = state.timers.get(id);
      if (!timer || !timer.isPaused) return state;

      const resumeDuration = timer.remainingTime > 0 ? timer.remainingTime : 2000;

      const timeoutId = window.setTimeout(() => {
        get().removeToast(id);
      }, resumeDuration);

      const newTimers = new Map(state.timers);
      newTimers.set(id, {
        ...timer,
        timeoutId,
        startTime: Date.now(),
        remainingTime: resumeDuration,
        isPaused: false,
      });

      return { timers: newTimers };
    });
  },
}));

export function useToast() {
  const addToast = useToastStore((state) => state.addToast);
  const removeToast = useToastStore((state) => state.removeToast);

  return {
    toast: (message: string | { title?: string; description: string; variant?: "default" | "destructive"; duration?: number }) => {
      if (typeof message === "string") {
        return addToast({ description: message });
      } else {
        return addToast(message);
      }
    },
    dismiss: removeToast,
  };
}
