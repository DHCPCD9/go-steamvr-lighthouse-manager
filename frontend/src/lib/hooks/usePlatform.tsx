import { useContext, useEffect, useState } from "preact/hooks"
import { WebsocketContext } from "../context/websocket.context"


export const usePlatform = () => {

    const { platform } = useContext(WebsocketContext);
    return platform
}