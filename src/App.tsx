import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Chat } from "@/components/chat";
import { Speak } from "@/components/speak";
import { Transcribe } from "@/components/transcribe";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/app";

type Tab = "chat" | "transcribe" | "speak";
const TABS: { id: Tab; label: string }[] = [
  { id: "chat", label: "Chat" },
  { id: "transcribe", label: "Transcribe" },
  { id: "speak", label: "Speak" },
];

function App() {
  const { theme, toggleTheme } = useAppStore();
  const [tab, setTab] = useState<Tab>("chat");

  return (
    <div className={theme}>
      <div className="bg-background text-foreground min-h-screen flex flex-col items-center p-6 gap-4">
        <div className="w-full max-w-2xl flex items-center justify-between">
          <div className="flex items-center gap-2">
            <h1 className="text-lg font-semibold">protoApp</h1>
            <Badge variant="secondary">v{__APP_VERSION__}</Badge>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={toggleTheme}
            aria-label={`Switch to ${theme === "light" ? "dark" : "light"} mode`}
          >
            {theme === "light" ? "Dark" : "Light"}
          </Button>
        </div>

        <div
          role="tablist"
          aria-label="Local AI panels"
          className="w-full max-w-2xl flex items-center gap-1 border-b"
        >
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              id={`tab-${t.id}`}
              aria-selected={tab === t.id}
              aria-controls={`panel-${t.id}`}
              onClick={() => setTab(t.id)}
              className={cn(
                "px-3 py-2 text-sm border-b-2 -mb-px transition-colors",
                tab === t.id
                  ? "border-primary text-foreground"
                  : "border-transparent text-muted-foreground hover:text-foreground",
              )}
            >
              {t.label}
            </button>
          ))}
        </div>

        <div
          id={`panel-${tab}`}
          role="tabpanel"
          aria-labelledby={`tab-${tab}`}
          className="w-full max-w-2xl flex justify-center"
        >
          {tab === "chat" && <Chat />}
          {tab === "transcribe" && <Transcribe />}
          {tab === "speak" && <Speak />}
        </div>
      </div>
    </div>
  );
}

export default App;
