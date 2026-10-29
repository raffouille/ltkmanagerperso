import { match } from "ts-pattern";

import type { RuneforgeError } from "@/lib/tauri";
import { m } from "@/paraglide/messages";

import type { ErrorCopy } from "./errors";

/** The wait that a rate-limited answer without `Retry-After` names: the 60 s window of the limit. */
const DEFAULT_RETRY_SECONDS = 60;

/** The copy for a failed call to RuneForge, each kind with its remedy. */
export function describeRuneforgeError(error: RuneforgeError): ErrorCopy {
  return match(error)
    .with({ kind: "NOT_CONNECTED" }, () => ({
      title: m["error.RUNEFORGE.NOT_CONNECTED.title"](),
      description: m["error.RUNEFORGE.NOT_CONNECTED.description"](),
    }))
    .with({ kind: "UNAUTHORIZED" }, () => ({
      title: m["error.RUNEFORGE.UNAUTHORIZED.title"](),
      description: m["error.RUNEFORGE.UNAUTHORIZED.description"](),
    }))
    .with({ kind: "FORBIDDEN" }, ({ scope }) => ({
      title: m["error.RUNEFORGE.FORBIDDEN.title"](),
      description:
        scope === null
          ? m["error.RUNEFORGE.FORBIDDEN.description"]()
          : m["error.RUNEFORGE.FORBIDDEN.scope.description"]({ scope }),
    }))
    .with({ kind: "NOT_FOUND" }, () => ({ title: m["error.RUNEFORGE.NOT_FOUND.title"]() }))
    .with({ kind: "PACKAGE_REJECTED" }, () => ({
      title: m["error.RUNEFORGE.PACKAGE_REJECTED.title"](),
      description: m["error.RUNEFORGE.PACKAGE_REJECTED.description"](),
    }))
    .with({ kind: "TAG_TAKEN" }, ({ suggestedTag }) => ({
      title: m["error.RUNEFORGE.TAG_TAKEN.title"](),
      description: m["error.RUNEFORGE.TAG_TAKEN.description"]({ tag: suggestedTag }),
    }))
    .with({ kind: "UPLOAD_STATE" }, () => ({
      title: m["error.RUNEFORGE.UPLOAD_STATE.title"](),
      description: m["error.RUNEFORGE.UPLOAD_STATE.description"](),
    }))
    .with({ kind: "PUBLISH_REFUSED" }, () => ({
      title: m["error.RUNEFORGE.PUBLISH_REFUSED.title"](),
      description: m["error.RUNEFORGE.PUBLISH_REFUSED.description"](),
    }))
    .with({ kind: "CONFLICT" }, () => ({
      title: m["error.RUNEFORGE.CONFLICT.title"](),
      description: m["error.RUNEFORGE.CONFLICT.description"](),
    }))
    .with({ kind: "TOO_LARGE" }, () => ({
      title: m["error.RUNEFORGE.TOO_LARGE.title"](),
      description: m["error.RUNEFORGE.TOO_LARGE.description"](),
    }))
    .with({ kind: "RATE_LIMITED" }, ({ retryAfter }) => ({
      title: m["error.RUNEFORGE.RATE_LIMITED.title"](),
      description: m["error.RUNEFORGE.RATE_LIMITED.description"]({
        seconds: retryAfter ?? DEFAULT_RETRY_SECONDS,
      }),
    }))
    .with({ kind: "STORAGE" }, ({ status }) => ({
      title: m["error.RUNEFORGE.STORAGE.title"](),
      description: m["error.RUNEFORGE.STORAGE.description"]({ status }),
    }))
    .with({ kind: "HTTP" }, ({ status, code }) => ({
      title: m["error.RUNEFORGE.HTTP.title"](),
      description: m["error.RUNEFORGE.HTTP.description"]({ status, code }),
    }))
    .with({ kind: "MALFORMED" }, ({ detail }) => ({
      title: m["error.RUNEFORGE.MALFORMED.title"](),
      detail,
    }))
    .with({ kind: "OFFLINE" }, ({ detail }) => ({
      title: m["error.RUNEFORGE.OFFLINE.title"](),
      description: m["error.RUNEFORGE.OFFLINE.description"](),
      detail,
    }))
    .with({ kind: "CANCELLED" }, () => ({ title: m["error.RUNEFORGE.CANCELLED.title"]() }))
    .with({ kind: "CREDENTIAL" }, ({ detail }) => ({
      title: m["error.RUNEFORGE.CREDENTIAL.title"](),
      description: m["error.RUNEFORGE.CREDENTIAL.description"](),
      detail,
    }))
    .exhaustive();
}
