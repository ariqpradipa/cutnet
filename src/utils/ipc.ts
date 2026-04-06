import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Device,
  NetworkInterface,
  BandwidthLimit,
  BandwidthStats,
  DefenderAlert,
  WhitelistEntry,
  HistoryEntry,
} from "@/lib/schemas";
import { NetworkInterfaceSchema } from "@/lib/schemas";

export interface ApiError {
  code: string;
  user_message: string;
  retryable: boolean;
  suggested_action?: string;
  technical_details?: string;
}

export function isApiError(err: unknown): err is ApiError {
  return (
    typeof err === "object" &&
    err !== null &&
    "code" in err &&
    "user_message" in err &&
    typeof (err as ApiError).code === "string" &&
    typeof (err as ApiError).user_message === "string"
  );
}

export function parseApiError(err: unknown): ApiError {
  if (isApiError(err)) {
    return err;
  }
  
  if (err instanceof Error) {
    return {
      code: "UNKNOWN_ERROR",
      user_message: err.message,
      retryable: false,
    };
  }
  
  if (typeof err === "string") {
    try {
      const parsed = JSON.parse(err);
      if (isApiError(parsed)) {
        return parsed;
      }
    } catch {
      return {
        code: "UNKNOWN_ERROR",
        user_message: err,
        retryable: false,
      };
    }
  }
  
  return {
    code: "UNKNOWN_ERROR",
    user_message: "An unexpected error occurred",
    retryable: false,
  };
}

export async function getInterfaces(): Promise<NetworkInterface[]> {
  const result = await invoke<NetworkInterface[]>("get_interfaces");
  return result.map((iface) => NetworkInterfaceSchema.parse(iface));
}

export async function startArpScan(interfaceName: string): Promise<void> {
  await invoke("start_arp_scan", { interfaceName });
}

export async function startPingScan(interfaceName: string): Promise<void> {
  await invoke("start_ping_scan", { interfaceName });
}

export async function stopScan(): Promise<void> {
  await invoke("stop_scan");
}

export async function killDevice(device: Device): Promise<void> {
  await invoke("kill_device", { 
    ip: device.ip,
    mac: device.mac,
  });
}

export async function unkillDevice(device: Device): Promise<void> {
  await invoke("unkill_device", {
    ip: device.ip,
    mac: device.mac,
  });
}

export async function killAllDevices(devices: Device[]): Promise<void> {
  await invoke("kill_all_devices", {
    devices: devices.map(d => ({ ip: d.ip, mac: d.mac })),
  });
}

export async function unkillAllDevices(): Promise<void> {
  await invoke("unkill_all_devices");
}

export async function getMacAddress(interfaceName: string): Promise<string> {
  return await invoke<string>("get_mac_address", { interfaceName });
}

export async function setMacAddress(
  interfaceName: string,
  newMac: string
): Promise<void> {
  await invoke("set_mac_address", { interfaceName, newMac });
}

export async function cloneMacAddress(
  fromInterface: string,
  toInterface: string
): Promise<void> {
  await invoke("clone_mac_address", { from: fromInterface, to: toInterface });
}

export interface DeviceUpdateEvent {
  type: "device_found" | "device_lost" | "device_updated";
  device: Device;
}

export function onDeviceUpdate(
  callback: (event: DeviceUpdateEvent) => void
): () => void {
  const unlisten = listen<DeviceUpdateEvent>("device-update", (event) => {
    callback(event.payload);
  });

  return async () => {
    (await unlisten)();
  };
}

export interface ScanProgressEvent {
  type: "arp" | "ping";
  progress: number;
  devicesFound: number;
}

export function onScanProgress(
  callback: (event: ScanProgressEvent) => void
): () => void {
  const unlisten = listen<ScanProgressEvent>("scan-progress", (event) => {
    callback(event.payload);
  });

  return async () => {
    (await unlisten)();
  };
}

// Device found event
export interface DeviceFoundEvent {
  device: {
    ip: string;
    mac: string;
    hostname: string | null;
    vendor: string | null;
    is_router: boolean;
    is_me: boolean;
  };
}

export function onDeviceFound(
  callback: (event: DeviceFoundEvent) => void
): () => void {
  const unlisten = listen<DeviceFoundEvent>("device-found", (event) => {
    callback(event.payload);
  });

  return async () => {
    (await unlisten)();
  };
}

// Device lost event
export interface DeviceLostEvent {
  device: {
    ip: string;
    mac: string;
    hostname: string | null;
    vendor: string | null;
    is_router: boolean;
    is_me: boolean;
  };
}

