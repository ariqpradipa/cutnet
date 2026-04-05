"use client"

import { useEffect, useState, useCallback } from "react"
import { useNetworkStore, markAppRunning } from "@/stores/networkStore"
import { useDeviceStore } from "@/stores/deviceStore"
import {
  getInterfaces,
  startArpScan,
  startPingScan,
  stopScan,
  onScanProgress,
  onScanCompleted,
  onDeviceFound,
} from "@/utils/ipc"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Checkbox } from "@/components/ui/checkbox"
import { Separator } from "@/components/ui/separator"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import {
  RefreshCw,
  Zap,
  Radar,
  Square,
  Network,
  AlertCircle,
  Loader2,
} from "lucide-react"
import { cn } from "@/lib/utils"

export function ScanControls() {
  // Network store state
  const {
    interfaces,
    activeInterface,
    isScanning,
    scanProgress,
    setInterfaces,
    setActiveInterface,
    setScanning,
    setScanProgress,
  } = useNetworkStore()

  // Device store for device count
  const { devices } = useDeviceStore()

  // Local state
  const [autoRefresh, setAutoRefresh] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [scanType, setScanType] = useState<"arp" | "ping" | null>(null)

  // Load interfaces on mount
  useEffect(() => {
    const loadInterfaces = async () => {
      try {
        const ifaces = await getInterfaces()
        setInterfaces(ifaces)
        // Set first interface as active if none selected
        if (ifaces.length > 0 && !activeInterface) {
          setActiveInterface(ifaces[0])
        }
        if (ifaces.length > 0) {
          markAppRunning();
        }
      } catch (err) {
        console.error("Failed to load interfaces:", err)
        setError("Failed to load network interfaces")
      }
    }

    loadInterfaces()
  }, [setInterfaces, setActiveInterface, activeInterface])

  // Listen to scan progress events
  useEffect(() => {
    const unlisten = onScanProgress((event) => {
      setScanProgress({
        type: event.type,
        progress: event.progress,
        devices_found: event.devicesFound,
      })
    })

    return () => {
      unlisten()
    }
  }, [setScanProgress])

  // Listen to device found events
  useEffect(() => {
    const unlisten = onDeviceFound((event) => {
      useDeviceStore.getState().addDevice(event.device)
    })

    return () => {
      unlisten()
    }
  }, [])

  // Listen to scan completed events
  useEffect(() => {
    const unlisten = onScanCompleted((event) => {
      setScanning(false)
      setScanProgress(null)
      setScanType(null)
    })

    return () => {
      unlisten()
    }
  }, [setScanning, setScanProgress])

  // Auto-refresh interval
  useEffect(() => {
    if (!autoRefresh || !activeInterface || isScanning) return

    const interval = setInterval(async () => {
      if (!isScanning && activeInterface) {
        try {
          setScanning(true)
          setScanType("arp")
          await startArpScan(activeInterface.name)
        } catch (err) {
          console.error("Auto-scan failed:", err)
          setScanning(false)
        }
      }
    }, 30000)

    return () => clearInterval(interval)
  }, [autoRefresh, activeInterface, isScanning, setScanning])

  const handleArpScan = useCallback(async () => {
    if (!activeInterface) {
      setError("Please select a network interface first")
      return
    }

    setError(null)
    useDeviceStore.getState().setDevices([])
    useDeviceStore.getState().clearKillStates()
    setScanType("arp")
    setScanning(true)
    setScanProgress(null)

    try {
      await startArpScan(activeInterface.name)
    } catch (err) {
      console.error("ARP scan failed:", err)
      setError("ARP scan failed. Please check your network connection.")
      setScanning(false)
      setScanType(null)
    }
  }, [activeInterface, setScanning, setScanProgress])

  const handlePingScan = useCallback(async () => {
    if (!activeInterface) {
      setError("Please select a network interface first")
      return
    }

    setError(null)
    useDeviceStore.getState().setDevices([])
    useDeviceStore.getState().clearKillStates()
    setScanType("ping")
    setScanning(true)
    setScanProgress(null)

    try {
      await startPingScan(activeInterface.name)
    } catch (err) {
      console.error("Ping scan failed:", err)
      setError("Ping scan failed. Please check your network connection.")
      setScanning(false)
      setScanType(null)
    }
  }, [activeInterface, setScanning, setScanProgress])

  const handleStopScan = useCallback(async () => {
    try {
      await stopScan()
      setScanning(false)
      setScanProgress(null)
      setScanType(null)
    } catch (err) {
      console.error("Failed to stop scan:", err)
      setError("Failed to stop scan")
    }
  }, [setScanning, setScanProgress])

  const handleInterfaceChange = useCallback(
    (value: string) => {
      const iface = interfaces.find((i) => i.name === value)
      if (iface) {
        setActiveInterface(iface)
        setError(null)
      }
    },
    [interfaces, setActiveInterface]
  )

  // Calculate network range
  const getNetworkRange = useCallback(() => {
    if (!activeInterface) return "-"
    const ipParts = activeInterface.ip.split(".")
    const netmaskParts = activeInterface.netmask.split(".")
    const networkParts = ipParts.map((part, i) =>
      (parseInt(part) & parseInt(netmaskParts[i])).toString()
    )
    // Calculate CIDR
    const cidr = netmaskParts
      .map((part) => (parseInt(part) >>> 0).toString(2).padStart(8, "0"))
      .join("")
      .split("1").length - 1
    return `${networkParts.join(".")}/${cidr}`
  }, [activeInterface])

  // Get gateway info
  const getGateway = useCallback(() => {
    // In a real implementation, this would come from the backend
    // For now, derive from broadcast address (typically .1 in the network)
    if (!activeInterface) return "-"
    const ipParts = activeInterface.ip.split(".")
    return `${ipParts[0]}.${ipParts[1]}.${ipParts[2]}.1`
  }, [activeInterface])

  return (
    <TooltipProvider delayDuration={200}>
      <Card className="w-full">
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <div className="flex items-center gap-2">
            <Network className="h-4 w-4 text-muted-foreground" />
            <CardTitle>Scan Controls</CardTitle>
          </div>
          <div className="flex items-center gap-2">
            <label
              htmlFor="auto-refresh"
              className="text-xs text-muted-foreground cursor-pointer select-none"
            >
              Auto
            </label>
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center gap-2">
                  <RefreshCw
                    className={cn(
                      "h-3 w-3 text-muted-foreground",
                      autoRefresh && "animate-spin"
                    )}
                  />
                  <Checkbox
                    id="auto-refresh"
                    checked={autoRefresh}
                    onCheckedChange={(checked) => setAutoRefresh(!!checked)}
                    disabled={isScanning}
                  />
                </div>
              </TooltipTrigger>
              <TooltipContent side="left">
                <p>Auto-refresh every 30 seconds</p>
              </TooltipContent>
            </Tooltip>
          </div>
        </CardHeader>

        <CardContent className="space-y-4">
          {/* Network Info Row */}
          <div className="flex flex-wrap items-center gap-3 text-xs">
            {/* Interface Selector */}
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground whitespace-nowrap">
                Interface:
              </span>
              <Select
                value={activeInterface?.name || ""}
                onValueChange={handleInterfaceChange}
                disabled={isScanning}
              >
                <SelectTrigger className="h-7 w-32">
                  <SelectValue placeholder="Select..." />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    {interfaces.length === 0 ? (
                      <SelectItem value="none" disabled>
                        No interfaces found
                      </SelectItem>
                    ) : (
                      interfaces.map((iface) => (
                        <SelectItem key={iface.name} value={iface.name}>
                          {iface.name}
                        </SelectItem>
                      ))
                    )}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>

            <Separator orientation="vertical" className="h-4" />

            {/* Network Range */}
            <div className="flex items-center gap-1.5">
              <span className="text-muted-foreground">Network:</span>
              <span className="font-mono text-foreground">
                {getNetworkRange()}
              </span>
            </div>

            <Separator orientation="vertical" className="h-4" />

            {/* Gateway */}
            <div className="flex items-center gap-1.5">
              <span className="text-muted-foreground">Gateway:</span>
              <span className="font-mono text-foreground">{getGateway()}</span>
            </div>
          </div>

          <Separator />

          {/* Scan Buttons */}
          <div className="flex flex-wrap items-center gap-2">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="default"
                  size="sm"
                  onClick={handleArpScan}
                  disabled={isScanning || interfaces.length === 0}
                  className="gap-1.5"
                >
                  {isScanning && scanType === "arp" ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <Zap className="h-3.5 w-3.5" />
                  )}
                  ARP Scan
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p>Fast ARP scan (usually takes 1-2 seconds)</p>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={handlePingScan}
                  disabled={isScanning || interfaces.length === 0}
                  className="gap-1.5"
                >
                  {isScanning && scanType === "ping" ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <Radar className="h-3.5 w-3.5" />
                  )}
                  Ping Scan
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p>Thorough ICMP ping scan (takes 10-30 seconds)</p>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={handleStopScan}
                  disabled={!isScanning}
                  className="gap-1.5"
                >
                  <Square className="h-3.5 w-3.5 fill-current" />
                  Stop
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p>Stop the current scan</p>
              </TooltipContent>
            </Tooltip>

            {/* Status Badge */}
            <div className="ml-auto">
              <Badge
                variant={isScanning ? "default" : "secondary"}
                className="gap-1"
              >
                {isScanning ? (
                  <>
                    <span className="relative flex h-1.5 w-1.5">
                      <span
                        className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary-foreground opacity-75"
                      />
                      <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-primary-foreground" />
                    </span>
                    {scanType === "arp" ? "ARP Scanning" : "Ping Scanning"}
                  </>
                ) : (
                  "Idle"
                )}
              </Badge>
            </div>
          </div>

          {/* Progress Section */}
          {isScanning && (
            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs">
                <span className="text-muted-foreground">Scanning...</span>
                <span className="font-mono">
                  {scanProgress
                    ? `${Math.round(scanProgress.progress)}%`
                    : "0%"}
                </span>
              </div>
              <Progress
                value={scanProgress ? scanProgress.progress : 0}
                className="h-2"
              />
              <div className="flex items-center justify-between text-xs text-muted-foreground">
                <span>
                  {scanProgress
                    ? `${scanProgress.devices_found} devices found`
                    : "Starting scan..."}
                </span>
                <span>
                  {devices.length > 0 && `${devices.length} total in list`}
                </span>
              </div>
            </div>
          )}

          {/* Error Display */}
          {error && (
            <div className="flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive">
              <AlertCircle className="h-4 w-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {/* Device Count (when not scanning) */}
          {!isScanning && devices.length > 0 && (
            <div className="flex items-center justify-between text-xs text-muted-foreground pt-2">
              <span>
                {devices.length} device{devices.length !== 1 ? "s" : ""} found
              </span>
              {autoRefresh && (
                <span className="flex items-center gap-1">
                  <RefreshCw className="h-3 w-3 animate-spin" />
                  Auto-refresh enabled
                </span>
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </TooltipProvider>
  )
}
