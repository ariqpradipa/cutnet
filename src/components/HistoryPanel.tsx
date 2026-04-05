"use client"

import { useEffect, useState, useCallback } from "react"
import { getHistory, clearHistory } from "@/utils/ipc"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Trash2, Clock } from "lucide-react"

interface HistoryEntry {
  ip: string
  mac: string
  hostname: string | null
  vendor: string | null
  custom_name: string | null
  joined_at: number
  left_at: number | null
}

function formatTimestamp(ts: number): string {
  return new Date(ts * 1000).toLocaleString()
}

function formatDuration(joinedAt: number, leftAt: number): string {
  const seconds = leftAt - joinedAt
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`
  const hours = Math.floor(seconds / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  return `${hours}h ${mins}m`
}

export function HistoryPanel() {
  const [entries, setEntries] = useState<HistoryEntry[]>([])
  const [loading, setLoading] = useState(true)

  const loadHistory = useCallback(async () => {
    try {
      const data = await getHistory()
      setEntries(data)
    } catch (err) {
      console.error("Failed to load history:", err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    loadHistory()
  }, [loadHistory])

  const handleClear = useCallback(async () => {
    try {
      await clearHistory()
      setEntries([])
    } catch (err) {
      console.error("Failed to clear history:", err)
    }
  }, [])

  const sortedEntries = [...entries].sort(
    (a, b) => b.joined_at - a.joined_at
  )

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Clock className="size-4" />
          <span>{entries.length} event{entries.length !== 1 ? "s" : ""} recorded</span>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={handleClear}
          disabled={entries.length === 0}
        >
          <Trash2 data-icon="inline-start" />
          Clear History
        </Button>
      </div>

      <ScrollArea className="h-[400px] rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Event</TableHead>
              <TableHead>IP Address</TableHead>
              <TableHead>MAC Address</TableHead>
              <TableHead>Hostname</TableHead>
              <TableHead>Vendor</TableHead>
              <TableHead>Joined At</TableHead>
              <TableHead>Left At</TableHead>
              <TableHead>Duration</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <TableRow>
                <TableCell colSpan={8} className="h-24 text-center text-muted-foreground">
                  Loading history...
                </TableCell>
              </TableRow>
            ) : sortedEntries.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="h-24 text-center text-muted-foreground">
                  No history yet. Device sessions will appear here after scanning.
                </TableCell>
              </TableRow>
            ) : (
              sortedEntries.map((entry, idx) => (
                <TableRow key={`${entry.ip}-${entry.joined_at}-${idx}`}>
                  <TableCell>
                    <Badge variant={entry.left_at ? "secondary" : "default"}>
                      {entry.left_at ? "Left" : "Joined"}
                    </Badge>
                  </TableCell>
                  <TableCell className="font-mono text-xs">{entry.ip}</TableCell>
                  <TableCell className="font-mono text-xs">{entry.mac}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {entry.custom_name || entry.hostname || "Unknown"}
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {entry.vendor || "Unknown"}
                  </TableCell>
                  <TableCell className="text-xs">
                    {formatTimestamp(entry.joined_at)}
                  </TableCell>
                  <TableCell className="text-xs">
                    {entry.left_at ? formatTimestamp(entry.left_at) : "—"}
                  </TableCell>
                  <TableCell className="text-xs">
                    {entry.left_at
                      ? formatDuration(entry.joined_at, entry.left_at)
                      : "Still online"}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </ScrollArea>
    </div>
  )
}
