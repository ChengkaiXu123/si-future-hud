import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import type { Plugin } from "vite";

const supportedVersion = "4.0.4";
const initializationAnchor = "\t\t\tif (renderer.getFailureReason()) {";
const cleanupAnchor =
  "\t\t\tisInitializedRef.current = false;\n\t\t\tisInitializingRef.current = false;\n\t\t};\n\t}, []);";

/** Temporary, version-locked fixes to the official React adapter, not its renderer.
 * Remove after upstream handles disposal while initialize() is pending.
 */
export function patchShaderLifecycle(source: string, version: string): string {
  if (version !== supportedVersion)
    throw new Error(
      `Shader lifecycle patch requires shaders ${supportedVersion}; found ${version}. Review the upstream lifecycle before changing the patch.`,
    );
  const replaceOnce = (needle: string, replacement: string) => {
    if (source.split(needle).length !== 2)
      throw new Error(
        "Shader lifecycle patch source drifted: expected exactly one official lifecycle anchor.",
      );
    source = source.replace(needle, replacement);
  };
  replaceOnce(
    initializationAnchor,
    "\t\t\t// A detached component may finish requesting its GPU after effect cleanup.\n" +
      "\t\t\tif (isCleanedUpRef.current) {\n" +
      "\t\t\t\trenderer.cleanup();\n" +
      "\t\t\t\tisInitializingRef.current = false;\n" +
      "\t\t\t\treturn;\n" +
      "\t\t\t}\n" +
      initializationAnchor,
  );
  replaceOnce(
    cleanupAnchor,
    "\t\t\tisInitializedRef.current = false;\n" +
      "\t\t\t// Keep a pending initialization latched across StrictMode effect replay.\n" +
      "\t\t\t// Its continuation either serves the live effect or disposes the renderer.\n" +
      "\t\t};\n\t}, []);",
  );
  return source;
}

export function shaderLifecycle(): Plugin {
  const require = createRequire(import.meta.url);
  const packageRoot = dirname(
    dirname(dirname(require.resolve("shaders/core"))),
  );
  const { version } = JSON.parse(
    readFileSync(join(packageRoot, "package.json"), "utf8"),
  ) as { version: string };
  if (version !== supportedVersion)
    throw new Error(
      `Shader lifecycle patch requires shaders ${supportedVersion}; found ${version}. Review or remove the patch before upgrading.`,
    );
  return {
    name: "agent-hud-shader-lifecycle",
    enforce: "pre",
    config: () => ({ optimizeDeps: { exclude: ["shaders"] } }),
    transform(source, id) {
      const path = id.replaceAll("\\", "/");
      if (path.endsWith("/shaders/dist/react/Shader.js"))
        return { code: patchShaderLifecycle(source, version), map: null };
    },
  };
}
