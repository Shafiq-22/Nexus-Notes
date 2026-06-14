import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { Node as PMNode } from "@tiptap/pm/model";

const RE = /\[\[([^\[\]\n]+)\]\]/g;

export interface WikiLinkOptions {
  onOpen: (target: string) => void;
}

const key = new PluginKey<DecorationSet>("wikilink");

function buildDecos(doc: PMNode): DecorationSet {
  const decos: Decoration[] = [];
  doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return;
    RE.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = RE.exec(node.text))) {
      const from = pos + m.index;
      decos.push(Decoration.inline(from, from + m[0].length, { class: "wikilink" }));
    }
  });
  return DecorationSet.create(doc, decos);
}

function targetAt(doc: PMNode, pos: number): string | null {
  let found: string | null = null;
  doc.descendants((node, start) => {
    if (found || !node.isText || !node.text) return;
    RE.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = RE.exec(node.text))) {
      const from = start + m.index;
      const to = from + m[0].length;
      if (pos >= from && pos <= to) {
        found = m[1].split(/[|#]/)[0].trim();
        return;
      }
    }
  });
  return found;
}

/** Renders `[[wikilinks]]` as clickable tokens. Serialization is unaffected —
 * the literal `[[...]]` text round-trips to Markdown unchanged. */
export const WikiLink = Extension.create<WikiLinkOptions>({
  name: "wikilink",
  addOptions() {
    return { onOpen: () => {} };
  },
  addProseMirrorPlugins() {
    const onOpen = this.options.onOpen;
    return [
      new Plugin<DecorationSet>({
        key,
        state: {
          init: (_, { doc }) => buildDecos(doc),
          apply: (tr, old) => (tr.docChanged ? buildDecos(tr.doc) : old),
        },
        props: {
          decorations(state) {
            return key.getState(state);
          },
          handleClick(view, pos) {
            const target = targetAt(view.state.doc, pos);
            if (target) {
              onOpen(target);
              return true;
            }
            return false;
          },
        },
      }),
    ];
  },
});
