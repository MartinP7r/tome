<script lang="ts">
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

  let status = $state<StatusReport_Serialize | null>(null);
  let error = $state<TomeError | null>(null);

  // Typed discriminated-union result straight from @bindings — no casts.
  commands.getStatus().then((res) => {
    if (res.status === "ok") status = res.data;
    else error = res.error;
  });
</script>

<div class="app">
  <h1>tome</h1>
  <p class="subtitle">Svelte spike · live StatusReport from your tome_home</p>

  {#if error}
    <div class="error-banner">
      <strong>[{error.code}]</strong>
      {error.message}
    </div>
  {:else if status}
    <div class="cards">
      <div class="card">
        <div class="label">Configured</div>
        <div class="value">{status.configured ? "Yes" : "No"}</div>
      </div>
      <div class="card">
        <div class="label">Library skills</div>
        <div class="value">{count(status.library_count)}</div>
      </div>
      <div class="card">
        <div class="label">Health issues</div>
        <div class="value">{count(status.health)}</div>
      </div>
      <div class="card">
        <div class="label">Last sync</div>
        <div class="value" style="font-size: 13px">
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
            <td class="mono">{status.library_dir}</td>
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
          {#each status.directories as d (d.name)}
            <tr>
              <td>
                {d.name}
                {#if d.override_applied}<span class="mono"> (override)</span>{/if}
              </td>
              <td class="mono">{d.directory_type}</td>
              <td><span class={roleClass(d.role)}>{d.role}</span></td>
              <td>{count(d.skill_count)}</td>
              <td class="mono">{d.path}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>

    {#if status.unowned.length > 0}
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
            {#each status.unowned as skill (skill.name)}
              <tr>
                <td>{skill.name}</td>
                <td class="mono">{skill.previous_source ?? "—"}</td>
                <td class="mono">{skill.synced_at}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    {/if}
  {:else}
    <p>Loading…</p>
  {/if}
</div>
