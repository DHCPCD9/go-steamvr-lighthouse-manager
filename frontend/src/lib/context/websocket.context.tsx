import { createContext } from 'preact'
import { useEffect, useRef, useState, useCallback, useMemo } from 'preact/hooks'
import type { ClientPayload, ClientPlatformPayload, ClientUpdateFoundPayload, Configuration, LighthouseGroup, LighthouseStation, WebsocketPayload } from "@src/lib/types/index"
import type { ReactNode } from 'react'
import { eventHandlers } from './handlers/index'

export interface WebsocketContextType {
    lighthouses: Record<string, LighthouseStation>,
    steamvr: {
        active: boolean
    },
    groups: Record<string, LighthouseGroup>,
    configuration: Configuration,
    websocket: {
        ready: boolean,
    },
    platform: ClientPlatformPayload['data'],
    scanning: boolean,
    send: (payload: ClientPayload) => void,
    update: ClientUpdateFoundPayload['data'] | null
}

const DEFAULT_CONTEXT_VALUE: WebsocketContextType = {
    lighthouses: {},
    steamvr: {
        active: false
    },
    groups: {},
    configuration: {
        allow_tray: false,
        groups: {},
        is_steamvr_installed: false,
        is_steamvr_managed: false,
        known_base_stations: {},
        tray_notified: false,
        branch: "main"
    },
    websocket: {
        ready: false
    },
    platform: {
        system: "windows",
        flags: "",
        version: ""
    },
    send: () => {
        console.warn("WebSocket not ready!")
    },
    scanning: false,
    update: null
};

export const WebsocketContext = createContext<WebsocketContextType>(DEFAULT_CONTEXT_VALUE);

export const WebsocketProvider = ({ children }: { children: ReactNode }) => {
    const [state, setState] = useState<WebsocketContextType>(DEFAULT_CONTEXT_VALUE);
    const ws = useRef<WebSocket | null>(null);
    const reconnectTimeoutRef = useRef<number | null>(null);
    
    const handleMessage = useCallback((data: WebsocketPayload) => {
        const handler = eventHandlers[data.event];
        if (handler) {
            const stateUpdater = handler(data);
            if (stateUpdater) {
                setState(stateUpdater);
            }
        } else {
            console.warn(`Unknown event: ${data.event}`);
        }
    }, []);

    const send = useCallback((payload: ClientPayload) => {
        if (ws.current?.readyState === WebSocket.OPEN) {
            ws.current.send(JSON.stringify(payload));
        } else {
            console.warn("WebSocket not ready!");
        }
    }, []);

    const createSocket = useCallback((): WebSocket => {
        const socket = new WebSocket("ws://localhost:15065/api/lighthouse/websocket");

        socket.onopen = () => {
            console.log("WebSocket connected");
            setState(prevState => ({
                ...prevState,
                websocket: { ready: true }
            }));
        };

        socket.onclose = (event) => {
            console.log("WebSocket disconnected:", event.code, event.reason);
            setState(prevState => ({
                ...prevState,
                websocket: { ready: false }
            }));

            if (event.code !== 1000) {
                reconnectTimeoutRef.current = setTimeout(() => {
                    console.log("Reconnecting...");
                    ws.current = createSocket();
                }, 1000);
            }
        };

        socket.onerror = (error) => {
            console.error("WebSocket error:", error);
        };

        socket.onmessage = (e) => {
            try {
                const data = JSON.parse(e.data);
                handleMessage(data);
            } catch (error) {
                console.error("Error parsing WebSocket message:", error);
            }
        };

        return socket;
    }, [handleMessage]);

    useEffect(() => {
        ws.current = createSocket();

        return () => {
            if (reconnectTimeoutRef.current) {
                clearTimeout(reconnectTimeoutRef.current);
            }
            if (ws.current) {
                ws.current.close(1000, "Component unmounting");
            }
        };
    }, [createSocket]);

    const contextValue = useMemo(() => ({
        ...state,
        send
    }), [state, send]);

    return (
        <WebsocketContext.Provider value={contextValue}>
            {children}
        </WebsocketContext.Provider>
    );
}