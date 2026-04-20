import { useQuery } from "@tanstack/react-query";
import { greet } from "@/services/commands";

export const useGreet = (name: string) =>
  useQuery({
    queryKey: ["greet", name],
    queryFn: () => greet(name),
    enabled: name.length > 0,
  });
