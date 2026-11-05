export const HTTPS_PORT: number;
export const HTTP_PORT: number;
export function startServer(): Promise<{ httpsPort: number; httpPort: number; close(): Promise<void> }>;
