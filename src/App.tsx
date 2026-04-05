import { useState, useEffect } from "react";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { DeviceTable } from "@/components/DeviceTable";
import { ScanControls } from "@/components/ScanControls";
import { SettingsPanel } from "@/components/SettingsPanel";
import { HistoryPanel } from "@/components/HistoryPanel";
import { ForwardingPanel } from "@/components/ForwardingPanel";
import { Shield, Users, Settings, Clock, Moon, Sun, ArrowLeftRight } from "lucide-react";
import { useNetworkStore } from "@/stores/networkStore";
import { ToastContainer } from "@/components/ToastContainer";
import { useTauriEvents } from "@/hooks/useTauriEvents";
import ErrorBoundary from "@/components/ErrorBoundary";
import "./App.css";

function AppContent() {
  const [activeTab, setActiveTab] = useState("devices");
  const { isRunning, isScanning } = useNetworkStore();
  const [isDark, setIsDark] = useState(false);

  useTauriEvents();

  useEffect(() => {
    const stored = localStorage.getItem("cutnet-dark-mode");
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    const initialDark = stored === "true" || (stored === null && prefersDark);
    setIsDark(initialDark);
    document.documentElement.classList.toggle("dark", initialDark);
  }, []);

  const toggleDarkMode = () => {
    const newDark = !isDark;
    setIsDark(newDark);
    document.documentElement.classList.toggle("dark", newDark);
    localStorage.setItem("cutnet-dark-mode", newDark ? "true" : "false");
  };

  return (
    <TooltipProvider>
      <main className="container mx-auto p-6 min-h-screen bg-background">
        <header className="mb-8">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-4xl font-bold text-foreground mb-2">CutNet</h1>
              <p className="text-muted-foreground">
                Network Administration Tool
              </p>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={toggleDarkMode}
                className="p-2 rounded-full hover:bg-accent transition-colors"
                aria-label="Toggle dark mode"
              >
                {isDark ? <Sun className="size-5" /> : <Moon className="size-5" />}
              </button>
              {isScanning && (
                <Badge variant="default" className="animate-pulse">
                  Scanning
                </Badge>
              )}
              <Badge variant={isRunning ? "default" : "secondary"}>
                <span className={`inline-block w-2 h-2 rounded-full mr-1 ${isRunning ? 'bg-green-500' : 'bg-gray-400'}`} />
                {isRunning ? "Running" : "Stopped"}
              </Badge>
            </div>
          </div>
        </header>

        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <TabsList className="grid w-full grid-cols-5 mb-6">
            <TabsTrigger value="devices" className="flex items-center gap-2">
              <Users className="size-4" />
              Devices
            </TabsTrigger>
            <TabsTrigger value="history" className="flex items-center gap-2">
              <Clock className="size-4" />
              History
            </TabsTrigger>
            <TabsTrigger value="forwarding" className="flex items-center gap-2">
              <ArrowLeftRight className="size-4" />
              Forwarding
            </TabsTrigger>
            <TabsTrigger value="defender" className="flex items-center gap-2">
              <Shield className="size-4" />
              Defender
            </TabsTrigger>
            <TabsTrigger value="settings" className="flex items-center gap-2">
              <Settings className="size-4" />
              Settings
            </TabsTrigger>
          </TabsList>

          <TabsContent value="devices" className="space-y-6">
            <ScanControls />
            <DeviceTable />
          </TabsContent>

          <TabsContent value="history" className="space-y-6">
            <HistoryPanel />
          </TabsContent>

          <TabsContent value="forwarding" className="space-y-6">
            <ForwardingPanel />
          </TabsContent>

          <TabsContent value="defender" className="space-y-6">
            <SettingsPanel defaultTab="defender" />
          </TabsContent>

          <TabsContent value="settings" className="space-y-6">
            <SettingsPanel />
          </TabsContent>
        </Tabs>
        <ToastContainer />
      </main>
    </TooltipProvider>
  );
}

export default function App() {
  return (
    <ErrorBoundary>
      <AppContent />
    </ErrorBoundary>
  );
}
