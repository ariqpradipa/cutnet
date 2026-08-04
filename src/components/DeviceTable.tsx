import { useState, useMemo, useCallback, useEffect } from "react"
import { useDeviceStore } from "@/stores/deviceStore"
import { useNetworkStore } from "@/stores/networkStore"
import { killDevice, unkillDevice, killAllDevices, unkillAllDevices, setDeviceCustomName, getCustomDeviceNames, addWhitelistEntry, setBandwidthLimit, removeBandwidthLimit, getBandwidthStats } from "@/utils/ipc"
import { BandwidthControl } from "./BandwidthControl"
import type { Device, KillState, BandwidthLimit, BandwidthStats } from "@/lib/schemas";
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from "@/components/ui/dialog"
import { Separator } from "@/components/ui/separator"
import {
  Wifi,
  ShieldAlert,
  Power,
  PowerOff,
  Info,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  Router,
  Monitor,
  Copy,
  Check,
  Search,
  Pencil,
  Gauge,
  Smartphone,
  Laptop,
  Gamepad2,
  Server,
  Tv,
  ChevronLeft,
  ChevronRight,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { useToastStore } from "@/hooks/useToast"

function getDeviceIcon(vendor: string | null, hostname: string | null) {
  const vendorLower = (vendor || "").toLowerCase();
  const hostnameLower = (hostname || "").toLowerCase();
  
  if (["cisco", "netgear", "tp-link", "asus", "d-link", "linksys", "ubiquiti", "mikrotik", "router", "gateway"].some(v => vendorLower.includes(v))) {
    return Router;
  }
  if (vendorLower.includes("apple") || hostnameLower.includes("iphone") || hostnameLower.includes("ipad") || hostnameLower.includes("mac")) {
    if (hostnameLower.includes("iphone")) return Smartphone;
    if (hostnameLower.includes("ipad")) return Laptop;
    return Laptop;
  }
  if (vendorLower.includes("samsung") || hostnameLower.includes("samsung")) {
    return Smartphone;
  }
  if (vendorLower.includes("microsoft") && hostnameLower.includes("xbox") || 
      vendorLower.includes("sony") && hostnameLower.includes("playstation") ||
      vendorLower.includes("nintendo") || hostnameLower.includes("switch")) {
    return Gamepad2;
  }
  if (vendorLower.includes("lg") || (vendorLower.includes("samsung") && hostnameLower.includes("tv")) ||
      hostnameLower.includes("roku") || hostnameLower.includes("fire") || hostnameLower.includes("chromecast")) {
    return Tv;
  }
  if (vendorLower.includes("dell") || vendorLower.includes("hp") || vendorLower.includes("lenovo") || 
      hostnameLower.includes("server") || hostnameLower.includes("nas")) {
    return Server;
  }
  
  return Monitor;
}

type SortField = "status" | "ip" | "mac" | "vendor" | "hostname" | "actions"
type SortDirection = "asc" | "desc"

interface SortState {
  field: SortField
  direction: SortDirection
}

function getDeviceStatus(
  device: Device,
  killStates: Map<string, KillState>,
  isScanning: boolean
): {
  variant: "default" | "destructive" | "secondary"
  label: string
  icon: typeof Wifi
} {
  const killState = killStates.get(device.mac)

  if (killState?.is_killed) {
    return {
      variant: "destructive",
      label: "Killed",
      icon: ShieldAlert,
    }
  }

  if (isScanning) {
    return {
      variant: "secondary",
      label: "Scanning",
      icon: Wifi,
    }
  }

  return {
    variant: "default",
    label: "Online",
    icon: Wifi,
  }
}

export function DeviceTable() {
  const { 
    devices, 
    selectedDevice, 
    killStates, 
    bandwidthLimits,
    selectDevice, 
    setKillState, 
    updateDevice,
    setBandwidthLimit: setStoreBandwidthLimit,
    removeBandwidthLimit: removeStoreBandwidthLimit
  } = useDeviceStore()
  const { isScanning } = useNetworkStore()
  const { addToast } = useToastStore()

  const [sort, setSort] = useState<SortState>({ field: "ip", direction: "asc" })
  const [selectedRows, setSelectedRows] = useState<Set<string>>(new Set())
  const [showDetailDialog, setShowDetailDialog] = useState(false)
  const [copiedField, setCopiedField] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")
  const [customNames, setCustomNames] = useState<Record<string, string>>({})
  const [editingName, setEditingName] = useState<string | null>(null)
  const [editingValue, setEditingValue] = useState("")
  const [showKillAllConfirm, setShowKillAllConfirm] = useState(false)
  const [showBandwidthDialog, setShowBandwidthDialog] = useState(false)
  const [selectedBandwidthDevice, setSelectedBandwidthDevice] = useState<Device | null>(null)
  const [bandwidthStats, setBandwidthStats] = useState<BandwidthStats | null>(null)
  const [currentPage, setCurrentPage] = useState(1)
  const PAGE_SIZE = 20

  useEffect(() => {
    const loadNames = async () => {
      try {
        const names = await getCustomDeviceNames()
        setCustomNames(names)
      } catch (err) {
        console.error("Failed to load custom names:", err)
      }
    }
    loadNames()
  }, [])

  const handleSort = useCallback(
    (field: SortField) => {
      setSort((prev) => ({
        field,
        direction:
          prev.field === field && prev.direction === "asc" ? "desc" : "asc",
      }))
    },
    []
  )

  const getSortIcon = useCallback(
    (field: SortField) => {
      if (sort.field !== field) {
        return <ArrowUpDown className="size-3" />
      }
      return sort.direction === "asc" ? (
        <ArrowUp className="size-3" />
      ) : (
        <ArrowDown className="size-3" />
      )
    },
    [sort]
  )

  // Reset to page 1 when search changes
  useEffect(() => { setCurrentPage(1) }, [searchQuery])

  const filteredAndSortedDevices = useMemo(() => {
    const filtered = devices.filter((device) => {
      if (!searchQuery.trim()) return true
      const q = searchQuery.toLowerCase()
      const displayName = customNames[device.ip] || device.hostname || ""
      return (
        device.ip.toLowerCase().includes(q) ||
        device.mac.toLowerCase().includes(q) ||
        (device.hostname || "").toLowerCase().includes(q) ||
        (device.vendor || "").toLowerCase().includes(q) ||
        displayName.toLowerCase().includes(q)
      )
    })

    const sorted = [...filtered]
    sorted.sort((a, b) => {
      let comparison = 0

      switch (sort.field) {
        case "status": {
          const statusA = getDeviceStatus(a, killStates, isScanning).label
          const statusB = getDeviceStatus(b, killStates, isScanning).label
          comparison = statusA.localeCompare(statusB)
          break
        }
        case "ip": {
          // Numeric sort: convert each octet to a number for correct ordering
          const toNum = (ip: string) =>
            ip.split(".").reduce((acc, o) => acc * 256 + parseInt(o, 10), 0)
          comparison = toNum(a.ip) - toNum(b.ip)
          break
        }
        case "mac":
          comparison = a.mac.localeCompare(b.mac)
          break
        case "vendor":
          comparison = (a.vendor || "").localeCompare(b.vendor || "")
          break
        case "hostname": {
          const nameA = customNames[a.ip] || a.hostname || ""
          const nameB = customNames[b.ip] || b.hostname || ""
          comparison = nameA.localeCompare(nameB)
          break
        }
        default:
          comparison = 0
      }

      return sort.direction === "asc" ? comparison : -comparison
    })
    return sorted
  }, [devices, sort, killStates, isScanning, searchQuery, customNames])

  const handleRowClick = useCallback(
    (device: Device) => {
      selectDevice(selectedDevice?.ip === device.ip ? null : device)
    },
    [selectedDevice, selectDevice]
  )

  const handleKillToggle = useCallback(
    async (device: Device, e: React.MouseEvent) => {
      e.stopPropagation();
      const killState = killStates.get(device.mac);
      const wasKilled = killState?.is_killed;

      try {
        if (wasKilled) {
          await unkillDevice(device);
          setKillState(device.mac, { mac: device.mac, is_killed: false, kill_type: "none" });
          addToast({ title: "Device restored", description: `${device.ip} is back online`, variant: "default" });
        } else {
          await killDevice(device);
          setKillState(device.mac, { mac: device.mac, is_killed: true, kill_type: "arp_poison" });
          addToast({ title: "Device killed", description: `${device.ip} has been disconnected`, variant: "destructive" });
        }
      } catch (err) {
        setKillState(device.mac, { mac: device.mac, is_killed: wasKilled ?? false, kill_type: wasKilled ? "arp_poison" : "none" });
        addToast({
          title: "Operation failed",
          description: `Failed to ${wasKilled ? "restore" : "kill"} ${device.ip}: ${err instanceof Error ? err.message : "Unknown error"}`,
          variant: "destructive",
        });
      }
    },
    [killStates, setKillState, addToast]
  )

  const handleKillAll = useCallback(async () => {
    const killableDevices = devices.filter(d => !killStates.get(d.mac)?.is_killed && !d.is_me);
    if (killableDevices.length === 0) {
      addToast({ title: "No devices to kill", description: "All devices are already killed or this is your machine." });
      return;
    }
    setShowKillAllConfirm(true);
  }, [devices, killStates, addToast]);

  const confirmKillAll = useCallback(async () => {
    setShowKillAllConfirm(false);
    const killableDevices = devices.filter(d => !killStates.get(d.mac)?.is_killed && !d.is_me);
    try {
      await killAllDevices(killableDevices);
      for (const device of killableDevices) {
        setKillState(device.mac, { mac: device.mac, is_killed: true, kill_type: "arp_poison" });
      }
      addToast({ title: "All devices killed", description: `${killableDevices.length} device(s) disconnected`, variant: "destructive" });
    } catch (err) {
      addToast({ title: "Kill All failed", description: err instanceof Error ? err.message : "Unknown error", variant: "destructive" });
    }
  }, [devices, killStates, setKillState, addToast]);

  const handleUnkillAll = useCallback(async () => {
    const killedDevices = devices.filter(d => killStates.get(d.mac)?.is_killed);
    if (killedDevices.length === 0) {
      addToast({ title: "No devices to restore", description: "No devices are currently killed." });
      return;
    }
    try {
      await unkillAllDevices();
      for (const device of killedDevices) {
        setKillState(device.mac, { mac: device.mac, is_killed: false, kill_type: "none" });
      }
      addToast({ title: "All devices restored", description: `${killedDevices.length} device(s) reconnected` });
    } catch (err) {
      addToast({ title: "Unkill All failed", description: err instanceof Error ? err.message : "Unknown error", variant: "destructive" });
    }
  }, [devices, killStates, setKillState, addToast]);

  const handleKillSelected = useCallback(async () => {
    const selectedDevices = devices.filter(d => selectedRows.has(d.ip));
    const failed: string[] = [];
    for (const device of selectedDevices) {
      try {
        await killDevice(device);
        setKillState(device.mac, { mac: device.mac, is_killed: true, kill_type: "arp_poison" });
      } catch {
        failed.push(device.ip);
      }
    }
    if (failed.length > 0) {
      addToast({ title: "Partial failure", description: `Failed to kill: ${failed.join(", ")}`, variant: "destructive" });
    } else {
      addToast({ title: "Devices killed", description: `${selectedDevices.length} device(s) disconnected`, variant: "destructive" });
    }
    setSelectedRows(new Set());
  }, [devices, selectedRows, setKillState, addToast]);

  const handleUnkillSelected = useCallback(async () => {
    const selectedDevices = devices.filter(d => selectedRows.has(d.ip));
    const failed: string[] = [];
    for (const device of selectedDevices) {
      try {
        await unkillDevice(device);
        setKillState(device.mac, { mac: device.mac, is_killed: false, kill_type: "none" });
      } catch {
        failed.push(device.ip);
      }
    }
    if (failed.length > 0) {
      addToast({ title: "Partial failure", description: `Failed to restore: ${failed.join(", ")}`, variant: "destructive" });
    } else {
      addToast({ title: "Devices restored", description: `${selectedDevices.length} device(s) reconnected` });
    }
    setSelectedRows(new Set());
  }, [devices, selectedRows, setKillState, addToast]);

  const handleSelectAll = useCallback(
    (checked: boolean) => {
      if (checked) {
        setSelectedRows(new Set(devices.map((d) => d.ip)))
      } else {
        setSelectedRows(new Set())
      }
    },
    [devices]
  )

  const handleSelectRow = useCallback(
    (ip: string, checked: boolean) => {
      const newSelected = new Set(selectedRows)
      if (checked) {
        newSelected.add(ip)
      } else {
        newSelected.delete(ip)
      }
      setSelectedRows(newSelected)
    },
    [selectedRows]
  )

  const handleCopy = useCallback((text: string, field: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 1500);
  }, []);

  const allSelected = devices.length > 0 && selectedRows.size === devices.length
  const someSelected = selectedRows.size > 0 && selectedRows.size < devices.length
  const killedCount = devices.filter(d => killStates.get(d.mac)?.is_killed).length;
  const killableCount = devices.filter(d => !killStates.get(d.mac)?.is_killed && !d.is_me).length;

  const startEditingName = useCallback((device: Device) => {
    setEditingName(device.ip)
    setEditingValue(customNames[device.ip] || device.hostname || "")
  }, [customNames])

  const saveCustomName = useCallback(async (ip: string) => {
    try {
      await setDeviceCustomName(ip, editingValue)
      setCustomNames((prev) => ({ ...prev, [ip]: editingValue }))
      updateDevice(ip, { custom_name: editingValue || null })
    } catch (err) {
      console.error("Failed to save custom name:", err)
    } finally {
      setEditingName(null)
      setEditingValue("")
    }
  }, [editingValue, updateDevice])

  const handleNameKeyDown = useCallback((e: React.KeyboardEvent, ip: string) => {
    if (e.key === "Enter") {
      saveCustomName(ip)
    } else if (e.key === "Escape") {
      setEditingName(null)
      setEditingValue("")
    }
  }, [saveCustomName])

  const handleOpenBandwidthControl = useCallback((device: Device, e: React.MouseEvent) => {
    e.stopPropagation()
    setSelectedBandwidthDevice(device)
    setBandwidthStats(null)
    setShowBandwidthDialog(true)
    
    getBandwidthStats(device.mac).then(stats => {
      setBandwidthStats(stats)
    }).catch(() => {
    })
  }, [])

  const handleSetBandwidthLimit = useCallback(async (mac: string, download: number | null, upload: number | null) => {
    try {
      await setBandwidthLimit(mac, download, upload)
      const limit: BandwidthLimit = { mac, download_limit_kbps: download, upload_limit_kbps: upload, enabled: true }
      setStoreBandwidthLimit(mac, limit)
      addToast({ title: "Bandwidth limit applied", description: `Speed limits set for device` })
    } catch (err) {
      addToast({ title: "Failed to apply bandwidth limit", description: err instanceof Error ? err.message : "Unknown error", variant: "destructive" })
      throw err
    }
  }, [setStoreBandwidthLimit, addToast])

  const handleRemoveBandwidthLimit = useCallback(async (mac: string) => {
    try {
      await removeBandwidthLimit(mac)
      removeStoreBandwidthLimit(mac)
      addToast({ title: "Bandwidth limit removed", description: `Speed limits cleared for device` })
    } catch (err) {
      addToast({ title: "Failed to remove bandwidth limit", description: err instanceof Error ? err.message : "Unknown error", variant: "destructive" })
      throw err
    }
  }, [removeStoreBandwidthLimit, addToast])

  const getDeviceBandwidthDisplay = useCallback((mac: string) => {
    const limit = bandwidthLimits.get(mac)
    if (!limit || !limit.enabled) return null
    
    const hasDownload = limit.download_limit_kbps && limit.download_limit_kbps > 0
    const hasUpload = limit.upload_limit_kbps && limit.upload_limit_kbps > 0
    
    if (!hasDownload && !hasUpload) return null
    
    if (hasDownload && hasUpload) {
      return `↓${limit.download_limit_kbps} ↑${limit.upload_limit_kbps} KB/s`
    } else if (hasDownload) {
      return `↓${limit.download_limit_kbps} KB/s`
    } else {
      return `↑${limit.upload_limit_kbps} KB/s`
    }
  }, [bandwidthLimits])

  return (
    <TooltipProvider>
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-2">
          <Button
            variant="destructive"
            size="sm"
            onClick={handleKillAll}
            disabled={devices.length === 0}
          >
            <PowerOff data-icon="inline-start" />
            Kill All
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleUnkillAll}
            disabled={killedCount === 0}
          >
            <Power data-icon="inline-start" />
            Unkill All
          </Button>
          {killedCount > 0 && (
            <Badge variant="destructive" className="ml-auto">
              {killedCount} killed
            </Badge>
          )}
        </div>

        <div className="flex items-center gap-2 px-2">
          <div className="relative flex-1 max-w-xs">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
            <Input
              type="text"
              placeholder="Search by IP, MAC, hostname, vendor…"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 h-8"
            />
          </div>
          {searchQuery && (
            <span className="text-xs text-muted-foreground">
              {filteredAndSortedDevices.length} result{filteredAndSortedDevices.length !== 1 ? "s" : ""}
            </span>
          )}
        </div>

        {selectedRows.size > 0 && (
          <div className="flex items-center gap-2 px-2">
            <span className="text-xs text-muted-foreground">
              {selectedRows.size} device{selectedRows.size === 1 ? "" : "s"} selected
            </span>
            <Button variant="destructive" size="xs" onClick={handleKillSelected}>
              <PowerOff data-icon="inline-start" />
              Kill Selected
            </Button>
            <Button variant="outline" size="xs" onClick={handleUnkillSelected}>
              <Power data-icon="inline-start" />
              Unkill Selected
            </Button>
          </div>
        )}

        <ScrollArea className="h-[calc(100vh-22rem)] min-h-[280px] rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[40px]">
                  <Checkbox
                    checked={allSelected}
                    data-state={someSelected ? "indeterminate" : undefined}
                    onCheckedChange={handleSelectAll}
                    aria-label="Select all devices"
                  />
                </TableHead>
                <TableHead className="cursor-pointer" onClick={() => handleSort("status")}>
                  <div className="flex items-center gap-1">Status {getSortIcon("status")}</div>
                </TableHead>
                <TableHead className="cursor-pointer" onClick={() => handleSort("ip")}>
                  <div className="flex items-center gap-1">IP Address {getSortIcon("ip")}</div>
                </TableHead>
                <TableHead className="cursor-pointer" onClick={() => handleSort("mac")}>
                  <div className="flex items-center gap-1">MAC Address {getSortIcon("mac")}</div>
                </TableHead>
                <TableHead className="cursor-pointer" onClick={() => handleSort("vendor")}>
                  <div className="flex items-center gap-1">Vendor {getSortIcon("vendor")}</div>
                </TableHead>
                <TableHead className="cursor-pointer" onClick={() => handleSort("hostname")}>
                  <div className="flex items-center gap-1">Hostname {getSortIcon("hostname")}</div>
                </TableHead>
                <TableHead className="text-center">
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <span className="inline-flex items-center gap-1 cursor-help">
                        <Gauge className="size-3" />
                        Bandwidth
                      </span>
                    </TooltipTrigger>
                    <TooltipContent><p>Click to set speed limits for this device</p></TooltipContent>
                  </Tooltip>
                </TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredAndSortedDevices.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={8} className="h-24 text-center text-muted-foreground">
                    {searchQuery
                      ? "No devices match your search."
                      : "No devices found. Start a scan to discover devices."}
                  </TableCell>
                </TableRow>
              ) : (
                filteredAndSortedDevices
                  .slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE)
                  .map((device) => {
                  const status = getDeviceStatus(device, killStates, isScanning)
                  const StatusIcon = status.icon
                  const isSelected = selectedDevice?.ip === device.ip
                  const isKilled = killStates.get(device.mac)?.is_killed
                  const displayName = customNames[device.ip] || device.hostname || "Unknown"
                  const isEditing = editingName === device.ip

                  return (
                    <TableRow
                      key={device.ip}
                      className={cn("cursor-pointer transition-colors", isSelected && "bg-muted/70")}
                      onClick={() => handleRowClick(device)}
                      data-state={isSelected ? "selected" : undefined}
                    >
                      <TableCell>
                        <Checkbox
                          checked={selectedRows.has(device.ip)}
                          onCheckedChange={(checked) => handleSelectRow(device.ip, checked as boolean)}
                          onClick={(e) => e.stopPropagation()}
                          aria-label={`Select device ${device.ip}`}
                        />
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1 flex-wrap">
                          <Badge variant={status.variant}>
                            <StatusIcon data-icon="inline-start" />
                            {status.label}
                          </Badge>
                          {device.is_router && (
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <Badge variant="outline" className="text-amber-600 border-amber-300 dark:border-amber-700 dark:text-amber-400">
                                  <Router data-icon="inline-start" />Router
                                </Badge>
                              </TooltipTrigger>
                              <TooltipContent>Default Gateway</TooltipContent>
                            </Tooltip>
                          )}
                          {device.is_me && (
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <Badge variant="secondary"><Monitor data-icon="inline-start" />You</Badge>
                              </TooltipTrigger>
                              <TooltipContent>This machine</TooltipContent>
                            </Tooltip>
                          )}
                        </div>
                      </TableCell>
                      <TableCell className="font-mono text-xs">{device.ip}</TableCell>
                      <TableCell className="font-mono text-xs">{device.mac}</TableCell>
                      <TableCell className="text-muted-foreground">{device.vendor || "Unknown"}</TableCell>
                      <TableCell className="text-muted-foreground">
                        {isEditing ? (
                          <Input
                            value={editingValue}
                            onChange={(e) => setEditingValue(e.target.value)}
                            onBlur={() => saveCustomName(device.ip)}
                            onKeyDown={(e) => handleNameKeyDown(e, device.ip)}
                            onClick={(e) => e.stopPropagation()}
                            className="h-6 text-xs w-32"
                            autoFocus
                          />
                        ) : (
                          <div
                            className="flex items-center gap-1 group cursor-pointer"
                            onDoubleClick={(e) => { e.stopPropagation(); startEditingName(device) }}
                          >
                            {(() => {
                              const DeviceIcon = getDeviceIcon(device.vendor, displayName);
                              return <DeviceIcon className="size-3 text-muted-foreground shrink-0" />;
                            })()}
                            <span className="truncate max-w-[120px]">{displayName}</span>
                            <Pencil
                              className="size-3 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                              onClick={(e) => { e.stopPropagation(); startEditingName(device) }}
                            />
                          </div>
                        )}
                      </TableCell>
                      <TableCell
                        className="text-center text-muted-foreground text-xs cursor-pointer hover:text-foreground"
                        onClick={(e) => handleOpenBandwidthControl(device, e)}
                      >
                        {getDeviceBandwidthDisplay(device.mac) || (
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <span className="cursor-help">—</span>
                            </TooltipTrigger>
                            <TooltipContent><p>Click to set speed limit</p></TooltipContent>
                          </Tooltip>
                        )}
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center justify-end gap-1">
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant={isKilled ? "outline" : "destructive"}
                                size="icon-xs"
                                onClick={(e) => handleKillToggle(device, e)}
                                disabled={device.is_me}
                              >
                                {isKilled ? <Power className="size-3" /> : <PowerOff className="size-3" />}
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>{isKilled ? "Restore device" : "Block device"}</TooltipContent>
                          </Tooltip>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon-xs"
                                onClick={(e) => { e.stopPropagation(); selectDevice(device); setShowDetailDialog(true) }}
                              >
                                <Info className="size-3" />
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>View details</TooltipContent>
                          </Tooltip>
                        </div>
                      </TableCell>
                    </TableRow>
                  )
                })
              )}
            </TableBody>
          </Table>
        </ScrollArea>

        {/* Pagination */}
        {filteredAndSortedDevices.length > PAGE_SIZE && (
          <div className="flex items-center justify-between px-1 text-xs text-muted-foreground">
            <span>
              Showing {(currentPage - 1) * PAGE_SIZE + 1}–{Math.min(currentPage * PAGE_SIZE, filteredAndSortedDevices.length)} of {filteredAndSortedDevices.length} devices
            </span>
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="icon-xs"
                onClick={() => setCurrentPage(p => Math.max(1, p - 1))}
                disabled={currentPage === 1}
              >
                <ChevronLeft className="size-3.5" />
              </Button>
              <span className="px-1">Page {currentPage} of {Math.ceil(filteredAndSortedDevices.length / PAGE_SIZE)}</span>
              <Button
                variant="ghost"
                size="icon-xs"
                onClick={() => setCurrentPage(p => Math.min(Math.ceil(filteredAndSortedDevices.length / PAGE_SIZE), p + 1))}
                disabled={currentPage === Math.ceil(filteredAndSortedDevices.length / PAGE_SIZE)}
              >
                <ChevronRight className="size-3.5" />
              </Button>
            </div>
          </div>
        )}
      </div>

      <Dialog open={showDetailDialog} onOpenChange={setShowDetailDialog}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Monitor className="size-4" />
              Device Details
            </DialogTitle>
            <DialogDescription>
              {selectedDevice?.ip || "No device selected"}
            </DialogDescription>
          </DialogHeader>

          {selectedDevice && (
            <div className="space-y-4">
              <div className="flex items-center gap-2">
                {selectedDevice.is_router && (
                  <Badge variant="outline" className="text-amber-600 border-amber-300 dark:border-amber-700 dark:text-amber-400">
                    <Router data-icon="inline-start" />
                    Router
                  </Badge>
                )}
                {selectedDevice.is_me && (
                  <Badge variant="secondary">
                    <Monitor data-icon="inline-start" />
                    This Machine
                  </Badge>
                )}
                {killStates.get(selectedDevice.mac)?.is_killed ? (
                  <Badge variant="destructive">
                    <ShieldAlert data-icon="inline-start" />
                    Killed
                  </Badge>
                ) : (
                  <Badge variant="default">
                    <Wifi data-icon="inline-start" />
                    Online
                  </Badge>
                )}
              </div>

              <Separator />

              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs text-muted-foreground">IP Address</p>
                    <p className="font-mono text-sm">{selectedDevice.ip}</p>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => handleCopy(selectedDevice.ip, "ip")}
                  >
                    {copiedField === "ip" ? (
                      <Check className="size-3.5 text-emerald-500" />
                    ) : (
                      <Copy className="size-3.5" />
                    )}
                  </Button>
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs text-muted-foreground">MAC Address</p>
                    <p className="font-mono text-sm">{selectedDevice.mac}</p>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => handleCopy(selectedDevice.mac, "mac")}
                  >
                    {copiedField === "mac" ? (
                      <Check className="size-3.5 text-emerald-500" />
                    ) : (
                      <Copy className="size-3.5" />
                    )}
                  </Button>
                </div>

                <div>
                  <p className="text-xs text-muted-foreground">Hostname</p>
                  <p className="text-sm">{selectedDevice.hostname || "Unknown"}</p>
                </div>

                <div>
                  <p className="text-xs text-muted-foreground">Vendor</p>
                  <p className="text-sm">{selectedDevice.vendor || "Unknown"}</p>
                </div>
              </div>

              <Separator />

              <DialogFooter className="gap-2 sm:gap-0">
                <DialogClose asChild>
                  <Button variant="outline">Close</Button>
                </DialogClose>
                <Button
                  variant="outline"
                  onClick={async () => {
                    try {
                      await addWhitelistEntry(selectedDevice.mac);
                      addToast({ title: "Whitelist", description: `${selectedDevice.mac} added to whitelist` });
                    } catch (err) {
                      addToast({ title: "Whitelist failed", description: err instanceof Error ? err.message : "Unknown error", variant: "destructive" });
                    }
                    setShowDetailDialog(false);
                  }}
                >
                  Add to Whitelist
                </Button>
                {!selectedDevice.is_me && (
                  <Button
                    variant={killStates.get(selectedDevice.mac)?.is_killed ? "outline" : "destructive"}
                    onClick={(e) => {
                      handleKillToggle(selectedDevice, e as unknown as React.MouseEvent);
                    }}
                  >
                    {killStates.get(selectedDevice.mac)?.is_killed ? (
                      <>
                        <Power data-icon="inline-start" />
                        Unkill
                      </>
                    ) : (
                      <>
                        <PowerOff data-icon="inline-start" />
                        Kill
                      </>
                    )}
                  </Button>
                )}
              </DialogFooter>
            </div>
          )}
        </DialogContent>
      </Dialog>

      <Dialog open={showKillAllConfirm} onOpenChange={setShowKillAllConfirm}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Block {killableCount} device{killableCount !== 1 ? "s" : ""}?</DialogTitle>
            <DialogDescription>
              This will block internet access for all non-whitelisted devices on your network.
              This action can be reversed.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowKillAllConfirm(false)}>Cancel</Button>
            <Button variant="destructive" onClick={confirmKillAll}>
              <PowerOff data-icon="inline-start" />
              Block All
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {selectedBandwidthDevice && (
        <BandwidthControl
          device={selectedBandwidthDevice}
          limit={bandwidthLimits.get(selectedBandwidthDevice.mac) || null}
          stats={bandwidthStats}
          isOpen={showBandwidthDialog}
          onClose={() => {
            setShowBandwidthDialog(false)
            setSelectedBandwidthDevice(null)
            setBandwidthStats(null)
          }}
          onSetLimit={handleSetBandwidthLimit}
          onRemoveLimit={handleRemoveBandwidthLimit}
        />
      )}
    </TooltipProvider>
  )
}
