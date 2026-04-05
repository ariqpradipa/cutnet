import { TooltipProvider } from "@/components/ui/tooltip";
import { Badge } from "@/components/ui/badge";
import { DeviceTable } from "@/components/DeviceTable";
import { ScanControls } from "@/components/ScanControls";
import { useNetworkStore } from "@/stores/networkStore";
import "./App.css";

function App() {
  const { isRunning, isScanning } = useNetworkStore();

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

        <div className="grid gap-6">
          <ScanControls />
          <DeviceTable />
        </div>
      </main>
    </TooltipProvider>
  );
}

export default App;
