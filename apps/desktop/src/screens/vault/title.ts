/** An item with no title, or only blanks, shows the "Untitled" placeholder wherever it is listed. */
export function titleOrUntitled(title: string | null, untitled: string): string {
  return title === null || title.trim() === "" ? untitled : title;
}
