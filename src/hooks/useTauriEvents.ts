import { useEffect } from "react";
import {
  onDeviceKilled,
  onDeviceRestored,
  onError,
  onDeviceUpdate,
  onDeviceFound,
  onDeviceLost,
  onScanCompleted,
  onArpSpoofDetected,
  type DeviceKilledEvent,
  type DeviceRestoredEvent,
  type IpcErrorEvent,
  type DeviceUpdateEvent,
  type DeviceFoundEvent,
  type DeviceLostEvent,
  type ScanCompletedEvent,
  type ArpSpoofDetectedEvent,
} from "@/utils/ipc";
import { useDeviceStore } from "@/stores/deviceStore";
import { useNetworkStore } from "@/stores/networkStore";
import { useToastStore } from "@/hooks/useToast";

/**
 * Custom hook that sets up all Tauri IPC event listeners.
 * Listens for device state changes, scan events, errors, and defender alerts.
 * Cleans up all listeners on unmount.
 */
export function useTauriEvents() {
  const { setKillState, addDevice, updateDevice, removeDevice } =
    useDeviceStore();
  const { setScanning } = useNetworkStore();
  const { addToast } = useToastStore();

  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    // Device killed → update kill state to active
    const unlistenDeviceKilled = onDeviceKilled(
      (event: DeviceKilledEvent) => {
        setKillState(event.mac, {
          mac: event.mac,
          is_killed: true,
          kill_type: "arp_poison",
        });
        addToast({
          title: "Device Killed",
          description: `${event.ip} (${event.mac}) has been disconnected`,
          variant: "destructive",
        });
      }
    );
    unlisteners.push(unlistenDeviceKilled);

    // Device restored → update kill state to inactive
    const unlistenDeviceRestored = onDeviceRestored(
      (event: DeviceRestoredEvent) => {
        setKillState(event.mac, {
          mac: event.mac,
          is_killed: false,
          kill_type: "none",
        });
        addToast({
          title: "Device Restored",
          description: `${event.ip} (${event.mac}) has been reconnected`,
        });
      }
    );
    unlisteners.push(unlistenDeviceRestored);

    // Error events → show toast notification
    const unlistenError = onError((event: IpcErrorEvent) => {
      addToast({
        title: "Error",
        description: event.message,
        variant: "destructive",
      });
    });
    unlisteners.push(unlistenError);

    // Generic device update → update device in store
    const unlistenDeviceUpdate = onDeviceUpdate(
      (event: DeviceUpdateEvent) => {
        const { type, device } = event;
        if (type === "device_found") {
          addDevice(device);
        } else if (type === "device_lost") {
          removeDevice(device.ip);
        } else if (type === "device_updated") {
          updateDevice(device.ip, device);
        }
      }
    );
    unlisteners.push(unlistenDeviceUpdate);

    // Device found event (legacy)
    const unlistenDeviceFound = onDeviceFound((event: DeviceFoundEvent) => {
      addDevice(event.device);
    });
    unlisteners.push(unlistenDeviceFound);

    // Device lost event (legacy)
    const unlistenDeviceLost = onDeviceLost((event: DeviceLostEvent) => {
      removeDevice(event.device.ip);
    });
    unlisteners.push(unlistenDeviceLost);

    // Scan completed
    const unlistenScanCompleted = onScanCompleted(
      (event: ScanCompletedEvent) => {
        setScanning(false);
        if (event.success) {
          addToast({
            title: "Scan Complete",
            description: `Found ${event.total_devices} device(s) on the network`,
          });
        } else {
          addToast({
            title: "Scan Failed",
            description: "The network scan did not complete successfully",
            variant: "destructive",
          });
        }
      }
    );
    unlisteners.push(unlistenScanCompleted);

    // ARP spoof detected (defender)
    const unlistenArpSpoof = onArpSpoofDetected(
      (event: ArpSpoofDetectedEvent) => {
        addToast({
          title: "ARP Spoofing Detected",
          description: `Suspicious activity: ${event.attacker_mac} claiming ${event.claimed_ip}`,
          variant: "destructive",
        });
      }
    );
    unlisteners.push(unlistenArpSpoof);

    // Cleanup all listeners on unmount
    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [setKillState, addDevice, updateDevice, removeDevice, setScanning, addToast]);
}
