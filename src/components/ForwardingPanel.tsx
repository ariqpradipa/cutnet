"use client";

import { useState, useEffect, useCallback, useRef } from "react";
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
  Activity,
  ShieldOff,
  Trash2,
  RefreshCw,
  Loader2,
  Info,
} from "lucide-react";
import { useNetworkStore } from "@/stores/networkStore";
import { useDeviceStore } from "@/stores/deviceStore";
import {
  startForwarding,
  stopForwarding,
  isForwardingActive,
  getForwardingStats,
  getForwardingRules,
  addForwardingRule,
  removeForwardingRule,
} from "@/utils/ipc";
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
  const { activeInterface } = useNetworkStore();
  const devices = useDeviceStore((s) => s.devices);
  const [selectedVictim, setSelectedVictim] = useState<string>("");
  const [routerMac, setRouterMac] = useState<string>("");
  const [isForwardingEnabled, setIsForwardingEnabled] = useState<boolean>(false);
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
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const victimDevice = devices.find((d: Device) => d.mac === selectedVictim);

  // Auto-select router when devices load
  useEffect(() => {
    const router = devices.find((d: Device) => d.is_router);
    if (router && !routerMac) setRouterMac(router.mac);
  }, [devices, routerMac]);

  const loadForwardingStatus = useCallback(async () => {
    if (!selectedVictim || !routerMac || !activeInterface) return;
    try {
      const active = await isForwardingActive(selectedVictim, routerMac, activeInterface.name);
      setIsForwardingEnabled(active);
    } catch { /* ignore */ }
  }, [selectedVictim, routerMac, activeInterface]);

  const loadStats = useCallback(async () => {
    if (!selectedVictim || !routerMac || !activeInterface) return;
    try {
      const s = await getForwardingStats(selectedVictim, routerMac, activeInterface.name);
      setStats(s as ForwardingStats);
    } catch { /* ignore — session may not be active */ }
  }, [selectedVictim, routerMac, activeInterface]);

  const loadRules = useCallback(async () => {
    if (!selectedVictim || !routerMac || !activeInterface) return;
    try {
      const r = await getForwardingRules(selectedVictim, routerMac, activeInterface.name);
      setRules(r as ForwardingRule[]);
    } catch { /* ignore */ }
  }, [selectedVictim, routerMac, activeInterface]);

  // Single poll interval for status + stats — only when a victim is selected
  useEffect(() => {
    if (pollRef.current) clearInterval(pollRef.current);

    if (!selectedVictim) return;

    loadForwardingStatus();

    pollRef.current = setInterval(() => {
      loadForwardingStatus();
      if (isForwardingEnabled) loadStats();
    }, 2000);

    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [selectedVictim, routerMac, activeInterface, isForwardingEnabled, loadForwardingStatus, loadStats]);

  useEffect(() => {
    if (isForwardingEnabled) loadRules();
  }, [isForwardingEnabled, loadRules]);

  const handleToggleForwarding = async () => {
    if (!selectedVictim || !routerMac || !activeInterface) {
      setError("Please select a victim device and router first");
      return;
    }
    setIsLoading(true);
    setError(null);
    try {
      if (isForwardingEnabled) {
        await stopForwarding(selectedVictim, routerMac, activeInterface.name);
        setIsForwardingEnabled(false);
        setRules([]);
      } else {
        await startForwarding(selectedVictim, routerMac, activeInterface.name);
        setIsForwardingEnabled(true);
        await loadRules();
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
      await addForwardingRule(selectedVictim, routerMac, activeInterface.name, {
        id: crypto.randomUUID(),
        protocol,
        port,
        action,
      } as Parameters<typeof addForwardingRule>[3]);
      await loadRules();
    } catch (err) {
      console.error("Failed to add rule:", err);
    }
  };

  const handleRemoveRule = async (ruleId: string) => {
    if (!selectedVictim || !routerMac || !activeInterface) return;
    try {
      await removeForwardingRule(selectedVictim, routerMac, activeInterface.name, ruleId);
      await loadRules();
    } catch (err) {
      console.error("Failed to remove rule:", err);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${sizes[i]}`;
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ArrowLeftRight className="size-4" />
          MITM Packet Forwarding
        </CardTitle>
        <CardDescription>
          Intercept and forward traffic between a target device and the router
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <Alert variant="destructive">
          <AlertTriangle className="size-4" />
          <AlertTitle>Legal Notice</AlertTitle>
          <AlertDescription>
            Packet forwarding and interception may violate privacy laws. Only use on networks you
            own or have explicit written permission to monitor. No packet content is logged.
          </AlertDescription>
        </Alert>

        <Alert>
          <Info className="size-4" />
          <AlertTitle>System requirement</AlertTitle>
          <AlertDescription>
            IP forwarding must be enabled at the OS level for traffic to flow through your machine.
            On Linux: <code className="text-xs bg-muted px-1 rounded">sysctl -w net.ipv4.ip_forward=1</code>.
            On macOS: <code className="text-xs bg-muted px-1 rounded">sysctl -w net.inet.ip.forwarding=1</code>.
          </AlertDescription>
        </Alert>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="victim-select">Target Device (Victim)</Label>
            <Select value={selectedVictim} onValueChange={setSelectedVictim} disabled={isForwardingEnabled}>
              <SelectTrigger id="victim-select">
                <SelectValue placeholder="Select a device to intercept" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>Connected Devices</SelectLabel>
                  {devices.filter((d: Device) => !d.is_router && !d.is_me).map((device: Device) => (
                    <SelectItem key={device.mac} value={device.mac}>
                      {device.custom_name || device.hostname || device.ip} — {device.mac}
                    </SelectItem>
                  ))}
                  {devices.filter((d) => !d.is_router && !d.is_me).length === 0 && (
                    <SelectItem value="none" disabled>No devices available — run a scan first</SelectItem>
                  )}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="router-mac">Router / Gateway</Label>
            <Select value={routerMac} onValueChange={setRouterMac} disabled={isForwardingEnabled}>
              <SelectTrigger id="router-mac">
                <SelectValue placeholder="Select router" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>Gateway Devices</SelectLabel>
                  {devices.filter((d: Device) => d.is_router).map((device: Device) => (
                    <SelectItem key={device.mac} value={device.mac}>
                      {device.ip} — {device.mac}
                    </SelectItem>
                  ))}
                  {activeInterface && (
                    <SelectItem value={activeInterface.mac}>
                      Local interface ({activeInterface.mac})
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
                  {isForwardingEnabled
                    ? <Activity className="size-8 text-emerald-500" />
                    : <ShieldOff className="size-8 text-muted-foreground" />
                  }
                  <div>
                    <h4 className="text-sm font-medium">Packet Forwarding</h4>
                    <p className="text-xs text-muted-foreground">
                      {victimDevice
                        ? `Target: ${victimDevice.custom_name || victimDevice.hostname || victimDevice.ip}`
                        : "Select a victim device"}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {isLoading && <Loader2 className="size-4 animate-spin text-muted-foreground" />}
                  <Badge variant={isForwardingEnabled ? "default" : "secondary"}>
                    {isForwardingEnabled ? "Active" : "Inactive"}
                  </Badge>
                  <div className={isLoading ? "pointer-events-none opacity-50" : ""}>
                    <Switch
                      checked={isForwardingEnabled}
                      onCheckedChange={handleToggleForwarding}
                      disabled={!selectedVictim || isLoading}
                    />
                  </div>
                </div>
              </div>

              {isForwardingEnabled && (
                <>
                  <Separator />

                  <div className="grid grid-cols-3 gap-4">
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-2xl font-bold tabular-nums">{stats.packets_forwarded.toLocaleString()}</p>
                      <p className="text-xs text-muted-foreground mt-1">Forwarded</p>
                    </div>
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-2xl font-bold tabular-nums">{stats.packets_dropped.toLocaleString()}</p>
                      <p className="text-xs text-muted-foreground mt-1">Dropped</p>
                    </div>
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-2xl font-bold tabular-nums">{stats.active_connections}</p>
                      <p className="text-xs text-muted-foreground mt-1">Connections</p>
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-4">
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-lg font-bold">{formatBytes(stats.bytes_forwarded)}</p>
                      <p className="text-xs text-muted-foreground mt-1">Bytes Forwarded</p>
                    </div>
                    <div className="rounded-lg bg-muted p-3 text-center">
                      <p className="text-lg font-bold">{formatBytes(stats.bytes_dropped)}</p>
                      <p className="text-xs text-muted-foreground mt-1">Bytes Dropped</p>
                    </div>
                  </div>

                  <Separator />

                  <div className="space-y-2">
                    <div className="flex items-center justify-between">
                      <h4 className="text-sm font-medium">Filtering Rules</h4>
                      <Badge variant="outline">{rules.length} rule{rules.length !== 1 ? "s" : ""}</Badge>
                    </div>
                    <ScrollArea className="h-32 rounded-md border">
                      {rules.length > 0 ? (
                        <div className="p-2 space-y-2">
                          {rules.map((rule) => (
                            <div key={rule.id} className="flex items-center justify-between rounded-md bg-muted p-2">
                              <div className="flex items-center gap-2">
                                <Badge variant={rule.action === "Block" ? "destructive" : "default"}>
                                  {rule.action}
                                </Badge>
                                <span className="text-xs font-mono">
                                  {rule.protocol}{rule.port ? `:${rule.port}` : ""}
                                </span>
                                {rule.description && (
                                  <span className="text-xs text-muted-foreground">{rule.description}</span>
                                )}
                              </div>
                              <Button variant="ghost" size="icon-xs" onClick={() => handleRemoveRule(rule.id)}>
                                <Trash2 className="size-3 text-destructive" />
                              </Button>
                            </div>
                          ))}
                        </div>
                      ) : (
                        <div className="flex items-center justify-center h-full">
                          <p className="text-xs text-muted-foreground">No rules — all traffic is forwarded</p>
                        </div>
                      )}
                    </ScrollArea>

                    <div className="flex gap-2 flex-wrap">
                      <Button variant="outline" size="sm" onClick={() => handleAddRule("TCP", 80, "Block")}>
                        Block HTTP (80)
                      </Button>
                      <Button variant="outline" size="sm" onClick={() => handleAddRule("TCP", 443, "Block")}>
                        Block HTTPS (443)
                      </Button>
                      <Button variant="outline" size="sm" onClick={() => handleAddRule("UDP", 53, "Block")}>
                        Block DNS (53)
                      </Button>
                    </div>

                    <Button variant="ghost" size="sm" onClick={loadStats} className="w-full gap-1.5">
                      <RefreshCw className="size-3.5" />
                      Refresh Stats
                    </Button>
                  </div>
                </>
              )}
            </>
          )}

          {error && (
            <Alert variant="destructive">
              <AlertTriangle className="size-4" />
              <AlertTitle>Error</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
