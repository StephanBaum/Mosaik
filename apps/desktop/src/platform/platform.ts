/** The desktop platform, from the webview user agent. Used for shortcut and label variants. */
import { createContext, useContext } from "react";

export type Platform = "mac" | "windows" | "linux";

export function detectPlatform(userAgent: string = navigator.userAgent): Platform {
  if (/Macintosh|Mac OS X/.test(userAgent)) {
    return "mac";
  }
  return /Windows/.test(userAgent) ? "windows" : "linux";
}

export const PlatformContext = createContext<Platform>(detectPlatform());

export function usePlatform(): Platform {
  return useContext(PlatformContext);
}
