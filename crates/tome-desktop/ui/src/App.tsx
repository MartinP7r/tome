import { useEffect, useState } from "react";
// Single source of truth: the co-located canonical generated bindings.
import { commands } from "./bindings";
import type {
  StatusReport_Serialize,
  CountOrError_Serialize,
  DirectoryRole,
  TomeError,
} from "./bindings";

function count(c: CountOrError_Serialize): string {
  if (c.error) return c.error;
  return c.count === null ? "—" : String(c.count);
}

function roleClass(role: DirectoryRole): string {
  return `badge role-${role}`;
}

export default function App() {
  const [status, setStatus] = useState<StatusReport_Serialize | null>(null);
  const [err, setErr] = useState<TomeError | null>(null);

  useEffect(() => {
    // Typed discriminated-union result straight from ./bindings — no casts.
    commands.getStatus().then((res) => {
      if (res.status === "ok") setStatus(res.data);
      else setErr(res.error);
    });
  }, []);

  if (err) {
    return (
      <div className="app">
        <h1>tome</h1>
        <div className="error-banner">
          <strong>[{err.code}]</strong> {err.message}
          {err.context.length > 0 && (
            <ul>
              {err.context.map((c, i) => (
                <li key={i}>{c}</li>
              ))}
            </ul>
          )}
        </div>
      </div>
    );
  }

  if (!status) return <div className="app">Loading…</div>;

  return (
    <div className="app">
      <h1>tome</h1>
      <p className="subtitle">Live StatusReport from your tome_home</p>

      <div className="cards">
        <div className="card">
          <div className="label">Configured</div>
          <div className="value">{status.configured ? "Yes" : "No"}</div>
        </div>
        <div className="card">
          <div className="label">Library skills</div>
          <div className="value">{count(status.library_count)}</div>
        </div>
        <div className="card">
          <div className="label">Health issues</div>
          <div className="value">{count(status.health)}</div>
        </div>
        <div className="card">
          <div className="label">Last sync</div>
          <div className="value" style={{ fontSize: 13 }}>
            {status.last_sync ?? "never"}
          </div>
        </div>
      </div>

      <section>
        <h2>Library</h2>
        <table>
          <tbody>
            <tr>
              <th>Path</th>
              <td className="mono">{status.library_dir}</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section>
        <h2>Directories ({status.directories.length})</h2>
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Role</th>
              <th>Skills</th>
              <th>Path</th>
            </tr>
          </thead>
          <tbody>
            {status.directories.map((d) => (
              <tr key={d.name}>
                <td>
                  {d.name}
                  {d.override_applied && <span className="mono"> (override)</span>}
                </td>
                <td className="mono">{d.directory_type}</td>
                <td>
                  <span className={roleClass(d.role)}>{d.role}</span>
                </td>
                <td>{count(d.skill_count)}</td>
                <td className="mono">{d.path}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      {status.unowned.length > 0 && (
        <section>
          <h2>Unowned skills ({status.unowned.length})</h2>
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Last-known source</th>
                <th>Synced</th>
              </tr>
            </thead>
            <tbody>
              {status.unowned.map((s) => (
                <tr key={s.name}>
                  <td>{s.name}</td>
                  <td className="mono">{s.previous_source ?? "—"}</td>
                  <td className="mono">{s.synced_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </div>
  );
}
