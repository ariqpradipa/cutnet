import { z } from "zod";

// Device type validation schema
export const DeviceSchema = z.object({
  ip: z.string().regex(/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/),
  mac: z.string().regex(/^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$/),
  hostname: z.string().nullable(),
  vendor: z.string().nullable(),
  is_router: z.boolean(),
  is_me: z.boolean(),
  custom_name: z.string().nullable().optional(),
});

export type Device = z.infer<typeof DeviceSchema>;

// Network interface schema
export const NetworkInterfaceSchema = z.object({
  name: z.string(),
  ip: z.string().regex(/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/),
  mac: z.string().regex(/^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$/),
  broadcast_addr: z.string().regex(/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/),
  netmask: z.string().regex(/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/),
});

export type NetworkInterface = z.infer<typeof NetworkInterfaceSchema>;

// Scan progress schema
export const ScanProgressSchema = z.object({
  type: z.enum(["arp", "ping"]),
  progress: z.number().min(0).max(100),
  devices_found: z.number().min(0),
});

export type ScanProgress = z.infer<typeof ScanProgressSchema>;

// Kill state schema
export const KillStateSchema = z.object({
  mac: z.string(),
  is_killed: z.boolean(),
  kill_type: z.enum(["none", "arp_poison", "one_way", "full"]),
});

export type KillState = z.infer<typeof KillStateSchema>;

// IPC Response wrappers
export const SuccessResponseSchema = z.object({
  success: z.literal(true),
  data: z.unknown(),
});

export const ErrorResponseSchema = z.object({
  success: z.literal(false),
  error: z.string(),
});

export const IpcResponseSchema = z.union([
  SuccessResponseSchema,
  ErrorResponseSchema,
]);

export type IpcResponse = z.infer<typeof IpcResponseSchema>;

export const DefenderAlertSchema = z.object({
  timestamp: z.number(),
  claimed_ip: z.string(),
  legitimate_mac: z.string(),
  attacker_mac: z.string(),
  alert_type: z.string(),
});

export type DefenderAlert = z.infer<typeof DefenderAlertSchema>;

export const WhitelistEntrySchema = z.object({
  mac: z.string(),
  label: z.string().nullable().optional(),
  added_at: z.number(),
});

export type WhitelistEntry = z.infer<typeof WhitelistEntrySchema>;

export const HistoryEntrySchema = z.object({
  ip: z.string(),
  mac: z.string(),
  hostname: z.string().nullable(),
  vendor: z.string().nullable(),
  join_time: z.number(),
  leave_time: z.number().nullable(),
  was_killed: z.boolean().optional(),
});

export type HistoryEntry = z.infer<typeof HistoryEntrySchema>;

export const BandwidthLimitSchema = z.object({
  mac: z.string(),
  download_limit_kbps: z.number().nullable(),
  upload_limit_kbps: z.number().nullable(),
  enabled: z.boolean(),
});

export type BandwidthLimit = z.infer<typeof BandwidthLimitSchema>;

export const BandwidthStatsSchema = z.object({
  mac: z.string(),
  current_download_kbps: z.number(),
  current_upload_kbps: z.number(),
  total_download_bytes: z.number(),
  total_upload_bytes: z.number(),
});

export type BandwidthStats = z.infer<typeof BandwidthStatsSchema>;
