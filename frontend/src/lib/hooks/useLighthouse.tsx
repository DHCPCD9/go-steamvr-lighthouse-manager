import { useContext, useEffect, useState } from "preact/hooks"
import { WebsocketContext } from "../context/websocket.context"


export const useLighthouse = (id: string) => {

    const { lighthouses } = useContext(WebsocketContext);

    return lighthouses[id]
}