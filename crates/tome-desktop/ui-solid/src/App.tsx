import { createResource, For, Show, type Component } from "solid-js";
// Single source of truth: the canonical generated bindings, aliased (not copied).
import { commands } from "@bindings";
import type {
  CountOrError_Serialize,
  DirectoryRole,
  StatusReport_Serialize,
  TomeError,
} from "@bindings";

function count(c: CountOrError_Serialize): string {
  if (c.error) return c.error;
  return c.count === null ? "—" : String(c.count);
}

function roleClass(role: DirectoryRole): string {
  return `badge role-${role}`;
}

const App: Component = () => {
  // createResource models the async typed result idiomatically; we split the
  // discriminated union into two derived accessors so the JSX stays flat.
  const [result] = createResource(() => commands.getStatus());
  const status = (): StatusReport_Serialize | null => {
    const r = result();
    return r && r.status === "ok" ? r.data : null;
  };
  const error = (): TomeError | null => {
    const r = result();
    return r && r.status === "error" ? r.error : null;
  };

  return (
    <div class="app">
      <h1>tome</h1>
      <p class="subtitle">Solid spike · live StatusReport from your tome_home</p>

      <Show when={error()}>
        {(err) => (
          <div class="error-banner">
            <strong>[{err().code}]</strong> {err().message}
          </div>
        )}
      </Show>

      <Show when={status()} fallback={<Show when={!error()}><p>Loading…</p></Show>}>
        {(s) => (
          <>
            <div class="cards">
              <div class="card">
                <div class="label">Configured</div>
                <div class="value">{s().configured ? "Yes" : "No"}</div>
              </div>
              <div class="card">
                <div class="label">Library skills</div>
                <div class="value">{count(s().library_count)}</div>
              </div>
              <div class="card">
                <div class="label">Health issues</div>
                <div class="value">{count(s().health)}</div>
              </div>
              <div class="card">
                <div class="label">Last sync</div>
                <div class="value" style={{ "font-size": "13px" }}>
                  {s().last_sync ?? "never"}
                </div>
              </div>
            </div>

            <section>
              <h2>Library</h2>
              <table>
                <tbody>
                  <tr>
                    <th>Path</th>
                    <td class="mono">{s().library_dir}</td>
                  </tr>
                </tbody>
              </table>
            </section>

            <section>
              <h2>Directories ({s().directories.length})</h2>
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
                  <For each={s().directories}>
                    {(d) => (
                      <tr>
                        <td>
                          {d.name}
                          <Show when={d.override_applied}>
                            <span class="mono"> (override)</span>
                          </Show>
                        </td>
                        <td class="mono">{d.directory_type}</td>
                        <td>
                          <span class={roleClass(d.role)}>{d.role}</span>
                        </td>
                        <td>{count(d.skill_count)}</td>
                        <td class="mono">{d.path}</td>
                      </tr>
                    )}
                  </For>
                </tbody>
              </table>
            </section>

            <Show when={s().unowned.length > 0}>
              <section>
                <h2>Unowned skills ({s().unowned.length})</h2>
                <table>
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Last-known source</th>
                      <th>Synced</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={s().unowned}>
                      {(skill) => (
                        <tr>
                          <td>{skill.name}</td>
                          <td class="mono">{skill.previous_source ?? "—"}</td>
                          <td class="mono">{skill.synced_at}</td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </section>
            </Show>
          </>
        )}
      </Show>
    </div>
  );
};

export default App;
