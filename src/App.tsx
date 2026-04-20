import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { useGreet } from "@/hooks/use-greet";
import { useAppStore } from "@/stores/app";

function App() {
  const [name, setName] = useState("");
  const [submitted, setSubmitted] = useState("");
  const { theme, toggleTheme } = useAppStore();
  const { data, isFetching, error } = useGreet(submitted);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitted(name);
  };

  return (
    <div className={theme}>
      <div className="bg-background text-foreground min-h-screen flex items-center justify-center p-6">
        <Card className="w-full max-w-md">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>tauri-starter</CardTitle>
              <Badge variant="secondary">v0.1.0</Badge>
            </div>
            <CardDescription>
              Tauri 2 · React 19 · Vite 7 · shadcn/ui · Zustand · TanStack Query
            </CardDescription>
          </CardHeader>

          <Separator />

          <CardContent className="pt-6 space-y-4">
            <form onSubmit={handleSubmit} className="flex gap-2">
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Enter your name…"
              />
              <Button type="submit" disabled={isFetching || !name}>
                {isFetching ? "…" : "Greet"}
              </Button>
            </form>

            {error && (
              <p className="text-destructive text-sm">
                {error instanceof Error ? error.message : "Command failed"}
              </p>
            )}

            {data && (
              <div className="space-y-1">
                <p className="text-sm font-medium">{data.message}</p>
                <p className="text-muted-foreground text-xs">app version: {data.version}</p>
              </div>
            )}

            <Separator />

            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">Theme: {theme}</span>
              <Button variant="outline" size="sm" onClick={toggleTheme}>
                Toggle
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export default App;
