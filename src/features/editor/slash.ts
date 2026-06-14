import { Extension } from "@tiptap/core";
import Suggestion from "@tiptap/suggestion";
import type { Editor, Range } from "@tiptap/core";

interface Item {
  title: string;
  hint: string;
  run: (editor: Editor, range: Range) => void;
}

function items(query: string): Item[] {
  const all: Item[] = [
    { title: "Heading 1", hint: "#", run: (e, r) => e.chain().focus().deleteRange(r).setNode("heading", { level: 1 }).run() },
    { title: "Heading 2", hint: "##", run: (e, r) => e.chain().focus().deleteRange(r).setNode("heading", { level: 2 }).run() },
    { title: "Heading 3", hint: "###", run: (e, r) => e.chain().focus().deleteRange(r).setNode("heading", { level: 3 }).run() },
    { title: "Bullet list", hint: "-", run: (e, r) => e.chain().focus().deleteRange(r).toggleBulletList().run() },
    { title: "Numbered list", hint: "1.", run: (e, r) => e.chain().focus().deleteRange(r).toggleOrderedList().run() },
    { title: "To-do list", hint: "[ ]", run: (e, r) => e.chain().focus().deleteRange(r).toggleTaskList().run() },
    { title: "Quote", hint: ">", run: (e, r) => e.chain().focus().deleteRange(r).toggleBlockquote().run() },
    { title: "Code block", hint: "```", run: (e, r) => e.chain().focus().deleteRange(r).toggleCodeBlock().run() },
    { title: "Divider", hint: "---", run: (e, r) => e.chain().focus().deleteRange(r).setHorizontalRule().run() },
    { title: "Table", hint: "▦", run: (e, r) => e.chain().focus().deleteRange(r).insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run() },
  ];
  const q = query.toLowerCase();
  return all.filter((i) => i.title.toLowerCase().includes(q));
}

export const SlashCommand = Extension.create({
  name: "slashCommand",
  addProseMirrorPlugins() {
    return [
      Suggestion({
        editor: this.editor,
        char: "/",
        startOfLine: false,
        command: ({ editor, range, props }) => (props as Item).run(editor, range),
        items: ({ query }) => items(query),
        // DOM-rendered popup (no extra deps).
        render: () => {
          let el: HTMLDivElement | null = null;
          let list: Item[] = [];
          let selected = 0;
          let pick: (item: Item) => void = () => {};

          const draw = () => {
            if (!el) return;
            el.innerHTML = "";
            list.forEach((it, i) => {
              const row = document.createElement("div");
              row.className = "slash-item";
              row.setAttribute("aria-selected", String(i === selected));
              row.innerHTML = `<span style="opacity:.6;width:26px;display:inline-block">${it.hint}</span><span>${it.title}</span>`;
              row.onmousedown = (e) => {
                e.preventDefault();
                pick(it);
              };
              el!.appendChild(row);
            });
          };
          const place = (rect: DOMRect | null) => {
            if (!el || !rect) return;
            el.style.position = "fixed";
            el.style.left = `${rect.left}px`;
            el.style.top = `${rect.bottom + 6}px`;
          };

          return {
            onStart: (props: any) => {
              list = props.items;
              selected = 0;
              pick = (it) => props.command(it);
              el = document.createElement("div");
              el.className = "slash-menu";
              document.body.appendChild(el);
              place(props.clientRect?.());
              draw();
            },
            onUpdate: (props: any) => {
              list = props.items;
              pick = (it) => props.command(it);
              if (selected >= list.length) selected = 0;
              place(props.clientRect?.());
              draw();
            },
            onKeyDown: (props: any) => {
              const e: KeyboardEvent = props.event;
              if (e.key === "ArrowDown") {
                selected = (selected + 1) % Math.max(list.length, 1);
                draw();
                return true;
              }
              if (e.key === "ArrowUp") {
                selected = (selected - 1 + list.length) % Math.max(list.length, 1);
                draw();
                return true;
              }
              if (e.key === "Enter") {
                if (list[selected]) pick(list[selected]);
                return true;
              }
              return e.key === "Escape";
            },
            onExit: () => {
              el?.remove();
              el = null;
            },
          };
        },
      }),
    ];
  },
});
