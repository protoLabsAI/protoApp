import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Chat } from "@/components/chat";
import { useAppStore } from "@/stores/app";

function App() {
  const { theme, toggleTheme } = useAppStore();

  return (
    <div className={theme}>
      <div className="bg-background text-foreground min-h-screen flex flex-col items-center p-6 gap-4">
        <div className="w-full max-w-2xl flex items-center justify-between">
          <div className="flex items-center gap-2">
            <h1 className="text-lg font-semibold">protoApp</h1>
            <Badge variant="secondary">v{__APP_VERSION__}</Badge>
          </div>
          <Button variant="outline" size="sm" onClick={toggleTheme}>
            {theme === "light" ? "Dark" : "Light"}
          </Button>
        </div>
        <Chat />
      </div>
    </div>
  );
}

export default App;
