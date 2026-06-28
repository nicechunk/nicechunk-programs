import { buildChunkRenderData } from "./chunkWorkerBuild.js";

self.onmessage = (event) => {
  const { requestId, payload } = event.data ?? {};
  try {
    const result = buildChunkRenderData(payload);
    self.postMessage({ requestId, ok: true, result }, result.transfer);
  } catch (error) {
    self.postMessage({ requestId, ok: false, error: error?.message ?? String(error) });
  }
};
