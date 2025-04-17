import { useContext, useEffect, useState } from "preact/hooks"
import { WebsocketContext } from "../context/websocket.context"


export const useLighthouses = () => {

    const { lighthouses } = useContext(WebsocketContext);

    return Object.values(lighthouses)
}