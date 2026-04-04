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
