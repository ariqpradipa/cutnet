"use client"

import { useState } from "react"
import type { Device } from "@/lib/schemas"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { Gauge, ArrowDown, ArrowUp, Trash2, Save } from "lucide-react"
import { cn } from "@/lib/utils"

interface BandwidthLimit {
  mac: string
  download_limit_kbps: number | null
  upload_limit_kbps: number | null
  enabled: boolean
}

interface BandwidthStats {
  mac: string
  current_download_kbps: number
  current_upload_kbps: number
  total_download_bytes: number
  total_upload_bytes: number
}

interface BandwidthControlProps {
  device: Device
  limit: BandwidthLimit | null
  stats: BandwidthStats | null
  isOpen: boolean
  onClose: () => void
  onSetLimit: (mac: string, download: number | null, upload: number | null) => Promise<void>
  onRemoveLimit: (mac: string) => Promise<void>
}

const PRESET_LIMITS = [
  { value: "0", label: "Unlimited" },
  { value: "100", label: "100 KB/s" },
  { value: "256", label: "256 KB/s" },
  { value: "512", label: "512 KB/s" },
  { value: "1024", label: "1 MB/s" },
  { value: "2048", label: "2 MB/s" },
  { value: "5120", label: "5 MB/s" },
  { value: "10240", label: "10 MB/s" },
]

function formatSpeed(kbps: number): string {
  if (kbps === 0) return "Unlimited"
  if (kbps < 1024) return `${kbps} KB/s`
  return `${(kbps / 1024).toFixed(1)} MB/s`
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const units = ["B", "KB", "MB", "GB", "TB"]
  let size = bytes
  let unitIndex = 0
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  return `${size.toFixed(2)} ${units[unitIndex]}`
}

export function BandwidthControl({
  device,
  limit,
  stats,
  isOpen,
  onClose,
  onSetLimit,
  onRemoveLimit,
}: BandwidthControlProps) {
  const [downloadLimit, setDownloadLimit] = useState<string>(
    limit?.download_limit_kbps?.toString() || "0"
  )
  const [uploadLimit, setUploadLimit] = useState<string>(
    limit?.upload_limit_kbps?.toString() || "0"
  )
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const hasLimit = limit?.enabled && (limit.download_limit_kbps || limit.upload_limit_kbps)

  const handleApply = async () => {
    setIsLoading(true)
    setError(null)

    try {
      const download = downloadLimit === "0" ? null : parseInt(downloadLimit, 10)
      const upload = uploadLimit === "0" ? null : parseInt(uploadLimit, 10)

      await onSetLimit(device.mac, download, upload)
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to apply bandwidth limit")
    } finally {
      setIsLoading(false)
    }
  }

  const handleRemove = async () => {
    setIsLoading(true)
    setError(null)

    try {
      await onRemoveLimit(device.mac)
      setDownloadLimit("0")
      setUploadLimit("0")
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to remove bandwidth limit")
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[450px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Gauge className="size-5" />
            Bandwidth Control
          </DialogTitle>
          <DialogDescription>
            Set upload and download speed limits for {device.hostname || device.ip}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Current Status */}
          {hasLimit && (
            <div className="rounded-lg border bg-muted/50 p-4">
              <h4 className="text-sm font-medium mb-2">Current Limits</h4>
              <div className="flex gap-4">
                <div className="flex items-center gap-2">
                  <ArrowDown className="size-4 text-blue-500" />
                  <span className="text-sm">
                    {formatSpeed(limit?.download_limit_kbps || 0)}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <ArrowUp className="size-4 text-green-500" />
                  <span className="text-sm">
                    {formatSpeed(limit?.upload_limit_kbps || 0)}
                  </span>
                </div>
              </div>
            </div>
          )}

          {/* Stats */}
          {stats && (stats.total_download_bytes > 0 || stats.total_upload_bytes > 0) && (
            <div className="rounded-lg border bg-muted/50 p-4">
              <h4 className="text-sm font-medium mb-2">Usage Statistics</h4>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <p className="text-xs text-muted-foreground">Downloaded</p>
                  <p className="text-sm font-medium">{formatBytes(stats.total_download_bytes)}</p>
                </div>
                <div>
                  <p className="text-xs text-muted-foreground">Uploaded</p>
                  <p className="text-sm font-medium">{formatBytes(stats.total_upload_bytes)}</p>
                </div>
              </div>
              {(stats.current_download_kbps > 0 || stats.current_upload_kbps > 0) && (
                <div className="mt-3 pt-3 border-t grid grid-cols-2 gap-4">
                  <div>
                    <p className="text-xs text-muted-foreground">Current Down</p>
                    <p className="text-sm font-medium text-blue-500">
                      {formatSpeed(stats.current_download_kbps)}
                    </p>
                  </div>
                  <div>
                    <p className="text-xs text-muted-foreground">Current Up</p>
                    <p className="text-sm font-medium text-green-500">
                      {formatSpeed(stats.current_upload_kbps)}
                    </p>
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Download Limit */}
          <div className="space-y-2">
            <Label htmlFor="download-limit" className="flex items-center gap-2">
              <ArrowDown className="size-4 text-blue-500" />
              Download Limit
            </Label>
            <Select
              value={downloadLimit}
              onValueChange={setDownloadLimit}
            >
              <SelectTrigger id="download-limit">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PRESET_LIMITS.map((preset) => (
                  <SelectItem key={preset.value} value={preset.value}>
                    {preset.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Upload Limit */}
          <div className="space-y-2">
            <Label htmlFor="upload-limit" className="flex items-center gap-2">
              <ArrowUp className="size-4 text-green-500" />
              Upload Limit
            </Label>
            <Select
              value={uploadLimit}
              onValueChange={setUploadLimit}
            >
              <SelectTrigger id="upload-limit">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PRESET_LIMITS.map((preset) => (
                  <SelectItem key={preset.value} value={preset.value}>
                    {preset.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Error Message */}
          {error && (
            <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </div>
          )}

          {/* Permission Warning */}
          <div className="rounded-md bg-amber-500/10 p-3 text-sm text-amber-600">
            Note: Bandwidth control requires administrator/root privileges to modify network rules.
          </div>
        </div>

        <DialogFooter className="gap-2">
          {hasLimit && (
            <Button
              variant="destructive"
              onClick={handleRemove}
              disabled={isLoading}
              className="gap-2"
            >
              <Trash2 className="size-4" />
              Remove Limit
            </Button>
          )}
          <Button
            onClick={handleApply}
            disabled={isLoading}
            className="gap-2"
          >
            <Save className="size-4" />
            {isLoading ? "Applying..." : "Apply Limit"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
