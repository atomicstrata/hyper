/** Schedules page targets for the hypergraph browser POC. No Chrome here. */

export const MAX_PAGE_TARGETS = 100;

export function createPageSession({maxTargets = MAX_PAGE_TARGETS} = {}) {
  const targets = new Map();
  let live = null;

  return {
    async open(vertex, driver) {
      const url = typeof vertex?.url === 'string' ? vertex.url.trim() : '';
      if (!url) return {skipped: true};
      if (targets.has(vertex.id)) return {skipped: true};
      if (targets.size >= maxTargets) {
        throw new Error(`Page target limit is ${maxTargets}`);
      }
      const targetId = await driver.createTarget(url);
      targets.set(vertex.id, targetId);
      await driver.bringToFront(targetId);
      const jpeg = await driver.capture(targetId);
      return {skipped: false, vertexId: vertex.id, jpeg};
    },

    async focus(vertexId, driver) {
      if (live && live !== vertexId) {
        await driver.stopScreencast(targets.get(live));
        live = null;
      }
      if (vertexId == null) return;
      const targetId = targets.get(vertexId);
      if (!targetId) throw new Error(`No page target for ${vertexId}`);
      if (live === vertexId) return;
      await driver.bringToFront(targetId);
      await driver.startScreencast(targetId);
      live = vertexId;
    },
  };
}
