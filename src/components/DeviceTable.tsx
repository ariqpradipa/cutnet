"use client"

import { useState, useMemo, useCallback } from "react"
import { useDeviceStore } from "@/stores/deviceStore"
import { useNetworkStore } from "@/stores/networkStore"
import { killDevice, unkillDevice } from "@/utils/ipc"
import type { Device, KillState } from "@/lib/schemas"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
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
  Wifi,
  ShieldAlert,
  MoreVertical,
  Power,
  PowerOff,
  Info,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
} from "lucide-react"
import { cn } from "@/lib/utils"

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
  const { devices, selectedDevice, killStates, selectDevice, setKillState } =
    useDeviceStore()
  const { isScanning } = useNetworkStore()

  const [sort, setSort] = useState<SortState>({ field: "ip", direction: "asc" })
  const [selectedRows, setSelectedRows] = useState<Set<string>>(new Set())

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

  const sortedDevices = useMemo(() => {
    const sorted = [...devices]
    sorted.sort((a, b) => {
      let comparison = 0

      switch (sort.field) {
        case "status":
          const statusA = getDeviceStatus(a, killStates, isScanning).label
          const statusB = getDeviceStatus(b, killStates, isScanning).label
          comparison = statusA.localeCompare(statusB)
          break
        case "ip":
          comparison = a.ip.localeCompare(b.ip)
          break
        case "mac":
          comparison = a.mac.localeCompare(b.mac)
          break
        case "vendor":
          comparison = (a.vendor || "").localeCompare(b.vendor || "")
          break
        case "hostname":
          comparison = (a.hostname || "").localeCompare(b.hostname || "")
          break
        default:
          comparison = 0
      }

      return sort.direction === "asc" ? comparison : -comparison
    })
    return sorted
  }, [devices, sort, killStates, isScanning])

  const handleRowClick = useCallback(
    (device: Device) => {
      selectDevice(selectedDevice?.ip === device.ip ? null : device)
    },
    [selectedDevice, selectDevice]
  )

  const handleKillToggle = useCallback(
    async (device: Device, e: React.MouseEvent) => {
      e.stopPropagation()
      const killState = killStates.get(device.mac)

      if (killState?.is_killed) {
        await unkillDevice(device)
        setKillState(device.mac, {
          mac: device.mac,
          is_killed: false,
          kill_type: "none",
        })
      } else {
        await killDevice(device)
        setKillState(device.mac, {
          mac: device.mac,
          is_killed: true,
          kill_type: "arp_poison",
        })
      }
    },
    [killStates, setKillState]
  )

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

  const allSelected = devices.length > 0 && selectedRows.size === devices.length
  const someSelected = selectedRows.size > 0 && selectedRows.size < devices.length

  return (
    <TooltipProvider>
      <div className="flex flex-col gap-2">
        {selectedRows.size > 0 && (
          <div className="flex items-center gap-2 px-2">
            <span className="text-xs text-muted-foreground">
              {selectedRows.size} device{selectedRows.size === 1 ? "" : "s"} selected
            </span>
            <Button
              variant="destructive"
              size="xs"
              onClick={async () => {
                const selectedDevices = devices.filter((d) => selectedRows.has(d.ip))
                for (const device of selectedDevices) {
                  await killDevice(device)
                  setKillState(device.mac, {
                    mac: device.mac,
                    is_killed: true,
                    kill_type: "arp_poison",
                  })
                }
              }}
            >
              <PowerOff data-icon="inline-start" />
              Kill Selected
            </Button>
            <Button
              variant="outline"
              size="xs"
              onClick={async () => {
                const selectedDevices = devices.filter((d) => selectedRows.has(d.ip))
                for (const device of selectedDevices) {
                  await unkillDevice(device)
                  setKillState(device.mac, {
                    mac: device.mac,
                    is_killed: false,
                    kill_type: "none",
                  })
                }
              }}
            >
              <Power data-icon="inline-start" />
              Unkill Selected
            </Button>
          </div>
        )}

        <ScrollArea className="h-[400px] rounded-md border">
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
                <TableHead
                  className="cursor-pointer"
                  onClick={() => handleSort("status")}
                >
                  <div className="flex items-center gap-1">
                    Status
                    {getSortIcon("status")}
                  </div>
                </TableHead>
                <TableHead
                  className="cursor-pointer"
                  onClick={() => handleSort("ip")}
                >
                  <div className="flex items-center gap-1">
                    IP Address
                    {getSortIcon("ip")}
                  </div>
                </TableHead>
                <TableHead
                  className="cursor-pointer"
                  onClick={() => handleSort("mac")}
                >
                  <div className="flex items-center gap-1">
                    MAC Address
                    {getSortIcon("mac")}
                  </div>
                </TableHead>
                <TableHead
                  className="cursor-pointer"
                  onClick={() => handleSort("vendor")}
                >
                  <div className="flex items-center gap-1">
                    Vendor
                    {getSortIcon("vendor")}
                  </div>
                </TableHead>
                <TableHead
                  className="cursor-pointer"
                  onClick={() => handleSort("hostname")}
                >
                  <div className="flex items-center gap-1">
                    Hostname
                    {getSortIcon("hostname")}
                  </div>
                </TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {sortedDevices.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={7}
                    className="h-24 text-center text-muted-foreground"
                  >
                    No devices found. Start a scan to discover devices.
                  </TableCell>
                </TableRow>
              ) : (
                sortedDevices.map((device) => {
                  const status = getDeviceStatus(device, killStates, isScanning)
                  const StatusIcon = status.icon
                  const isSelected = selectedDevice?.ip === device.ip
                  const isKilled = killStates.get(device.mac)?.is_killed

                  return (
                    <TableRow
                      key={device.ip}
                      className={cn(
                        "cursor-pointer transition-colors",
                        isSelected && "bg-muted/70"
                      )}
                      onClick={() => handleRowClick(device)}
                      data-state={isSelected ? "selected" : undefined}
                    >
                      <TableCell>
                        <Checkbox
                          checked={selectedRows.has(device.ip)}
                          onCheckedChange={(checked) =>
                            handleSelectRow(device.ip, checked as boolean)
                          }
                          onClick={(e) => e.stopPropagation()}
                          aria-label={`Select device ${device.ip}`}
                        />
                      </TableCell>
                      <TableCell>
                        <Badge variant={status.variant}>
                          <StatusIcon data-icon="inline-start" />
                          {status.label}
                        </Badge>
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {device.ip}
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {device.mac}
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {device.vendor || "Unknown"}
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {device.hostname || "Unknown"}
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
                                {isKilled ? (
                                  <Power className="size-3" />
                                ) : (
                                  <PowerOff className="size-3" />
                                )}
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>
                              {isKilled ? "Unkill device" : "Kill device"}
                            </TooltipContent>
                          </Tooltip>

                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon-xs"
                                onClick={(e) => {
                                  e.stopPropagation()
                                  selectDevice(device)
                                }}
                              >
                                <Info className="size-3" />
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>View details</TooltipContent>
                          </Tooltip>

                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon-xs"
                                onClick={(e) => e.stopPropagation()}
                              >
                                <MoreVertical className="size-3" />
                              </Button>
                            </TooltipTrigger>
                            <TooltipContent>More actions</TooltipContent>
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
      </div>
    </TooltipProvider>
  )
}
