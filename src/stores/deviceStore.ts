import { create } from "zustand";
import type { Device, KillState, BandwidthLimit, BandwidthStats } from "@/lib/schemas";

interface DeviceStore {
  devices: Device[];
  selectedDevice: Device | null;
  killStates: Map<string, KillState>;
  bandwidthLimits: Map<string, BandwidthLimit>;
  bandwidthStats: Map<string, BandwidthStats>;

  setDevices: (devices: Device[]) => void;
  addDevice: (device: Device) => void;
  updateDevice: (ip: string, updates: Partial<Device>) => void;
  removeDevice: (ip: string) => void;
  selectDevice: (device: Device | null) => void;
  setKillState: (mac: string, state: KillState) => void;
  clearKillStates: () => void;
  setBandwidthLimit: (mac: string, limit: BandwidthLimit) => void;
  removeBandwidthLimit: (mac: string) => void;
  setBandwidthStats: (mac: string, stats: BandwidthStats) => void;
  clearBandwidthData: () => void;
}

export const useDeviceStore = create<DeviceStore>((set, get) => ({
  devices: [],
  selectedDevice: null,
  killStates: new Map(),
  bandwidthLimits: new Map(),
  bandwidthStats: new Map(),

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

  setBandwidthLimit: (mac, limit) => {
    const bandwidthLimits = new Map(get().bandwidthLimits);
    bandwidthLimits.set(mac, limit);
    set({ bandwidthLimits });
  },

  removeBandwidthLimit: (mac) => {
    const bandwidthLimits = new Map(get().bandwidthLimits);
    bandwidthLimits.delete(mac);
    set({ bandwidthLimits });
  },

  setBandwidthStats: (mac, stats) => {
    const bandwidthStats = new Map(get().bandwidthStats);
    bandwidthStats.set(mac, stats);
    set({ bandwidthStats });
  },

  clearBandwidthData: () => set({ 
    bandwidthLimits: new Map(),
    bandwidthStats: new Map()
  }),
}));
