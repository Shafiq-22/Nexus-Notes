import { useEffect, useRef, useState } from "react";
import ForceGraph2D from "react-force-graph-2d";
import { api } from "../lib/ipc";
import { useVault } from "../store";
import type { GraphData } from "../lib/types";

const cssVar = (n: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(n).trim() || "#888";

export default function GraphView() {
  const [data, setData] = useState<GraphData>({ nodes: [], links: [] });
  const [size, setSize] = useState({ w: 800, h: 600 });
  const wrapRef = useRef<HTMLDivElement>(null);
  const openNote = useVault((s) => s.openNote);

  useEffect(() => {
    void api.graphGlobal().then(setData);
  }, []);

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const measure = () => setSize({ w: el.clientWidth, h: el.clientHeight });
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    measure();
    return () => ro.disconnect();
  }, []);

  const Graph = ForceGraph2D as unknown as (props: Record<string, unknown>) => JSX.Element;

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-3 border-b border-border px-6 py-2.5">
        <span className="text-sm font-medium">Knowledge Graph</span>
        <span className="text-xs text-text-faint">
          {data.nodes.length} notes · {data.links.length} links
        </span>
      </div>
      <div ref={wrapRef} className="min-h-0 flex-1">
        <Graph
          width={size.w}
          height={size.h}
          graphData={data}
          backgroundColor={cssVar("--bg-primary")}
          nodeId="id"
          nodeVal="val"
          nodeLabel="label"
          nodeColor={() => cssVar("--accent")}
          linkColor={() => cssVar("--border-strong")}
          linkWidth={1}
          nodeRelSize={4}
          cooldownTicks={80}
          onNodeClick={(n: { id?: string }) => n.id && openNote(n.id)}
          nodeCanvasObjectMode={() => "after"}
          nodeCanvasObject={(node: any, ctx: CanvasRenderingContext2D, scale: number) => {
            if (scale < 1.3) return;
            const fontSize = 11 / scale;
            ctx.font = `${fontSize}px Inter, sans-serif`;
            ctx.fillStyle = cssVar("--text-muted");
            ctx.textAlign = "center";
            ctx.textBaseline = "top";
            ctx.fillText(node.label, node.x, node.y + Math.sqrt(node.val) * 4 + 2);
          }}
        />
      </div>
    </div>
  );
}
