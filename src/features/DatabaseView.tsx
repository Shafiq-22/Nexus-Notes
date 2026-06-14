import { useEffect, useState } from "react";
import clsx from "clsx";
import { api } from "../lib/ipc";
import { useVault } from "../store";
import type { DbColumn, DbResult, DbRow } from "../lib/types";
import { Plus } from "../components/icons";

const inputCls =
  "w-full min-w-[80px] rounded bg-transparent px-2 py-1 text-sm outline-none focus:bg-bg-primary";

function Cell({ col, value, onChange }: { col: DbColumn; value: unknown; onChange: (v: unknown) => void }) {
  if (col.type === "select" || col.type === "multiselect") {
    return (
      <select
        className={clsx(inputCls, "cursor-pointer")}
        value={String(value ?? "")}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="" />
        {col.options.map((o) => (
          <option key={o} value={o}>
            {o}
          </option>
        ))}
      </select>
    );
  }
  if (col.type === "checkbox")
    return <input type="checkbox" checked={!!value} onChange={(e) => onChange(e.target.checked)} />;
  if (col.type === "number")
    return (
      <input
        type="number"
        className={inputCls}
        defaultValue={String(value ?? "")}
        onBlur={(e) => onChange(e.target.value === "" ? null : Number(e.target.value))}
      />
    );
  if (col.type === "date")
    return (
      <input
        type="date"
        className={inputCls}
        defaultValue={String(value ?? "")}
        onBlur={(e) => onChange(e.target.value)}
      />
    );
  return (
    <input
      className={inputCls}
      defaultValue={String(value ?? "")}
      onBlur={(e) => onChange(e.target.value)}
    />
  );
}

function TableView({
  data,
  onEdit,
  onOpen,
}: {
  data: DbResult;
  onEdit: (row: DbRow, key: string, v: unknown) => void;
  onOpen: (path: string) => void;
}) {
  const { schema, rows } = data;
  return (
    <table className="w-full border-collapse text-sm">
      <thead>
        <tr className="border-b border-border-strong text-left text-text-muted">
          <th className="px-3 py-2 font-medium">Name</th>
          {schema.columns.map((c) => (
            <th key={c.key} className="px-3 py-2 font-medium capitalize">
              {c.key}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.path} className="border-b border-border hover:bg-bg-secondary">
            <td className="px-3 py-1.5">
              <button className="font-medium text-text-accent hover:underline" onClick={() => onOpen(row.path)}>
                {row.title}
              </button>
            </td>
            {schema.columns.map((c) => (
              <td key={c.key} className="px-1 py-1">
                <Cell col={c} value={row.fields[c.key]} onChange={(v) => onEdit(row, c.key, v)} />
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function KanbanView({
  data,
  groupBy,
  onMove,
  onOpen,
}: {
  data: DbResult;
  groupBy: string;
  onMove: (row: DbRow, key: string, v: unknown) => void;
  onOpen: (path: string) => void;
}) {
  const col = data.schema.columns.find((c) => c.key === groupBy);
  const columns = [...(col?.options ?? []), "—"];
  const rowsFor = (value: string) =>
    data.rows.filter((r) => (String(r.fields[groupBy] ?? "") || "—") === value);

  return (
    <div className="flex h-full gap-3 overflow-x-auto pb-2">
      {columns.map((value) => (
        <div
          key={value}
          className="flex w-64 shrink-0 flex-col rounded-lg bg-bg-secondary p-2"
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => {
            const p = e.dataTransfer.getData("text/plain");
            const row = data.rows.find((r) => r.path === p);
            if (row) onMove(row, groupBy, value === "—" ? "" : value);
          }}
        >
          <div className="mb-2 flex items-center gap-2 px-1 text-xs font-semibold uppercase text-text-muted">
            <span className="capitalize">{value}</span>
            <span className="text-text-faint">{rowsFor(value).length}</span>
          </div>
          <div className="flex flex-col gap-2">
            {rowsFor(value).map((row) => (
              <div
                key={row.path}
                draggable
                onDragStart={(e) => e.dataTransfer.setData("text/plain", row.path)}
                onClick={() => onOpen(row.path)}
                className="cursor-grab rounded-md border border-border bg-bg-primary p-2.5 text-sm shadow-sm hover:border-border-strong active:cursor-grabbing"
              >
                <div className="font-medium">{row.title}</div>
                <div className="mt-1 flex flex-wrap gap-1">
                  {data.schema.columns
                    .filter((c) => c.key !== groupBy && row.fields[c.key] != null && row.fields[c.key] !== "")
                    .map((c) => (
                      <span
                        key={c.key}
                        className="chip bg-bg-tertiary text-text-muted"
                      >
                        {String(row.fields[c.key])}
                      </span>
                    ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

export default function DatabaseView() {
  const activeDb = useVault((s) => s.activeDb);
  const openDatabase = useVault((s) => s.openDatabase);
  const openNote = useVault((s) => s.openNote);
  const [data, setData] = useState<DbResult | null>(null);
  const [view, setView] = useState(0);

  const load = (p: string) => api.queryDatabase(p).then(setData);

  useEffect(() => {
    if (activeDb) void load(activeDb);
    else void api.listDatabases().then((dbs) => dbs[0] && openDatabase(dbs[0].path));
  }, [activeDb, openDatabase]);

  if (!activeDb)
    return (
      <div className="flex h-full items-center justify-center px-8 text-center text-text-faint">
        No databases yet. Create a folder containing a <code className="mx-1">.nexusdb.json</code> file to
        turn it into a database.
      </div>
    );
  if (!data)
    return <div className="flex h-full items-center justify-center text-text-faint">Loading…</div>;

  const currentView = data.schema.views[view] || { type: "table", name: "Table", groupBy: null };
  const edit = async (row: DbRow, key: string, value: unknown) => {
    await api.upsertRecord(activeDb, row.path, { [key]: value });
    await load(activeDb);
  };
  const addRecord = async () => {
    const title = window.prompt("New record name");
    if (!title) return;
    await api.upsertRecord(activeDb, null, { title, [data.schema.columns[0]?.key ?? "status"]: "" });
    await load(activeDb);
  };

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-3 border-b border-border px-6 py-2.5">
        <span className="text-sm font-medium">{data.schema.name}</span>
        <div className="flex items-center gap-0.5 rounded-lg bg-bg-tertiary p-0.5">
          {data.schema.views.map((v, i) => (
            <button
              key={v.name}
              className={clsx("tab", i === view && "tab-active")}
              onClick={() => setView(i)}
            >
              {v.name}
            </button>
          ))}
        </div>
        <span className="text-xs text-text-faint">{data.rows.length} records</span>
        <button
          className="ml-auto flex items-center gap-1 rounded-md bg-accent px-2.5 py-1 text-sm font-medium text-white hover:bg-accent-hover"
          onClick={addRecord}
        >
          <Plus size={14} /> New
        </button>
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        {currentView.type === "kanban" ? (
          <KanbanView
            data={data}
            groupBy={currentView.groupBy || data.schema.columns[0]?.key || "status"}
            onMove={edit}
            onOpen={openNote}
          />
        ) : (
          <TableView data={data} onEdit={edit} onOpen={openNote} />
        )}
      </div>
    </div>
  );
}
