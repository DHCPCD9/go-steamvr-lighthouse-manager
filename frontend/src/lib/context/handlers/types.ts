import type { WebsocketContextType } from '../websocket.context';

export type StateUpdater = (prevState: WebsocketContextType) => WebsocketContextType;
export type EventHandler = (data: any) => StateUpdater | void;
export type EventHandlerMap = Record<string, EventHandler>;