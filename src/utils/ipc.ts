import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Device, NetworkInterface } from "@/lib/schemas";
import { NetworkInterfaceSchema } from "@/lib/schemas";

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
  await invoke("clone_mac_address", { fromInterface, toInterface });
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

export async function getDefenderAlerts(): Promise<any[]> {
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

export async function getWhitelistEntries(): Promise<any[]> {
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
