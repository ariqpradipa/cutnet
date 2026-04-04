import { TooltipProvider } from "@/components/ui/tooltip";
import { DeviceTable } from "@/components/DeviceTable";
import { ScanControls } from "@/components/ScanControls";
import "./App.css";

function App() {
  return (
    <TooltipProvider>
      <main className="container mx-auto p-6 min-h-screen bg-background">
        <header className="mb-8">
          <h1 className="text-4xl font-bold text-foreground mb-2">CutNet</h1>
          <p className="text-muted-foreground">
            Network Administration Tool
          </p>
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
