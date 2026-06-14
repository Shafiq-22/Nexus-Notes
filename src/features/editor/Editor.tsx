import { useEffect, useRef, useState } from "react";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "tiptap-markdown";
import TaskList from "@tiptap/extension-task-list";
import TaskItem from "@tiptap/extension-task-item";
import Link from "@tiptap/extension-link";
import Placeholder from "@tiptap/extension-placeholder";
import Table from "@tiptap/extension-table";
import TableRow from "@tiptap/extension-table-row";
import TableCell from "@tiptap/extension-table-cell";
import TableHeader from "@tiptap/extension-table-header";
import { api } from "../../lib/ipc";
import { useVault } from "../../store";
import { SlashCommand } from "./slash";
import { WikiLink } from "./wikilink";
import type { FsEvent } from "../../lib/types";
import { Star } from "../../components/icons";

const basename = (p: string) => (p.split("/").pop() || p).replace(/\.md$/, "");

export default function Editor({ path }: { path: string }) {
  const fm = useRef<Record<string, unknown>>({});
  const baseHash = useRef("");
  const saveTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const [saved, setSaved] = useState(true);

  const openNote = useVault((s) => s.openNote);
  const favorites = useVault((s) => s.favorites);
  const toggleFavorite = useVault((s) => s.toggleFavorite);

  const resolveOpen = async (target: string) => {
    const sugg = await api.searchSuggest(target);
    const notes = sugg.filter((s) => s.kind === "note");
    const pick = notes.find((s) => s.label.toLowerCase() === target.toLowerCase()) || notes[0];
    if (pick?.path) openNote(pick.path);
  };

  const editor = useEditor({
    extensions: [
      StarterKit,
      Markdown.configure({ html: false, transformPastedText: true, transformCopiedText: true }),
      TaskList,
      TaskItem.configure({ nested: true }),
      Link.configure({ openOnClick: false }),
      Placeholder.configure({ placeholder: "Write something, or press / for commands…" }),
      Table.configure({ resizable: false }),
      TableRow,
      TableHeader,
      TableCell,
      SlashCommand,
      WikiLink.configure({ onOpen: resolveOpen }),
    ],
    content: "",
    editorProps: { attributes: { class: "tiptap" } },
    onUpdate: ({ editor }) => {
      setSaved(false);
      clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        const md = editor.storage.markdown.getMarkdown();
        const meta = await api.writeNote(path, md, fm.current);
        baseHash.current = meta.hash;
        setSaved(true);
      }, 600);
    },
  });

  // Load note when the path or editor changes.
  useEffect(() => {
    let active = true;
    void api.readNote(path).then((n) => {
      if (!active || !editor) return;
      fm.current = n.frontmatter || {};
      baseHash.current = n.hash;
      editor.commands.setContent(n.markdown);
      setSaved(true);
    });
    return () => {
      active = false;
    };
  }, [path, editor]);

  // Reload on external edits (only when we have no unsaved changes).
  useEffect(() => {
    const handler = (e: Event) => {
      const det = (e as CustomEvent<FsEvent>).detail;
      if (det.path !== path || !editor) return;
      if (saved && det.meta && det.meta.hash !== baseHash.current) {
        void api.readNote(path).then((n) => {
          fm.current = n.frontmatter || {};
          baseHash.current = n.hash;
          editor.commands.setContent(n.markdown);
        });
      }
    };
    window.addEventListener("nexus:external-change", handler);
    return () => window.removeEventListener("nexus:external-change", handler);
  }, [path, editor, saved]);

  const isFav = favorites.includes(path);
  const crumbs = path.split("/");

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-border px-6 py-2.5 text-sm">
        <span className="truncate text-text-muted">
          {crumbs.slice(0, -1).join(" / ")}
          {crumbs.length > 1 && " / "}
          <span className="text-text-normal">{basename(path)}</span>
        </span>
        <button className="icon-btn ml-auto" title="Favorite" onClick={() => toggleFavorite(path)}>
          <Star filled={isFav} size={16} />
        </button>
        <span className="w-14 text-right text-xs text-text-faint">{saved ? "Saved" : "Saving…"}</span>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto max-w-[760px] px-10 py-10">
          <EditorContent editor={editor} />
        </div>
      </div>
    </div>
  );
}
