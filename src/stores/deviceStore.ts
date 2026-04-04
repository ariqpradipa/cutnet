import { create } from "zustand";
import type { Device, KillState } from "@/lib/schemas";

interface DeviceStore {
  devices: Device[];
  selectedDevice: Device | null;
  killStates: Map<string, KillState>;

  setDevices: (devices: Device[]) => void;
  addDevice: (device: Device) => void;
  updateDevice: (ip: string, updates: Partial<Device>) => void;
  removeDevice: (ip: string) => void;
  selectDevice: (device: Device | null) => void;
  setKillState: (mac: string, state: KillState) => void;
  clearKillStates: () => void;
}

export const useDeviceStore = create<DeviceStore>((set, get) => ({
  devices: [],
  selectedDevice: null,
  killStates: new Map(),

  setDevices: (devices) => set({ devices }),

  addDevice: (device) => {
    const currentDevices = get().devices;
    const existingIndex = currentDevices.findIndex(d => d.ip === device.ip);
    
    if (existingIndex >= 0) {
      const newDevices = [...currentDevices];
      newDevices[existingIndex] = device;
      set({ devices: newDevices });
    } else {
      set({ devices: [...currentDevices, device] });
    }
  },

  updateDevice: (ip, updates) => {
    const currentDevices = get().devices;
    const newDevices = currentDevices.map(d => 
      d.ip === ip ? { ...d, ...updates } : d
    );
    set({ devices: newDevices });
  },

  removeDevice: (ip) => {
    const currentDevices = get().devices;
    set({ devices: currentDevices.filter(d => d.ip !== ip) });
  },

  selectDevice: (device) => set({ selectedDevice: device }),

  setKillState: (mac, state) => {
    const killStates = new Map(get().killStates);
    killStates.set(mac, state);
    set({ killStates });
  },

  clearKillStates: () => set({ killStates: new Map() }),
}));
