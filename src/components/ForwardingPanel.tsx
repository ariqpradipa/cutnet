"use client";

import { useState, useEffect, useCallback } from "react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertTitle, AlertDescription } from "@/components/ui/alert";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  ArrowLeftRight,
  AlertTriangle,
  Shield,
  ShieldOff,
  Activity,
  Trash2,
  Play,
  Square,
  RefreshCw,
} from "lucide-react";
import { useNetworkStore } from "@/stores/networkStore";
import { invoke } from "@tauri-apps/api/core";
import type { Device } from "@/lib/schemas";

interface ForwardingStats {
  packets_forwarded: number;
  bytes_forwarded: number;
  packets_dropped: number;
  bytes_dropped: number;
  packets_modified: number;
  active_connections: number;
}

interface ForwardingRule {
  id: string;
  protocol: "TCP" | "UDP" | "ICMP" | "All";
  port: number | null;
  action: "Allow" | "Block" | "Log" | "Modify";
  description?: string;
}

export function ForwardingPanel() {
  const { devices, activeInterface } = useNetworkStore();
  const [selectedVictim, setSelectedVictim] = useState<string>("");
  const [routerMac, setRouterMac] = useState<string>("");
  const [isForwardingEnabled, setIsForwardingEnabled] = useState<boolean>(false);
  const [isSystemForwardingEnabled, setIsSystemForwardingEnabled] = useState<boolean>(false);
  const [stats, setStats] = useState<ForwardingStats>({
    packets_forwarded: 0,
    bytes_forwarded: 0,
    packets_dropped: 0,
    bytes_dropped: 0,
    packets_modified: 0,
    active_connections: 0,
  });
  const [rules, setRules] = useState<ForwardingRule[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const victimDevice = devices.find((d: Device) => d.mac === selectedVictim);

  const loadSystemForwardingStatus = useCallback(async () => {
    try {
      const enabled = await invoke<boolean>("is_system_ip_forwarding_enabled");
      setIsSystemForwardingEnabled(enabled);
    } catch (err) {
      console.error("Failed to check system forwarding status:", err);
    }
  }, []);

  const loadForwardingStatus = useCallback(async () => {
    if (!selectedVictim || !routerMac || !activeInterface) return;

    try {
      const active = await invoke<boolean>("is_forwarding_active", {
        victimMac: selectedVictim,
        routerMac,
        interfaceName: activeInterface.name,
      });
      setIsForwardingEnabled(active);
    } catch (err) {
      console.error("Failed to check forwarding status:", err);
    }
  }, [selectedVictim, routerMac, activeInterface]);

  const loadStats = useCallback(async () => {
    if (!selectedVictim || !routerMac || !activeInterface) return;

    try {
      const forwardingStats = await invoke<ForwardingStats>("get_forwarding_stats", {
        victimMac: selectedVictim,
        routerMac,
        interfaceName: activeInterface.name,
      });
      setStats(forwardingStats);
    } catch (err) {
      console.error("Failed to load forwarding stats:", err);
    }
  }, [selectedVictim, routerMac, activeInterface]);

  const loadRules = useCallback(async () => {
    if (!selectedVictim || !routerMac || !activeInterface) return;

    try {
      const forwardingRules = await invoke<ForwardingRule[]>("get_forwarding_rules", {
        victimMac: selectedVictim,
        routerMac,
        interfaceName: activeInterface.name,
      });
      setRules(forwardingRules);
    } catch (err) {
      console.error("Failed to load forwarding rules:", err);
    }
  }, [selectedVictim, routerMac, activeInterface]);

  useEffect(() => {
    loadSystemForwardingStatus();
    const interval = setInterval(loadSystemForwardingStatus, 5000);
    return () => clearInterval(interval);
  }, [loadSystemForwardingStatus]);

  useEffect(() => {
    if (selectedVictim) {
      loadForwardingStatus();
      const interval = setInterval(() => {
        loadForwardingStatus();
        loadStats();
      }, 1000);
      return () => clearInterval(interval);
    }
  }, [selectedVictim, loadForwardingStatus, loadStats]);

  useEffect(() => {
    if (isForwardingEnabled) {
      loadRules();
    }
  }, [isForwardingEnabled, loadRules]);

  const handleEnableSystemForwarding = async () => {
    setIsLoading(true);
    setError(null);
    try {
      await invoke("enable_system_ip_forwarding");
      setIsSystemForwardingEnabled(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to enable system IP forwarding");
    } finally {
      setIsLoading(false);
    }
  };

  const handleDisableSystemForwarding = async () => {
    setIsLoading(true);
    setError(null);
    try {
      await invoke("disable_system_ip_forwarding");
      setIsSystemForwardingEnabled(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to disable system IP forwarding");
    } finally {
      setIsLoading(false);
    }
  };

  const handleToggleForwarding = async () => {
    if (!selectedVictim || !routerMac || !activeInterface) {
      setError("Please select a victim device and ensure router is configured");
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      if (isForwardingEnabled) {
        await invoke("disable_forwarding", {
          victimMac: selectedVictim,
          routerMac,
          interfaceName: activeInterface.name,
        });
        setIsForwardingEnabled(false);
      } else {
        await invoke("enable_forwarding", {
          victimMac: selectedVictim,
          routerMac,
          interfaceName: activeInterface.name,
        });
        setIsForwardingEnabled(true);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to toggle forwarding");
    } finally {
      setIsLoading(false);
    }
  };

  const handleAddRule = async (protocol: "TCP" | "UDP", port: number, action: "Block" | "Allow") => {
    if (!selectedVictim || !routerMac || !activeInterface) return;

    try {
      await invoke("add_forwarding_rule", {
        victimMac: selectedVictim,
        routerMac,
        interfaceName: activeInterface.name,
        rule: {
          protocol,
          port,
          action,
        },
      });
      await loadRules();
    } catch (err) {
      console.error("Failed to add rule:", err);
    }
  };

  const handleRemoveRule = async (ruleId: string) => {
    if (!selectedVictim || !routerMac || !activeInterface) return;

    try {
      await invoke("remove_forwarding_rule", {
        victimMac: selectedVictim,
        routerMac,
        interfaceName: activeInterface.name,
        ruleId,
      });
      await loadRules();
    } catch (err) {
      console.error("Failed to remove rule:", err);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ArrowLeftRight className="size-4" />
          MITM Packet Forwarding
        </CardTitle>
        <CardDescription>
          Configure transparent packet forwarding for MITM operations
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <Alert variant="destructive">
          <AlertTriangle className="size-4" />
          <AlertTitle>Legal Notice</AlertTitle>
          <AlertDescription>
            Packet forwarding and interception may violate privacy laws. Only use on networks you
            own or have explicit permission to monitor. No packet content is logged.
          </AlertDescription>
        </Alert>

        <div className="space-y-4">
          <div className="flex items-center justify-between rounded-lg bg-muted p-4">
            <div className="flex items-center gap-3">
              {isSystemForwardingEnabled ? (
                <Shield className="size-8 text-emerald-500" />
              ) : (
                <ShieldOff className="size-8 text-muted-foreground" />
              )}
              <div>
                <h4 className="text-sm font-medium">System IP Forwarding</h4>
                <p className="text-xs text-muted-foreground">
                  {isSystemForwardingEnabled
                    ? "IP forwarding enabled at system level"
                    : "IP forwarding disabled - victim traffic will be blocked"}
                </p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant={isSystemForwardingEnabled ? "default" : "secondary"}>
                {isSystemForwardingEnabled ? "Enabled" : "Disabled"}
              </Badge>
              {isSystemForwardingEnabled ? (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleDisableSystemForwarding}
                  disabled={isLoading}
                >
                  <Square className="size-3.5" />
                  Disable
                </Button>
              ) : (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleEnableSystemForwarding}
                  disabled={isLoading}
                >
                  <Play className="size-3.5" />
                  Enable
                </Button>
              )}
            </div>
          </div>

          <Separator />

          <div className="space-y-2">
            <Label htmlFor="victim-select">Target Device (Victim)</Label>
            <Select value={selectedVictim} onValueChange={setSelectedVictim}>
              <SelectTrigger id="victim-select">
                <SelectValue placeholder="Select a device to forward" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>Connected Devices</SelectLabel>
                  {devices
                    .filter((d: Device) => !d.is_router && !d.is_me)
                    .map((device: Device) => (
                      <SelectItem key={device.mac} value={device.mac}>
                        {device.hostname || device.custom_name || device.ip} ({device.mac})
                      </SelectItem>
                    ))}
                  {devices.filter((d) => !d.is_router && !d.is_me).length === 0 && (
                    <SelectItem value="none" disabled>
                      No devices available
                    </SelectItem>
                  )}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="router-mac">Router MAC Address</Label>
            <Select value={routerMac} onValueChange={setRouterMac}>
              <SelectTrigger id="router-mac">
                <SelectValue placeholder="Select router" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>Router</SelectLabel>
                  {devices
                    .filter((d: Device) => d.is_router)
                    .map((device: Device) => (
                      <SelectItem key={device.mac} value={device.mac}>
                        {device.ip} ({device.mac})
                      </SelectItem>
                    ))}
                  {activeInterface && (
                    <SelectItem value={activeInterface.mac}>
                      Default Gateway ({activeInterface.mac})
                    </SelectItem>
                  )}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>

          {selectedVictim && (
            <>
              <div className="flex items-center justify-between rounded-lg bg-muted p-4">
                <div className="flex items-center gap-3">
                  {isForwardingEnabled ? (
                    <Activity className="size-8 text-emerald-500" />
                  ) : (
                    <ShieldOff className="size-8 text-muted-foreground" />
                  )}
                  <div>
                    <h4 className="text-sm font-medium">Packet Forwarding</h4>
                    <p className="text-xs text-muted-foreground">
                      {victimDevice
                        ? `Forwarding for ${victimDevice.ip}`
                        : "Select a victim device"}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Badge variant={isForwardingEnabled ? "default" : "secondary"}>
                    {isForwardingEnabled ? "Active" : "Inactive"}
                  </Badge>
                  <Switch
                    checked={isForwardingEnabled}
                    onCheckedChange={handleToggleForwarding}
                    disabled={!selectedVictim || isLoading}
                  />
                </div>
              </div>

              {isForwardingEnabled && (
                <>
                  <Separator />

                  <div className="grid grid-cols-3 gap-4">
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-2xl font-bold">{stats.packets_forwarded}</p>
                      <p className="text-xs text-muted-foreground">Packets Forwarded</p>
                    </div>
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-2xl font-bold">{stats.packets_dropped}</p>
                      <p className="text-xs text-muted-foreground">Packets Dropped</p>
                    </div>
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-2xl font-bold">{stats.active_connections}</p>
                      <p className="text-xs text-muted-foreground">Active Connections</p>
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-4">
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-lg font-bold">{formatBytes(stats.bytes_forwarded)}</p>
                      <p className="text-xs text-muted-foreground">Bytes Forwarded</p>
                    </div>
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-lg font-bold">{formatBytes(stats.bytes_dropped)}</p>
                      <p className="text-xs text-muted-foreground">Bytes Dropped</p>
                    </div>
                  </div>

                  <Separator />

                  <div className="space-y-2">
                    <div className="flex items-center justify-between">
                      <h4 className="text-sm font-medium">Forwarding Rules</h4>
                      <Badge variant="outline">{rules.length} rules</Badge>
                    </div>
                    <ScrollArea className="h-32 rounded-md border">
                      {rules.length > 0 ? (
                        <div className="p-2 space-y-2">
                          {rules.map((rule) => (
                            <div
                              key={rule.id}
                              className="flex items-center justify-between rounded-md bg-muted p-2"
                            >
                              <div className="flex items-center gap-2">
                                <Badge
                                  variant={rule.action === "Block" ? "destructive" : "default"}
                                >
                                  {rule.action}
                                </Badge>
                                <span className="text-xs">
                                  {rule.protocol}
                                  {rule.port && `:${rule.port}`}
                                </span>
                              </div>
                              <Button
                                variant="ghost"
                                size="icon-xs"
                                onClick={() => handleRemoveRule(rule.id)}
                              >
                                <Trash2 className="size-3 text-destructive" />
                              </Button>
                            </div>
                          ))}
                        </div>
                      ) : (
                        <div className="flex items-center justify-center h-full">
                          <p className="text-xs text-muted-foreground">No rules configured</p>
                        </div>
                      )}
                    </ScrollArea>
                  </div>

                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleAddRule("TCP", 80, "Block")}
                      className="flex-1"
                    >
                      Block HTTP (80)
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleAddRule("TCP", 443, "Block")}
                      className="flex-1"
                    >
                      Block HTTPS (443)
                    </Button>
                  </div>

                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => loadStats()}
                    className="w-full"
                  >
                    <RefreshCw className="size-3.5" />
                    Refresh Stats
                  </Button>
                </>
              )}
            </>
          )}

          {error && (
            <Alert variant="destructive">
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
        </div>
      </CardContent>
    </Card>
  );
}