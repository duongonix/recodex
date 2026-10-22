export type ReleaseNoteBlock = {
  kind: "heading" | "paragraph" | "list-item";
  text: string;
};

function plainText(markdown: string): string {
  return markdown
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g, "$1 — $2")
    .replace(/\*\*(.*?)\*\*|__(.*?)__/g, (_match, boldA: string | undefined, boldB: string | undefined) => boldA ?? boldB ?? "")
    .replace(/\*(.*?)\*|_(.*?)_/g, (_match, italicA: string | undefined, italicB: string | undefined) => italicA ?? italicB ?? "")
    .replace(/~~(.*?)~~/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .trim();
}

function isListItem(line: string): boolean {
  return /^\s*(?:[-*+]\s+|\d+[.)]\s+)/.test(line);
}

export function parseReleaseNotes(markdown: string): ReleaseNoteBlock[] {
  const lines = markdown.replace(/\r\n?/g, "\n").split("\n");
  const blocks: ReleaseNoteBlock[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index].trim();
    if (!line) {
      index += 1;
      continue;
    }

    const heading = line.match(/^#{1,6}\s+(.+)$/);
    if (heading) {
      blocks.push({ kind: "heading", text: plainText(heading[1]) });
      index += 1;
      continue;
    }

    if (isListItem(lines[index])) {
      while (index < lines.length && isListItem(lines[index])) {
        const item = lines[index].trim().replace(/^(?:[-*+]\s+|\d+[.)]\s+)/, "");
        blocks.push({ kind: "list-item", text: plainText(item) });
        index += 1;
      }
      continue;
    }

    const paragraph = [line];
    index += 1;
    while (index < lines.length && lines[index].trim() && !/^\s*#{1,6}\s+/.test(lines[index]) && !isListItem(lines[index])) {
      paragraph.push(lines[index].trim());
      index += 1;
    }
    blocks.push({ kind: "paragraph", text: plainText(paragraph.join(" ")) });
  }

  return blocks;
}
