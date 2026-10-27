import { test } from "node:test";
import assert from "node:assert/strict";
import {
  ORIGINAL_ARTWORK_FILES,
  agentArtwork,
  agentArtworkKey,
  agentVendorName,
} from "../src/agent-artwork.ts";

// The resting HUD draws each agent by its vendor name (HUDMarkStrip passes
// `vendor.name` to BotMark), so every name must lead back to its own mark.
test("every agent's vendor name finds its own artwork", () => {
  for (const file of Object.values(ORIGINAL_ARTWORK_FILES)) {
    if (file.endsWith("-dark")) continue;
    const name = agentVendorName(file);
    assert.equal(agentArtworkKey(name), agentArtworkKey(file), name);
    assert.notEqual(agentArtwork(name), null, name);
  }
});
