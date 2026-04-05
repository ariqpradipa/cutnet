"use client";

import { useState, useCallback, useMemo, useEffect } from "react";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Alert, AlertTitle, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Network,
  Shield,
  List,
  Info,
  Wifi,
  AlertTriangle,
  Trash2,
  Plus,
  RotateCcw,
  Copy,
  Check,
  ShieldAlert,
  ShieldCheck,
  ExternalLink,
  RefreshCw,
  ShieldOff,
} from "lucide-react";
import { useNetworkStore } from "@/stores/networkStore";
import {
  setMacAddress as ipcSetMacAddress,
  flushArpCache as ipcFlushArpCache,
  startDefender as ipcStartDefender,
  stopDefender as ipcStopDefender,
  getDefenderAlerts as ipcGetDefenderAlerts,
  isDefenderActive as ipcIsDefenderActive,
  addWhitelistEntry as ipcAddWhitelistEntry,
  removeWhitelistEntry as ipcRemoveWhitelistEntry,
  getWhitelistEntries as ipcGetWhitelistEntries,
  setWhitelistProtect as ipcSetWhitelistProtect,
  onArpSpoofDetected,
} from "@/utils/ipc";

const changeMac = async (
  interfaceName: string,
  newMac: string
): Promise<{ success: boolean; error?: string }> => {
  try {
    await ipcSetMacAddress(interfaceName, newMac);
    return { success: true };
  } catch (err) {
    return { success: false, error: err instanceof Error ? err.message : "Failed to change MAC address" };
  }
};

// MAC address validation regex
const MAC_REGEX = /^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$/;

interface AlertLogEntry {
  id: string;
  timestamp: Date;
  attackerMac: string;
  attackerIp: string;
  type: string;
}

interface WhitelistEntry {
  id: string;
  mac: string;
  label?: string;
}

interface SettingsPanelProps {
  className?: string;
  defaultTab?: "network" | "mac" | "defender" | "whitelist" | "about";
}

