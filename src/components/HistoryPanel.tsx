"use client"

import { useEffect, useState, useCallback, useMemo } from "react"
import { getHistory, clearHistory } from "@/utils/ipc"
import type { HistoryEntry } from "@/lib/schemas"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog"
import { Trash2, Clock, RefreshCw, Search, ChevronLeft, ChevronRight, ShieldAlert } from "lucide-react"
import { cn } from "@/lib/utils"

const PAGE_SIZE = 15

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
  const [showClearConfirm, setShowClearConfirm] = useState(false)
  const [historySearch, setHistorySearch] = useState("")
  const [currentPage, setCurrentPage] = useState(1)

  const loadHistory = useCallback(async () => {
    setLoading(true)
    try {
      const data = await getHistory()
      setEntries(data)
    } catch (err) {
      console.error("Failed to load history:", err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { loadHistory() }, [loadHistory])

  // Reset page when search changes
  useEffect(() => { setCurrentPage(1) }, [historySearch])

  const handleClear = useCallback(() => {
    if (entries.length === 0) return
    setShowClearConfirm(true)
  }, [entries.length])

  const confirmClear = useCallback(async () => {
    setShowClearConfirm(false)
    try {
      await clearHistory()
      setEntries([])
    } catch (err) {
      console.error("Failed to clear history:", err)
    }
  }, [])

  const filteredEntries = useMemo(() => {
    const sorted = [...entries].sort((a, b) => b.join_time - a.join_time)
    if (!historySearch.trim()) return sorted
    const q = historySearch.toLowerCase()
    return sorted.filter(e =>
      e.ip.toLowerCase().includes(q) ||
      e.mac.toLowerCase().includes(q) ||
      (e.hostname || "").toLowerCase().includes(q) ||
      (e.vendor || "").toLowerCase().includes(q)
    )
  }, [entries, historySearch])

  const totalPages = Math.ceil(filteredEntries.length / PAGE_SIZE)
  const paginatedEntries = filteredEntries.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE)

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Clock className="size-4" />
          <span>{entries.length} event{entries.length !== 1 ? "s" : ""} recorded</span>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={loadHistory} disabled={loading}>
            <RefreshCw className={cn("size-3.5", loading && "animate-spin")} />
            Refresh
          </Button>
          <Button variant="outline" size="sm" onClick={handleClear} disabled={entries.length === 0}>
            <Trash2 data-icon="inline-start" />
            Clear History
          </Button>
        </div>
      </div>

      {/* Search */}
      <div className="relative max-w-sm">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
        <Input
          placeholder="Filter by IP, MAC, hostname…"
          value={historySearch}
          onChange={(e) => setHistorySearch(e.target.value)}
          className="pl-9 h-8"
        />
      </div>

      <ScrollArea className="h-[calc(100vh-22rem)] min-h-[280px] rounded-md border">
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
              // Loading skeleton
              Array.from({ length: 3 }).map((_, i) => (
                <TableRow key={i}>
                  {Array.from({ length: 8 }).map((_, j) => (
                    <TableCell key={j}>
                      <div className="h-4 bg-muted rounded animate-pulse" />
                    </TableCell>
                  ))}
                </TableRow>
              ))
            ) : filteredEntries.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="h-24 text-center text-muted-foreground">
                  {historySearch
                    ? "No events match your filter."
                    : "No history yet. Device sessions will appear here after scanning."}
                </TableCell>
              </TableRow>
            ) : (
              paginatedEntries.map((entry, idx) => (
                <TableRow key={`${entry.ip}-${entry.join_time}-${idx}`}>
                  <TableCell>
                    <div className="flex items-center gap-1 flex-wrap">
                      <Badge variant={entry.leave_time ? "secondary" : "default"}>
                        {entry.leave_time ? "Left" : "Online"}
                      </Badge>
                      {entry.was_killed && (
                        <Badge variant="destructive" className="gap-1">
                          <ShieldAlert className="size-3" />
                          Killed
                        </Badge>
                      )}
                    </div>
                  </TableCell>
                  <TableCell className="font-mono text-xs">{entry.ip}</TableCell>
                  <TableCell className="font-mono text-xs">{entry.mac}</TableCell>
                  <TableCell className="text-muted-foreground">{entry.hostname || "—"}</TableCell>
                  <TableCell className="text-muted-foreground">{entry.vendor || "—"}</TableCell>
                  <TableCell className="text-xs">{formatTimestamp(entry.join_time)}</TableCell>
                  <TableCell className="text-xs">
                    {entry.leave_time ? formatTimestamp(entry.leave_time) : "—"}
                  </TableCell>
                  <TableCell className="text-xs">
                    {entry.leave_time ? formatDuration(entry.join_time, entry.leave_time) : "Active"}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </ScrollArea>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between px-1 text-xs text-muted-foreground">
          <span>
            Showing {(currentPage - 1) * PAGE_SIZE + 1}–{Math.min(currentPage * PAGE_SIZE, filteredEntries.length)} of {filteredEntries.length}
          </span>
          <div className="flex items-center gap-1">
            <Button variant="ghost" size="icon-xs" onClick={() => setCurrentPage(p => Math.max(1, p - 1))} disabled={currentPage === 1}>
              <ChevronLeft className="size-3.5" />
            </Button>
            <span className="px-1">Page {currentPage} of {totalPages}</span>
            <Button variant="ghost" size="icon-xs" onClick={() => setCurrentPage(p => Math.min(totalPages, p + 1))} disabled={currentPage === totalPages}>
              <ChevronRight className="size-3.5" />
            </Button>
          </div>
        </div>
      )}

      <Dialog open={showClearConfirm} onOpenChange={setShowClearConfirm}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Clear history?</DialogTitle>
            <DialogDescription>
              This will permanently delete all {entries.length} recorded event{entries.length !== 1 ? "s" : ""}. This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowClearConfirm(false)}>Cancel</Button>
            <Button variant="destructive" onClick={confirmClear}>
              <Trash2 data-icon="inline-start" />
              Clear History
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
