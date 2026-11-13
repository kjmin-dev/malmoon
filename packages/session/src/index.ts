// The gateway session for TypeScript clients: protocol types generated from malmoon-core, a
// reconnecting WebSocket client, and the caption list logic.

export { applyCaption } from "./captions.ts";
export { connectSession, type Session, type SessionEvent, type SessionOptions } from "./client.ts";
export type { CaptionStage } from "./generated/CaptionStage.ts";
export type { CaptionUpdate } from "./generated/CaptionUpdate.ts";
export type { Channel } from "./generated/Channel.ts";
export type { ClientKind } from "./generated/ClientKind.ts";
export type { ClientMessage } from "./generated/ClientMessage.ts";
export { PROTOCOL_VERSION, SESSION_PATH } from "./generated/constants.ts";
export type { ErrorCode } from "./generated/ErrorCode.ts";
export type { ServerMessage } from "./generated/ServerMessage.ts";