export function SettingsPanel({ className, defaultTab = "network" }: SettingsPanelProps) {
  const { interfaces, activeInterface, setActiveInterface } = useNetworkStore();

  // Network tab state
  const [selectedInterface, setSelectedInterface] = useState<string>(
    activeInterface?.name ?? ""
  );
  const [refreshInterval, setRefreshInterval] = useState<number>(30);
  const [ipForwarding, setIpForwarding] = useState<boolean>(false);
  const [isFlushingArp, setIsFlushingArp] = useState<boolean>(false);

  // MAC Address tab state
  const [currentMac, setCurrentMac] = useState<string>(
    activeInterface?.mac ?? "00:00:00:00:00:00"
  );
  const [newMac, setNewMac] = useState<string>("");
  const [originalMac, setOriginalMac] = useState<string>(
    activeInterface?.mac ?? ""
  );
  const [cloneSourceInterface, setCloneSourceInterface] = useState<string>("");
  const [isChangingMac, setIsChangingMac] = useState<boolean>(false);
  const [macError, setMacError] = useState<string | null>(null);

  // Defender tab state
  const [defenderEnabled, setDefenderEnabled] = useState<boolean>(false);
  const [defenderStatus, setDefenderStatus] = useState<"active" | "inactive">(
    "inactive"
  );
  const [alertNotifications, setAlertNotifications] = useState<boolean>(true);
  const [alertLog, setAlertLog] = useState<AlertLogEntry[]>([]);

  // Load defender state on mount
  useEffect(() => {
    const loadDefenderState = async () => {
      try {
        const active = await ipcIsDefenderActive();
        setDefenderEnabled(active);
        setDefenderStatus(active ? "active" : "inactive");
        
        const alerts = await ipcGetDefenderAlerts();
        setAlertLog(alerts.map((a: any) => ({
          id: a.timestamp.toString(),
          timestamp: new Date(a.timestamp * 1000),
          attackerMac: a.attacker_mac,
          attackerIp: a.claimed_ip,
          type: a.alert_type,
        })));
      } catch (err) {
        console.error("Failed to load defender state:", err);
      }
    };
    loadDefenderState();
  }, []);

  // Listen to real-time ARP spoof detection events
  useEffect(() => {
    const unlisten = onArpSpoofDetected((event) => {
      setAlertLog((prev) => [
        ...prev,
        {
          id: event.timestamp.toString() + Math.random().toString(36).slice(2),
          timestamp: new Date(event.timestamp * 1000),
          attackerMac: event.attacker_mac,
          attackerIp: event.claimed_ip,
          type: event.alert_type,
        },
      ]);
    });

    return () => {
      unlisten();
    };
  }, []);

  // Sync currentMac and selectedInterface when activeInterface changes
  useEffect(() => {
    if (activeInterface) {
      setCurrentMac(activeInterface.mac);
      setSelectedInterface(activeInterface.name);
      if (!originalMac) {
        setOriginalMac(activeInterface.mac);
      }
    }
  }, [activeInterface, originalMac]);

  // Whitelist tab state
  const [whitelist, setWhitelist] = useState<WhitelistEntry[]>([]);
  const [newWhitelistMac, setNewWhitelistMac] = useState<string>("");
  const [newWhitelistLabel, setNewWhitelistLabel] = useState<string>("");
  const [protectWhitelisted, setProtectWhitelisted] = useState<boolean>(true);
  const [whitelistError, setWhitelistError] = useState<string | null>(null);

  // Load whitelist on mount
  useEffect(() => {
    const loadWhitelist = async () => {
      try {
        const entries = await ipcGetWhitelistEntries();
        setWhitelist(entries.map((e: any) => ({
          id: e.mac,
          mac: e.mac,
          label: e.label,
        })));
      } catch (err) {
        console.error("Failed to load whitelist:", err);
      }
    };
    loadWhitelist();
  }, []);

  // About tab state
  const [isCheckingUpdates, setIsCheckingUpdates] = useState<boolean>(false);
  const [updateStatus, setUpdateStatus] = useState<
    null | { available: boolean; version?: string }
  >(null);

  // Get currently selected interface details
  const selectedInterfaceDetails = useMemo(() => {
    return interfaces.find((iface) => iface.name === selectedInterface);
  }, [interfaces, selectedInterface]);

  // Validate MAC address
  const validateMac = useCallback((mac: string): boolean => {
    return MAC_REGEX.test(mac);
  }, []);

  // Handle interface selection change
  const handleInterfaceChange = useCallback(
    (value: string) => {
      setSelectedInterface(value);
      const iface = interfaces.find((i) => i.name === value);
      if (iface) {
        setActiveInterface(iface);
        setCurrentMac(iface.mac);
        if (!originalMac) {
          setOriginalMac(iface.mac);
        }
      }
    },
    [interfaces, setActiveInterface, originalMac]
  );

  // Handle MAC address change
  const handleChangeMac = useCallback(async () => {
    if (!validateMac(newMac)) {
      setMacError("Invalid MAC address format. Use XX:XX:XX:XX:XX:XX");
      return;
    }

    setIsChangingMac(true);
    setMacError(null);

    try {
      const result = await changeMac(selectedInterface, newMac);
      if (result.success) {
        setCurrentMac(newMac);
        setNewMac("");
      } else {
        setMacError(result.error ?? "Failed to change MAC address");
      }
    } catch (err) {
      setMacError(
        err instanceof Error ? err.message : "An unexpected error occurred"
      );
    } finally {
      setIsChangingMac(false);
    }
  }, [newMac, selectedInterface, validateMac]);

  // Handle restore original MAC
  const handleRestoreMac = useCallback(async () => {
    if (!originalMac) return;

    setIsChangingMac(true);
    setMacError(null);

    try {
      const result = await changeMac(selectedInterface, originalMac);
      if (result.success) {
        setCurrentMac(originalMac);
        setNewMac("");
      } else {
        setMacError(result.error ?? "Failed to restore MAC address");
      }
    } catch (err) {
      setMacError(
        err instanceof Error ? err.message : "An unexpected error occurred"
      );
    } finally {
      setIsChangingMac(false);
    }
  }, [originalMac, selectedInterface]);

  // Handle clone MAC from interface
  const handleCloneMac = useCallback(async () => {
    if (!cloneSourceInterface) return;

    const sourceIface = interfaces.find((i) => i.name === cloneSourceInterface);
    if (!sourceIface) return;

    setNewMac(sourceIface.mac);
    setMacError(null);
  }, [cloneSourceInterface, interfaces]);

  // Handle flush ARP cache
  const handleFlushArp = useCallback(async () => {
    setIsFlushingArp(true);
    try {
      await ipcFlushArpCache();
    } catch (err) {
      console.error("Error flushing ARP cache:", err);
    } finally {
      setIsFlushingArp(false);
    }
  }, []);

  // Handle defender toggle
  const handleDefenderToggle = useCallback(async (enabled: boolean) => {
    try {
      if (enabled) {
        await ipcStartDefender();
      } else {
        await ipcStopDefender();
      }
      setDefenderEnabled(enabled);
      setDefenderStatus(enabled ? "active" : "inactive");
      
      const alerts = await ipcGetDefenderAlerts();
      setAlertLog(alerts.map((a: any) => ({
        id: a.timestamp.toString(),
        timestamp: new Date(a.timestamp * 1000),
        attackerMac: a.attacker_mac,
        attackerIp: a.claimed_ip,
        type: a.alert_type,
      })));
    } catch (err) {
      console.error("Error toggling defender:", err);
    }
  }, []);

  // Handle add to whitelist
  const handleAddToWhitelist = useCallback(async () => {
    if (!validateMac(newWhitelistMac)) {
      setWhitelistError("Invalid MAC address format");
      return;
    }

    try {
      await ipcAddWhitelistEntry(newWhitelistMac, newWhitelistLabel || undefined);
      const entries = await ipcGetWhitelistEntries();
      setWhitelist(entries.map((e: any) => ({
        id: e.mac,
        mac: e.mac,
        label: e.label,
      })));
      setNewWhitelistMac("");
      setNewWhitelistLabel("");
      setWhitelistError(null);
    } catch (err) {
      setWhitelistError(err instanceof Error ? err.message : "Failed to add to whitelist");
    }
  }, [newWhitelistMac, newWhitelistLabel, validateMac]);

  // Handle remove from whitelist
  const handleRemoveFromWhitelist = useCallback(async (id: string) => {
    try {
      await ipcRemoveWhitelistEntry(id);
      const entries = await ipcGetWhitelistEntries();
      setWhitelist(entries.map((e: any) => ({
        id: e.mac,
        mac: e.mac,
        label: e.label,
      })));
    } catch (err) {
      console.error("Failed to remove from whitelist:", err);
    }
  }, []);

  // Handle protect whitelisted toggle
  const handleProtectWhitelistedChange = useCallback(async (enabled: boolean) => {
    try {
      await ipcSetWhitelistProtect(enabled);
      setProtectWhitelisted(enabled);
    } catch (err) {
      console.error("Failed to set whitelist protect:", err);
    }
  }, []);

  // Handle check for updates
  const handleCheckUpdates = useCallback(async () => {
    setIsCheckingUpdates(true);
    setUpdateStatus(null);

    // Simulate API call
    await new Promise((resolve) => setTimeout(resolve, 1500));

    setUpdateStatus({ available: false });
    setIsCheckingUpdates(false);
  }, []);

  // Format timestamp for alert log
  const formatTimestamp = useCallback((date: Date): string => {
    return date.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }, []);

  return (
    <Card className={className}>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Network className="size-4" />
          Settings
        </CardTitle>
        <CardDescription>
          Configure network settings, MAC address, ARP defender, and whitelist
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue={defaultTab} className="w-full">
          <TabsList className="grid w-full grid-cols-5">
            <TabsTrigger value="network" className="flex items-center gap-1.5">
              <Wifi className="size-3.5" />
              <span className="hidden sm:inline">Network</span>
            </TabsTrigger>
            <TabsTrigger value="mac" className="flex items-center gap-1.5">
              <RotateCcw className="size-3.5" />
              <span className="hidden sm:inline">MAC Address</span>
            </TabsTrigger>
            <TabsTrigger value="defender" className="flex items-center gap-1.5">
              <Shield className="size-3.5" />
              <span className="hidden sm:inline">Defender</span>
            </TabsTrigger>
            <TabsTrigger value="whitelist" className="flex items-center gap-1.5">
              <List className="size-3.5" />
              <span className="hidden sm:inline">Whitelist</span>
            </TabsTrigger>
            <TabsTrigger value="about" className="flex items-center gap-1.5">
              <Info className="size-3.5" />
              <span className="hidden sm:inline">About</span>
            </TabsTrigger>
          </TabsList>

          {/* Network Tab */}
          <TabsContent value="network" className="space-y-4 mt-4">
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="interface-select">Network Interface</Label>
                <Select
                  value={selectedInterface}
                  onValueChange={handleInterfaceChange}
                >
                  <SelectTrigger id="interface-select" className="w-full">
                    <SelectValue placeholder="Select interface" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      <SelectLabel>Available Interfaces</SelectLabel>
                      {interfaces.map((iface) => (
                        <SelectItem key={iface.name} value={iface.name}>
                          {iface.name} ({iface.ip})
                        </SelectItem>
                      ))}
                      {interfaces.length === 0 && (
                        <SelectItem value="none" disabled>
                          No interfaces found
                        </SelectItem>
                      )}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </div>

              {selectedInterfaceDetails && (
                <div className="rounded-lg bg-muted p-4 space-y-2">
                  <h4 className="text-sm font-medium">Interface Details</h4>
                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div>
                      <span className="text-muted-foreground">IP Address:</span>
                      <p className="font-mono">{selectedInterfaceDetails.ip}</p>
                    </div>
                    <div>
                      <span className="text-muted-foreground">MAC Address:</span>
                      <p className="font-mono">{selectedInterfaceDetails.mac}</p>
                    </div>
                    <div>
                      <span className="text-muted-foreground">Netmask:</span>
                      <p className="font-mono">{selectedInterfaceDetails.netmask}</p>
                    </div>
                    <div>
                      <span className="text-muted-foreground">Broadcast:</span>
                      <p className="font-mono">
                        {selectedInterfaceDetails.broadcast_addr}
                      </p>
                    </div>
                  </div>
                </div>
              )}

              <Separator />

              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label htmlFor="refresh-interval">Refresh Interval</Label>
                    <p className="text-xs text-muted-foreground">
                      Device list refresh interval in seconds
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Input
                      id="refresh-interval"
                      type="number"
                      min={10}
                      max={300}
                      value={refreshInterval}
                      onChange={(e) =>
                        setRefreshInterval(parseInt(e.target.value) || 30)
                      }
                      className="w-20"
                    />
                    <span className="text-xs text-muted-foreground">sec</span>
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label htmlFor="ip-forwarding">IP Forwarding</Label>
                    <p className="text-xs text-muted-foreground">
                      Enable IP packet forwarding
                    </p>
                  </div>
                  <Switch
                    id="ip-forwarding"
                    checked={ipForwarding}
                    onCheckedChange={setIpForwarding}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label>ARP Cache</Label>
                    <p className="text-xs text-muted-foreground">
                      Clear the ARP cache
                    </p>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleFlushArp}
                    disabled={isFlushingArp}
                  >
                    {isFlushingArp ? (
                      <RefreshCw className="size-3.5 animate-spin" />
                    ) : (
                      <Trash2 className="size-3.5" />
                    )}
                    Flush Cache
                  </Button>
                </div>
              </div>
            </div>
          </TabsContent>

          {/* MAC Address Tab */}
          <TabsContent value="mac" className="space-y-4 mt-4">
            <Alert variant="destructive">
              <AlertTriangle className="size-3.5" />
              <AlertTitle>Administrator privileges required</AlertTitle>
              <AlertDescription>
                Changing MAC address requires admin/root privileges. This may
                temporarily disconnect your network.
              </AlertDescription>
            </Alert>

            <div className="space-y-4">
              <div className="rounded-lg bg-muted p-4">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs text-muted-foreground">
                      Current MAC Address
                    </p>
                    <p className="font-mono text-sm font-medium">{currentMac}</p>
                  </div>
                  <Badge variant="secondary">
                    {selectedInterface || "No interface"}
                  </Badge>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="new-mac">New MAC Address</Label>
                <div className="flex gap-2">
                  <Input
                    id="new-mac"
                    placeholder="XX:XX:XX:XX:XX:XX"
                    value={newMac}
                    onChange={(e) => {
                      setNewMac(e.target.value);
                      setMacError(null);
                    }}
                    className={macError ? "border-destructive" : ""}
                  />
                  <Button
                    onClick={handleChangeMac}
                    disabled={!newMac || isChangingMac}
                  >
                    {isChangingMac ? (
                      <RefreshCw className="size-3.5 animate-spin" />
                    ) : (
                      <Check className="size-3.5" />
                    )}
                    Change
                  </Button>
                </div>
                {macError && (
                  <p className="text-xs text-destructive">{macError}</p>
                )}
                <p className="text-xs text-muted-foreground">
                  Format: XX:XX:XX:XX:XX:XX (hexadecimal)
                </p>
              </div>

              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={handleRestoreMac}
                  disabled={!originalMac || isChangingMac}
                  className="flex-1"
                >
                  <RotateCcw className="size-3.5" />
                  Restore Original
                </Button>
                <Button
                  variant="outline"
                  onClick={() => setNewMac("")}
                  disabled={!newMac}
                  className="flex-1"
                >
                  <Trash2 className="size-3.5" />
                  Clear
                </Button>
              </div>

              <Separator />

              <div className="space-y-2">
                <Label>Clone from Interface</Label>
                <div className="flex gap-2">
                  <Select
                    value={cloneSourceInterface}
                    onValueChange={setCloneSourceInterface}
                  >
                    <SelectTrigger className="flex-1">
                      <SelectValue placeholder="Select source interface" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectGroup>
                        <SelectLabel>Interfaces</SelectLabel>
                        {interfaces
                          .filter((iface) => iface.name !== selectedInterface)
                          .map((iface) => (
                            <SelectItem key={iface.name} value={iface.name}>
                              {iface.name} ({iface.mac})
                            </SelectItem>
                          ))}
                      </SelectGroup>
                    </SelectContent>
                  </Select>
                  <Button
                    variant="secondary"
                    onClick={handleCloneMac}
                    disabled={!cloneSourceInterface}
                  >
                    <Copy className="size-3.5" />
                    Clone
                  </Button>
                </div>
              </div>
            </div>
          </TabsContent>

          {/* Defender Tab */}
          <TabsContent value="defender" className="space-y-4 mt-4">
            <div className="space-y-4">
              <div className="flex items-center justify-between rounded-lg bg-muted p-4">
                <div className="flex items-center gap-3">
                  {defenderStatus === "active" ? (
                    <ShieldCheck className="size-8 text-emerald-500" />
                  ) : (
                    <ShieldOff className="size-8 text-muted-foreground" />
                  )}
                  <div>
                    <h4 className="text-sm font-medium">ARP Defender</h4>
                    <p className="text-xs text-muted-foreground">
                      {defenderStatus === "active"
                        ? "Protecting against ARP spoofing attacks"
                        : "Protection is currently disabled"}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Badge
                    variant={defenderStatus === "active" ? "default" : "secondary"}
                  >
                    {defenderStatus === "active" ? "Active" : "Inactive"}
                  </Badge>
                  <Switch
                    checked={defenderEnabled}
                    onCheckedChange={handleDefenderToggle}
                  />
                </div>
              </div>

              <Alert>
                <ShieldAlert className="size-3.5" />
                <AlertTitle>Local protection only</AlertTitle>
                <AlertDescription>
                  ARP defender only protects your local machine. Other devices on
                  the network are not protected by this feature.
                </AlertDescription>
              </Alert>

              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label htmlFor="alert-notifications">Alert Notifications</Label>
                  <p className="text-xs text-muted-foreground">
                    Show notifications for detected attacks
                  </p>
                </div>
                <Switch
                  id="alert-notifications"
                  checked={alertNotifications}
                  onCheckedChange={setAlertNotifications}
                />
              </div>

              <Separator />

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <h4 className="text-sm font-medium">Alert Log</h4>
                  <Badge variant="outline">{alertLog.length} entries</Badge>
                </div>
                <ScrollArea className="h-48 rounded-md border">
                  {alertLog.length > 0 ? (
                    <div className="p-2 space-y-2">
                      {alertLog.map((entry) => (
                        <div
                          key={entry.id}
                          className="flex items-start gap-2 rounded-md bg-muted p-2 text-xs"
                        >
                          <ShieldAlert className="size-3.5 text-destructive shrink-0 mt-0.5" />
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2">
                              <span className="font-medium">{entry.type}</span>
                              <span className="text-muted-foreground">
                                {formatTimestamp(entry.timestamp)}
                              </span>
                            </div>
                            <p className="text-muted-foreground truncate">
                              From: {entry.attackerMac} ({entry.attackerIp})
                            </p>
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="flex items-center justify-center h-full">
                      <p className="text-xs text-muted-foreground">
                        No alerts detected
                      </p>
                    </div>
                  )}
                </ScrollArea>
              </div>
            </div>
          </TabsContent>

          {/* Whitelist Tab */}
          <TabsContent value="whitelist" className="space-y-4 mt-4">
            <div className="space-y-4">
              <Alert>
                <Info className="size-3.5" />
                <AlertTitle>Whitelist protection</AlertTitle>
                <AlertDescription>
                  Whitelisted devices are excluded from network scans and will not
                  appear in the device list. Enable &quot;Protect whitelisted&quot;
                  to prevent them from being killed.
                </AlertDescription>
              </Alert>

              <div className="flex items-center justify-between rounded-lg bg-muted p-4">
                <div className="space-y-0.5">
                  <Label htmlFor="protect-whitelisted">
                    Protect whitelisted devices
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Prevent kill operations on whitelisted MAC addresses
                  </p>
                </div>
                <Switch
                  id="protect-whitelisted"
                  checked={protectWhitelisted}
                  onCheckedChange={handleProtectWhitelistedChange}
                />
              </div>

              <Separator />

              <div className="space-y-2">
                <h4 className="text-sm font-medium">Add Device</h4>
                <div className="flex gap-2">
                  <Input
                    placeholder="MAC Address (XX:XX:XX:XX:XX:XX)"
                    value={newWhitelistMac}
                    onChange={(e) => {
                      setNewWhitelistMac(e.target.value);
                      setWhitelistError(null);
                    }}
                    className={whitelistError ? "border-destructive" : ""}
                  />
                  <Input
                    placeholder="Label (optional)"
                    value={newWhitelistLabel}
                    onChange={(e) => setNewWhitelistLabel(e.target.value)}
                    className="w-32"
                  />
                  <Button onClick={handleAddToWhitelist} size="icon">
                    <Plus className="size-3.5" />
                  </Button>
                </div>
                {whitelistError && (
                  <p className="text-xs text-destructive">{whitelistError}</p>
                )}
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <h4 className="text-sm font-medium">Whitelisted Devices</h4>
                  <Badge variant="outline">{whitelist.length} devices</Badge>
                </div>
                <ScrollArea className="h-48 rounded-md border">
                  {whitelist.length > 0 ? (
                    <div className="p-2 space-y-2">
                      {whitelist.map((entry) => (
                        <div
                          key={entry.id}
                          className="flex items-center justify-between rounded-md bg-muted p-2"
                        >
                          <div className="flex items-center gap-2 min-w-0">
                            {protectWhitelisted && (
                              <ShieldCheck className="size-3.5 text-emerald-500 shrink-0" />
                            )}
                            <div className="min-w-0">
                              <p className="text-xs font-mono font-medium truncate">
                                {entry.mac}
                              </p>
                              {entry.label && (
                                <p className="text-xs text-muted-foreground truncate">
                                  {entry.label}
                                </p>
                              )}
                            </div>
                          </div>
                          <Button
                            variant="ghost"
                            size="icon-xs"
                            onClick={() => handleRemoveFromWhitelist(entry.id)}
                          >
                            <Trash2 className="size-3 text-destructive" />
                          </Button>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="flex items-center justify-center h-full">
                      <p className="text-xs text-muted-foreground">
                        No devices in whitelist
                      </p>
                    </div>
                  )}
                </ScrollArea>
              </div>
            </div>
          </TabsContent>

          {/* About Tab */}
          <TabsContent value="about" className="space-y-4 mt-4">
            <div className="space-y-4">
              <div className="flex items-center gap-4">
                <div className="size-16 rounded-lg bg-primary flex items-center justify-center">
                  <Network className="size-8 text-primary-foreground" />
                </div>
                <div>
                  <h3 className="text-lg font-semibold">CutNet</h3>
                  <p className="text-sm text-muted-foreground">
                    Network Administration Tool
                  </p>
                  <div className="flex items-center gap-2 mt-1">
                    <Badge variant="secondary">v0.1.0</Badge>
                    <span className="text-xs text-muted-foreground">
                      Beta Release
                    </span>
                  </div>
                </div>
              </div>

              <Separator />

              <div className="space-y-2">
                <h4 className="text-sm font-medium">Credits</h4>
                <p className="text-xs text-muted-foreground">
                  Built with Tauri, React, and shadcn/ui. Special thanks to the
                  open-source community for making this project possible.
                </p>
              </div>

              <Alert variant="destructive">
                <AlertTriangle className="size-3.5" />
                <AlertTitle>Legal Disclaimer</AlertTitle>
                <AlertDescription>
                  This tool is intended for educational purposes and authorized
                  network administration only. Unauthorized use on networks you
                  do not own or have explicit permission to test is illegal and
                  unethical. The authors assume no liability for misuse.
                </AlertDescription>
              </Alert>

              <div className="space-y-2">
                <h4 className="text-sm font-medium">Resources</h4>
                <div className="flex flex-wrap gap-2">
                  <Button variant="outline" size="sm" asChild>
                    <a
                      href="https://github.com/cutnet/cutnet"
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <ExternalLink className="size-3.5" />
                      Documentation
                    </a>
                  </Button>
                  <Button variant="outline" size="sm" asChild>
                    <a
                      href="https://github.com/cutnet/cutnet/issues"
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <ExternalLink className="size-3.5" />
                      Report Issue
                    </a>
                  </Button>
                </div>
              </div>

              <Separator />

              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>Software Updates</Label>
                  <p className="text-xs text-muted-foreground">
                    Check for the latest version
                  </p>
                </div>
                <Button
                  variant="outline"
                  onClick={handleCheckUpdates}
                  disabled={isCheckingUpdates}
                >
                  {isCheckingUpdates ? (
                    <RefreshCw className="size-3.5 animate-spin" />
                  ) : (
                    <RefreshCw className="size-3.5" />
                  )}
                  Check for Updates
                </Button>
              </div>

              {updateStatus && (
                <Alert
                  variant={updateStatus.available ? "default" : undefined}
                  className={!updateStatus.available ? "bg-muted" : undefined}
                >
                  {updateStatus.available ? (
                    <>
                      <Check className="size-3.5" />
                      <AlertTitle>Update available</AlertTitle>
                      <AlertDescription>
                        Version {updateStatus.version} is now available for
                        download.
                      </AlertDescription>
                    </>
                  ) : (
                    <>
                      <Check className="size-3.5" />
                      <AlertTitle>Up to date</AlertTitle>
                      <AlertDescription>
                        You are running the latest version of CutNet.
                      </AlertDescription>
                    </>
                  )}
                </Alert>
              )}
            </div>
          </TabsContent>
        </Tabs>
      </CardContent>
    </Card>
  );
}
