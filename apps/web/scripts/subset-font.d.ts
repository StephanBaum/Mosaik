declare module "subset-font" {
  export interface SubsetOptions {
    targetFormat?: "sfnt" | "woff" | "woff2";
    variationAxes?: Record<string, number | { min?: number; max?: number; default?: number }>;
    preserveNameIds?: number[];
    noLayoutClosure?: boolean;
    keepAllGlyphs?: boolean;
  }
  export default function subsetFont(font: Buffer, text: string, options?: SubsetOptions): Promise<Buffer>;
}