export function onDeviceLost(
  callback: (event: DeviceLostEvent) => void
): () => void {
  const unlisten = listen<DeviceLostEvent>("device-lost", (event) => {
    callback(event.payload);
  });

  return async () => {
    (await unlisten)();
  };
}

// Scan completed event
export interface ScanCompletedEvent {
  total_devices: number;
  success: boolean;
}

export function onScanCompleted(
  callback: (event: ScanCompletedEvent) => void
): () => void {
  const unlisten = listen<ScanCompletedEvent>("scan-completed", (event) => {
    callback(event.payload);
  });

  return async () => {
    (await unlisten)();
  };
}

// Device killed event
export interface DeviceKilledEvent {
  ip: string;
  mac: string;
}

export function onDeviceKilled(
  callback: (event: DeviceKilledEvent) => void
): () => void {
  const unlisten = listen<DeviceKilledEvent>("device-killed", (event) => {
    callback(event.payload);
  });

  return async () => {
    (await unlisten)();
  };
}

// Device restored event
export interface DeviceRestoredEvent {
  ip: string;
  mac: string;
}

export function onDeviceRestored(
  callback: (event: DeviceRestoredEvent) => void
): () => void {
  const unlisten = listen<DeviceRestoredEvent>("device-restored", (event) => {
    callback(event.payload);
  });

  return async () => {
    (await unlisten)();
  };
}

// Error event
export interface IpcErrorEvent {
  message: string;
  code: string | null;
}

export function onError(
  callback: (event: IpcErrorEvent) => void
): () => void {
  const unlisten = listen<IpcErrorEvent>("error", (event) => {
    callback(event.payload);
  });

  return async () => {
    (await unlisten)();
  };
}

// Defender events
export interface ArpSpoofDetectedEvent {
  timestamp: number;
  claimed_ip: string;
  legitimate_mac: string;
  attacker_mac: string;
  alert_type: string;
}

export function onArpSpoofDetected(
  callback: (event: ArpSpoofDetectedEvent) => void
): () => void {
  const unlisten = listen<ArpSpoofDetectedEvent>("arp-spoof-detected", (event) => {
    callback(event.payload);
  });

  return async () => {
    (await unlisten)();
  };
}

// New IPC commands for defender/whitelist/arp-flush
export async function startDefender(): Promise<void> {
  await invoke("start_defender");
}

export async function stopDefender(): Promise<void> {
  await invoke("stop_defender");
}

export async function getDefenderAlerts(): Promise<DefenderAlert[]> {
  return await invoke("get_defender_alerts");
}

export async function clearDefenderAlerts(): Promise<void> {
  await invoke("clear_defender_alerts");
}

export async function isDefenderActive(): Promise<boolean> {
  return await invoke("is_defender_active");
}

export async function addWhitelistEntry(mac: string, label?: string): Promise<void> {
  await invoke("add_whitelist_entry", { mac, label });
}

export async function removeWhitelistEntry(mac: string): Promise<boolean> {
  return await invoke("remove_whitelist_entry", { mac });
}

export async function getWhitelistEntries(): Promise<WhitelistEntry[]> {
  return await invoke("get_whitelist_entries");
}

export async function setWhitelistProtect(enabled: boolean): Promise<void> {
  await invoke("set_whitelist_protect", { enabled });
}

export async function isWhitelisted(mac: string): Promise<boolean> {
  return await invoke("is_whitelisted", { mac });
}

export async function flushArpCache(): Promise<void> {
  await invoke("flush_arp_cache_cmd");
}

export async function getHistory(): Promise<HistoryEntry[]> {
  return await invoke("get_history");
}

export async function clearHistory(): Promise<void> {
  await invoke("clear_history");
}

export async function setDeviceCustomName(ip: string, name: string): Promise<void> {
  await invoke("set_device_custom_name", { ip, name });
}

export async function getCustomDeviceNames(): Promise<Record<string, string>> {
  return await invoke("get_custom_device_names");
}

// Bandwidth control functions
export async function setBandwidthLimit(
  mac: string,
  downloadKbps: number | null,
  uploadKbps: number | null
): Promise<void> {
  await invoke("set_bandwidth_limit", { mac, downloadKbps, uploadKbps });
}

export async function removeBandwidthLimit(mac: string): Promise<void> {
  await invoke("remove_bandwidth_limit", { mac });
}

export async function getBandwidthLimits(): Promise<BandwidthLimit[]> {
  return await invoke("get_bandwidth_limits");
}

export async function getBandwidthStats(mac: string): Promise<BandwidthStats> {
  return await invoke("get_bandwidth_stats", { mac });
}
