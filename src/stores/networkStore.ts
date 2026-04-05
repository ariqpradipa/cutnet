import { create } from "zustand";
import type { NetworkInterface, ScanProgress, Device } from "@/lib/schemas";

interface NetworkStore {
  interfaces: NetworkInterface[];
  devices: Device[];
  activeInterface: NetworkInterface | null;
  isScanning: boolean;
  scanProgress: ScanProgress | null;
  isRunning: boolean;

  setInterfaces: (interfaces: NetworkInterface[]) => void;
  setDevices: (devices: Device[]) => void;
  setActiveInterface: (iface: NetworkInterface | null) => void;
  setScanProgress: (progress: ScanProgress | null) => void;
  setScanning: (scanning: boolean) => void;
  setIsRunning: (running: boolean) => void;
}

export const useNetworkStore = create<NetworkStore>((set) => ({
  interfaces: [],
  devices: [],
  activeInterface: null,
  isScanning: false,
  scanProgress: null,
  isRunning: false,

  setInterfaces: (interfaces) => set({ interfaces }),
  setDevices: (devices) => set({ devices }),
  setActiveInterface: (iface) => set({ activeInterface: iface }),
  setScanProgress: (progress) => set({ scanProgress: progress }),
  setScanning: (scanning) => set({ isScanning: scanning }),
  setIsRunning: (running) => set({ isRunning: running }),
}));

// Helper to mark app as running (called after interfaces load)
export function markAppRunning() {
  useNetworkStore.getState().setIsRunning(true);
}
